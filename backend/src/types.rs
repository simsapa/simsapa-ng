use std::fs::File;
use std::io::Read;
use std::path::PathBuf;
use serde::{Serialize, Deserialize};
use std::str::FromStr;
use anyhow::Result;
use thiserror::Error;

use crate::models_appdata::Sutta;
use crate::models_dictionaries::DictWord;
use crate::models_dpd::{DpdHeadword, DpdRoot};

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
    DictWords,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SearchMode {
    FulltextMatch,
    ContainsMatch,
    HeadwordMatch,
    TitleMatch,
    DpdIdMatch,
    DpdLookup,
    Combined,
    UidMatch,
    RegExMatch,
}

#[derive(Debug, Clone)]
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
