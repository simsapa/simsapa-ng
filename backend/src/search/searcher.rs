use std::collections::HashMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use anyhow::Result;
use tantivy::collector::TopDocs;
use tantivy::query::{BooleanQuery, Occur, QueryParser, TermQuery};
use tantivy::schema::{IndexRecordOption, Value};
use tantivy::{Index, IndexReader, Term};

use crate::logger::{info, warn};
use crate::types::SearchResult;
use crate::AppGlobalPaths;

use super::schema::{build_dict_schema, build_sutta_schema};
use super::tokenizer::register_tokenizers;
pub use super::types::SearchFilters;

/// Holds open indexes for fulltext searching.
pub struct FulltextSearcher {
    /// Map of language → (Index, IndexReader) for sutta indexes
    sutta_indexes: HashMap<String, (Index, IndexReader)>,
    /// Map of language → (Index, IndexReader) for dict_word indexes
    dict_indexes: HashMap<String, (Index, IndexReader)>,
}

/// Returned by `FulltextSearcher::debug_query()`: the formatted debug text
/// plus an optional parse-error message.
#[derive(Debug)]
pub struct DebugQueryResult {
    pub debug_text: String,
    pub parse_error: Option<String>,
}

impl FulltextSearcher {
    /// Open all available per-language indexes under the given paths.
    pub fn open(paths: &AppGlobalPaths) -> Result<Self> {
        let sutta_indexes = Self::open_indexes(&paths.suttas_index_dir, true)?;
        let dict_indexes = Self::open_indexes(&paths.dict_words_index_dir, false)?;

        info(&format!(
            "FulltextSearcher opened: {} sutta language indexes, {} dict language indexes",
            sutta_indexes.len(),
            dict_indexes.len()
        ));

        Ok(Self {
            sutta_indexes,
            dict_indexes,
        })
    }

    /// Scan a directory for per-language subdirectories and open each as a Tantivy index.
    fn open_indexes(base_dir: &Path, is_sutta: bool) -> Result<HashMap<String, (Index, IndexReader)>> {
        let mut map = HashMap::new();

        match base_dir.try_exists() {
            Ok(true) => {}
            _ => return Ok(map),
        }

        let entries = std::fs::read_dir(base_dir)?;
        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }

            let lang = match path.file_name().and_then(|n| n.to_str()) {
                Some(name) => name.to_string(),
                None => continue,
            };

