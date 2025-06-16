// use std::any::Any;
use std::collections::HashMap;
use std::error::Error;

use regex::Regex;
use diesel::prelude::*;

use crate::{get_app_data, get_app_globals};
use crate::types::{SearchArea, SearchMode, SearchParams, SearchResult};
use crate::helpers::{consistent_niggahita, unique_search_results};
use crate::db::appdata_models::Sutta;
use crate::db::dictionaries_models::DictWord;
use crate::db::DbManager;
use crate::logger::error;

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
        lang: String,
        query_text_orig: String,
        params: SearchParams,
        area: SearchArea,
    ) -> Self {
        let g = get_app_globals();
        SearchQueryTask {
            dbm,
            query_text: consistent_niggahita(Some(query_text_orig)),
            search_mode: params.mode,
            search_area: area,
            page_len: params.page_len.unwrap_or(g.page_len),
            lang,
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

        error(&format!("Can't create fragment, query not found: {}", query));

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
        let content = x.summary.as_deref()
            .filter(|s| !s.is_empty())
            .or(x.definition_plain.as_deref())
            .filter(|s| !s.is_empty())
            .or(x.definition_html.as_deref())
            .unwrap_or("");

        let snippet = self.fragment_around_query(&self.query_text, content);
        SearchResult::from_dict_word(x, snippet)
    }

    fn uid_sutta(&mut self) -> Result<Vec<SearchResult>, Box<dyn Error>> {
        use crate::db::appdata_schema::suttas::dsl::*;
        let app_data = get_app_data();
        let db_conn = &mut app_data.dbm.appdata.get_conn()?;

        let query_uid = self.query_text.to_lowercase().replace("uid:", "");

        let res = suttas
            .filter(uid.eq(query_uid))
            .select(Sutta::as_select())
            .first(db_conn);

        match res {
            Ok(sutta) => {
                Ok(vec![self.db_sutta_to_result(&sutta)])
            }
            Err(_) => {
                Ok(Vec::new())
            }
        }
    }

    fn uid_word(&mut self) -> Result<Vec<SearchResult>, Box<dyn Error>> {
        // TODO: review details in query_task.py
        use crate::db::dictionaries_schema::dict_words::dsl::*;
        let app_data = get_app_data();
        let db_conn = &mut app_data.dbm.dictionaries.get_conn()?;

        let query_uid = self.query_text.to_lowercase().replace("uid:", "");

        let res = dict_words
            .filter(uid.eq(query_uid))
            .select(DictWord::as_select())
            .first(db_conn);

        match res {
            Ok(res_word) => {
                Ok(vec![self.db_word_to_result(&res_word)])
            }
            Err(_) => {
                Ok(Vec::new())
            }
        }
    }

    /// Fetches a page of results for Suttas using CONTAINS or REGEX matching.
    fn suttas_contains_or_regex_match_page(
        &mut self,
        page_num: usize,
    ) -> Result<Vec<SearchResult>, Box<dyn Error>> {
        // TODO: review details in query_task.py
        use crate::db::appdata_schema::suttas::dsl::*;

        let app_data = get_app_data();
        let db_conn = &mut app_data.dbm.appdata.get_conn()?;

        // Box query for dynamic filtering
        let mut query = suttas.into_boxed();
        // A separate query for the total count. Can't clone the query before the offset limit.
        let mut count_query = suttas.into_boxed();

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

        Ok(search_results)
    }

    fn dict_words_contains_or_regex_match_page(
        &mut self,
        page_num: usize,
    ) -> Result<Vec<SearchResult>, Box<dyn Error>> {
        // TODO: review details in query_task.py
        use crate::db::dictionaries_schema::dict_words::dsl::*;

        let app_data = get_app_data();
        let db_conn = &mut app_data.dbm.dictionaries.get_conn()?;

        let mut query = dict_words.into_boxed();
        let mut count_query = dict_words.into_boxed();

        // --- Source Filtering ---
        if let Some(ref source_val) = self.source {
            let pattern = format!("%/{}", source_val);
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
                    query = query.filter(definition_plain.like(format!("%{}%", term)));
                    count_query = count_query.filter(definition_plain.like(format!("%{}%", term)));
                }
                SearchMode::RegExMatch => {
                    // FIXME use diesel regex match
                    query = query.filter(definition_plain.like(format!("%{}%", term)));
                    count_query = count_query.filter(definition_plain.like(format!("%{}%", term)));
                }
                _ => {
                    return Err(format!(
                        "Invalid search mode in dict_words_contains_or_regex_match_page: {:?}",
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
        let db_results: Vec<DictWord> = query.load::<DictWord>(db_conn)?;

        // --- Map to SearchResult ---
        let search_results = db_results
            .iter()
            .map(|sutta| self.db_word_to_result(sutta))
            .collect();

        Ok(search_results)

    }

    pub fn dpd_lookup(&mut self) -> Result<Vec<SearchResult>, Box<dyn Error>> {
        // DPD is English.
        if self.lang != "en" {
            return Ok(Vec::new());
        }

        let app_data = get_app_data();
        let res_page = app_data.dbm.dpd.dpd_lookup(&self.query_text, false, true)?;

        // FIXME implement paging in DPD lookup results.
        let limit_page = res_page[0..100].to_vec();

        Ok(limit_page)
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
            SearchMode::Combined => {
                let mut res: Vec<SearchResult> = Vec::new();

                // Display all DPD Lookup results (not many) on the
                // first (0 index) results page by boosting their scores.
                if page_num == 0 {
                    // Run DPD Lookup and boost results to the top.
                    let mut dpd_results: Vec<SearchResult> = self.dpd_lookup()?;
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
                Ok(unique_search_results(res))
            }

            SearchMode::UidMatch => {
                match self.search_area {
                    SearchArea::Suttas => {
                        self.uid_sutta()
                    }
                    SearchArea::DictWords => {
                        self.uid_word()
                    }
                }
            }

            SearchMode::ContainsMatch | SearchMode::RegExMatch => {
                match self.search_area {
                    SearchArea::Suttas => {
                        self.suttas_contains_or_regex_match_page(page_num)
                    }
                    SearchArea::DictWords => {
                        self.dict_words_contains_or_regex_match_page(page_num)
                    }
                }
            }

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
                // Re-highlight the snippet based on the full query text
                // Note: _db_sutta_to_result already created a basic snippet.
                // This step applies the final highlighting spans.
                result.snippet = self.highlight_query_in_content(&self.query_text, &result.snippet);
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
