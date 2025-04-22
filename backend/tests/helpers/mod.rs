use std::env;

use simsapa_backend::types::{SearchMode, SearchParams};

pub fn appdata_db_setup() {
    unsafe { env::set_var("DATABASE_PATH", "../appdata.sqlite3"); }
}

pub fn get_contains_params() -> SearchParams {
    SearchParams {
        mode: SearchMode::ContainsMatch,
        page_len: None,
        lang: Some("en".to_string()),
        lang_include: true,
        source: None,
        source_include: true,
        enable_regex: false,
        fuzzy_distance: 0,
    }
}
