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
fn test_epub_spine_item_titles() {
    // Test that EPUB spine items get correct titles from TOC or HTML title tags
    // Expected titles for BuddhistMonasticCode_251013.epub:
    // Spine items are actual files in reading order, not TOC entries
    // 1. Cover (Cover.html - from HTML title tag, no TOC entry)
    // 2. BMC I: The Pāṭimokkha Rules (Section0001.html - from TOC, overrides HTML "Titlepage")
    // 3. Copyright (Section0002.html - from TOC)
    // 4. Quote (Section0003.html - from HTML title tag, no TOC entry)
    // 5. Abbreviations (Section0004.html - from TOC)
    // 6. Preface (Section0005.html - from TOC)
    // 7. Dhamma-Vinaya (Section0006.html - from TOC)
    // 8. Pāṭimokkha (Section0007.html - from TOC)
    // 9. Nissaya (Section0008.html - from TOC)

    h::app_data_setup();
    let app_data = get_app_data();

    let epub_path = PathBuf::from("tests/data/BuddhistMonasticCode_251013.epub");
    let book_uid = "test-bmc";

    // Clean up any existing test data
    let _ = app_data.dbm.appdata.do_write(|db_conn| {
        diesel::delete(books::table.filter(books::uid.eq(book_uid)))
            .execute(db_conn)
    });

    // Import the EPUB
    let result = app_data.import_epub_to_db(&epub_path, book_uid);
    assert!(result.is_ok(), "EPUB import failed: {:?}", result.err());

    // Verify the first 10 spine item titles
    let spine_items: Vec<(i32, Option<String>)> = app_data.dbm.appdata.do_read(|db_conn| {
        book_spine_items::table
            .filter(book_spine_items::book_uid.eq(book_uid))
            .select((book_spine_items::spine_index, book_spine_items::title))
            .order(book_spine_items::spine_index.asc())
            .limit(10)
            .load::<(i32, Option<String>)>(db_conn)
    }).expect("Query failed");

    let expected_titles = vec![
        "Cover",
        "BMC I: The Pāṭimokkha Rules",
        "Copyright",
        "Quote",
        "Abbreviations",
        "Preface",
        "Dhamma-Vinaya",
        "Pāṭimokkha",
        "Nissaya",
        "Disrobing",
    ];

    for (i, (spine_index, title)) in spine_items.iter().enumerate() {
        let expected = expected_titles[i];
        let actual = title.as_deref().unwrap_or("");
        assert_eq!(
            actual, expected,
            "Spine index {} (file index {}): expected '{}', got '{}'",
            spine_index, i, expected, actual
        );
    }

    // Clean up
    let _ = app_data.dbm.appdata.do_write(|db_conn| {
        diesel::delete(books::table.filter(books::uid.eq(book_uid)))
            .execute(db_conn)
    });
}

#[test]
#[serial]
fn test_epub_import_general() {
    // General test for EPUB import functionality

    h::app_data_setup();
    let app_data = get_app_data();

    let epub_path = PathBuf::from("tests/data/BuddhistMonasticCode_251013.epub");
    let book_uid = "test-general-epub-import";

    // Clean up any existing test data
    let _ = app_data.dbm.appdata.do_write(|db_conn| {
        diesel::delete(books::table.filter(books::uid.eq(book_uid)))
            .execute(db_conn)
    });

    // Import the EPUB
    let result = app_data.import_epub_to_db(&epub_path, book_uid);
    assert!(result.is_ok(), "EPUB import failed: {:?}", result.err());

    // Verify book record
    let book_count = app_data.dbm.appdata.do_read(|db_conn| {
        books::table
            .filter(books::uid.eq(book_uid))
            .count()
            .get_result::<i64>(db_conn)
    }).expect("Query failed");

    assert_eq!(book_count, 1, "Expected exactly one book record");

    // Verify spine items exist
    let spine_count = app_data.dbm.appdata.do_read(|db_conn| {
        book_spine_items::table
            .filter(book_spine_items::book_uid.eq(book_uid))
            .count()
            .get_result::<i64>(db_conn)
    }).expect("Query failed");

    assert!(spine_count > 0, "EPUB should have spine items");

    // Verify spine items have content
    let first_spine_item = app_data.dbm.appdata.do_read(|db_conn| {
        book_spine_items::table
            .filter(book_spine_items::book_uid.eq(book_uid))
            .select((book_spine_items::content_html, book_spine_items::content_plain))
            .first::<(Option<String>, Option<String>)>(db_conn)
    }).expect("Query failed");

    assert!(first_spine_item.0.is_some(), "content_html should be extracted");
    assert!(first_spine_item.1.is_some(), "content_plain should be extracted");

    // Clean up
    let _ = app_data.dbm.appdata.do_write(|db_conn| {
        diesel::delete(books::table.filter(books::uid.eq(book_uid)))
            .execute(db_conn)
    });
}

#[test]
#[serial]
fn test_epub_html_escape_conversion() {
    // Test that HTML escape codes in spine item titles are converted to Unicode
    // For its-essential-meaning.epub, the first spine item should have title "The Buddha's Teaching"
    // instead of "The Buddha&#8217;s Teaching"

    h::app_data_setup();
    let app_data = get_app_data();

    let epub_path = PathBuf::from("tests/data/its-essential-meaning.epub");
    let book_uid = "test-its-essential-meaning";

    // Clean up any existing test data
    let _ = app_data.dbm.appdata.do_write(|db_conn| {
        diesel::delete(books::table.filter(books::uid.eq(book_uid)))
            .execute(db_conn)
    });

    // Import the EPUB
    let result = app_data.import_epub_to_db(&epub_path, book_uid);
    assert!(result.is_ok(), "EPUB import failed: {:?}", result.err());

    // Get the first spine item title
    let first_spine_item: (i32, Option<String>) = app_data.dbm.appdata.do_read(|db_conn| {
        book_spine_items::table
            .filter(book_spine_items::book_uid.eq(book_uid))
            .select((book_spine_items::spine_index, book_spine_items::title))
            .order(book_spine_items::spine_index.asc())
            .first::<(i32, Option<String>)>(db_conn)
    }).expect("Query failed");

    let actual_title = first_spine_item.1.as_deref().unwrap_or("");

    // The title should be "The Buddha's Teaching" with proper Unicode apostrophe (U+2019), not "The Buddha&#8217;s Teaching"
    let expected_title = format!("The Buddha{}s Teaching", '\u{2019}');
    assert_eq!(
        actual_title, expected_title,
        "Expected first spine item title to be '{}' with proper Unicode apostrophe (U+2019), got '{}'",
        expected_title, actual_title
    );

    // Verify that the title doesn't contain HTML escape codes
    assert!(!actual_title.contains("&#8217;"), "Title should not contain HTML escape codes");
    assert!(!actual_title.contains("&amp;"), "Title should not contain HTML escape codes");
    assert!(!actual_title.contains("&lt;"), "Title should not contain HTML escape codes");
    assert!(!actual_title.contains("&gt;"), "Title should not contain HTML escape codes");
    assert!(!actual_title.contains("&quot;"), "Title should not contain HTML escape codes");

    // Clean up
    let _ = app_data.dbm.appdata.do_write(|db_conn| {
        diesel::delete(books::table.filter(books::uid.eq(book_uid)))
            .execute(db_conn)
    });
}
