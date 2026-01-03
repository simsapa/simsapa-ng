// use std::any::Any;
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::time::Instant;

use regex::Regex;
use diesel::prelude::*;
use diesel::sql_query;
use diesel::sql_types::{Text, BigInt};

use crate::helpers::{normalize_query_text, sutta_range_from_ref};
use crate::{get_app_data, get_app_globals};
use crate::types::{SearchArea, SearchMode, SearchParams, SearchResult};
use crate::db::appdata_models::{Sutta, BookSpineItem};
use crate::db::dictionaries_models::DictWord;
use crate::db::DbManager;
use crate::logger::{info, warn, error};

#[derive(QueryableByName)]
struct CountResult {
    #[diesel(sql_type = BigInt)]
    count: i64,
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
    pub highlighted_result_pages: HashMap<usize, Vec<SearchResult>>,
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
        let lang_filter = params.lang.clone().unwrap_or_else(|| String::new());

        // For UidMatch mode, don't normalize the query text to preserve dots and other characters
        // For other modes, normalize to handle punctuation and spacing
        let query_text = if params.mode == SearchMode::UidMatch {
            query_text_orig.to_lowercase()
        } else {
            normalize_query_text(Some(query_text_orig))
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
            highlighted_result_pages: HashMap::new(),
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
        let highlighted = re.replace_all(&content, "<span class='match'>$1</span>");
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

    fn uid_sutta(&mut self, page_num: usize) -> Result<Vec<SearchResult>, Box<dyn Error>> {
        use crate::db::appdata_schema::suttas::dsl::*;
        use diesel::result::Error as DieselError;

        let app_data = get_app_data();
        let db_conn = &mut app_data.dbm.appdata.get_conn()?;

        let query_uid = self.query_text.to_lowercase().replace("uid:", "");

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
            Ok(sutta) => {
                // Found exact match - return single result
                self.db_query_hits_count = 1;
                Ok(vec![self.db_sutta_to_result(&sutta)])
            }
            Err(DieselError::NotFound) => {
                // No exact match found
                // Check if this is actually a range query (e.g., sn56.11-15) or just a simple ref (e.g., sn56.11)
                let is_range_query = query_uid.contains('-') &&
                    query_uid.chars().filter(|&c| c == '-').count() == 1 &&
                    query_uid.split('-').all(|part| part.chars().any(char::is_numeric));

                if is_range_query {
                    // Try range query for actual ranges like sn56.11-15
                    match self.uid_sutta_range(&query_uid, page_num) {
                        Ok(results) if !results.is_empty() => {
                            return Ok(results);
                        }
                        _ => {
                            // Range query failed, fall through to LIKE query
                        }
                    }
                }

                // For simple references like sn56.11, use LIKE query to get all translations
                self.uid_sutta_like(&query_uid, page_num)
            }
            Err(e) => {
                error(&format!("{}", e));
                // Err(Box::new(e))
                // return an empty list instead of the error.
                Ok(Vec::new())
            }
        }
    }

