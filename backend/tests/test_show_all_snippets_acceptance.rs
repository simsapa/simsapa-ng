//! Integration acceptance tests for "Show All Snippets" + the snippet exclusion
//! filter, against the real appdata DB (see
//! docs/search-snippet-highlight-pipeline.md).
//!
//! Acceptance record: fulltext `pajahati` matches `cnd8/pli/ms`, a Cūḷaniddesa
//! commentary whose content contains the literal `pajahati` together with
//! stemmed inflections (`pajahitvā`, `pajaheyyuṁ`). With `show_all_snippets` on
//! the record expands into one focal-highlighted, non-nested snippet per matched
//! occurrence. The record is isolated with a `uid_prefix` filter so the test is
//! independent of where it ranks among the ~291 records this query matches in
//! the full corpus.
//!
//! NB: the PRD's worked example ("exactly two snippets") used a *truncated*
//! excerpt. The real record yields several occurrences, and because each snippet
//! window is wide (matching the single-snippet window), the `pajahati` window
//! also contains the plain word `pajahitvā`. So the snippet-exclusion substring
//! test (Req 13–14) legitimately drops every snippet whose *window* contains the
//! excluded string, not only the focally-highlighted one — see
//! exclusion_drops_every_snippet_whose_window_contains_term below.

mod helpers;

use serial_test::serial;

use simsapa_backend::get_app_data;
use simsapa_backend::query_task::SearchQueryTask;
use simsapa_backend::types::{SearchArea, SearchMode, SearchParams, SearchResult};

const ACCEPT_UID: &str = "cnd8/pli/ms";

fn params(show_all_snippets: bool, snippet_exclude: Option<Vec<String>>) -> SearchParams {
    SearchParams {
        mode: SearchMode::FulltextMatch,
        page_len: Some(10),
        // Isolate the acceptance record so the test does not depend on corpus
        // ranking; total_hits is then this one record.
        uid_prefix: Some(ACCEPT_UID.to_string()),
        show_all_snippets,
        snippet_exclude,
        ..Default::default()
    }
}

/// Returns (page rows, total_hits = record count).
fn run(show_all_snippets: bool, snippet_exclude: Option<Vec<String>>) -> (Vec<SearchResult>, i64) {
    let app_data = get_app_data();
    let mut task = SearchQueryTask::new(
        &app_data.dbm,
        "pajahati".to_string(),
        params(show_all_snippets, snippet_exclude),
        SearchArea::Suttas,
    );
    let rows = task.results_page(0).expect("results_page should succeed");
    (rows, task.total_hits())
}

fn rows_for(results: &[SearchResult], uid: &str) -> Vec<SearchResult> {
    results.iter().filter(|r| r.uid == uid).cloned().collect()
}

fn span_count(snippet: &str) -> usize {
    snippet.matches("class='match'").count()
}

/// The single focally-highlighted word of an expanded snippet.
fn focal_word(snippet: &str) -> String {
    snippet
        .split("class='match'>")
        .nth(1)
        .and_then(|s| s.split('<').next())
        .unwrap_or("")
        .to_string()
}

#[test]
#[serial]
fn show_all_snippets_expands_record_into_focal_non_nested_snippets() {
    helpers::app_data_setup();

    // Flag off: exactly one whole-record row (is_snippet == false).
    let (off, total_off) = run(false, None);
    let off_rows = rows_for(&off, ACCEPT_UID);
    assert_eq!(off_rows.len(), 1, "flag off: one row per record");
    assert!(!off_rows[0].is_snippet, "flag off: row is a whole-record row");
    assert_eq!(total_off, 1, "uid_prefix isolates the one record");

    // Flag on: the record expands into several per-occurrence snippet rows.
    let (on, total_on) = run(true, None);
    let rows = rows_for(&on, ACCEPT_UID);
    assert!(rows.len() >= 2, "flag on: record expands to >=2 snippets, got {}", rows.len());
    assert_eq!(total_on, 1, "expansion must not change the record count");

    for r in &rows {
        assert!(r.is_snippet, "expanded rows are flagged is_snippet");
        // Focal-only: exactly one highlight per expanded snippet, never nested.
        assert_eq!(span_count(&r.snippet), 1, "expanded snippet must focal-highlight one occurrence: {}", r.snippet);
        assert!(!r.snippet.contains("class='match'><span"), "no nested match spans: {}", r.snippet);
    }

    // The two key forms from the PRD must each appear as their own focally
    // highlighted snippet: the literal `pajahati` and the stemmed `pajahitvā`.
    let focals: Vec<String> = rows.iter().map(|r| focal_word(&r.snippet)).collect();
    assert!(focals.iter().any(|f| f == "pajahati"), "expected a snippet focally highlighting 'pajahati', got {:?}", focals);
    assert!(focals.iter().any(|f| f == "pajahitvā"), "expected a snippet focally highlighting 'pajahitvā', got {:?}", focals);

    // Focal-only cross-check: the snippet that highlights `pajahati` contains the
    // word `pajahitvā` in its window but leaves it un-highlighted.
    let pajahati_snip = rows.iter().find(|r| focal_word(&r.snippet) == "pajahati").unwrap();
    assert!(pajahati_snip.snippet.contains("pajahitvā"), "the pajahati window should contain the word pajahitvā");
    assert!(!pajahati_snip.snippet.contains("<span class='match'>pajahitvā"), "focal-only: pajahitvā must not be highlighted in the pajahati snippet");
}

#[test]
#[serial]
fn exclusion_drops_every_snippet_whose_window_contains_term() {
    helpers::app_data_setup();

    let (full, _) = run(true, None);
    let full_rows = rows_for(&full, ACCEPT_UID);
    assert!(full_rows.len() >= 2, "precondition: record expands");

    // Excluding `pajahitvā` drops every snippet whose window contains that
    // string (diacritic-insensitive); a snippet without it (the `pajaheyyuṁ`
    // occurrence) survives, so the record stays on the page.
    let (excl, total_excl) = run(true, Some(vec!["pajahitvā".to_string()]));
    let excl_rows = rows_for(&excl, ACCEPT_UID);

    assert!(!excl_rows.is_empty(), "a snippet without the excluded term survives, so the record stays");
    assert!(excl_rows.len() < full_rows.len(), "exclusion removed some snippets");
    for r in &excl_rows {
        assert!(!r.snippet.to_lowercase().contains("pajahitvā"), "surviving snippet must not contain the excluded term: {}", r.snippet);
    }
    // Record count (total_hits) is NOT adjusted by exclusion (Resolved Decision 1).
    assert_eq!(total_excl, 1, "exclusion must not change the record count");

    // Diacritic-insensitive: the un-accented `pajahitva` excludes the same set.
    let (excl_ascii, _) = run(true, Some(vec!["pajahitva".to_string()]));
    let excl_ascii_rows = rows_for(&excl_ascii, ACCEPT_UID);
    assert_eq!(
        excl_ascii_rows.len(),
        excl_rows.len(),
        "diacritic-insensitive exclusion: 'pajahitva' matches 'pajahitvā'"
    );
}
