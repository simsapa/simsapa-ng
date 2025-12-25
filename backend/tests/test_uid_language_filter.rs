use serial_test::serial;

mod helpers;
use helpers as h;

use simsapa_backend::{get_app_data};
use simsapa_backend::query_task::SearchQueryTask;
use simsapa_backend::types::SearchArea;

#[test]
#[serial]
fn test_sutta_uid_match_no_language_filter() {
    h::app_data_setup();
    let app_data = get_app_data();

    // Test with no language filter (None)
    let params_none = h::get_uid_params_with_lang(None);
    let query = "sn56.11";

    let mut query_task = SearchQueryTask::new(
        &app_data.dbm,
        query.to_string(),
        params_none,
        SearchArea::Suttas,
    );

    let results = match query_task.results_page(0) {
        Ok(x) => x,
        Err(s) => {
            panic!("{}", s);
        }
    };

    println!("=== No language filter ===");
    println!("Total hits: {}", query_task.total_hits());
    println!("Results on page 0: {}", results.len());
    for (i, result) in results.iter().enumerate() {
        println!("{}: uid={}, lang={:?}", i, result.uid, result.lang);
    }

    assert!(!results.is_empty(), "Should have results");
}

#[test]
#[serial]
fn test_sutta_uid_match_language_filter_returns_all_languages() {
    h::app_data_setup();
    let app_data = get_app_data();

    // Test with "Language" filter value (should behave like no filter and return all languages)
    let params_language = h::get_uid_params_with_lang(Some("Language".to_string()));
    let params_none = h::get_uid_params_with_lang(None);
    let query = "sn56.11";

    let mut query_task_language = SearchQueryTask::new(
        &app_data.dbm,
        query.to_string(),
        params_language,
        SearchArea::Suttas,
    );

    let mut query_task_none = SearchQueryTask::new(
        &app_data.dbm,
        query.to_string(),
        params_none,
        SearchArea::Suttas,
    );

    let results_language = match query_task_language.results_page(0) {
        Ok(x) => x,
        Err(s) => {
            panic!("{}", s);
        }
    };

    let results_none = match query_task_none.results_page(0) {
        Ok(x) => x,
        Err(s) => {
            panic!("{}", s);
        }
    };

    println!("=== 'Language' filter ===");
    println!("Total hits: {}", query_task_language.total_hits());
    println!("Results on page 0: {}", results_language.len());
    for (i, result) in results_language.iter().enumerate() {
        println!("{}: uid={}, lang={:?}", i, result.uid, result.lang);
    }

    assert!(!results_language.is_empty(), "Should have results with 'Language' filter");

    // Verify that 'Language' filter returns the same total count as None filter
    assert_eq!(query_task_language.total_hits(), query_task_none.total_hits(),
               "'Language' filter should return same total as no filter");

    // Verify that we have at least English and Pali results (which should exist in default database)
    let has_english = results_language.iter().any(|r| r.lang == Some("en".to_string()));
    let has_pali = results_language.iter().any(|r| r.lang == Some("pli".to_string()));

    assert!(has_english, "Should have English results when 'Language' filter is selected");
    assert!(has_pali, "Should have Pali results when 'Language' filter is selected");
}

#[test]
#[serial]
fn test_sutta_uid_match_english_filter() {
    h::app_data_setup();
    let app_data = get_app_data();

    // Test with explicit English filter
    let params_en = h::get_uid_params_with_lang(Some("en".to_string()));
    let query = "sn56.11";

    let mut query_task = SearchQueryTask::new(
        &app_data.dbm,
        query.to_string(),
        params_en,
        SearchArea::Suttas,
    );

    let results = match query_task.results_page(0) {
        Ok(x) => x,
        Err(s) => {
            panic!("{}", s);
        }
    };

    println!("=== English filter ===");
    println!("Total hits: {}", query_task.total_hits());
    println!("Results on page 0: {}", results.len());
    for (i, result) in results.iter().enumerate() {
        println!("{}: uid={}, lang={:?}", i, result.uid, result.lang);
    }

    assert!(!results.is_empty(), "Should have English results");

    // Verify all results are English
    for result in &results {
        assert_eq!(result.lang, Some("en".to_string()),
                   "Expected English language, got {:?} for uid {}", result.lang, result.uid);
    }
}

