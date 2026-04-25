// use std::any::Any;
use std::collections::HashSet;
use std::error::Error;
use std::time::Instant;

use regex::Regex;
use diesel::prelude::*;
use diesel::sql_query;
use diesel::sql_types::{Text, BigInt};

use crate::helpers::{normalize_plain_text, normalize_query_text, remove_inter_word_hyphens, sutta_range_from_ref};
use crate::{get_app_data, get_app_globals};
use crate::types::{SearchArea, SearchMode, SearchParams, SearchResult};
use crate::db::appdata_models::{Sutta, BookSpineItem};
use crate::db::dictionaries_models::DictWord;
use crate::db::DbManager;
use crate::logger::{info, warn, error};

/// Upper bound on rows materialised by an unpaginated mode handler before
/// merge/filter/pagination in `results_page`. Applied as `LIMIT` on FTS5
/// SQL queries. Keep in sync with `SAFETY_LIMIT_TANTIVY`.
pub(crate) const SAFETY_LIMIT_SQL: i64 = 100_000;

/// Upper bound on tantivy hits returned to `results_page` per index.
/// Passed to `TopDocs::with_limit`. Keep in sync with `SAFETY_LIMIT_SQL`.
pub(crate) const SAFETY_LIMIT_TANTIVY: usize = 100_000;

/// Sanitize a user-supplied prefix for direct embedding in a SQL LIKE pattern.
/// Returns `Some(prefix_lowercase)` if the input is non-empty and contains only
/// safe characters (alphanumeric, dot, hyphen, underscore, slash); otherwise `None`.
fn sanitize_uid_like_prefix(input: Option<&str>) -> Option<String> {
    let s = input?.trim();
    if s.is_empty() {
        return None;
    }
    if s.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_' | '/')) {
        Some(s.to_lowercase())
    } else {
        None
    }
}

/// Convert a `BoldDefinition` row to a `SearchResult`. The snippet is the
/// first ~300 chars of `commentary_plain`; final highlighting is applied in
/// `results_page` via `highlight_query_in_content`.
fn bold_definition_to_search_result(bd: &crate::db::dpd_models::BoldDefinition) -> SearchResult {
    const SNIPPET_CHARS: usize = 300;
    let snippet: String = bd.commentary_plain.chars().take(SNIPPET_CHARS).collect();
    SearchResult {
        uid: bd.uid.clone(),
        schema_name: "dpd".to_string(),
        table_name: "bold_definitions".to_string(),
        // In bold_definitions, the equivalent of source_uid is the ref_code field (e.g. vina, mna, vvt)
        source_uid: Some(bd.ref_code.clone()),
        title: bd.bold.clone(),
        // And it also serves as the sutta_ref, since it indicates the Vinaya, Majjhima, etc. origin
        sutta_ref: Some(bd.ref_code.clone()),
        nikaya: Some(bd.nikaya.clone()),
        author: None,
        lang: Some("pli".to_string()),
        snippet,
        page_number: None,
        score: None,
        rank: None,
    }
}

pub struct SearchQueryTask<'a> {
    pub dbm: &'a DbManager,
    pub query_text: String,
    pub search_mode: SearchMode,
    pub search_area: SearchArea,
    pub page_len: usize,
    pub lang: String,
    pub lang_include: bool,
    pub source: Option<String>,
    pub source_include: bool,
    pub include_cst_mula: bool,
    pub include_cst_commentary: bool,
    pub nikaya_prefix: Option<String>,
    pub uid_prefix: Option<String>,
    pub uid_suffix: Option<String>,
    pub include_ms_mula: bool,
    pub include_comm_bold_definitions: bool,
    pub db_all_results: Vec<SearchResult>,
    pub db_query_hits_count: i64, // Use i64 for Diesel's count result
}

impl<'a> SearchQueryTask<'a> {
    pub fn new(
        dbm: &'a DbManager,
        query_text_orig: String,
        params: SearchParams,
        area: SearchArea,
    ) -> Self {
        let g = get_app_globals();
        // Use params.lang if provided and not empty, otherwise use empty string for no filter
        let lang_filter = params.lang.clone().unwrap_or_default();

        // For UidMatch mode, don't normalize the query text to preserve dots and other characters.
        // For FulltextMatch mode, apply normalize_plain_text — the same iti-sandhi and niggahita
        // normalization applied to content_plain — so user query variations (e.g. 'dhovananti',
        // 'dhovanan’ti') match the stored text. normalize_plain_text does not strip punctuation,
        // so tantivy's quote/+/-/must operators remain intact. Additionally strip inter-word
        // hyphens because content_plain was produced by compact_plain_text, which runs
        // remove_punct and drops them (so `Dhammapada-aṭṭhakathā` is stored as
        // `dhammapadaaṭṭhakathā`). Only hyphens surrounded by word chars are removed, so
        // tantivy's `-term` must-not operator is left alone.
        // For other modes, normalize to handle punctuation and spacing.
        let query_text = match params.mode {
            SearchMode::UidMatch => {
                query_text_orig.to_lowercase()
            }
            SearchMode::FulltextMatch => {
                remove_inter_word_hyphens(&normalize_plain_text(&query_text_orig))
            }
            _ => {
                normalize_query_text(Some(query_text_orig))
            }
        };

        SearchQueryTask {
            dbm,
            query_text,
            search_mode: params.mode,
            search_area: area,
            page_len: params.page_len.unwrap_or(g.page_len),
            lang: lang_filter,
            lang_include: params.lang_include,
            source: params.source,
            source_include: params.source_include,
            include_cst_mula: params.include_cst_mula,
            include_cst_commentary: params.include_cst_commentary,
            nikaya_prefix: params.nikaya_prefix,
            uid_prefix: params.uid_prefix,
            uid_suffix: params.uid_suffix,
            include_ms_mula: params.include_ms_mula,
            include_comm_bold_definitions: params.include_comm_bold_definitions,
            db_all_results: Vec::new(),
            db_query_hits_count: 0,
        }
    }

    /// Highlights occurrences of a single plain text query term in the content
    /// using regex.
    pub fn highlight_text(&self, term: &str, content: &str) -> Result<String, regex::Error> {
        // Lowercase the term. Content should already be in lowercase.
        let term = term.to_lowercase();
        // Escape regex special characters in the search term
        let escaped_term = regex::escape(&term);
        // Content and term are expected to be lowercase, no need for case-insensitive matching.
        let pattern = format!("({})", escaped_term);
        let re = Regex::new(&pattern)?;
        let highlighted = re.replace_all(content, "<span class='match'>$1</span>");
        Ok(highlighted.into_owned())
    }

    /// Highlights all terms from the query (handles "AND") in the content.
    fn highlight_query_in_content(&self, query: &str, content: &str) -> String {
        let terms: Vec<&str> = if query.contains(" AND ") {
            query.split(" AND ").map(|s| s.trim()).collect()
        } else {
            vec![query]
        };

        let mut current_content = content.to_string();
        for term in terms {
            // Handle potential regex errors, though unlikely with escaped input
            match self.highlight_text(term, &current_content) {
                Ok(highlighted) => current_content = highlighted,
                Err(e) => {
                    error(&format!("Regex error during highlighting: {}", e));
                    // Skip the term.
                    continue;
                }
            }
        }
        current_content
    }

