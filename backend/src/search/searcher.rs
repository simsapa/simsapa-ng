use std::collections::HashMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use anyhow::Result;
use tantivy::collector::{Count, TopDocs};
use tantivy::query::{BooleanQuery, Occur, QueryParser, RegexQuery, TermQuery};
use tantivy::schema::{IndexRecordOption, Value};
use tantivy::{Index, IndexReader, Term};

use crate::logger::{info, warn};
use crate::types::SearchResult;
use crate::AppGlobalPaths;

use super::schema::{build_bold_definitions_schema, build_dict_schema, build_library_schema, build_sutta_schema};
use super::tokenizer::register_tokenizers;
pub use super::types::SearchFilters;

/// Identifies the type of index for schema selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum IndexType {
    Sutta,
    Dict,
    Library,
    BoldDefinitions,
}

/// Holds open indexes for fulltext searching.
pub struct FulltextSearcher {
    /// Map of language → (Index, IndexReader) for sutta indexes
    sutta_indexes: HashMap<String, (Index, IndexReader)>,
    /// Map of language → (Index, IndexReader) for dict_word indexes
    dict_indexes: HashMap<String, (Index, IndexReader)>,
    /// Map of language → (Index, IndexReader) for library book chapter indexes
    library_indexes: HashMap<String, (Index, IndexReader)>,
    /// Single DPD bold-definitions index (Pāli only), stored at the root of
    /// `bold_definitions_index_dir` rather than under a per-language subdir.
    bold_definitions_index: Option<(Index, IndexReader)>,
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
        let sutta_indexes = Self::open_indexes(&paths.suttas_index_dir, IndexType::Sutta)?;
        let dict_indexes = Self::open_indexes(&paths.dict_words_index_dir, IndexType::Dict)?;
        let library_indexes = Self::open_indexes(&paths.library_index_dir, IndexType::Library)?;
        let bold_definitions_index = Self::open_bold_definitions_index(&paths.bold_definitions_index_dir)?;

        info(&format!(
            "FulltextSearcher opened: {} sutta language indexes, {} dict language indexes, {} library language indexes, bold_definitions={}",
            sutta_indexes.len(),
            dict_indexes.len(),
            library_indexes.len(),
            bold_definitions_index.is_some(),
        ));

