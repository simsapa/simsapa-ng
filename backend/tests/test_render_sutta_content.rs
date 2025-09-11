/// Test HTML Rendering for Suttas

use std::fs;
use std::path::PathBuf;

use simsapa_backend::get_app_data;
use simsapa_backend::html_format::{html_indent, extract_element_by_id_from_indented};

mod helpers;
use helpers as h;

#[test]
fn test_html_for_pali() {
    h::app_data_setup();

    let app_data = get_app_data();
    let sutta = app_data.dbm.appdata.get_sutta("mn2/pli/ms").expect("Can't get sutta from db");

    let html = app_data.render_sutta_content(&sutta, None, None).expect("Can't render the html");

    assert!(html.contains(r#"<div class='suttacentral bilara-text'>"#));

    assert!(html.contains(r#"<header><ul><li class='division'><span class="segment" id="mn2:0.1"><span class="root" lang="pli" translate="no"><span class="text" lang="la">Majjhima Nikāya 2 </span></span></span></li></ul>"#));

    assert!(html.contains(r#"<span class="segment" id="mn2:1.1"><span class="root" lang="pli" translate="no"><span class="text" lang="la">Evaṁ me sutaṁ—</span></span></span>"#));

    assert!(html.contains(r#"<span class="segment" id="mn2:2.1"><span class="root" lang="pli" translate="no"><span class="text" lang="la">“sabbāsavasaṁvarapariyāyaṁ vo, bhikkhave, desessāmi. </span></span></span>"#));
}

#[test]
fn test_html_en() {
    h::app_data_setup();

    let app_data = get_app_data();
    let sutta = app_data.dbm.appdata.get_sutta("dn22/en/thanissaro").expect("Can't get sutta from db");

    let html = app_data.render_sutta_content(&sutta, None, None).expect("Can't render the html");

    // fs::write(PathBuf::from("dn22_en_thanissaro.html"), html.clone()).expect("Unable to write file!");

    let expected_html = fs::read_to_string(PathBuf::from("tests/data/dn22_en_thanissaro.html"))
        .expect("Failed to read file");

    assert_eq!(html, expected_html);
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

#[test]
fn test_pali_only() {
    h::app_data_setup();

    let sutta_uid = "sn56.11/pli/ms".to_string();
    let sutta_name = format!("{}", sutta_uid.replace("/", "_"));
    let sutta_filename = format!("{}.html", sutta_name);
    let do_write = false;

    let app_data = get_app_data();
    let sutta = app_data.dbm.appdata.get_sutta(&sutta_uid).expect("Can't get sutta from db");

    let html = app_data.render_sutta_content(&sutta, None, None).expect("Can't render the html");
    let html = extract_element_by_id_from_indented(&html_indent(&html), "sn56.11").unwrap_or("None".to_string());

    if do_write {
        fs::write(PathBuf::from(&sutta_filename), html.clone()).expect("Unable to write file!");

        let sc_html = fs::read_to_string(PathBuf::from(&format!("tests/data/{}.suttacentral.html", &sutta_name))).expect("Failed to read file");
        let sc_html = extract_element_by_id_from_indented(&html_indent(&sc_html), "sn56.11").unwrap_or("None".to_string());

        fs::write(PathBuf::from(&format!("tests/data/{}.suttacentral.main.html", &sutta_name)), sc_html.clone()).expect("Unable to write file!");
    }

    let expected_html = fs::read_to_string(PathBuf::from(&format!("tests/data/{}", &sutta_filename)))
        .expect("Failed to read file");

    assert_eq!(html, expected_html);
}

#[test]
fn test_sn56_11_html_format_validation() {
    h::app_data_setup();

    let app_data = get_app_data();
    let sutta = app_data.dbm.appdata.get_sutta("sn56.11/pli/ms").expect("Can't get sutta from db");

    let html = app_data.render_sutta_content(&sutta, None, None).expect("Can't render the html");

    // Validate SuttaCentral format structure
    assert!(html.contains(r#"<div class='suttacentral bilara-text'>"#));

    // Validate specific segment format with nested spans
    assert!(html.contains(r#"<span class="segment" id="sn56.11:0.1"><span class="root" lang="pli" translate="no"><span class="text" lang="la">Saṁyutta Nikāya 56.11 </span></span></span>"#));

    // Validate ordering - these segments should appear in this order
    let pos_0_1 = html.find(r#"id="sn56.11:0.1""#).expect("Should find segment 0.1");
    let pos_0_2 = html.find(r#"id="sn56.11:0.2""#).expect("Should find segment 0.2");
    let pos_0_3 = html.find(r#"id="sn56.11:0.3""#).expect("Should find segment 0.3");
    let pos_1_1 = html.find(r#"id="sn56.11:1.1""#).expect("Should find segment 1.1");

    // Assert correct ordering
    assert!(pos_0_1 < pos_0_2, "Segment 0.1 should come before 0.2");
    assert!(pos_0_2 < pos_0_3, "Segment 0.2 should come before 0.3");
    assert!(pos_0_3 < pos_1_1, "Segment 0.3 should come before 1.1");

    // Validate against reference file
    let expected_html = fs::read_to_string(PathBuf::from("tests/data/sn56.11_pli_ms.html"))
        .expect("Failed to read reference file");

    assert_eq!(html, expected_html);
}