    /// Creates a snippet around the first occurrence of the query term, respecting UTF-8 character boundaries.
    pub fn fragment_around_text(
        &self,
        term: &str,
        content: &str,
        chars_before: usize,
        chars_after: usize,
    ) -> String {
        // Use case-insensitive search for finding the byte position
        if let Some(term_byte_idx) = content.to_lowercase().find(&term.to_lowercase()) {
            // Find the character index corresponding to the start of the term
            let term_char_idx = content.char_indices()
                .enumerate() // Get (char_index, (byte_index, char))
                .find(|&(_, (byte_idx, _))| byte_idx == term_byte_idx)
                .map(|(char_idx, _)| char_idx)
                .unwrap_or(0); // Should always be found if term_byte_idx is valid

            // Calculate target start/end character indices
            let target_start_char_idx = term_char_idx.saturating_sub(chars_before);
            // Calculate term length in characters safely
            let term_char_len = term.chars().count();
            let target_end_char_idx = (term_char_idx + term_char_len + chars_after)
                                        .min(content.chars().count()); // Clamp to content length

            // Find the byte index for the target start character index
            let start_byte_idx = content.char_indices()
                .nth(target_start_char_idx)
                .map(|(byte_idx, _)| byte_idx)
                .unwrap_or(0); // Default to start if index is 0

            // Find the byte index for the target end character index
            let end_byte_idx = content.char_indices()
                .nth(target_end_char_idx)
                .map(|(byte_idx, _)| byte_idx)
                .unwrap_or(content.len()); // Default to end of string if index is out of bounds

            // --- Refine boundaries to whitespace (optional but nicer) ---
            let mut final_start_byte_idx = start_byte_idx;
            let mut prefix = "";
            // If we moved back from the start of the term, try to find whitespace
            if target_start_char_idx > 0 && start_byte_idx > 0 {
                 prefix = "... ";
                 // Search backwards from the calculated start byte index
                 final_start_byte_idx = content[..start_byte_idx]
                     .rfind(|c: char| c.is_whitespace())
                     .map_or(start_byte_idx, |pos| pos + content[pos..].chars().next().map_or(0, |c| c.len_utf8())); // Start after whitespace char
            }

            let mut final_end_byte_idx = end_byte_idx;
            let mut postfix = "";
             // If we haven't reached the end of the content, try to find whitespace
            if target_end_char_idx < content.chars().count() && end_byte_idx < content.len() {
                 postfix = " ...";
                 // Search forwards from the calculated end byte index
                 final_end_byte_idx = content[end_byte_idx..]
                     .find(|c: char| c.is_whitespace())
                     .map_or(end_byte_idx, |pos| end_byte_idx + pos); // End before whitespace char
            }

            // Ensure start is <= end after adjustments
            if final_start_byte_idx > final_end_byte_idx {
                 final_start_byte_idx = start_byte_idx; // Revert start adjustment if it crossed end
                 final_end_byte_idx = end_byte_idx;   // Revert end adjustment
                 // Recalculate prefix/postfix based on original indices
                 prefix = if target_start_char_idx > 0 { "... " } else { "" };
                 postfix = if target_end_char_idx < content.chars().count() { " ..." } else { "" };
            }

            // Final slice using calculated byte indices (guaranteed to be char boundaries)
            format!("{}{}{}", prefix, &content[final_start_byte_idx..final_end_byte_idx], postfix)

        } else {
            // If term not found, return a beginning chunk based on character count
            let target_end_char_idx = (chars_before + chars_after).min(content.chars().count());
            let end_byte_idx = content.char_indices()
                .nth(target_end_char_idx)
                .map(|(byte_idx, _)| byte_idx)
                .unwrap_or(content.len());
            let postfix = if target_end_char_idx < content.chars().count() { " ..." } else { "" };
            format!("{}{}", &content[0..end_byte_idx], postfix)
        }
    }

    /// Creates a snippet around query terms (handles "AND").
    pub fn fragment_around_query(&self, query: &str, content: &str) -> String {
        if query.starts_with("uid:") || query.ends_with("/dpd") {
            return self.fragment_around_text("", content, 20, 500);
        }
        // Simple approach: find the first term and fragment around it.
        // FIXME: A more complex approach could try to find a fragment containing multiple terms.
        let (terms, before, after) = if query.contains(" AND ") {
            (query.split(" AND ").map(|s| s.trim()).collect::<Vec<&str>>(), 10, 50)
        } else {
            (vec![query], 20, 500)
        };

        // Find the first term present in the content and fragment around it
        for term in terms {
             if content.to_lowercase().contains(&term.to_lowercase()) {
                 return self.fragment_around_text(term, content, before, after);
             }
        }

        warn(&format!("Can't create fragment, query terms not found in content: {}", query));

        // If no terms are found, return the beginning of the content
        self.fragment_around_text("", content, before, after)
    }

    /// Helper to choose content (plain or HTML) and create a snippet.
    fn db_sutta_to_result(&self, x: &Sutta) -> SearchResult {
        let content = x.content_plain.as_deref() // Prefer plain text
            .filter(|s| !s.is_empty()) // Ensure it's not empty
            .or(x.content_html.as_deref()) // Fallback to HTML
            .unwrap_or(""); // Default to empty string if both are None/empty

        let snippet = self.fragment_around_query(&self.query_text, content);
        SearchResult::from_sutta(x, snippet)
    }

    fn db_word_to_result(&self, x: &DictWord) -> SearchResult {
        // For DPD words (dict_label contains "dpd"), try to get meaning from DpdHeadword
        // This provides a more useful snippet with pos, meaning, construction, and grammar
        let snippet = if x.dict_label.to_lowercase().contains("dpd") {
            // Extract lemma_1 from uid by removing the "/dpd" suffix
            // e.g., "dhamma 1/dpd" -> "dhamma 1"
            let lemma_1 = x.uid.trim_end_matches("/dpd");

            // Try to get DPD meaning snippet using lemma_1
            let app_data = get_app_data();
            app_data.dbm.dpd.get_dpd_meaning_snippet(lemma_1)
                .unwrap_or_else(|| {
                    // Fallback to original content if DPD lookup fails
                    let content = x.summary.as_deref()
                        .filter(|s| !s.is_empty())
                        .or(x.definition_plain.as_deref())
                        .filter(|s| !s.is_empty())
                        .or(x.definition_html.as_deref())
                        .unwrap_or("");
                    self.fragment_around_query(&self.query_text, content)
                })
        } else {
            // Non-DPD dictionaries: use original content
            let content = x.summary.as_deref()
                .filter(|s| !s.is_empty())
                .or(x.definition_plain.as_deref())
                .filter(|s| !s.is_empty())
                .or(x.definition_html.as_deref())
                .unwrap_or("");
            self.fragment_around_query(&self.query_text, content)
        };

        SearchResult::from_dict_word(x, snippet)
    }

    fn db_book_spine_item_to_result(&self, x: &BookSpineItem) -> SearchResult {
        let content = x.content_plain.as_deref()
            .filter(|s| !s.is_empty())
            .or(x.content_html.as_deref())
            .unwrap_or("");

        let snippet = self.fragment_around_query(&self.query_text, content);
        SearchResult::from_book_spine_item(x, snippet)
    }

    fn uid_sutta_all(&mut self) -> Result<Vec<SearchResult>, Box<dyn Error>> {
        use crate::db::appdata_schema::suttas::dsl::*;
        use diesel::result::Error as DieselError;

        let app_data = get_app_data();
        let db_conn = &mut app_data.dbm.appdata.get_conn()?;

        let query_uid = self.query_text.to_lowercase()
            .replace("uid:", "")
            .replace(' ', "");

        // First, try exact match
        let mut exact_match_query = suttas.into_boxed();
        exact_match_query = exact_match_query.filter(uid.eq(query_uid.clone()));

        // Apply language filter if specified
        if !self.lang.is_empty() && self.lang != "Language" {
            exact_match_query = exact_match_query.filter(language.eq(&self.lang));
        }

        let exact_match_result = exact_match_query
            .select(Sutta::as_select())
            .first(db_conn);

        match exact_match_result {
            // Found exact match - return single result
            Ok(sutta) => Ok(vec![self.db_sutta_to_result(&sutta)]),
            Err(DieselError::NotFound) => {
                // No exact match found
                // Try range query for both actual ranges like sn56.11-15
                // and for single references like sn17.20 which might fall within a range like sn17.13-20
                match self.uid_sutta_range_all(&query_uid) {
                    Ok(results) if !results.is_empty() => Ok(results),
                    // Range query failed, fall through to LIKE query
                    // For simple references like sn56.11, use LIKE query to get all translations
                    _ => self.uid_sutta_like_all(&query_uid),
                }
            }
            Err(e) => {
                error(&format!("{}", e));
                // return an empty list instead of the error.
                Ok(Vec::new())
            }
        }
    }

    fn uid_sutta_range_all(
        &mut self,
        query_uid: &str,
    ) -> Result<Vec<SearchResult>, Box<dyn Error>> {
        use crate::db::appdata_schema::suttas::dsl::*;

        // Parse the query uid to extract range information
        let range = match sutta_range_from_ref(query_uid) {
            Some(r) => r,
            None => return Ok(Vec::new()),
        };

        // Only proceed if we have both start and end values (meaning it's a numeric query)
        let (range_start, _range_end) = match (range.start, range.end) {
            (Some(s), Some(e)) => (s as i32, e as i32),
            _ => return Ok(Vec::new()),
        };

        let app_data = get_app_data();
        let db_conn = &mut app_data.dbm.appdata.get_conn()?;

        // Build query to find suttas where the query number falls within the stored range
        let mut query = suttas.into_boxed();
        query = query
            .filter(sutta_range_group.eq(&range.group))
            .filter(sutta_range_start.is_not_null())
            .filter(sutta_range_end.is_not_null())
            .filter(sutta_range_start.le(range_start))
            .filter(sutta_range_end.ge(range_start));

        // Apply language filter if specified
        if !self.lang.is_empty() && self.lang != "Language" {
            query = query.filter(language.eq(&self.lang));
        }

        // Execute query
        let results = query
            .order(uid.asc()) // Order by uid for consistent pagination
            .limit(SAFETY_LIMIT_SQL)
            .select(Sutta::as_select())
            .load::<Sutta>(db_conn)?;

        if results.len() as i64 >= SAFETY_LIMIT_SQL {
            warn(&format!(
                "uid_sutta_range_all hit SAFETY_LIMIT_SQL={} (query='{}')",
                SAFETY_LIMIT_SQL, query_uid
            ));
        }

        // Map to SearchResult
        Ok(results.iter().map(|s| self.db_sutta_to_result(s)).collect())
    }

