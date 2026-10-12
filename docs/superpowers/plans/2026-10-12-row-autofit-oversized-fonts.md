# Row Auto-Fit for Oversized Fonts (#833) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Rows without a user-set height grow to fit their tallest whole-cell font when a file opens and when whole-row/column formatting changes fonts, matching Excel's auto-fit; `customHeight` rows keep their height and clip as Excel does.

**Architecture:** Two one-site fixes in existing flows. (1) `apply_style`'s whole-row/column branch already collects the touched cells — extend `Ctx::fit_rows` from them so the existing `commit()` → `auto_row_height()` pass re-fits the rows. (2) All file opens funnel through `open_bytes_with_password` in `io.rs` — after parsing, grow non-custom rows via the existing `auto_row_height`, clamped grow-only (a file's taller non-custom row is never shrunk; our estimate can differ from Excel's). No render change: once rows fit, the degenerate top-pin clamp in `cell_text_y` only fires for `customHeight` rows, where clipping matches Excel.

**Tech Stack:** Rust; crates `engine` (L5) using `model`/`xlsx` (L2) APIs already in scope. No new dependencies.

**Spec:** Upstream issue https://github.com/storytold/gridcraft/issues/833 and the bounded design approved in chat 2026-10-12 (sequencing decision: plain-cell paths on main now; per-run font measurement is a follow-up once rich-text-runs PR #837 merges — explicitly out of scope here). Triage facts: `auto_row_height` (`crates/engine/src/cmd/mod.rs:458`) has one call site (`commit`), and `format.rs`'s line-target branch `continue`s before the `fit_rows.extend` that the cell-by-cell branch has.

## Global Constraints

- No `unwrap`/`expect`/`panic!`/`unsafe` in non-test code (crate lints deny them); errors via `Result` and `?`. Tests may `unwrap`.
- Input-derived numbers are hostile: never iterate `1..MAX_ROWS`; fit only rows that have cells (sparse iteration). Existing caps stay (100,000-row fit guard, 546.0 height cap in `auto_row_height`).
- Excel-parity rules fixed by the spec: non-`custom` rows grow to fit their tallest whole-cell font on open and after whole-row/column formatting; `custom` rows never change; hidden rows stay hidden; open is grow-only (never shrinks a file's height).
- Layering: changes stay in `engine` (L5); `model`/`xlsx`/`core` are consumed, not modified.
- Never break wasm: new code is pure computation, no `std::env`, no threads.
- `cargo xtask ci` green before every commit (fmt, clippy -D warnings, tests, assets, layers, wasm).
- One task id per commit: `#833: <summary>`.
- Tests build workbooks in code; no binary fixtures.
- Per-run (rich text) font sizes are out of scope; do not touch `grid.rs` rendering.

## Review Focus

1. **`customHeight` rows must never be resized** by open-time fitting or whole-row formatting (user-set heights are sacred). → Task 1 test 3, Task 2 test B.
2. **Open must never shrink** a file's taller non-custom row (our line-height estimate differs from Excel's; shrinking would visibly change files that already look right). → Task 2 test C (grow-only clamp).
3. **Whole-column formatting spans 1,048,576 logical rows** — fitting must mark only rows that have cells, not the million-row span (hostile-input rule). → Task 1 test 2 asserts the sheet's `rows` map stays sparse.
4. **Default-font workbooks open as a no-op**: the plain-row fast path in `auto_row_height` must keep default heights and not pollute the `rows` map (big-file open cost stays O(content), sample files unchanged). → Task 2 test E.
5. **Hidden rows stay hidden** through open-time fitting and formatting. → Task 2 test D.

