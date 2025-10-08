use std::fs::File;
use std::io::Read;
use std::path::PathBuf;
use serde::{Serialize, Deserialize};
use std::str::FromStr;
use anyhow::Result;
use thiserror::Error;

use crate::db::appdata_models::Sutta;
use crate::db::dictionaries_models::DictWord;
use crate::db::dpd_models::{DpdHeadword, DpdRoot};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum QueryType {
    #[serde(rename = "suttas")]
    Suttas,
    #[serde(rename = "words")]
    Words,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum QuoteScope {
    #[serde(rename = "sutta")]
    Sutta,
    #[serde(rename = "nikaya")]
    Nikaya,
    #[serde(rename = "all")]
    All,
}

// Custom error for parsing QuoteScope from string
#[derive(Error, Debug, PartialEq, Eq)]
#[error("Invalid QuoteScope value: {0}")]
pub struct ParseQuoteScopeError(String);

// Implement FromStr to parse strings into QuoteScope
impl FromStr for QuoteScope {
    type Err = ParseQuoteScopeError;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "sutta" => Ok(QuoteScope::Sutta),
            "nikaya" => Ok(QuoteScope::Nikaya),
            "all" => Ok(QuoteScope::All),
            _ => Err(ParseQuoteScopeError(s.to_string())),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuttaQuote {
    pub quote: String,
    pub selection_range: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SearchArea {
    Suttas,
    Dictionary,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub enum SearchMode {
    FulltextMatch,
    ContainsMatch,
    HeadwordMatch,
    TitleMatch,
    DpdIdMatch,
    #[serde(rename = "DPD Lookup")]
    DpdLookup,
    Combined,
    UidMatch,
    RegExMatch,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct SearchParams {
    pub mode: SearchMode,
    pub page_len: Option<usize>,
    pub lang: Option<String>,
    pub lang_include: bool,
    pub source: Option<String>,
    pub source_include: bool,
    pub enable_regex: bool,
    pub fuzzy_distance: i32,
}

impl Default for SearchParams {
    fn default() -> Self {
        SearchParams {
            mode: SearchMode::ContainsMatch,
            page_len: None,
            lang: Some("en".to_string()),
            lang_include: true,
            source: None,
            source_include: true,
            enable_regex: false,
            fuzzy_distance: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub uid: String,
    // database schema name (appdata or userdata)
    pub schema_name: String,
    // database table name (e.g. suttas or dict_words)
    pub table_name: String,
    pub source_uid: Option<String>,
    pub title: String,
    pub sutta_ref: Option<String>,
    pub nikaya: Option<String>,
    pub author: Option<String>,
    // highlighted snippet
    pub snippet: String,
    // page number in a document
    pub page_number: Option<i32>,
    pub score: Option<f32>,
    pub rank: Option<i32>,
}

impl SearchResult {
    pub fn load_from_json(path: &PathBuf) -> Result<Vec<Self>, String> {
        let mut file = match File::open(path) {
            Ok(file) => file,
            Err(e) => return Err(format!("Failed to open file: {}", e)),
        };

        let mut contents = String::new();
        match file.read_to_string(&mut contents) {
            Ok(_) => (),
            Err(e) => return Err(format!("Failed to read file: {}", e)),
        }

        match serde_json::from_str(&contents) {
            Ok(results) => Ok(results),
            Err(e) => Err(format!("Failed to parse JSON: {}", e)),
        }
    }

    pub fn from_sutta(sutta: &Sutta, snippet: String) -> SearchResult {
        SearchResult {
            uid: sutta.uid.to_string(),
            schema_name: "appdata".to_string(), // FIXME: implement later
            table_name: "suttas".to_string(), // TODO: can we get the table name from diesel?
            source_uid: sutta.source_uid.clone(),
            title: sutta.title.clone().unwrap_or_default(),
            sutta_ref: Some(sutta.sutta_ref.clone()),
            nikaya: Some(sutta.nikaya.clone()),
            author: None,
            snippet,
            page_number: None,
            score: None,
            rank: None,
        }
    }

    pub fn from_dict_word(x: &DictWord, snippet: String) -> SearchResult {
        // From dict_word_to_search_result()
        SearchResult {
            uid: x.uid.to_string(),
            schema_name: "appdata".to_string(), // FIXME: implement later
            table_name: "dict_words".to_string(), // TODO: can we get the table name from diesel?
            source_uid: x.source_uid.clone(),
            title: x.word.clone(),
            sutta_ref: None,
            nikaya: None,
            author: None,
            snippet,
            page_number: None,
            score: None,
            rank: None,
        }
    }

    pub fn from_title_str(title: &str) -> SearchResult {
        SearchResult {
            uid: title.to_string(),
            schema_name: "".to_string(),
            table_name: "".to_string(),
            source_uid: Some(title.to_string()),
            title: title.to_string(),
            sutta_ref: None,
            nikaya: None,
            author: None,
            snippet: "".to_string(),
            page_number: None,
            score: None,
            rank: None,
        }
    }

    pub fn from_dpd_headword(x: &DpdHeadword, snippet: String) -> SearchResult {
        // FIXME: use UDpdWord enum
        // From dict_word_to_search_result()
        SearchResult {
            uid: x.uid.to_string(),
            schema_name: "dpd".to_string(), // FIXME: implement later
            table_name: "dpd_headwords".to_string(), // TODO: can we get the table name from diesel?
            source_uid: Some("dpd".to_string()), // TODO implement .source_uid()
            title: x.word(),
            sutta_ref: None,
            nikaya: None,
            author: None,
            snippet,
            page_number: None,
            score: None,
            rank: None,
        }
    }

    pub fn from_dpd_root(x: &DpdRoot, snippet: String) -> SearchResult {
        // FIXME: use UDpdWord enum
        // From dict_word_to_search_result()
        SearchResult {
            uid: x.uid.to_string(),
            schema_name: "dpd".to_string(), // FIXME: implement later
            table_name: "dpd_roots".to_string(), // TODO: can we get the table name from diesel?
            source_uid: Some("dpd".to_string()), // TODO implement .source_uid()
            title: x.word(),
            sutta_ref: None,
            nikaya: None,
            author: None,
            snippet,
            page_number: None,
            score: None,
            rank: None,
        }
    }

}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResultPage {
    pub total_hits: usize,
    pub page_len: usize,
    pub page_num: usize,
    pub results: Vec<SearchResult>,
}

/// Options for word processing in gloss operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WordProcessingOptions {
    pub no_duplicates_globally: bool,
    pub skip_common: bool,
    pub common_words: Vec<String>,
    pub existing_global_stems: std::collections::HashMap<String, bool>,
    pub existing_paragraph_unrecognized: std::collections::HashMap<String, Vec<String>>,
    pub existing_global_unrecognized: Vec<String>,
}

/// Information about a word for processing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WordInfo {
    pub word: String,
    pub sentence: String,
}

/// Result of processing a single word for glossing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessedWord {
    pub original_word: String,
    pub results: Vec<crate::db::dpd::LookupResult>, // DPD lookup results
    pub selected_index: i32,
    pub stem: String,
    pub example_sentence: String,
}

/// Result indicating an unrecognized word
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnrecognizedWord {
    pub is_unrecognized: bool,
    pub word: String,
}

/// Result of processing a word (either recognized or unrecognized)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum WordProcessingResult {
    Recognized(ProcessedWord),
    Unrecognized(UnrecognizedWord),
    Skipped, // For common words or duplicates
}

/// Input data for processing all paragraphs in background
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AllParagraphsProcessingInput {
    pub paragraphs: Vec<String>,
    pub options: WordProcessingOptions,
}

/// Input data for processing a single paragraph in background
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SingleParagraphProcessingInput {
    pub paragraph_text: String,
    pub options: WordProcessingOptions,
}

/// Result data for a single paragraph processing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParagraphProcessingResult {
    pub paragraph_index: usize,
    pub words_data: Vec<ProcessedWord>,
    pub unrecognized_words: Vec<String>,
}

/// Result data for all paragraphs processing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AllParagraphsProcessingResult {
    pub success: bool,
    pub paragraphs: Vec<ParagraphProcessingResult>,
    pub global_unrecognized_words: Vec<String>,
    pub updated_global_stems: std::collections::HashMap<String, bool>,
}

/// Result data for single paragraph processing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SingleParagraphProcessingResult {
    pub success: bool,
    pub paragraph_index: usize,
    pub words_data: Vec<ProcessedWord>,
    pub unrecognized_words: Vec<String>,
    pub updated_global_stems: std::collections::HashMap<String, bool>,
}

/// Error response for background processing operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackgroundProcessingError {
    pub success: bool,
    pub error: String,
}

/// Input data for Anki CSV export in background
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnkiCsvExportInput {
    pub gloss_data_json: String,
    pub export_format: String,
    pub include_cloze: bool,
    pub templates: AnkiCsvTemplates,
}

/// Anki CSV templates
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnkiCsvTemplates {
    pub front: String,
    pub back: String,
    pub cloze_front: String,
    pub cloze_back: String,
}

/// Result data for Anki CSV export
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnkiCsvExportResult {
    pub success: bool,
    pub files: Vec<AnkiCsvFile>,
    pub error: Option<String>,
}

/// A single Anki CSV file result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnkiCsvFile {
    pub filename: String,
    pub content: String,
}
