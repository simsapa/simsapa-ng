use std::fs;
use std::path::PathBuf;
use std::collections::HashSet;
use std::time::Instant;

use diesel::prelude::*;
use diesel::sql_types::{Text, Integer};
use anyhow::Result;
use serde_json;
use serde::Serialize;

use crate::db::dpd_models::*;
use crate::db::DatabaseHandle;

use crate::{get_app_data, get_create_simsapa_app_assets_path};
use crate::helpers::{word_uid, pali_to_ascii, strip_html, root_info_clean_plaintext, normalize_query_text};
use crate::pali_stemmer::pali_stem;
use crate::pali_sort::{pali_sort_key, sort_search_results_natural};
use crate::types::SearchResult;
use crate::logger::{info, error};

pub type DpdDbHandle = DatabaseHandle;

#[derive(Serialize)]
pub struct LookupResult {
    uid: String,
    word: String,
    summary: String,
}

impl LookupResult {
    pub fn from_search_result(i: &SearchResult) -> LookupResult {
        LookupResult {
            uid: i.uid.clone(),
            word: i.title.clone(),
            summary: i.snippet.clone(),
        }
    }

    pub fn from_search_results(res: &Vec<SearchResult>) -> Vec<LookupResult> {
        res.iter().map(|i| { LookupResult::from_search_result(i) })
                  .collect()
    }
}

