use crate::models_appdata::*;
use crate::models_dictionaries::*;
use crate::schema_appdata::suttas::dsl::*;
use diesel::prelude::*;
// use diesel::sqlite::Sqlite;

use std::env;
use std::path::PathBuf;
use std::fs;

use dotenvy::dotenv;
use regex::Regex;

use crate::get_create_simsapa_app_root;

/// Returns connections to appdata.sqlite3 and dictionaries.sqlite3
pub fn establish_connection() -> (SqliteConnection, SqliteConnection) {
    dotenv().ok();

    let simsapa_dir = match env::var("SIMSAPA_DIR") {
        Ok(s) => PathBuf::from(s),
        Err(_) => {
            if let Ok(p) = get_create_simsapa_app_root() {
                p
            } else {
                PathBuf::from(".")
            }
        }
    };

    let app_assets_dir = simsapa_dir.join("app-assets");
    let appdata_db_path = app_assets_dir.join("appdata.sqlite3");
    let dict_db_path = app_assets_dir.join("dictionaries.sqlite3");

    if !appdata_db_path.exists() {
        panic!("Appdata database file not found at expected location: {:?}", appdata_db_path);
    }

    if !dict_db_path.exists() {
        panic!("Dictionary database file not found at expected location: {:?}", dict_db_path);
    }

    // if !db_path.exists() {
    //     panic!("File not found: {}", db_path.as_os_str().to_str().expect("os_str Error!"));
    // }

    let appdata_abs_path = fs::canonicalize(appdata_db_path.clone()).unwrap_or(appdata_db_path);
    let appdata_database_url = format!("sqlite://{}", appdata_abs_path.as_os_str().to_str().expect("os_str Error!"));
    let appdata_conn = SqliteConnection::establish(&appdata_database_url)
        .unwrap_or_else(|_| panic!("Error connecting to {}", appdata_database_url));

    let dict_abs_path = fs::canonicalize(dict_db_path.clone()).unwrap_or(dict_db_path);
    let dict_database_url = format!("sqlite://{}", dict_abs_path.as_os_str().to_str().expect("os_str Error!"));
    let dict_conn = SqliteConnection::establish(&dict_database_url)
        .unwrap_or_else(|_| panic!("Error connecting to {}", dict_database_url));

    (appdata_conn, dict_conn)
}

pub fn create_dictionary(db_conn: &mut SqliteConnection,
                         new_dict: NewDictionary) -> Result<Dictionary, diesel::result::Error> {
    use crate::schema_dictionaries::dictionaries;
    diesel::insert_into(dictionaries::table)
        .values(&new_dict)
        .returning(Dictionary::as_returning())
        .get_result(db_conn)
}

pub fn delete_dictionary_by_label(db_conn: &mut SqliteConnection,
                                  dict_label: &str) -> Result<usize, diesel::result::Error> {
    use crate::schema_dictionaries::dictionaries::dsl::*;

    let res = diesel::delete(dictionaries.filter(label.eq(dict_label)))
        .execute(db_conn);
    res
}

pub fn create_dict_word(db_conn: &mut SqliteConnection,
                        new_dict_word: &NewDictWord) -> Result<DictWord, diesel::result::Error> {
    use crate::schema_dictionaries::dict_words;
    diesel::insert_into(dict_words::table)
        .values(new_dict_word)
        .returning(DictWord::as_returning())
        .get_result(db_conn)
}

pub fn create_dict_words_batch(db_conn: &mut SqliteConnection,
                               new_words: &[NewDictWord]) -> Result<usize, diesel::result::Error> {
    use crate::schema_dictionaries::dict_words;
    diesel::insert_into(dict_words::table)
        .values(new_words)
        .execute(db_conn)
}

// pub fn publish_post() {
//     use crate::appdata_schema::posts::dsl::{posts, published};

//     let post_id: i32 = 2;
//     let connection = &mut establish_connection();

//     let post = diesel::update(posts.find(post_id))
//         .set(published.eq(true))
//         .returning(Post::as_returning())
//         .get_result(connection)
//         .unwrap();

//     println!("Published post {}", post.title);
// }

pub fn get_sutta(sutta_uid: &str) -> Option<Sutta> {
    use crate::schema_appdata::suttas::dsl::suttas;

    let (conn, _) = &mut establish_connection();

    let sutta = suttas
        .filter(uid.eq(sutta_uid))
        .select(Sutta::as_select())
        .first(conn)
        .optional();

    match sutta {
        Ok(x) => x,
        Err(e) => {
            eprintln!("{}", e);
            None
        },
    }
}

pub fn delete_sutta() {
    use crate::schema_appdata::suttas::dsl::*;

    let pattern = "unwholesome";

    let (conn, _) = &mut establish_connection();
    let num_deleted = diesel::delete(suttas.filter(content_html.like(pattern)))
        .execute(conn)
        .expect("Error deleting suttas");

    println!("Deleted {} suttas", num_deleted);
}

fn sort_suttas(res: Vec<Sutta>) -> Vec<Sutta> {
    // Sort Pali ms first as the results.
    // Then add Pali other sources,
    // then the non-Pali items, sorted by language.
    //
    // Single-pass manual bucketing means we walk the vector once,
    // avoiding per-element cloning.

    let mut results = Vec::new();
    let mut pli_others = Vec::new();
    let mut remaining = Vec::new();

    for s in res.into_iter() {
        if s.language == "pli" {
            if s.uid.ends_with("/ms") {
                results.push(s);
            } else {
                pli_others.push(s);
            }
        } else {
            remaining.push(s);
        }
    }

    // Sort non-pli by language
    remaining.sort_by(|a, b| a.language.cmp(&b.language));
    // Assemble final list
    results.extend(pli_others);
    results.extend(remaining);
    results
}

pub fn get_translations_for_sutta_uid(sutta_uid: &str) -> Vec<String> {
    // See sutta_search_window_state.py::_add_related_tabs()

    // Capture the reference before the first '/'
    let re = Regex::new(r"^([^/]+)/.*").expect("Invalid regex");
    let uid_ref = re.replace(&sutta_uid, "$1").to_string();

    use crate::schema_appdata::suttas::dsl::suttas;

    let (conn, _) = &mut establish_connection();

    let mut res: Vec<Sutta> = Vec::new();

    if let Ok(a) = suttas
        .select(Sutta::as_select())
        .filter(uid.ne(sutta_uid))
        .filter(uid.like(format!("{}/%", uid_ref)))
        .load(conn) {
            res.extend(a);
    }

    let res_sorted_uids: Vec<String> = sort_suttas(res)
        .into_iter().map(|s| s.uid).collect();

    res_sorted_uids
}