    fn uid_sutta_range(
        &mut self,
        query_uid: &str,
        page_num: usize,
    ) -> Result<Vec<SearchResult>, Box<dyn Error>> {
        use crate::db::appdata_schema::suttas::dsl::*;

        // Parse the query uid to extract range information
        let range = match sutta_range_from_ref(query_uid) {
            Some(r) => r,
            None => return Ok(Vec::new()),
        };

        // Only proceed if we have both start and end values (meaning it's a numeric query)
        let (range_start, range_end) = match (range.start, range.end) {
            (Some(s), Some(e)) => (s as i32, e as i32),
            _ => return Ok(Vec::new()),
        };

        let app_data = get_app_data();
        let db_conn = &mut app_data.dbm.appdata.get_conn()?;

        // Build query to find suttas where the query number falls within the stored range
        let mut count_query = suttas.into_boxed();
        count_query = count_query
            .filter(sutta_range_group.eq(&range.group))
            .filter(sutta_range_start.is_not_null())
            .filter(sutta_range_end.is_not_null())
            .filter(sutta_range_start.le(range_start))
            .filter(sutta_range_end.ge(range_end));

        // Apply language filter if specified
        if !self.lang.is_empty() && self.lang != "Language" {
            count_query = count_query.filter(language.eq(&self.lang));
        }

        // Count total hits for pagination
        let count = count_query
            .count()
            .get_result::<i64>(db_conn)?;

        self.db_query_hits_count = count;

        // If no results, return empty vector
        if count == 0 {
            return Ok(Vec::new());
        }

        // Calculate pagination
        let offset = (page_num * self.page_len) as i64;
        let limit = self.page_len as i64;

        // Build main query with same filters
        let mut query = suttas.into_boxed();
        query = query
            .filter(sutta_range_group.eq(&range.group))
            .filter(sutta_range_start.is_not_null())
            .filter(sutta_range_end.is_not_null())
            .filter(sutta_range_start.le(range_start))
            .filter(sutta_range_end.ge(range_end));

        // Apply language filter if specified
        if !self.lang.is_empty() && self.lang != "Language" {
            query = query.filter(language.eq(&self.lang));
        }

        // Execute paginated query
        let results = query
            .order(uid.asc()) // Order by uid for consistent pagination
            .limit(limit)
            .offset(offset)
            .select(Sutta::as_select())
            .load::<Sutta>(db_conn)?;

        // Map to SearchResult
        let search_results = results
            .iter()
            .map(|sutta| self.db_sutta_to_result(sutta))
            .collect();

        Ok(search_results)
    }

    fn uid_sutta_like(
        &mut self,
        query_uid: &str,
        page_num: usize
    ) -> Result<Vec<SearchResult>, Box<dyn Error>> {
        use crate::db::appdata_schema::suttas::dsl::*;

        info(&format!("uid_sutta_like(): query_uid='{}', page_num={}, lang='{}'", query_uid, page_num, self.lang));

        let app_data = get_app_data();
        let db_conn = &mut app_data.dbm.appdata.get_conn()?;

        let like_pattern = format!("{}%", query_uid);

        // Build query with language filter
        let mut count_query = suttas.into_boxed();
        count_query = count_query.filter(uid.like(&like_pattern));

        // Apply language filter if specified
        if !self.lang.is_empty() && self.lang != "Language" {
            count_query = count_query.filter(language.eq(&self.lang));
        }

        // Count total hits for pagination
        let count = count_query
            .count()
            .get_result::<i64>(db_conn)?;

        self.db_query_hits_count = count;

        // If no results, return empty vector
        if count == 0 {
            return Ok(Vec::new());
        }

        // Calculate pagination
        let offset = (page_num * self.page_len) as i64;
        let limit = self.page_len as i64;

        // Build main query with same filters
        let mut query = suttas.into_boxed();
        query = query.filter(uid.like(&like_pattern));

        // Apply language filter if specified
        if !self.lang.is_empty() && self.lang != "Language" {
            query = query.filter(language.eq(&self.lang));
        }

        // Execute paginated query
        let results = query
            .order(uid.asc()) // Order by uid for consistent pagination
            .limit(limit)
            .offset(offset)
            .select(Sutta::as_select())
            .load::<Sutta>(db_conn)?;

        info(&format!("uid_sutta_like(): Found {} results for page {}, total_hits={}", results.len(), page_num, self.db_query_hits_count));

        // Map to SearchResult
        let search_results = results
            .iter()
            .map(|sutta| self.db_sutta_to_result(sutta))
            .collect();

        Ok(search_results)
    }

