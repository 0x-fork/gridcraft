//! `[0]!Name` ("this workbook") in chart references: read as the bare name, written back for
//! workbook-scoped names only.

use std::io::Read;
use std::sync::Arc;

use gridcraft_model::{Anchor, Chart, ChartKind, DefinedName, LegendPos, Series, Sheet, Workbook};

use crate::chart::chart_ref_from_file;
use crate::{read_xlsx, write_xlsx};

#[test]
fn this_book_prefix_is_dropped_only_in_front_of_names() {
    for (file, model) in [
        ("[0]!TaskNames", "TaskNames"),
        ("=[0]!TaskNames", "TaskNames"),
        ("  [0]!_Hidden ", "_Hidden"),
        ("([0]!a,[0]!b)", "(a,b)"),
        // External books keep their index: they go through externalLink parts.
        ("[1]Sheet1!$A$1:$A$5", "[1]Sheet1!$A$1:$A$5"),
        ("[2]!Rates", "[2]!Rates"),
        // Not a name after the prefix, or not at a name's start.
        ("[0]!$A$1", "[0]!$A$1"),
        ("Plan[[0]]", "Plan[[0]]"),
        // Inside a quoted sheet name or a string.
        ("'[0]!x'!$A$1", "'[0]!x'!$A$1"),
        ("\"[0]!x\"", "\"[0]!x\""),
        ("'It''s [0]!'!$A$1", "'It''s [0]!'!$A$1"),
        ("Sheet1!$B$2:$B$9", "Sheet1!$B$2:$B$9"),
        ("", ""),
        ("[0]!", "[0]!"),
        ("[0]!é", "é"),
    ] {
        assert_eq!(chart_ref_from_file(file), model, "{file}");
    }
}

fn series(name: &str, values: &str) -> Series {
    Series {
        name: Some(name.into()),
        categories: Some("Items".into()),
        values: values.into(),
        bubble_sizes: None,
        color: None,
        secondary: false,
        kind: None,
        smooth: false,
        trendlines: Vec::new(),
    }
}

fn name(n: &str, scope: Option<usize>, formula: &str) -> DefinedName {
    DefinedName { name: n.into(), scope, formula: formula.into(), comment: String::new(), hidden: false }
}

#[test]
fn workbook_names_are_written_with_the_prefix_and_sheet_names_without() {
    let mut wb = Workbook::default();
    let mut s = Sheet::new("Data");
    s.charts.push(Chart {
        id: 1,
        kind: ChartKind::BarStacked,
        anchor: Anchor { cell: gridcraft_core::CellRef::new(0, 3), width: 480.0, height: 300.0, ..Anchor::default() },
        title: None,
        series: vec![series("Title", "Amounts"), series("Data!$B$1", "Local"), series("\"Plain\"", "Data!$B$2:$B$4")],
        legend: LegendPos::Bottom,
        data_labels: false,
        gridlines: true,
        style: 2,
        x_title: None,
        y_title: None,
        source: None,
        by_rows: false,
        pivot: None,
    });
    wb.sheets = vec![Arc::new(s)];
    wb.names = vec![
        name("Items", None, "Data!$A$2:$A$4"),
        name("Amounts", None, "OFFSET(Items,0,1)"),
        name("Title", None, "Data!$B$1"),
        name("Local", Some(0), "Data!$C$2:$C$4"),
    ];
    let bytes = write_xlsx(&wb).unwrap();
    let mut xml = String::new();
    zip::ZipArchive::new(std::io::Cursor::new(&bytes)).unwrap().by_name("xl/charts/chart1.xml").unwrap().read_to_string(&mut xml).unwrap();
    assert!(xml.contains("<c:f>[0]!Items</c:f>"), "{xml}");
    assert!(xml.contains("<c:f>[0]!Amounts</c:f>"), "{xml}");
    assert!(xml.contains("<c:tx><c:strRef><c:f>[0]!Title</c:f>"), "{xml}");
    // A sheet-scoped name is not this workbook's name: no prefix.
    assert!(xml.contains("<c:f>Local</c:f>"), "{xml}");
    assert!(xml.contains("<c:tx><c:v>Plain</c:v></c:tx>"), "{xml}");

    let (back, _) = read_xlsx(&bytes).unwrap();
    let got: Vec<(Option<&str>, Option<&str>, &str)> =
        back.sheets[0].charts[0].series.iter().map(|s| (s.name.as_deref(), s.categories.as_deref(), s.values.as_str())).collect();
    assert_eq!(
        got,
        [(Some("Title"), Some("Items"), "Amounts"), (Some("Data!$B$1"), Some("Items"), "Local"), (Some("Plain"), Some("Items"), "Data!$B$2:$B$4"),]
    );
}