(Undo integrity — formatting's height change must vanish on undo — is pinned inside Task 1 test 1.)

---

### Task 1: Whole-row/column formatting re-fits row heights

**Files:**
- Modify: `crates/engine/src/cmd/format.rs:237-244` (the line-target branch of `apply_style`)
- Test: `crates/engine/tests/row_autofit.rs` (new file)

**Interfaces:**
- Consumes: `Ctx::fit_rows: Vec<(usize, u32)>` (sheet index, row) — already deduped by `commit()`; `cells: Vec<(CellRef, StyleId)>` already collected at format.rs:238-239.
- Produces: nothing new — behavior only.

- [ ] **Step 1: Write the failing tests** in `crates/engine/tests/row_autofit.rs`, following the harness in `crates/engine/tests/autofit_filter_buttons.rs` (`Session::new()`, `s.new_workbook()`, `s.execute(...)`, read state via `s.doc().unwrap().wb.active().unwrap()`; copy its `sheet_mut` helper for setting row info):

```rust
use serde_json::json;

#[test]
fn whole_row_font_format_grows_row_height_and_undoes() {
    let mut s = gridcraft_engine::Session::new();
    s.new_workbook();
    s.execute("range.setValues", json!({"range": "A5", "values": [["plain big bold"]]})).unwrap();
    let before = s.doc().unwrap().wb.active().unwrap().row_height(4);
    s.execute("home.fontSize", json!({"range": "5:5", "size": 20})).unwrap();
    let after = s.doc().unwrap().wb.active().unwrap().row_height(4);
    assert!(after > before + 10.0, "20pt whole-row format should grow the row: before={before}, after={after}");
    s.execute("edit.undo", json!({})).unwrap();
    assert_eq!(s.doc().unwrap().wb.active().unwrap().row_height(4), before, "undo must restore the height");
}

#[test]
fn whole_column_font_format_grows_only_rows_with_cells() {
    let mut s = gridcraft_engine::Session::new();
    s.new_workbook();
    s.execute("range.setValues", json!({"range": "A1", "values": [["a"]]})).unwrap();
    s.execute("range.setValues", json!({"range": "A50", "values": [["b"]]})).unwrap();
    s.execute("home.fontSize", json!({"range": "A:A", "size": 20})).unwrap();
    let sh = s.doc().unwrap().wb.active().unwrap();
    assert!(sh.row_height(0) > 30.0 && sh.row_height(49) > 30.0, "rows with cells grow");
    assert!(sh.rows.len() <= 3, "full-column format must fit only rows that have cells, rows map has {} entries", sh.rows.len());
}

#[test]
fn whole_row_format_does_not_resize_custom_height_rows() {
    let mut s = gridcraft_engine::Session::new();
    s.new_workbook();
    s.execute("range.setValues", json!({"range": "A5", "values": [["plain big bold"]]})).unwrap();
    { let sh = sheet_mut(&mut s); let e = sh.rows.entry(4).or_default(); e.custom = true; e.size = Some(25.0); }
    s.execute("home.fontSize", json!({"range": "5:5", "size": 20})).unwrap();
    assert_eq!(s.doc().unwrap().wb.active().unwrap().row_height(4), 25.0, "custom-height rows keep their height");
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p gridcraft-engine --test row_autofit`
Expected: tests 1 and 2 FAIL (height unchanged — the line branch never marks `fit_rows`); test 3 may PASS already (pins existing behavior).

- [ ] **Step 3: Implement the marking** in `crates/engine/src/cmd/format.rs`, in the `if full_rows || full_cols` branch, after the `for (c, st) in cells` loop and before its `continue` (line 244):

```rust
// Formatting changed these cells' fonts: re-fit their rows (Excel auto-fits).
cx.fit_rows.extend(cells.iter().map(|(c, _)| (sheet, c.row)));
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p gridcraft-engine --test row_autofit`
Expected: 3 passed. Also run `cargo test -p gridcraft-engine` (no regressions in style/undo behavior).

- [ ] **Step 5: Commit**

```bash
git add crates/engine/src/cmd/format.rs crates/engine/tests/row_autofit.rs
git commit -m "#833: whole-row/column formatting re-fits row heights"
```

### Task 2: Opened files grow non-custom rows to their tallest font

**Files:**
- Modify: `crates/engine/src/io.rs:57-74` (`open_bytes_with_password`) and add `fn fit_imported_rows` next to it
- Test: `crates/engine/src/tests.rs` (extend, near the existing `file.open` tests around line 1004)

**Interfaces:**
- Consumes: `crate::cmd::auto_row_height(wb: &mut Workbook, si: usize, row: u32)` (pub, cmd/mod.rs:458); `Workbook::sheets: Vec<Arc<Sheet>>` (pub field); `Sheet::cells.iter() -> impl Iterator<Item = (CellRef, &Cell)>` (model store.rs:89); `Sheet::rows: map row -> LineInfo { size: Option<f32>, custom: bool, hidden: bool, .. }`; model builders `Workbook::new()`, `wb.styles.derive(StyleId::default(), |s: &mut Style| s.font.size = 20.0) -> StyleId`, `sh.set_value(CellRef, Value)`, `sh.set_style(CellRef, StyleId)`; `gridcraft_xlsx::write_xlsx(&wb) -> Result<Vec<u8>>`; `crate::io::base64_encode(&[u8])`.
- Produces: `fn fit_imported_rows(wb: &mut Workbook)` — private to `io.rs`, called at both `open_plain` call sites in `open_bytes_with_password`.

- [ ] **Step 1: Write the failing tests and fixture builder** in `crates/engine/src/tests.rs`. Fixture workbooks are built with the model API directly (never through commands, which would auto-fit them) and exported with `write_xlsx`, which omits `ht` for rows without a size — exactly the bug's input:

```rust
fn big_font_workbook(row_info: Option<(f32, bool, bool)>) -> Vec<u8> {
    // row_info = (size, custom, hidden) for row 5 (index 4), None = no row entry at all.
    // Cell A5 = "plain big bold", whole-cell font 20pt. 11pt cell in A1 for contrast rows.
    // Returns write_xlsx bytes. Uses Workbook::new, styles.derive, set_value, set_style.
}

#[test] // A — the issue's repro
fn open_grows_default_height_rows_to_their_font() {
    let mut s = Session::new();
    s.execute("file.open", json!({"name": "bigfont.xlsx", "base64": crate::io::base64_encode(&big_font_workbook(None))})).unwrap();
    let h = s.doc().unwrap().wb.active().unwrap().row_height(4);
    assert!(h >= 30.0, "20pt cell must grow its default-height row on open, got {h}");
    assert_eq!(s.doc().unwrap().wb.active().unwrap().row_height(0), /* default height captured before open */, "11pt row stays default");
}

#[test] // B — Review Focus 1
fn open_keeps_custom_height_rows() {
    // fixture with row_info (25.0, true, false): open → row_height(4) == 25.0
}

#[test] // C — Review Focus 2
fn open_never_shrinks_taller_auto_rows() {
    // fixture with row_info (60.0, false, false) and an 11pt cell: open → row_height(4) == 60.0
}

#[test] // D — Review Focus 5
fn open_fitting_keeps_hidden_rows_hidden() {
    // fixture with row_info (20.0, false, true) and the 20pt cell: open → height grows or keeps 20.0,
    // and rows.get(&4).hidden stays true
}

#[test] // E — Review Focus 4
fn open_leaves_plain_workbooks_untouched() {
    // fixture with only 11pt cells, no row entries: open → sh.rows.get(&0) is None or has size == None
}
```

Comments carry each test's assertions where code is elided; fill them in fully when writing.

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p gridcraft-engine open_`
Expected: A FAILS (row stays default — nothing fits on open); B and C may PASS (pin current behavior); D/E PASS or FAIL depending on current import behavior — record actual results in the commit-free state; only A's failure is required before implementing.

- [ ] **Step 3: Implement `fit_imported_rows` in `crates/engine/src/io.rs` and call it from both `open_plain` sites in `open_bytes_with_password`** (the plain early-return at ~line 58-59 and the decrypted path at ~line 71; keep warnings untouched):

```rust
/// Excel auto-fits rows to their tallest font when opening a file: rows without a user-set
/// height grow to fit their content. Rows the file made taller keep their height — our
/// line-height estimate can differ from Excel's, and shrinking files that already look
/// right would be a visible regression. Custom-height rows are left alone entirely.
fn fit_imported_rows(wb: &mut Workbook) {
    for si in 0..wb.sheets.len() {
        let Some(sh) = wb.sheet(si) else { continue };
        let mut rows: Vec<u32> = sh.cells.iter().map(|(c, _)| c.row).collect();
        rows.sort_unstable();
        rows.dedup();
        for row in rows {
            let before = wb.sheet(si).and_then(|sh| sh.rows.get(&row)).and_then(|i| i.size);
            crate::cmd::auto_row_height(wb, si, row);
            let Some(before) = before else { continue };
            let after = wb.sheet(si).and_then(|sh| sh.rows.get(&row)).and_then(|i| i.size);
            if after.is_none_or(|a| before > a + 0.5) {
                if let Some(sh) = wb.sheet_mut(si) {
                    sh.rows.entry(row).or_default().size = Some(before);
                }
            }
        }
    }
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p gridcraft-engine open_` then `cargo test -p gridcraft-engine` (all), plus a sample sanity check: `cargo test -p gridcraft-engine --test command_sequence_fuzz` (undo/round-trip/recalc invariants over opens).

- [ ] **Step 5: Commit**

```bash
git add crates/engine/src/io.rs crates/engine/src/tests.rs
git commit -m "#833: opened files grow non-custom rows to their tallest font"
```

### Task 3: Docs and tracking

**Files:**
- Modify: `docs/target-app-parity.md:165` (the "Rows and columns … AutoFit" row)
- Modify: `docs/gaps.md` (revision history; check §24 for a stale row-height mention)
- Modify: `ROADMAP.md` (progress log + revision history + "Last updated" line)

**Interfaces:**
- Consumes: merged state of Tasks 1-2.
- Produces: nothing (docs).

- [ ] **Step 1: Update the three docs**
  - `docs/target-app-parity.md:165`: append to the evidence column: `; rows auto-fit oversized fonts on open and row formatting (#833)`.
  - `docs/gaps.md`: add revision-history row `| 2026-10-12 | minor | rows auto-fit oversized fonts on open and whole-row/column formatting (#833) |`.
  - `ROADMAP.md`: progress-log row `| 2026-10-12 | Rows auto-fit oversized fonts: opening a file and whole-row/column formatting grow non-custom rows to their tallest font (#833) |`; matching revision-history row; bump the `**Last updated:**` date.

- [ ] **Step 2: Verify docs build/lint state unchanged**

Run: `cargo xtask ci`
Expected: green (fmt, clippy, tests, assets, layers, wasm).

- [ ] **Step 3: Commit**

```bash
git add docs/target-app-parity.md docs/gaps.md ROADMAP.md
git commit -m "#833: docs — track row auto-fit for oversized fonts"
```

---

**Post-PR (not a plan task):** comment on issue #833 linking the PR and noting the per-run font-size follow-up that lands after rich-text-runs PR #837 merges (per-run sizes are not measured by `auto_row_height` yet).
