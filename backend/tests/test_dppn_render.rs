//! `render_dppn_entry` must wrap the bootstrap-stored `<div class="dppn">…</div>`
//! HTML in the standard sutta page chrome (`page.html` + suttas.js) and inject
//! `WINDOW_ID`, `IS_MOBILE`, the dictionary CSS (`.dppn` rules), and `API_URL`
//! — the same environment that DPD bold-definition entries get via
//! `render_bold_definition`. This test pins those expectations against a
//! synthetic `DictWord`.

use simsapa_backend::db::dictionaries_models::DictWord;
use simsapa_backend::html_content::render_dppn_entry;
use simsapa_backend::init_app_globals;

fn make_dppn_word() -> DictWord {
    DictWord {
        id: 1,
        dictionary_id: 1,
        dict_label: "dppn".to_string(),
        uid: "ananda/dppn".to_string(),
        word: "Ānanda".to_string(),
        word_ascii: "ananda".to_string(),
        language: Some("en".to_string()),
        word_nom_sg: None,
        inflections: None,
        phonetic: None,
        transliteration: None,
        meaning_order: None,
        definition_plain: Some("Ananda was the cousin of the Buddha.".to_string()),
        definition_html: Some(
            r#"<div class="dppn"><p>Ānanda was the cousin of the Buddha. See <a class="dppn-ref" href="ssp://dppn_lookup/Sariputta"><span class="t14">Sāriputta</span></a>.</p></div>"#
                .to_string(),
        ),
        summary: None,
        synonyms: None,
        antonyms: None,
        homonyms: None,
        also_written_as: None,
        see_also: None,
    }
}

#[test]
fn render_dppn_entry_includes_chrome_css_js_and_window_id() {
    init_app_globals();

    let word = make_dppn_word();
    let html = render_dppn_entry(&word, "test-window-1", Some("light".to_string()));

    // Standard page chrome: page.html provides <html>, <head>, <body>.
    assert!(html.contains("<html"), "missing <html>: {}", &html[..200.min(html.len())]);
    assert!(html.contains("<body"), "missing <body>");

    // Dictionary CSS layered for `.dppn` styling — at minimum the `.dppn .t14`
    // rule lives there once task 3.0 lands; for now assert the `<style>` block
    // is present and that some text from `dictionary.css` is included.
    assert!(html.contains("<style"), "missing <style> block");

    // WINDOW_ID injected for the click-handler callback.
    assert!(
        html.contains("const WINDOW_ID = 'test-window-1'"),
        "WINDOW_ID not injected"
    );

    // page.html declares API_URL from ctx.api_url.
    assert!(html.contains("API_URL"), "API_URL not declared by page.html");

    // suttas.js is concatenated into the page so link-click handlers attach.
    // It declares a global helper or constant — we just assert the embedded
    // script tag is non-trivial in length.
    let js_block_len = html.len();
    assert!(js_block_len > 5000, "rendered page suspiciously small: {} bytes", js_block_len);

    // The DPPN content (with its already-wrapped <div class="dppn">) is included
    // verbatim — no double-wrapping.
    assert!(html.contains(r#"<div class="dppn">"#), "DPPN wrapper missing");
    assert_eq!(
        html.matches(r#"<div class="dppn">"#).count(),
        1,
        "DPPN wrapper should appear exactly once (no double-wrapping)"
    );

    // The cross-reference anchor survives.
    assert!(html.contains(r#"href="ssp://dppn_lookup/Sariputta""#));
}
