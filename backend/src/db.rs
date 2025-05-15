use std::env;
use std::path::PathBuf;
use std::fs;
use std::collections::HashSet;

use diesel::prelude::*;
use diesel::sql_types::{Text, Integer};
// use diesel::sqlite::Sqlite;

use lazy_static::lazy_static;
use dotenvy::dotenv;
use regex::Regex;

use crate::models_appdata::*;
use crate::models_dictionaries::*;
use crate::models_dpd::{DpdRoot, DpdHeadword, Lookup, UDpdWord};
use crate::schema_appdata::suttas::dsl::*;
use crate::{get_create_simsapa_app_root, get_create_simsapa_app_assets_path};
use crate::helpers::{word_uid, pali_to_ascii, strip_html, root_info_clean_plaintext};
use crate::pali_stemmer::pali_stem;
use crate::types::SearchResult;

/// Returns connections as a tuple to appdata.sqlite3, dictionaries.sqlite3, dpd.sqlite3
pub fn establish_connection() -> (SqliteConnection, SqliteConnection, SqliteConnection) {
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
    let dpd_db_path = app_assets_dir.join("dpd.sqlite3");

    if !appdata_db_path.exists() {
        panic!("Appdata database file not found at expected location: {:?}", appdata_db_path);
    }

    if !dict_db_path.exists() {
        panic!("Dictionary database file not found at expected location: {:?}", dict_db_path);
    }

    if !dpd_db_path.exists() {
        panic!("Dictionary database file not found at expected location: {:?}", dpd_db_path);
    }

    let appdata_abs_path = fs::canonicalize(appdata_db_path.clone()).unwrap_or(appdata_db_path);
    let appdata_database_url = format!("sqlite://{}", appdata_abs_path.as_os_str().to_str().expect("os_str Error!"));
    let appdata_conn = SqliteConnection::establish(&appdata_database_url)
        .unwrap_or_else(|_| panic!("Error connecting to {}", appdata_database_url));

    let dict_abs_path = fs::canonicalize(dict_db_path.clone()).unwrap_or(dict_db_path);
    let dict_database_url = format!("sqlite://{}", dict_abs_path.as_os_str().to_str().expect("os_str Error!"));
    let dict_conn = SqliteConnection::establish(&dict_database_url)
        .unwrap_or_else(|_| panic!("Error connecting to {}", dict_database_url));

    let dpd_abs_path = fs::canonicalize(dpd_db_path.clone()).unwrap_or(dpd_db_path);
    let dpd_database_url = format!("sqlite://{}", dpd_abs_path.as_os_str().to_str().expect("os_str Error!"));
    let dpd_conn = SqliteConnection::establish(&dpd_database_url)
        .unwrap_or_else(|_| panic!("Error connecting to {}", dpd_database_url));

    (appdata_conn, dict_conn, dpd_conn)
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
                                  dict_label_val: &str) -> Result<usize, diesel::result::Error> {
    use crate::schema_dictionaries::dictionaries::dsl::*;

    let res = diesel::delete(dictionaries.filter(label.eq(dict_label_val)))
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

    let (conn, _, _) = &mut establish_connection();

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

    let (conn, _, _) = &mut establish_connection();
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

    let (db_conn, _, _) = &mut establish_connection();

    let mut res: Vec<Sutta> = Vec::new();

    if let Ok(a) = suttas
        .select(Sutta::as_select())
        .filter(uid.ne(sutta_uid))
        .filter(uid.like(format!("{}/%", uid_ref)))
        .load(db_conn) {
            res.extend(a);
    }

    let res_sorted_uids: Vec<String> = sort_suttas(res)
        .into_iter().map(|s| s.uid).collect();

    res_sorted_uids
}