    fn uid_sutta_like_all(
        &mut self,
        query_uid: &str,
    ) -> Result<Vec<SearchResult>, Box<dyn Error>> {
        use crate::db::appdata_schema::suttas::dsl::*;

        info(&format!("uid_sutta_like_all(): query_uid='{}', lang='{}'", query_uid, self.lang));

        let app_data = get_app_data();
        let db_conn = &mut app_data.dbm.appdata.get_conn()?;

        let like_pattern = format!("{}%", query_uid);

        // Build main query with filters
        let mut query = suttas.into_boxed();
        query = query.filter(uid.like(&like_pattern));

        // Apply language filter if specified
        if !self.lang.is_empty() && self.lang != "Language" {
            query = query.filter(language.eq(&self.lang));
        }

        // Execute query
        let results = query
            .order(uid.asc()) // Order by uid for consistent pagination
            .limit(SAFETY_LIMIT_SQL)
            .select(Sutta::as_select())
            .load::<Sutta>(db_conn)?;

        if results.len() as i64 >= SAFETY_LIMIT_SQL {
            warn(&format!(
                "uid_sutta_like_all hit SAFETY_LIMIT_SQL={} (query='{}')",
                SAFETY_LIMIT_SQL, query_uid
            ));
        }

        // Map to SearchResult
        Ok(results.iter().map(|s| self.db_sutta_to_result(s)).collect())
    }

    fn uid_word_all(&mut self) -> Result<Vec<SearchResult>, Box<dyn Error>> {
        use crate::db::dictionaries_schema::dict_words::dsl::*;
        let app_data = get_app_data();

        let query_uid = self.query_text.to_lowercase().replace("uid:", "");

        // Check if this is a DPD numeric UID (e.g., "123/dpd" or just "123")
        // DPD headword UIDs are numeric IDs, optionally with /dpd suffix
        let ref_str = query_uid.replace("/dpd", "");
        if query_uid.ends_with("/dpd") && ref_str.chars().all(char::is_numeric) {
            // Use dpd_lookup which handles numeric UIDs
            return Ok(app_data.dbm.dpd.dpd_lookup(&query_uid, false, true)?);
        }

        let db_conn = &mut app_data.dbm.dictionaries.get_conn()?;

        // First try exact UID match for dict_words
        let res = dict_words
            .filter(uid.eq(&query_uid))
            .select(DictWord::as_select())
            .first(db_conn);

        if let Ok(res_word) = res {
            return Ok(vec![self.db_word_to_result(&res_word)]);
        }
        // Exact match not found, continue to try partial match

        // Fallback: Check if this is a partial UID that needs LIKE query
        // e.g., "dhamma 1" should match "dhamma 1.01/dpd", "dhamma 1.02/dpd", etc.
        // A partial UID has a space followed by a number but no dot after the number
        // Only try this if exact match failed (UIDs like "kamma 1", "kamma 2" exist)
        lazy_static::lazy_static! {
            static ref RE_PARTIAL_DICT_UID: Regex = Regex::new(r"^[a-zāīūṁṃṅñṭḍṇḷ]+ \d+(/[a-z]+)?$").unwrap();
        }
        if RE_PARTIAL_DICT_UID.is_match(&query_uid) {
            // Use LIKE query to find matching UIDs
            // Remove any trailing /dpd for the LIKE pattern
            let base_uid = query_uid.trim_end_matches("/dpd");
            let like_pattern = format!("{}%", base_uid);

            let res: Vec<DictWord> = dict_words
                .filter(uid.like(&like_pattern))
                .order(uid.asc())
                .select(DictWord::as_select())
                .load(db_conn)?;

            return Ok(res.iter().map(|w| self.db_word_to_result(w)).collect());
        }

        // No results found
        Ok(Vec::new())
    }

    fn uid_book_spine_item_all(&mut self) -> Result<Vec<SearchResult>, Box<dyn Error>> {
        use crate::db::appdata_schema::book_spine_items::dsl::*;
        let app_data = get_app_data();
        let db_conn = &mut app_data.dbm.appdata.get_conn()?;

        let query_uid = self.query_text.to_lowercase().replace("uid:", "");

        // If query_uid contains a dot (e.g., "bmc.0"), it's a spine_item_uid
        // If it doesn't contain a dot (e.g., "bmc"), it's a book_uid
        if query_uid.contains('.') {
            // Search for specific spine item
            let res = book_spine_items
                .filter(spine_item_uid.eq(query_uid))
                .select(BookSpineItem::as_select())
                .first(db_conn);

            match res {
                Ok(spine_item) => {
                    Ok(vec![self.db_book_spine_item_to_result(&spine_item)])
                }
                Err(_) => {
                    Ok(Vec::new())
                }
            }
        } else {
            // Search for all spine items with this book_uid
            let res = book_spine_items
                .filter(book_uid.eq(query_uid))
                .select(BookSpineItem::as_select())
                .order(spine_index.asc())
                .load(db_conn);

            match res {
                Ok(spine_items) => {
                    let results: Vec<SearchResult> = spine_items
                        .iter()
                        .map(|item| self.db_book_spine_item_to_result(item))
                        .collect();
                    Ok(results)
                }
                Err(_) => {
                    Ok(Vec::new())
                }
            }
        }
    }

    fn suttas_contains_match_fts5_all(&mut self) -> Result<Vec<SearchResult>, Box<dyn Error>> {
        info(&format!("suttas_contains_match_fts5_all(): query_text: {}, lang filter: {}, include_ms_mula: {}, include_cst_mula: {}, include_cst_commentary: {}", &self.query_text, &self.lang, self.include_ms_mula, self.include_cst_mula, self.include_cst_commentary));
        let timer = Instant::now();

        let app_data = get_app_data();
        let db_conn = &mut app_data.dbm.appdata.get_conn()?;

        let like_pattern = format!("%{}%", self.query_text);

        // Determine if we need language filtering
        let apply_lang_filter = !self.lang.is_empty() && self.lang != "Language";

        // Build dynamic WHERE clauses for CST/commentary filtering
        let mut extra_where = String::new();

        if !self.include_cst_mula {
            extra_where.push_str(
                " AND NOT (s.uid LIKE '%/cst' AND s.uid NOT LIKE '%.att%/cst' AND s.uid NOT LIKE '%.tik%/cst')"
            );
        }

        if !self.include_cst_commentary {
            extra_where.push_str(
                " AND NOT (s.uid LIKE '%.att%/%' OR s.uid LIKE '%.tik%/%')"
            );
        }

        if let Some(prefix) = sanitize_uid_like_prefix(self.nikaya_prefix.as_deref()) {
            extra_where.push_str(&format!(" AND f.nikaya LIKE '{}%'", prefix));
        }

        if let Some(prefix) = sanitize_uid_like_prefix(self.uid_prefix.as_deref()) {
            extra_where.push_str(&format!(" AND f.uid LIKE '{}%'", prefix));
        }

        if !self.include_ms_mula {
            extra_where.push_str(" AND NOT (f.source_uid = 'ms')");
        }

        info(&format!("extra_where: {}", extra_where));

        // --- Execute Query ---
        let select_sql = if apply_lang_filter {
            format!(
                "SELECT s.* FROM suttas_fts f JOIN suttas s ON f.sutta_id = s.id WHERE f.content_plain LIKE ? AND f.language = ?{} ORDER BY s.id LIMIT ?",
                extra_where
            )
        } else {
            format!(
                "SELECT s.* FROM suttas_fts f JOIN suttas s ON f.sutta_id = s.id WHERE f.content_plain LIKE ?{} ORDER BY s.id LIMIT ?",
                extra_where
            )
        };

        let db_results: Vec<Sutta> = if apply_lang_filter {
            sql_query(&select_sql)
                .bind::<Text, _>(&like_pattern)
                .bind::<Text, _>(&self.lang)
                .bind::<BigInt, _>(SAFETY_LIMIT_SQL)
                .load(db_conn)?
        } else {
            sql_query(&select_sql)
                .bind::<Text, _>(&like_pattern)
                .bind::<BigInt, _>(SAFETY_LIMIT_SQL)
                .load(db_conn)?
        };

        if db_results.len() as i64 >= SAFETY_LIMIT_SQL {
            warn(&format!(
                "suttas_contains_match_fts5_all hit SAFETY_LIMIT_SQL={} (query='{}')",
                SAFETY_LIMIT_SQL, &self.query_text
            ));
        }

        // --- Map to SearchResult ---
        let search_results = db_results
            .iter()
            .map(|sutta| self.db_sutta_to_result(sutta))
            .collect();

        info(&format!("Query took: {:?}", timer.elapsed()));
        Ok(search_results)
    }

