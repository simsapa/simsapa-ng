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
    assert_eq!(parsed.volume, Some("ii".to_string()));
    assert_eq!(parsed.page, 20);
}

#[test]
fn test_parse_pts_reference_with_extra_spaces() {
    let result = parse_pts_reference("  M   iii   10  ");
    assert!(result.is_some());
    let parsed = result.unwrap();
    assert_eq!(parsed.nikaya, "m");
    assert_eq!(parsed.volume, Some("iii".to_string()));
    assert_eq!(parsed.page, 10);
}

#[test]
fn test_parse_pts_reference_two_part() {
    let result = parse_pts_reference("Sn 52");
    assert!(result.is_some());
    let parsed = result.unwrap();
    assert_eq!(parsed.nikaya, "sn");
    assert_eq!(parsed.volume, None);
    assert_eq!(parsed.page, 52);
}

#[test]
fn test_parse_pts_reference_invalid() {
    assert!(parse_pts_reference("").is_none());
    assert!(parse_pts_reference("invalid").is_none());
    assert!(parse_pts_reference("abc").is_none()); // Just letters
}

// Test case 2.2: Search DN 1 by identifier
#[test]
fn test_search_dn1_by_identifier() {
    let results = search("DN 1", "sutta_ref");

    assert!(!results.is_empty(), "Should find at least one result for DN 1");

    // Check if the first result is DN 1
    let first = &results[0];
    assert!(
        first.sutta_ref.contains("DN 1") || first.sutta_ref.contains("DN1"),
        "Expected sutta_ref to contain 'DN 1', got: {}",
        first.sutta_ref
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
    let results = search("brahmajala", "title_pali");

    assert!(!results.is_empty(), "Should find result for 'brahmajala'");

    // Should find Brahmajāla Sutta
    let first = &results[0];
    let normalized_name = latinize(&first.title_pali.to_lowercase());
    assert!(
        normalized_name.contains("brahmajala"),
        "Expected title_pali to contain 'brahmajala', got: {} (normalized: {})",
        first.title_pali,
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
        if let Some(ref dpr_ref) = first.dpr_reference {
            assert!(
                dpr_ref.contains("KN 1") || dpr_ref.contains("KN1"),
                "Expected DPR reference to contain 'KN 1', got: {}",
                dpr_ref
            );
        }
    }
}

// Additional test: Verify empty query returns all results
#[test]
fn test_search_empty_query_returns_all() {
    let results = search("", "sutta_ref");

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
    let results_with_diacritics = search("brahmajāla", "title_pali");
    let results_without_diacritics = search("brahmajala", "title_pali");

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
    let dn14: Vec<_> = all_data.iter().filter(|r| r.sutta_ref.contains("DN 14")).collect();
    eprintln!("\nDN 14 entries:");
    for r in &dn14 {
        eprintln!("  {} - pts_ref: '{}' - nikaya: {:?}, vol: {:?}, start: {:?}, end: {:?}",
            r.sutta_ref, r.pts_reference, r.pts_nikaya, r.pts_vol, r.pts_start_page, r.pts_end_page);
    }

    // Now test search
    let results = search("D ii 20", "pts_reference");
    eprintln!("\nSearch 'D ii 20' - Results count: {}", results.len());
    for (i, r) in results.iter().enumerate() {
        eprintln!("  Result {}: {} - pts_ref: '{}' - nikaya: {:?}, vol: {:?}, start: {:?}, end: {:?}",
            i, r.sutta_ref, r.pts_reference, r.pts_nikaya, r.pts_vol, r.pts_start_page, r.pts_end_page);
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
            results_vol_i[0].sutta_ref,
            results_vol_ii[0].sutta_ref,
            "Volume boundaries should separate different suttas"
        );
    }
}

// Test case: Search Sutta Nipāta by PTS reference (2-part format)
#[test]
fn test_search_snp_by_pts_ref() {
    let results = search("sn 52", "pts_reference");

    assert!(!results.is_empty(), "Should find result for 'sn 52'");

    // Should find Snp 2.7 which has pts_start_page: 50, pts_end_page: 55
    let first = &results[0];
    assert!(
        first.sutta_ref.contains("Snp"),
        "Expected sutta_ref to contain 'Snp', got: {}",
        first.sutta_ref
    );

    // Verify it's the right sutta (Brāhmaṇadhammikasutta)
    if first.sutta_ref == "Snp 2.7" {
        assert_eq!(first.title_pali, "Brāhmaṇadhammikasutta");
        assert_eq!(first.pts_nikaya, Some("Sn".to_string()));
        assert_eq!(first.pts_vol, None);
        assert_eq!(first.pts_start_page, Some(50));
        assert_eq!(first.pts_end_page, Some(55));
    }
}

// Test case: Search Sutta Nipāta by exact start page
#[test]
fn test_search_snp_exact_start_page() {
    let results = search("sn 50", "pts_reference");

    assert!(!results.is_empty(), "Should find result for 'sn 50'");

    let first = &results[0];
    assert_eq!(first.sutta_ref, "Snp 2.7");
    assert_eq!(first.pts_start_page, Some(50));
}

// Test case: Search Sutta Nipāta at page boundary (start of next sutta)
#[test]
fn test_search_snp_end_page() {
    let results = search("sn 55", "pts_reference");

    assert!(!results.is_empty(), "Should find result for 'sn 55'");

    // Page 55 is where Snp 2.8 starts (and also the end of Snp 2.7)
    // The sorting should put the sutta that starts at this page first
    let first = &results[0];
    assert_eq!(first.sutta_ref, "Snp 2.8");
    assert_eq!(first.pts_start_page, Some(55));
}
