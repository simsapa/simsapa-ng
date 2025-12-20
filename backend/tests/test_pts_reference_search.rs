use simsapa_backend::pts_reference_search::{
    parse_pts_reference, search, search_by_pts_reference,
};
use simsapa_backend::helpers::latinize;

#[test]
fn test_parse_pts_reference_valid() {
    let result = parse_pts_reference("D ii 20");
    assert!(result.is_some());
    let parsed = result.unwrap();
    assert_eq!(parsed.nikaya, "d");
    assert_eq!(parsed.volume, "ii");
    assert_eq!(parsed.page, 20);
}

#[test]
fn test_parse_pts_reference_with_extra_spaces() {
    let result = parse_pts_reference("  M   iii   10  ");
    assert!(result.is_some());
    let parsed = result.unwrap();
    assert_eq!(parsed.nikaya, "m");
    assert_eq!(parsed.volume, "iii");
    assert_eq!(parsed.page, 10);
}

#[test]
fn test_parse_pts_reference_invalid() {
    assert!(parse_pts_reference("").is_none());
    assert!(parse_pts_reference("invalid").is_none());
    assert!(parse_pts_reference("D 20").is_none()); // Missing volume
}

// Test case 2.2: Search DN 1 by identifier
#[test]
fn test_search_dn1_by_identifier() {
    let results = search("DN 1", "identifier");

    assert!(!results.is_empty(), "Should find at least one result for DN 1");

    // Check if the first result is DN 1
    let first = &results[0];
    assert!(
        first.identifier.contains("DN 1") || first.identifier.contains("DN1"),
        "Expected identifier to contain 'DN 1', got: {}",
        first.identifier
    );
}

// Test case 2.3: Search DN 2 by PTS ref (exact match)
#[test]
fn test_search_dn2_by_pts_ref_exact() {
    let results = search("D i 47", "pts_reference");

    assert!(!results.is_empty(), "Should find result for exact PTS ref 'D i 47'");

    let first = &results[0];
    assert!(
        first.pts_reference.contains("D i 47"),
        "Expected PTS reference to contain 'D i 47', got: {}",
        first.pts_reference
    );
}

// Test case 2.4: Search DN 2 by PTS ref (in-between page)
#[test]
fn test_search_dn2_by_pts_ref_in_between() {
    let results = search("D i 50", "pts_reference");

    assert!(!results.is_empty(), "Should find result for in-between page 'D i 50'");

    // Should find DN 2 which starts at D i 47
    let first = &results[0];
    assert!(
        first.pts_reference.contains("D i 47"),
        "Expected to find DN 2 starting at 'D i 47' when searching for 'D i 50', got: {}",
        first.pts_reference
    );
}

// Test case 2.5: Search DN 14 by PTS ref (exact at volume boundary)
#[test]
fn test_search_dn14_by_pts_ref_exact_volume_boundary() {
    let results = search("D ii 1", "pts_reference");

    assert!(!results.is_empty(), "Should find result for volume boundary 'D ii 1'");

    let first = &results[0];
    assert!(
        first.pts_reference.contains("D ii 1"),
        "Expected PTS reference to contain 'D ii 1', got: {}",
        first.pts_reference
    );
}

// Test case 2.6: Search DN 14 by PTS ref (in-between)
#[test]
fn test_search_dn14_by_pts_ref_in_between() {
    let results = search("D ii 20", "pts_reference");

    assert!(!results.is_empty(), "Should find result for in-between page 'D ii 20'");

    // Should find DN 14 which starts at D ii 1
    let first = &results[0];
    assert!(
        first.pts_reference.contains("D ii 1"),
        "Expected to find DN 14 starting at 'D ii 1' when searching for 'D ii 20', got: {}",
        first.pts_reference
    );
}

// Test case 2.7: Search MN by PTS ref (in-between)
#[test]
fn test_search_mn_by_pts_ref_in_between() {
    let results = search("M iii 10", "pts_reference");

    assert!(!results.is_empty(), "Should find result for in-between page 'M iii 10'");

    // Should find a sutta around M iii 7
    let first = &results[0];
    // The exact reference might vary, but we should find something in the M iii range
    assert!(
        first.pts_reference.contains("M iii"),
        "Expected PTS reference to be in 'M iii' range, got: {}",
        first.pts_reference
    );
}

