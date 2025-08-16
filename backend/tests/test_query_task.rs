use std::collections::HashMap;

use simsapa_backend::types::{SearchArea, SearchMode};
use simsapa_backend::query_task::SearchQueryTask;
use simsapa_backend::get_app_data;

mod helpers;
use helpers as h;

#[test]
fn test_highlight_text_simple() {
    h::app_data_setup();
    let task = h::create_test_task("satipaṭṭhā", SearchMode::ContainsMatch);
    let content = "sīlaṁ nissāya sīle patiṭṭhāya cattāro satipaṭṭhāne bhāveyyāsi";
    let highlighted = task.highlight_text(&task.query_text, content).unwrap();
    assert_eq!(highlighted, "sīlaṁ nissāya sīle patiṭṭhāya cattāro <span class='match'>satipaṭṭhā</span>ne bhāveyyāsi");
}

#[test]
fn test_highlight_text_uppercase() {
    h::app_data_setup();
    let task = h::create_test_task("SATIpaṭṭhā", SearchMode::ContainsMatch);
    let content = "sīlaṁ nissāya sīle patiṭṭhāya cattāro satipaṭṭhāne bhāveyyāsi";
    let highlighted = task.highlight_text(&task.query_text, content).unwrap();
    assert_eq!(highlighted, "sīlaṁ nissāya sīle patiṭṭhāya cattāro <span class='match'>satipaṭṭhā</span>ne bhāveyyāsi");
}

#[test]
fn test_highlight_text_regex_special_chars() {
    h::app_data_setup();
    let task = h::create_test_task("test", SearchMode::ContainsMatch);
    let content = "This has regex .*+ chars";
    let highlighted = task.highlight_text(".*+", content).unwrap();
    assert_eq!(highlighted, "This has regex <span class='match'>.*+</span> chars");
}

#[test]
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
fn test_sutta_search_contains_match() {
    h::app_data_setup();
    let app_data = get_app_data();
    let params = h::get_contains_params();

    let query = "satipaṭṭhāna";

    let mut query_task = SearchQueryTask::new(
        &app_data.dbm,
        "en".to_string(),
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
    assert_eq!(results[0].uid, "mil5.3.7/en/tw_rhysdavids");
    assert!(results[0].snippet.starts_with("... accordance with the rules of <span class='match'>satipaṭṭhāna</span>"));
    assert!(results[0].snippet.ends_with("law of property to carry on the traditions of the khattiya clans and to fight ..."));
}

#[test]
fn test_sutta_search_contains_match_with_punctuation() {
    h::app_data_setup();
    let app_data = get_app_data();
    let params = h::get_contains_params();

    let mut queries: HashMap<&str, &str> = HashMap::new();
    queries.insert("Anāsavañca vo, bhikkhave, desessāmi",
                   "sn43.14-43/pli/ms");
    queries.insert("padakkhiṇaṁ mano-kammaṁ",
                   "an3.155/pli/ms");
    queries.insert("saraṇaṁ…pe॰…anusāsanī’’ti?",
                   "sn43.14/pli/cst4");
    queries.insert("katamañca, bhikkhave, nibbānaṁ…pe॰… abyāpajjhañca [abyāpajjhañca (sī॰ syā॰ kaṁ॰ pī॰)] vo, bhikkhave, desessāmi abyāpajjhagāmiñca maggaṁ.",
                   "sn43.14/pli/cst4");

    for (query_text, first_result_uid) in queries.into_iter() {
        let mut query_task = SearchQueryTask::new(
            &app_data.dbm,
            "en".to_string(),
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
fn test_dict_word_search_contains_match() {
    h::app_data_setup();
    let app_data = get_app_data();
    let params = h::get_uid_params();

    let query = "awakening factor of enlightenment";

    let mut query_task = SearchQueryTask::new(
        &app_data.dbm,
        "en".to_string(),
        query.to_string(),
        params,
        SearchArea::DictWords,
    );

    let results = match query_task.results_page(0) {
        Ok(x) => x,
        Err(s) => {
            panic!("{}", s);
        }
    };

    // println!("{}", results[0].snippet);

    assert_eq!(results[0].uid, "sambojjhaṅga/dpd");
    assert!(results[0].snippet.starts_with("masc element of <span class='match'>awakening factor of enlightenment</span>"));
}

#[test]
fn test_dict_word_uid_match() {
    h::app_data_setup();
    let app_data = get_app_data();
    let params = h::get_uid_params();

    let query = "satipaṭṭhāna 1/dpd";

    let mut query_task = SearchQueryTask::new(
        &app_data.dbm,
        "en".to_string(),
        query.to_string(),
        params,
        SearchArea::DictWords,
    );

    let results = match query_task.results_page(0) {
        Ok(x) => x,
        Err(s) => {
            panic!("{}", s);
        }
    };

    // println!("{}", results[0].snippet);

    assert_eq!(results[0].uid, "satipaṭṭhāna 1/dpd");
    assert!(results[0].snippet.starts_with("masc attending mindfully being present with mindfulness [sati + upaṭṭhāna]"));
}