    fn dict_words_contains_match_fts5_all(&mut self) -> Result<Vec<SearchResult>, Box<dyn Error>> {
        info(&format!("dict_words_contains_match_fts5_all(): query_text: {}", &self.query_text));
        let timer = Instant::now();

        let app_data = get_app_data();
        let db_conn = &mut app_data.dbm.dictionaries.get_conn()?;
        let dpd_conn = &mut app_data.dbm.dpd.get_conn()?;

        use crate::db::dpd_models::DpdHeadword;
        use crate::db::dpd_schema::dpd_headwords::dsl as dpd_dsl;
        use crate::db::dictionaries_schema::dict_words::dsl as dict_dsl;

        // --- Term Filtering ---
        let terms: Vec<&str> = if self.query_text.contains(" AND ") {
            self.query_text.split(" AND ").map(|s| s.trim()).collect()
        } else {
            vec![self.query_text.as_str()]
        };

        // Three-phase search: DpdHeadword exact -> DpdHeadword contains -> DictWord definition

        let mut all_results: Vec<DictWord> = Vec::new();
        let mut result_uids: HashSet<String> = HashSet::new();

        // Phase 1: Exact matches on DpdHeadword.lemma_clean
        // dpd.lemma_clean has btree index and dpd.lemma_1 has unique constraint and so implicitly indexed.
        for term in &terms {
            let exact_matches: Vec<DpdHeadword> = dpd_dsl::dpd_headwords
                .filter(dpd_dsl::lemma_clean.eq(term))
                .order(dpd_dsl::id)
                .limit(SAFETY_LIMIT_SQL)
                .load::<DpdHeadword>(dpd_conn)?;

            // Convert DpdHeadword results to DictWord using their UIDs
            for headword in exact_matches {
                // Use the lemma_1 as the key for deduplication
                let headword_key = headword.lemma_1.clone();

                if !result_uids.contains(&headword_key) {
                    // Find corresponding DictWord by matching the word field to headword.lemma_1
                    let mut dict_query = dict_dsl::dict_words.into_boxed();

                    // Apply source filtering
                    // In the dictionaries.sqlite3, the equivalent of source_uid is dict_label.
                    if let Some(ref source_val) = self.source {
                        if self.source_include {
                            dict_query = dict_query.filter(dict_dsl::dict_label.eq(source_val));
                        } else {
                            dict_query = dict_query.filter(dict_dsl::dict_label.ne(source_val));
                        }
                    }

                    // Match DictWord.word with DpdHeadword.lemma_1
                    let dict_word_result: Result<DictWord, _> = dict_query
                        .filter(dict_dsl::word.eq(&headword.lemma_1))
                        .first::<DictWord>(db_conn);

                    if let Ok(dict_word) = dict_word_result {
                        result_uids.insert(headword_key);
                        all_results.push(dict_word);
                    }
                }
            }
        }

        // Query the FTS table to get headword IDs efficiently
        #[derive(QueryableByName)]
        struct HeadwordId {
            #[diesel(sql_type = diesel::sql_types::Integer)]
            headword_id: i32,
        }

        // Phase 2: Contains matches on DpdHeadword.lemma_1
        // Use dpd_headwords_fts with trigram tokenizer for efficient substring matching
        for term in &terms {
            let like_pattern = format!("%{}%", term);

            let fts_query = String::from(
                r#"
                SELECT headword_id
                FROM dpd_headwords_fts
                WHERE lemma_1 LIKE ?
                ORDER BY headword_id
                LIMIT ?
                "#
            );

            let headword_ids: Vec<HeadwordId> = sql_query(&fts_query)
                .bind::<Text, _>(&like_pattern)
                .bind::<BigInt, _>(SAFETY_LIMIT_SQL)
                .load::<HeadwordId>(dpd_conn)?;

            // Fetch full DpdHeadword records using the IDs
            let ids: Vec<i32> = headword_ids.iter().map(|h| h.headword_id).collect();
            let mut contains_matches: Vec<DpdHeadword> = dpd_dsl::dpd_headwords
                .filter(dpd_dsl::id.eq_any(&ids))
                .load::<DpdHeadword>(dpd_conn)?;

            // Sort by lemma_1 length in ascending order (shorter lemmas first)
            contains_matches.sort_by_key(|h| h.lemma_1.len());

            // Convert DpdHeadword results to DictWord by matching lemma_1 to word
            for headword in contains_matches {
                // Use the lemma_1 as the key for deduplication
                let headword_key = headword.lemma_1.clone();

                if !result_uids.contains(&headword_key) {
                    // Find corresponding DictWord by matching the word field to headword.lemma_1
                    let mut dict_query = dict_dsl::dict_words.into_boxed();

                    // Apply source filtering
                    if let Some(ref source_val) = self.source {
                        if self.source_include {
                            dict_query = dict_query.filter(dict_dsl::dict_label.eq(source_val));
                        } else {
                            dict_query = dict_query.filter(dict_dsl::dict_label.ne(source_val));
                        }
                    }

                    // Match DictWord.word with DpdHeadword.lemma_1
                    let dict_word_result: Result<DictWord, _> = dict_query
                        .filter(dict_dsl::word.eq(&headword.lemma_1))
                        .first::<DictWord>(db_conn);

                    if let Ok(dict_word) = dict_word_result {
                        result_uids.insert(headword_key);
                        all_results.push(dict_word);
                    }
                }
            }
        }

        // Phase 3: FTS5 search on DictWord.definition_plain
        for term in &terms {
            let like_pattern = format!("%{}%", term);

            // Build the FTS5 query with source filtering
            // In the dictionaries.sqlite3, the equivalent of source_uid is dict_label.
            // dict_label is available in the FTS table for filtering
            let fts_query = if self.source.is_some() {
                if self.source_include {
                    String::from(
                        r#"
                        SELECT d.*
                        FROM dict_words_fts f
                        JOIN dict_words d ON f.dict_word_id = d.id
                        WHERE f.definition_plain LIKE ? AND f.dict_label = ?
                        ORDER BY d.id
                        LIMIT ?
                        "#
                    )
                } else {
                    String::from(
                        r#"
                        SELECT d.*
                        FROM dict_words_fts f
                        JOIN dict_words d ON f.dict_word_id = d.id
                        WHERE f.definition_plain LIKE ? AND f.dict_label != ?
                        ORDER BY d.id
                        LIMIT ?
                        "#
                    )
                }
            } else {
                String::from(
                    r#"
                    SELECT d.*
                    FROM dict_words_fts f
                    JOIN dict_words d ON f.dict_word_id = d.id
                    WHERE f.definition_plain LIKE ?
                    ORDER BY d.id
                    LIMIT ?
                    "#
                )
            };

            let def_results: Vec<DictWord> = if let Some(ref source_val) = self.source {
                sql_query(&fts_query)
                    .bind::<Text, _>(&like_pattern)
                    .bind::<Text, _>(source_val)
                    .bind::<BigInt, _>(SAFETY_LIMIT_SQL)
                    .load(db_conn)?
            } else {
                sql_query(&fts_query)
                    .bind::<Text, _>(&like_pattern)
                    .bind::<BigInt, _>(SAFETY_LIMIT_SQL)
                    .load(db_conn)?
            };

            // Add definition results that aren't already included
            for result in def_results {
                if !result_uids.contains(&result.word) {
                    result_uids.insert(result.word.clone());
                    all_results.push(result);
                }
            }
        }

        // Phase 4: Fallback to word_ascii matching if no results found
        // This allows queries like 'sutthu' to find 'suṭṭhu'
        if all_results.is_empty() {
            for term in &terms {
                // Try exact match on word_ascii
                let ascii_matches: Vec<DpdHeadword> = dpd_dsl::dpd_headwords
                    .filter(dpd_dsl::word_ascii.eq(term))
                    .order(dpd_dsl::id)
                    .limit(SAFETY_LIMIT_SQL)
                    .load::<DpdHeadword>(dpd_conn)?;

                for headword in ascii_matches {
                    let headword_key = headword.lemma_1.clone();

                    if !result_uids.contains(&headword_key) {
                        let mut dict_query = dict_dsl::dict_words.into_boxed();

                        if let Some(ref source_val) = self.source {
                            if self.source_include {
                                dict_query = dict_query.filter(dict_dsl::dict_label.eq(source_val));
                            } else {
                                dict_query = dict_query.filter(dict_dsl::dict_label.ne(source_val));
                            }
                        }

                        let dict_word_result: Result<DictWord, _> = dict_query
                            .filter(dict_dsl::word.eq(&headword.lemma_1))
                            .first::<DictWord>(db_conn);

                        if let Ok(dict_word) = dict_word_result {
                            result_uids.insert(headword_key);
                            all_results.push(dict_word);
                        }
                    }
                }

                // If still no results, try contains match on word_ascii
                if all_results.is_empty() {
                    let like_pattern = format!("%{}%", term);

                    let fts_query = String::from(
                        r#"
                        SELECT id
                        FROM dpd_headwords
                        WHERE word_ascii LIKE ?
                        ORDER BY id
                        LIMIT ?
                        "#
                    );

                    let headword_ids: Vec<HeadwordId> = sql_query(&fts_query)
                        .bind::<Text, _>(&like_pattern)
                        .bind::<BigInt, _>(SAFETY_LIMIT_SQL)
                        .load::<HeadwordId>(dpd_conn)?;

                    let ids: Vec<i32> = headword_ids.iter().map(|h| h.headword_id).collect();
                    let mut contains_matches: Vec<DpdHeadword> = dpd_dsl::dpd_headwords
                        .filter(dpd_dsl::id.eq_any(&ids))
                        .load::<DpdHeadword>(dpd_conn)?;

                    contains_matches.sort_by_key(|h| h.lemma_1.len());

                    for headword in contains_matches {
                        let headword_key = headword.lemma_1.clone();

                        if !result_uids.contains(&headword_key) {
                            let mut dict_query = dict_dsl::dict_words.into_boxed();

                            if let Some(ref source_val) = self.source {
                                if self.source_include {
                                    dict_query = dict_query.filter(dict_dsl::dict_label.eq(source_val));
                                } else {
                                    dict_query = dict_query.filter(dict_dsl::dict_label.ne(source_val));
                                }
                            }

                            let dict_word_result: Result<DictWord, _> = dict_query
                                .filter(dict_dsl::word.eq(&headword.lemma_1))
                                .first::<DictWord>(db_conn);

                            if let Ok(dict_word) = dict_word_result {
                                result_uids.insert(headword_key);
                                all_results.push(dict_word);
                            }
                        }
                    }
                }
            }
        }

        if all_results.len() as i64 >= SAFETY_LIMIT_SQL {
            warn(&format!(
                "dict_words_contains_match_fts5_all hit SAFETY_LIMIT_SQL={} (query='{}')",
                SAFETY_LIMIT_SQL, &self.query_text
            ));
        }

        // Map to SearchResult
        let search_results: Vec<SearchResult> = all_results
            .iter()
            .map(|dict_word| self.db_word_to_result(dict_word))
            .collect();

        info(&format!("Query took: {:?}", timer.elapsed()));
        Ok(search_results)
    }