/// Map an inflected word form to headwords
fn inflection_to_pali_words(
    db_conn: &mut SqliteConnection,
    word_form: &str,
) -> QueryResult<Vec<DpdHeadword>> {
    use crate::schema_dpd::lookup::dsl as lk;
    use crate::schema_dpd::dpd_headwords::dsl as hd;

    let i2h = lk::lookup
        .filter(lk::lookup_key.eq(word_form))
        .first::<Lookup>(db_conn)
        .optional()?;

    if let Some(i2h) = i2h {
        let headwords = hd::dpd_headwords
            .filter(hd::id.eq_any(i2h.headwords_unpack()))
            .load::<DpdHeadword>(db_conn)?;
        Ok(headwords)
    } else {
        Ok(Vec::new())
    }
}

pub fn dpd_deconstructor_query(db_conn: &mut SqliteConnection,
                               query_text: &str,
                               exact_only: bool)
                               -> QueryResult<Option<Lookup>> {
    use crate::schema_dpd::lookup::dsl::*;

    // NOTE: Return exact match if possible because 'starts with' matches show confusing additional words.

    // Attempt 1: Exact match.
    let mut result: Option<Lookup> = lookup
        .filter(lookup_key.eq(query_text))
        .first::<Lookup>(db_conn)
        .optional()?; // .optional() converts NotFound to Ok(None), other errors propagate.

    // Attempt 2: If not exact_only, try to match as 'starts with'.
    if !exact_only {
        if result.is_none() && query_text.chars().count() >= 4 {
            result = lookup
                .filter(lookup_key.like(&format!("{}%", query_text)))
                .first::<Lookup>(db_conn)
                .optional()?;
        }
    }

    // Attempt 3: If the query contained multiple words, remove spaces to find compound forms.
    if result.is_none() && query_text.contains(' ') {
        let query_text_no_space = query_text.replace(" ", "");
        // Avoid querying with an empty string if query_text was all spaces.
        if !query_text_no_space.is_empty() {
            result = lookup
                .filter(lookup_key.eq(&query_text_no_space))
                .first::<Lookup>(db_conn)
                .optional()?;
        }
    }

    // Attempt 4: remove the last letter
    if !exact_only {
        // If there were no exact match in the deconstructor, and query_text is
        // long enough, remove the last letter and match as 'starts with'.
        if result.is_none() && query_text.chars().count() >= 4 {
            let num_chars = query_text.chars().count();
            // Since chars().count() >= 4, num_chars - 1 will be >= 3.
            // So, all_but_last_char will not be empty.
            let all_but_last_char: String = query_text.chars().take(num_chars - 1).collect();

            result = lookup
                .filter(lookup_key.like(&format!("{}%", all_but_last_char)))
                .first::<Lookup>(db_conn)
                .optional()?;
        }
    }

    Ok(result)
}

pub fn dpd_deconstructor_list(query: &str) -> Vec<String> {
    let (_, _, db_conn) = &mut establish_connection();
    match dpd_deconstructor_query(db_conn, query, false) {
        Ok(res) => {
            if let Some(r) = res {
                r.deconstructor_unpack()
            } else {
                Vec::new()
            }
        }

        Err(e) => {
            println!("ERROR: {}", e);
            Vec::new()
        }
    }
}

/// Convert deconstructor entries to Pāli headwords
pub fn dpd_deconstructor_to_pali_words(
    db_conn: &mut SqliteConnection,
    query_text: &str,
    exact_only: bool,
) -> QueryResult<Vec<DpdHeadword>> {
    let mut seen: Vec<String> = Vec::new();
    let mut results: Vec<DpdHeadword> = Vec::new();

    if let Some(lookup) = dpd_deconstructor_query(db_conn, query_text, exact_only)? {
        for word in lookup.deconstructor_flat().iter() {
            for hw in inflection_to_pali_words(db_conn, word)? {
                if seen.contains(&hw.lemma_1) {
                    continue;
                } else {
                    seen.push(hw.lemma_1.clone());
                    results.push(hw);
                }
            }
        }
    }

    Ok(results)
}

