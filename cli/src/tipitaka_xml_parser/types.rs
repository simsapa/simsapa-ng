//! Data structures for representing Tipitaka XML hierarchical structure
//!
//! These types model the VRI CST Tipitaka XML format, which organizes texts
//! in a hierarchical structure: nikaya → book → vagga → sutta.

use serde::{Deserialize, Serialize};

/// Represents the full hierarchical structure of a nikaya (collection)
///
/// # Example
/// ```
/// TipitakaCollection {
///     nikaya: "Majjhimanikāyo".to_string(),
///     books: vec![...],
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TipitakaCollection {
    /// Name of the nikaya (e.g., "Majjhimanikāyo")
    pub nikaya: String,
    /// Books within this nikaya
    pub books: Vec<Book>,
}

/// Represents a book (paṇṇāsa) within a nikaya
///
/// # Example
/// ```
/// Book {
///     id: "mn1".to_string(),
///     title: "Mūlapaṇṇāsapāḷi".to_string(),
///     vaggas: vec![...],
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Book {
    /// Unique identifier from XML (e.g., "mn1")
    pub id: String,
    /// Title of the book
    pub title: String,
    /// Vaggas (chapters) within this book
    pub vaggas: Vec<Vagga>,
}

/// Represents a vagga (chapter) within a book
///
/// # Example
/// ```
/// Vagga {
///     id: "mn1_1".to_string(),
///     title: "1. Mūlapariyāyavaggo".to_string(),
///     suttas: vec![...],
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vagga {
    /// Unique identifier from XML (e.g., "mn1_1")
    pub id: String,
    /// Title of the vagga
    pub title: String,
    /// Suttas within this vagga
    pub suttas: Vec<Sutta>,
}

/// Represents a single sutta with its content and metadata
///
/// # Example
/// ```
/// Sutta {
///     title: "1. Mūlapariyāyasuttaṃ".to_string(),
///     content_xml: vec![...],
///     metadata: SuttaMetadata { ... },
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sutta {
    /// Title of the sutta
    pub title: String,
    /// Raw XML elements representing sutta content
    pub content_xml: Vec<XmlElement>,
    /// Metadata for database insertion
    pub metadata: SuttaMetadata,
}

/// Metadata for a sutta, used for database insertion
///
/// Contains information needed to populate the appdata schema.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuttaMetadata {
    /// Unique identifier (e.g., "vri-cst/mn1.1")
    pub uid: String,
    /// Reference string (e.g., "MN 1")
    pub sutta_ref: String,
    /// Nikaya name
    pub nikaya: String,
    /// Full hierarchy path (e.g., "Majjhimanikāyo/Mūlapaṇṇāsapāḷi/Mūlapariyāyavaggo")
    pub group_path: String,
    /// Index within the group
    pub group_index: Option<i32>,
    /// Order index for sorting
    pub order_index: Option<i32>,
}

/// XML content elements parsed from the source files
///
/// Represents the different types of XML elements found in VRI CST Tipitaka XML.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum XmlElement {
    /// A paragraph element with render attribute and optional number
    Paragraph {
        /// Render type (e.g., "bodytext", "nikaya", "centre")
        rend: String,
        /// Optional paragraph number
        n: Option<String>,
        /// Content nodes within the paragraph
        content: Vec<ContentNode>,
    },
    /// Highlighted text (hi element) with render attribute
    HighlightedText {
        /// Render type (e.g., "paranum", "dot", "bold")
        rend: String,
        /// Text content
        content: String,
    },
    /// A note/variant reading
    Note {
        /// Note content
        content: String,
    },
    /// Page break reference
    PageBreak {
        /// Edition identifier (e.g., "M", "V", "P", "T")
        ed: String,
        /// Page number
        n: String,
    },
}

/// Content nodes that can appear within paragraphs
///
/// Represents inline content within paragraph elements.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ContentNode {
    /// Plain text content
    Text(String),
    /// Highlighted inline text (rend attribute, text content)
    Hi(String, String),
    /// Inline note/variant
    Note(String),
    /// Inline page break reference
    PageBreak {
        /// Edition identifier
        ed: String,
        /// Page number
        n: String,
    },
}
