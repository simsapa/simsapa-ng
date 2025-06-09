use simsapa_backend::types::SearchArea;
use simsapa_backend::query_task::SearchQueryTask;
use simsapa_backend::db;

mod helpers;
use helpers as h;

#[test]
fn test_sutta_search_contains_match() {
    h::appdata_db_setup();
    let dbm = db::get_dbm();

    let params = h::get_contains_params();

    let query = "satipaṭṭhāna";

    let mut query_task = SearchQueryTask::new(
        dbm,
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

    assert_eq!(results[0].uid, "mil5.3.7/en/tw_rhysdavids");
    assert!(results[0].snippet.starts_with("... accordance with the rules of <span class='match'>satipaṭṭhāna</span>"));
    assert!(results[0].snippet.ends_with("law of property to carry on the traditions of the khattiya clans and to fight ..."));
}

#[test]
fn test_dict_word_search_contains_match() {
    h::appdata_db_setup();
    let dbm = db::get_dbm();
    let params = h::get_uid_params();

    let query = "awakening factor of enlightenment";

    let mut query_task = SearchQueryTask::new(
        dbm,
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
    h::appdata_db_setup();
    let dbm = db::get_dbm();
    let params = h::get_uid_params();

    let query = "satipaṭṭhāna 1/dpd";

    let mut query_task = SearchQueryTask::new(
        dbm,
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