/// Parse word models into search results, deduplicating and optional sorting
fn parse_words(
    words_res: Vec<UDpdWord>,
    _do_pali_sort: bool,
) -> Vec<SearchResult> {
    let mut uniq_pali_keys: HashSet<String> = HashSet::new();
    let mut uniq_words: Vec<UDpdWord> = Vec::new();

    for w in words_res {
        if uniq_pali_keys.insert(w.word()) {
            uniq_words.push(w);
        }
    }

    // FIXME implement sorting
    // if do_pali_sort {
    //     uniq_words.sort_by_key(|w| pali_sort_key(w.word()));
    // }

    let mut res_page: Vec<SearchResult> = Vec::new();

    for w in uniq_words {
        match &w {
            UDpdWord::Headword(h) => {
                let meaning = if !h.meaning_1.is_empty() {
                    &h.meaning_1
                } else {
                    &h.meaning_2
                };
                let snippet = format!("{} <b>·</b> <i>{}</i>", meaning, strip_html(&h.grammar));
                res_page.push(SearchResult::from_dpd_headword(h, snippet));
            }
            UDpdWord::Root(r) => {
                let snippet = format!(
                    "{} <b>·</b> <i>{}</i>",
                    r.root_meaning,
                    root_info_clean_plaintext(&r.root_info)
                );
                res_page.push(SearchResult::from_dpd_root(r, snippet));
            }
        }
    }

    res_page
}

