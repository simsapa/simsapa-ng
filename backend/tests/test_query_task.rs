use std::collections::HashMap;

use diesel::prelude::*;
use serial_test::serial;
use simsapa_backend::types::{SearchArea, SearchMode};
use simsapa_backend::query_task::SearchQueryTask;
use simsapa_backend::get_app_data;
use simsapa_backend::db::appdata_models::NewSutta;
use simsapa_backend::db::appdata_schema::suttas;

mod helpers;
use helpers as h;

#[test]
#[serial]
fn test_highlight_text_simple() {
    h::app_data_setup();
    let task = h::create_test_task("satipaṭṭhā", SearchMode::ContainsMatch);
    let content = "sīlaṁ nissāya sīle patiṭṭhāya cattāro satipaṭṭhāne bhāveyyāsi";
    let highlighted = task.highlight_text(&task.query_text, content).unwrap();
    assert_eq!(highlighted, "sīlaṁ nissāya sīle patiṭṭhāya cattāro <span class='match'>satipaṭṭhā</span>ne bhāveyyāsi");
}

#[test]
#[serial]
fn test_highlight_text_uppercase() {
    h::app_data_setup();
    let task = h::create_test_task("SATIpaṭṭhā", SearchMode::ContainsMatch);
    let content = "sīlaṁ nissāya sīle patiṭṭhāya cattāro satipaṭṭhāne bhāveyyāsi";
    let highlighted = task.highlight_text(&task.query_text, content).unwrap();
    assert_eq!(highlighted, "sīlaṁ nissāya sīle patiṭṭhāya cattāro <span class='match'>satipaṭṭhā</span>ne bhāveyyāsi");
}

#[test]
#[serial]
fn test_highlight_text_regex_special_chars() {
    h::app_data_setup();
    let task = h::create_test_task("test", SearchMode::ContainsMatch);
    let content = "This has regex .*+ chars";
    let highlighted = task.highlight_text(".*+", content).unwrap();
    assert_eq!(highlighted, "This has regex <span class='match'>.*+</span> chars");
}

#[test]
#[serial]
fn test_fragment_around_text_middle() {
    h::app_data_setup();
    let task = h::create_test_task("satipaṭṭhā", SearchMode::ContainsMatch);
    let content = "sīlaṁ nissāya sīle patiṭṭhāya cattāro satipaṭṭhāne bhāveyyāsi";
    let fragment = task.fragment_around_text(&task.query_text, content, 10, 200);
    assert!(fragment.contains(&task.query_text));
    assert!(fragment.starts_with("... patiṭṭhāya cattāro satipaṭṭhāne"));
    assert!(fragment.ends_with("bhāveyyāsi"));
}

#[test]
#[serial]
fn test_sutta_search_contains_match() {
    h::app_data_setup();
    let app_data = get_app_data();
    let params = h::get_contains_params_with_lang(Some("en".to_string()));

    let query = "satipaṭṭhāna";

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

    assert!(!results.is_empty());
    // Verify the query term appears in the snippet
    assert!(results[0].snippet.contains("<span class='match'>satipaṭṭhāna</span>"));

    // FIXME Earlier when headers were not indexed:
    // assert_eq!(results[0].uid, "mil5.3.7/en/tw_rhysdavids");
    // assert!(results[0].snippet.starts_with("... accordance with the rules of <span class='match'>satipaṭṭhāna</span>"));
    // assert!(results[0].snippet.ends_with("law of property to carry on the traditions of the khattiya clans and to fight ..."));

    // FIXME Now with headers indexed:
    assert_eq!(results[0].uid, "mn10/en/horner");
    assert!(results[0].snippet.starts_with("... middle length sayings <span class='match'>satipaṭṭhāna</span> suttaṁ"));

    // Verify all results are English
    for result in &results {
        assert_eq!(result.lang, Some("en".to_string()));
    }
}

