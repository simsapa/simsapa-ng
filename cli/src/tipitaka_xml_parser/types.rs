//! Core data structures for the Tipitaka XML parser
//!
//! This module defines the types used throughout the parser for representing
//! XML fragments, group hierarchies, and nikaya structures.

use serde::{Deserialize, Serialize};

/// Type of XML fragment
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum FragmentType {
    /// Header fragment (contains metadata but not sutta content)
    Header,
    /// Sutta fragment (contains actual sutta text)
    Sutta,
}

/// Type of group in the hierarchy
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum GroupType {
    /// Nikaya level (e.g., Dīghanikāya)
    Nikaya,
    /// Book level (e.g., Sīlakkhandhavaggo)
    Book,
    /// Vagga level (e.g., Mūlapariyāyavaggo)
    Vagga,
    /// Samyutta level (e.g., Devatāsaṃyutta)
    Samyutta,
    /// Sutta level (e.g., Brahmajālasutta)
    Sutta,
}

/// Represents a level in the group hierarchy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupLevel {
    /// Type of this group level
    pub group_type: GroupType,
    /// Number/index of this group
    pub group_number: Option<i32>,
    /// Title of this group
    pub title: String,
    /// ID attribute (if present)
    pub id: Option<String>,
}

/// Represents a fragment of XML with associated metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct XmlFragment {
    /// Type of this fragment
    pub fragment_type: FragmentType,
    /// Raw XML content of this fragment
    pub content: String,
    /// Starting line number in source file (1-indexed)
    pub start_line: usize,
    /// Ending line number in source file (1-indexed)
    pub end_line: usize,
    /// Starting character position within start_line (0-indexed)
    pub start_char: usize,
    /// Ending character position within end_line (0-indexed, exclusive)
    pub end_char: usize,
    /// Hierarchy levels at the time this fragment was created
    pub group_levels: Vec<GroupLevel>,
}