    fn book_spine_items_contains_match_fts5_all(&mut self) -> Result<Vec<SearchResult>, Box<dyn Error>> {
        info(&format!("book_spine_items_contains_match_fts5_all(): query_text: {}, lang filter: {}", &self.query_text, &self.lang));
        let timer = Instant::now();

        let app_data = get_app_data();
        let db_conn = &mut app_data.dbm.appdata.get_conn()?;

        let like_pattern = format!("%{}%", self.query_text);

        // Determine if we need language filtering
        let apply_lang_filter = !self.lang.is_empty() && self.lang != "Language";

        // --- Execute Query ---
        let db_results: Vec<BookSpineItem> = if apply_lang_filter {
            sql_query(
                r#"
                SELECT b.*
                FROM book_spine_items_fts f
                JOIN book_spine_items b ON f.spine_item_id = b.id
                WHERE f.content_plain LIKE ? AND f.language = ?
                ORDER BY b.id
                LIMIT ?
                "#
            )
            .bind::<Text, _>(&like_pattern)
            .bind::<Text, _>(&self.lang)
            .bind::<BigInt, _>(SAFETY_LIMIT_SQL)
            .load(db_conn)?
        } else {
            sql_query(
                r#"
                SELECT b.*
                FROM book_spine_items_fts f
                JOIN book_spine_items b ON f.spine_item_id = b.id
                WHERE f.content_plain LIKE ?
                ORDER BY b.id
                LIMIT ?
                "#
            )
            .bind::<Text, _>(&like_pattern)
            .bind::<BigInt, _>(SAFETY_LIMIT_SQL)
            .load(db_conn)?
        };

        if db_results.len() as i64 >= SAFETY_LIMIT_SQL {
            warn(&format!(
                "book_spine_items_contains_match_fts5_all hit SAFETY_LIMIT_SQL={} (query='{}')",
                SAFETY_LIMIT_SQL, &self.query_text
            ));
        }

        // --- Map to SearchResult ---
        let search_results = db_results
            .iter()
            .map(|spine_item| self.db_book_spine_item_to_result(spine_item))
            .collect();

        info(&format!("Query took: {:?}", timer.elapsed()));
        Ok(search_results)
    }

    /// Substring match on bold_definitions.bold using the trigram FTS5 index.
    /// Used by DPD Lookup and Headword Match.
    /// The query is lowercased for matching; **not** run through the Pāli
    /// normalization pipeline (DPD Lookup / Headword Match operate on the
    /// as-stored bold field). Caller is responsible for gating on
    /// `include_comm_bold_definitions`.
    fn query_bold_definitions_bold_fts5(
        &self,
        query: &str,
    ) -> Result<Vec<SearchResult>, Box<dyn Error>> {
        use crate::db::dpd_models::BoldDefinition;
        use diesel::sql_types::BigInt;

        let q = query.trim().to_lowercase();
        if q.is_empty() {
            return Ok(Vec::new());
        }

        let app_data = get_app_data();
        let dpd_conn = &mut app_data.dbm.dpd.get_conn()?;

        let like_pattern = format!("%{}%", q);
        // Match against both the original `bold` (e.g. "suṭṭhu") and the
        // ASCII-folded `bold_ascii` (e.g. "sutthu") so ASCII queries find
        // diacritic entries — mirrors the word_ascii lookup path.
        //
        // Apply uid_prefix / uid_suffix at the SQL level so the row budget
        // (LIMIT) is spent on rows that will survive the post-filter. The
        // filters default to `%` (match anything) when unset so the bind
        // count is constant.
        let uid_prefix_pat = Self::normalized_filter(&self.uid_prefix)
            .map(|p| format!("{}%", p))
            .unwrap_or_else(|| "%".to_string());
        let uid_suffix_pat = Self::normalized_filter(&self.uid_suffix)
            .map(|s| format!("%{}", s))
            .unwrap_or_else(|| "%".to_string());
        let sql = r#"
            SELECT bd.*
            FROM bold_definitions_bold_fts f
            JOIN bold_definitions bd ON bd.id = f.bold_definitions_id
            WHERE (f.bold LIKE ? OR f.bold_ascii LIKE ?)
              AND bd.uid LIKE ?
              AND bd.uid LIKE ?
            ORDER BY bd.id
            LIMIT ?
        "#;

        let bds: Vec<BoldDefinition> = sql_query(sql)
            .bind::<Text, _>(&like_pattern)
            .bind::<Text, _>(&like_pattern)
            .bind::<Text, _>(&uid_prefix_pat)
            .bind::<Text, _>(&uid_suffix_pat)
            .bind::<BigInt, _>(SAFETY_LIMIT_SQL)
            .load(dpd_conn)?;

        if bds.len() as i64 >= SAFETY_LIMIT_SQL {
            warn(&format!(
                "query_bold_definitions_bold_fts5 hit SAFETY_LIMIT_SQL={} (query='{}')",
                SAFETY_LIMIT_SQL, q
            ));
        }

        Ok(bds.iter().map(bold_definition_to_search_result).collect())
    }