/// Recommended defaults: do_pali_sort = false, exact_only = true
pub fn dpd_lookup(
    db_conn: &mut SqliteConnection,
    query_text_orig: &str,
    do_pali_sort: bool,
    exact_only: bool,
) -> QueryResult<Vec<SearchResult>> {
    use crate::schema_dpd::dpd_headwords;
    use crate::schema_dpd::dpd_roots;

    // Normalize
    let query_text = query_text_orig.to_lowercase();
    lazy_static! {
        static ref re_ti: Regex = Regex::new(r"[’']ti$").unwrap();
    };
    let query_text = re_ti.replace_all(&query_text, "ti").into_owned();

    let mut res_words: Vec<UDpdWord> = Vec::new();

    // Query text may be an uid or an id number.
    // DpdHeadword uid is id_number/dpd, DpdRoot uid is root/dpd.
    if query_text.ends_with("/dpd") || query_text.chars().all(char::is_numeric) {
        let ref_str = query_text.replace("/dpd", "");
        // If the remaining reference string is numeric, it is a DpdHeadword
        if ref_str.chars().all(char::is_numeric) {
            if let Ok(id_val) = ref_str.parse::<i32>() {
                let r_opt = dpd_headwords::table
                    .filter(dpd_headwords::id.eq(id_val))
                    .first::<DpdHeadword>(db_conn)
                    .optional()?;
                if let Some(r) = r_opt {
                    res_words.push(UDpdWord::Headword(r));
                }
            }
        } else {
            // Else it is a DpdRoot
            let r_opt = dpd_roots::table
                .filter(dpd_roots::uid.eq(&query_text))
                .first::<DpdRoot>(db_conn)
                .optional()?;
            if let Some(r) = r_opt {
                res_words.push(UDpdWord::Root(r));
            }
        }
    }

    if !res_words.is_empty() {
        return Ok(parse_words(res_words, do_pali_sort));
    }

    // Word exact match.
    let r = dpd_headwords::table
        .filter(dpd_headwords::lemma_clean.eq(&query_text)
                .or(dpd_headwords::word_ascii.eq(&query_text)))
        .load::<DpdHeadword>(db_conn)?;
    res_words.extend(r.into_iter().map(UDpdWord::Headword));

    let r = dpd_roots::table
        .filter(dpd_roots::root_clean.eq(&query_text)
                .or(dpd_roots::root_no_sign.eq(&query_text))
                .or(dpd_roots::word_ascii.eq(&query_text)))
        .load::<DpdRoot>(db_conn)?;
    res_words.extend(r.into_iter().map(UDpdWord::Root));

    // Add matches from DPD inflections_to_headwords, regardless of earlier results.
    // This will include cases such as:
    // - assa: gen. of ima
    // - assa: imp 2nd sg of assati
    let r = inflection_to_pali_words(db_conn, &query_text)?;
    res_words.extend(r.into_iter().map(UDpdWord::Headword));

    if res_words.is_empty() {
        // Stem form exact match.
        let stem = pali_stem(&query_text, false);
        let r = dpd_headwords::table
            .filter(dpd_headwords::stem.eq(&stem))
            .load::<DpdHeadword>(db_conn)?;
        res_words.extend(r.into_iter().map(UDpdWord::Headword));
    }

    if res_words.is_empty() {
        // If the query contained multiple words, remove spaces to find compound forms.
        if query_text.contains(' ') {
            let nospace_query = query_text.replace(' ', "");
            let r = dpd_headwords::table
                .filter(dpd_headwords::lemma_clean.eq(&nospace_query)
                        .or(dpd_headwords::word_ascii.eq(&nospace_query)))
                .load::<DpdHeadword>(db_conn)?;
            res_words.extend(r.into_iter().map(UDpdWord::Headword));
        }
    }

    if res_words.is_empty() {
        // i2h result doesn't exist.
        // Lookup query text in dpd_deconstructor.
        let r = dpd_deconstructor_to_pali_words(db_conn, &query_text, exact_only)?;
        res_words.extend(r.into_iter().map(UDpdWord::Headword));
    }

    if res_words.is_empty() {
        // - no exact match in dpd_headwords or dpd_roots
        // - not in i2h
        // - not in deconstructor.
        //
        // Lookup dpd_headwords which start with the query_text.

        // Word starts with.
        let r = dpd_headwords::table
            .filter(dpd_headwords::lemma_clean.like(format!("{}%", query_text))
                    .or(dpd_headwords::word_ascii.like(format!("{}%", query_text))))
            .load::<DpdHeadword>(db_conn)?;
        res_words.extend(r.into_iter().map(UDpdWord::Headword));

        if res_words.is_empty() {
            // Stem form starts with.
            let stem = pali_stem(&query_text, false);
            let r = dpd_headwords::table
                .filter(dpd_headwords::stem.like(format!("{}%", stem)))
                .load::<DpdHeadword>(db_conn)?;
            res_words.extend(r.into_iter().map(UDpdWord::Headword));
        }
    }

    Ok(parse_words(res_words, do_pali_sort))
}

pub fn dpd_lookup_list(query: &str) -> Vec<String> {
    let (_, _, db_conn) = &mut establish_connection();
    match dpd_lookup(db_conn, query, false, true) {
        Ok(res) => {
            res.iter().map(|i| i.snippet.clone()).collect()
        }

        Err(e) => {
            println!("ERROR: {}", e);
            Vec::new()
        }
    }
}


/// Remove duplicates based on title, schema_name, and uid
pub fn unique_search_results(mut results: Vec<SearchResult>) -> Vec<SearchResult> {
    let mut seen: HashSet<String> = HashSet::new();
    results.retain(|item| {
        let key = format!("{} {} {}", item.title, item.schema_name, item.uid);
        if seen.contains(&key) {
            false
        } else {
            seen.insert(key);
            true
        }
    });
    results
}