#[test]
#[serial]
fn test_sutta_search_contains_match_with_punctuation() {
    h::app_data_setup();
    let app_data = get_app_data();
    // These are Pali queries, so use Pali language filter
    let params = h::get_contains_params_with_lang(Some("pli".to_string()));

    let mut queries: HashMap<&str, &str> = HashMap::new();
    queries.insert("Anāsavañca vo, bhikkhave, desessāmi",
                   "sn43.14-43/pli/ms");
    queries.insert("padakkhiṇaṁ mano-kammaṁ",
                   "an3.155/pli/ms");
    queries.insert("na ca mayaṁ labhāma bhagavantaṁ dassanāyā’ti.",
                   "pli-tv-kd7/pli/ms");
    queries.insert("yaṁ jaññā— ‘sakkomi ajjeva gantun’ti.",
                   "pli-tv-kd4/pli/ms");
    // NOTE: cst4 is not currently included in the bootstrap
    // queries.insert("pañca kāladānānī’’ti.",
    //                "an5.36/pli/cst4");
    // queries.insert("saraṇaṁ…pe॰…anusāsanī’’ti?",
    //                "sn43.14/pli/cst4");
    // queries.insert("katamañca, bhikkhave, nibbānaṁ…pe॰… abyāpajjhañca [abyāpajjhañca (sī॰ syā॰ kaṁ॰ pī॰)] vo, bhikkhave, desessāmi abyāpajjhagāmiñca maggaṁ.",
    //                "sn43.14/pli/cst4");
    // queries.insert("pāṇina’’nti.. chaṭṭhaṁ.",
    //                "an5.36/pli/cst4");

    for (query_text, first_result_uid) in queries.into_iter() {
        let mut query_task = SearchQueryTask::new(
            &app_data.dbm,
            query_text.to_string(),
            params.clone(),
            SearchArea::Suttas,
        );

        let results = match query_task.results_page(0) {
            Ok(x) => x,
            Err(s) => {
                panic!("{}", s);
            }
        };

        assert!(!results.is_empty());
        assert_eq!(results[0].uid, first_result_uid.to_string());
    }
}

#[test]
#[serial]
fn test_sutta_search_contains_match_exact_results() {
    h::app_data_setup();
    let app_data = get_app_data();
    let params = h::get_contains_params_with_lang(Some("pli".to_string()));

    let mut queries: HashMap<&str, Vec<&str>> = HashMap::new();
    // Note: Only one sutta contains this text in the current database
    queries.insert("Anāsavañca vo, bhikkhave, desessāmi",
                   // FIXME: cst4 is not currently included in the db bootstrap
                   // vec!["sn43.14-43/pli/ms", "sn43.14/pli/cst4"]
                   vec!["sn43.14-43/pli/ms"]
    );

    for (query_text, expected_uids) in queries.into_iter() {
        let mut query_task = SearchQueryTask::new(
            &app_data.dbm,
            query_text.to_string(),
            params.clone(),
            SearchArea::Suttas,
        );

        let results = match query_task.results_page(0) {
            Ok(x) => x,
            Err(s) => {
                panic!("{}", s);
            }
        };

        assert!(!results.is_empty());
        assert_eq!(results.len(), expected_uids.len());
        for (idx, expected_uid) in expected_uids.iter().enumerate() {
            assert_eq!(results[idx].uid, expected_uid.to_string());
        }
    }
}

#[test]
#[serial]
fn test_dict_word_search_contains_match() {
    h::app_data_setup();
    let app_data = get_app_data();
    let params = h::get_contains_params_with_lang(Some("en".to_string()));

    let query = "element of awakening; factor of enlightenment";

    let mut query_task = SearchQueryTask::new(
        &app_data.dbm,
        query.to_string(),
        params,
        SearchArea::Dictionary,
    );

    let results = match query_task.results_page(0) {
        Ok(x) => x,
        Err(s) => {
            panic!("{}", s);
        }
    };

    // println!("{}", results[0].snippet);

    assert_eq!(results[0].uid, "sambodhiyaṅga/dpd");
    assert!(results[0].snippet.starts_with("masc <span class='match'>element of awakening factor of enlightenment</span>"));
}

#[test]
#[serial]
fn test_dict_word_uid_match() {
    h::app_data_setup();
    let app_data = get_app_data();
    let params = h::get_uid_params();

    let query = "satipaṭṭhāna 1/dpd";

    let mut query_task = SearchQueryTask::new(
        &app_data.dbm,
        query.to_string(),
        params,
        SearchArea::Dictionary,
    );

    let results = match query_task.results_page(0) {
        Ok(x) => x,
        Err(s) => {
            panic!("{}", s);
        }
    };

    println!("{}", results[0].snippet);

    assert_eq!(results[0].uid, "satipaṭṭhāna 1/dpd");
    assert!(results[0].snippet.starts_with("masc attending mindfully being present with mindfulness [sati + upaṭṭhāna]"));
}