    /// Substring match on bold_definitions.commentary_plain using the trigram
    /// FTS5 index. Used by Contains Match. Caller is responsible for gating
    /// on `include_comm_bold_definitions`.
    fn query_bold_definitions_commentary_fts5(
        &self,
        normalized_query: &str,
    ) -> Result<Vec<SearchResult>, Box<dyn Error>> {
        use crate::db::dpd_models::BoldDefinition;
        use diesel::sql_types::BigInt;

        let q = normalized_query.trim();
        if q.is_empty() {
            return Ok(Vec::new());
        }

        let app_data = get_app_data();
        let dpd_conn = &mut app_data.dbm.dpd.get_conn()?;

        let like_pattern = format!("%{}%", q);
        // Push uid_prefix / uid_suffix down to SQL so the LIMIT is spent on
        // rows that will survive the post-filter. Default patterns are `%`.
        let uid_prefix_pat = Self::normalized_filter(&self.uid_prefix)
            .map(|p| format!("{}%", p))
            .unwrap_or_else(|| "%".to_string());
        let uid_suffix_pat = Self::normalized_filter(&self.uid_suffix)
            .map(|s| format!("%{}", s))
            .unwrap_or_else(|| "%".to_string());
        let sql = r#"
            SELECT bd.*
            FROM bold_definitions_fts f
            JOIN bold_definitions bd ON bd.id = f.bold_definitions_id
            WHERE f.commentary_plain LIKE ?
              AND bd.uid LIKE ?
              AND bd.uid LIKE ?
            ORDER BY bd.id
            LIMIT ?
        "#;

        let bds: Vec<BoldDefinition> = sql_query(sql)
            .bind::<Text, _>(&like_pattern)
            .bind::<Text, _>(&uid_prefix_pat)
            .bind::<Text, _>(&uid_suffix_pat)
            .bind::<BigInt, _>(SAFETY_LIMIT_SQL)
            .load(dpd_conn)?;

        if bds.len() as i64 >= SAFETY_LIMIT_SQL {
            warn(&format!(
                "query_bold_definitions_commentary_fts5 hit SAFETY_LIMIT_SQL={} (query='{}')",
                SAFETY_LIMIT_SQL, q
            ));
        }

        Ok(bds.iter().map(bold_definition_to_search_result).collect())
    }

    /// DPD Lookup — returns all headword results unpaginated. DPD is only
    /// Pāli-to-English so language filters are ignored. Pagination, bold-definition
    /// merging, uid filtering, and highlighting are owned by `results_page`.
    /// Assume that if the DPD Lookup was selected then stale language settings can be ignored.
    pub fn dpd_lookup_all(&mut self) -> Result<Vec<SearchResult>, Box<dyn Error>> {
        let app_data = get_app_data();
        app_data
            .dbm
            .dpd
            .dpd_lookup(&self.query_text, false, true)
            .map_err(Into::into)
    }

    fn suttas_title_match_all(&mut self) -> Result<Vec<SearchResult>, Box<dyn Error>> {
        info(&format!("suttas_title_match_all(): query_text: {}, lang filter: {}", &self.query_text, &self.lang));
        let timer = Instant::now();

        use crate::db::appdata_schema::suttas::dsl::*;

        let app_data = get_app_data();
        let db_conn = &mut app_data.dbm.appdata.get_conn()?;

        let like_pattern = format!("%{}%", self.query_text);

        // Apply title search filter (search in both title and title_ascii)
        let mut query = suttas.into_boxed();
        query = query.filter(
            title.like(&like_pattern)
            .or(title_ascii.like(&like_pattern))
        );

        // Apply language filter if specified
        if !self.lang.is_empty() && self.lang != "Language" {
            query = query.filter(language.eq(&self.lang));
        }

        // --- CST Mūla Filtering ---
        if !self.include_cst_mula {
            query = query.filter(
                diesel::dsl::not(
                    uid.like("%/cst")
                        .and(uid.not_like("%.att%/cst"))
                        .and(uid.not_like("%.tik%/cst"))
                )
            );
        }

        // --- Commentary Filtering ---
        if !self.include_cst_commentary {
            query = query.filter(
                diesel::dsl::not(
                    uid.like("%.att%/%")
                    .or(uid.like("%.tik%/%"))
                )
            );
        }

        // --- Nikaya prefix filtering ---
        if let Some(prefix) = sanitize_uid_like_prefix(self.nikaya_prefix.as_deref()) {
            query = query.filter(nikaya.like(format!("{}%", prefix)));
        }

        // --- UID prefix filtering ---
        if let Some(prefix) = sanitize_uid_like_prefix(self.uid_prefix.as_deref()) {
            query = query.filter(uid.like(format!("{}%", prefix)));
        }

        // --- MS Mūla Filtering ---
        if !self.include_ms_mula {
            query = query.filter(diesel::dsl::not(source_uid.eq("ms")));
        }

        // Execute query
        let db_results: Vec<Sutta> = query
            .order(uid.asc())
            .limit(SAFETY_LIMIT_SQL)
            .select(Sutta::as_select())
            .load(db_conn)?;

        if db_results.len() as i64 >= SAFETY_LIMIT_SQL {
            warn(&format!(
                "suttas_title_match_all hit SAFETY_LIMIT_SQL={} (query='{}')",
                SAFETY_LIMIT_SQL, &self.query_text
            ));
        }

        // Map to SearchResult
        let search_results = db_results
            .iter()
            .map(|sutta| self.db_sutta_to_result(sutta))
            .collect();

        info(&format!("Query took: {:?}", timer.elapsed()));
        Ok(search_results)
    }

    fn library_title_match_all(&mut self) -> Result<Vec<SearchResult>, Box<dyn Error>> {
        info(&format!("library_title_match_all(): query_text: {}, lang filter: {}", &self.query_text, &self.lang));
        let timer = Instant::now();

        use crate::db::appdata_schema::books::dsl as books_dsl;
        use crate::db::appdata_schema::book_spine_items::dsl as spine_dsl;

        let app_data = get_app_data();
        let db_conn = &mut app_data.dbm.appdata.get_conn()?;

        let like_pattern = format!("%{}%", self.query_text);

        // Determine if we need language filtering
        let apply_lang_filter = !self.lang.is_empty() && self.lang != "Language";

        let mut all_results: Vec<SearchResult> = Vec::new();

        // Books: emit one SearchResult per book (first spine item).
        // Always execute books query
        let mut books_query = books_dsl::books.into_boxed();
        books_query = books_query.filter(books_dsl::title.like(&like_pattern));

        if apply_lang_filter {
            books_query = books_query.filter(books_dsl::language.eq(&self.lang));
        }

        let book_uids: Vec<String> = books_query
            .order(books_dsl::id.asc())
            .limit(SAFETY_LIMIT_SQL)
            .select(books_dsl::uid)
            .load(db_conn)?;

        // For each book, get the first spine item
        for book_uid in book_uids {
            let first_spine_item: Result<BookSpineItem, _> = spine_dsl::book_spine_items
                .filter(spine_dsl::book_uid.eq(&book_uid))
                .order(spine_dsl::spine_index.asc())
                .first::<BookSpineItem>(db_conn);

            if let Ok(spine_item) = first_spine_item {
                all_results.push(self.db_book_spine_item_to_result(&spine_item));
            }
        }

        // Spine items: emit one SearchResult per matching spine item.
        // Always execute spine_items query
        let spine_results: Vec<BookSpineItem> = if apply_lang_filter {
            sql_query(
                r#"
                SELECT b.*
                FROM book_spine_items_fts f
                JOIN book_spine_items b ON f.spine_item_id = b.id
                WHERE f.title LIKE ? AND f.language = ?
                ORDER BY b.id
                LIMIT ?
                "#
            )
            .bind::<Text, _>(&like_pattern)
            .bind::<Text, _>(&self.lang)
            .bind::<BigInt, _>(SAFETY_LIMIT_SQL)
            .load(db_conn)?
        } else {
            sql_query(
                r#"
                SELECT b.*
                FROM book_spine_items_fts f
                JOIN book_spine_items b ON f.spine_item_id = b.id
                WHERE f.title LIKE ?
                ORDER BY b.id
                LIMIT ?
                "#
            )
            .bind::<Text, _>(&like_pattern)
            .bind::<BigInt, _>(SAFETY_LIMIT_SQL)
            .load(db_conn)?
        };

        for spine_item in spine_results {
            all_results.push(self.db_book_spine_item_to_result(&spine_item));
        }

        if all_results.len() as i64 >= SAFETY_LIMIT_SQL {
            warn(&format!(
                "library_title_match_all hit SAFETY_LIMIT_SQL={} (query='{}')",
                SAFETY_LIMIT_SQL, &self.query_text
            ));
        }

        info(&format!("Query took: {:?}", timer.elapsed()));
        Ok(all_results)
    }