pub fn import_migrate_dpd(dpd_input_path: &PathBuf, dpd_output_path: Option<PathBuf>) -> Result<(), String> {
    // Migrate the db at the provided input path.
    let migrate_db_path = dpd_input_path.to_path_buf();

    // Find or create the DPD dict record in appdata
    let (db_conn, _, _) = &mut establish_connection();
    let dpd_dict = find_or_create_dpd_dictionary(db_conn)
        .map_err(|e| format!("{}", e))?;

    // Run the migrations on the db
    migrate_dpd(&migrate_db_path, dpd_dict.id)
        .map_err(|e| format!("{}", e))?;

    // Move to local data folder or the specified path
    let output_path = match dpd_output_path {
        Some(p) => p,
        None => get_create_simsapa_app_assets_path().join("dpd.sqlite3"),
    };

    match fs::rename(&migrate_db_path, &output_path) {
        Ok(_) => {},
        Err(e) => {
            // fs::rename() doesn't work across different fs mounts, e.g. partitions.
            // In that case, copy and remove.
            // Otherwise, return the error.
            let msg = format!("{}", e);
            // Error executing command: Invalid cross-device link (os error 18)
            if msg.contains("Invalid cross-device link") {
                fs::copy(&migrate_db_path, &output_path).map_err(|e| format!("{}", e))?;
                fs::remove_file(&migrate_db_path).map_err(|e| format!("{}", e))?;
            } else {
                return Err(format!("{}", e));
            }
        }
    }

    Ok(())
}

/// Find or create DPD Dictionary record with label 'dpd'
pub fn find_or_create_dpd_dictionary(db_conn: &mut SqliteConnection)
                                     -> Result<Dictionary, diesel::result::Error> {
    use crate::schema_dictionaries::dictionaries::dsl::*;

    match dictionaries
        .select(Dictionary::as_select())
        .filter(label.eq("dpd"))
        .first(db_conn) {
            Ok(x) => return Ok(x),
            Err(_) => {},
        }

    // If not returned yet, create a new record
    let new_dict = NewDictionary {
        label: "dpd",
        title: "Digital Pāḷi Dictionary",
        dict_type: "sql", // FIXME dict_type = DictTypeName.Sql.value,
        .. Default::default()
    };

    create_dictionary(db_conn, new_dict)
}

/// Save cf_set and sandhi_contractions
pub fn save_dpd_caches(_db_conn: &mut SqliteConnection)
                       -> Result<(), diesel::result::Error> {
    // TODO implement
    Ok(())
}

// Structs for querying sqlite_master and PRAGMA table_info
#[derive(QueryableByName, Debug)]
struct TableName {
    #[diesel(sql_type = Text)]
    name: String,
}

#[derive(QueryableByName, Debug)]
struct ColumnInfo {
    #[diesel(sql_type = Text)]
    name: String,
}

/// Iterates through all tables and text columns to replace 'ṃ' with 'ṁ'.
pub fn replace_all_niggahitas(db_conn: &mut SqliteConnection) -> Result<(), diesel::result::Error> {
    println!("replace_all_niggahitas()");

    // Avoid "FOREIGN KEY constraint failed" errors when updating columns that
    // might be part of a foreign key relationship.
    diesel::sql_query("PRAGMA foreign_keys = OFF;").execute(db_conn)?;

    let res = db_conn.transaction::<_, diesel::result::Error, _>(|conn| {
        // Get all table names
        let tables = diesel::sql_query("SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%';")
            .load::<TableName>(conn)?;

        for table_row in tables {
            let table_name = table_row.name;

            // Get all column names of the table
            let pragma_query = format!("PRAGMA table_info('{}');", table_name);
            let columns = diesel::sql_query(pragma_query)
                .load::<ColumnInfo>(conn)?;

            // For each column, replace 'ṃ' with 'ṁ' in the content
            for column_row in columns {
                let column_name = column_row.name;

                let update_query = format!(
                    "UPDATE `{}` SET `{}` = REPLACE(`{}`, 'ṃ', 'ṁ') WHERE `{}` LIKE '%ṃ%';",
                    table_name, column_name, column_name, column_name
                );
                diesel::sql_query(update_query).execute(conn)?;
            }
        }
        Ok(())
    });

    // Re-enable foreign keys
    diesel::sql_query("PRAGMA foreign_keys = ON;").execute(db_conn)?;

    // Return the result of the transaction
    res
}

