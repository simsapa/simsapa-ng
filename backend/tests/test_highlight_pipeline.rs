//! End-to-end regression tests for the producer-owned, non-nested highlight
//! pipeline (see docs/search-snippet-highlight-pipeline.md).
//!
//! The bug being guarded: Fulltext snippets were highlighted by tantivy in
//! `render_snippet` and then re-highlighted by the central `highlight_row`
//! pass, producing nested `<span class='match'><span class='match'>…`. After
//! the refactor, highlighting is producer-owned and `highlight_row` is a
//! fallback that only touches still-plain snippets — so no snippet from any
//! mode may contain nested match spans.

mod helpers;

use serial_test::serial;

use simsapa_backend::get_app_data;
use simsapa_backend::query_task::SearchQueryTask;
use simsapa_backend::types::{SearchArea, SearchMode, SearchParams, SearchResult};

fn make_params(mode: SearchMode) -> SearchParams {
    SearchParams {
        mode,
        page_len: Some(10),
        ..Default::default()
    }
}

fn run(area: SearchArea, query: &str, mode: SearchMode) -> Vec<SearchResult> {
    let app_data = get_app_data();
    let params = make_params(mode);
    let mut task = SearchQueryTask::new(&app_data.dbm, query.to_string(), params, area);
    task.results_page(0).expect("results_page should succeed")
}

/// No snippet may contain nested match spans, regardless of mode.
fn assert_no_nested_spans(results: &[SearchResult], label: &str) {
    for r in results {
        assert!(
            !r.snippet.contains("class='match'><span"),
            "{label}: nested match spans in snippet for uid '{}': {}",
            r.uid,
            r.snippet
        );
    }
}

#[test]
#[serial]
fn fulltext_suttas_snippets_are_highlighted_and_non_nested() {
    helpers::app_data_setup();
    let results = run(SearchArea::Suttas, "pajahati", SearchMode::FulltextMatch);
    assert!(!results.is_empty(), "expected fulltext matches for 'pajahati'");
    assert_no_nested_spans(&results, "fulltext suttas");
    // At least one snippet should actually be highlighted (producer-owned).
    assert!(
        results.iter().any(|r| r.snippet.contains("class='match'")),
        "expected at least one highlighted fulltext snippet"
    );
}

#[test]
#[serial]
fn contains_suttas_snippets_are_highlighted_and_non_nested() {
    helpers::app_data_setup();
    let results = run(SearchArea::Suttas, "pajahati", SearchMode::ContainsMatch);
    assert!(!results.is_empty(), "expected contains matches for 'pajahati'");
    assert_no_nested_spans(&results, "contains suttas");
    assert!(
        results.iter().any(|r| r.snippet.contains("class='match'")),
        "expected at least one highlighted contains snippet"
    );
}

/// Contains is literal-only: a query for `pajahati` must never highlight the
/// inflected `pajahitvā` (that is Fulltext's job, via the stemmer).
#[test]
#[serial]
fn contains_does_not_highlight_inflections() {
    helpers::app_data_setup();
    let results = run(SearchArea::Suttas, "pajahati", SearchMode::ContainsMatch);
    for r in &results {
        assert!(
            !r.snippet.contains("<span class='match'>pajahitvā</span>"),
            "contains wrongly highlighted an inflection in uid '{}': {}",
            r.uid,
            r.snippet
        );
    }
}
