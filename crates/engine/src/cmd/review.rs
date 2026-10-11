//! Review tab: protection, spelling (basic), workbook statistics, comment navigation and the
//! Comments pane, Show All Notes and Convert to Comments.

use gridcraft_core::CellRef;
use gridcraft_model::{SheetProtection, Visibility, password_hash};
use serde_json::{Value as Json, json};

use super::*;

pub fn specs() -> Vec<CommandSpec> {
    vec![
        cmd!(
            "review.protectSheet",
            "Protect Sheet…",
            ["Review", "Protect"],
            None,
            "{password?, formatCells?, formatColumns?, formatRows?, insertRows?, insertColumns?, deleteRows?, deleteColumns?, sort?, autofilter?}",
            has_doc,
            protect_sheet
        ),
        cmd!("review.unprotectSheet", "Unprotect Sheet", ["Review", "Protect"], None, "{password?}", has_doc, unprotect_sheet),
        cmd!("review.protectWorkbook", "Protect Workbook", ["Review", "Protect"], None, "{on?: bool}", has_doc, protect_workbook),
        cmd!(query "review.workbookStatistics", "Workbook Statistics", ["Review", "Proofing"], None, "{}", has_doc, stats),
        cmd!(query "review.checkAccessibility", "Check Accessibility", ["Review", "Accessibility"], None, "{} → issues", has_doc, accessibility),
        cmd!(noundo "review.previousComment", "Previous Comment", ["Review", "Comments"], None, "{} → {sheet, cell}", has_comments, |s, _| {
            step_comment(s, false)
        }),
        cmd!(noundo "review.nextComment", "Next Comment", ["Review", "Comments"], None, "{} → {sheet, cell}", has_comments, |s, _| step_comment(
            s, true
        )),
        cmd!(query "review.showComments", "Show Comments", ["Review", "Comments"], None, "{on?: bool} (toggles the Comments pane)", has_doc, |s, p| {
            s.comments_pane = bool_param(p, "on").unwrap_or(!s.comments_pane);
            Ok(json!({"on": s.comments_pane}))
        }),
        cmd!(query "review.showAllNotes", "Show All Notes", ["Review", "Notes"], None, "{on?: bool}", has_doc, |s, p| {
            s.show_all_notes = bool_param(p, "on").unwrap_or(!s.show_all_notes);
            Ok(json!({"on": s.show_all_notes}))
        }),
        cmd!(
            "review.convertToComments",
            "Convert to Comments",
            ["Review", "Notes"],
            None,
            "{} (every note in the workbook) → {converted}",
            has_doc,
            convert_notes
        ),
    ]
}

/// Threaded comments in navigation order: visible sheets in tab order, then row by row.
fn comment_cells(s: &Session) -> Vec<(usize, CellRef)> {
    let Some(d) = s.active() else { return vec![] };
    d.wb.sheets
        .iter()
        .enumerate()
        .filter(|(_, sh)| sh.visibility == Visibility::Visible)
        // `CellRef` orders by row, then column: reading order.
        .flat_map(|(i, sh)| sh.comments.iter().filter(|(_, c)| c.threaded).map(move |(at, _)| (i, *at)))
        .collect()
}

fn has_comments(s: &Session) -> std::result::Result<(), String> {
    has_doc(s)?;
    if comment_cells(s).is_empty() { Err("there are no comments in this workbook".into()) } else { Ok(()) }
}

/// Selects the next (or previous) threaded comment after the active cell, moving on to the next
/// sheet and wrapping around at the end of the workbook.
fn step_comment(s: &mut Session, forward: bool) -> Result<Json> {
    let list = comment_cells(s);
    let d = s.doc()?;
    let here = (d.wb.active_sheet, d.selection.active);
    let target = if forward {
        list.iter().find(|c| **c > here).or_else(|| list.first())
    } else {
        list.iter().rev().find(|c| **c < here).or_else(|| list.last())
    };
    let Some(&(sheet, at)) = target else {
        return Err(EngineError::Other("There are no comments in this workbook.".into()));
    };
    let d = s.doc_mut()?;
    d.wb_switch_sheet(sheet);
    d.selection = crate::Selection::at(at);
    if let Some(sh) = d.wb.active() {
        d.selection.expand_merges(sh);
    }
    let name = d.wb.active().map(|sh| sh.name.clone()).unwrap_or_default();
    Ok(json!({"sheet": name, "cell": at.a1()}))
}