#[test]
#[serial]
fn test_sutta_search_uid_match_with_language_filter() {
    h::app_data_setup();
    let app_data = get_app_data();

    // Test with Pali language filter
    let params_pli = h::get_uid_params_with_lang(Some("pli".to_string()));
    let query = "mn1";

    let mut query_task = SearchQueryTask::new(
        &app_data.dbm,
        query.to_string(),
        params_pli,
        SearchArea::Suttas,
    );

    let results = match query_task.results_page(0) {
        Ok(x) => x,
        Err(s) => {
            panic!("{}", s);
        }
    };

    // Verify all results are in Pali
    assert!(!results.is_empty());
    for result in &results {
        assert_eq!(result.lang, Some("pli".to_string()),
                   "Expected Pali language, got {:?} for uid {}", result.lang, result.uid);
    }
}

#[test]
#[serial]
fn test_sutta_search_contains_match_fts5_with_language_filter() {
    h::app_data_setup();
    let app_data = get_app_data();

    // Test with Pali language filter
    let params_pli = h::get_contains_params_with_lang(Some("pli".to_string()));
    let query = "satipaṭṭhāna";

    let mut query_task = SearchQueryTask::new(
        &app_data.dbm,
        query.to_string(),
        params_pli,
        SearchArea::Suttas,
    );

    let results = match query_task.results_page(0) {
        Ok(x) => x,
        Err(s) => {
            panic!("{}", s);
        }
    };

    // Verify all results are in Pali
    assert!(!results.is_empty());
    for result in &results {
        assert_eq!(result.lang, Some("pli".to_string()),
                   "Expected Pali language, got {:?} for uid {}", result.lang, result.uid);
        assert!(result.snippet.contains("satipaṭṭhāna"));
    }
}

#[test]
#[serial]
fn test_sutta_search_contains_match_fts5_with_english_language_filter() {
    h::app_data_setup();
    let app_data = get_app_data();

    // Test with English language filter
    let params_en = h::get_contains_params_with_lang(Some("en".to_string()));
    // Use a word which may occur in English and Pāli texts as well
    let query = "dhamma";

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

    // Verify all results are in English
    assert!(!results.is_empty());
    for result in &results {
        assert_eq!(result.lang, Some("en".to_string()),
                   "Expected English language, got {:?} for uid {}", result.lang, result.uid);
        assert!(result.snippet.to_lowercase().contains("dhamma"));
    }
}

#[test]
#[serial]
fn test_sutta_search_contains_match_fts5_no_language_filter() {
    h::app_data_setup();
    let app_data = get_app_data();

    // Test with no language filter (None) - using a query that appears in multiple languages
    // The word "bhikkhu" appears in Pali, English, and Thai texts
    let params_none = h::get_contains_params_with_lang(None);
    let query = "bhikkhu";

    let mut query_task_none = SearchQueryTask::new(
        &app_data.dbm,
        query.to_string(),
        params_none.clone(),
        SearchArea::Suttas,
    );

    let _results_none = match query_task_none.results_page(0) {
        Ok(x) => x,
        Err(s) => {
            panic!("{}", s);
        }
    };

    // Test with "Language" filter value (should behave like no filter)
    let params_language = h::get_contains_params_with_lang(Some("Language".to_string()));

    let mut query_task_language = SearchQueryTask::new(
        &app_data.dbm,
        query.to_string(),
        params_language,
        SearchArea::Suttas,
    );

    let _results_language = match query_task_language.results_page(0) {
        Ok(x) => x,
        Err(s) => {
            panic!("{}", s);
        }
    };

    // The actual test: verify that both None and "Language" return the same total count,
    // so both are likely returning the same results across all languages (not filtering).
    let total_none = query_task_none.total_hits();
    let total_language = query_task_language.total_hits();

    assert!(total_none > 0, "No filter should return results");
    assert!(total_language > 0, "'Language' filter should return results");
    assert_eq!(total_none, total_language,
               "No filter ({}) and 'Language' filter ({}) should return the same number of total hits",
               total_none, total_language);

    // Verify that we have results from multiple languages by comparing with a Pali-only filter
    let params_pli = h::get_contains_params_with_lang(Some("pli".to_string()));
    let mut query_task_pli = SearchQueryTask::new(
        &app_data.dbm,
        query.to_string(),
        params_pli,
        SearchArea::Suttas,
    );
    let _results_pli = query_task_pli.results_page(0).unwrap();
    let total_pli = query_task_pli.total_hits();

    // The unfiltered results should have MORE results than Pali-only
    assert!(total_none > total_pli,
            "Unfiltered search ({}) should return more results than Pali-only ({})",
            total_none, total_pli);
}