    fn lemma_1_dpd_headword_match_fts5_all(&mut self) -> Result<Vec<SearchResult>, Box<dyn Error>> {
        info(&format!("lemma_1_dpd_headword_match_fts5_all(): query_text: {}", &self.query_text));
        let timer = Instant::now();

        let app_data = get_app_data();
        let dpd_conn = &mut app_data.dbm.dpd.get_conn()?;
        let dict_conn = &mut app_data.dbm.dictionaries.get_conn()?;

        use crate::db::dpd_models::DpdHeadword;
        use crate::db::dpd_schema::dpd_headwords::dsl as dpd_dsl;
        use crate::db::dictionaries_schema::dict_words::dsl as dict_dsl;

        let like_pattern = format!("%{}%", self.query_text);

        // Query the FTS table to get headword IDs efficiently
        #[derive(QueryableByName)]
        struct HeadwordId {
            #[diesel(sql_type = diesel::sql_types::Integer)]
            headword_id: i32,
        }

        // Get headword IDs from FTS
        let fts_query = String::from(
            r#"
            SELECT headword_id
            FROM dpd_headwords_fts
            WHERE lemma_1 LIKE ?
            ORDER BY headword_id
            LIMIT ?
            "#
        );

        let headword_ids: Vec<HeadwordId> = sql_query(&fts_query)
            .bind::<Text, _>(&like_pattern)
            .bind::<BigInt, _>(SAFETY_LIMIT_SQL)
            .load::<HeadwordId>(dpd_conn)?;

        if headword_ids.len() as i64 >= SAFETY_LIMIT_SQL {
            warn(&format!(
                "lemma_1_dpd_headword_match_fts5_all hit SAFETY_LIMIT_SQL={} (query='{}')",
                SAFETY_LIMIT_SQL, &self.query_text
            ));
        }

        // Fetch full DpdHeadword records using the IDs
        let ids: Vec<i32> = headword_ids.iter().map(|h| h.headword_id).collect();

        if ids.is_empty() {
            return Ok(Vec::new());
        }

        let headwords: Vec<DpdHeadword> = dpd_dsl::dpd_headwords
            .filter(dpd_dsl::id.eq_any(&ids))
            .load::<DpdHeadword>(dpd_conn)?;

        // Convert DpdHeadword results to SearchResults via DictWord
        let mut search_results: Vec<SearchResult> = Vec::new();

        for headword in headwords {
            // Try to find corresponding DictWord by matching word field to headword.lemma_1
            let mut dict_query = dict_dsl::dict_words.into_boxed();

            // Apply source filtering if specified
            if let Some(ref source_val) = self.source {
                if self.source_include {
                    dict_query = dict_query.filter(dict_dsl::dict_label.eq(source_val));
                } else {
                    dict_query = dict_query.filter(dict_dsl::dict_label.ne(source_val));
                }
            }

            // Match DictWord.word with DpdHeadword.lemma_1
            let dict_word_result: Result<DictWord, _> = dict_query
                .filter(dict_dsl::word.eq(&headword.lemma_1))
                .first::<DictWord>(dict_conn);

            if let Ok(dict_word) = dict_word_result {
                search_results.push(self.db_word_to_result(&dict_word));
            }
        }

        info(&format!("Query took: {:?}", timer.elapsed()));
        Ok(search_results)
    }

    /// Dispatches to the area/mode-specific `_all` handler, returning the
    /// full unpaginated result set (capped by SAFETY_LIMIT). Does not apply
    /// uid filters, bold-definition merging, or highlighting — those are
    /// owned by `results_page`.
    fn fetch_regular_unpaginated(&mut self) -> Result<Vec<SearchResult>, Box<dyn Error>> {
        match self.search_mode {
            SearchMode::DpdLookup => match self.search_area {
                // DPD Lookup mode - only works for Dictionary search area
                SearchArea::Dictionary => self.dpd_lookup_all(),
                _ => Ok(Vec::new()),
            },

            SearchMode::Combined => Ok(Vec::new()),

            SearchMode::UidMatch => match self.search_area {
                SearchArea::Suttas => self.uid_sutta_all(),
                SearchArea::Dictionary => self.uid_word_all(),
                SearchArea::Library => self.uid_book_spine_item_all(),
            },

            SearchMode::ContainsMatch => match self.search_area {
                SearchArea::Suttas => self.suttas_contains_match_fts5_all(),
                SearchArea::Dictionary => self.dict_words_contains_match_fts5_all(),
                SearchArea::Library => self.book_spine_items_contains_match_fts5_all(),
            },

            SearchMode::TitleMatch => match self.search_area {
                SearchArea::Suttas => self.suttas_title_match_all(),
                // Title Match doesn't make sense for dictionary
                SearchArea::Dictionary => Ok(Vec::new()),
                // Search in the book and book_spine_item chapter titles
                SearchArea::Library => self.library_title_match_all(),
            },

            SearchMode::HeadwordMatch => match self.search_area {
                SearchArea::Dictionary => self.lemma_1_dpd_headword_match_fts5_all(),
                // Headword Match doesn't make sense for suttas and library
                _ => Ok(Vec::new()),
            },

            SearchMode::FulltextMatch => match self.search_area {
                SearchArea::Suttas => self.fulltext_suttas_all(),
                SearchArea::Dictionary => self.fulltext_dict_words_all(),
                SearchArea::Library => self.fulltext_library_all(),
            },

            _ => {
                error(&format!("Search mode {:?} not yet implemented.", self.search_mode));
                Ok(Vec::new())
            }
        }
    }

    /// Stage C: returns true iff bold-definition fetching is enabled for the
    /// current mode/area. (PRD §4.3.12: bold definitions are dictionary-only.)
    fn should_fetch_bold(&self) -> bool {
        self.search_area == SearchArea::Dictionary
            && self.include_comm_bold_definitions
            && matches!(
                self.search_mode,
                SearchMode::DpdLookup
                    | SearchMode::HeadwordMatch
                    | SearchMode::ContainsMatch
                    | SearchMode::FulltextMatch
            )
    }

    /// Stage C: dispatches to the appropriate bold-definition helper for the
    /// current mode, returning all hits unpaginated (capped by SAFETY_LIMIT).
    /// The Contains/Fulltext modes normalise the query first because the
    /// stored `commentary_plain` is normalised Pāli.
    fn fetch_bold_unpaginated(&self) -> Result<Vec<SearchResult>, Box<dyn Error>> {
        match self.search_mode {
            SearchMode::DpdLookup | SearchMode::HeadwordMatch => {
                self.query_bold_definitions_bold_fts5(&self.query_text)
            }
            SearchMode::ContainsMatch => {
                let q = normalize_plain_text(&self.query_text);
                self.query_bold_definitions_commentary_fts5(&q)
            }
            SearchMode::FulltextMatch => {
                let q = normalize_plain_text(&self.query_text);
                self.query_bold_definitions_fulltext_all(&q)
            }
            _ => Ok(Vec::new()),
        }
    }

    /// Stage D: stable linear merge of two score-sorted vectors. Each input
    /// is assumed to already be sorted by descending `SearchResult.score`;
    /// the output preserves that ordering and never drops items. Inter-index
    /// BM25 scores are not strictly comparable so the relative bias between
    /// indexes is acceptable per PRD §4.3.12.
    fn merge_by_score_desc(
        a: Vec<SearchResult>,
        b: Vec<SearchResult>,
    ) -> Vec<SearchResult> {
        let mut out = Vec::with_capacity(a.len() + b.len());
        let mut ai = a.into_iter().peekable();
        let mut bi = b.into_iter().peekable();
        loop {
            match (ai.peek(), bi.peek()) {
                (Some(x), Some(y)) => {
                    let sx = x.score.unwrap_or(0.0);
                    let sy = y.score.unwrap_or(0.0);
                    if sx >= sy {
                        out.push(ai.next().unwrap());
                    } else {
                        out.push(bi.next().unwrap());
                    }
                }
                (Some(_), None) => {
                    out.extend(ai);
                    break;
                }
                (None, Some(_)) => {
                    out.extend(bi);
                    break;
                }
                (None, None) => break,
            }
        }
        out
    }

    /// Returns a lowercased, trimmed string if the option is `Some` and non-empty.
    fn normalized_filter(opt: &Option<String>) -> Option<String> {
        opt.as_ref()
            .map(|s| s.trim().to_lowercase())
            .filter(|s| !s.is_empty())
    }

    // (needs_post_filter removed in Stage D — uid filtering is now applied
    // unconditionally inside results_page via apply_uid_filters.)

    fn apply_uid_filters(&self, results: Vec<SearchResult>) -> Vec<SearchResult> {
        let suffix = Self::normalized_filter(&self.uid_suffix);
        let prefix = Self::normalized_filter(&self.uid_prefix);
        let prefix_handled_by_sql = matches!(self.search_area, SearchArea::Suttas);
        if suffix.is_none() && (prefix.is_none() || prefix_handled_by_sql) {
            return results;
        }
        results
            .into_iter()
            .filter(|r| {
                let uid_lc = r.uid.to_lowercase();
                if let Some(ref s) = suffix {
                    if !uid_lc.ends_with(s) {
                        return false;
                    }
                }
                if !prefix_handled_by_sql {
                    if let Some(ref p) = prefix {
                        if !uid_lc.starts_with(p) {
                            return false;
                        }
                    }
                }
                true
            })
            .collect()
    }

