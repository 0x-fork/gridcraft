//! The keytip sequence table: every Excel-style Alt access sequence and what it does.
//!
//! This is the single source of truth for sequential access keys. Both the matcher in
//! [`super::KeyTips::process`] and the badge painter in `ribbon.rs` read from here, so a
//! sequence is added in exactly one place.
//!
//! Regenerated (T5) from `plan/keytips-map-results.json` — the Excel-sequence collection and
//! catalog join. Do not hand-edit entries without updating that source data.

use super::Action;

/// What a fully-matched sequence does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Entry {
    /// Switch to this ribbon tab and keep the prefix: the tab's own badges take over.
    Tab(&'static str),
    /// Keep the prefix: this sequence is a group-level badge; deeper sequences finish it.
    Continue,
    /// Complete the sequence: run the action and clear the prefix.
    Finish(Action),
}

/// All known sequences, in no particular order (lookup is linear; sizes are small).
pub const SEQUENCES: &[(&str, Entry)] = &[
    // Ribbon tabs. Legacy Edit/Format paths (E/O/OC) lead to the same Home menus.
    ("H", Entry::Tab("Home")),
    ("N", Entry::Tab("Insert")),
    ("P", Entry::Tab("Page Layout")),
    ("M", Entry::Tab("Formulas")),
    ("A", Entry::Tab("Data")),
    ("R", Entry::Tab("Review")),
    ("W", Entry::Tab("View")),
    // Draw is a two-letter unit in Excel: the badge shows "JI" as one keytip, and the
    // matcher waits after "J" (implicit level — no table entry needed).
    ("JI", Entry::Tab("Draw")),
    ("E", Entry::Tab("Home")),
    ("O", Entry::Tab("Home")),
    // Home tab groups.
    ("HA", Entry::Continue),
    ("HV", Entry::Continue),
    ("HO", Entry::Continue),
    ("OC", Entry::Continue),
    // Home > Alignment.
    ("HAR", Entry::Finish(Action::Command("home.alignRight"))),
    ("HAL", Entry::Finish(Action::Command("home.alignLeft"))),
    ("HAC", Entry::Finish(Action::Command("home.alignCenter"))),
    // Paste Special — ribbon and legacy routes.
    ("HVS", Entry::Finish(Action::Dialog("pasteSpecial"))),
    ("ES", Entry::Finish(Action::Dialog("pasteSpecial"))),
    // AutoFit column width — ribbon and legacy routes.
    ("HOI", Entry::Finish(Action::Command("home.autofitColumnWidth"))),
    ("OCA", Entry::Finish(Action::Command("home.autofitColumnWidth"))),
];

/// Look up a complete sequence.
pub fn lookup(sequence: &str) -> Option<Entry> {
    SEQUENCES.iter().find(|(seq, _)| *seq == sequence).map(|(_, entry)| *entry)
}

/// True if `prefix` is a strict prefix of any known sequence (badges wait for more keys).
pub fn has_deeper(prefix: &str) -> bool {
    SEQUENCES.iter().any(|(seq, _)| seq.len() > prefix.len() && seq.starts_with(prefix))
}

/// The sequence that switches to `tab_name`, if that tab has one (may be a multi-letter unit
/// like Draw's "JI").
pub fn tab_sequence(tab_name: &str) -> Option<&'static str> {
    SEQUENCES.iter().find_map(|(seq, entry)| match entry {
        Entry::Tab(name) if *name == tab_name => Some(*seq),
        _ => None,
    })
}

/// True if `seq` is a group level: an explicit Continue entry, or an implicit level that only
/// exists because deeper sequences start with it (like "J" before Draw's "JI").
pub fn is_group_level(seq: &str) -> bool {
    matches!(lookup(seq), Some(Entry::Continue)) || (lookup(seq).is_none() && has_deeper(seq))
}

fn matches_target(action: &Action, target: &str) -> bool {
    match action {
        Action::Command(id) => *id == target,
        Action::Dialog(name) => format!("dialog:{name}") == target,
        Action::Tab(_) => false,
    }
}

/// All sequences that finish by running `target` (a command id or `dialog:<name>`).
pub fn sequences_for(target: &str) -> Vec<&'static str> {
    SEQUENCES
        .iter()
        .filter_map(|(seq, entry)| match entry {
            Entry::Finish(action) if matches_target(action, target) => Some(*seq),
            _ => None,
        })
        .collect()
}