impl DpdDbHandle {
    /// Map an inflected word form to headwords
    fn inflection_to_pali_words(&self, word_form: &str) -> Result<Vec<DpdHeadword>> {
        use crate::db::dpd_schema::lookup::dsl as lk;
        use crate::db::dpd_schema::dpd_headwords::dsl as hd;

        let db_conn = &mut self.get_conn().expect("Can't get db conn");

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

    pub fn dpd_deconstructor_query(&self, query_text: &str, exact_only: bool) -> Result<Option<Lookup>> {
        use crate::db::dpd_schema::lookup::dsl::*;

        let db_conn = &mut self.get_conn().expect("Can't get db conn");

        // NOTE: Return exact match if possible because 'starts with' matches show confusing additional words.

        let query_text = normalize_query_text(Some(query_text.to_string()));

        // Attempt 1: Exact match.
        let mut result: Option<Lookup> = lookup
            .filter(lookup_key.eq(query_text.clone()))
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

    pub fn dpd_deconstructor_list(&self, query: &str) -> Vec<String> {
        match self.dpd_deconstructor_query(query, false) {
            Ok(res) => {
                if let Some(r) = res {
                    r.deconstructor_unpack()
                } else {
                    Vec::new()
                }
            }

            Err(e) => {
                error(&format!("{}", e));
                Vec::new()
            }
        }
    }

    /// Convert deconstructor entries to Pāli headwords
    pub fn dpd_deconstructor_to_pali_words(&self, query_text: &str, exact_only: bool) -> Result<Vec<DpdHeadword>> {
        let mut seen: Vec<String> = Vec::new();
        let mut results: Vec<DpdHeadword> = Vec::new();

        if let Some(lookup) = self.dpd_deconstructor_query(query_text, exact_only)? {
            for word in lookup.deconstructor_flat().iter() {
                for hw in self.inflection_to_pali_words(word)? {
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

    /// Recommended defaults: do_pali_sort = false, exact_only = true
    pub fn dpd_lookup(
        &self,
        query_text_orig: &str,
        do_pali_sort: bool,
        exact_only: bool,
    ) -> Result<Vec<SearchResult>> {
        info(&format!("dpd_lookup(): query_text_orig: {}", query_text_orig));
        let timer = Instant::now();

        use crate::db::dpd_schema::dpd_headwords;
        use crate::db::dpd_schema::dpd_roots;

        let db_conn = &mut self.get_conn().expect("Can't get db conn");

        let query_text = normalize_query_text(Some(query_text_orig.to_string()));

        // Collect word results in groups, with more "obvious" results grouped first.
        // Sort within groups by "natural" order to get dict numbers right,
        // but don't sort the final list to not lose the priority groups.

        let mut results: Vec<SearchResult> = Vec::new();
        let mut results_uids: Vec<String> = Vec::new();

        // Query text may be an uid or an id number.
        // DpdHeadword uid is id_number/dpd, DpdRoot uid is root/dpd.
        if query_text.ends_with("/dpd") || query_text.chars().all(char::is_numeric) {
            let mut res_words: Vec<UDpdWord> = Vec::new();
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

            let mut res = parse_words(res_words, do_pali_sort);
            results_uids.extend(res.iter().map(|i| i.uid.clone()));

            sort_search_results_natural(&mut res);
            results.extend(res);
        }

        if !results.is_empty() {
            return Ok(results);
        }

        // Word exact match.
        {
            let r = dpd_headwords::table
                .filter(dpd_headwords::lemma_clean.eq(&query_text)
                        .or(dpd_headwords::word_ascii.eq(&query_text)))
                .load::<DpdHeadword>(db_conn)?;

            let mut res_words: Vec<UDpdWord> = Vec::new();
            res_words.extend(r.into_iter().map(UDpdWord::Headword));

            let mut res = parse_words(res_words, do_pali_sort);
            res.retain(|i| !results_uids.contains(&i.uid));
            results_uids.extend(res.iter().map(|i| i.uid.clone()));
            sort_search_results_natural(&mut res);
            results.extend(res);
        }

        // For two OR conditions on indexed columns, SQLite can efficiently use the indexes to combine the results.
        // For three OR conditions, it is recommended to split the queries.
        //
        // let r = dpd_roots::table
        //     .filter(dpd_roots::root_clean.eq(&query_text)
        //             .or(dpd_roots::root_no_sign.eq(&query_text))
        //             .or(dpd_roots::word_ascii.eq(&query_text)))
        //     .load::<DpdRoot>(db_conn)?;
        // res_words.extend(r.into_iter().map(UDpdWord::Root));

        // Instead, use a HashSet to collect results:

        let mut roots = HashSet::new();
        roots.extend(dpd_roots::table.filter(dpd_roots::root_clean.eq(&query_text)).load::<DpdRoot>(db_conn)?);
        roots.extend(dpd_roots::table.filter(dpd_roots::root_no_sign.eq(&query_text)).load::<DpdRoot>(db_conn)?);
        roots.extend(dpd_roots::table.filter(dpd_roots::word_ascii.eq(&query_text)).load::<DpdRoot>(db_conn)?);

        {
            let mut res_words: Vec<UDpdWord> = Vec::new();
            res_words.extend(roots.into_iter().map(UDpdWord::Root));

            let mut res = parse_words(res_words, do_pali_sort);
            res.retain(|i| !results_uids.contains(&i.uid));
            results_uids.extend(res.iter().map(|i| i.uid.clone()));
            sort_search_results_natural(&mut res);
            results.extend(res);
        }

        // Add matches from DPD inflections_to_headwords, regardless of earlier results.
        // This will include cases such as:
        // - assa: gen. of ima
        // - assa: imp 2nd sg of assati
        let r = self.inflection_to_pali_words(&query_text)?;
        {
            let mut res_words: Vec<UDpdWord> = Vec::new();
            res_words.extend(r.into_iter().map(UDpdWord::Headword));

            let mut res = parse_words(res_words, do_pali_sort);
            res.retain(|i| !results_uids.contains(&i.uid));
            results_uids.extend(res.iter().map(|i| i.uid.clone()));
            sort_search_results_natural(&mut res);
            results.extend(res);
        }

        if results.is_empty() {
            // Stem form exact match.
            let stem = pali_stem(&query_text, false);
            let r = dpd_headwords::table
                .filter(dpd_headwords::stem.eq(&stem))
                .load::<DpdHeadword>(db_conn)?;
            {
                let mut res_words: Vec<UDpdWord> = Vec::new();
                res_words.extend(r.into_iter().map(UDpdWord::Headword));

                let mut res = parse_words(res_words, do_pali_sort);
                res.retain(|i| !results_uids.contains(&i.uid));
                results_uids.extend(res.iter().map(|i| i.uid.clone()));
                sort_search_results_natural(&mut res);
                results.extend(res);
            }
        }

        if results.is_empty() {
            // If the query contained multiple words, remove spaces to find compound forms.
            if query_text.contains(' ') {
                let nospace_query = query_text.replace(' ', "");
                let r = dpd_headwords::table
                    .filter(dpd_headwords::lemma_clean.eq(&nospace_query)
                            .or(dpd_headwords::word_ascii.eq(&nospace_query)))
                    .load::<DpdHeadword>(db_conn)?;
                {
                    let mut res_words: Vec<UDpdWord> = Vec::new();
                    res_words.extend(r.into_iter().map(UDpdWord::Headword));

                    let mut res = parse_words(res_words, do_pali_sort);
                    res.retain(|i| !results_uids.contains(&i.uid));
                    results_uids.extend(res.iter().map(|i| i.uid.clone()));
                    sort_search_results_natural(&mut res);
                    results.extend(res);
                }
            }
        }

        if results.is_empty() {
            // i2h result doesn't exist.
            // Lookup query text in dpd_deconstructor.
            let r = self.dpd_deconstructor_to_pali_words(&query_text, exact_only)?;
            {
                let mut res_words: Vec<UDpdWord> = Vec::new();
                res_words.extend(r.into_iter().map(UDpdWord::Headword));

                let mut res = parse_words(res_words, do_pali_sort);
                res.retain(|i| !results_uids.contains(&i.uid));
                results_uids.extend(res.iter().map(|i| i.uid.clone()));
                sort_search_results_natural(&mut res);
                results.extend(res);
            }
        }

        if results.is_empty() {
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

            {
                let mut res_words: Vec<UDpdWord> = Vec::new();
                res_words.extend(r.into_iter().map(UDpdWord::Headword));

                let mut res = parse_words(res_words, do_pali_sort);
                res.retain(|i| !results_uids.contains(&i.uid));
                results_uids.extend(res.iter().map(|i| i.uid.clone()));
                sort_search_results_natural(&mut res);
                results.extend(res);
            }

            if results.is_empty() {
                // Stem form starts with.
                let stem = pali_stem(&query_text, false);
                let r = dpd_headwords::table
                    .filter(dpd_headwords::stem.like(format!("{}%", stem)))
                    .load::<DpdHeadword>(db_conn)?;
                {
                    let mut res_words: Vec<UDpdWord> = Vec::new();
                    res_words.extend(r.into_iter().map(UDpdWord::Headword));

                    let mut res = parse_words(res_words, do_pali_sort);
                    res.retain(|i| !results_uids.contains(&i.uid));
                    results_uids.extend(res.iter().map(|i| i.uid.clone()));
                    sort_search_results_natural(&mut res);
                    results.extend(res);
                }
            }
        }

        info(&format!("Query took: {:?}", timer.elapsed()));
        Ok(results)
    }

    pub fn dpd_lookup_list(&self, query: &str) -> Vec<String> {
        match self.dpd_lookup(query, false, true) {
            Ok(res) => {
                res.iter().map(|i| format!("<b>{}</b> {}", i.title, i.snippet)).collect()
            }

            Err(e) => {
                error(&format!("{}", e));
                Vec::new()
            }
        }
    }

    pub fn dpd_lookup_json(&self, query: &str) -> String {
        let list: Vec<LookupResult> = match self.dpd_lookup(query, false, true) {
            Ok(res) => LookupResult::from_search_results(&res),

            Err(e) => {
                error(&format!("{}", e));
                Vec::new()
            }
        };
        serde_json::to_string(&list).unwrap_or_default()
    }
}

/// Parse word models into search results, deduplicating and optional sorting
fn parse_words(
    words_res: Vec<UDpdWord>,
    do_pali_sort: bool,
) -> Vec<SearchResult> {
    let mut uniq_pali_keys: HashSet<String> = HashSet::new();
    let mut uniq_words: Vec<UDpdWord> = Vec::new();

    for w in words_res {
        if uniq_pali_keys.insert(w.word()) {
            uniq_words.push(w);
        }
    }

    if do_pali_sort {
        uniq_words.sort_by_key(|w| pali_sort_key(&w.word()));
    }

    let mut res_page: Vec<SearchResult> = Vec::new();

    for w in uniq_words {
        match &w {
            UDpdWord::Headword(h) => {
                let meaning = if !h.meaning_1.is_empty() {
                    &h.meaning_1
                } else {
                    &h.meaning_2
                };
                let construction = if h.construction.is_empty() {
                    " ".to_string()
                } else {
                    // The construction field can contain variations, separated by newlines
                    // [na > a + saṁ + √ñā + ā + a], [asaññā + a]
                    format!(" <b>[{}]</b> ", h.construction.replace("\n", "], ["))
                };
                // NOTE: Don't prefix the snippet with the word, it causes repetition.
                // If needed, it can be added just before display.
                let snippet = format!("<i>({})</i> {} {} <i>{}</i>",
                                      h.pos,
                                      meaning,
                                      construction,
                                      strip_html(&h.grammar));
                res_page.push(SearchResult::from_dpd_headword(h, snippet));
            }
            UDpdWord::Root(r) => {
                let snippet = format!(
                    "<b>{}</b> {} <b>·</b> <i>{}</i>",
                    r.word(),
                    r.root_meaning,
                    root_info_clean_plaintext(&r.root_info)
                );
                res_page.push(SearchResult::from_dpd_root(r, snippet));
            }
        }
    }

    res_page
}

pub fn import_migrate_dpd(dpd_input_path: &PathBuf, dpd_output_path: Option<PathBuf>) -> Result<(), String> {
    // Migrate the db at the provided input path.
    let migrate_db_path = dpd_input_path.to_path_buf();

    // Find or create the DPD dict record in dictionaries
    let app_data = get_app_data();
    let dpd_dict = app_data.dbm.dictionaries.find_or_create_dpd_dictionary()
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
    info("replace_all_niggahitas()");

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

/// Update the DPD db schema to agree with Diesel dpd_models.rs
pub fn dpd_update_schema(db_conn: &mut SqliteConnection) -> Result<(), diesel::result::Error> {
    info("dpd_update_schema()");
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
    info("migrate_dpd()");

    let abs_path = fs::canonicalize(dpd_db_path.to_path_buf()).unwrap_or(dpd_db_path.to_path_buf());
    let database_url = format!("sqlite://{}", abs_path.as_os_str().to_str().expect("os_str Error!"));
    let mut db_conn = SqliteConnection::establish(&database_url)
        .unwrap_or_else(|_| panic!("Error connecting to {}", database_url));

    dpd_update_schema(&mut db_conn)?;

    // Now the DPD schema is up to date with the Diesel dpd_models.rs definition.

    use crate::db::dpd_schema::{dpd_headwords, dpd_roots};

    info("Updating dictionary_id ...");

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

    info("Updating dpd_headwords ...");

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

    info("Updating dpd_roots ...");

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

    // Execute DPD B-tree indexes SQL script
    info("Executing DPD B-tree indexes script...");
    let sql_content = include_str!("../../../scripts/dpd-btree-indexes.sql");
    diesel::sql_query(sql_content).execute(&mut db_conn)?;
    info("Successfully created DPD B-tree indexes");

    Ok(())
}
