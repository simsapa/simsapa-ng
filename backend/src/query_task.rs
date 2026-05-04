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
use crate::logger::{debug, info, warn, error};

/// Defense-in-depth ceiling on SQL `LIMIT` for unbounded multi-phase fetches
/// (e.g. dict_words_contains_match_fts5's per-phase intermediate fetch). Real
/// pagination is done with `LIMIT page_len OFFSET page_num*page_len`; this
/// constant only caps pathological per-phase intermediate sets that aren't
/// yet expressed as a single paged SQL query.
pub(crate) const SAFETY_LIMIT_SQL: i64 = 50_000;

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
    pub dict_source_uids: Option<Vec<String>>,
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
            dict_source_uids: params.dict_source_uids.clone(),
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

        if (results.len() as i64) >= SAFETY_LIMIT_SQL {
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

        if (results.len() as i64) >= SAFETY_LIMIT_SQL {
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
            return Ok(app_data.dbm.dpd.dpd_lookup(&query_uid, false, true, None, None)?);
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

    /// Per-page contains-match against suttas_fts. Pushes uid_prefix and
    /// uid_suffix down as `suttas.uid LIKE ?` clauses, and runs a parallel
    /// COUNT(*) over the same predicate so the caller has the true post-filter
    /// hit count without materializing every row.
    fn suttas_contains_match_fts5(
        &self,
        page_num: usize,
        page_len: usize,
    ) -> Result<(Vec<SearchResult>, usize), Box<dyn Error>> {
        info(&format!("suttas_contains_match_fts5(): query_text: {}, lang filter: {}, include_ms_mula: {}, include_cst_mula: {}, include_cst_commentary: {}", &self.query_text, &self.lang, self.include_ms_mula, self.include_cst_mula, self.include_cst_commentary));
        let timer = Instant::now();

        let app_data = get_app_data();
        let db_conn = &mut app_data.dbm.appdata.get_conn()?;

        let like_pattern = format!("%{}%", self.query_text);
        let apply_lang_filter = !self.lang.is_empty() && self.lang != "Language";

        // Build dynamic WHERE clauses for CST/commentary filtering
        let mut extra_where = String::new();

        if !self.include_cst_mula {
            extra_where.push_str(
                " AND NOT (s.uid LIKE '%/cst' AND s.uid NOT LIKE '%.att%/cst' AND s.uid NOT LIKE '%.tik%/cst')"
            );
        }

        if !self.include_cst_commentary {
            // There are %.att/pli/cst and %.att.xml/pli/cst, but .att and .tik commentaries are CST so they always end in /cst
            extra_where.push_str(
                " AND NOT (s.uid LIKE '%.att%/cst' OR s.uid LIKE '%.tik%/cst')"
            );
        }

        if let Some(prefix) = sanitize_uid_like_prefix(self.nikaya_prefix.as_deref()) {
            extra_where.push_str(&format!(" AND f.nikaya LIKE '{}%'", prefix));
        }

        if !self.include_ms_mula {
            extra_where.push_str(" AND NOT (f.source_uid = 'ms')");
        }

        // Push uid_prefix and uid_suffix down as parameter-bound LIKE clauses
        // against suttas.uid (raw, btree-backed for prefix). Unset filters bind
        // the no-op pattern `%`, keeping the bind count constant — diesel's
        // `sql_query` chained-bind types prevent conditional binding.
        let uid_prefix_pat = Self::normalized_filter(&self.uid_prefix)
            .map(|p| format!("{}%", p))
            .unwrap_or_else(|| "%".to_string());
        let uid_suffix_pat = Self::normalized_filter(&self.uid_suffix)
            .map(|s| format!("%{}", s))
            .unwrap_or_else(|| "%".to_string());

        info(&format!("extra_where: {}", extra_where));

        let where_clause = if apply_lang_filter {
            format!(
                "WHERE f.content_plain LIKE ? AND f.language = ?{} AND s.uid LIKE ? AND s.uid LIKE ?",
                extra_where
            )
        } else {
            format!(
                "WHERE f.content_plain LIKE ?{} AND s.uid LIKE ? AND s.uid LIKE ?",
                extra_where
            )
        };

        // --- Cheap COUNT(*) for true total ---
        #[derive(QueryableByName)]
        struct CountRow {
            #[diesel(sql_type = BigInt)]
            c: i64,
        }
        let count_sql = format!(
            "SELECT COUNT(*) AS c FROM suttas_fts f JOIN suttas s ON f.sutta_id = s.id {}",
            where_clause
        );
        let total: i64 = if apply_lang_filter {
            sql_query(&count_sql)
                .bind::<Text, _>(&like_pattern)
                .bind::<Text, _>(&self.lang)
                .bind::<Text, _>(&uid_prefix_pat)
                .bind::<Text, _>(&uid_suffix_pat)
                .get_result::<CountRow>(db_conn)?
                .c
        } else {
            sql_query(&count_sql)
                .bind::<Text, _>(&like_pattern)
                .bind::<Text, _>(&uid_prefix_pat)
                .bind::<Text, _>(&uid_suffix_pat)
                .get_result::<CountRow>(db_conn)?
                .c
        };

        // --- Page fetch ---
        let select_sql = format!(
            "SELECT s.* FROM suttas_fts f JOIN suttas s ON f.sutta_id = s.id {} ORDER BY s.id LIMIT ? OFFSET ?",
            where_clause
        );
        let offset = (page_num as i64).saturating_mul(page_len as i64);
        let db_results: Vec<Sutta> = if apply_lang_filter {
            sql_query(&select_sql)
                .bind::<Text, _>(&like_pattern)
                .bind::<Text, _>(&self.lang)
                .bind::<Text, _>(&uid_prefix_pat)
                .bind::<Text, _>(&uid_suffix_pat)
                .bind::<BigInt, _>(page_len as i64)
                .bind::<BigInt, _>(offset)
                .load(db_conn)?
        } else {
            sql_query(&select_sql)
                .bind::<Text, _>(&like_pattern)
                .bind::<Text, _>(&uid_prefix_pat)
                .bind::<Text, _>(&uid_suffix_pat)
                .bind::<BigInt, _>(page_len as i64)
                .bind::<BigInt, _>(offset)
                .load(db_conn)?
        };

        let search_results: Vec<SearchResult> = db_results
            .iter()
            .map(|sutta| self.db_sutta_to_result(sutta))
            .collect();

        info(&format!("Query took: {:?}", timer.elapsed()));
        Ok((search_results, total as usize))
    }

    /// Per-page contains-match across DpdHeadword + DictWord. Pushes both
    /// `uid_prefix` and `uid_suffix` down to SQL at every phase so the
    /// per-phase fetch_limit_sql cap is spent on rows that survive the
    /// filter. The multi-phase dedup union is materialised, its length is
    /// the true post-filter total, and the requested page is then sliced.
    /// (A single cheap `SELECT COUNT(*)` isn't feasible across the four
    /// phases without restructuring; the union length is authoritative.)
    /// Page-sized variant: builds the full filtered union via
    /// `dict_words_contains_match_fts5_full` and slices for the requested
    /// page. Used by the direct (non-bold) ContainsMatch+Dictionary path.
    fn dict_words_contains_match_fts5(
        &self,
        page_num: usize,
        page_len: usize,
    ) -> Result<(Vec<SearchResult>, usize), Box<dyn Error>> {
        let (full, total) = self.dict_words_contains_match_fts5_full()?;
        let offset = page_num.saturating_mul(page_len);
        let page: Vec<SearchResult> = full.into_iter().skip(offset).take(page_len).collect();
        Ok((page, total))
    }

    /// Multi-phase fallback search used by ContainsMatch + Dictionary.
    ///
    /// Phases (in dedup order):
    ///   1. DpdHeadword exact `lemma_clean` match → resolved to `dict_words` by `word == lemma_1`.
    ///   2. DpdHeadword contains `lemma_1` (via `dpd_headwords_fts`) → same resolution.
    ///   3. **Unified `dict_words_fts` retrieval** covering both indexed columns:
    ///      `(f.word LIKE ? OR f.definition_plain LIKE ?)`. This phase pushes the
    ///      `dict_label IN (...)` inclusion set *into* SQL via a JOIN to `dict_words`
    ///      so the trigram index serves the substring match while the existing btree
    ///      index `dict_words_dict_label_idx` (and the composite `(dict_label, word)`
    ///      index) serves the inclusion-set filter. This is what surfaces
    ///      user-imported dictionaries whose headwords are not DPD lemmas.
    ///   4. ASCII fallback on `dpd_headwords.word_ascii`, used only when phases 1–3
    ///      returned nothing — lets queries like `sutthu` find `suṭṭhu`.
    ///   5. *(intentionally absent)* — what would be a "user-headword substring"
    ///      pass against `dict_words_fts.word LIKE` is fully covered by the unified
    ///      Phase 3 above (`f.word LIKE ?` is one half of the OR). Splitting it out
    ///      would only re-fetch rows that Phase 3 already returns, so it is documented
    ///      and skipped per `tasks-prd-integrate-stardict-filtering.md` task 1.4.
    ///
    /// Each phase pushes `uid_prefix` / `uid_suffix` down to SQL. Cross-phase
    /// deduplication is by `dict_words.id` — switched from `dict_words.word` so
    /// distinct rows that happen to share a headword (common across user-imported
    /// dictionaries) are not collapsed.
    ///
    /// Pagination contract: returns the dedup-union as `Vec<SearchResult>` along
    /// with `total = full.len()`; callers slice it to produce a page (`total`
    /// remains exact under materialise-then-slice). The unified Phase 3 pushes
    /// `dict_label IN (...)` into SQL; Phases 1, 2, and 4 push the same
    /// inclusion set into their per-headword `dict_words` lookups (otherwise
    /// they over-retrieve rows whose `dict_label` is excluded — e.g. DPD's own
    /// rows when DPD is disabled — leaving the post-filter to empty out early
    /// pages while `total` reflects the unfiltered union). With every phase
    /// pushing the inclusion set, the dispatcher's
    /// `apply_dict_source_uids_filter` post-filter is a safety net that should
    /// drop zero rows in normal operation.
    fn dict_words_contains_match_fts5_full(
        &self,
    ) -> Result<(Vec<SearchResult>, usize), Box<dyn Error>> {
        info(&format!("dict_words_contains_match_fts5_full(): query_text: {}", &self.query_text));
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
        // Cross-phase deduplication is by `dict_words.id` so distinct rows
        // sharing a headword (common across user-imported dictionaries) are
        // not collapsed. (Previously dedup keyed on `dict_words.word`.)
        let mut result_ids: HashSet<i32> = HashSet::new();

        // Push uid_prefix and uid_suffix down to SQL at every phase. `'%'`
        // is the no-op pattern when the filter is unset, keeping the bind
        // count constant.
        let uid_prefix_pat = Self::normalized_filter(&self.uid_prefix)
            .map(|p| format!("{}%", p))
            .unwrap_or_else(|| "%".to_string());
        let uid_suffix_pat = Self::normalized_filter(&self.uid_suffix)
            .map(|s| format!("%{}", s))
            .unwrap_or_else(|| "%".to_string());

        // Inclusion-set push-down for the DPD-driven phases (1, 2, 4).
        // These phases resolve a DPD lemma to a `dict_words` row by
        // `word == lemma_1`, which can match rows in *any* dictionary.
        // Without this push-down they over-retrieve rows whose `dict_label`
        // is not in the inclusion set (e.g. DPD's own rows when the user
        // disables DPD), inflating `total` and leaving the post-filter to
        // empty out early pages — observable as "Page 1 of 53" with an
        // empty page 1. Pushing `dict_label IN (set)` here keeps `total`
        // consistent with the visible page contents (PRD §2.6).
        //
        // dict_source_uids contract:
        //   - None             → no constraint, every dictionary contributes.
        //   - Some(non-empty)  → restrict to that set.
        //   - Some(empty)      → skip these phases entirely (would return zero anyway).
        let skip_dpd_driven_phases =
            matches!(self.dict_source_uids.as_deref(), Some(s) if s.is_empty());
        let dpd_driven_inclusion: Option<Vec<String>> = match self.dict_source_uids.as_deref() {
            Some(set) if !set.is_empty() => Some(set.to_vec()),
            _ => None,
        };

        // Phase 1: Exact matches on DpdHeadword.lemma_clean
        // dpd.lemma_clean has btree index and dpd.lemma_1 has unique constraint and so implicitly indexed.
        if !skip_dpd_driven_phases {
            for term in &terms {
                let exact_matches: Vec<DpdHeadword> = dpd_dsl::dpd_headwords
                    .filter(dpd_dsl::lemma_clean.eq(term))
                    .order(dpd_dsl::id)
                    .limit(SAFETY_LIMIT_SQL)
                    .load::<DpdHeadword>(dpd_conn)?;

                // Convert DpdHeadword results to DictWord using their UIDs
                for headword in exact_matches {
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

                    if let Some(ref set) = dpd_driven_inclusion {
                        dict_query = dict_query.filter(dict_dsl::dict_label.eq_any(set.clone()));
                    }

                    // Match DictWord.word with DpdHeadword.lemma_1
                    let dict_word_result: Result<DictWord, _> = dict_query
                        .filter(dict_dsl::word.eq(&headword.lemma_1))
                        .filter(dict_dsl::uid.like(&uid_prefix_pat))
                        .filter(dict_dsl::uid.like(&uid_suffix_pat))
                        .first::<DictWord>(db_conn);

                    if let Ok(dict_word) = dict_word_result
                        && result_ids.insert(dict_word.id) {
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
        if !skip_dpd_driven_phases {
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

                    if let Some(ref set) = dpd_driven_inclusion {
                        dict_query = dict_query.filter(dict_dsl::dict_label.eq_any(set.clone()));
                    }

                    // Match DictWord.word with DpdHeadword.lemma_1
                    let dict_word_result: Result<DictWord, _> = dict_query
                        .filter(dict_dsl::word.eq(&headword.lemma_1))
                        .filter(dict_dsl::uid.like(&uid_prefix_pat))
                        .filter(dict_dsl::uid.like(&uid_suffix_pat))
                        .first::<DictWord>(db_conn);

                    if let Ok(dict_word) = dict_word_result
                        && result_ids.insert(dict_word.id) {
                            all_results.push(dict_word);
                        }
                }
            }
        }

        // Phase 3: Unified `dict_words_fts`-driven retrieval covering both
        // indexed columns — `f.word LIKE ? OR f.definition_plain LIKE ?`.
        // Pushes the `dict_label IN (...)` inclusion set into SQL via the
        // JOIN to `dict_words`, so the trigram index serves the substring
        // match while `dict_words_dict_label_idx` (and the composite
        // `(dict_label, word)` index) serves the inclusion-set filter.
        // Surfaces user-imported dictionary entries whose `word` is not a
        // DPD lemma — what was previously invisible to phases 1, 2, 4.
        //
        // dict_source_uids contract:
        //   - None             → no inclusion-set constraint, search every dict.
        //   - Some(non-empty)  → push `dict_label IN (...)` into SQL.
        //   - Some(empty)      → skip Phase 3 entirely (inclusion set would drop everything).
        let skip_phase3 = matches!(self.dict_source_uids.as_deref(), Some(s) if s.is_empty());
        let phase3_in_clause: Option<(String, Vec<String>)> = match self.dict_source_uids.as_deref() {
            Some(set) => Self::dict_label_in_clause(set),
            None => None,
        };

        if !skip_phase3 {
            for term in &terms {
                let like_pattern = format!("%{}%", term);

                let mut sql = String::from(
                    "SELECT d.* FROM dict_words d \
                     JOIN dict_words_fts f ON f.dict_word_id = d.id \
                     WHERE (f.word LIKE ? OR f.definition_plain LIKE ?)"
                );

                if self.source.is_some() {
                    if self.source_include {
                        sql.push_str(" AND d.dict_label = ?");
                    } else {
                        sql.push_str(" AND d.dict_label != ?");
                    }
                }

                if let Some((ph, _)) = &phase3_in_clause {
                    sql.push_str(" AND d.dict_label IN (");
                    sql.push_str(ph);
                    sql.push(')');
                }

                sql.push_str(" AND d.uid LIKE ? AND d.uid LIKE ? ORDER BY d.id LIMIT ?");

                let mut q = sql_query(&sql)
                    .into_boxed::<diesel::sqlite::Sqlite>()
                    .bind::<Text, _>(like_pattern.clone())
                    .bind::<Text, _>(like_pattern.clone());

                if let Some(ref source_val) = self.source {
                    q = q.bind::<Text, _>(source_val.clone());
                }

                if let Some((_, binds)) = &phase3_in_clause {
                    for v in binds {
                        q = q.bind::<Text, _>(v.clone());
                    }
                }

                q = q
                    .bind::<Text, _>(uid_prefix_pat.clone())
                    .bind::<Text, _>(uid_suffix_pat.clone())
                    .bind::<BigInt, _>(SAFETY_LIMIT_SQL);

                let def_results: Vec<DictWord> = q.load(db_conn)?;

                for result in def_results {
                    if result_ids.insert(result.id) {
                        all_results.push(result);
                    }
                }
            }
        }

        // Phase 5 (intentionally absent): a dedicated user-headword
        // substring pass against `dict_words_fts.word LIKE` would re-fetch
        // exactly the rows the unified Phase 3 already returns, since
        // `f.word LIKE ?` is one half of Phase 3's OR. Documented in
        // tasks-prd-integrate-stardict-filtering.md task 1.4 and skipped.

        // Phase 4: Fallback to word_ascii matching if no results found
        // This allows queries like 'sutthu' to find 'suṭṭhu'
        if all_results.is_empty() && !skip_dpd_driven_phases {
            for term in &terms {
                // Try exact match on word_ascii
                let ascii_matches: Vec<DpdHeadword> = dpd_dsl::dpd_headwords
                    .filter(dpd_dsl::word_ascii.eq(term))
                    .order(dpd_dsl::id)
                    .limit(SAFETY_LIMIT_SQL)
                    .load::<DpdHeadword>(dpd_conn)?;

                for headword in ascii_matches {
                    let mut dict_query = dict_dsl::dict_words.into_boxed();

                    if let Some(ref source_val) = self.source {
                        if self.source_include {
                            dict_query = dict_query.filter(dict_dsl::dict_label.eq(source_val));
                        } else {
                            dict_query = dict_query.filter(dict_dsl::dict_label.ne(source_val));
                        }
                    }

                    if let Some(ref set) = dpd_driven_inclusion {
                        dict_query = dict_query.filter(dict_dsl::dict_label.eq_any(set.clone()));
                    }

                    let dict_word_result: Result<DictWord, _> = dict_query
                        .filter(dict_dsl::word.eq(&headword.lemma_1))
                        .filter(dict_dsl::uid.like(&uid_prefix_pat))
                        .filter(dict_dsl::uid.like(&uid_suffix_pat))
                        .first::<DictWord>(db_conn);

                    if let Ok(dict_word) = dict_word_result
                        && result_ids.insert(dict_word.id) {
                            all_results.push(dict_word);
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
                        let mut dict_query = dict_dsl::dict_words.into_boxed();

                        if let Some(ref source_val) = self.source {
                            if self.source_include {
                                dict_query = dict_query.filter(dict_dsl::dict_label.eq(source_val));
                            } else {
                                dict_query = dict_query.filter(dict_dsl::dict_label.ne(source_val));
                            }
                        }

                        if let Some(ref set) = dpd_driven_inclusion {
                            dict_query = dict_query.filter(dict_dsl::dict_label.eq_any(set.clone()));
                        }

                        let dict_word_result: Result<DictWord, _> = dict_query
                            .filter(dict_dsl::word.eq(&headword.lemma_1))
                            .filter(dict_dsl::uid.like(&uid_prefix_pat))
                            .filter(dict_dsl::uid.like(&uid_suffix_pat))
                            .first::<DictWord>(db_conn);

                        if let Ok(dict_word) = dict_word_result
                            && result_ids.insert(dict_word.id) {
                                all_results.push(dict_word);
                            }
                    }
                }
            }
        }

        if (all_results.len() as i64) >= SAFETY_LIMIT_SQL {
            warn(&format!(
                "dict_words_contains_match_fts5 hit SAFETY_LIMIT_SQL={} (query='{}')",
                SAFETY_LIMIT_SQL, &self.query_text
            ));
        }

        // The dedup-union length is the true post-filter total. Under-counts
        // only when a per-phase safety cap is hit on very broad queries.
        let total = all_results.len();

        let search_results: Vec<SearchResult> = all_results
            .iter()
            .map(|dict_word| self.db_word_to_result(dict_word))
            .collect();

        info(&format!("Query took: {:?}", timer.elapsed()));
        Ok((search_results, total))
    }

    /// Per-page contains-match against book_spine_items_fts. Pushes uid_prefix
    /// and uid_suffix down as `book_spine_items.spine_item_uid LIKE ?` clauses,
    /// and runs a parallel COUNT(*) over the same predicate so the caller has
    /// the true post-filter hit count without materializing every row.
    fn book_spine_items_contains_match_fts5(
        &self,
        page_num: usize,
        page_len: usize,
    ) -> Result<(Vec<SearchResult>, usize), Box<dyn Error>> {
        info(&format!("book_spine_items_contains_match_fts5(): query_text: {}, lang filter: {}", &self.query_text, &self.lang));
        let timer = Instant::now();

        let app_data = get_app_data();
        let db_conn = &mut app_data.dbm.appdata.get_conn()?;

        let like_pattern = format!("%{}%", self.query_text);

        // Determine if we need language filtering
        let apply_lang_filter = !self.lang.is_empty() && self.lang != "Language";

        // Push uid_prefix and uid_suffix down to SQL so the LIMIT is spent on
        // rows that survive the filter. Default '%' matches anything when
        // unset, keeping the bind count constant.
        let uid_prefix_pat = Self::normalized_filter(&self.uid_prefix)
            .map(|p| format!("{}%", p))
            .unwrap_or_else(|| "%".to_string());
        let uid_suffix_pat = Self::normalized_filter(&self.uid_suffix)
            .map(|s| format!("%{}", s))
            .unwrap_or_else(|| "%".to_string());

        // --- Cheap COUNT(*) for true total ---
        #[derive(QueryableByName)]
        struct CountRow {
            #[diesel(sql_type = BigInt)]
            c: i64,
        }
        let total: i64 = if apply_lang_filter {
            sql_query(
                r#"
                SELECT COUNT(*) AS c
                FROM book_spine_items_fts f
                JOIN book_spine_items b ON f.spine_item_id = b.id
                WHERE f.content_plain LIKE ? AND f.language = ? AND b.spine_item_uid LIKE ? AND b.spine_item_uid LIKE ?
                "#
            )
            .bind::<Text, _>(&like_pattern)
            .bind::<Text, _>(&self.lang)
            .bind::<Text, _>(&uid_prefix_pat)
            .bind::<Text, _>(&uid_suffix_pat)
            .get_result::<CountRow>(db_conn)?
            .c
        } else {
            sql_query(
                r#"
                SELECT COUNT(*) AS c
                FROM book_spine_items_fts f
                JOIN book_spine_items b ON f.spine_item_id = b.id
                WHERE f.content_plain LIKE ? AND b.spine_item_uid LIKE ? AND b.spine_item_uid LIKE ?
                "#
            )
            .bind::<Text, _>(&like_pattern)
            .bind::<Text, _>(&uid_prefix_pat)
            .bind::<Text, _>(&uid_suffix_pat)
            .get_result::<CountRow>(db_conn)?
            .c
        };

        // --- Page fetch ---
        let offset = (page_num as i64).saturating_mul(page_len as i64);
        let db_results: Vec<BookSpineItem> = if apply_lang_filter {
            sql_query(
                r#"
                SELECT b.*
                FROM book_spine_items_fts f
                JOIN book_spine_items b ON f.spine_item_id = b.id
                WHERE f.content_plain LIKE ? AND f.language = ? AND b.spine_item_uid LIKE ? AND b.spine_item_uid LIKE ?
                ORDER BY b.id
                LIMIT ? OFFSET ?
                "#
            )
            .bind::<Text, _>(&like_pattern)
            .bind::<Text, _>(&self.lang)
            .bind::<Text, _>(&uid_prefix_pat)
            .bind::<Text, _>(&uid_suffix_pat)
            .bind::<BigInt, _>(page_len as i64)
            .bind::<BigInt, _>(offset)
            .load(db_conn)?
        } else {
            sql_query(
                r#"
                SELECT b.*
                FROM book_spine_items_fts f
                JOIN book_spine_items b ON f.spine_item_id = b.id
                WHERE f.content_plain LIKE ? AND b.spine_item_uid LIKE ? AND b.spine_item_uid LIKE ?
                ORDER BY b.id
                LIMIT ? OFFSET ?
                "#
            )
            .bind::<Text, _>(&like_pattern)
            .bind::<Text, _>(&uid_prefix_pat)
            .bind::<Text, _>(&uid_suffix_pat)
            .bind::<BigInt, _>(page_len as i64)
            .bind::<BigInt, _>(offset)
            .load(db_conn)?
        };

        let search_results: Vec<SearchResult> = db_results
            .iter()
            .map(|spine_item| self.db_book_spine_item_to_result(spine_item))
            .collect();

        info(&format!("Query took: {:?}", timer.elapsed()));
        Ok((search_results, total as usize))
    }

    /// Substring match on bold_definitions.bold using the trigram FTS5 index.
    /// Used by DPD Lookup and Headword Match.
    /// Substring match on `bold_definitions.bold` / `bold_ascii` via the
    /// FTS5 trigram index. Returns `(total_count, slice)` where the slice
    /// covers `[offset .. offset+limit)` of the deterministic
    /// `ORDER BY bd.id` stream. `limit = 0` runs only the COUNT and skips
    /// the row fetch — useful when the caller only needs the boundary
    /// total to compute its slice ranges.
    fn query_bold_definitions_bold_fts5(
        &self,
        query: &str,
        offset: usize,
        limit: usize,
    ) -> Result<(i64, Vec<SearchResult>), Box<dyn Error>> {
        use crate::db::dpd_models::BoldDefinition;
        use diesel::sql_types::BigInt;

        let q = query.trim().to_lowercase();
        if q.is_empty() {
            return Ok((0, Vec::new()));
        }

        let app_data = get_app_data();
        let dpd_conn = &mut app_data.dbm.dpd.get_conn()?;

        let like_pattern = format!("%{}%", q);
        // Match against both the original `bold` (e.g. "suṭṭhu") and the
        // ASCII-folded `bold_ascii` (e.g. "sutthu") so ASCII queries find
        // diacritic entries — mirrors the word_ascii lookup path.
        //
        // Push uid_prefix / uid_suffix down to SQL. Default `%` (match
        // anything) when unset keeps the bind count constant.
        let uid_prefix_pat = Self::normalized_filter(&self.uid_prefix)
            .map(|p| format!("{}%", p))
            .unwrap_or_else(|| "%".to_string());
        let uid_suffix_pat = Self::normalized_filter(&self.uid_suffix)
            .map(|s| format!("%{}", s))
            .unwrap_or_else(|| "%".to_string());

        // Cheap COUNT(*) to get true total before LIMIT-bounded fetch.
        #[derive(QueryableByName)]
        struct CountRow {
            #[diesel(sql_type = BigInt)]
            c: i64,
        }
        let count_sql = r#"
            SELECT COUNT(*) AS c
            FROM bold_definitions_bold_fts f
            JOIN bold_definitions bd ON bd.id = f.bold_definitions_id
            WHERE (f.bold LIKE ? OR f.bold_ascii LIKE ?)
              AND bd.uid LIKE ?
              AND bd.uid LIKE ?
        "#;
        let total: i64 = sql_query(count_sql)
            .bind::<Text, _>(&like_pattern)
            .bind::<Text, _>(&like_pattern)
            .bind::<Text, _>(&uid_prefix_pat)
            .bind::<Text, _>(&uid_suffix_pat)
            .get_result::<CountRow>(dpd_conn)?
            .c;

        if limit == 0 {
            return Ok((total, Vec::new()));
        }

        let sql = r#"
            SELECT bd.*
            FROM bold_definitions_bold_fts f
            JOIN bold_definitions bd ON bd.id = f.bold_definitions_id
            WHERE (f.bold LIKE ? OR f.bold_ascii LIKE ?)
              AND bd.uid LIKE ?
              AND bd.uid LIKE ?
            ORDER BY bd.id
            LIMIT ? OFFSET ?
        "#;

        let bds: Vec<BoldDefinition> = sql_query(sql)
            .bind::<Text, _>(&like_pattern)
            .bind::<Text, _>(&like_pattern)
            .bind::<Text, _>(&uid_prefix_pat)
            .bind::<Text, _>(&uid_suffix_pat)
            .bind::<BigInt, _>(limit as i64)
            .bind::<BigInt, _>(offset as i64)
            .load(dpd_conn)?;

        if (bds.len() as i64) >= SAFETY_LIMIT_SQL {
            warn(&format!(
                "query_bold_definitions_bold_fts5 hit SAFETY_LIMIT_SQL={} (query='{}')",
                SAFETY_LIMIT_SQL, q
            ));
        }

        let results: Vec<SearchResult> = bds.iter().map(bold_definition_to_search_result).collect();
        Ok((total, results))
    }

    /// Substring match on `bold_definitions.commentary_plain` via the
    /// trigram FTS5 index. Used by Contains Match + Dictionary. Returns
    /// `(total_count, slice)` where the slice covers `[offset .. offset+limit)`
    /// of the deterministic `ORDER BY bd.id` stream. `limit = 0` runs
    /// only the COUNT.
    fn query_bold_definitions_commentary_fts5(
        &self,
        normalized_query: &str,
        offset: usize,
        limit: usize,
    ) -> Result<(i64, Vec<SearchResult>), Box<dyn Error>> {
        use crate::db::dpd_models::BoldDefinition;
        use diesel::sql_types::BigInt;

        let q = normalized_query.trim();
        if q.is_empty() {
            return Ok((0, Vec::new()));
        }

        let app_data = get_app_data();
        let dpd_conn = &mut app_data.dbm.dpd.get_conn()?;

        let like_pattern = format!("%{}%", q);
        // Push uid_prefix / uid_suffix down to SQL. Default patterns are `%`.
        let uid_prefix_pat = Self::normalized_filter(&self.uid_prefix)
            .map(|p| format!("{}%", p))
            .unwrap_or_else(|| "%".to_string());
        let uid_suffix_pat = Self::normalized_filter(&self.uid_suffix)
            .map(|s| format!("%{}", s))
            .unwrap_or_else(|| "%".to_string());

        // Cheap COUNT(*) to get the true total before LIMIT-bounded fetch.
        #[derive(QueryableByName)]
        struct CountRow {
            #[diesel(sql_type = BigInt)]
            c: i64,
        }
        let count_sql = r#"
            SELECT COUNT(*) AS c
            FROM bold_definitions_fts f
            JOIN bold_definitions bd ON bd.id = f.bold_definitions_id
            WHERE f.commentary_plain LIKE ?
              AND bd.uid LIKE ?
              AND bd.uid LIKE ?
        "#;
        let total: i64 = sql_query(count_sql)
            .bind::<Text, _>(&like_pattern)
            .bind::<Text, _>(&uid_prefix_pat)
            .bind::<Text, _>(&uid_suffix_pat)
            .get_result::<CountRow>(dpd_conn)?
            .c;

        if limit == 0 {
            return Ok((total, Vec::new()));
        }

        let sql = r#"
            SELECT bd.*
            FROM bold_definitions_fts f
            JOIN bold_definitions bd ON bd.id = f.bold_definitions_id
            WHERE f.commentary_plain LIKE ?
              AND bd.uid LIKE ?
              AND bd.uid LIKE ?
            ORDER BY bd.id
            LIMIT ? OFFSET ?
        "#;

        let bds: Vec<BoldDefinition> = sql_query(sql)
            .bind::<Text, _>(&like_pattern)
            .bind::<Text, _>(&uid_prefix_pat)
            .bind::<Text, _>(&uid_suffix_pat)
            .bind::<BigInt, _>(limit as i64)
            .bind::<BigInt, _>(offset as i64)
            .load(dpd_conn)?;

        if (bds.len() as i64) >= SAFETY_LIMIT_SQL {
            warn(&format!(
                "query_bold_definitions_commentary_fts5 hit SAFETY_LIMIT_SQL={} (query='{}')",
                SAFETY_LIMIT_SQL, q
            ));
        }

        let results: Vec<SearchResult> = bds.iter().map(bold_definition_to_search_result).collect();
        Ok((total, results))
    }

    /// Fetch the full filtered DPD result set. uid_prefix / uid_suffix are
    /// pushed down into every per-phase SQL query against `dpd_headwords` /
    /// `dpd_roots` so the storage layer never returns rows that can't appear
    /// in the final result. The multi-phase fallback structure of `dpd_lookup`
    /// (exact match → roots → inflections → stem → compound → deconstructor →
    /// prefix) means we can't issue a single paginated SQL query, so the full
    /// filtered union is materialised in memory and the caller slices.
    fn dpd_lookup_full(&self) -> Result<Vec<SearchResult>, Box<dyn Error>> {
        let app_data = get_app_data();
        let results = app_data.dbm.dpd.dpd_lookup(
            &self.query_text,
            false,
            true,
            self.uid_prefix.as_deref(),
            self.uid_suffix.as_deref(),
        )?;
        Ok(results)
    }

    /// Per-page DPD Lookup with SQL-side uid filtering. DPD is only
    /// Pāli-to-English so language filters are ignored.
    pub fn dpd_lookup(
        &self,
        page_num: usize,
        page_len: usize,
    ) -> Result<(Vec<SearchResult>, usize), Box<dyn Error>> {
        let results = self.dpd_lookup_full()?;
        let total = results.len();
        let offset = page_num.saturating_mul(page_len);
        let page: Vec<SearchResult> = results.into_iter().skip(offset).take(page_len).collect();
        Ok((page, total))
    }

    /// Per-page suttas title-match. Pushes uid_prefix and uid_suffix down to
    /// `suttas.uid LIKE ?`, plus the existing CST / nikaya / MS-mūla filters.
    /// A parallel `count()` over the same boxed predicate yields the true
    /// total without materialising every row.
    fn suttas_title_match(
        &self,
        page_num: usize,
        page_len: usize,
    ) -> Result<(Vec<SearchResult>, usize), Box<dyn Error>> {
        info(&format!("suttas_title_match(): query_text: {}, lang filter: {}", &self.query_text, &self.lang));
        let timer = Instant::now();

        use crate::db::appdata_schema::suttas::dsl::*;
        use diesel::dsl::count_star;

        let app_data = get_app_data();
        let db_conn = &mut app_data.dbm.appdata.get_conn()?;

        let like_pattern = format!("%{}%", self.query_text);

        // Build the predicate twice (once for COUNT, once for the page fetch);
        // diesel's boxed queries are not cloneable, and the predicate is not
        // expensive to construct.
        let build_query = || {
            let mut query = suttas.into_boxed();
            query = query.filter(
                title.like(&like_pattern)
                .or(title_ascii.like(&like_pattern))
            );

            if !self.lang.is_empty() && self.lang != "Language" {
                query = query.filter(language.eq(&self.lang));
            }

            if !self.include_cst_mula {
                query = query.filter(
                    diesel::dsl::not(
                        uid.like("%/cst")
                            .and(uid.not_like("%.att%/cst"))
                            .and(uid.not_like("%.tik%/cst"))
                    )
                );
            }

            if !self.include_cst_commentary {
                query = query.filter(
                    diesel::dsl::not(
                        uid.like("%.att%/cst")
                        .or(uid.like("%.tik%/cst"))
                    )
                );
            }

            if let Some(prefix) = sanitize_uid_like_prefix(self.nikaya_prefix.as_deref()) {
                query = query.filter(nikaya.like(format!("{}%", prefix)));
            }

            if let Some(prefix) = Self::normalized_filter(&self.uid_prefix) {
                query = query.filter(uid.like(format!("{}%", prefix)));
            }
            if let Some(suffix) = Self::normalized_filter(&self.uid_suffix) {
                query = query.filter(uid.like(format!("%{}", suffix)));
            }

            if !self.include_ms_mula {
                query = query.filter(diesel::dsl::not(source_uid.eq("ms")));
            }

            query
        };

        let total: i64 = build_query()
            .select(count_star())
            .first(db_conn)?;

        let offset = (page_num as i64).saturating_mul(page_len as i64);
        let db_results: Vec<Sutta> = build_query()
            .order(uid.asc())
            .limit(page_len as i64)
            .offset(offset)
            .select(Sutta::as_select())
            .load(db_conn)?;

        let search_results: Vec<SearchResult> = db_results
            .iter()
            .map(|sutta| self.db_sutta_to_result(sutta))
            .collect();

        info(&format!("Query took: {:?}", timer.elapsed()));
        Ok((search_results, total as usize))
    }

    /// Per-page library title-match: union of books and spine-item title hits.
    /// uid_prefix and uid_suffix are pushed down on each branch
    /// (`books.uid` / `book_spine_items.spine_item_uid`); the materialised
    /// union length is the authoritative total and the requested page is
    /// then sliced.
    fn library_title_match(
        &self,
        page_num: usize,
        page_len: usize,
    ) -> Result<(Vec<SearchResult>, usize), Box<dyn Error>> {
        info(&format!("library_title_match(): query_text: {}, lang filter: {}", &self.query_text, &self.lang));
        let timer = Instant::now();

        use crate::db::appdata_schema::books::dsl as books_dsl;
        use crate::db::appdata_schema::book_spine_items::dsl as spine_dsl;

        let app_data = get_app_data();
        let db_conn = &mut app_data.dbm.appdata.get_conn()?;

        let like_pattern = format!("%{}%", self.query_text);
        let apply_lang_filter = !self.lang.is_empty() && self.lang != "Language";

        // Push uid_prefix / uid_suffix down on `books.uid` for the books
        // branch and `b.spine_item_uid` for the spine-items branch. `'%'` is
        // the no-op pattern when unset.
        let uid_prefix_pat = Self::normalized_filter(&self.uid_prefix)
            .map(|p| format!("{}%", p))
            .unwrap_or_else(|| "%".to_string());
        let uid_suffix_pat = Self::normalized_filter(&self.uid_suffix)
            .map(|s| format!("%{}", s))
            .unwrap_or_else(|| "%".to_string());

        let mut all_results: Vec<SearchResult> = Vec::new();

        // Books branch.
        let mut books_query = books_dsl::books.into_boxed();
        books_query = books_query.filter(books_dsl::title.like(&like_pattern));
        books_query = books_query
            .filter(books_dsl::uid.like(&uid_prefix_pat))
            .filter(books_dsl::uid.like(&uid_suffix_pat));

        if apply_lang_filter {
            books_query = books_query.filter(books_dsl::language.eq(&self.lang));
        }

        let book_uids: Vec<String> = books_query
            .order(books_dsl::id.asc())
            .limit(SAFETY_LIMIT_SQL)
            .select(books_dsl::uid)
            .load(db_conn)?;

        for book_uid in book_uids {
            let first_spine_item: Result<BookSpineItem, _> = spine_dsl::book_spine_items
                .filter(spine_dsl::book_uid.eq(&book_uid))
                .order(spine_dsl::spine_index.asc())
                .first::<BookSpineItem>(db_conn);

            if let Ok(spine_item) = first_spine_item {
                all_results.push(self.db_book_spine_item_to_result(&spine_item));
            }
        }

        // Spine items branch.
        let spine_results: Vec<BookSpineItem> = if apply_lang_filter {
            sql_query(
                r#"
                SELECT b.*
                FROM book_spine_items_fts f
                JOIN book_spine_items b ON f.spine_item_id = b.id
                WHERE f.title LIKE ? AND f.language = ? AND b.spine_item_uid LIKE ? AND b.spine_item_uid LIKE ?
                ORDER BY b.id
                LIMIT ?
                "#
            )
            .bind::<Text, _>(&like_pattern)
            .bind::<Text, _>(&self.lang)
            .bind::<Text, _>(&uid_prefix_pat)
            .bind::<Text, _>(&uid_suffix_pat)
            .bind::<BigInt, _>(SAFETY_LIMIT_SQL)
            .load(db_conn)?
        } else {
            sql_query(
                r#"
                SELECT b.*
                FROM book_spine_items_fts f
                JOIN book_spine_items b ON f.spine_item_id = b.id
                WHERE f.title LIKE ? AND b.spine_item_uid LIKE ? AND b.spine_item_uid LIKE ?
                ORDER BY b.id
                LIMIT ?
                "#
            )
            .bind::<Text, _>(&like_pattern)
            .bind::<Text, _>(&uid_prefix_pat)
            .bind::<Text, _>(&uid_suffix_pat)
            .bind::<BigInt, _>(SAFETY_LIMIT_SQL)
            .load(db_conn)?
        };

        for spine_item in spine_results {
            all_results.push(self.db_book_spine_item_to_result(&spine_item));
        }

        let total = all_results.len();
        let offset = page_num.saturating_mul(page_len);
        let page: Vec<SearchResult> = all_results.into_iter().skip(offset).take(page_len).collect();

        info(&format!("Query took: {:?}", timer.elapsed()));
        Ok((page, total))
    }

    /// Per-page Headword Match against `dpd_headwords_fts.lemma_1`. Pushes
    /// uid_prefix and uid_suffix down to the per-headword DictWord lookup.
    /// The materialised SearchResult union length is the authoritative total
    /// (the raw FTS lemma_1 count would overstate by including headwords
    /// that don't resolve to a DictWord); the requested page is then sliced.
    /// Page-sized variant: builds the full filtered list via
    /// `lemma_1_dpd_headword_match_fts5_full` and slices for the requested page.
    fn lemma_1_dpd_headword_match_fts5(
        &self,
        page_num: usize,
        page_len: usize,
    ) -> Result<(Vec<SearchResult>, usize), Box<dyn Error>> {
        let (full, total) = self.lemma_1_dpd_headword_match_fts5_full()?;
        let offset = page_num.saturating_mul(page_len);
        let page: Vec<SearchResult> = full.into_iter().skip(offset).take(page_len).collect();
        Ok((page, total))
    }

    /// Headword match — merges two retrieval paths so user-imported StarDict
    /// dictionaries participate alongside DPD's `lemma_1` index:
    ///
    /// - **Path A (DPD):** FTS5 against `dpd_headwords_fts.lemma_1`, then
    ///   resolve each matched DPD headword to its `dict_words` row by
    ///   `word == lemma_1` constrained to `dict_label = "dpd"`. Enabled when
    ///   `dict_source_uids` is `None` (no constraint) or `Some(set)` with
    ///   `"dpd"` in `set`. Skipped when the user solos a non-DPD dictionary.
    ///
    /// - **Path B (user-headword):** SQL against `dict_words` JOINed to
    ///   `dict_words_fts` with `f.word LIKE ?` and `dw.dict_label IN
    ///   (non_dpd_set)`. Enabled for the non-DPD subset of the inclusion
    ///   set; when `dict_source_uids` is `None`, Path B searches every
    ///   non-DPD dictionary by issuing the same query without the `IN`
    ///   clause. The trigram index on `dict_words_fts.word` (added in PRD
    ///   §5.0) serves the substring match; `dict_words_dict_label_idx`
    ///   serves the inclusion-set filter via the JOIN.
    ///
    /// Merge: dedup by `dict_words.id`. Sort: exact `word == query_text`
    /// rows first, then contains rows; tie-break by `dict_label`, then `id`
    /// for stable ordering. The materialised union length is the
    /// authoritative `total` (materialise-then-slice; PRD §2.5 contract
    /// preserved).
    ///
    /// `dict_source_uids` contract:
    ///   - None             → Path A enabled, Path B over every non-DPD dict.
    ///   - Some(non-empty)  → Path A iff `"dpd" ∈ set`; Path B over `set \ {"dpd"}`.
    ///   - Some(empty)      → both paths skipped, returns `(empty, 0)`.
    fn lemma_1_dpd_headword_match_fts5_full(
        &self,
    ) -> Result<(Vec<SearchResult>, usize), Box<dyn Error>> {
        info(&format!("lemma_1_dpd_headword_match_fts5_full(): query_text: {}", &self.query_text));
        let timer = Instant::now();

        let app_data = get_app_data();
        let dpd_conn = &mut app_data.dbm.dpd.get_conn()?;
        let dict_conn = &mut app_data.dbm.dictionaries.get_conn()?;

        use crate::db::dpd_models::DpdHeadword;
        use crate::db::dpd_schema::dpd_headwords::dsl as dpd_dsl;
        use crate::db::dictionaries_schema::dict_words::dsl as dict_dsl;

        let like_pattern = format!("%{}%", self.query_text);

        // Decide which paths run based on the inclusion-set contract.
        let (path_a_enabled, non_dpd_subset): (bool, Option<Vec<String>>) =
            match self.dict_source_uids.as_deref() {
                None => (true, None),
                Some(set) if set.is_empty() => {
                    return Ok((Vec::new(), 0));
                }
                Some(set) => {
                    let has_dpd = set.iter().any(|s| s == "dpd");
                    let non_dpd: Vec<String> =
                        set.iter().filter(|s| *s != "dpd").cloned().collect();
                    let subset = if non_dpd.is_empty() { None } else { Some(non_dpd) };
                    (has_dpd, subset)
                }
            };
        // `non_dpd_subset` semantics:
        //   - Outer `None`  → Path B searches every non-DPD dict (no IN clause).
        //   - `Some(vec)`   → Path B restricts to that subset.
        //   - When `dict_source_uids` is `Some(set)` with no non-DPD entries,
        //     Path B is skipped entirely.
        let path_b_enabled = matches!(self.dict_source_uids.as_deref(), None)
            || non_dpd_subset.is_some();

        // Push uid_prefix and uid_suffix down to the DictWord lookup. `'%'`
        // is the no-op pattern when the filter is unset.
        let uid_prefix_pat = Self::normalized_filter(&self.uid_prefix)
            .map(|p| format!("{}%", p))
            .unwrap_or_else(|| "%".to_string());
        let uid_suffix_pat = Self::normalized_filter(&self.uid_suffix)
            .map(|s| format!("%{}", s))
            .unwrap_or_else(|| "%".to_string());

        let mut all_rows: Vec<DictWord> = Vec::new();
        let mut seen_ids: HashSet<i32> = HashSet::new();

        // ---------- Path A: DPD lemma_1 → dict_words(dict_label="dpd") ----------
        if path_a_enabled {
            #[derive(QueryableByName)]
            struct HeadwordId {
                #[diesel(sql_type = diesel::sql_types::Integer)]
                headword_id: i32,
            }

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

            if (headword_ids.len() as i64) >= SAFETY_LIMIT_SQL {
                warn(&format!(
                    "lemma_1_dpd_headword_match_fts5 Path A hit SAFETY_LIMIT_SQL={} (query='{}')",
                    SAFETY_LIMIT_SQL, &self.query_text
                ));
            }

            let ids: Vec<i32> = headword_ids.iter().map(|h| h.headword_id).collect();

            if !ids.is_empty() {
                let headwords: Vec<DpdHeadword> = dpd_dsl::dpd_headwords
                    .filter(dpd_dsl::id.eq_any(&ids))
                    .load::<DpdHeadword>(dpd_conn)?;

                for headword in headwords {
                    let mut dict_query = dict_dsl::dict_words.into_boxed();

                    if let Some(ref source_val) = self.source {
                        if self.source_include {
                            dict_query = dict_query.filter(dict_dsl::dict_label.eq(source_val));
                        } else {
                            dict_query = dict_query.filter(dict_dsl::dict_label.ne(source_val));
                        }
                    }

                    // Path A is the DPD path; constrain the DictWord lookup
                    // to dict_label = "dpd" so we don't accidentally surface
                    // a non-DPD row that happens to share the lemma. This
                    // keeps Path A's totals in sync with the inclusion set.
                    let dict_word_result: Result<DictWord, _> = dict_query
                        .filter(dict_dsl::dict_label.eq("dpd"))
                        .filter(dict_dsl::word.eq(&headword.lemma_1))
                        .filter(dict_dsl::uid.like(&uid_prefix_pat))
                        .filter(dict_dsl::uid.like(&uid_suffix_pat))
                        .first::<DictWord>(dict_conn);

                    if let Ok(dict_word) = dict_word_result
                        && seen_ids.insert(dict_word.id)
                    {
                        all_rows.push(dict_word);
                    }
                }
            }
        }

        // ---------- Path B: dict_words_fts.word → user-imported dicts ----------
        if path_b_enabled {
            let in_clause: Option<(String, Vec<String>)> = match &non_dpd_subset {
                Some(set) => Self::dict_label_in_clause(set),
                None => None,
            };

            let mut sql = String::from(
                "SELECT dw.* FROM dict_words dw \
                 JOIN dict_words_fts f ON f.dict_word_id = dw.id \
                 WHERE f.word LIKE ?"
            );

            if self.source.is_some() {
                if self.source_include {
                    sql.push_str(" AND dw.dict_label = ?");
                } else {
                    sql.push_str(" AND dw.dict_label != ?");
                }
            }

            if let Some((ph, _)) = &in_clause {
                sql.push_str(" AND dw.dict_label IN (");
                sql.push_str(ph);
                sql.push(')');
            } else {
                // No explicit subset → exclude DPD so Path B doesn't
                // duplicate Path A's dict_label="dpd" rows.
                sql.push_str(" AND dw.dict_label != 'dpd'");
            }

            sql.push_str(" AND dw.uid LIKE ? AND dw.uid LIKE ? ORDER BY dw.id LIMIT ?");

            let mut q = sql_query(&sql)
                .into_boxed::<diesel::sqlite::Sqlite>()
                .bind::<Text, _>(like_pattern.clone());

            if let Some(ref source_val) = self.source {
                q = q.bind::<Text, _>(source_val.clone());
            }

            if let Some((_, binds)) = &in_clause {
                for v in binds {
                    q = q.bind::<Text, _>(v.clone());
                }
            }

            q = q
                .bind::<Text, _>(uid_prefix_pat.clone())
                .bind::<Text, _>(uid_suffix_pat.clone())
                .bind::<BigInt, _>(SAFETY_LIMIT_SQL);

            let user_rows: Vec<DictWord> = q.load(dict_conn)?;

            if (user_rows.len() as i64) >= SAFETY_LIMIT_SQL {
                warn(&format!(
                    "lemma_1_dpd_headword_match_fts5 Path B hit SAFETY_LIMIT_SQL={} (query='{}')",
                    SAFETY_LIMIT_SQL, &self.query_text
                ));
            }

            for row in user_rows {
                if seen_ids.insert(row.id) {
                    all_rows.push(row);
                }
            }
        }

        // Merge ordering: exact `word == query_text` rows first, then
        // contains rows; tie-break by dict_label then id.
        let qt = self.query_text.as_str();
        all_rows.sort_by(|a, b| {
            let a_exact = a.word == qt;
            let b_exact = b.word == qt;
            b_exact.cmp(&a_exact)
                .then_with(|| a.dict_label.cmp(&b.dict_label))
                .then_with(|| a.id.cmp(&b.id))
        });

        let search_results: Vec<SearchResult> = all_rows
            .iter()
            .map(|dw| self.db_word_to_result(dw))
            .collect();
        let total = search_results.len();

        info(&format!("Query took: {:?}", timer.elapsed()));
        Ok((search_results, total))
    }

    // ===== Per-mode handlers (Stage 3 dispatch) =====
    //
    // Each handler takes `page_num` and returns `(Vec<SearchResult>, total)`.
    // Filter push-down (uid prefix/suffix, lang, source, etc.) happens inside
    // each handler at the storage layer; the caller does not post-filter.

    fn fulltext_suttas(&self, page_num: usize) -> Result<(Vec<SearchResult>, usize), Box<dyn Error>> {
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
            uid_suffix: self.uid_suffix.clone(),
            sutta_ref: None,
            include_cst_mula: self.include_cst_mula,
            include_cst_commentary: self.include_cst_commentary,
            include_ms_mula: self.include_ms_mula,
            include_bold_definitions: true,
            dict_source_uids: None,
        };

        let query_text = self.query_text.clone();

        let (total, results) = match with_fulltext_searcher(|searcher| {
            if !searcher.has_sutta_indexes() {
                warn("No sutta fulltext indexes available.");
                return Ok((0usize, Vec::new()));
            }
            searcher.search_suttas_with_count(&query_text, &filters, self.page_len, page_num)
        }) {
            Some(Ok(x)) => x,
            Some(Err(e)) => return Err(e.into()),
            None => {
                warn("Fulltext searcher not initialized. Indexes may not exist.");
                (0usize, Vec::new())
            }
        };

        Ok((results, total))
    }

    /// Fulltext + Dictionary handler. The dict index is unified: dict_words
    /// rows and DPD bold-definition rows live together, distinguished by the
    /// `is_bold_definition` field. A single tantivy call returns the page
    /// directly — no cover-fetch, no Rust-side merge. When
    /// `include_comm_bold_definitions == false`, bold rows are excluded via
    /// `Occur::MustNot` at the query stage. BM25 is internally consistent
    /// across both kinds.
    fn fulltext_dict(&self, page_num: usize) -> Result<(Vec<SearchResult>, usize), Box<dyn Error>> {
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
            uid_prefix: self.uid_prefix.clone(),
            uid_suffix: self.uid_suffix.clone(),
            sutta_ref: None,
            include_cst_mula: true,
            include_cst_commentary: true,
            include_ms_mula: true,
            include_bold_definitions: self.include_comm_bold_definitions,
            dict_source_uids: self.dict_source_uids.clone(),
        };

        let query_text = self.query_text.clone();

        let (total, results) = match with_fulltext_searcher(|searcher| {
            if !searcher.has_dict_indexes() {
                warn("No dict_word fulltext indexes available.");
                return Ok((0usize, Vec::new()));
            }
            searcher.search_dict_words_with_count(&query_text, &filters, self.page_len, page_num)
        }) {
            Some(Ok(x)) => x,
            Some(Err(e)) => return Err(e.into()),
            None => {
                warn("Fulltext searcher not initialized. Indexes may not exist.");
                (0usize, Vec::new())
            }
        };

        Ok((results, total))
    }

    fn fulltext_library(&self, page_num: usize) -> Result<(Vec<SearchResult>, usize), Box<dyn Error>> {
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
            uid_prefix: self.uid_prefix.clone(),
            uid_suffix: self.uid_suffix.clone(),
            sutta_ref: None,
            include_cst_mula: true,
            include_cst_commentary: true,
            include_ms_mula: true,
            include_bold_definitions: true,
            dict_source_uids: None,
        };

        let query_text = self.query_text.clone();

        let (total, results) = match with_fulltext_searcher(|searcher| {
            if !searcher.has_library_indexes() {
                warn("No library fulltext indexes available.");
                return Ok((0usize, Vec::new()));
            }
            searcher.search_library_with_count(&query_text, &filters, self.page_len, page_num)
        }) {
            Some(Ok(x)) => x,
            Some(Err(e)) => return Err(e.into()),
            None => {
                warn("Fulltext searcher not initialized. Indexes may not exist.");
                (0usize, Vec::new())
            }
        };

        Ok((results, total))
    }

    /// Boundary-aware page splitter for two concatenated streams: regular
    /// rows first (count `regular_total`), then bold-definition rows (count
    /// `bold_total`). For the requested page `[page_num*page_len .. +page_len)`
    /// returns the offset/limit pair to apply to each stream so that exactly
    /// `page_len` rows (or fewer on the last page) are fetched in total.
    /// Cost is O(page_len) regardless of `page_num` — no cover-fetch.
    fn split_page_across_streams(
        regular_total: usize,
        page_num: usize,
        page_len: usize,
    ) -> (usize, usize, usize, usize) {
        let start = page_num.saturating_mul(page_len);
        let end = start.saturating_add(page_len);

        let reg_offset = start.min(regular_total);
        let reg_end = end.min(regular_total);
        let reg_limit = reg_end.saturating_sub(reg_offset);

        let bold_offset = start.saturating_sub(regular_total);
        let bold_end = end.saturating_sub(regular_total);
        let bold_limit = bold_end.saturating_sub(bold_offset);

        (reg_offset, reg_limit, bold_offset, bold_limit)
    }

    /// DPD Lookup + bold-definitions append. The DPD lookup is structurally
    /// multi-phase with per-phase dedup so it's materialised in memory
    /// (with uid filters pushed down to keep it small); the bold side is
    /// fetched with a true `LIMIT/OFFSET` SQL query for just the bold slice
    /// the page needs (or only its COUNT when the page lies entirely inside
    /// the regular range). No cover-fetch.
    fn dpd_lookup_with_bold(&self, page_num: usize) -> Result<(Vec<SearchResult>, usize), Box<dyn Error>> {
        let regular_full = self.dpd_lookup_full()?;
        let regular_total = regular_full.len();

        let (reg_off, reg_lim, bold_off, bold_lim) =
            Self::split_page_across_streams(regular_total, page_num, self.page_len);

        let reg_slice: Vec<SearchResult> = regular_full
            .into_iter()
            .skip(reg_off)
            .take(reg_lim)
            .collect();

        let (bold_total, bold_slice) =
            self.query_bold_definitions_bold_fts5(&self.query_text, bold_off, bold_lim)?;

        let mut page = reg_slice;
        page.extend(bold_slice);
        let total = regular_total + bold_total as usize;
        Ok((page, total))
    }

    /// Headword match + bold-definitions append. Regular side is the
    /// multi-phase headword-FTS5 result, materialised then sliced; bold
    /// side is a true paged SQL fetch.
    fn headword_match_with_bold(&self, page_num: usize) -> Result<(Vec<SearchResult>, usize), Box<dyn Error>> {
        let (regular_full, regular_total) = self.lemma_1_dpd_headword_match_fts5_full()?;

        let (reg_off, reg_lim, bold_off, bold_lim) =
            Self::split_page_across_streams(regular_total, page_num, self.page_len);

        let reg_slice: Vec<SearchResult> = regular_full
            .into_iter()
            .skip(reg_off)
            .take(reg_lim)
            .collect();

        let (bold_total, bold_slice) =
            self.query_bold_definitions_bold_fts5(&self.query_text, bold_off, bold_lim)?;

        let mut page = reg_slice;
        page.extend(bold_slice);
        let total = regular_total + bold_total as usize;
        Ok((page, total))
    }

    /// ContainsMatch + Dictionary + bold-definitions append. Same shape:
    /// multi-phase regular set materialised, bold side fetched only for
    /// the slice (or only counted) via `LIMIT/OFFSET`.
    fn dict_contains_with_bold(&self, page_num: usize) -> Result<(Vec<SearchResult>, usize), Box<dyn Error>> {
        let (regular_full, regular_total) = self.dict_words_contains_match_fts5_full()?;

        let (reg_off, reg_lim, bold_off, bold_lim) =
            Self::split_page_across_streams(regular_total, page_num, self.page_len);

        let reg_slice: Vec<SearchResult> = regular_full
            .into_iter()
            .skip(reg_off)
            .take(reg_lim)
            .collect();

        let normalized_q = normalize_plain_text(&self.query_text);
        let (bold_total, bold_slice) =
            self.query_bold_definitions_commentary_fts5(&normalized_q, bold_off, bold_lim)?;

        let mut page = reg_slice;
        page.extend(bold_slice);
        let total = regular_total + bold_total as usize;
        Ok((page, total))
    }

    /// UidMatch handler: the existing `uid_*_all` impls already return
    /// small bounded result sets (single exact-match or a uid-prefix-LIKE
    /// hit set), so we slice in Rust.
    fn uid_match(&mut self, page_num: usize) -> Result<(Vec<SearchResult>, usize), Box<dyn Error>> {
        let all = match self.search_area {
            SearchArea::Suttas => self.uid_sutta_all()?,
            SearchArea::Dictionary => self.uid_word_all()?,
            SearchArea::Library => self.uid_book_spine_item_all()?,
        };
        let total = all.len();
        let start = page_num.saturating_mul(self.page_len);
        let page: Vec<SearchResult> = all.into_iter().skip(start).take(self.page_len).collect();
        Ok((page, total))
    }

    /// Returns a lowercased, trimmed string if the option is `Some` and non-empty.
    fn normalized_filter(opt: &Option<String>) -> Option<String> {
        opt.as_ref()
            .map(|s| s.trim().to_lowercase())
            .filter(|s| !s.is_empty())
    }

    /// Build a `dict_label IN (?, ?, ...)` clause for embedding in raw SQL.
    /// Returns the placeholder string (without the surrounding parens) and a
    /// vector of bind values, or `None` when the set is empty so the caller
    /// can skip the phase entirely. Callers handling the `dict_source_uids`
    /// contract treat `Some(empty)` as "drop everything" (skip the phase) and
    /// `None` as "no constraint" (drop the IN clause); this helper services
    /// only the non-empty case.
    fn dict_label_in_clause(set: &[String]) -> Option<(String, Vec<String>)> {
        if set.is_empty() {
            return None;
        }
        let placeholders = std::iter::repeat_n("?", set.len())
            .collect::<Vec<_>>()
            .join(", ");
        Some((placeholders, set.to_vec()))
    }

    /// Filter-aware mode dispatch. Each per-mode handler pushes its filters
    /// (uid prefix/suffix included) down to the storage layer and returns
    /// `(page, total)`. `db_query_hits_count` is written exactly once here.
    pub fn results_page(&mut self, page_num: usize) -> Result<Vec<SearchResult>, Box<dyn Error>> {
        let (page, total) = match self.search_mode {
            SearchMode::FulltextMatch => match self.search_area {
                SearchArea::Suttas => self.fulltext_suttas(page_num)?,
                SearchArea::Dictionary => self.fulltext_dict(page_num)?,
                SearchArea::Library => self.fulltext_library(page_num)?,
            },

            SearchMode::ContainsMatch => match self.search_area {
                SearchArea::Suttas => self.suttas_contains_match_fts5(page_num, self.page_len)?,
                SearchArea::Dictionary => {
                    if self.include_comm_bold_definitions {
                        self.dict_contains_with_bold(page_num)?
                    } else {
                        self.dict_words_contains_match_fts5(page_num, self.page_len)?
                    }
                }
                SearchArea::Library => self.book_spine_items_contains_match_fts5(page_num, self.page_len)?,
            },

            SearchMode::TitleMatch => match self.search_area {
                SearchArea::Suttas => self.suttas_title_match(page_num, self.page_len)?,
                // Title Match doesn't make sense for dictionary
                SearchArea::Dictionary => (Vec::new(), 0),
                SearchArea::Library => self.library_title_match(page_num, self.page_len)?,
            },

            SearchMode::HeadwordMatch => match self.search_area {
                SearchArea::Dictionary => {
                    if self.include_comm_bold_definitions {
                        self.headword_match_with_bold(page_num)?
                    } else {
                        self.lemma_1_dpd_headword_match_fts5(page_num, self.page_len)?
                    }
                }
                _ => (Vec::new(), 0),
            },

            SearchMode::DpdLookup => match self.search_area {
                SearchArea::Dictionary => {
                    if self.include_comm_bold_definitions {
                        self.dpd_lookup_with_bold(page_num)?
                    } else {
                        self.dpd_lookup(page_num, self.page_len)?
                    }
                }
                _ => (Vec::new(), 0),
            },

            SearchMode::UidMatch => self.uid_match(page_num)?,

            SearchMode::Combined => (Vec::new(), 0),

            _ => {
                error(&format!("Search mode {:?} not yet implemented.", self.search_mode));
                (Vec::new(), 0)
            }
        };

        // Dictionary inclusion-set post-filter. The Tantivy dict path
        // already pushes this down via `add_dict_filters`; the SQL paths
        // (Contains / DpdLookup / HeadwordMatch / UidMatch on Dictionary)
        // do not. Apply a uniform post-filter here so all paths agree.
        // Bold-definition rows are gated independently by
        // `include_comm_bold_definitions`; we never drop them on the basis
        // of `dict_source_uids` because their source_uid is a per-row
        // ref_code rather than a dictionary label.
        let (page, total) = if self.search_area == SearchArea::Dictionary
            && self.search_mode != SearchMode::FulltextMatch
        {
            self.apply_dict_source_uids_filter(page, total)
        } else {
            (page, total)
        };

        self.db_query_hits_count = total as i64;

        Ok(page.into_iter().map(|r| self.highlight_row(r)).collect())
    }

    /// Restrict dict_words rows to those whose `source_uid` (= `dict_label`)
    /// is in `self.dict_source_uids`. Bold-definition rows are never
    /// dropped here — they're toggled separately by
    /// `include_comm_bold_definitions`. When the inclusion set is `None`
    /// the input is returned unchanged; when it is `Some([])` every
    /// dict_words row is dropped.
    fn apply_dict_source_uids_filter(
        &self,
        page: Vec<SearchResult>,
        total: usize,
    ) -> (Vec<SearchResult>, usize) {
        let Some(set) = self.dict_source_uids.as_ref() else {
            return (page, total);
        };

        let set: HashSet<&str> = set.iter().map(|s| s.as_str()).collect();
        let original_dict_words = page
            .iter()
            .filter(|r| r.table_name == "dict_words")
            .count();

        let filtered: Vec<SearchResult> = page
            .into_iter()
            .filter(|r| {
                if r.table_name != "dict_words" {
                    return true;
                }
                match r.source_uid.as_deref() {
                    Some(uid) => set.contains(uid),
                    None => false,
                }
            })
            .collect();

        let dropped = original_dict_words.saturating_sub(
            filtered.iter().filter(|r| r.table_name == "dict_words").count(),
        );
        if dropped > 0 {
            debug(&format!(
                "dict_source_uids post-filter dropped {} rows on {:?}",
                dropped, self.search_mode
            ));
        }
        let new_total = total.saturating_sub(dropped);
        (filtered, new_total)
    }

    fn highlight_row(&self, mut r: SearchResult) -> SearchResult {
        let is_dpd_result = r.table_name == "dpd_headwords"
            || r.table_name == "dpd_roots"
            || (r.table_name == "dict_words"
                && r.source_uid.as_ref().is_some_and(|s| s.to_lowercase().contains("dpd")));

        if !is_dpd_result {
            let q = normalize_plain_text(&self.query_text);
            r.snippet = self.highlight_query_in_content(&q, &r.snippet);
        }
        r
    }

    /// Returns the total number of hits found in the last database query.
    pub fn total_hits(&self) -> i64 {
        self.db_query_hits_count
    }
}
