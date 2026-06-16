//! Integration tests for Contains-mode "Show All Snippets" expansion
//! (see docs/search-snippet-highlight-pipeline.md).
//!
//! Contains is literal-only: each expanded row is one literal occurrence of the
//! normalized query in the record's `content_plain`, focal-highlighted with a
//! single non-nested span. Inflected forms (e.g. `pajahitvā` for `pajahati`)
//! are never highlighted — that is Fulltext's job.

mod helpers;

use serial_test::serial;

use simsapa_backend::get_app_data;
use simsapa_backend::query_task::SearchQueryTask;
use simsapa_backend::types::{SearchArea, SearchMode, SearchParams, SearchResult};

fn contains_params(show_all_snippets: bool) -> SearchParams {
    SearchParams {
        mode: SearchMode::ContainsMatch,
        page_len: Some(10),
        show_all_snippets,
        ..Default::default()
    }
}

/// Run a Contains query and return (rows, record_total).
fn run(query: &str, show_all_snippets: bool) -> (Vec<SearchResult>, i64) {
    let app_data = get_app_data();
    let params = contains_params(show_all_snippets);
    let mut task = SearchQueryTask::new(&app_data.dbm, query.to_string(), params, SearchArea::Suttas);
    let rows = task.results_page(0).expect("results_page should succeed");
    (rows, task.total_hits())
}

/// Expanding does not change the record total, and every expanded row is a
/// single non-nested focal span flagged `is_snippet`.
#[test]
#[serial]
fn contains_show_all_snippets_expands_records() {
    helpers::app_data_setup();

    let (off_rows, off_total) = run("dhamma", false);
    let (on_rows, on_total) = run("dhamma", true);

    assert!(!off_rows.is_empty(), "expected contains matches for 'dhamma'");

    // Record total is the same regardless of expansion (pagination is
    // record-based).
    assert_eq!(off_total, on_total, "record total must not change with expansion");

    // Flag off: one row per record, not flagged as a snippet.
    for r in &off_rows {
        assert!(!r.is_snippet, "flag-off rows are whole-record rows");
    }

    // A common word like 'dhamma' recurs within suttas, so expansion produces
    // strictly more rows than the record count for this page.
    assert!(
        on_rows.len() >= off_rows.len(),
        "expansion should not drop rows ({} on vs {} off)",
        on_rows.len(),
        off_rows.len()
    );

    // At least one record on the page must contain the query more than once,
    // proving genuine per-occurrence expansion happened.
    let expanded_any = on_rows.iter().filter(|r| r.is_snippet).count() > off_rows.len();
    assert!(
        expanded_any || on_rows.len() > off_rows.len(),
        "expected at least one multi-occurrence record to expand"
    );

    // Each expanded snippet row: exactly one non-nested match span.
    for r in on_rows.iter().filter(|r| r.is_snippet) {
        assert!(
            !r.snippet.contains("class='match'><span"),
            "nested span in expanded snippet for uid '{}': {}",
            r.uid,
            r.snippet
        );
        assert_eq!(
            r.snippet.matches("class='match'").count(),
            1,
            "expanded contains snippet must highlight exactly its focal occurrence: {}",
            r.snippet
        );
    }
}

/// Contains expansion is literal-only: a query for `pajahati` never highlights
/// the inflected `pajahitvā` in any expanded row.
#[test]
#[serial]
fn contains_expansion_does_not_highlight_inflections() {
    helpers::app_data_setup();
    let (rows, _) = run("pajahati", true);
    for r in &rows {
        assert!(
            !r.snippet.contains("<span class='match'>pajahitvā</span>"),
            "contains expansion wrongly highlighted an inflection in uid '{}': {}",
            r.uid,
            r.snippet
        );
    }
}
