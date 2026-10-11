//! GENERATED from keytips.json by `cargo xtask keytips`. Edit the JSON, not this file.

use super::{Action, Entry};

/// All known keytip sequences. Intermediate levels are implicit: the matcher waits
/// whenever a pressed letter starts a deeper sequence, so only terminal entries are
/// listed here. See keytip_table.rs for the helpers that read this table.
pub const SEQUENCES: &[(&str, Entry)] = &[
    ("H", Entry::Tab("Home")),
    ("N", Entry::Tab("Insert")),
    ("JI", Entry::Tab("Draw")), // Two-letter unit. Deeper Draw letters differ between Excel and GridCraft; left blank for future contributors.
    ("P", Entry::Tab("Page Layout")),
    ("M", Entry::Tab("Formulas")),
    ("A", Entry::Tab("Data")),
    ("R", Entry::Tab("Review")),
    ("W", Entry::Tab("View")),
    ("E", Entry::Tab("Home")), // Legacy Edit-menu path (Excel 2003); leads to the same Home menus.
    ("O", Entry::Tab("Home")), // Legacy Format-menu path (Excel 2003); leads to the same Home menus.
    ("HAR", Entry::Finish(Action::Command("home.alignRight"))),
    ("HAL", Entry::Finish(Action::Command("home.alignLeft"))),
    ("HAC", Entry::Finish(Action::Command("home.alignCenter"))),
    ("HVS", Entry::Finish(Action::Dialog("pasteSpecial"))),
    ("ES", Entry::Finish(Action::Dialog("pasteSpecial"))), // Legacy Edit menu.
    ("HOI", Entry::Finish(Action::Command("home.autofitColumnWidth"))),
    ("OCA", Entry::Finish(Action::Command("home.autofitColumnWidth"))), // Legacy Format menu.
    ("WVG", Entry::Finish(Action::Command("view.gridlines"))),
];
