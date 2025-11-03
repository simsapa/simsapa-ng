//! Core data structures for the Tipitaka XML parser
//!
//! This module defines the types used throughout the parser for representing
//! XML fragments, group hierarchies, and nikaya structures.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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
    /// Source XML filename for tracking which file this fragment came from. Corresponds to cst_file in TsvRecord.
    pub xml_filename: String,
    /// Index of this fragment in the list of fragments parsed from the XML file (0-indexed)
    pub frag_idx: usize,
    /// CST file identifier (derived from xml_filename)
    pub cst_file: Option<String>,
    /// CST code (e.g., "dn1.1", "mn1.5.1")
    pub cst_code: Option<String>,
    /// CST vagga title (e.g., "5. Cūḷayamakavaggo")
    pub cst_vagga: Option<String>,
    /// CST sutta title (e.g., "1. Brahmajālasuttaṃ")
    pub cst_sutta: Option<String>,
    /// CST paragraph number (from first <p rend="bodytext" n="...">)
    pub cst_paranum: Option<String>,
    /// SuttaCentral code (e.g., "dn1", "mn41")
    pub sc_code: Option<String>,
    /// SuttaCentral sutta title (e.g., "Brahmajālasutta")
    pub sc_sutta: Option<String>,
}

/// Manual adjustment for a specific fragment
#[derive(Debug, Clone)]
pub struct FragmentAdjustment {
    /// XML filename
    pub xml_filename: String,
    /// Fragment index (0-indexed)
    pub frag_idx: usize,
    /// Override end line (1-indexed)
    pub end_line: Option<usize>,
    /// Override end character position (0-indexed)
    pub end_char: Option<usize>,
}

/// Key for looking up fragment adjustments
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FragmentKey {
    pub xml_filename: String,
    pub frag_idx: usize,
}

/// Container for fragment adjustments loaded from TSV
pub type FragmentAdjustments = HashMap<FragmentKey, FragmentAdjustment>;

use anyhow::{Context, Result};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

/// Load fragment adjustments from a TSV file
///
/// The TSV file should have a header line with at least these fields:
/// - xml_filename: Name of the XML file
/// - frag_idx: Fragment index (0-indexed)
/// - end_line: (Optional) Override end line number (1-indexed)
/// - end_char: (Optional) Override end character position (0-indexed)
///
/// # Arguments
/// * `tsv_path` - Path to the TSV file
///
/// # Returns
/// HashMap mapping (xml_filename, frag_idx) to FragmentAdjustment
pub fn load_fragment_adjustments(tsv_path: &Path) -> Result<FragmentAdjustments> {
    let file = File::open(tsv_path)
        .with_context(|| format!("Failed to open adjustments TSV file: {:?}", tsv_path))?;
    
    let reader = BufReader::new(file);
    let mut lines = reader.lines();
    
    // Read header line
    let header = lines.next()
        .ok_or_else(|| anyhow::anyhow!("TSV file is empty"))?
        .context("Failed to read header line")?;
    
    // Parse header to find column indices
    let columns: Vec<&str> = header.split('\t').collect();
    let xml_filename_idx = columns.iter().position(|&c| c == "xml_filename")
        .ok_or_else(|| anyhow::anyhow!("Missing 'xml_filename' column in TSV header"))?;
    let frag_idx_col = columns.iter().position(|&c| c == "frag_idx")
        .ok_or_else(|| anyhow::anyhow!("Missing 'frag_idx' column in TSV header"))?;
    let end_line_idx = columns.iter().position(|&c| c == "end_line");
    let end_char_idx = columns.iter().position(|&c| c == "end_char");
    
    let mut adjustments = FragmentAdjustments::new();
    
    // Parse data lines
    for (line_num, line_result) in lines.enumerate() {
        let line = line_result
            .with_context(|| format!("Failed to read line {} of TSV file", line_num + 2))?;
        
        // Skip empty lines
        if line.trim().is_empty() {
            continue;
        }
        
        let fields: Vec<&str> = line.split('\t').collect();
        
        // Extract xml_filename
        let xml_filename = fields.get(xml_filename_idx)
            .ok_or_else(|| anyhow::anyhow!("Missing xml_filename field on line {}", line_num + 2))?
            .trim()
            .to_string();
        
        // Extract frag_idx
        let frag_idx_str = fields.get(frag_idx_col)
            .ok_or_else(|| anyhow::anyhow!("Missing frag_idx field on line {}", line_num + 2))?
            .trim();
        let frag_idx: usize = frag_idx_str.parse()
            .with_context(|| format!("Invalid frag_idx '{}' on line {}", frag_idx_str, line_num + 2))?;
        
        // Extract end_line if present
        let end_line = if let Some(idx) = end_line_idx {
            fields.get(idx)
                .and_then(|s| {
                    let trimmed = s.trim();
                    if trimmed.is_empty() {
                        None
                    } else {
                        trimmed.parse::<usize>().ok()
                    }
                })
        } else {
            None
        };
        
        // Extract end_char if present
        let end_char = if let Some(idx) = end_char_idx {
            fields.get(idx)
                .and_then(|s| {
                    let trimmed = s.trim();
                    if trimmed.is_empty() {
                        None
                    } else {
                        trimmed.parse::<usize>().ok()
                    }
                })
        } else {
            None
        };
        
        // Create adjustment
        let adjustment = FragmentAdjustment {
            xml_filename: xml_filename.clone(),
            frag_idx,
            end_line,
            end_char,
        };
        
        let key = FragmentKey {
            xml_filename,
            frag_idx,
        };
        
        adjustments.insert(key, adjustment);
    }
    
    Ok(adjustments)
}
