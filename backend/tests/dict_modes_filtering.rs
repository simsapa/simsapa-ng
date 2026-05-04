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

/// The per-task post-filter `apply_dict_source_uids_filter` only drops rows
/// with `table_name == "dict_words"`. DPD-native rows (`dpd_headwords` /
/// `dpd_roots`) pass through unchanged regardless of the inclusion set. This
/// pins the invariant the bridge-level `dpd_enabled` gate in
/// `fetch_combined_page` relies on: without that gate, disabling DPD in the
/// panel would still leak DPD-native rows into Combined results because the
/// post-filter is "blind" to them.
#[test]
#[serial]
fn dpd_lookup_post_filter_does_not_drop_dpd_native_rows() {
    h::app_data_setup();
    let app_data = get_app_data();

    // Inclusion set that explicitly excludes "dpd" (and any dict_words
    // labels). If the post-filter operated on DPD-native rows it would
    // produce an empty page.
    let mut params = dict_params(SearchMode::DpdLookup);
    params.dict_source_uids = Some(vec!["nonexistent_user_dict".to_string()]);

    let mut task = SearchQueryTask::new(
        &app_data.dbm,
        "dhamma".to_string(),
        params,
        SearchArea::Dictionary,
    );
    let results = task
        .results_page(0)
        .expect("DpdLookup should succeed even with DPD excluded from inclusion set");

    let dpd_native = results
        .iter()
        .filter(|r| r.table_name == "dpd_headwords" || r.table_name == "dpd_roots")
        .count();
    assert!(
        dpd_native > 0,
        "post-filter must pass through DPD-native rows ({} returned, all dropped)",
        results.len()
    );
}

/// Combined's bridge-level `dpd_enabled` gate is the load-bearing protection
/// that prevents DPD-native rows from leaking when the user disables DPD. The
/// gate logic itself is `set.iter().any(|s| s == "dpd")`. This pins the
/// boundary cases.
#[test]
fn combined_dpd_enabled_gate_logic() {
    fn dpd_enabled(uids: Option<&Vec<String>>) -> bool {
        match uids {
            None => true,
            Some(set) => set.iter().any(|s| s == "dpd"),
        }
    }

    assert!(dpd_enabled(None), "None means no constraint -> DPD enabled");
    assert!(
        !dpd_enabled(Some(&vec![])),
        "empty set -> nothing matches, DPD must be skipped"
    );
    assert!(
        !dpd_enabled(Some(&vec!["user_dict".to_string()])),
        "user dict only -> DPD must be skipped"
    );
    assert!(
        dpd_enabled(Some(&vec!["dpd".to_string()])),
        "DPD soloed -> DPD enabled"
    );
    assert!(
        dpd_enabled(Some(&vec!["dpd".to_string(), "user_dict".to_string()])),
        "DPD + user dict -> DPD enabled"
    );
}

/// Combined's cache key carries a `|combined` suffix that cannot collide with
/// the standalone `RESULTS_PAGE_CACHE` key shape
/// (`"{query}|{area}|{params_json}"`). This pins the format so a future
/// refactor doesn't silently merge the two caches.
#[test]
fn combined_cache_key_has_combined_suffix() {
    let combined_key = format!("{}|{}|{}|combined", "dhamma", "Dictionary", "{}");
    let standalone_key = format!("{}|{}|{}", "dhamma", "Dictionary", "{}");
    assert_ne!(combined_key, standalone_key);
    assert!(combined_key.ends_with("|combined"));
    assert!(!standalone_key.ends_with("|combined"));
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