#[test]
#[serial]
fn test_sutta_uid_range_match() {
    h::app_data_setup();
    let app_data = get_app_data();
    let db_conn = &mut app_data.dbm.appdata.get_conn().unwrap();

    // Insert test suttas with range UIDs
    // Example: sn30.7-16 should match queries for sn30.7, sn30.10, sn30.16, etc.
    let test_sutta_range = NewSutta {
        uid: "sn30.7-16/pli/ms",
        sutta_ref: "SN 30.7-16",
        nikaya: "sn",
        language: "pli",
        group_path: None,
        group_index: None,
        order_index: None,
        sutta_range_group: Some("sn30"),
        sutta_range_start: Some(7),
        sutta_range_end: Some(16),
        title: Some("Test Range Sutta"),
        title_ascii: Some("Test Range Sutta"),
        title_pali: None,
        title_trans: None,
        description: None,
        content_plain: Some("Test content for sn30.7-16 range"),
        content_html: None,
        content_json: None,
        content_json_tmpl: None,
        source_uid: Some("ms"),
        source_info: None,
        source_language: None,
        message: None,
        copyright: None,
        license: None,
    };

    // Insert the test sutta
    diesel::insert_into(suttas::table)
        .values(&test_sutta_range)
        .execute(db_conn)
        .unwrap();

    let params = h::get_uid_params_with_lang(Some("pli".to_string()));

    // Test query for sn30.10 which should match the range sn30.7-16
    let query = "sn30.10";
    let mut query_task = SearchQueryTask::new(
        &app_data.dbm,
        query.to_string(),
        params.clone(),
        SearchArea::Suttas,
    );

    let results = match query_task.results_page(0) {
        Ok(x) => x,
        Err(s) => {
            panic!("{}", s);
        }
    };

    assert!(!results.is_empty(), "Should find sutta with range sn30.7-16 for query sn30.10");
    assert_eq!(results[0].uid, "sn30.7-16/pli/ms");
    assert_eq!(results[0].title, "Test Range Sutta");

    // Test query for sn30.7 (start of range)
    let query_start = "sn30.7";
    let mut query_task_start = SearchQueryTask::new(
        &app_data.dbm,
        query_start.to_string(),
        params.clone(),
        SearchArea::Suttas,
    );

    let results_start = query_task_start.results_page(0).unwrap();
    assert!(!results_start.is_empty(), "Should find sutta for start of range sn30.7");
    assert_eq!(results_start[0].uid, "sn30.7-16/pli/ms");

    // Test query for sn30.16 (end of range)
    let query_end = "sn30.16";
    let mut query_task_end = SearchQueryTask::new(
        &app_data.dbm,
        query_end.to_string(),
        params.clone(),
        SearchArea::Suttas,
    );

    let results_end = query_task_end.results_page(0).unwrap();
    assert!(!results_end.is_empty(), "Should find sutta for end of range sn30.16");
    assert_eq!(results_end[0].uid, "sn30.7-16/pli/ms");

    // Test query outside the range (sn30.6 and sn30.17) should not match
    let query_before = "sn30.6";
    let mut query_task_before = SearchQueryTask::new(
        &app_data.dbm,
        query_before.to_string(),
        params.clone(),
        SearchArea::Suttas,
    );

    let results_before = query_task_before.results_page(0).unwrap();
    // Should not find the range sutta, might find others via LIKE query
    let has_range_sutta = results_before.iter().any(|r| r.uid == "sn30.7-16/pli/ms");
    assert!(!has_range_sutta, "Should not find sutta with range sn30.7-16 for query sn30.6");

    // Clean up test data
    diesel::delete(suttas::table)
        .filter(suttas::uid.eq("sn30.7-16/pli/ms"))
        .execute(db_conn)
        .unwrap();
}

