use serde::Deserialize;

use crate::types::{SearchArea, SearchMode};

/// Filters that can be applied to fulltext search.
#[derive(Debug, Default, Clone, Deserialize)]
pub struct SearchFilters {
    pub lang: Option<String>,
    pub lang_include: bool,
    pub source_uid: Option<String>,
    pub source_include: bool,
    pub nikaya: Option<String>,
    pub sutta_ref: Option<String>,
}

/// A single step in a composable search pipeline.
#[derive(Debug, Clone, Deserialize)]
pub struct SearchStep {
    pub mode: SearchMode,
    pub query_text: String,
    /// Filters applied at this step
    pub filters: SearchFilters,
}

/// A search pipeline is a list of steps, each narrowing the previous results.
/// For now, only single-step pipelines are supported.
#[derive(Debug, Clone, Deserialize)]
pub struct SearchPipeline {
    pub steps: Vec<SearchStep>,
    pub area: SearchArea,
    pub page_len: Option<usize>,
}