    fn uid_word(&mut self) -> Result<Vec<SearchResult>, Box<dyn Error>> {
        // TODO: review details in query_task.py
        use crate::db::dictionaries_schema::dict_words::dsl::*;
        let app_data = get_app_data();

        let query_uid = self.query_text.to_lowercase().replace("uid:", "");

        // Check if this is a DPD numeric UID (e.g., "123/dpd" or just "123")
        // DPD headword UIDs are numeric IDs, optionally with /dpd suffix
        let ref_str = query_uid.replace("/dpd", "");
        if query_uid.ends_with("/dpd") && ref_str.chars().all(char::is_numeric) {
            // Use dpd_lookup which handles numeric UIDs
            let results = app_data.dbm.dpd.dpd_lookup(&query_uid, false, true)?;
            self.db_query_hits_count = results.len() as i64;
            return Ok(results);
        }

        let db_conn = &mut app_data.dbm.dictionaries.get_conn()?;

        // First try exact UID match for dict_words
        let res = dict_words
            .filter(uid.eq(&query_uid))
            .select(DictWord::as_select())
            .first(db_conn);

        match res {
            Ok(res_word) => {
                self.db_query_hits_count = 1;
                return Ok(vec![self.db_word_to_result(&res_word)]);
            }
            Err(_) => {
                // Exact match not found, continue to try partial match
            }
        }

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

            self.db_query_hits_count = res.len() as i64;
            return Ok(res.iter().map(|w| self.db_word_to_result(w)).collect());
        }