#[test]
#[serial]
fn test_sutta_uid_match_hungarian_filter() {
    h::app_data_setup();
    let app_data = get_app_data();

    // First check if Hungarian suttas exist in the database
    use simsapa_backend::db::appdata_schema::suttas::dsl::*;
    use diesel::prelude::*;

    let db_conn = &mut app_data.dbm.appdata.get_conn().expect("Failed to get database connection");
    let hu_count: i64 = suttas
        .filter(language.eq("hu"))
        .count()
        .get_result(db_conn)
        .expect("Failed to count Hungarian suttas");

    if hu_count == 0 {
        println!("⚠️  WARNING: Skipping Hungarian sutta test - no Hungarian suttas found in database.");
        println!("   To run this test, import Hungarian suttas using SuttaLanguagesWindow.");
        return;
    }

    // Test with explicit Hungarian filter
    let params_hu = h::get_uid_params_with_lang(Some("hu".to_string()));
    let query = "sn56.11";

    let mut query_task = SearchQueryTask::new(
        &app_data.dbm,
        query.to_string(),
        params_hu,
        SearchArea::Suttas,
    );

    let results = match query_task.results_page(0) {
        Ok(x) => x,
        Err(s) => {
            panic!("{}", s);
        }
    };

    println!("=== Hungarian filter ===");
    println!("Total hits: {}", query_task.total_hits());
    println!("Results on page 0: {}", results.len());
    for (i, result) in results.iter().enumerate() {
        println!("{}: uid={}, lang={:?}", i, result.uid, result.lang);
    }

    assert!(!results.is_empty(), "Should have Hungarian results");

    // Verify all results are Hungarian
    for result in &results {
        assert_eq!(result.lang, Some("hu".to_string()),
                   "Expected Hungarian language, got {:?} for uid {}", result.lang, result.uid);
    }
}

#[test]
#[serial]
fn test_sutta_uid_match_language_filter_includes_all_available_languages() {
    h::app_data_setup();
    let app_data = get_app_data();

    // Check if Hungarian suttas exist in the database
    use simsapa_backend::db::appdata_schema::suttas::dsl::*;
    use diesel::prelude::*;

    let db_conn = &mut app_data.dbm.appdata.get_conn().expect("Failed to get database connection");
    let hu_count: i64 = suttas
        .filter(language.eq("hu"))
        .count()
        .get_result(db_conn)
        .expect("Failed to count Hungarian suttas");

    if hu_count == 0 {
        println!("⚠️  WARNING: Skipping comprehensive language test - no Hungarian suttas found in database.");
        println!("   This test verifies that all languages (including Hungarian) appear with 'Language' filter.");
        println!("   To run this test, import Hungarian suttas using SuttaLanguagesWindow.");
        return;
    }

    // Test with "Language" filter value (should return all languages including Hungarian)
    let params_language = h::get_uid_params_with_lang(Some("Language".to_string()));
    let query = "sn56.11";

    let mut query_task = SearchQueryTask::new(
        &app_data.dbm,
        query.to_string(),
        params_language,
        SearchArea::Suttas,
    );

    let results = match query_task.results_page(0) {
        Ok(x) => x,
        Err(s) => {
            panic!("{}", s);
        }
    };

    println!("=== 'Language' filter with Hungarian suttas available ===");
    println!("Total hits: {}", query_task.total_hits());
    println!("Results on page 0: {}", results.len());
    for (i, result) in results.iter().enumerate() {
        println!("{}: uid={}, lang={:?}", i, result.uid, result.lang);
    }

    assert!(!results.is_empty(), "Should have results");

    // Check that we have Hungarian results when Hungarian suttas exist
    let has_hungarian = results.iter().any(|r| r.lang == Some("hu".to_string()));
    assert!(has_hungarian,
            "Should have Hungarian results when 'Language' filter is selected and Hungarian suttas exist in database");
}