/// Review › Notes › Convert to Comments: every note in the workbook becomes a threaded comment
/// with the same author and text.
fn convert_notes(s: &mut Session, _: &Json) -> Result<Json> {
    let sheets: Vec<usize> =
        s.doc()?.wb.sheets.iter().enumerate().filter(|(_, sh)| sh.comments.values().any(|c| !c.threaded)).map(|(i, _)| i).collect();
    if sheets.is_empty() {
        return Ok(json!({"converted": 0}));
    }
    edit(s, |cx| {
        let mut n = 0usize;
        for i in &sheets {
            for c in cx.sheet_mut(*i)?.comments.values_mut().filter(|c| !c.threaded) {
                c.threaded = true;
                c.visible = false;
                n += 1;
            }
        }
        Ok(json!({"converted": n}))
    })
}

fn protect_sheet(s: &mut Session, p: &Json) -> Result<Json> {
    let sheet = target_sheet(s, p)?;
    let b = |k: &str| bool_param(p, k).unwrap_or(false);
    let prot = SheetProtection {
        password_hash: str_param(p, "password").filter(|x| !x.is_empty()).map(password_hash),
        select_locked: true,
        select_unlocked: true,
        format_cells: b("formatCells"),
        format_columns: b("formatColumns"),
        format_rows: b("formatRows"),
        insert_columns: b("insertColumns"),
        insert_rows: b("insertRows"),
        delete_columns: b("deleteColumns"),
        delete_rows: b("deleteRows"),
        sort: b("sort"),
        autofilter: b("autofilter"),
    };
    edit(s, |cx| {
        cx.sheet_mut(sheet)?.protection = Some(prot.clone());
        Ok(Json::Null)
    })
}

fn unprotect_sheet(s: &mut Session, p: &Json) -> Result<Json> {
    let sheet = target_sheet(s, p)?;
    let ph = s.doc()?.wb.sheet(sheet).and_then(|sh| sh.protection.as_ref()).and_then(|pr| pr.password_hash);
    if let Some(h) = ph
        && str_param(p, "password").map(password_hash) != Some(h)
    {
        return Err(EngineError::Other("The password you supplied is not correct.".into()));
    }
    edit(s, |cx| {
        cx.sheet_mut(sheet)?.protection = None;
        Ok(Json::Null)
    })
}

fn protect_workbook(s: &mut Session, p: &Json) -> Result<Json> {
    let cur = s.doc()?.wb.protected_structure;
    let on = bool_param(p, "on").unwrap_or(!cur);
    edit(s, |cx| {
        cx.wb.protected_structure = on;
        Ok(json!({"on": on}))
    })
}

fn stats(s: &mut Session, _: &Json) -> Result<Json> {
    let d = s.doc()?;
    let sh = d.wb.active().ok_or(EngineError::NoDocument)?;
    let formulas = sh.cells.iter().filter(|(_, c)| c.formula.is_some()).count();
    Ok(json!({
        "sheet": {"name": sh.name, "endOfSheet": sh.used_range().map(|r| r.end.a1()), "cellsWithData": sh.cells.iter().filter(|(_, c)| !c.value.is_empty()).count(), "tables": sh.tables.len(), "formulas": formulas, "charts": sh.charts.len(), "comments": sh.comments.len()},
        "workbook": {"sheets": d.wb.sheets.len(), "cellsWithData": d.wb.sheets.iter().map(|s| s.cells.len()).sum::<usize>(), "tables": d.wb.sheets.iter().map(|s| s.tables.len()).sum::<usize>(), "charts": d.wb.sheets.iter().map(|s| s.charts.len()).sum::<usize>(), "names": d.wb.names.len()}
    }))
}