/// Group-level letters for a menu anchor: sequences one key past `prefix` that continue toward
/// any of `targets` (the menu's items) — the badge shown on the button that opens the menu.
pub fn group_extension(prefix: &str, targets: &[&str]) -> Vec<&'static str> {
    let mut out = Vec::new();
    for target in targets {
        for seq in sequences_for(target) {
            if seq.len() <= prefix.len() + 1 || !seq.starts_with(prefix) {
                continue;
            }
            // The intermediate one key past the prefix is the group badge, if it is a
            // group-level sequence (explicit or implicit) rather than the finish itself.
            let candidate = &seq[..prefix.len() + 1];
            if is_group_level(candidate) && !out.contains(&candidate) {
                out.push(candidate);
            }
        }
    }
    out
}

/// The badge for a menu item: its finish sequence exactly one key below the current prefix.
pub fn item_letter(prefix: &str, target: &str) -> Option<&'static str> {
    sequences_for(target).into_iter().find(|seq| seq.len() == prefix.len() + 1 && seq.starts_with(prefix))
}

/// Badge sequences for a command control: every sequence for `target` that continues from the
/// current prefix (the painter strips the prefix, so the remaining letters show).
pub fn control_badges(prefix: &str, target: &str) -> Vec<&'static str> {
    if prefix.is_empty() {
        return Vec::new(); // empty prefix shows only tab badges
    }
    sequences_for(target).into_iter().filter(|seq| seq.len() > prefix.len() && seq.starts_with(prefix)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tab_letters_exist_for_standard_tabs() {
        for (tab, letter) in [
            ("Home", "H"),
            ("Insert", "N"),
            ("Page Layout", "P"),
            ("Formulas", "M"),
            ("Data", "A"),
            ("Review", "R"),
            ("View", "W"),
            ("Draw", "JI"), // two-letter unit, shown as one badge
        ] {
            assert_eq!(tab_sequence(tab), Some(letter), "tab {tab}");
        }
        assert_eq!(tab_sequence("Automate"), None);
    }

    #[test]
    fn sequences_for_finds_command_and_dialog_targets() {
        assert_eq!(sequences_for("home.alignRight"), vec!["HAR"]);
        assert_eq!(sequences_for("dialog:pasteSpecial"), vec!["HVS", "ES"]);
        assert_eq!(sequences_for("home.autofitColumnWidth"), vec!["HOI", "OCA"]);
    }

    #[test]
    fn implicit_levels_wait_for_deeper_sequences() {
        assert!(has_deeper("J"), "JI is reachable only through the implicit J level");
        assert!(is_group_level("J"), "implicit levels are group levels");
        assert!(is_group_level("HV"), "explicit Continue levels are group levels");
        assert!(!is_group_level("H"), "a terminal tab is not a group level");
        assert!(!is_group_level("HVS"), "a finish is not a group level");
        assert!(!has_deeper("X"));
    }

    #[test]
    fn group_extension_paints_only_continue_levels() {
        assert_eq!(group_extension("H", &["dialog:pasteSpecial"]), vec!["HV"]);
        assert!(group_extension("", &["dialog:pasteSpecial"]).is_empty(), "tab letters paint on tabs, not buttons");
        assert!(group_extension("E", &["dialog:pasteSpecial"]).is_empty(), "the finish letter paints on the menu item");
    }

    #[test]
    fn item_letter_is_one_level_deep() {
        assert_eq!(item_letter("HV", "dialog:pasteSpecial"), Some("HVS"));
        assert_eq!(item_letter("E", "dialog:pasteSpecial"), Some("ES"));
        assert_eq!(item_letter("H", "dialog:pasteSpecial"), None, "the item badge waits for its group letter");
        assert_eq!(item_letter("HO", "home.autofitColumnWidth"), Some("HOI"));
    }

    #[test]
    fn control_badges_require_a_known_prefix() {
        assert!(control_badges("", "home.alignRight").is_empty(), "empty prefix shows only tab badges");
        assert_eq!(control_badges("HA", "home.alignRight"), vec!["HAR"]);
        assert!(control_badges("HZ", "home.alignRight").is_empty(), "unknown prefixes show nothing");
    }
}
