//! The Excel formula language: tokenizer, parser, syntax tree, printer (A1 and R1C1) and
//! reference adjustment for copy/fill and structural edits.
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]
#![forbid(unsafe_code)]

pub mod adjust;
pub mod ast;
mod input;
pub mod lexer;
pub mod parser;
pub mod printer;

pub use ast::{Anchor, BinOp, Expr, RefKind, Reference, SheetSel, StructItem, StructRef, TrimMode, UnOp};
pub use input::{FormulaLocale, parse_input, print_input};
pub use parser::{MAX_FORMULA_LEN, ParseError, normalize_function_name, parse};
pub use printer::{print, print_r1c1, quote_sheet};

/// Whether two LET/LAMBDA names are the same: case-insensitive in any script, as in Excel
/// (`dług` = `DŁUG`, `ß` = `ẞ`). Each character is lower-cased, then upper-cased, so letters whose
/// upper case is not one character (`ß` -> `SS`) still match their capital.
pub fn same_name(a: &str, b: &str) -> bool {
    let fold = |s: &str| s.chars().flat_map(char::to_lowercase).flat_map(char::to_uppercase).collect::<String>();
    a == b || fold(a) == fold(b)
}

#[cfg(test)]
mod tests;