            match Self::open_single_index(&path, &lang, is_sutta) {
                Ok((index, reader)) => {
                    map.insert(lang.clone(), (index, reader));
                }
                Err(e) => {
                    warn(&format!("Failed to open index at {}: {}", path.display(), e));
                }
            }
        }

        Ok(map)
    }

    fn open_single_index(dir: &Path, lang: &str, is_sutta: bool) -> Result<(Index, IndexReader)> {
        let schema = if is_sutta {
            build_sutta_schema(lang)
        } else {
            build_dict_schema(lang)
        };

        let mmap_dir = tantivy::directory::MmapDirectory::open(dir)?;
        let index = Index::open_or_create(mmap_dir, schema)?;
        register_tokenizers(&index, lang);

        let reader = index.reader()?;
        Ok((index, reader))
    }

    /// Run the named tokenizer on `text` and return the resulting tokens as a
    /// comma-separated string.
    pub fn tokenize_to_string(index: &Index, tokenizer_name: &str, text: &str) -> Result<String> {
        let mut tokenizer = index
            .tokenizers()
            .get(tokenizer_name)
            .ok_or_else(|| anyhow::anyhow!("tokenizer '{}' not registered", tokenizer_name))?;

        let mut stream = tokenizer.token_stream(text);
        let mut tokens: Vec<String> = Vec::new();
        while stream.advance() {
            tokens.push(stream.token().text.clone());
        }
        Ok(tokens.join(", "))
    }

    /// Build a human-readable debug report for the given query.
    ///
    /// For each relevant language index (respecting `filters.lang`) the report
    /// includes:
    /// - tokenization results for both `{lang}_stem` and `{lang}_normalize`
    /// - whether stemming changed any tokens
    /// - parsed query ASTs for `content` and `content_exact` fields
    /// - total document count
    ///
    /// Parse errors are captured but do **not** short-circuit: partial results
    /// (tokens, doc count) are still included.
    pub fn debug_query(&self, query_text: &str, filters: &SearchFilters) -> Result<DebugQueryResult> {
        let mut out = String::new();
        let mut first_parse_error: Option<String> = None;

        let indexes = &self.sutta_indexes;
        if indexes.is_empty() {
            return Ok(DebugQueryResult {
                debug_text: "No sutta indexes available.".to_string(),
                parse_error: None,
            });
        }

        // Determine which languages to search (same logic as search_indexes)
        let langs_to_search: Vec<&String> = if let Some(ref lang) = filters.lang {
            if filters.lang_include && !lang.is_empty() && lang != "Language" {
                indexes.keys().filter(|k| *k == lang).collect()
            } else {
                indexes.keys().collect()
            }
        } else {
            indexes.keys().collect()
        };

        let mut sorted_langs: Vec<&String> = langs_to_search;
        sorted_langs.sort();

        for lang in sorted_langs {
            let Some((index, reader)) = indexes.get(lang) else {
                continue;
            };

            writeln!(out, "=== Language: {} ===", lang)?;
            writeln!(out)?;

            // --- Tokenization ---
            let stem_name = format!("{}_stem", lang);
            let norm_name = format!("{}_normalize", lang);

            let stem_tokens = Self::tokenize_to_string(index, &stem_name, query_text)
                .unwrap_or_else(|e| format!("(error: {})", e));
            let norm_tokens = Self::tokenize_to_string(index, &norm_name, query_text)
                .unwrap_or_else(|e| format!("(error: {})", e));

            writeln!(out, "Tokens ({}_stem):      {}", lang, stem_tokens)?;
            writeln!(out, "Tokens ({}_normalize): {}", lang, norm_tokens)?;

            // Stemming effect analysis
            if stem_tokens != norm_tokens {
                writeln!(out, "Stemming effect: stemmed differs from exact")?;
            } else {
                writeln!(out, "Stemming effect: no change (stemmed == exact)")?;
            }
            writeln!(out)?;

            // --- Parsed queries ---
            let schema = index.schema();
            if let Ok(content_field) = schema.get_field("content") {
                let parser = QueryParser::for_index(index, vec![content_field]);
                match parser.parse_query(query_text) {
                    Ok(q) => writeln!(out, "Parsed query (content):\n{:#?}", q)?,
                    Err(e) => {
                        let err_msg = format!("{}", e);
                        writeln!(out, "Parsed query (content): ERROR: {}", err_msg)?;
                        if first_parse_error.is_none() {
                            first_parse_error = Some(err_msg);
                        }
                    }
                }
                writeln!(out)?;
            }

            if let Ok(content_exact_field) = schema.get_field("content_exact") {
                let parser = QueryParser::for_index(index, vec![content_exact_field]);
                match parser.parse_query(query_text) {
                    Ok(q) => writeln!(out, "Parsed query (content_exact):\n{:#?}", q)?,
                    Err(e) => {
                        let err_msg = format!("{}", e);
                        writeln!(out, "Parsed query (content_exact): ERROR: {}", err_msg)?;
                        if first_parse_error.is_none() {
                            first_parse_error = Some(err_msg);
                        }
                    }
                }
                writeln!(out)?;
            }

            // --- Doc count ---
            let num_docs = reader.searcher().num_docs();
            writeln!(out, "Total docs in index: {}", num_docs)?;
            writeln!(out)?;
        }

        Ok(DebugQueryResult {
            debug_text: out,
            parse_error: first_parse_error,
        })
    }

    /// Check if any sutta indexes are available.
    pub fn has_sutta_indexes(&self) -> bool {
        !self.sutta_indexes.is_empty()
    }

    /// Check if any dict indexes are available.
    pub fn has_dict_indexes(&self) -> bool {
        !self.dict_indexes.is_empty()
    }

    /// Search sutta indexes.
    pub fn search_suttas(
        &self,
        query_text: &str,
        filters: &SearchFilters,
        page_len: usize,
    ) -> Result<Vec<SearchResult>> {
        self.search_indexes(query_text, filters, page_len, &self.sutta_indexes, true)
    }

    /// Search dict_word indexes.
    pub fn search_dict_words(
        &self,
        query_text: &str,
        filters: &SearchFilters,
        page_len: usize,
    ) -> Result<Vec<SearchResult>> {
        self.search_indexes(query_text, filters, page_len, &self.dict_indexes, false)
    }

    fn search_indexes(
        &self,
        query_text: &str,
        filters: &SearchFilters,
        page_len: usize,
        indexes: &HashMap<String, (Index, IndexReader)>,
        is_sutta: bool,
    ) -> Result<Vec<SearchResult>> {
        if indexes.is_empty() {
            return Ok(Vec::new());
        }

        // Determine which languages to search
        let langs_to_search: Vec<&String> = if let Some(ref lang) = filters.lang {
            if filters.lang_include && !lang.is_empty() && lang != "Language" {
                // Only search the specified language
                indexes.keys().filter(|k| *k == lang).collect()
            } else {
                indexes.keys().collect()
            }
        } else {
            indexes.keys().collect()
        };

        // Collect results from all matching languages with scores
        let mut all_scored: Vec<(f32, SearchResult)> = Vec::new();

        for lang in langs_to_search {
            if let Some((index, reader)) = indexes.get(lang) {
                match self.search_single_index(query_text, filters, page_len, index, reader, is_sutta) {
                    Ok(scored_results) => {
                        all_scored.extend(scored_results);
                    }
                    Err(e) => {
                        warn(&format!("Fulltext search error for lang {}: {}", lang, e));
                    }
                }
            }
        }

        // Sort by score descending (interleaved by score, not grouped by language)
        all_scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

        // Truncate to page_len
        let results: Vec<SearchResult> = all_scored
            .into_iter()
            .take(page_len)
            .map(|(_, r)| r)
            .collect();

        Ok(results)
    }

    fn search_single_index(
        &self,
        query_text: &str,
        filters: &SearchFilters,
        page_len: usize,
        index: &Index,
        reader: &IndexReader,
        is_sutta: bool,
    ) -> Result<Vec<(f32, SearchResult)>> {
        let searcher = reader.searcher();
        let schema = index.schema();

        let content_field = schema.get_field("content")?;
        let content_exact_field = schema.get_field("content_exact")?;

        // Build dual-field query: content (Must) + content_exact (Should, boosted)
        let content_parser = QueryParser::for_index(index, vec![content_field]);
        let content_exact_parser = QueryParser::for_index(index, vec![content_exact_field]);

        let content_query = content_parser.parse_query(query_text)?;
        let content_exact_query = content_exact_parser.parse_query(query_text)?;

        let boosted_exact = tantivy::query::BoostQuery::new(
            Box::new(content_exact_query),
            2.0,
        );

        let mut subqueries: Vec<(Occur, Box<dyn tantivy::query::Query>)> = vec![
            (Occur::Must, Box::new(content_query)),
            (Occur::Should, Box::new(boosted_exact)),
        ];

        // Add filter term queries
        if is_sutta {
            Self::add_sutta_filters(&mut subqueries, filters, &schema)?;
        } else {
            Self::add_dict_filters(&mut subqueries, filters, &schema)?;
        }

        let combined_query = BooleanQuery::new(subqueries);

        let top_docs = searcher.search(&combined_query, &TopDocs::with_limit(page_len))?;

        let mut results = Vec::new();

        for (score, doc_address) in top_docs {
            let doc: tantivy::TantivyDocument = searcher.doc(doc_address)?;

            let result = if is_sutta {
                self.sutta_doc_to_result(&doc, &schema, score, query_text, index, &searcher, content_field)?
            } else {
                self.dict_doc_to_result(&doc, &schema, score, query_text, index, &searcher, content_field)?
            };

            results.push((score, result));
        }

        Ok(results)
    }

    fn add_sutta_filters(
        subqueries: &mut Vec<(Occur, Box<dyn tantivy::query::Query>)>,
        filters: &SearchFilters,
        schema: &tantivy::schema::Schema,
    ) -> Result<()> {
        if let Some(ref source) = filters.source_uid {
            if filters.source_include && !source.is_empty() {
                let field = schema.get_field("source_uid")?;
                let term = Term::from_field_text(field, source);
                subqueries.push((Occur::Must, Box::new(TermQuery::new(term, IndexRecordOption::Basic))));
            }
        }

        if let Some(ref nikaya) = filters.nikaya {
            if !nikaya.is_empty() {
                let field = schema.get_field("nikaya")?;
                let term = Term::from_field_text(field, nikaya);
                subqueries.push((Occur::Must, Box::new(TermQuery::new(term, IndexRecordOption::Basic))));
            }
        }

        if let Some(ref sutta_ref) = filters.sutta_ref {
            if !sutta_ref.is_empty() {
                let field = schema.get_field("sutta_ref")?;
                let term = Term::from_field_text(field, sutta_ref);
                subqueries.push((Occur::Must, Box::new(TermQuery::new(term, IndexRecordOption::Basic))));
            }
        }

        Ok(())
    }

    fn add_dict_filters(
        subqueries: &mut Vec<(Occur, Box<dyn tantivy::query::Query>)>,
        filters: &SearchFilters,
        schema: &tantivy::schema::Schema,
    ) -> Result<()> {
        if let Some(ref source) = filters.source_uid {
            if filters.source_include && !source.is_empty() {
                let field = schema.get_field("source_uid")?;
                let term = Term::from_field_text(field, source);
                subqueries.push((Occur::Must, Box::new(TermQuery::new(term, IndexRecordOption::Basic))));
            }
        }

        Ok(())
    }

    fn sutta_doc_to_result(
        &self,
        doc: &tantivy::TantivyDocument,
        schema: &tantivy::schema::Schema,
        score: f32,
        query_text: &str,
        index: &Index,
        searcher: &tantivy::Searcher,
        content_field: tantivy::schema::Field,
    ) -> Result<SearchResult> {
        let uid = Self::get_text_field(doc, schema, "uid");
        let title = Self::get_text_field(doc, schema, "title");
        let language = Self::get_text_field(doc, schema, "language");
        let source_uid = Self::get_text_field(doc, schema, "source_uid");
        let sutta_ref = Self::get_text_field(doc, schema, "sutta_ref");
        let nikaya = Self::get_text_field(doc, schema, "nikaya");

        let snippet = self.generate_snippet(index, searcher, content_field, query_text, doc)?;

        Ok(SearchResult {
            uid,
            schema_name: "appdata".to_string(),
            table_name: "suttas".to_string(),
            source_uid: Some(source_uid).filter(|s| !s.is_empty()),
            title,
            sutta_ref: Some(sutta_ref).filter(|s| !s.is_empty()),
            nikaya: Some(nikaya).filter(|s| !s.is_empty()),
            author: None,
            lang: Some(language).filter(|s| !s.is_empty()),
            snippet,
            page_number: None,
            score: Some(score),
            rank: None,
        })
    }

    fn dict_doc_to_result(
        &self,
        doc: &tantivy::TantivyDocument,
        schema: &tantivy::schema::Schema,
        score: f32,
        query_text: &str,
        index: &Index,
        searcher: &tantivy::Searcher,
        content_field: tantivy::schema::Field,
    ) -> Result<SearchResult> {
        let uid = Self::get_text_field(doc, schema, "uid");
        let word = Self::get_text_field(doc, schema, "word");
        let language = Self::get_text_field(doc, schema, "language");
        let source_uid = Self::get_text_field(doc, schema, "source_uid");

        let snippet = self.generate_snippet(index, searcher, content_field, query_text, doc)?;

        Ok(SearchResult {
            uid,
            schema_name: "appdata".to_string(),
            table_name: "dict_words".to_string(),
            source_uid: Some(source_uid).filter(|s| !s.is_empty()),
            title: word,
            sutta_ref: None,
            nikaya: None,
            author: None,
            lang: Some(language).filter(|s| !s.is_empty()),
            snippet,
            page_number: None,
            score: Some(score),
            rank: None,
        })
    }

    fn get_text_field(doc: &tantivy::TantivyDocument, schema: &tantivy::schema::Schema, field_name: &str) -> String {
        schema
            .get_field(field_name)
            .ok()
            .and_then(|f| doc.get_first(f))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string()
    }

    fn generate_snippet(
        &self,
        index: &Index,
        searcher: &tantivy::Searcher,
        content_field: tantivy::schema::Field,
        query_text: &str,
        doc: &tantivy::TantivyDocument,
    ) -> Result<String> {
        let query_parser = QueryParser::for_index(index, vec![content_field]);
        let query = query_parser.parse_query(query_text)?;

        let mut snippet_gen = tantivy::snippet::SnippetGenerator::create(searcher, &query, content_field)?;
        snippet_gen.set_max_num_chars(200);

        let snippet = snippet_gen.snippet_from_doc(doc);
        let html = snippet.to_html();

        // Post-process: <b> → <span class='match'>, </b> → </span>
        let processed = html
            .replace("<b>", "<span class='match'>")
            .replace("</b>", "</span>");

        Ok(processed)
    }
}