        Ok(Self {
            sutta_indexes,
            dict_indexes,
            library_indexes,
            bold_definitions_index,
        })
    }

    /// Open indexes from explicit directory paths (without needing AppGlobalPaths).
    ///
    /// Useful for CLI tools or tests that manage index directories directly.
    /// Pass an empty or non-existent path to skip sutta, dict, or library indexes.
    pub fn open_from_dirs(suttas_index_dir: &Path, dict_words_index_dir: &Path, library_index_dir: Option<&Path>) -> Result<Self> {
        let sutta_indexes = Self::open_indexes(suttas_index_dir, IndexType::Sutta)?;
        let dict_indexes = Self::open_indexes(dict_words_index_dir, IndexType::Dict)?;
        let library_indexes = if let Some(dir) = library_index_dir {
            Self::open_indexes(dir, IndexType::Library)?
        } else {
            HashMap::new()
        };

        Ok(Self {
            sutta_indexes,
            dict_indexes,
            library_indexes,
            bold_definitions_index: None,
        })
    }

    /// Open the single bold-definitions index (stored at the root of its dir,
    /// not under per-language subdirs since only Pāli is indexed).
    fn open_bold_definitions_index(dir: &Path) -> Result<Option<(Index, IndexReader)>> {
        match dir.try_exists() {
            Ok(true) => {}
            _ => return Ok(None),
        }

        // Require the tantivy meta.json to exist before attempting to open —
        // otherwise `Index::open_or_create` would silently initialise an empty
        // index against a stale directory.
        let meta = dir.join("meta.json");
        match meta.try_exists() {
            Ok(true) => {}
            _ => return Ok(None),
        }

        match Self::open_single_index(dir, "pli", IndexType::BoldDefinitions) {
            Ok(pair) => Ok(Some(pair)),
            Err(e) => {
                warn(&format!("Failed to open bold_definitions index at {}: {}", dir.display(), e));
                Ok(None)
            }
        }
    }

    /// Scan a directory for per-language subdirectories and open each as a Tantivy index.
    fn open_indexes(base_dir: &Path, index_type: IndexType) -> Result<HashMap<String, (Index, IndexReader)>> {
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

            match Self::open_single_index(&path, &lang, index_type) {
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

    fn open_single_index(dir: &Path, lang: &str, index_type: IndexType) -> Result<(Index, IndexReader)> {
        let schema = match index_type {
            IndexType::Sutta => build_sutta_schema(lang),
            IndexType::Dict => build_dict_schema(lang),
            IndexType::Library => build_library_schema(lang),
            IndexType::BoldDefinitions => build_bold_definitions_schema(lang),
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

    /// Search sutta indexes, returning (total_hits, results).
    pub fn search_suttas_with_count(
        &self,
        query_text: &str,
        filters: &SearchFilters,
        page_len: usize,
        page_num: usize,
    ) -> Result<(usize, Vec<SearchResult>)> {
        self.search_indexes(query_text, filters, page_len, page_num, &self.sutta_indexes, IndexType::Sutta, true)
    }

    /// Search dict_word indexes, returning (total_hits, results).
    pub fn search_dict_words_with_count(
        &self,
        query_text: &str,
        filters: &SearchFilters,
        page_len: usize,
        page_num: usize,
    ) -> Result<(usize, Vec<SearchResult>)> {
        self.search_indexes(query_text, filters, page_len, page_num, &self.dict_indexes, IndexType::Dict, true)
    }

    /// Search library indexes, returning (total_hits, results).
    pub fn search_library_with_count(
        &self,
        query_text: &str,
        filters: &SearchFilters,
        page_len: usize,
        page_num: usize,
    ) -> Result<(usize, Vec<SearchResult>)> {
        self.search_indexes(query_text, filters, page_len, page_num, &self.library_indexes, IndexType::Library, true)
    }

    /// Check if any library indexes are available.
    pub fn has_library_indexes(&self) -> bool {
        !self.library_indexes.is_empty()
    }

    /// Check if the bold-definitions index is available.
    pub fn has_bold_definitions_index(&self) -> bool {
        self.bold_definitions_index.is_some()
    }

    /// Search the DPD bold-definitions index, returning `(total_hits, results)`.
    ///
    /// Only the Pāli commentary content is indexed, so language filters in
    /// `filters` are ignored. If no index is available, returns `(0, vec![])`.
    pub fn search_bold_definitions_with_count(
        &self,
        query_text: &str,
        filters: &SearchFilters,
        page_len: usize,
        page_num: usize,
    ) -> Result<(usize, Vec<SearchResult>)> {
        let Some((index, reader)) = &self.bold_definitions_index else {
            return Ok((0, Vec::new()));
        };

        // Fetch enough results to cover all pages up to the requested one.
        let limit = (page_num + 1) * page_len;

        let (total_hits, scored) = self.search_single_index(
            query_text,
            filters,
            limit,
            index,
            reader,
            IndexType::BoldDefinitions,
            true,
        )?;

        let results: Vec<SearchResult> = scored
            .into_iter()
            .skip(page_num * page_len)
            .take(page_len)
            .map(|(_, r)| r)
            .collect();

        Ok((total_hits, results))
    }

    #[allow(clippy::too_many_arguments)]
    fn search_indexes(
        &self,
        query_text: &str,
        filters: &SearchFilters,
        page_len: usize,
        page_num: usize,
        indexes: &HashMap<String, (Index, IndexReader)>,
        index_type: IndexType,
        with_count: bool,
    ) -> Result<(usize, Vec<SearchResult>)> {
        if indexes.is_empty() {
            return Ok((0, Vec::new()));
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

        // Fetch enough results from each index to cover all pages up to the requested one
        let limit = (page_num + 1) * page_len;

        // Collect results from all matching languages with scores
        let mut all_scored: Vec<(f32, SearchResult)> = Vec::new();
        let mut total_hits: usize = 0;

        for lang in langs_to_search {
            if let Some((index, reader)) = indexes.get(lang) {
                match self.search_single_index(query_text, filters, limit, index, reader, index_type, with_count) {
                    Ok((count, scored_results)) => {
                        total_hits += count;
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

        // Extract the correct page
        let results: Vec<SearchResult> = all_scored
            .into_iter()
            .skip(page_num * page_len)
            .take(page_len)
            .map(|(_, r)| r)
            .collect();

        Ok((total_hits, results))
    }

    #[allow(clippy::too_many_arguments)]
    fn search_single_index(
        &self,
        query_text: &str,
        filters: &SearchFilters,
        page_len: usize,
        index: &Index,
        reader: &IndexReader,
        index_type: IndexType,
        with_count: bool,
    ) -> Result<(usize, Vec<(f32, SearchResult)>)> {
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
        match index_type {
            IndexType::Sutta => Self::add_sutta_filters(&mut subqueries, filters, &schema)?,
            IndexType::Dict => Self::add_dict_filters(&mut subqueries, filters, &schema)?,
            IndexType::Library => Self::add_library_filters(&mut subqueries, filters, &schema)?,
            IndexType::BoldDefinitions => {} // No additional filters for bold_definitions
        }

        let combined_query = BooleanQuery::new(subqueries);

        let (top_docs, count) = if with_count {
            searcher.search(&combined_query, &(TopDocs::with_limit(page_len), Count))?
        } else {
            let top_docs = searcher.search(&combined_query, &TopDocs::with_limit(page_len))?;
            (top_docs, 0)
        };

        // Build a single SnippetGenerator and reuse across all docs in this
        // call. Previously we constructed one per doc, which dominated runtime
        // for queries with hundreds-to-thousands of hits (e.g. fulltext +
        // suffix-filter where every candidate must be materialized for the
        // Rust-side post-filter).
        let snippet_gen = {
            let parser = QueryParser::for_index(index, vec![content_field]);
            let parsed = parser.parse_query(query_text)?;
            let mut g = tantivy::snippet::SnippetGenerator::create(&searcher, &parsed, content_field)?;
            g.set_max_num_chars(200);
            g
        };

        let mut results = Vec::with_capacity(top_docs.len());

        for (score, doc_address) in top_docs {
            let doc: tantivy::TantivyDocument = searcher.doc(doc_address)?;

            let result = match index_type {
                IndexType::Sutta => self.sutta_doc_to_result(&doc, &schema, score, &snippet_gen)?,
                IndexType::Dict => self.dict_doc_to_result(&doc, &schema, score, &snippet_gen)?,
                IndexType::Library => self.library_doc_to_result(&doc, &schema, score, &snippet_gen)?,
                IndexType::BoldDefinitions => self.bold_definition_doc_to_result(&doc, &schema, score, &snippet_gen)?,
            };

            results.push((score, result));
        }

        Ok((count, results))
    }

    fn add_sutta_filters(
        subqueries: &mut Vec<(Occur, Box<dyn tantivy::query::Query>)>,
        filters: &SearchFilters,
        schema: &tantivy::schema::Schema,
    ) -> Result<()> {
        if let Some(ref source) = filters.source_uid
            && filters.source_include && !source.is_empty()
        {
            let field = schema.get_field("source_uid")?;
            let term = Term::from_field_text(field, source);
            subqueries.push((Occur::Must, Box::new(TermQuery::new(term, IndexRecordOption::Basic))));
        }

        if let Some(ref nikaya) = filters.nikaya_prefix
            && !nikaya.is_empty()
        {
            let field = schema.get_field("nikaya")?;
            let pattern = format!("{}.*", regex::escape(&nikaya.to_lowercase()));
            let regex_query = RegexQuery::from_pattern(&pattern, field)?;
            subqueries.push((Occur::Must, Box::new(regex_query)));
        }

        if let Some(ref uid_prefix) = filters.uid_prefix
            && !uid_prefix.is_empty()
        {
            let field = schema.get_field("uid")?;
            let pattern = format!("{}.*", regex::escape(&uid_prefix.to_lowercase()));
            let regex_query = RegexQuery::from_pattern(&pattern, field)?;
            subqueries.push((Occur::Must, Box::new(regex_query)));
        }

        if let Some(ref sutta_ref) = filters.sutta_ref
            && !sutta_ref.is_empty()
        {
            let field = schema.get_field("sutta_ref")?;
            let term = Term::from_field_text(field, sutta_ref);
            subqueries.push((Occur::Must, Box::new(TermQuery::new(term, IndexRecordOption::Basic))));
        }

        // CST mula/commentary filtering: only exclude CST-sourced texts, not all sources.
        // This matches the SQL behavior in ContainsMatch which filters on uid LIKE '%/cst'.
        if !filters.include_cst_mula {
            let is_mula_field = schema.get_field("is_mula")?;
            let source_field = schema.get_field("source_uid")?;
            // Build: is_mula=true AND source_uid="cst"
            let cst_mula_query = BooleanQuery::new(vec![
                (Occur::Must, Box::new(TermQuery::new(Term::from_field_bool(is_mula_field, true), IndexRecordOption::Basic)) as Box<dyn tantivy::query::Query>),
                (Occur::Must, Box::new(TermQuery::new(Term::from_field_text(source_field, "cst"), IndexRecordOption::Basic))),
            ]);
            subqueries.push((Occur::MustNot, Box::new(cst_mula_query)));
        }

        // MS Mūla exclusion: exclude is_mula=true AND source_uid="ms" texts.
        if !filters.include_ms_mula {
            let is_mula_field = schema.get_field("is_mula")?;
            let source_field = schema.get_field("source_uid")?;
            let ms_mula_query = BooleanQuery::new(vec![
                (Occur::Must, Box::new(TermQuery::new(Term::from_field_bool(is_mula_field, true), IndexRecordOption::Basic)) as Box<dyn tantivy::query::Query>),
                (Occur::Must, Box::new(TermQuery::new(Term::from_field_text(source_field, "ms"), IndexRecordOption::Basic))),
            ]);
            subqueries.push((Occur::MustNot, Box::new(ms_mula_query)));
        }

        if !filters.include_cst_commentary {
            let is_commentary_field = schema.get_field("is_commentary")?;
            let source_field = schema.get_field("source_uid")?;
            // Build: is_commentary=true AND source_uid="cst"
            let cst_commentary_query = BooleanQuery::new(vec![
                (Occur::Must, Box::new(TermQuery::new(Term::from_field_bool(is_commentary_field, true), IndexRecordOption::Basic)) as Box<dyn tantivy::query::Query>),
                (Occur::Must, Box::new(TermQuery::new(Term::from_field_text(source_field, "cst"), IndexRecordOption::Basic))),
            ]);
            subqueries.push((Occur::MustNot, Box::new(cst_commentary_query)));
        }

        Ok(())
    }

    fn add_dict_filters(
        subqueries: &mut Vec<(Occur, Box<dyn tantivy::query::Query>)>,
        filters: &SearchFilters,
        schema: &tantivy::schema::Schema,
    ) -> Result<()> {
        if let Some(ref source) = filters.source_uid
            && filters.source_include && !source.is_empty()
        {
            let field = schema.get_field("source_uid")?;
            let term = Term::from_field_text(field, source);
            subqueries.push((Occur::Must, Box::new(TermQuery::new(term, IndexRecordOption::Basic))));
        }

        // uid is tokenized via simple_fold, so this push-down can over-match
        // (a non-prefix token equal to the search prefix slips through). The
        // Rust-side `apply_uid_filters` is the source of truth; this filter
        // is a pure narrowing optimization to keep the candidate set small.
        if let Some(ref uid_prefix) = filters.uid_prefix
            && !uid_prefix.is_empty()
        {
            let field = schema.get_field("uid")?;
            let pattern = format!("{}.*", regex::escape(&uid_prefix.to_lowercase()));
            let regex_query = RegexQuery::from_pattern(&pattern, field)?;
            subqueries.push((Occur::Must, Box::new(regex_query)));
        }

        Ok(())
    }

    fn add_library_filters(
        subqueries: &mut Vec<(Occur, Box<dyn tantivy::query::Query>)>,
        filters: &SearchFilters,
        schema: &tantivy::schema::Schema,
    ) -> Result<()> {
        // spine_item_uid is tokenized via simple_fold; same caveat as
        // add_dict_filters — Rust-side filter remains authoritative.
        if let Some(ref uid_prefix) = filters.uid_prefix
            && !uid_prefix.is_empty()
        {
            let field = schema.get_field("spine_item_uid")?;
            let pattern = format!("{}.*", regex::escape(&uid_prefix.to_lowercase()));
            let regex_query = RegexQuery::from_pattern(&pattern, field)?;
            subqueries.push((Occur::Must, Box::new(regex_query)));
        }

        Ok(())
    }

    fn sutta_doc_to_result(
        &self,
        doc: &tantivy::TantivyDocument,
        schema: &tantivy::schema::Schema,
        score: f32,
        snippet_gen: &tantivy::snippet::SnippetGenerator,
    ) -> Result<SearchResult> {
        let uid = Self::get_text_field(doc, schema, "uid");
        let title = Self::get_text_field(doc, schema, "title");
        let language = Self::get_text_field(doc, schema, "language");
        let source_uid = Self::get_text_field(doc, schema, "source_uid");
        let sutta_ref = Self::get_text_field(doc, schema, "sutta_ref");
        let nikaya = Self::get_text_field(doc, schema, "nikaya");

        let snippet = Self::render_snippet(snippet_gen, doc);

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
        snippet_gen: &tantivy::snippet::SnippetGenerator,
    ) -> Result<SearchResult> {
        let uid = Self::get_text_field(doc, schema, "uid");
        let word = Self::get_text_field(doc, schema, "word");
        let language = Self::get_text_field(doc, schema, "language");
        let source_uid = Self::get_text_field(doc, schema, "source_uid");

        let snippet = Self::render_snippet(snippet_gen, doc);

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

    fn library_doc_to_result(
        &self,
        doc: &tantivy::TantivyDocument,
        schema: &tantivy::schema::Schema,
        score: f32,
        snippet_gen: &tantivy::snippet::SnippetGenerator,
    ) -> Result<SearchResult> {
        let uid = Self::get_text_field(doc, schema, "spine_item_uid");
        let title = Self::get_text_field(doc, schema, "title");
        let book_title = Self::get_text_field(doc, schema, "book_title");
        let author = Self::get_text_field(doc, schema, "author");
        let language = Self::get_text_field(doc, schema, "language");

        // Use book_title as source_uid for display purposes
        let display_title = if !book_title.is_empty() && !title.is_empty() {
            format!("{} — {}", book_title, title)
        } else if !title.is_empty() {
            title
        } else {
            book_title.clone()
        };

        let snippet = Self::render_snippet(snippet_gen, doc);

        Ok(SearchResult {
            uid,
            schema_name: "appdata".to_string(),
            table_name: "book_spine_items".to_string(),
            source_uid: Some(book_title).filter(|s| !s.is_empty()),
            title: display_title,
            sutta_ref: None,
            nikaya: None,
            author: Some(author).filter(|s| !s.is_empty()),
            lang: Some(language).filter(|s| !s.is_empty()),
            snippet,
            page_number: None,
            score: Some(score),
            rank: None,
        })
    }

    fn bold_definition_doc_to_result(
        &self,
        doc: &tantivy::TantivyDocument,
        schema: &tantivy::schema::Schema,
        score: f32,
        snippet_gen: &tantivy::snippet::SnippetGenerator,
    ) -> Result<SearchResult> {
        let uid = Self::get_text_field(doc, schema, "uid");
        let bold = Self::get_text_field(doc, schema, "bold");
        let ref_code = Self::get_text_field(doc, schema, "ref_code");
        let nikaya = Self::get_text_field(doc, schema, "nikaya");

        let snippet = Self::render_snippet(snippet_gen, doc);

        Ok(SearchResult {
            uid,
            schema_name: "dpd".to_string(),
            table_name: "bold_definitions".to_string(),
            // In bold_definitions, the equivalent of source_uid is the ref_code field (e.g. vina, mna, vvt)
            source_uid: Some(ref_code.clone()),
            title: bold,
            // ref_code also serves as the sutta_ref (Vinaya, Majjhima, etc. origin)
            sutta_ref: Some(ref_code),
            nikaya: Some(nikaya).filter(|s| !s.is_empty()),
            author: None,
            lang: Some("pli".to_string()),
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

    /// Render a single document's snippet using a pre-built `SnippetGenerator`.
    /// The generator is constructed once per `search_single_index` call —
    /// constructing it per-doc dominated runtime for queries returning many
    /// hits.
    fn render_snippet(
        snippet_gen: &tantivy::snippet::SnippetGenerator,
        doc: &tantivy::TantivyDocument,
    ) -> String {
        let snippet = snippet_gen.snippet_from_doc(doc);
        snippet
            .to_html()
            .replace("<b>", "<span class='match'>")
            .replace("</b>", "</span>")
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
            library_indexes: HashMap::new(),
            bold_definitions_index: None,
        };

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
            library_indexes: HashMap::new(),
            bold_definitions_index: None,
        };

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
            library_indexes: HashMap::new(),
            bold_definitions_index: None,
        };

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

        // "bhikkhūnaṁ" should stem differently than normalize
        let result = searcher.debug_query("bhikkhūnaṁ", &filters).unwrap();
        assert!(result.debug_text.contains("Stemming effect: stemmed differs from exact"));
    }

    #[test]
    fn test_ascii_query_matches_pali_text() {
        // ASCII "bhikkhave" should match Pāli "bhikkhave" in the test document
        let (index, reader) = create_test_index("pli");
        let mut sutta_indexes = HashMap::new();
        sutta_indexes.insert("pli".to_string(), (index, reader));

        let searcher = FulltextSearcher {
            sutta_indexes,
            dict_indexes: HashMap::new(),
            library_indexes: HashMap::new(),
            bold_definitions_index: None,
        };

        let filters = SearchFilters {
            lang: Some("pli".to_string()),
            lang_include: true,
            source_uid: None,
            source_include: false,
            nikaya_prefix: None,
            uid_prefix: None,
            sutta_ref: None,
            include_cst_mula: true,
            include_cst_commentary: true,
            include_ms_mula: true,
        };

        // "sattanam" is the ASCII-folded form of "sattānaṁ" in the test document.
        // The stemmer now operates on ASCII input, so it stems "sattanam" → "satta"
        // matching the indexed stem of "sattānaṁ" → fold → "sattanam" → stem → "satta".
        let (count, results) = searcher.search_suttas_with_count("sattanam", &filters, 10, 0).unwrap();
        assert!(count > 0, "ASCII query 'sattanam' should match Pāli 'sattānaṁ'");
        assert!(!results.is_empty());
        assert_eq!(results[0].uid, "sn12.2/pli/ms");
    }

    #[test]
    fn test_ascii_query_jaramaranam() {
        // "jaramaranam" should match "jarāmaraṇaṁ"
        let (index, reader) = create_test_index("pli");
        let mut sutta_indexes = HashMap::new();
        sutta_indexes.insert("pli".to_string(), (index, reader));

        let searcher = FulltextSearcher {
            sutta_indexes,
            dict_indexes: HashMap::new(),
            library_indexes: HashMap::new(),
            bold_definitions_index: None,
        };

        let filters = SearchFilters {
            lang: Some("pli".to_string()),
            lang_include: true,
            source_uid: None,
            source_include: false,
            nikaya_prefix: None,
            uid_prefix: None,
            sutta_ref: None,
            include_cst_mula: true,
            include_cst_commentary: true,
            include_ms_mula: true,
        };

        let (count, results) = searcher.search_suttas_with_count("jaramaranam", &filters, 10, 0).unwrap();
        assert!(count > 0, "ASCII query 'jaramaranam' should match Pāli 'jarāmaraṇaṁ'");
        assert!(!results.is_empty());
    }

    #[test]
    fn test_ascii_query_with_declensions() {
        // "vinnanam" should match documents containing multiple declensions of viññāṇa
        let lang = "pli";
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

        let text = "viññāṇaṁ viññāṇena viññāṇassa viññāṇānaṁ";
        writer
            .add_document(doc!(
                uid => "sn12.1/pli/ms",
                title => "Test",
                language => lang,
                source_uid => "ms",
                sutta_ref => "SN 12.1",
                nikaya => "sn",
                content => text,
                content_exact => text
            ))
            .unwrap();

        writer.commit().unwrap();

        let reader = index.reader().unwrap();
        let mut sutta_indexes = HashMap::new();
        sutta_indexes.insert("pli".to_string(), (index, reader));

        let searcher = FulltextSearcher {
            sutta_indexes,
            dict_indexes: HashMap::new(),
            library_indexes: HashMap::new(),
            bold_definitions_index: None,
        };

        let filters = SearchFilters {
            lang: Some("pli".to_string()),
            lang_include: true,
            source_uid: None,
            source_include: false,
            nikaya_prefix: None,
            uid_prefix: None,
            sutta_ref: None,
            include_cst_mula: true,
            include_cst_commentary: true,
            include_ms_mula: true,
        };

        let (count, results) = searcher.search_suttas_with_count("vinnanam", &filters, 10, 0).unwrap();
        assert!(count > 0, "ASCII query 'vinnanam' should match documents with viññāṇa declensions");
        assert!(!results.is_empty());
    }

    #[test]
    fn test_debug_query_no_indexes() {
        let searcher = FulltextSearcher {
            sutta_indexes: HashMap::new(),
            dict_indexes: HashMap::new(),
            library_indexes: HashMap::new(),
            bold_definitions_index: None,
        };

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

        let result = searcher.debug_query("test", &filters).unwrap();
        assert_eq!(result.debug_text, "No sutta indexes available.");
        assert!(result.parse_error.is_none());
    }
}
