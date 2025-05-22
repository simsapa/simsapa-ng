/// Test HTML Rendering for Suttas

use std::fs;
use std::path::PathBuf;
use std::collections::HashMap;

use simsapa_backend::{db, API_URL};
use simsapa_backend::app_data::AppData;
use simsapa_backend::export_helpers::render_sutta_content;

mod helpers;
use helpers::appdata_db_setup;

#[test]
fn test_html_for_pali() {
    appdata_db_setup();

    let dbm = db::get_dbm();
    let sutta = dbm.appdata.get_sutta("mn2/pli/ms").expect("Can't get sutta from db");

    let settings = HashMap::new();
    let db_conn = dbm.appdata.get_conn().expect("No appdata conn");
    let mut app_data = AppData::new(db_conn, settings, API_URL.to_string());

    let html = render_sutta_content(&mut app_data, &sutta, None, None).expect("Can't render the html");

    assert!(html.contains(r#"<div class='suttacentral bilara-text'>"#));

    assert!(html.contains(r#"<header><ul><li class='division'><span data-tmpl-key='mn2:0.1'>Majjhima Nikāya 2 </span></li></ul>"#));

    assert!(html.contains(r#"<p><span data-tmpl-key='mn2:2.1'>“sabbāsavasaṁvarapariyāyaṁ vo, bhikkhave, desessāmi. </span>"#));
}

#[test]
fn test_line_by_line_with_variants() {
    appdata_db_setup();

    let dbm = db::get_dbm();
    let sutta = dbm.appdata.get_sutta("sn1.61/en/sujato").expect("Can't get sutta from db");

    let settings = HashMap::new();
    let db_conn = dbm.appdata.get_conn().expect("No appdata conn");
    let mut app_data = AppData::new(db_conn, settings, API_URL.to_string());

    let html = render_sutta_content(&mut app_data, &sutta, None, None).expect("Can't render the html");

    // fs::write(PathBuf::from("sn1.61_en_sujato.html"), html.clone()).expect("Unable to write file!");

    let expected_html = fs::read_to_string(PathBuf::from("tests/data/sn1.61_en_sujato.html"))
        .expect("Failed to read file");

    assert_eq!(html, expected_html);
}