/// Get the path to the sutta index dir for inspection (e.g., checking if indexes exist).
pub fn sutta_index_dir(paths: &AppGlobalPaths) -> &PathBuf {
    &paths.suttas_index_dir
}

/// Get the path to the dict_words index dir.
pub fn dict_index_dir(paths: &AppGlobalPaths) -> &PathBuf {
    &paths.dict_words_index_dir
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::schema::build_sutta_schema;
    use super::super::tokenizer::register_tokenizers;
    use tantivy::doc;

    /// Create a temporary in-memory sutta index with one document for the given language.
    fn create_test_index(lang: &str) -> (Index, IndexReader) {
        let schema = build_sutta_schema(lang);
        let index = Index::create_in_ram(schema.clone());
        register_tokenizers(&index, lang);

        let mut writer = index.writer_with_num_threads(1, 15_000_000).unwrap();

        let uid = schema.get_field("uid").unwrap();
        let title = schema.get_field("title").unwrap();
        let language = schema.get_field("language").unwrap();
        let source_uid = schema.get_field("source_uid").unwrap();
        let sutta_ref = schema.get_field("sutta_ref").unwrap();
        let nikaya = schema.get_field("nikaya").unwrap();
        let content = schema.get_field("content").unwrap();
        let content_exact = schema.get_field("content_exact").unwrap();

        writer
            .add_document(doc!(
                uid => "sn12.2/pli/ms",
                title => "Vibhaṅgasutta",
                language => lang,
                source_uid => "ms",
                sutta_ref => "SN 12.2",
                nikaya => "sn",
                content => "Katamo ca bhikkhave jarāmaraṇaṁ. Yā tesaṁ tesaṁ sattānaṁ.",
                content_exact => "Katamo ca bhikkhave jarāmaraṇaṁ. Yā tesaṁ tesaṁ sattānaṁ."
            ))
            .unwrap();

        writer.commit().unwrap();

        let reader = index.reader().unwrap();
        (index, reader)
    }

    #[test]
    fn test_tokenize_to_string_stem() {
        let (index, _reader) = create_test_index("pli");
        let result = FulltextSearcher::tokenize_to_string(&index, "pli_stem", "bhikkhūnaṁ dhammo").unwrap();
        assert_eq!(result, "bhikkhu, dhamma");
    }

    #[test]
    fn test_tokenize_to_string_normalize() {
        let (index, _reader) = create_test_index("pli");
        let result = FulltextSearcher::tokenize_to_string(&index, "pli_normalize", "bhikkhūnaṁ dhammo").unwrap();
        // normalize: lowercase + niggahita norm + ascii fold, but no stemming
        assert!(result.contains("bhikkhunam"));
        assert!(result.contains("dhammo"));
    }

    #[test]
    fn test_tokenize_to_string_unknown_tokenizer() {
        let (index, _reader) = create_test_index("pli");
        let result = FulltextSearcher::tokenize_to_string(&index, "nonexistent", "test");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not registered"));
    }

    #[test]
    fn test_debug_query_basic() {
        let (index, reader) = create_test_index("pli");
        let mut sutta_indexes = HashMap::new();
        sutta_indexes.insert("pli".to_string(), (index, reader));

        let searcher = FulltextSearcher {
            sutta_indexes,
            dict_indexes: HashMap::new(),
        };

        let filters = SearchFilters {
            lang: None,
            lang_include: false,
            source_uid: None,
            source_include: false,
            nikaya: None,
            sutta_ref: None,
        };

        let result = searcher.debug_query("bhikkhave", &filters).unwrap();

        assert!(result.debug_text.contains("=== Language: pli ==="));
        assert!(result.debug_text.contains("Tokens (pli_stem):"));
        assert!(result.debug_text.contains("Tokens (pli_normalize):"));
        assert!(result.debug_text.contains("Parsed query (content):"));
        assert!(result.debug_text.contains("Parsed query (content_exact):"));
        assert!(result.debug_text.contains("Total docs in index: 1"));
        assert!(result.parse_error.is_none());
    }

    #[test]
    fn test_debug_query_invalid_query_partial_results() {
        let (index, reader) = create_test_index("pli");
        let mut sutta_indexes = HashMap::new();
        sutta_indexes.insert("pli".to_string(), (index, reader));

        let searcher = FulltextSearcher {
            sutta_indexes,
            dict_indexes: HashMap::new(),
        };

        let filters = SearchFilters {
            lang: None,
            lang_include: false,
            source_uid: None,
            source_include: false,
            nikaya: None,
            sutta_ref: None,
        };

        // Unbalanced quotes should cause a parse error but still return partial results
        let result = searcher.debug_query("\"unclosed quote", &filters).unwrap();

        // Tokens should still be present even if query parsing fails
        assert!(result.debug_text.contains("Tokens (pli_stem):"));
        assert!(result.debug_text.contains("Total docs in index: 1"));
        // Parse error should be reported
        assert!(result.parse_error.is_some());
    }

    #[test]
    fn test_debug_query_stemming_effect() {
        let (index, reader) = create_test_index("pli");
        let mut sutta_indexes = HashMap::new();
        sutta_indexes.insert("pli".to_string(), (index, reader));

        let searcher = FulltextSearcher {
            sutta_indexes,
            dict_indexes: HashMap::new(),
        };

        let filters = SearchFilters {
            lang: None,
            lang_include: false,
            source_uid: None,
            source_include: false,
            nikaya: None,
            sutta_ref: None,
        };

        // "bhikkhūnaṁ" should stem differently than normalize
        let result = searcher.debug_query("bhikkhūnaṁ", &filters).unwrap();
        assert!(result.debug_text.contains("Stemming effect: stemmed differs from exact"));
    }

    #[test]
    fn test_debug_query_no_indexes() {
        let searcher = FulltextSearcher {
            sutta_indexes: HashMap::new(),
            dict_indexes: HashMap::new(),
        };

        let filters = SearchFilters {
            lang: None,
            lang_include: false,
            source_uid: None,
            source_include: false,
            nikaya: None,
            sutta_ref: None,
        };

        let result = searcher.debug_query("test", &filters).unwrap();
        assert_eq!(result.debug_text, "No sutta indexes available.");
        assert!(result.parse_error.is_none());
    }
}
