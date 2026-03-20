mod helpers;

use serial_test::serial;
use serde::Deserialize;

use simsapa_backend::search::searcher::{FulltextSearcher, SearchFilters};

#[derive(Debug, Deserialize)]
struct ExpectedResult {
    uid: String,
    title: String,
    language: String,
    source_uid: String,
    score: f32,
    snippet_html: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ExpectedOutput {
    total: usize,
    results: Vec<ExpectedResult>,
}

/// Integration test: fulltext search with phrase query
///
/// Reproduces the CLI command:
///   fulltext-search --limit 10 --snippet --lang "pli" --source "ms" --format "json" '"so ce" evaṁ vadeyya'
#[test]
#[serial]
fn test_fulltext_search_so_ce_evam_vadeyya() {
    helpers::app_data_setup();
    let globals = simsapa_backend::get_app_globals();

    let searcher = FulltextSearcher::open(&globals.paths)
        .expect("Failed to open fulltext indexes");

    assert!(searcher.has_sutta_indexes(), "Sutta indexes should be available");

    let filters = SearchFilters {
        lang: Some("pli".to_string()),
        lang_include: true,
        source_uid: Some("ms".to_string()),
        source_include: true,
        nikaya: None,
        sutta_ref: None,
    };

    let query = r#""so ce" evaṁ vadeyya"#;
    let limit = 10;

    let (total, results) = searcher
        .search_suttas_with_count(query, &filters, limit)
        .expect("Search should succeed");

    // Load expected results from test data
    let expected_json = std::fs::read_to_string("tests/data/fulltext_search_so_ce_evam_vadeyya.json")
        .expect("Failed to read tests/data/fulltext_search_so_ce_evam_vadeyya.json");
    let expected: ExpectedOutput = serde_json::from_str(&expected_json)
        .expect("Failed to parse json");

    // Total hits should match
    assert_eq!(total, expected.total,
        "Total hits mismatch: got {}, expected {}", total, expected.total);

    // Result count should match
    assert_eq!(results.len(), expected.results.len(),
        "Result count mismatch: got {}, expected {}", results.len(), expected.results.len());

    // Each result should match uid, title, language, source_uid, and score
    for (i, (actual, exp)) in results.iter().zip(expected.results.iter()).enumerate() {
        assert_eq!(actual.uid, exp.uid,
            "Result {}: uid mismatch: got '{}', expected '{}'", i, actual.uid, exp.uid);

        assert_eq!(actual.title, exp.title,
            "Result {}: title mismatch for '{}': got '{}', expected '{}'",
            i, exp.uid, actual.title, exp.title);

        assert_eq!(actual.lang.as_deref(), Some(exp.language.as_str()),
            "Result {}: language mismatch for '{}'", i, exp.uid);

        assert_eq!(actual.source_uid.as_deref(), Some(exp.source_uid.as_str()),
            "Result {}: source_uid mismatch for '{}'", i, exp.uid);

        let actual_score = actual.score.unwrap_or(0.0);
        assert!((actual_score - exp.score).abs() < 0.01,
            "Result {}: score mismatch for '{}': got {}, expected {}",
            i, exp.uid, actual_score, exp.score);

        // Verify snippet is non-empty when expected
        if let Some(ref exp_snippet) = exp.snippet_html {
            assert!(!actual.snippet.is_empty(),
                "Result {}: snippet should not be empty for '{}'", i, exp.uid);
            assert_eq!(&actual.snippet, exp_snippet,
                "Result {}: snippet mismatch for '{}'", i, exp.uid);
        }
    }
}