#[test]
#[serial]
fn test_sutta_uid_range_match_an() {
    h::app_data_setup();
    let app_data = get_app_data();
    let db_conn = &mut app_data.dbm.appdata.get_conn().unwrap();

    // Insert test sutta with an2.32-41 range
    let test_sutta_an = NewSutta {
        uid: "an2.32-41/pli/ms",
        sutta_ref: "AN 2.32-41",
        nikaya: "an",
        language: "pli",
        group_path: None,
        group_index: None,
        order_index: None,
        sutta_range_group: Some("an2"),
        sutta_range_start: Some(32),
        sutta_range_end: Some(41),
        title: Some("Test AN Range Sutta"),
        title_ascii: Some("Test AN Range Sutta"),
        title_pali: None,
        title_trans: None,
        description: None,
        content_plain: Some("Test content for an2.32-41 range"),
        content_html: None,
        content_json: None,
        content_json_tmpl: None,
        source_uid: Some("ms"),
        source_info: None,
        source_language: None,
        message: None,
        copyright: None,
        license: None,
    };

    diesel::insert_into(suttas::table)
        .values(&test_sutta_an)
        .execute(db_conn)
        .unwrap();

    let params = h::get_uid_params_with_lang(Some("pli".to_string()));

    // Test query for an2.33 which should match the range an2.32-41
    let query = "an2.33";
    let mut query_task = SearchQueryTask::new(
        &app_data.dbm,
        query.to_string(),
        params.clone(),
        SearchArea::Suttas,
    );

    let results = match query_task.results_page(0) {
        Ok(x) => x,
        Err(s) => {
            panic!("{}", s);
        }
    };

    assert!(!results.is_empty(), "Should find sutta with range an2.32-41 for query an2.33");
    assert_eq!(results[0].uid, "an2.32-41/pli/ms");
    assert_eq!(results[0].title, "Test AN Range Sutta");

    // Test query for an2.40 (within range)
    let query_mid = "an2.40";
    let mut query_task_mid = SearchQueryTask::new(
        &app_data.dbm,
        query_mid.to_string(),
        params.clone(),
        SearchArea::Suttas,
    );

    let results_mid = query_task_mid.results_page(0).unwrap();
    assert!(!results_mid.is_empty(), "Should find sutta for an2.40 within range");
    assert_eq!(results_mid[0].uid, "an2.32-41/pli/ms");

    // Clean up test data
    diesel::delete(suttas::table)
        .filter(suttas::uid.eq("an2.32-41/pli/ms"))
        .execute(db_conn)
        .unwrap();
}

