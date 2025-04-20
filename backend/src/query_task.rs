// use std::any::Any;
use std::collections::HashMap;
use std::error::Error;

use regex::Regex;
use diesel::prelude::*;

use crate::PAGE_LEN;
use crate::types::{SearchArea, SearchMode, SearchParams, SearchResult};
use crate::helpers::consistent_niggahita;
use crate::models::Sutta;
use crate::db::establish_connection;

pub struct SearchQueryTask {
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

impl SearchQueryTask {
    pub fn new(
        lang: String,
        query_text_orig: String,
        params: SearchParams,
        area: SearchArea,
    ) -> Self {
        SearchQueryTask {
            query_text: consistent_niggahita(Some(query_text_orig)),
            search_mode: params.mode,
            search_area: area,
            page_len: params.page_len.unwrap_or(PAGE_LEN),
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
        // Escape regex special characters in the search term
        let escaped_term = regex::escape(term);
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
                    eprintln!("Regex error during highlighting: {}", e);
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

        eprintln!("Can't create fragment, query not found: {}", query);

        // If no terms are found, return the beginning of the content
        self.fragment_around_text("", content, before, after)
    }

    /// Helper to choose content (plain or HTML) and create a snippet.
    fn db_sutta_to_result(&self, sutta: &Sutta) -> SearchResult {
        let content = sutta.content_plain.as_deref() // Prefer plain text
            .filter(|s| !s.is_empty()) // Ensure it's not empty
            .or(sutta.content_html.as_deref()) // Fallback to HTML
            .unwrap_or(""); // Default to empty string if both are None/empty

        let snippet = self.fragment_around_query(&self.query_text, content);
        SearchResult::from_sutta(sutta, snippet)
    }

    /// Fetches a page of results for Suttas using CONTAINS or REGEX matching.
    fn suttas_contains_or_regex_match_page(
        &mut self,
        page_num: usize,
    ) -> Result<Vec<SearchResult>, Box<dyn Error>> {
        use crate::schema::suttas::dsl::*;

        let conn = &mut establish_connection();

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
        self.db_query_hits_count = count_query.select(diesel::dsl::count_star()).first(conn)?;

        // --- Apply Pagination ---
        let offset = (page_num * self.page_len) as i64;
        query = query.offset(offset).limit(self.page_len as i64);

        // --- Execute Query ---
        // println!("Executing Query: {:?}", diesel::debug_query::<diesel::sqlite::Sqlite, _>(&query));
        let db_results: Vec<Sutta> = query.load::<Sutta>(conn)?;

        // --- Map to SearchResult ---
        let search_results = db_results
            .iter()
            .map(|sutta| self.db_sutta_to_result(sutta))
            .collect();

        Ok(search_results)
    }

    /// Gets a specific page of search results, performing the query if needed.
    pub fn results_page(&mut self, page_num: usize) -> Result<Vec<SearchResult>, Box<dyn Error>> {
        // Check cache first
        if let Some(cached_page) = self.highlighted_result_pages.get(&page_num) {
            return Ok(cached_page.clone()); // Return a clone to avoid borrow issues
        }

        // --- Perform Search Based on Mode and Area ---
        let results = match self.search_mode {
            SearchMode::ContainsMatch | SearchMode::RegExMatch => {
                match self.search_area {
                    SearchArea::Suttas => {
                        self.suttas_contains_or_regex_match_page(page_num)
                    }
                    SearchArea::DictWords => {
                        // FIXME: implement later
                        eprintln!("Search area {:?} not yet implemented.", self.search_area);
                        self.db_query_hits_count = 0;
                        Ok(Vec::new())
                    }
                }
            }
            _ => {
                // FIXME: implement later
                eprintln!("Search mode {:?} not yet implemented.", self.search_mode);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{SearchParams, SearchArea, SearchMode};

    fn create_test_task(query_text: &str, search_mode: SearchMode) -> SearchQueryTask {
        let params = SearchParams {
            mode: search_mode,
            page_len: Some(PAGE_LEN),
            lang: Some("en".to_string()),
            lang_include: false,
            source: None,
            source_include: false,
            enable_regex: false,
            fuzzy_distance: 0,
        };

        SearchQueryTask::new(
            "en".to_string(),
            query_text.to_string(),
            params,
            SearchArea::Suttas,
        )
    }

    #[test]
    fn test_highlight_text_simple() {
        let task = create_test_task("satipaṭṭhā", SearchMode::ContainsMatch);
        let content = "sīlaṁ nissāya sīle patiṭṭhāya cattāro satipaṭṭhāne bhāveyyāsi";
        let highlighted = task.highlight_text(&task.query_text, content).unwrap();
        assert_eq!(highlighted, "sīlaṁ nissāya sīle patiṭṭhāya cattāro <span class='match'>satipaṭṭhā</span>ne bhāveyyāsi");
    }

    #[test]
    fn test_highlight_text_regex_special_chars() {
        let task = create_test_task("test", SearchMode::ContainsMatch);
        let content = "This has regex .*+ chars";
        let highlighted = task.highlight_text(".*+", content).unwrap();
        assert_eq!(highlighted, "This has regex <span class='match'>.*+</span> chars");
    }

    #[test]
    fn test_fragment_around_text_middle() {
        let task = create_test_task("satipaṭṭhā", SearchMode::ContainsMatch);
        let content = "sīlaṁ nissāya sīle patiṭṭhāya cattāro satipaṭṭhāne bhāveyyāsi";
        let fragment = task.fragment_around_text(&task.query_text, content, 10, 200);
        assert!(fragment.contains(&task.query_text));
        assert!(fragment.starts_with("... patiṭṭhāya cattāro satipaṭṭhāne"));
        assert!(fragment.ends_with("bhāveyyāsi ..."));
    }
}
