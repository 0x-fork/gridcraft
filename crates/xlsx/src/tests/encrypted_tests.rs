//! Encrypted workbooks end to end: compound file → streams → plain package.

use crate::testing::{compound_file, encrypt_workbook, encrypt_workbook_standard};
use crate::{DecryptError, decrypt};

#[test]
fn the_test_compound_writer_reads_back() {
    let big: Vec<u8> = (0..3_000_000u32).map(|i| (i % 253) as u8).collect();
    let mid: Vec<u8> = (0..4095u32).map(|i| i as u8).collect();
    let file = compound_file(&[("Small", b"hello"), ("Big", &big), ("Empty", b""), ("Mid", &mid)]);
    let cf = crate::cfb::Compound::open(&file).unwrap();
    assert_eq!(cf.root_stream("Mid").unwrap().unwrap(), mid, "just under the cutoff: the mini stream");
    assert_eq!(cf.root_stream("Small").unwrap().unwrap(), b"hello");
    assert_eq!(cf.root_stream("big").unwrap().unwrap(), big);
    assert_eq!(cf.root_stream("Empty").unwrap().unwrap(), b"");
    assert_eq!(cf.root_stream("Missing").unwrap(), None);
}

#[test]
fn encrypted_files_decrypt_to_the_package() {
    let package: Vec<u8> = (0..3_000_000u32).map(|i| (i * 31 % 251) as u8).collect();
    let agile = encrypt_workbook(&package, "Pass1234");
    assert!(crate::cfb::is_encrypted(&agile));
    assert_eq!(decrypt(&agile, "Pass1234").unwrap(), package);
    assert_eq!(decrypt(&agile, "wrong"), Err(DecryptError::WrongPassword));
    let standard = encrypt_workbook_standard(&package, "Pass1234");
    assert_eq!(decrypt(&standard, "Pass1234").unwrap(), package);
}

#[test]
fn a_compound_file_without_the_streams_is_malformed() {
    let file = compound_file(&[("EncryptionInfo", &[4, 0, 4, 0])]);
    assert!(matches!(decrypt(&file, "x"), Err(DecryptError::Malformed(_))));
    assert!(matches!(decrypt(b"not a compound file", "x"), Err(DecryptError::Malformed(_))));
}

#[test]
fn a_gridcraft_workbook_survives_encryption() {
    let b2 = gridcraft_core::CellRef::parse("B2").unwrap();
    let mut wb = gridcraft_model::Workbook::new();
    wb.sheet_mut(0).unwrap().set_value(b2, gridcraft_core::Value::Number(42.0));
    let xlsx = crate::write_xlsx(&wb).unwrap();
    let file = encrypt_workbook(&xlsx, "Pass1234");
    assert_eq!(crate::sniff(&file), crate::Format::Encrypted);
    let (back, _) = crate::read_xlsx(&decrypt(&file, "Pass1234").unwrap()).unwrap();
    assert_eq!(back.active().unwrap().value(b2), gridcraft_core::Value::Number(42.0));
}