/// Update the DPD db schema to agree with Diesel models_dpd.rs
pub fn dpd_update_schema(db_conn: &mut SqliteConnection) -> Result<(), diesel::result::Error> {
    println!("dpd_update_schema()");
    db_conn.transaction::<_, diesel::result::Error, _>(|conn| {
        // Remove some columns.
        diesel::sql_query("ALTER TABLE dpd_headwords DROP COLUMN created_at;").execute(conn)?;
        diesel::sql_query("ALTER TABLE dpd_headwords DROP COLUMN updated_at;").execute(conn)?;

        diesel::sql_query("ALTER TABLE dpd_roots DROP COLUMN created_at;").execute(conn)?;
        diesel::sql_query("ALTER TABLE dpd_roots DROP COLUMN updated_at;").execute(conn)?;

        // Rename columns with conflicting names.
        // NOTE: Quote column names which are keywords.

        diesel::sql_query("ALTER TABLE inflection_templates RENAME COLUMN 'like' TO like_col;").execute(conn)?;
        diesel::sql_query("ALTER TABLE family_set RENAME COLUMN 'set' TO set_col;").execute(conn)?;

        diesel::sql_query("ALTER TABLE dpd_headwords RENAME COLUMN family_root TO family_root_fk;").execute(conn)?;
        diesel::sql_query("ALTER TABLE dpd_headwords RENAME COLUMN family_word TO family_word_fk;").execute(conn)?;
        diesel::sql_query("ALTER TABLE dpd_headwords RENAME COLUMN family_compound TO family_compound_fk;").execute(conn)?;
        diesel::sql_query("ALTER TABLE dpd_headwords RENAME COLUMN family_idioms TO family_idioms_fk;").execute(conn)?;
        diesel::sql_query("ALTER TABLE dpd_headwords RENAME COLUMN family_set TO family_set_fk;").execute(conn)?;

        // Add dictionary_id:
        // - dpd_headwords
        // - dpd_roots
        diesel::sql_query("ALTER TABLE dpd_headwords ADD COLUMN dictionary_id INTEGER NOT NULL DEFAULT 0;").execute(conn)?;
        diesel::sql_query("ALTER TABLE dpd_roots ADD COLUMN dictionary_id INTEGER NOT NULL DEFAULT 0;").execute(conn)?;

        // dpd_headwords: uid, word_ascii, lemma_clean

        diesel::sql_query("ALTER TABLE dpd_headwords ADD COLUMN uid VARCHAR NOT NULL DEFAULT '';").execute(conn)?;
        diesel::sql_query("ALTER TABLE dpd_headwords ADD COLUMN word_ascii VARCHAR NOT NULL DEFAULT '';").execute(conn)?;
        diesel::sql_query("ALTER TABLE dpd_headwords ADD COLUMN lemma_clean VARCHAR NOT NULL DEFAULT '';").execute(conn)?;

        // dpd_roots: uid, word_ascii, root_clean, root_no_sign

        diesel::sql_query("ALTER TABLE dpd_roots ADD COLUMN uid VARCHAR NOT NULL DEFAULT '';").execute(conn)?;
        diesel::sql_query("ALTER TABLE dpd_roots ADD COLUMN word_ascii VARCHAR NOT NULL DEFAULT '';").execute(conn)?;
        diesel::sql_query("ALTER TABLE dpd_roots ADD COLUMN root_clean VARCHAR NOT NULL DEFAULT '';").execute(conn)?;
        diesel::sql_query("ALTER TABLE dpd_roots ADD COLUMN root_no_sign VARCHAR NOT NULL DEFAULT '';").execute(conn)?;
        Ok(())
    })
}