#[test]
#[serial]
fn test_sutta_uid_match_partial_nikaya_reference() {
    h::app_data_setup();
    let app_data = get_app_data();

    // Test with partial nikaya reference (e.g., 'sn56') should match all suttas starting with 'sn56'
    let params = h::get_uid_params_with_lang(Some("Language".to_string()));
    let query = "sn56";

    let mut query_task = SearchQueryTask::new(
        &app_data.dbm,
        query.to_string(),
        params,
        SearchArea::Suttas,
    );

    let results = match query_task.results_page(0) {
        Ok(x) => x,
        Err(s) => {
            panic!("{}", s);
        }
    };

    println!("=== Partial nikaya reference 'sn56' ===");
    println!("Total hits: {}", query_task.total_hits());
    println!("Results on page 0: {}", results.len());
    for (i, result) in results.iter().take(5).enumerate() {
        println!("{}: uid={}, lang={:?}", i, result.uid, result.lang);
    }

    assert!(!results.is_empty(), "Should have results for partial nikaya reference 'sn56'");

    // Verify that results have UIDs starting with 'sn56'
    for result in &results {
        assert!(result.uid.starts_with("sn56"),
                "Expected UID to start with 'sn56', got {}", result.uid);
    }

    // Should have multiple different suttas (not just sn56.11)
    let unique_base_uids: std::collections::HashSet<String> = results.iter()
        .map(|r| {
            // Extract base UID before language code (e.g., "sn56.11/en/sujato" -> "sn56.11")
            r.uid.split('/').next().unwrap_or(&r.uid).to_string()
        })
        .collect();

    println!("Found {} unique base UIDs", unique_base_uids.len());
    assert!(unique_base_uids.len() > 1,
            "Should find multiple different suttas for 'sn56', found: {:?}", unique_base_uids);
}

#[test]
#[serial]
fn test_sutta_uid_match_short_nikaya_reference() {
    h::app_data_setup();
    let app_data = get_app_data();

    // Test with short nikaya reference (e.g., 'dn') should match all suttas in that nikaya
    let params = h::get_uid_params_with_lang(Some("Language".to_string()));
    let query = "dn";

    let mut query_task = SearchQueryTask::new(
        &app_data.dbm,
        query.to_string(),
        params,
        SearchArea::Suttas,
    );

    let results = match query_task.results_page(0) {
        Ok(x) => x,
        Err(s) => {
            panic!("{}", s);
        }
    };

    println!("=== Short nikaya reference 'dn' ===");
    println!("Total hits: {}", query_task.total_hits());
    println!("Results on page 0: {}", results.len());
    for (i, result) in results.iter().take(10).enumerate() {
        println!("{}: uid={}, lang={:?}", i, result.uid, result.lang);
    }

    assert!(!results.is_empty(), "Should have results for nikaya reference 'dn'");

    // Verify that results have UIDs starting with 'dn'
    for result in &results {
        assert!(result.uid.starts_with("dn"),
                "Expected UID to start with 'dn', got {}", result.uid);
    }
}

#[test]
#[serial]
fn test_sutta_uid_match_with_language_filter_on_partial_reference() {
    h::app_data_setup();
    let app_data = get_app_data();

    // Test that language filter works correctly with partial references
    let params_pli = h::get_uid_params_with_lang(Some("pli".to_string()));
    let params_en = h::get_uid_params_with_lang(Some("en".to_string()));
    let query = "sn56";

    let mut query_task_pli = SearchQueryTask::new(
        &app_data.dbm,
        query.to_string(),
        params_pli,
        SearchArea::Suttas,
    );

    let mut query_task_en = SearchQueryTask::new(
        &app_data.dbm,
        query.to_string(),
        params_en,
        SearchArea::Suttas,
    );

    let results_pli = match query_task_pli.results_page(0) {
        Ok(x) => x,
        Err(s) => {
            panic!("{}", s);
        }
    };

    let results_en = match query_task_en.results_page(0) {
        Ok(x) => x,
        Err(s) => {
            panic!("{}", s);
        }
    };

    println!("=== Pali filter on 'sn56' ===");
    println!("Total hits: {}", query_task_pli.total_hits());
    println!("Results on page 0: {}", results_pli.len());

    println!("\n=== English filter on 'sn56' ===");
    println!("Total hits: {}", query_task_en.total_hits());
    println!("Results on page 0: {}", results_en.len());

    assert!(!results_pli.is_empty(), "Should have Pali results for 'sn56'");
    assert!(!results_en.is_empty(), "Should have English results for 'sn56'");

    // Verify all Pali results are in Pali
    for result in &results_pli {
        assert_eq!(result.lang, Some("pli".to_string()),
                   "Expected Pali language, got {:?} for uid {}", result.lang, result.uid);
    }

    // Verify all English results are in English
    for result in &results_en {
        assert_eq!(result.lang, Some("en".to_string()),
                   "Expected English language, got {:?} for uid {}", result.lang, result.uid);
    }
}