#[test]
#[serial]
fn test_sutta_uid_range_match_more_cases() {
    h::app_data_setup();
    let app_data = get_app_data();
    let db_conn = &mut app_data.dbm.appdata.get_conn().unwrap();

    let test_uids = vec![
        "dummy-sn17.13-20/pli/ms",
        "dummy-sn12.72-81/pli/ms",
        "dummy-an11.22-29/pli/ms",
    ];

    // Cleanup first
    diesel::delete(suttas::table)
        .filter(suttas::uid.eq_any(&test_uids))
        .execute(db_conn)
        .unwrap();

    let test_suttas = vec![
        NewSutta {
            uid: test_uids[0],
            sutta_ref: "SN 17.13-20",
            nikaya: "sn",
            language: "pli",
            group_path: None,
            group_index: None,
            order_index: None,
            sutta_range_group: Some("dummy-sn17"),
            sutta_range_start: Some(13),
            sutta_range_end: Some(20),
            title: Some("SN 17.13-20"),
            title_ascii: Some("SN 17.13-20"),
            title_pali: None,
            title_trans: None,
            description: None,
            content_plain: Some("Content 1"),
            content_html: None,
            content_json: None,
            content_json_tmpl: None,
            source_uid: Some("ms"),
            source_info: None,
            source_language: None,
            message: None,
            copyright: None,
            license: None,
        },
        NewSutta {
            uid: test_uids[1],
            sutta_ref: "SN 12.72-81",
            nikaya: "sn",
            language: "pli",
            group_path: None,
            group_index: None,
            order_index: None,
            sutta_range_group: Some("dummy-sn12"),
            sutta_range_start: Some(72),
            sutta_range_end: Some(81),
            title: Some("SN 12.72-81"),
            title_ascii: Some("SN 12.72-81"),
            title_pali: None,
            title_trans: None,
            description: None,
            content_plain: Some("Content 2"),
            content_html: None,
            content_json: None,
            content_json_tmpl: None,
            source_uid: Some("ms"),
            source_info: None,
            source_language: None,
            message: None,
            copyright: None,
            license: None,
        },
        NewSutta {
            uid: test_uids[2],
            sutta_ref: "AN 11.22-29",
            nikaya: "an",
            language: "pli",
            group_path: None,
            group_index: None,
            order_index: None,
            sutta_range_group: Some("dummy-an11"),
            sutta_range_start: Some(22),
            sutta_range_end: Some(29),
            title: Some("AN 11.22-29"),
            title_ascii: Some("AN 11.22-29"),
            title_pali: None,
            title_trans: None,
            description: None,
            content_plain: Some("Content 3"),
            content_html: None,
            content_json: None,
            content_json_tmpl: None,
            source_uid: Some("ms"),
            source_info: None,
            source_language: None,
            message: None,
            copyright: None,
            license: None,
        },
    ];

    diesel::insert_into(suttas::table)
        .values(&test_suttas)
        .execute(db_conn)
        .unwrap();

    let params = h::get_uid_params_with_lang(Some("pli".to_string()));

    // Test Case 1: "dummy-sn 17.20" -> dummy-sn17.13-20
    let query1 = "dummy-sn 17.20";
    let mut task1 = SearchQueryTask::new(
        &app_data.dbm,
        query1.to_string(),
        params.clone(),
        SearchArea::Suttas,
    );
    let results1 = task1.results_page(0).unwrap();
    assert!(!results1.is_empty(), "Failed to find 'dummy-sn 17.20'");
    assert_eq!(results1[0].uid, test_uids[0]);

    // Test Case 2: "dummy-sn 12.75" -> dummy-sn12.72-81
    let query2 = "dummy-sn 12.75";
    let mut task2 = SearchQueryTask::new(
        &app_data.dbm,
        query2.to_string(),
        params.clone(),
        SearchArea::Suttas,
    );
    let results2 = task2.results_page(0).unwrap();
    assert!(!results2.is_empty(), "Failed to find 'dummy-sn 12.75'");
    assert_eq!(results2[0].uid, test_uids[1]);

    // Test Case 3: "dummy-an 11.29" -> dummy-an11.22-29
    let query3 = "dummy-an 11.29";
    let mut task3 = SearchQueryTask::new(
        &app_data.dbm,
        query3.to_string(),
        params.clone(),
        SearchArea::Suttas,
    );
    let results3 = task3.results_page(0).unwrap();
    assert!(!results3.is_empty(), "Failed to find 'dummy-an 11.29'");
    assert_eq!(results3[0].uid, test_uids[2]);

    // Clean up
    diesel::delete(suttas::table)
        .filter(suttas::uid.eq_any(&test_uids))
        .execute(db_conn)
        .unwrap();
}