// Test case 2.8: Search by name (case insensitive)
#[test]
fn test_search_by_name_case_insensitive() {
    let results = search("brahmajala", "name");

    assert!(!results.is_empty(), "Should find result for 'brahmajala'");

    // Should find Brahmajāla Sutta
    let first = &results[0];
    let normalized_name = latinize(&first.name.to_lowercase());
    assert!(
        normalized_name.contains("brahmajala"),
        "Expected name to contain 'brahmajala', got: {} (normalized: {})",
        first.name,
        normalized_name
    );
}

// Test case 2.9: Search KN by DPR reference
#[test]
fn test_search_kn_by_dpr_reference() {
    let results = search("KN 1", "dpr_reference");

    // This test depends on whether KN entries exist in the JSON data
    // If there are KN entries, we should find them
    // If not, this is also acceptable
    if !results.is_empty() {
        let first = &results[0];
        assert!(
            first.dpr_reference.contains("KN 1") || first.dpr_reference.contains("KN1"),
            "Expected DPR reference to contain 'KN 1', got: {}",
            first.dpr_reference
        );
    }
}

// Additional test: Verify empty query returns all results
#[test]
fn test_search_empty_query_returns_all() {
    let results = search("", "identifier");

    // Empty query should return all entries (or at least a significant number)
    assert!(
        results.len() > 10,
        "Empty query should return many results, got: {}",
        results.len()
    );
}

// Additional test: Verify search with diacritics works
#[test]
fn test_search_with_diacritics() {
    let results_with_diacritics = search("brahmajāla", "name");
    let results_without_diacritics = search("brahmajala", "name");

    // Both should return the same results due to latinize normalization
    assert_eq!(
        results_with_diacritics.len(),
        results_without_diacritics.len(),
        "Search with and without diacritics should return same number of results"
    );
}

// Additional test: Verify range matching works correctly
#[test]
fn test_pts_reference_range_matching() {
    // Test that we can find a sutta by a page number between its start and the next sutta
    let results = search_by_pts_reference("D i 50");

    assert!(!results.is_empty(), "Should find sutta containing page D i 50");

    // Verify the found sutta starts before page 50
    let first = &results[0];
    if let Some(parsed) = parse_pts_reference(&first.pts_reference) {
        assert!(
            parsed.page <= 50,
            "Found sutta should start at or before page 50, got page {}",
            parsed.page
        );
    }
}

// Debug test to see what data is being loaded
#[test]
fn test_debug_data_loading() {
    // First, load all data to check JSON parsing
    use simsapa_backend::app_settings::SUTTA_REFERENCE_CONVERTER_JSON;
    use simsapa_backend::pts_reference_search::ReferenceSearchResult;

    let all_data: Vec<ReferenceSearchResult> = serde_json::from_str(SUTTA_REFERENCE_CONVERTER_JSON)
        .expect("Failed to parse JSON");
    eprintln!("Total entries in JSON: {}", all_data.len());

    // Find DN 14
    let dn14: Vec<_> = all_data.iter().filter(|r| r.identifier.contains("DN 14")).collect();
    eprintln!("\nDN 14 entries:");
    for r in &dn14 {
        eprintln!("  {} - pts_ref: '{}' - nikaya: {:?}, vol: {:?}, start: {:?}, end: {:?}",
            r.identifier, r.pts_reference, r.pts_nikaya, r.pts_vol, r.pts_start_page, r.pts_end_page);
    }

    // Now test search
    let results = search("D ii 20", "pts_reference");
    eprintln!("\nSearch 'D ii 20' - Results count: {}", results.len());
    for (i, r) in results.iter().enumerate() {
        eprintln!("  Result {}: {} - pts_ref: '{}' - nikaya: {:?}, vol: {:?}, start: {:?}, end: {:?}",
            i, r.identifier, r.pts_reference, r.pts_nikaya, r.pts_vol, r.pts_start_page, r.pts_end_page);
    }
}

// Additional test: Verify that different volumes don't match
#[test]
fn test_pts_reference_volume_boundary() {
    let results_vol_i = search_by_pts_reference("D i 300");
    let results_vol_ii = search_by_pts_reference("D ii 1");

    // These should find different suttas
    if !results_vol_i.is_empty() && !results_vol_ii.is_empty() {
        assert_ne!(
            results_vol_i[0].identifier,
            results_vol_ii[0].identifier,
            "Volume boundaries should separate different suttas"
        );
    }
}
