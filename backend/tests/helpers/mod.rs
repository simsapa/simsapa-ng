// use std::env;

use dotenvy::dotenv;

use simsapa_backend::{init_app_data, get_app_data, get_app_globals};
use simsapa_backend::query_task::SearchQueryTask;
use simsapa_backend::types::{SearchArea, SearchMode, SearchParams};

pub fn app_data_setup() {
    // unsafe { env::set_var("SIMSAPA_DIR", "../../assets-testing/"); }
    dotenv().ok();
    init_app_data();
}

#[allow(dead_code)]
pub fn get_contains_params_with_lang(lang: Option<String>) -> SearchParams {
    SearchParams {
        mode: SearchMode::ContainsMatch,
        page_len: None,
        lang,
        lang_include: true,
        source: None,
        source_include: true,
        enable_regex: false,
        fuzzy_distance: 0,
    }
}

#[allow(dead_code)]
pub fn get_uid_params_with_lang(lang: Option<String>) -> SearchParams {
    SearchParams {
        mode: SearchMode::UidMatch,
        page_len: None,
        lang,
        lang_include: true,
        source: None,
        source_include: true,
        enable_regex: false,
        fuzzy_distance: 0,
    }
}

#[allow(dead_code)]
pub fn get_uid_params() -> SearchParams {
    SearchParams {
        mode: SearchMode::UidMatch,
        page_len: None,
        lang: Some("en".to_string()),
        lang_include: true,
        source: None,
        source_include: true,
        enable_regex: false,
        fuzzy_distance: 0,
    }
}

#[allow(dead_code)]
pub fn create_test_task(query_text: &str, search_mode: SearchMode) -> SearchQueryTask {
    let app_data = get_app_data();
    let g = get_app_globals();

    let params = SearchParams {
        mode: search_mode,
        page_len: Some(g.page_len),
        lang: Some("en".to_string()),
        lang_include: false,
        source: None,
        source_include: false,
        enable_regex: false,
        fuzzy_distance: 0,
    };

    SearchQueryTask::new(
        &app_data.dbm,
        query_text.to_string(),
        params,
        SearchArea::Suttas,
    )
}