#[test]
#[serial]
fn test_sutta_uid_range_with_language_filter() {
    h::app_data_setup();
    let app_data = get_app_data();
    let db_conn = &mut app_data.dbm.appdata.get_conn().unwrap();

    // Insert test suttas with same range but different languages
    let test_sutta_pli = NewSutta {
        uid: "sn30.7-16/pli/ms",
        sutta_ref: "SN 30.7-16",
        nikaya: "sn",
        language: "pli",
        group_path: None,
        group_index: None,
        order_index: None,
        sutta_range_group: Some("sn30"),
        sutta_range_start: Some(7),
        sutta_range_end: Some(16),
        title: Some("Test Pali Range Sutta"),
        title_ascii: Some("Test Pali Range Sutta"),
        title_pali: None,
        title_trans: None,
        description: None,
        content_plain: Some("Pali content for sn30.7-16 range"),
        content_html: None,
        content_json: None,
        content_json_tmpl: None,
        source_uid: Some("ms"),
        source_info: None,
        source_language: None,
        message: None,
        copyright: None,
        license: None,
    };

    let test_sutta_en = NewSutta {
        uid: "sn30.7-16/en/sujato",
        sutta_ref: "SN 30.7-16",
        nikaya: "sn",
        language: "en",
        group_path: None,
        group_index: None,
        order_index: None,
        sutta_range_group: Some("sn30"),
        sutta_range_start: Some(7),
        sutta_range_end: Some(16),
        title: Some("Test English Range Sutta"),
        title_ascii: Some("Test English Range Sutta"),
        title_pali: None,
        title_trans: None,
        description: None,
        content_plain: Some("English content for sn30.7-16 range"),
        content_html: None,
        content_json: None,
        content_json_tmpl: None,
        source_uid: Some("sujato"),
        source_info: None,
        source_language: None,
        message: None,
        copyright: None,
        license: None,
    };

    diesel::insert_into(suttas::table)
        .values(&test_sutta_pli)
        .execute(db_conn)
        .unwrap();

    diesel::insert_into(suttas::table)
        .values(&test_sutta_en)
        .execute(db_conn)
        .unwrap();

    // Test with Pali language filter
    let params_pli = h::get_uid_params_with_lang(Some("pli".to_string()));
    let query = "sn30.10";
    let mut query_task = SearchQueryTask::new(
        &app_data.dbm,
        query.to_string(),
        params_pli,
        SearchArea::Suttas,
    );

    let results = query_task.results_page(0).unwrap();
    assert!(!results.is_empty(), "Should find Pali sutta");
    assert_eq!(results[0].uid, "sn30.7-16/pli/ms");
    assert_eq!(results[0].lang, Some("pli".to_string()));

    // Test with English language filter
    let params_en = h::get_uid_params_with_lang(Some("en".to_string()));
    let mut query_task_en = SearchQueryTask::new(
        &app_data.dbm,
        query.to_string(),
        params_en,
        SearchArea::Suttas,
    );

    let results_en = query_task_en.results_page(0).unwrap();
    assert!(!results_en.is_empty(), "Should find English sutta");
    assert_eq!(results_en[0].uid, "sn30.7-16/en/sujato");
    assert_eq!(results_en[0].lang, Some("en".to_string()));

    // Clean up test data
    diesel::delete(suttas::table)
        .filter(suttas::uid.eq("sn30.7-16/pli/ms"))
        .execute(db_conn)
        .unwrap();
    diesel::delete(suttas::table)
        .filter(suttas::uid.eq("sn30.7-16/en/sujato"))
        .execute(db_conn)
        .unwrap();
}

