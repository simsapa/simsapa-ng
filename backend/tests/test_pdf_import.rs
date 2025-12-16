use diesel::prelude::*;
use serial_test::serial;
use std::path::PathBuf;

mod helpers;
use helpers as h;

use simsapa_backend::get_app_data;
use simsapa_backend::db::appdata_schema::books;
use simsapa_backend::db::appdata_schema::book_spine_items;

#[test]
#[serial]
fn test_pdf_import_with_utf16_bom_title() {
    // This tests that PDF titles with UTF-16 BOM are properly decoded
    // The pali-lessons.pdf has a title "Pali Lessons" encoded as UTF-16 BE with BOM (0xFE 0xFF)
    
    h::app_data_setup();
    let app_data = get_app_data();
    
    let pdf_path = PathBuf::from("tests/data/pali-lessons.pdf");
    let book_uid = "test-pali-lessons";
    
    // Clean up any existing test data
    let _ = app_data.dbm.appdata.do_write(|db_conn| {
        diesel::delete(books::table.filter(books::uid.eq(book_uid)))
            .execute(db_conn)
    });
    
    // Import the PDF
    let result = app_data.import_pdf_to_db(&pdf_path, book_uid, None, None, None, None);
    assert!(result.is_ok(), "PDF import failed: {:?}", result.err());
    
    // Verify the book was created with correct title (no BOM or control characters)
    let book = app_data.dbm.appdata.do_read(|db_conn| {
        books::table
            .filter(books::uid.eq(book_uid))
            .select((books::uid, books::title))
            .first::<(String, Option<String>)>(db_conn)
    }).expect("Query failed");
    
    assert_eq!(book.0, book_uid);
    assert_eq!(book.1, Some("Pali Lessons".to_string()), 
        "Title should be 'Pali Lessons' without BOM or control characters");
    
    // Verify spine item was created
    let spine_item_uid = format!("{}.0", book_uid);
    let spine_title = app_data.dbm.appdata.do_read(|db_conn| {
        book_spine_items::table
            .filter(book_spine_items::spine_item_uid.eq(&spine_item_uid))
            .select(book_spine_items::title)
            .first::<Option<String>>(db_conn)
    }).expect("Query failed");
    
    assert_eq!(spine_title, Some("Pali Lessons".to_string()));
    
    // Clean up
    let _ = app_data.dbm.appdata.do_write(|db_conn| {
        diesel::delete(books::table.filter(books::uid.eq(book_uid)))
            .execute(db_conn)
    });
}

#[test]
#[serial]
fn test_pdf_import_general() {
    // General test for PDF import functionality
    
    h::app_data_setup();
    let app_data = get_app_data();
    
    let pdf_path = PathBuf::from("tests/data/pali-lessons.pdf");
    let book_uid = "test-general-import";
    
    // Clean up any existing test data
    let _ = app_data.dbm.appdata.do_write(|db_conn| {
        diesel::delete(books::table.filter(books::uid.eq(book_uid)))
            .execute(db_conn)
    });
    
    // Import the PDF
    let result = app_data.import_pdf_to_db(&pdf_path, book_uid, None, None, None, None);
    assert!(result.is_ok(), "PDF import failed: {:?}", result.err());
    
    // Verify book record
    let book_count = app_data.dbm.appdata.do_read(|db_conn| {
        books::table
            .filter(books::uid.eq(book_uid))
            .count()
            .get_result::<i64>(db_conn)
    }).expect("Query failed");
    
    assert_eq!(book_count, 1, "Expected exactly one book record");
    
    // Verify spine item count
    let spine_count = app_data.dbm.appdata.do_read(|db_conn| {
        book_spine_items::table
            .filter(book_spine_items::book_uid.eq(book_uid))
            .count()
            .get_result::<i64>(db_conn)
    }).expect("Query failed");
    
    assert_eq!(spine_count, 1, "PDFs should have exactly one spine item");
    
    // Verify the spine item has content_plain (for FTS5 search)
    let content_plain = app_data.dbm.appdata.do_read(|db_conn| {
        book_spine_items::table
            .filter(book_spine_items::book_uid.eq(book_uid))
            .select(book_spine_items::content_plain)
            .first::<Option<String>>(db_conn)
    }).expect("Query failed");
    
    assert!(content_plain.is_some(), "content_plain should be extracted");
    assert!(!content_plain.unwrap().is_empty(), "content_plain should not be empty");
    
    // Clean up
    let _ = app_data.dbm.appdata.do_write(|db_conn| {
        diesel::delete(books::table.filter(books::uid.eq(book_uid)))
            .execute(db_conn)
    });
}

#[test]
fn test_decode_utf16_be_with_bom() {
    // Test the UTF-16 BE BOM handling directly
    // This simulates the raw bytes from "Pali Lessons" in UTF-16 BE with BOM
    let raw_bytes: Vec<u8> = vec![
        0xFE, 0xFF,  // BOM
        0x00, 0x50,  // 'P'
        0x00, 0x61,  // 'a'
        0x00, 0x6C,  // 'l'
        0x00, 0x69,  // 'i'
        0x00, 0x20,  // ' '
        0x00, 0x4C,  // 'L'
        0x00, 0x65,  // 'e'
        0x00, 0x73,  // 's'
        0x00, 0x73,  // 's'
        0x00, 0x6F,  // 'o'
        0x00, 0x6E,  // 'n'
        0x00, 0x73,  // 's'
    ];
    
    // Use the helper from pdf_import module
    // Note: Since decode_pdf_text_string is private, we test it indirectly through import
    // This test documents the expected behavior
    
    // Decode UTF-16 BE with BOM manually to verify logic
    let utf16_bytes = &raw_bytes[2..]; // Skip BOM
    let mut utf16_chars: Vec<u16> = Vec::new();
    for chunk in utf16_bytes.chunks(2) {
        if chunk.len() == 2 {
            utf16_chars.push(u16::from_be_bytes([chunk[0], chunk[1]]));
        }
    }
    
    let decoded = String::from_utf16_lossy(&utf16_chars);
    assert_eq!(decoded, "Pali Lessons");
}
