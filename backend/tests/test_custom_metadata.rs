use diesel::prelude::*;
use serial_test::serial;
use std::path::Path;

mod helpers;
use helpers as h;

use simsapa_backend::get_app_data;
use simsapa_backend::db::appdata_schema::books;

#[test]
#[serial]
fn test_epub_custom_title_author_override() {
    h::app_data_setup();
    let app_data = get_app_data();
    let book_uid = "test-epub-custom-metadata";

    // Clean up any existing test data
    app_data.dbm.appdata.do_write(|db_conn| {
        diesel::delete(books::table.filter(books::uid.eq(book_uid)))
            .execute(db_conn)
    });

    let epub_path = Path::new("tests/data/BuddhistMonasticCode_251013.epub");

    // Import with custom title and author
    let custom_title = Some("My Custom Title");
    let custom_author = Some("My Custom Author");
    let result = app_data.import_epub_to_db(&epub_path, book_uid, custom_title, custom_author);
    assert!(result.is_ok(), "EPUB import failed: {:?}", result.err());

    // Verify custom metadata was used
    let book = app_data.dbm.appdata.do_read(|db_conn| {
        books::table
            .filter(books::uid.eq(book_uid))
            .select((books::title, books::author))
            .first::<(Option<String>, Option<String>)>(db_conn)
    }).expect("Query failed");

    assert_eq!(book.0, Some("My Custom Title".to_string()), "Custom title not saved");
    assert_eq!(book.1, Some("My Custom Author".to_string()), "Custom author not saved");

    // Clean up
    app_data.dbm.appdata.do_write(|db_conn| {
        diesel::delete(books::table.filter(books::uid.eq(book_uid)))
            .execute(db_conn)
    });
}

#[test]
#[serial]
fn test_epub_extracted_metadata_when_custom_empty() {
    h::app_data_setup();
    let app_data = get_app_data();
    let book_uid = "test-epub-extracted-metadata";

    // Clean up any existing test data
    app_data.dbm.appdata.do_write(|db_conn| {
        diesel::delete(books::table.filter(books::uid.eq(book_uid)))
            .execute(db_conn)
    });

    let epub_path = Path::new("tests/data/BuddhistMonasticCode_251013.epub");

    // Import with no custom metadata (None)
    let result = app_data.import_epub_to_db(&epub_path, book_uid, None, None);
    assert!(result.is_ok(), "EPUB import failed: {:?}", result.err());

    // Verify extracted metadata was used
    let book = app_data.dbm.appdata.do_read(|db_conn| {
        books::table
            .filter(books::uid.eq(book_uid))
            .select((books::title, books::author))
            .first::<(Option<String>, Option<String>)>(db_conn)
    }).expect("Query failed");

    // Should contain the original EPUB metadata
    assert_eq!(book.0, Some("The Buddhist Monastic Code, Volumes I & II".to_string()));
    assert_eq!(book.1, Some("Thanissaro Bhikkhu".to_string()));

    // Clean up
    app_data.dbm.appdata.do_write(|db_conn| {
        diesel::delete(books::table.filter(books::uid.eq(book_uid)))
            .execute(db_conn)
    });
}

#[test]
#[serial]
fn test_pdf_custom_title_author_override() {
    h::app_data_setup();
    let app_data = get_app_data();
    let book_uid = "test-pdf-custom-metadata";

    // Clean up any existing test data
    app_data.dbm.appdata.do_write(|db_conn| {
        diesel::delete(books::table.filter(books::uid.eq(book_uid)))
            .execute(db_conn)
    });

    let pdf_path = Path::new("tests/data/pali-lessons.pdf");

    // Import with custom title and author
    let custom_title = Some("My Custom PDF Title");
    let custom_author = Some("My Custom PDF Author");
    let result = app_data.import_pdf_to_db(&pdf_path, book_uid, custom_title, custom_author);
    assert!(result.is_ok(), "PDF import failed: {:?}", result.err());

    // Verify custom metadata was used
    let book = app_data.dbm.appdata.do_read(|db_conn| {
        books::table
            .filter(books::uid.eq(book_uid))
            .select((books::title, books::author))
            .first::<(Option<String>, Option<String>)>(db_conn)
    }).expect("Query failed");

    assert_eq!(book.0, Some("My Custom PDF Title".to_string()), "Custom title not saved");
    assert_eq!(book.1, Some("My Custom PDF Author".to_string()), "Custom author not saved");

    // Clean up
    app_data.dbm.appdata.do_write(|db_conn| {
        diesel::delete(books::table.filter(books::uid.eq(book_uid)))
            .execute(db_conn)
    });
}