#[test]
#[serial]
fn test_book_uid_query_returns_all_spine_items() {
    use simsapa_backend::db::appdata_schema::{books, book_spine_items};
    use simsapa_backend::db::appdata_models::{NewBook, NewBookSpineItem};

    h::app_data_setup();
    let app_data = get_app_data();
    let db_conn = &mut app_data.dbm.appdata.get_conn().unwrap();

    // Clean up any existing test data
    let _ = diesel::delete(books::table.filter(books::uid.eq("test-book-uid")))
        .execute(db_conn);

    // Insert a test book
    let new_book = NewBook {
        uid: "test-book-uid",
        document_type: "epub",
        title: Some("Test Book"),
        author: None,
        language: None,
        file_path: None,
        metadata_json: None,
        enable_embedded_css: false,
        toc_json: None,
    };

    diesel::insert_into(books::table)
        .values(&new_book)
        .execute(db_conn)
        .unwrap();

    // Get the book_id for foreign key
    let book_id: i32 = books::table
        .filter(books::uid.eq("test-book-uid"))
        .select(books::id)
        .first(db_conn)
        .unwrap();

    // Insert multiple spine items for this book
    for i in 0..3 {
        let spine_uid = format!("test-book-uid.{}", i);
        let resource_path = format!("chapter{}.html", i);
        let title_str = format!("Chapter {}", i + 1);
        let content_html_str = format!("<p>Content for chapter {}</p>", i);
        let content_plain_str = format!("Content for chapter {}", i);

        let spine_item = NewBookSpineItem {
            book_id,
            book_uid: "test-book-uid",
            spine_item_uid: &spine_uid,
            spine_index: i,
            resource_path: &resource_path,
            title: Some(&title_str),
            language: None,
            content_html: Some(&content_html_str),
            content_plain: Some(&content_plain_str),
        };

        diesel::insert_into(book_spine_items::table)
            .values(&spine_item)
            .execute(db_conn)
            .unwrap();
    }

    // Test query with book_uid (no dot) - should return all spine items
    let query = "uid:test-book-uid";
    let params = h::get_uid_params();
    let mut query_task = SearchQueryTask::new(
        &app_data.dbm,
        query.to_string(),
        params,
        SearchArea::Library,
    );

    let results = query_task.results_page(0).unwrap();

    // Should find all 3 spine items
    assert_eq!(results.len(), 3, "Should find all 3 spine items for book_uid");
    assert_eq!(results[0].uid, "test-book-uid.0");
    assert_eq!(results[1].uid, "test-book-uid.1");
    assert_eq!(results[2].uid, "test-book-uid.2");
    assert_eq!(results[0].title, "Chapter 1");
    assert_eq!(results[1].title, "Chapter 2");
    assert_eq!(results[2].title, "Chapter 3");

    // Clean up
    diesel::delete(books::table.filter(books::uid.eq("test-book-uid")))
        .execute(db_conn)
        .unwrap();
}

#[test]
#[serial]
fn test_book_spine_item_uid_query_returns_single_item() {
    use simsapa_backend::db::appdata_schema::{books, book_spine_items};
    use simsapa_backend::db::appdata_models::{NewBook, NewBookSpineItem};

    h::app_data_setup();
    let app_data = get_app_data();
    let db_conn = &mut app_data.dbm.appdata.get_conn().unwrap();

    // Clean up any existing test data
    let _ = diesel::delete(books::table.filter(books::uid.eq("test-book-spine")))
        .execute(db_conn);

    // Insert a test book
    let new_book = NewBook {
        uid: "test-book-spine",
        document_type: "epub",
        title: Some("Test Book Spine"),
        author: None,
        language: None,
        file_path: None,
        metadata_json: None,
        enable_embedded_css: false,
        toc_json: None,
    };

    diesel::insert_into(books::table)
        .values(&new_book)
        .execute(db_conn)
        .unwrap();

    let book_id: i32 = books::table
        .filter(books::uid.eq("test-book-spine"))
        .select(books::id)
        .first(db_conn)
        .unwrap();

    // Insert multiple spine items
    for i in 0..3 {
        let spine_uid = format!("test-book-spine.{}", i);
        let resource_path = format!("section{}.html", i);
        let title_str = format!("Section {}", i + 1);
        let content_html_str = format!("<p>Section {} content</p>", i);
        let content_plain_str = format!("Section {} content", i);

        let spine_item = NewBookSpineItem {
            book_id,
            book_uid: "test-book-spine",
            spine_item_uid: &spine_uid,
            spine_index: i,
            resource_path: &resource_path,
            title: Some(&title_str),
            language: None,
            content_html: Some(&content_html_str),
            content_plain: Some(&content_plain_str),
        };

        diesel::insert_into(book_spine_items::table)
            .values(&spine_item)
            .execute(db_conn)
            .unwrap();
    }

    // Test query with spine_item_uid (with dot) - should return only that specific item
    let query = "uid:test-book-spine.1";
    let params = h::get_uid_params();
    let mut query_task = SearchQueryTask::new(
        &app_data.dbm,
        query.to_string(),
        params,
        SearchArea::Library,
    );

    let results = query_task.results_page(0).unwrap();

    // Should find only the specific spine item
    assert_eq!(results.len(), 1, "Should find only 1 spine item for spine_item_uid");
    assert_eq!(results[0].uid, "test-book-spine.1");
    assert_eq!(results[0].title, "Section 2");

    // Clean up
    diesel::delete(books::table.filter(books::uid.eq("test-book-spine")))
        .execute(db_conn)
        .unwrap();
}
