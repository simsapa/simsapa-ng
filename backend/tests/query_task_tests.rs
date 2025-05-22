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
}
