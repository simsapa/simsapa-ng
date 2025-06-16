/// Test HTML Rendering for Suttas

use std::fs;
use std::path::PathBuf;

use simsapa_backend::get_app_data;

mod helpers;
use helpers as h;

#[test]
fn test_html_for_pali() {
    h::app_data_setup();

    let app_data = get_app_data();
    let sutta = app_data.dbm.appdata.get_sutta("mn2/pli/ms").expect("Can't get sutta from db");

    let html = app_data.render_sutta_content(&sutta, None, None).expect("Can't render the html");

    assert!(html.contains(r#"<div class='suttacentral bilara-text'>"#));

    assert!(html.contains(r#"<header><ul><li class='division'><span data-tmpl-key='mn2:0.1'>Majjhima Nikāya 2 </span></li></ul>"#));

    assert!(html.contains(r#"<p><span data-tmpl-key='mn2:2.1'>“sabbāsavasaṁvarapariyāyaṁ vo, bhikkhave, desessāmi. </span>"#));
}

#[test]
fn test_line_by_line_with_variants() {
    h::app_data_setup();

    let app_data = get_app_data();
    let sutta = app_data.dbm.appdata.get_sutta("sn1.61/en/sujato").expect("Can't get sutta from db");

    let html = app_data.render_sutta_content(&sutta, None, None).expect("Can't render the html");

    // fs::write(PathBuf::from("sn1.61_en_sujato.html"), html.clone()).expect("Unable to write file!");

    let expected_html = fs::read_to_string(PathBuf::from("tests/data/sn1.61_en_sujato.html"))
        .expect("Failed to read file");

    assert_eq!(html, expected_html);
}
