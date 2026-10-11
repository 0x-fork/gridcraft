//! Regenerate the keytip sequence table from `keytips.json`.
//!
//! `cargo xtask keytips`          regenerate crates/ui-egui/src/keytips/generated.rs
//! `cargo xtask keytips --check`  fail if the generated file differs from the JSON source

use std::fs;
use std::path::Path;

const TABS: &[&str] = &["Home", "Insert", "Draw", "Page Layout", "Formulas", "Data", "Review", "View", "Automate"];

/// Format generated source through `rustfmt` so the checked-in file matches
/// the emitter byte-for-byte.
fn rustfmt(src: &str) -> Result<String, String> {
    use std::io::Write;
    use std::process::{Command, Stdio};
    let mut child = Command::new("rustfmt")
        .args(["--edition", "2024", "--emit", "stdout"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("spawn rustfmt: {e}"))?;
    let stdin = child.stdin.as_mut().ok_or("rustfmt: failed to open stdin")?;
    stdin.write_all(src.as_bytes()).map_err(|e| format!("write rustfmt stdin: {e}"))?;
    let out = child.wait_with_output().map_err(|e| format!("wait rustfmt: {e}"))?;
    if !out.status.success() {
        return Err(format!("rustfmt failed: {}", String::from_utf8_lossy(&out.stderr)));
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

pub fn run(root: &Path, rest: &[&str]) -> Result<(), String> {
    let json_path = root.join("crates/ui-egui/src/keytips/keytips.json");
    let out_path = root.join("crates/ui-egui/src/keytips/keytip_table/generated.rs");
    let raw = generate(&fs::read_to_string(&json_path).map_err(|e| format!("read {}: {e}", json_path.display()))?)?;
    // The table is checked into the tree formatted, so emit through rustfmt:
    // otherwise `--check` always fails against the fmt'd file.
    let generated = rustfmt(&raw)?;
    if rest.contains(&"--check") {
        let on_disk = fs::read_to_string(&out_path).map_err(|e| format!("read {}: {e}", out_path.display()))?;
        if on_disk != generated {
            return Err(format!("{} is out of date — run `cargo xtask keytips` and commit the result", out_path.display()));
        }
        println!("keytips: {} up to date", out_path.display());
        return Ok(());
    }
    fs::write(&out_path, &generated).map_err(|e| format!("write {}: {e}", out_path.display()))?;
    println!("keytips: wrote {}", out_path.display());
    Ok(())
}

pub fn generate(source: &str) -> Result<String, String> {
    let doc: serde_json::Value = serde_json::from_str(source).map_err(|e| format!("invalid JSON: {e}"))?;
    let seqs = doc.get("sequences").and_then(|v| v.as_array()).ok_or("missing `sequences` array")?;

    let mut rows: Vec<(String, String, String)> = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for (i, entry) in seqs.iter().enumerate() {
        let at = format!("entry {}", i + 1);
        let seq = entry.get("seq").and_then(|v| v.as_str()).ok_or_else(|| format!("{at}: missing `seq`"))?;
        if seq.is_empty() || seq.len() > 6 || !seq.bytes().all(|b| b.is_ascii_alphanumeric()) {
            return Err(format!("{at}: seq {seq:?} must be 1-6 ASCII letters/digits (Excel's longest are 5, e.g. sparkline paths)"));
        }
        if !seen.insert(seq.to_string()) {
            return Err(format!("{at}: duplicate seq {seq:?}"));
        }
        let kind = entry.get("kind").and_then(|v| v.as_str()).ok_or_else(|| format!("{at}: missing `kind`"))?;
        let target = entry.get("target").and_then(|v| v.as_str()).unwrap_or("");
        let pending = entry.get("pending").and_then(|v| v.as_bool()).unwrap_or(false);
        let note = entry.get("note").and_then(|v| v.as_str()).unwrap_or("");
        let excel = entry.get("excel").and_then(|v| v.as_str()).unwrap_or("");
        if pending && kind != "command" {
            return Err(format!("{at}: `pending` is only valid with kind `command`"));
        }
        let value = match kind {
            "tab" => {
                if target.is_empty() || !TABS.contains(&target) {
                    return Err(format!("{at}: tab target {target:?} is not one of {TABS:?}"));
                }
                format!("Entry::Tab({target:?})")
            }
            "command" => {
                if target.is_empty() {
                    return Err(format!("{at}: command needs a `target` command id"));
                }
                if pending { format!("Entry::Finish(Action::Pending({target:?}))") } else { format!("Entry::Finish(Action::Command({target:?}))") }
            }
            "dialog" => {
                if target.is_empty() {
                    return Err(format!("{at}: dialog needs a `target` dialog name"));
                }
                format!("Entry::Finish(Action::Dialog({target:?}))")
            }
            "continue" => "Entry::Continue".to_string(),
            other => return Err(format!("{at}: unknown kind {other:?}")),
        };
        let mut comment = String::new();
        if pending {
            comment.push_str(" pending");
        }
        if !excel.is_empty() {
            if !comment.is_empty() {
                comment.push(';');
            }
            comment.push_str(&format!(" Excel: {excel}"));
        }
        if !note.is_empty() {
            if !comment.is_empty() {
                comment.push(';');
            }
            comment.push_str(&format!(" {note}"));
        }
        rows.push((seq.to_string(), value, comment));
    }
    // JSON order is authoritative and curated: the first entry for a tab is its canonical
    // badge (legacy aliases come later), and groups stay together. Do not sort.

    let mut out = String::from(
        "//! GENERATED from keytips.json by `cargo xtask keytips`. Edit the JSON, not this file.\n\n\
         use super::{Action, Entry};\n\n\
         /// All known keytip sequences. Intermediate levels are implicit: the matcher waits\n\
         /// whenever a pressed letter starts a deeper sequence, so only terminal entries are\n\
         /// listed here. See keytip_table.rs for the helpers that read this table.\n\
         pub const SEQUENCES: &[(&str, Entry)] = &[\n",
    );
    for (seq, value, comment) in &rows {
        out.push_str(&format!("    ({seq:?}, {value}),"));
        if !comment.is_empty() {
            out.push_str(&format!(" //{comment}"));
        }
        out.push('\n');
    }
    out.push_str("];\n");
    Ok(out)
}
