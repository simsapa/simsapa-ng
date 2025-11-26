use std::collections::HashMap;

use serial_test::serial;
use simsapa_backend::types::{SearchArea, SearchMode};
use simsapa_backend::query_task::SearchQueryTask;
use simsapa_backend::get_app_data;

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