fn accessibility(s: &mut Session, _: &Json) -> Result<Json> {
    let d = s.doc()?;
    let mut issues = Vec::new();
    for sh in &d.wb.sheets {
        for im in &sh.images {
            if im.alt.is_empty() {
                issues.push(json!({"sheet": sh.name, "issue": "Missing alternative text", "object": format!("Picture {}", im.id)}));
            }
        }
        if !sh.merges.is_empty() {
            issues.push(json!({"sheet": sh.name, "issue": "Merged cells", "count": sh.merges.len()}));
        }
        if sh.name.starts_with("Sheet") {
            issues.push(json!({"sheet": sh.name, "issue": "Default sheet name"}));
        }
    }
    Ok(Json::Array(issues))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn book() -> Session {
        let mut s = Session::new();
        s.new_workbook();
        s.execute("home.insertSheet", json!({"name": "Two"})).unwrap();
        s.execute("sheet.activate", json!({"sheet": 0})).unwrap();
        for (sheet, cell) in [(0, "C1"), (0, "A3"), (1, "B2")] {
            s.execute("review.newComment", json!({"sheet": sheet, "cell": cell, "text": "c", "author": "Ann"})).unwrap();
        }
        s.execute("review.newNote", json!({"sheet": 0, "cell": "A2", "text": "note", "author": "Bob"})).unwrap();
        s
    }

    fn at(s: &Session) -> (usize, String) {
        let d = s.doc().unwrap();
        (d.wb.active_sheet, d.selection.active.a1())
    }

    #[test]
    fn next_and_previous_comment_go_in_reading_order_across_sheets_and_wrap() {
        let mut s = book();
        s.execute("selection.set", json!({"cell": "B1"})).unwrap();
        // Notes are skipped: A2 holds a note, not a comment.
        let order = [(0, "C1"), (0, "A3"), (1, "B2"), (0, "C1")];
        for (sheet, cell) in order {
            s.execute("review.nextComment", json!({})).unwrap();
            assert_eq!(at(&s), (sheet, cell.to_string()));
        }
        for (sheet, cell) in [(1, "B2"), (0, "A3"), (0, "C1")] {
            s.execute("review.previousComment", json!({})).unwrap();
            assert_eq!(at(&s), (sheet, cell.to_string()));
        }
    }

    #[test]
    fn comment_navigation_is_disabled_without_comments() {
        let mut s = Session::new();
        s.new_workbook();
        s.execute("review.newNote", json!({"cell": "A1", "text": "note"})).unwrap();
        assert!(s.execute("review.nextComment", json!({})).is_err());
        assert!(s.execute("review.previousComment", json!({})).is_err());
        assert_eq!(at(&s), (0, "A1".to_string()));
    }

    #[test]
    fn convert_to_comments_turns_every_note_into_a_comment_and_undoes() {
        let mut s = book();
        s.execute("review.newNote", json!({"sheet": 1, "cell": "D4", "text": "second", "author": "Cy"})).unwrap();
        s.execute("review.showNote", json!({"sheet": 1, "cell": "D4"})).unwrap();
        let r = s.execute("review.convertToComments", json!({})).unwrap();
        assert_eq!(r["converted"], 2);
        let wb = &s.doc().unwrap().wb;
        let a2 = &wb.sheets[0].comments[&CellRef::parse("A2").unwrap()];
        let d4 = &wb.sheets[1].comments[&CellRef::parse("D4").unwrap()];
        assert!(a2.threaded && d4.threaded && !d4.visible);
        assert_eq!((a2.author.as_str(), a2.text.as_str(), d4.author.as_str(), d4.text.as_str()), ("Bob", "note", "Cy", "second"));
        // Nothing left to convert; one undo restores the notes.
        assert_eq!(s.execute("review.convertToComments", json!({})).unwrap()["converted"], 0);
        s.execute("edit.undo", json!({})).unwrap();
        assert!(!s.doc().unwrap().wb.sheets[0].comments[&CellRef::parse("A2").unwrap()].threaded);
    }

    #[test]
    fn show_comments_and_show_all_notes_toggle() {
        let mut s = book();
        assert_eq!(s.execute("review.showComments", json!({})).unwrap()["on"], true);
        assert_eq!(s.execute("review.showComments", json!({})).unwrap()["on"], false);
        assert_eq!(s.execute("review.showComments", json!({"on": true})).unwrap()["on"], true);
        assert!(s.comments_pane);
        assert_eq!(s.execute("review.showAllNotes", json!({})).unwrap()["on"], true);
        assert_eq!(s.execute("review.showAllNotes", json!({"on": false})).unwrap()["on"], false);
        assert!(!s.show_all_notes);
    }
}
