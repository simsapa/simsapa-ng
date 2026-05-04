// Integration tests for dictionary search-mode filtering invariants.
// PRD: tasks/prd-integrate-stardict-filtering.md.
//
// Task 3.3: cover the `Combined + Dictionary -> Err` and
// `Combined + Suttas -> FulltextMatch fallback` invariants. The remaining
// per-mode filtering tests (Contains / Headword / DPD invariants under user
// dictionary toggles) are added in task 7.1.

use serial_test::serial;
use simsapa_backend::get_app_data;
use simsapa_backend::query_task::SearchQueryTask;
use simsapa_backend::types::{SearchArea, SearchMode, SearchParams};

mod helpers;
use helpers as h;

fn dict_params(mode: SearchMode) -> SearchParams {
    SearchParams {
        mode,
        page_len: Some(20),
        lang: None,
        lang_include: true,
        source: None,
        source_include: true,
        enable_regex: false,
        fuzzy_distance: 0,
        include_cst_mula: true,
        include_cst_commentary: true,
        nikaya_prefix: None,
        uid_prefix: None,
        uid_suffix: None,
        include_ms_mula: true,
        include_comm_bold_definitions: false,
        dict_source_uids: None,
    }
}

fn suttas_params(mode: SearchMode) -> SearchParams {
    SearchParams {
        mode,
        page_len: Some(20),
        lang: Some("en".to_string()),
        lang_include: true,
        source: None,
        source_include: true,
        enable_regex: false,
        fuzzy_distance: 0,
        include_cst_mula: true,
        include_cst_commentary: true,
        nikaya_prefix: None,
        uid_prefix: None,
        uid_suffix: None,
        include_ms_mula: true,
        include_comm_bold_definitions: false,
        dict_source_uids: None,
    }
}

#[test]
#[serial]
fn combined_mode_dictionary_returns_err_at_query_task() {
    h::app_data_setup();
    let app_data = get_app_data();

    let mut task = SearchQueryTask::new(
        &app_data.dbm,
        "dhamma".to_string(),
        dict_params(SearchMode::Combined),
        SearchArea::Dictionary,
    );

    let result = task.results_page(0);
    assert!(
        result.is_err(),
        "Combined + Dictionary must Err at query_task layer (PRD §5.4: bridge-orchestrated)"
    );
    let msg = result.err().unwrap().to_string();
    assert!(
        msg.contains("bridge-orchestrated"),
        "error message should explain why; got: {msg}"
    );
}

#[test]
#[serial]
fn combined_mode_suttas_falls_back_to_fulltext() {
    h::app_data_setup();
    let app_data = get_app_data();

    let query = "satipaṭṭhāna";

    let mut combined_task = SearchQueryTask::new(
        &app_data.dbm,
        query.to_string(),
        suttas_params(SearchMode::Combined),
        SearchArea::Suttas,
    );
    let combined_results = combined_task
        .results_page(0)
        .expect("Combined + Suttas should fall back to FulltextMatch");

    let mut fulltext_task = SearchQueryTask::new(
        &app_data.dbm,
        query.to_string(),
        suttas_params(SearchMode::FulltextMatch),
        SearchArea::Suttas,
    );
    let fulltext_results = fulltext_task
        .results_page(0)
        .expect("FulltextMatch + Suttas baseline must succeed");

    assert_eq!(
        combined_results.len(),
        fulltext_results.len(),
        "Combined + Suttas page 0 size should match FulltextMatch + Suttas"
    );
    let combined_uids: Vec<_> = combined_results.iter().map(|r| &r.uid).collect();
    let fulltext_uids: Vec<_> = fulltext_results.iter().map(|r| &r.uid).collect();
    assert_eq!(
        combined_uids, fulltext_uids,
        "Combined + Suttas should yield the same uids in the same order as FulltextMatch + Suttas"
    );
}