        // No results found
        self.db_query_hits_count = 0;
        Ok(Vec::new())
    }

    fn uid_book_spine_item(&mut self) -> Result<Vec<SearchResult>, Box<dyn Error>> {
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
                    self.db_query_hits_count = results.len() as i64;
                    Ok(results)
                }
                Err(_) => {
                    Ok(Vec::new())
                }
            }
        }
    }

    /// Fetches a page of results for Suttas using CONTAINS or REGEX matching.
    #[allow(dead_code)]
    fn suttas_contains_or_regex_match_page(
        &mut self,
        page_num: usize,
    ) -> Result<Vec<SearchResult>, Box<dyn Error>> {
        info(&format!("suttas_contains_or_regex_match_page(): page_num: {}", page_num));
        info(&format!("query_text: {}", &self.query_text));
        let timer = Instant::now();
        // TODO: review details in query_task.py
        use crate::db::appdata_schema::suttas::dsl::*;

        let app_data = get_app_data();
        let db_conn = &mut app_data.dbm.appdata.get_conn()?;

        // Box query for dynamic filtering
        let mut query = suttas.into_boxed();
        // A separate query for the total count. Can't clone the query before the offset limit.
        let mut count_query = suttas.into_boxed();

        // --- Language Filtering ---
        if !self.lang.is_empty() && self.lang != "Language" {
            query = query.filter(language.eq(&self.lang));
            count_query = count_query.filter(language.eq(&self.lang));
        }

        // --- Source Filtering ---
        if let Some(ref source_val) = self.source {
            let pattern = format!("%/{}", source_val); // SQL LIKE pattern, e.g. ".../cst4"
            if self.source_include {
                query = query.filter(uid.like(pattern.clone()));
                count_query = count_query.filter(uid.like(pattern.clone()));
            } else {
                query = query.filter(uid.not_like(pattern.clone()));
                count_query = count_query.filter(uid.not_like(pattern.clone()));
            }
        }

        // --- Term Filtering ---
        let terms: Vec<&str> = if self.query_text.contains(" AND ") {
            self.query_text.split(" AND ").map(|s| s.trim()).collect()
        } else {
            vec![self.query_text.as_str()]
        };

        for term in terms {
            match self.search_mode {
                SearchMode::ContainsMatch => {
                    query = query.filter(content_plain.like(format!("%{}%", term)));
                    count_query = count_query.filter(content_plain.like(format!("%{}%", term)));
                }
                SearchMode::RegExMatch => {
                    // FIXME use diesel regex match
                    query = query.filter(content_plain.like(format!("%{}%", term)));
                    count_query = count_query.filter(content_plain.like(format!("%{}%", term)));
                }
                _ => {
                    return Err(format!(
                        "Invalid search mode in suttas_contains_or_regex_match_page: {:?}",
                        self.search_mode
                    )
                    .into());
                }
            }
        }

        // --- Count Total Hits ---
        self.db_query_hits_count = count_query.select(diesel::dsl::count_star()).first(db_conn)?;

        // --- Apply Pagination ---
        let offset = (page_num * self.page_len) as i64;
        query = query.offset(offset).limit(self.page_len as i64);

        // --- Execute Query ---
        // info(&format!("Executing Query: {:?}", diesel::debug_query::<diesel::sqlite::Sqlite, _>(&query)));
        let db_results: Vec<Sutta> = query.load::<Sutta>(db_conn)?;

        // --- Map to SearchResult ---
        let search_results = db_results
            .iter()
            .map(|sutta| self.db_sutta_to_result(sutta))
            .collect();

        info(&format!("Query took: {:?}", timer.elapsed()));
        Ok(search_results)
    }

    fn suttas_contains_match_fts5(
        &mut self,
        page_num: usize,
    ) -> Result<Vec<SearchResult>, Box<dyn Error>> {
        info(&format!("suttas_contains_match_fts5(): page_num: {}", page_num));
        info(&format!("query_text: {}, lang filter: {}", &self.query_text, &self.lang));
        let timer = Instant::now();

        let app_data = get_app_data();
        let db_conn = &mut app_data.dbm.appdata.get_conn()?;

        // TODO --- Source Filtering ---
        // TODO --- Term Filtering ---

        let like_pattern = format!("%{}%", self.query_text);

        // Determine if we need language filtering
        let apply_lang_filter = !self.lang.is_empty() && self.lang != "Language";

        // --- Count Total Hits ---
        let count_result: CountResult = if apply_lang_filter {
            sql_query(
                r#"
                SELECT COUNT(*) as count
                FROM suttas_fts f
                JOIN suttas s ON f.sutta_id = s.id
                WHERE f.content_plain LIKE ? AND f.language = ?
                "#
            )
            .bind::<Text, _>(&like_pattern)
            .bind::<Text, _>(&self.lang)
            .get_result(db_conn)?
        } else {
            sql_query(
                r#"
                SELECT COUNT(*) as count
                FROM suttas_fts f
                JOIN suttas s ON f.sutta_id = s.id
                WHERE f.content_plain LIKE ?
                "#
            )
            .bind::<Text, _>(&like_pattern)
            .get_result(db_conn)?
        };

        self.db_query_hits_count = count_result.count;
        info(&format!("db_query_hits_count: {}", self.db_query_hits_count));

        // --- Apply Pagination ---
        let offset = (page_num * self.page_len) as i64;
        let limit = self.page_len as i64;

        // NOTE: 'ORDER BY rank' is very slow.
        // Ordering by id for predictable results on the same query.
        // Without specifying the ordering, FTS5 results are not ordered and fluctuate.

        // --- Execute Query with Pagination ---
        let db_results: Vec<Sutta> = if apply_lang_filter {
            sql_query(
                r#"
                SELECT s.*
                FROM suttas_fts f
                JOIN suttas s ON f.sutta_id = s.id
                WHERE f.content_plain LIKE ? AND f.language = ?
                ORDER BY s.id
                LIMIT ? OFFSET ?
                "#
            )
            .bind::<Text, _>(&like_pattern)
            .bind::<Text, _>(&self.lang)
            .bind::<BigInt, _>(limit)
            .bind::<BigInt, _>(offset)
            .load(db_conn)?
        } else {
            sql_query(
                r#"
                SELECT s.*
                FROM suttas_fts f
                JOIN suttas s ON f.sutta_id = s.id
                WHERE f.content_plain LIKE ?
                ORDER BY s.id
                LIMIT ? OFFSET ?
                "#
            )
            .bind::<Text, _>(&like_pattern)
            .bind::<BigInt, _>(limit)
            .bind::<BigInt, _>(offset)
            .load(db_conn)?
        };

        // --- Map to SearchResult ---
        let search_results = db_results
            .iter()
            .map(|sutta| self.db_sutta_to_result(sutta))
            .collect();

        info(&format!("Query took: {:?}", timer.elapsed()));
        Ok(search_results)
    }

    fn dict_words_contains_match_fts5(
        &mut self,
        page_num: usize,
    ) -> Result<Vec<SearchResult>, Box<dyn Error>> {
        info(&format!("dict_words_contains_match_fts5(): page_num: {}", page_num));
        info(&format!("query_text: {}", &self.query_text));
        let timer = Instant::now();

        // TODO: review details in query_task.py

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

        // --- Calculate Pagination ---
        // Apply pagination in each phase to reduce the number of items fetched
        let query_offset = (page_num * self.page_len) as i64;
        let query_limit = self.page_len as i64;

        // Three-phase search: DpdHeadword exact -> DpdHeadword contains -> DictWord definition

        let mut all_results: Vec<DictWord> = Vec::new();
        let mut result_uids: HashSet<String> = HashSet::new();

        // Phase 1: Exact matches on DpdHeadword.lemma_clean
        // dpd.lemma_clean has btree index and dpd.lemma_1 has unique constraint and so implicitly indexed.
        for term in &terms {
            let exact_matches: Vec<DpdHeadword> = dpd_dsl::dpd_headwords
                .filter(dpd_dsl::lemma_clean.eq(term))
                .order(dpd_dsl::id)
                .limit(query_limit)
                .offset(query_offset)
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
                LIMIT ? OFFSET ?
                "#
            );

            let headword_ids: Vec<HeadwordId> = sql_query(&fts_query)
                .bind::<Text, _>(&like_pattern)
                .bind::<BigInt, _>(query_limit)
                .bind::<BigInt, _>(query_offset)
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
                        LIMIT ? OFFSET ?
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
                        LIMIT ? OFFSET ?
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
                    LIMIT ? OFFSET ?
                    "#
                )
            };

            let def_results: Vec<DictWord> = if let Some(ref source_val) = self.source {
                sql_query(&fts_query)
                    .bind::<Text, _>(&like_pattern)
                    .bind::<Text, _>(source_val)
                    .bind::<BigInt, _>(query_limit)
                    .bind::<BigInt, _>(query_offset)
                    .load(db_conn)?
            } else {
                sql_query(&fts_query)
                    .bind::<Text, _>(&like_pattern)
                    .bind::<BigInt, _>(query_limit)
                    .bind::<BigInt, _>(query_offset)
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

        // Set total hits count
        self.db_query_hits_count = all_results.len() as i64;

        // Apply array-based pagination which affects all collected results
        let offset = page_num * self.page_len;
        let end_idx = std::cmp::min(offset + self.page_len, all_results.len());

        let paginated_results = if offset >= all_results.len() {
            Vec::new()
        } else {
            all_results[offset..end_idx].to_vec()
        };

        // Map to SearchResult
        let search_results = paginated_results
            .iter()
            .map(|dict_word| self.db_word_to_result(dict_word))
            .collect();

        info(&format!("Query took: {:?}", timer.elapsed()));
        Ok(search_results)
    }

    fn book_spine_items_contains_match_fts5(
        &mut self,
        page_num: usize,
    ) -> Result<Vec<SearchResult>, Box<dyn Error>> {
        info(&format!("book_spine_items_contains_match_fts5(): page_num: {}", page_num));
        info(&format!("query_text: {}, lang filter: {}", &self.query_text, &self.lang));
        let timer = Instant::now();

        let app_data = get_app_data();
        let db_conn = &mut app_data.dbm.appdata.get_conn()?;

        let like_pattern = format!("%{}%", self.query_text);

        // Determine if we need language filtering
        let apply_lang_filter = !self.lang.is_empty() && self.lang != "Language";

        // --- Count Total Hits ---
        let count_result: CountResult = if apply_lang_filter {
            sql_query(
                r#"
                SELECT COUNT(*) as count
                FROM book_spine_items_fts f
                JOIN book_spine_items b ON f.spine_item_id = b.id
                WHERE f.content_plain LIKE ? AND f.language = ?
                "#
            )
            .bind::<Text, _>(&like_pattern)
            .bind::<Text, _>(&self.lang)
            .get_result(db_conn)?
        } else {
            sql_query(
                r#"
                SELECT COUNT(*) as count
                FROM book_spine_items_fts f
                JOIN book_spine_items b ON f.spine_item_id = b.id
                WHERE f.content_plain LIKE ?
                "#
            )
            .bind::<Text, _>(&like_pattern)
            .get_result(db_conn)?
        };

        self.db_query_hits_count = count_result.count;
        info(&format!("db_query_hits_count: {}", self.db_query_hits_count));

        // --- Apply Pagination ---
        let offset = (page_num * self.page_len) as i64;
        let limit = self.page_len as i64;

        // --- Execute Query with Pagination ---
        let db_results: Vec<BookSpineItem> = if apply_lang_filter {
            sql_query(
                r#"
                SELECT b.*
                FROM book_spine_items_fts f
                JOIN book_spine_items b ON f.spine_item_id = b.id
                WHERE f.content_plain LIKE ? AND f.language = ?
                ORDER BY b.id
                LIMIT ? OFFSET ?
                "#
            )
            .bind::<Text, _>(&like_pattern)
            .bind::<Text, _>(&self.lang)
            .bind::<BigInt, _>(limit)
            .bind::<BigInt, _>(offset)
            .load(db_conn)?
        } else {
            sql_query(
                r#"
                SELECT b.*
                FROM book_spine_items_fts f
                JOIN book_spine_items b ON f.spine_item_id = b.id
                WHERE f.content_plain LIKE ?
                ORDER BY b.id
                LIMIT ? OFFSET ?
                "#
            )
            .bind::<Text, _>(&like_pattern)
            .bind::<BigInt, _>(limit)
            .bind::<BigInt, _>(offset)
            .load(db_conn)?
        };

        // --- Map to SearchResult ---
        let search_results = db_results
            .iter()
            .map(|spine_item| self.db_book_spine_item_to_result(spine_item))
            .collect();

        info(&format!("Query took: {:?}", timer.elapsed()));
        Ok(search_results)
    }

    pub fn dpd_lookup(&mut self, page_num: usize) -> Result<Vec<SearchResult>, Box<dyn Error>> {
        // DPD is only English, so ignore checking self.lang (which may be "pli", "Language", or empty "").
        // Assume that if the DPD Lookup was selected then stale language settings can be ignored.
        let app_data = get_app_data();
        let all_results = app_data.dbm.dpd.dpd_lookup(&self.query_text, false, true)?;

        // Set total hits count for pagination
        self.db_query_hits_count = all_results.len() as i64;

        // Apply pagination
        let offset = page_num * self.page_len;
        let end_idx = std::cmp::min(offset + self.page_len, all_results.len());

        let paginated_results = if offset >= all_results.len() {
            Vec::new() // Return empty if page_num is beyond available results
        } else {
            all_results[offset..end_idx].to_vec()
        };

        Ok(paginated_results)
    }

    /// Gets a specific page of search results, performing the query if needed.
    pub fn results_page(&mut self, page_num: usize) -> Result<Vec<SearchResult>, Box<dyn Error>> {
        // Check cache first. If this results page has been calculated before, return it.
        if let Some(cached_page) = self.highlighted_result_pages.get(&page_num) {
            return Ok(cached_page.clone()); // Return a clone to avoid borrow issues
        }

        // Otherwise, run the queries and return the results page.

        // --- Perform Search Based on Mode and Area ---
        let results = match self.search_mode {
            SearchMode::DpdLookup => {
                // DPD Lookup mode - only works for Dictionary search area
                match self.search_area {
                    SearchArea::Dictionary => {
                        self.dpd_lookup(page_num)
                    }
                    SearchArea::Suttas => {
                        // DPD Lookup doesn't make sense for suttas
                        self.db_query_hits_count = 0;
                        Ok(Vec::new())
                    }
                    SearchArea::Library => {
                        // DPD Lookup doesn't make sense for library
                        self.db_query_hits_count = 0;
                        Ok(Vec::new())
                    }
                }
            }

            SearchMode::Combined => {
                let mut res: Vec<SearchResult> = Vec::new();

                // Display all DPD Lookup results (not many) on the
                // first (0 index) results page by boosting their scores.
                if page_num == 0 {
                    // Run DPD Lookup and boost results to the top.
                    let mut dpd_results: Vec<SearchResult> = self.dpd_lookup(0)?;
                    for item in dpd_results.iter_mut() {
                        match item.score {
                            Some(ref mut s) => *s += 10000.0,
                            None => item.score = Some(10000.0),
                        }
                    }
                    res.extend(dpd_results);
                    self.db_all_results = res.clone();
                }

                // The fulltext query has been executed before this step,
                // get highlighted snippets

                // FIXME implement when fulltext query works
                // let mut page_results = self.search_query.highlighted_results_page(page_num)?;
                // res.extend(page_results);

                // Deduplicate: unique by title, schema_name, and uid
                // NOTE: Is this necessary? Maybe When fulltext results are also added.
                // Ok(unique_search_results(res))

                Ok(res)
            }

            SearchMode::UidMatch => {
                match self.search_area {
                    SearchArea::Suttas => {
                        self.uid_sutta(page_num)
                    }
                    SearchArea::Dictionary => {
                        self.uid_word()
                    }
                    SearchArea::Library => {
                        self.uid_book_spine_item()
                    }
                }
            }

            SearchMode::ContainsMatch => {
                match self.search_area {
                    SearchArea::Suttas => {
                        self.suttas_contains_match_fts5(page_num)
                    }
                    SearchArea::Dictionary => {
                        self.dict_words_contains_match_fts5(page_num)
                    }
                    SearchArea::Library => {
                        self.book_spine_items_contains_match_fts5(page_num)
                    }
                }
            }

            // TODO handle SearchMode::RegExMatch with diesel regex
            // SearchMode::ContainsMatch | SearchMode::RegExMatch => {
            //     match self.search_area {
            //         SearchArea::Suttas => {
            //             self.suttas_contains_or_regex_match_page(page_num)
            //         }
            //         SearchArea::Dictionary => {
            //             self.dict_words_contains_or_regex_match_page(page_num)
            //         }
            //     }
            // }

            _ => {
                // FIXME: implement later
                error(&format!("Search mode {:?} not yet implemented.", self.search_mode));
                // Reset count and return empty for unimplemented modes for now
                self.db_query_hits_count = 0;
                Ok(Vec::new())
            }

        }?;

        // --- Apply Highlighting ---
        // The highlighting is now done *after* fetching and before caching
        let highlighted_results: Vec<SearchResult> = results
            .into_iter()
            .map(|mut result| {
                // Skip highlighting for DPD results (dpd_headwords, dpd_roots, dict_words with DPD source)
                // as they already have formatted meaning snippets from get_dpd_meaning_snippet()
                let is_dpd_result = result.table_name == "dpd_headwords"
                    || result.table_name == "dpd_roots"
                    || (result.table_name == "dict_words"
                        && result.source_uid.as_ref().map_or(false, |s| s.to_lowercase().contains("dpd")));

                if !is_dpd_result {
                    // Re-highlight the snippet based on the full query text
                    // Note: _db_sutta_to_result already created a basic snippet.
                    // This step applies the final highlighting spans.
                    result.snippet = self.highlight_query_in_content(&self.query_text, &result.snippet);
                }
                result
            })
            .collect();

        // --- Cache the highlighted results ---
        self.highlighted_result_pages.insert(page_num, highlighted_results.clone());

        Ok(highlighted_results)
    }

    /// Returns the total number of hits found in the last database query.
    pub fn total_hits(&self) -> i64 {
        self.db_query_hits_count
    }

}