    /// Gets a specific page of search results.
    ///
    /// Stage D pipeline:
    ///   1. fetch regular results unpaginated (mode/area `_all` handler)
    ///   2. fetch bold-definition results unpaginated (gated)
    ///   3. mode-specific merge (score-desc for Fulltext; concat otherwise)
    ///   4. apply uid filters; record `db_query_hits_count` exactly once
    ///   5. paginate
    ///   6. highlight only the returned page
    pub fn results_page(&mut self, page_num: usize) -> Result<Vec<SearchResult>, Box<dyn Error>> {
        let regular = self.fetch_regular_unpaginated()?;
        let bold = if self.should_fetch_bold() {
            self.fetch_bold_unpaginated()?
        } else {
            Vec::new()
        };

        let merged = match self.search_mode {
            SearchMode::FulltextMatch => Self::merge_by_score_desc(regular, bold),
            _ => {
                let mut v = regular;
                v.extend(bold);
                v
            }
        };

        let filtered = self.apply_uid_filters(merged);
        self.db_query_hits_count = filtered.len() as i64;

        let start = page_num * self.page_len;
        let end = std::cmp::min(start + self.page_len, filtered.len());
        let page: Vec<SearchResult> = if start >= filtered.len() {
            Vec::new()
        } else {
            filtered[start..end].to_vec()
        };

        let highlighted_results: Vec<SearchResult> = page
            .into_iter()
            .map(|mut result| {
                // Skip highlighting for DPD results (dpd_headwords, dpd_roots,
                // dict_words with DPD source) — they already carry formatted
                // meaning snippets from get_dpd_meaning_snippet().
                let is_dpd_result = result.table_name == "dpd_headwords"
                    || result.table_name == "dpd_roots"
                    || (result.table_name == "dict_words"
                        && result.source_uid.as_ref().is_some_and(|s| s.to_lowercase().contains("dpd")));

                if !is_dpd_result {
                    // Re-highlight the snippet based on the full query text
                    // Note: db_sutta_to_result() already created a basic snippet.
                    // This step applies the final highlighting spans.
                    result.snippet = self.highlight_query_in_content(&self.query_text, &result.snippet);
                }
                result
            })
            .collect();

        Ok(highlighted_results)
    }

    // ===== Fulltext Search Methods =====

    fn fulltext_suttas_all(&mut self) -> Result<Vec<SearchResult>, Box<dyn Error>> {
        use crate::with_fulltext_searcher;
        use crate::search::searcher::SearchFilters;

        let filters = SearchFilters {
            lang: if !self.lang.is_empty() && self.lang != "Language" {
                Some(self.lang.clone())
            } else {
                None
            },
            lang_include: self.lang_include,
            source_uid: self.source.clone(),
            source_include: self.source_include,
            nikaya_prefix: self.nikaya_prefix.clone(),
            uid_prefix: self.uid_prefix.clone(),
            sutta_ref: None,
            include_cst_mula: self.include_cst_mula,
            include_cst_commentary: self.include_cst_commentary,
            include_ms_mula: self.include_ms_mula,
        };

        let query_text = self.query_text.clone();

        let results = match with_fulltext_searcher(|searcher| {
            if !searcher.has_sutta_indexes() {
                warn("No sutta fulltext indexes available.");
                return Ok((0, Vec::new()));
            }
            searcher.search_suttas_with_count(&query_text, &filters, SAFETY_LIMIT_TANTIVY, 0)
        }) {
            Some(Ok((_total, results))) => results,
            Some(Err(e)) => return Err(e.into()),
            None => {
                warn("Fulltext searcher not initialized. Indexes may not exist.");
                Vec::new()
            }
        };

        if results.len() >= SAFETY_LIMIT_TANTIVY {
            warn(&format!(
                "fulltext_suttas_all hit SAFETY_LIMIT_TANTIVY={} (query='{}')",
                SAFETY_LIMIT_TANTIVY, &self.query_text
            ));
        }

        Ok(results)
    }

    fn fulltext_dict_words_all(&mut self) -> Result<Vec<SearchResult>, Box<dyn Error>> {
        use crate::with_fulltext_searcher;
        use crate::search::searcher::SearchFilters;

        let filters = SearchFilters {
            lang: if !self.lang.is_empty() && self.lang != "Language" {
                Some(self.lang.clone())
            } else {
                None
            },
            lang_include: self.lang_include,
            source_uid: self.source.clone(),
            source_include: self.source_include,
            nikaya_prefix: None,
            uid_prefix: None,
            sutta_ref: None,
            include_cst_mula: true,
            include_cst_commentary: true,
            include_ms_mula: true,
        };

        let query_text = self.query_text.clone();

        let results = match with_fulltext_searcher(|searcher| {
            if !searcher.has_dict_indexes() {
                warn("No dict_word fulltext indexes available.");
                return Ok((0, Vec::new()));
            }
            searcher.search_dict_words_with_count(&query_text, &filters, SAFETY_LIMIT_TANTIVY, 0)
        }) {
            Some(Ok((_total, results))) => results,
            Some(Err(e)) => return Err(e.into()),
            None => {
                warn("Fulltext searcher not initialized. Indexes may not exist.");
                Vec::new()
            }
        };

        if results.len() >= SAFETY_LIMIT_TANTIVY {
            warn(&format!(
                "fulltext_dict_words_all hit SAFETY_LIMIT_TANTIVY={} (query='{}')",
                SAFETY_LIMIT_TANTIVY, &self.query_text
            ));
        }

        Ok(results)
    }

    fn fulltext_library_all(&mut self) -> Result<Vec<SearchResult>, Box<dyn Error>> {
        use crate::with_fulltext_searcher;
        use crate::search::searcher::SearchFilters;

        let filters = SearchFilters {
            lang: if !self.lang.is_empty() && self.lang != "Language" {
                Some(self.lang.clone())
            } else {
                None
            },
            lang_include: self.lang_include,
            source_uid: None,
            source_include: false,
            nikaya_prefix: None,
            uid_prefix: None,
            sutta_ref: None,
            include_cst_mula: true,
            include_cst_commentary: true,
            include_ms_mula: true,
        };

        let query_text = self.query_text.clone();

        let results = match with_fulltext_searcher(|searcher| {
            if !searcher.has_library_indexes() {
                warn("No library fulltext indexes available.");
                return Ok((0, Vec::new()));
            }
            searcher.search_library_with_count(&query_text, &filters, SAFETY_LIMIT_TANTIVY, 0)
        }) {
            Some(Ok((_total, results))) => results,
            Some(Err(e)) => return Err(e.into()),
            None => {
                warn("Fulltext searcher not initialized. Indexes may not exist.");
                Vec::new()
            }
        };

        if results.len() >= SAFETY_LIMIT_TANTIVY {
            warn(&format!(
                "fulltext_library_all hit SAFETY_LIMIT_TANTIVY={} (query='{}')",
                SAFETY_LIMIT_TANTIVY, &self.query_text
            ));
        }

        Ok(results)
    }

    /// Returns the total number of hits found in the last database query.
    pub fn total_hits(&self) -> i64 {
        self.db_query_hits_count
    }

    /// Stage C helper: returns up to SAFETY_LIMIT_TANTIVY score-sorted bold
    /// fulltext hits from page 0. The returned `SearchResult.score` is preserved
    /// for downstream score-aware merging.
    fn query_bold_definitions_fulltext_all(
        &self,
        normalized_query: &str,
    ) -> Result<Vec<SearchResult>, Box<dyn Error>> {
        use crate::with_fulltext_searcher;
        use crate::search::searcher::SearchFilters;

        // Bold fetching deliberately passes uid_prefix/uid_suffix=None and an
        // empty SearchFilters: uid gating is owned by the unified Rust filter
        // in Stage F (analysis §7 decision 2.6). Other filters (lang, source,
        // CST/MS) are not meaningful for bold definitions. Do not "fix" this
        // by re-pushing filters down — the FTS5 bold helpers retain their
        // optional `bd.uid LIKE ?` push-down purely as an optimisation.
        let filters = SearchFilters {
            lang: None,
            lang_include: false,
            source_uid: None,
            source_include: false,
            nikaya_prefix: None,
            uid_prefix: None,
            sutta_ref: None,
            include_cst_mula: true,
            include_cst_commentary: true,
            include_ms_mula: true,
        };

        let query_text = normalized_query.to_string();

        let out = with_fulltext_searcher(|searcher| {
            if !searcher.has_bold_definitions_index() {
                return Ok::<_, anyhow::Error>((0usize, Vec::<SearchResult>::new()));
            }
            searcher.search_bold_definitions_with_count(
                &query_text, &filters, SAFETY_LIMIT_TANTIVY, 0,
            )
        });

        let results = match out {
            Some(Ok((_total, results))) => results,
            Some(Err(e)) => return Err(e.into()),
            None => Vec::new(),
        };

        if results.len() >= SAFETY_LIMIT_TANTIVY {
            warn(&format!(
                "query_bold_definitions_fulltext_all hit SAFETY_LIMIT_TANTIVY={} (query='{}')",
                SAFETY_LIMIT_TANTIVY, normalized_query
            ));
        }

        Ok(results)
    }

}