/// Performs schema changes and data migrations for DPD tables.
pub fn migrate_dpd(dpd_db_path: &PathBuf, dpd_dictionary_id: i32)
                   -> Result<(), diesel::result::Error> {
    println!("migrate_dpd()");

    let abs_path = fs::canonicalize(dpd_db_path.to_path_buf()).unwrap_or(dpd_db_path.to_path_buf());
    let database_url = format!("sqlite://{}", abs_path.as_os_str().to_str().expect("os_str Error!"));
    let mut db_conn = SqliteConnection::establish(&database_url)
        .unwrap_or_else(|_| panic!("Error connecting to {}", database_url));

    dpd_update_schema(&mut db_conn)?;

    // Now the DPD schema is up to date with the Diesel models_dpd.rs definition.

    use crate::schema_dpd::{dpd_headwords, dpd_roots};

    println!("Updating dictionary_id ...");

    let abs_path = fs::canonicalize(dpd_db_path.to_path_buf()).unwrap_or(dpd_db_path.to_path_buf());
    let database_url = format!("sqlite://{}", abs_path.as_os_str().to_str().expect("os_str Error!"));
    let mut db_conn = SqliteConnection::establish(&database_url)
        .unwrap_or_else(|_| panic!("Error connecting to {}", database_url));

    db_conn.transaction::<_, diesel::result::Error, _>(|conn| {
        // Update dictionary_id for all rows
        diesel::sql_query("UPDATE dpd_headwords SET dictionary_id = ?;")
            .bind::<Integer, _>(dpd_dictionary_id)
            .execute(conn)?;
        diesel::sql_query("UPDATE dpd_roots SET dictionary_id = ?;")
            .bind::<Integer, _>(dpd_dictionary_id)
            .execute(conn)?;

        Ok(())
    })?; // End of transaction

    println!("Updating dpd_headwords ...");

    // Update DpdHeadwords calculated fields
    db_conn.transaction::<_, diesel::result::Error, _>(|conn| {
        let headwords_all: Vec<DpdHeadword> = dpd_headwords::table.load::<DpdHeadword>(conn)?;

        for headword_item in headwords_all.iter() {
            let new_uid = word_uid(&headword_item.id.to_string(), "dpd");
            let new_lemma_clean = headword_item.calc_lemma_clean();
            // Use lemma_clean for word_ascii to remove trailing numbers.
            let new_word_ascii = pali_to_ascii(Some(&new_lemma_clean));

            diesel::update(dpd_headwords::table.find(headword_item.id))
                .set((
                    dpd_headwords::uid.eq(new_uid),
                    dpd_headwords::lemma_clean.eq(new_lemma_clean),
                    dpd_headwords::word_ascii.eq(new_word_ascii),
                ))
                .execute(conn)?;
        }
        Ok(())
    })?;

    println!("Updating dpd_roots ...");

    // Update DpdRoots calculated fields
    db_conn.transaction::<_, diesel::result::Error, _>(|conn| {
        let roots_all = dpd_roots::table.load::<DpdRoot>(conn)?;

        for root_item in roots_all.iter() {
            let new_uid = word_uid(&root_item.root, "dpd");
            let new_root_clean = root_item.calc_root_clean();
            let new_root_no_sign = root_item.calc_root_no_sign();
            // Use root_clean for word_ascii to remove trailing numbers.
            let new_word_ascii = pali_to_ascii(Some(&new_root_clean));

            diesel::update(dpd_roots::table.find(root_item.root.clone()))
                .set((
                    dpd_roots::uid.eq(new_uid),
                    dpd_roots::root_clean.eq(new_root_clean),
                    dpd_roots::root_no_sign.eq(new_root_no_sign),
                    dpd_roots::word_ascii.eq(new_word_ascii),
                ))
                .execute(conn)?;
        }
        Ok(())
    })?;

    // FIXME save_dpd_caches(dpd_db_session)

    replace_all_niggahitas(&mut db_conn)?;

    Ok(())
}
