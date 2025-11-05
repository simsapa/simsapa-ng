//! Fragment parsing with line tracking
//!
//! This module provides functionality to parse XML files into fragments
//! while tracking line numbers and hierarchy.

use anyhow::{Result, Context};
use quick_xml::Reader;
use quick_xml::events::Event;
use crate::tipitaka_xml_parser::types::{XmlFragment, FragmentType, GroupType, GroupLevel, FragmentAdjustments, FragmentKey};
use crate::tipitaka_xml_parser::nikaya_structure::NikayaStructure;
use std::collections::HashMap;

/// Embedded cst-vs-sc.tsv mapping file
pub static CST_VS_SC_TSV: &str = include_str!("../../assets/cst-vs-sc.tsv");

/// Line and character position tracking for XML reader
///
/// Tracks both line numbers (1-indexed) and character positions within lines (0-indexed).
/// This allows precise location tracking even when multiple elements are on the same line.
struct LineTrackingReader<'a> {
    reader: Reader<&'a [u8]>,
    current_line: usize,
    current_char: usize,  // Character position within current line (0-indexed)
    last_position: usize, // Byte position in content
    content: &'a str,
}

impl<'a> LineTrackingReader<'a> {
    /// Create a new line-tracking reader
    fn new(content: &'a str) -> Self {
        let mut reader = Reader::from_str(content);
        reader.trim_text(false); // Preserve whitespace
        reader.expand_empty_elements(false); // Keep empty elements as-is
        
        Self {
            reader,
            current_line: 1,
            current_char: 0,
            last_position: 0,
            content,
        }
    }
    
    /// Get the current line number (1-indexed)
    fn current_line(&self) -> usize {
        self.current_line
    }
    
    /// Get the current character position within the line (0-indexed)
    fn current_char(&self) -> usize {
        self.current_char
    }
    
    /// Update line and character position based on byte position
    fn update_position(&mut self, position: usize) {
        if position <= self.last_position {
            return;
        }
        
        let slice = &self.content.as_bytes()[self.last_position..position.min(self.content.len())];
        
        for &byte in slice {
            if byte == b'\n' {
                self.current_line += 1;
                self.current_char = 0;
            } else {
                self.current_char += 1;
            }
        }
        
        self.last_position = position;
    }
    
    /// Read the next event and update position tracking
    fn read_event(&mut self) -> Result<Event<'a>> {
        let event = self.reader
            .read_event()
            .context("Failed to read XML event")?;
        
        // Update position AFTER reading the event so line/char tracking
        // points to the end of the event, matching the byte position
        let position = self.reader.buffer_position();
        self.update_position(position);
        
        Ok(event)
    }
    
    /// Get the current buffer position
    fn buffer_position(&self) -> usize {
        self.reader.buffer_position()
    }
}

/// Hierarchy tracker for maintaining group level context
///
/// Tracks the current position in the nikaya hierarchy and manages
/// entering/exiting levels according to the nikaya structure.
struct HierarchyTracker {
    current_levels: Vec<GroupLevel>,
    nikaya_structure: NikayaStructure,
}

impl HierarchyTracker {
    /// Create a new hierarchy tracker
    fn new(nikaya_structure: NikayaStructure) -> Self {
        Self {
            current_levels: Vec::new(),
            nikaya_structure,
        }
    }
    
    /// Enter a new hierarchy level
    ///
    /// Determines the depth of the level type in the nikaya structure,
    /// truncates current_levels to the appropriate depth, and adds the new level.
    /// If a level of the same type exists at that depth, it updates the title but preserves the ID.
    fn enter_level(
        &mut self,
        level_type: GroupType,
        title: String,
        id: Option<String>,
        number: Option<i32>,
    ) {

        // Find the depth of this level type in the nikaya structure
        let depth = self.nikaya_structure.levels
            .iter()
            .position(|t| matches!((t, &level_type), 
                (GroupType::Nikaya, GroupType::Nikaya) |
                (GroupType::Book, GroupType::Book) |
                (GroupType::Vagga, GroupType::Vagga) |
                (GroupType::Samyutta, GroupType::Samyutta) |
                (GroupType::Sutta, GroupType::Sutta)
            ));
        
        if let Some(depth) = depth {
            // Check if we already have a level at this depth with the same type
            if self.current_levels.len() > depth {
                let existing = &self.current_levels[depth];
                // Check if same type
                let same_type = match (&existing.group_type, &level_type) {
                    (GroupType::Nikaya, GroupType::Nikaya) |
                    (GroupType::Book, GroupType::Book) |
                    (GroupType::Vagga, GroupType::Vagga) |
                    (GroupType::Samyutta, GroupType::Samyutta) |
                    (GroupType::Sutta, GroupType::Sutta) => true,
                    _ => false,
                };
                
                if same_type {
                    // Update the existing level, but preserve ID if new ID is None
                    let preserved_id = if id.is_none() {
                        existing.id.clone()
                    } else {
                        id.clone()
                    };
                    
                    // Only truncate child levels if we're providing a new ID
                    // If id is None, we're just updating the title, so keep child levels intact
                    let should_truncate = id.is_some();
                    
                    if should_truncate {
                        // Truncate levels after this one before updating
                        self.current_levels.truncate(depth + 1);
                    }
                    
                    self.current_levels[depth] = GroupLevel {
                        group_type: level_type,
                        group_number: number,
                        title,
                        id: preserved_id,
                    };
                    return;
                }
            }
            
            // Truncate to the appropriate depth (remove levels at this depth and below)
            self.current_levels.truncate(depth);
            
            // Add the new level
            self.current_levels.push(GroupLevel {
                group_type: level_type,
                group_number: number,
                title,
                id,
            });
        }
    }
    
    /// Get a clone of the current hierarchy levels
    fn get_current_levels(&self) -> Vec<GroupLevel> {
        self.current_levels.clone()
    }
}

/// Fragment boundary detector
///
/// Detects boundaries between fragments based on nikaya-specific rules
/// and extracts relevant metadata.
struct FragmentBoundaryDetector<'a> {
    nikaya_structure: &'a NikayaStructure,
    cst_file: &'a str,
}

impl<'a> FragmentBoundaryDetector<'a> {
    fn new(nikaya_structure: &'a NikayaStructure, cst_file: &'a str) -> Self {
        Self { nikaya_structure, cst_file }
    }
    
    /// Check if an element marks a level boundary and extract metadata
    ///
    /// Returns Some((GroupType, title, id, number)) if this is a boundary element
    fn check_boundary(
        &self,
        tag_name: &str,
        attributes: &HashMap<String, String>,
    ) -> Option<(GroupType, String, Option<String>, Option<i32>)> {
        match tag_name {
            "p" if attributes.get("rend") == Some(&"nikaya".to_string()) => {
                Some((GroupType::Nikaya, String::new(), None, None))
            },
            "p" if attributes.get("rend") == Some(&"book".to_string()) => {
                Some((GroupType::Book, String::new(), None, None))
            },
            "div" if attributes.get("type") == Some(&"book".to_string()) => {
                let id = attributes.get("id").cloned();
                Some((GroupType::Book, String::new(), id, None))
            },
            "div" if attributes.get("type") == Some(&"samyutta".to_string()) => {
                let id = attributes.get("id").cloned();
                Some((GroupType::Samyutta, String::new(), id, None))
            },
            "div" if attributes.get("type") == Some(&"vagga".to_string()) => {
                let id = attributes.get("id").cloned();
                Some((GroupType::Vagga, String::new(), id, None))
            },
            "div" if attributes.get("type") == Some(&"sutta".to_string()) => {
                let id = attributes.get("id").cloned();
                Some((GroupType::Sutta, String::new(), id, None))
            },
            "head" if attributes.get("rend") == Some(&"book".to_string()) => {
                Some((GroupType::Book, String::new(), None, None))
            },
            "head" if attributes.get("rend") == Some(&"nikaya".to_string()) => {
                Some((GroupType::Nikaya, String::new(), None, None))
            },
            "head" if attributes.get("rend") == Some(&"chapter".to_string()) => {
                // In DN, chapter = Sutta
                // In MN, chapter = Vagga
                if self.nikaya_structure.nikaya == "digha" {
                    Some((GroupType::Sutta, String::new(), None, None))
                } else {
                    Some((GroupType::Vagga, String::new(), None, None))
                }
            },
            "p" if attributes.get("rend") == Some(&"subhead".to_string()) => {
                // In MN and SN, subhead = Sutta title
                if self.nikaya_structure.nikaya == "majjhima" || 
                   self.nikaya_structure.nikaya == "samyutta" {
                    Some((GroupType::Sutta, String::new(), None, None))
                } else {
                    None
                }
            },
            _ => None,
        }
    }
    
    /// Check if this is a sutta boundary (start of actual sutta content)
    fn is_sutta_start(&self, tag_name: &str, attributes: &HashMap<String, String>) -> bool {
        // Check if this is a commentary or sub-commentary file
        let is_commentary = self.cst_file.ends_with(".att.xml") || self.cst_file.ends_with(".tik.xml");
        
        match self.nikaya_structure.nikaya.as_str() {
            "digha" => {
                if is_commentary {
                    // DN commentary: Use <head rend="chapter"> for sutta boundaries
                    // NOT <div type="sutta"> which marks introduction sections
                    tag_name == "head" && attributes.get("rend") == Some(&"chapter".to_string())
                } else {
                    // DN base text: Suttas are wrapped in <div type="sutta">
                    tag_name == "div" && attributes.get("type") == Some(&"sutta".to_string())
                }
            },
            "majjhima" | "samyutta" => {
                // MN/SN: Suttas are delimited by <p rend="subhead">
                // Each subhead starts a new sutta
                tag_name == "p" && attributes.get("rend") == Some(&"subhead".to_string())
            },
            "anguttara" => {
                // AN: Similar to MN/SN
                tag_name == "p" && attributes.get("rend") == Some(&"subhead".to_string())
            },
            _ => {
                // Default: look for div or subhead
                (tag_name == "div" && attributes.get("type") == Some(&"sutta".to_string())) ||
                (tag_name == "p" && attributes.get("rend") == Some(&"subhead".to_string()))
            }
        }
    }
}

/// Extract CST fields from fragment content
///
/// Derives cst_file, cst_code, cst_vagga, cst_sutta, and cst_paranum from the fragment.
///
/// # Arguments
/// * `fragment` - The fragment to process
/// * `nikaya_structure` - The nikaya structure for context
///
/// # Returns
/// Tuple of (cst_file, cst_code, cst_vagga, cst_sutta, cst_paranum)
fn derive_cst_fields(
    fragment: &XmlFragment,
    nikaya_structure: &NikayaStructure,
) -> (String, Option<String>, Option<String>, Option<String>, Option<String>) {
    let cst_file = fragment.cst_file.clone();
    
    // Only process Sutta fragments
    if !matches!(fragment.frag_type, crate::tipitaka_xml_parser::types::FragmentType::Sutta) {
        return (cst_file, None, None, None, None);
    }
    
    // Extract vagga from group_levels
    let cst_vagga = fragment.group_levels.iter()
        .find(|level| matches!(level.group_type, crate::tipitaka_xml_parser::types::GroupType::Vagga))
        .and_then(|level| {
            if level.title.trim().is_empty() {
                None
            } else {
                Some(level.title.clone())
            }
        })
        .or_else(|| {
            // Fallback: Extract vagga title from <head rend="chapter"> tag in fragment content
            extract_vagga_title_from_content(&fragment.content)
        });
    
    // Extract sutta title from group_levels (filter out empty titles)
    let cst_sutta = fragment.group_levels.iter()
        .find(|level| matches!(level.group_type, crate::tipitaka_xml_parser::types::GroupType::Sutta))
        .and_then(|level| {
            if level.title.trim().is_empty() {
                None
            } else {
                Some(level.title.clone())
            }
        })
        .or_else(|| {
            // Fallback: Extract title from <head> or <p rend="subhead"> tag in fragment content
            extract_sutta_title_from_content(&fragment.content)
        });
    
    // Extract cst_paranum from first <p rend="bodytext" n="...">
    let cst_paranum = extract_first_paranum(&fragment.content);
    
    // Derive cst_code from div id attributes and sutta number
    // Pass the cst_sutta as a parameter so it can be used for deriving the code
    let cst_code = derive_cst_code(fragment, nikaya_structure, cst_sutta.as_deref());
    
    (cst_file, cst_code, cst_vagga, cst_sutta, cst_paranum)
}

/// Extract sutta title from <head> or <p rend="subhead"> tag in fragment content
/// Prefers <p rend="subhead"> over <head rend="chapter"> to avoid extracting vagga titles
fn extract_sutta_title_from_content(content: &str) -> Option<String> {
    use quick_xml::Reader;
    use quick_xml::events::Event;
    
    let mut reader = Reader::from_str(content);
    reader.trim_text(false);
    let mut buf = Vec::new();
    
    let mut first_chapter_title: Option<String> = None;
    let mut first_subhead_title: Option<String> = None;
    
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                let name_bytes = e.name();
                let name = std::str::from_utf8(name_bytes.as_ref()).unwrap_or("");
                
                // Check both <head> and <p> tags
                if name == "head" || name == "p" {
                    // Check if this has rend="chapter" or rend="subhead"
                    let mut rend_value: Option<String> = None;
                    
                    for attr in e.attributes() {
                        if let Ok(attr) = attr {
                            let key = std::str::from_utf8(attr.key.as_ref()).unwrap_or("");
                            let value = attr.unescape_value().unwrap_or_default();
                            
                            if key == "rend" && (value == "chapter" || value == "subhead") {
                                rend_value = Some(value.to_string());
                                break;
                            }
                        }
                    }
                    
                    if let Some(rend) = rend_value {
                        // Read the text content
                        if let Ok(Event::Text(ref text)) = reader.read_event_into(&mut buf) {
                            let title_text = text.unescape().unwrap_or_default().trim().to_string();
                            
                            // Keep the full title including number prefix (e.g., "2. Brahmajālasuttaṃ")
                            // But skip if it's a subsection (like "Uddeso" which doesn't start with a number)
                            let looks_like_sutta_title = title_text.chars().next()
                                .map(|c| c.is_numeric())
                                .unwrap_or(false);
                            
                            if !title_text.is_empty() && looks_like_sutta_title {
                                if rend == "subhead" && first_subhead_title.is_none() {
                                    first_subhead_title = Some(title_text.clone());
                                } else if rend == "chapter" && first_chapter_title.is_none() {
                                    first_chapter_title = Some(title_text.clone());
                                }
                                
                                // If we found a subhead title, we can return immediately
                                // since subheads are sutta titles and take priority over chapter titles (vagga titles)
                                if rend == "subhead" {
                                    return Some(title_text);
                                }
                            }
                        }
                    }
                }
            },
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {},
        }
        buf.clear();
    }
    
    // Prefer subhead title over chapter title
    first_subhead_title.or(first_chapter_title)
}

/// Extract vagga title from <head rend="chapter"> tag in fragment content
fn extract_vagga_title_from_content(content: &str) -> Option<String> {
    use quick_xml::Reader;
    use quick_xml::events::Event;
    
    let mut reader = Reader::from_str(content);
    reader.trim_text(false);
    let mut buf = Vec::new();
    
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                let name_bytes = e.name();
                let name = std::str::from_utf8(name_bytes.as_ref()).unwrap_or("");
                
                // Look for <head> tags
                if name == "head" {
                    // Check if this has rend="chapter"
                    let mut is_chapter = false;
                    
                    for attr in e.attributes() {
                        if let Ok(attr) = attr {
                            let key = std::str::from_utf8(attr.key.as_ref()).unwrap_or("");
                            let value = attr.unescape_value().unwrap_or_default();
                            
                            if key == "rend" && value == "chapter" {
                                is_chapter = true;
                                break;
                            }
                        }
                    }
                    
                    if is_chapter {
                        // Read the text content
                        if let Ok(Event::Text(ref text)) = reader.read_event_into(&mut buf) {
                            let title_text = text.unescape().unwrap_or_default().trim().to_string();
                            
                            // Keep the full title including number prefix (e.g., "2. Sīhanādavaggo")
                            let looks_like_vagga_title = title_text.chars().next()
                                .map(|c| c.is_numeric())
                                .unwrap_or(false);
                            
                            if !title_text.is_empty() && looks_like_vagga_title {
                                return Some(title_text);
                            }
                        }
                    }
                }
            },
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {},
        }
        buf.clear();
    }
    
    None
}

/// Extract the first paragraph number from bodytext
fn extract_first_paranum(content: &str) -> Option<String> {
    use quick_xml::Reader;
    use quick_xml::events::Event;
    
    let mut reader = Reader::from_str(content);
    reader.trim_text(false);
    let mut buf = Vec::new();
    
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => {
                let name_bytes = e.name();
                let name = std::str::from_utf8(name_bytes.as_ref()).unwrap_or("");
                if name == "p" {
                    // Check if this is a bodytext paragraph
                    let mut is_bodytext = false;
                    let mut paranum = None;
                    
                    for attr in e.attributes() {
                        if let Ok(attr) = attr {
                            let key = std::str::from_utf8(attr.key.as_ref()).unwrap_or("");
                            let value = attr.unescape_value().unwrap_or_default();
                            
                            if key == "rend" && value == "bodytext" {
                                is_bodytext = true;
                            } else if key == "n" {
                                paranum = Some(value.to_string());
                            }
                        }
                    }
                    
                    if is_bodytext && paranum.is_some() {
                        return paranum;
                    }
                }
            },
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {},
        }
        buf.clear();
    }
    
    None
}

/// Derive CST code from fragment metadata
///
/// For DN: code is like "dn1.1" from div id="dn1_1" or div id="dn1" + sutta number "1."
/// For MN: code is like "mn1.5.1" from div id="mn1_5_1" or div id="mn1_5" + sutta number "1."
fn derive_cst_code(fragment: &XmlFragment, _nikaya_structure: &NikayaStructure, cst_sutta_title: Option<&str>) -> Option<String> {
    // First check if the Sutta level itself has an ID (like "dn1_12")
    // This is the most direct and reliable source
    if let Some(sutta_id) = fragment.group_levels.iter()
        .find_map(|level| {
            if matches!(level.group_type, crate::tipitaka_xml_parser::types::GroupType::Sutta) {
                level.id.as_ref()
            } else {
                None
            }
        }) {
        // Convert id format: "dn1_12" or "mn1_5_3" -> "dn1.12" or "mn1.5.3"
        let code = sutta_id.replace('_', ".");
        return Some(code);
    }
    
    // Fallback: Try to construct from book ID + vagga number + sutta number
    // For MN/SN: code format is "mn{book}.{vagga}.{sutta}" e.g., "mn1.1.10"
    // For DN: code format is "dn{book}.{sutta}" e.g., "dn1.10"
    
    // Get book ID (e.g., "dn1" or "mn1")
    let book_id = fragment.group_levels.iter()
        .find_map(|level| {
            if matches!(level.group_type, crate::tipitaka_xml_parser::types::GroupType::Book) {
                level.id.as_ref()
            } else {
                None
            }
        });
    
    // Get vagga number from title (e.g., "1" from "1. Mūlapariyāyavaggo")
    // This is more reliable than using the vagga ID since the ID may be inherited from the next vagga
    // However, for vagga 0 (introduction/preamble) in commentary files, the title is often empty,
    // so we fallback to extracting from the ID (e.g., "mn1_0" -> "0")
    let vagga_number = fragment.group_levels.iter()
        .find_map(|level| {
            if matches!(level.group_type, crate::tipitaka_xml_parser::types::GroupType::Vagga) {
                // First try: Extract number from title like "1. Vagga Name"
                level.title.split_whitespace()
                    .next()
                    .and_then(|first| first.strip_suffix('.'))
                    .filter(|num| num.chars().all(|c| c.is_numeric()))
                    .or_else(|| {
                        // Fallback: Extract from ID like "mn1_0" or "mn1_1"
                        // Split by underscore and take the last part
                        level.id.as_ref().and_then(|id| {
                            id.rsplit('_')
                                .next()
                                .filter(|num| num.chars().all(|c| c.is_numeric()))
                        })
                    })
            } else {
                None
            }
        });
    
    // Extract sutta number from title (e.g., "1. Brahmajālasuttaṃ" -> "1")
    // First try from Sutta GroupLevel
    let sutta_number = fragment.group_levels.iter()
        .find_map(|level| {
            if matches!(level.group_type, crate::tipitaka_xml_parser::types::GroupType::Sutta) {
                // Extract number from title like "1. Title" or "10. Title"
                level.title.split_whitespace()
                    .next()
                    .and_then(|first| first.strip_suffix('.'))
                    .filter(|num| num.chars().all(|c| c.is_numeric()))
            } else {
                None
            }
        })
        .or_else(|| {
            // Fallback: Extract from cst_sutta_title parameter (from fragment content)
            cst_sutta_title.and_then(|title| {
                title.split_whitespace()
                    .next()
                    .and_then(|first| first.strip_suffix('.'))
                    .filter(|num| num.chars().all(|c| c.is_numeric()))
            })
        });
    
    // Build the code based on what we have
    match (book_id, vagga_number, sutta_number) {
        (Some(book), Some(vagga), Some(sutta)) => {
            // MN/SN style: mn1.1.10
            Some(format!("{}.{}.{}", book, vagga, sutta))
        }
        (Some(book), Some(vagga), None) => {
            // MN/SN vagga 0 (introduction/preamble) in commentary files: mn1.0.0
            // These fragments don't have a sutta number, so we use "0" as placeholder
            Some(format!("{}.{}.0", book, vagga))
        }
        (Some(book), None, Some(sutta)) => {
            // DN style: dn1.10
            Some(format!("{}.{}", book, sutta))
        }
        _ => None,
    }
}

/// Convert line/char coordinates to byte position in XML content
///
/// # Arguments
/// * `xml_content` - The XML content string
/// * `target_line` - Target line number (1-indexed)
/// * `target_char` - Target character position (0-indexed byte offset within line)
///
/// # Returns
/// Byte position in the XML content
fn line_char_to_byte_pos(xml_content: &str, target_line: usize, target_char: usize) -> usize {
    let mut current_line = 1;
    let mut current_char = 0;
    
    for (byte_idx, byte) in xml_content.bytes().enumerate() {
        // Check if we've reached the target position BEFORE processing this byte
        if current_line == target_line && current_char == target_char {
            return byte_idx;
        }
        
        // Update position tracking
        if byte == b'\n' {
            current_line += 1;
            current_char = 0;
        } else {
            current_char += 1;
        }
    }
    
    // If we didn't find the position, return the end
    xml_content.len()
}

/// Apply fragment adjustments to override end position
///
/// If adjustments are provided for this fragment, use the adjusted end_line and end_char.
/// Returns (end_byte_pos, end_line, end_char)
fn apply_fragment_adjustment(
    xml_content: &str,
    default_end_pos: usize,
    default_end_line: usize,
    default_end_char: usize,
    cst_file: &str,
    frag_idx: usize,
    adjustments: Option<&FragmentAdjustments>,
) -> (usize, usize, usize) {
    // Check if there's an adjustment for this fragment
    if let Some(adjustments_map) = adjustments {
        let key = FragmentKey {
            cst_file: cst_file.to_string(),
            frag_idx,
        };
        
        if let Some(adjustment) = adjustments_map.get(&key) {
            // Apply adjustments if end_line is provided
            // If end_char is not provided, default to 0 (start of line)
            if let Some(adj_end_line) = adjustment.end_line {
                let adj_end_char = adjustment.end_char.unwrap_or(0);
                // Convert adjusted line/char to byte position
                let adj_end_pos = line_char_to_byte_pos(xml_content, adj_end_line, adj_end_char);
                return (adj_end_pos, adj_end_line, adj_end_char);
            }
        }
    }
    
    // No adjustment - use default detection
    (default_end_pos, default_end_line, default_end_char)
}

/// Parse XML content into fragments with line tracking
///
/// # Arguments
/// * `xml_content` - The complete XML file content
/// * `nikaya_structure` - The structure configuration for this nikaya
/// * `cst_file` - Name of the XML file being parsed
/// * `adjustments` - Optional fragment adjustments to apply
/// * `populate_sc_fields` - Whether to populate SC fields from embedded TSV
///
/// # Returns
/// Vector of fragments or error if parsing fails
pub fn parse_into_fragments(
    xml_content: &str,
    nikaya_structure: &NikayaStructure,
    cst_file: &str,
    adjustments: Option<&FragmentAdjustments>,
    populate_sc_fields: bool,
) -> Result<Vec<XmlFragment>> {
    let mut reader = LineTrackingReader::new(xml_content);
    let mut hierarchy = HierarchyTracker::new(nikaya_structure.clone());
    let detector = FragmentBoundaryDetector::new(nikaya_structure, cst_file);
    
    let mut fragments: Vec<XmlFragment> = Vec::new();
    // Track: (byte_pos, line_num, char_pos)
    let mut current_fragment_start: Option<(usize, usize, usize)> = None;
    let mut current_frag_type: Option<FragmentType> = None;
    // Store hierarchy levels at the time fragment starts
    let mut current_fragment_group_levels: Vec<GroupLevel> = Vec::new();
    let mut pending_title: Option<(GroupType, String, Option<String>, Option<i32>)> = None; // (type, title, id, number)
    let mut in_sutta_content = false;
    // For MN/SN: track if we just saw a subhead element (will check text to see if numbered)
    let mut pending_subhead_check: Option<(usize, usize, usize)> = None; // (pos, line, char) of the subhead tag
    let mut seen_body_tag = false; // Track if we've seen the <body> opening tag
    let mut seen_first_sutta = false; // Track if we've encountered the first sutta marker
    let mut seen_first_vagga_or_sutta = false; // Track if we've seen the first vagga or sutta div
    let mut div_depth = 0; // Track div nesting depth to know when a sutta closes
    let mut sutta_div_depth: Option<usize> = None; // Track the depth of the current sutta div
    // For DN commentary: track the position of <div type="sutta"> that precedes <head rend="chapter">
    let mut pending_sutta_div_pos: Option<(usize, usize, usize)> = None;
    // For MN/SN: track the position of <div type="vagga"> that precedes <p rend="subhead">
    let mut pending_vagga_div_pos: Option<(usize, usize, usize)> = None;
    
    // Start with a Header fragment at the beginning of the file
    current_fragment_start = Some((0, 1, 0));
    current_frag_type = Some(FragmentType::Header);
    current_fragment_group_levels = hierarchy.get_current_levels();
    
    loop {
        // Capture position BEFORE reading the event (this is the start of the tag)
        let event_start_pos = reader.buffer_position();
        let event_start_line = reader.current_line();
        let event_start_char = reader.current_char();
        
        let event = reader.read_event()?;
        
        // Capture position AFTER reading the event (this is the end of the tag)
        let current_line = reader.current_line();
        let current_char = reader.current_char();
        let current_pos = reader.buffer_position();
        
        match event {
            Event::Start(ref e) | Event::Empty(ref e) => {
                let name_bytes = e.name();
                let tag_name = std::str::from_utf8(name_bytes.as_ref())
                    .context("Invalid UTF-8 in tag name")?
                    .to_string();
                
                // Parse attributes
                let mut attributes = HashMap::new();
                for attr in e.attributes() {
                    let attr = attr.context("Failed to parse attribute")?;
                    let key = std::str::from_utf8(attr.key.as_ref())
                        .context("Invalid UTF-8 in attribute key")?;
                    let value = attr.unescape_value()
                        .context("Failed to unescape attribute value")?;
                    attributes.insert(key.to_string(), value.to_string());
                }
                
                // Special handling for <body> tag - close Header fragment after it
                // Content after <body> will be included in the first Sutta fragment
                if tag_name == "body" && !seen_body_tag {
                    seen_body_tag = true;
                    
                    // Close the Header fragment right after the <body> tag
                    if let (Some((frag_start_pos, frag_start_line, frag_start_char)), Some(frag_type)) = 
                        (current_fragment_start, current_frag_type.as_ref()) {
                        
                        // Apply adjustments if any
                        let (end_pos, end_line, end_char) = apply_fragment_adjustment(
                            xml_content,
                            current_pos,
                            current_line,
                            current_char,
                            cst_file,
                            fragments.len(),
                            adjustments,
                        );
                        
                        let content = xml_content[frag_start_pos..end_pos].to_string();
                        if !content.trim().is_empty() {
                            fragments.push(XmlFragment {
                                nikaya: nikaya_structure.nikaya.clone(),
                                frag_type: FragmentType::Header,
                                content,
                                start_line: frag_start_line,
                                end_line,
                                start_char: frag_start_char,
                                end_char,
                                group_levels: current_fragment_group_levels.clone(),
                                cst_file: cst_file.to_string(),
                                frag_idx: fragments.len(),
                                frag_review: None,
                                cst_code: None,
                                cst_vagga: None,
                                cst_sutta: None,
                                cst_paranum: None,
                                sc_code: None,
                                sc_sutta: None,
                            });
                        }
                    }
                    
                    // Start a Sutta fragment immediately after <body>
                    // Content between <body> and the first sutta marker will be included
                    current_fragment_start = Some((current_pos, current_line, current_char));
                    current_frag_type = Some(FragmentType::Sutta);
                    current_fragment_group_levels = hierarchy.get_current_levels();
                    in_sutta_content = true;
                }
                
                // Check for boundary
                if let Some((group_type, _, id, number)) = detector.check_boundary(&tag_name, &attributes) {
                    // For <div> elements with an ID, enter the level immediately to preserve the ID
                    // The title will be updated later from a child <head> element
                    if tag_name == "div" && id.is_some() {
                        // Before entering a new Vagga or Sutta level, close any open sutta fragment
                        // This ensures the fragment uses the CURRENT level, not the next one
                        // BUT: Don't close for the FIRST vagga/sutta - that should include the preamble content
                        let is_vagga_or_sutta_level = matches!(group_type, GroupType::Vagga) || matches!(group_type, GroupType::Sutta);
                        let is_first_vagga_or_sutta = !seen_first_vagga_or_sutta && is_vagga_or_sutta_level;
                        
                        if is_first_vagga_or_sutta {
                            // Mark that we've seen the first vagga/sutta, but don't close the fragment
                            // The preamble content will be included with the first sutta
                            seen_first_vagga_or_sutta = true;
                        } else if is_vagga_or_sutta_level && in_sutta_content {
                            if let (Some((frag_start_pos, frag_start_line, frag_start_char)), Some(frag_type)) = 
                                (current_fragment_start, current_frag_type.as_ref()) {
                                
                                // Only close if this is a Sutta fragment and has actual sutta content
                                if matches!(frag_type, FragmentType::Sutta) {
                                    let tentative_content = xml_content[frag_start_pos..event_start_pos].to_string();
                                    let has_sutta_content = tentative_content.contains("rend=\"subhead\"") || 
                                                           tentative_content.contains("rend=\"chapter\"") ||
                                                           tentative_content.contains("rend=\"bodytext\"");
                                    
                                    if has_sutta_content {
                                        // Close at the current position (before the new vagga/sutta div)
                                        let (end_pos, end_line, end_char) = apply_fragment_adjustment(
                                            xml_content,
                                            event_start_pos,
                                            event_start_line,
                                            event_start_char,
                                            cst_file,
                                            fragments.len(),
                                            adjustments,
                                        );
                                        
                                        // Create content with adjusted end position
                                        let content = xml_content[frag_start_pos..end_pos].to_string();
                                        
                                if !content.trim().is_empty() {
                                    fragments.push(XmlFragment {
                                        nikaya: nikaya_structure.nikaya.clone(),
                                        frag_type: frag_type.clone(),
                                        content,
                                        start_line: frag_start_line,
                                        end_line,
                                        start_char: frag_start_char,
                                        end_char,
                                        group_levels: current_fragment_group_levels.clone(),
                                        cst_file: cst_file.to_string(),
                                        frag_idx: fragments.len(),
                                        frag_review: None,
                                        cst_code: None,
                                        cst_vagga: None,
                                        cst_sutta: None,
                                        cst_paranum: None,
                                        sc_code: None,
                                        sc_sutta: None,
                                    });
                                }
                                
                                        // Start new fragment at the adjusted end position of the previous fragment
                                        // This ensures no gap in XML reconstruction when adjustments are used
                                        current_fragment_start = Some((end_pos, end_line, end_char));
                                        current_frag_type = Some(FragmentType::Sutta);
                                        // Note: we'll update group_levels AFTER entering the new level
                                    }
                                }
                            }
                        }
                        
                        hierarchy.enter_level(group_type.clone(), String::new(), id, number);
                        
                        // Update group_levels after entering any new level while a fragment is open
                        if current_fragment_start.is_some() {
                            current_fragment_group_levels = hierarchy.get_current_levels();
                        }
                        
                        // Don't set pending_title - the next <head> will update the title
                    } else {
                        // For other elements, we'll get the title from the text content, so store it as pending
                        // EXCEPT for MN/SN subheads which need text content validation
                        let is_mn_sn_subhead = (nikaya_structure.nikaya == "majjhima" || 
                                               nikaya_structure.nikaya == "samyutta") &&
                                              matches!(group_type, GroupType::Sutta) &&
                                              tag_name == "p" && 
                                              attributes.get("rend") == Some(&"subhead".to_string());
                        
                        if !is_mn_sn_subhead {
                            pending_title = Some((group_type.clone(), String::new(), id, number));
                        }
                    }
                }
                
                // Track div depth for nested div elements
                if tag_name == "div" {
                    div_depth += 1;
                    
                    // For DN commentary: <div type="sutta"> precedes <head rend="chapter">
                    // Store its position to use when we encounter the <head> tag
                    let is_commentary = cst_file.ends_with(".att.xml") || cst_file.ends_with(".tik.xml");
                    if is_commentary && 
                       nikaya_structure.nikaya == "digha" &&
                       attributes.get("type") == Some(&"sutta".to_string()) {
                        pending_sutta_div_pos = Some((event_start_pos, event_start_line, event_start_char));
                    }
                    
                    // For MN/SN: <div type="vagga"> precedes <p rend="subhead">
                    // Store its position to use when we encounter the subhead
                    if (nikaya_structure.nikaya == "majjhima" || nikaya_structure.nikaya == "samyutta") &&
                       attributes.get("type") == Some(&"vagga".to_string()) {
                        pending_vagga_div_pos = Some((event_start_pos, event_start_line, event_start_char));
                    }
                }
                
                // Handle sutta boundaries based on nikaya structure
                let is_potential_sutta_marker = detector.is_sutta_start(&tag_name, &attributes);
                
                // For MN/SN, we need to check the text content to see if it's a numbered subhead
                if is_potential_sutta_marker && 
                   (nikaya_structure.nikaya == "majjhima" || nikaya_structure.nikaya == "samyutta") &&
                   tag_name == "p" && attributes.get("rend") == Some(&"subhead".to_string()) {
                    // Store START position of the tag for later text check
                    pending_subhead_check = Some((event_start_pos, event_start_line, event_start_char));
                } else if is_potential_sutta_marker {
                    // Check if this sutta marker is a div that should track depth
                    // For DN base text: <div type="sutta"> IS the sutta marker, so track depth
                    // For DN commentary: <head rend="chapter"> is the sutta marker, <div type="sutta"> is NOT
                    let is_commentary = cst_file.ends_with(".att.xml") || cst_file.ends_with(".tik.xml");
                    let should_track_div_depth = tag_name == "div" && 
                                                 attributes.get("type") == Some(&"sutta".to_string()) &&
                                                 !is_commentary;
                    
                    // Check if this is the first sutta marker after <body>
                    if !seen_first_sutta && in_sutta_content {
                        // This is the FIRST sutta marker - don't close current fragment
                        // Just mark that we've seen it
                        seen_first_sutta = true;
                        
                        // Only track div depth if this is a div-based sutta marker
                        if should_track_div_depth {
                            sutta_div_depth = Some(div_depth);
                        }
                        // Continue with the current fragment
                    } else if seen_first_sutta {
                        // This is a SUBSEQUENT sutta marker - start a new fragment
                        // For DN commentary, check if there's a pending <div type="sutta"> position
                        // If so, use that as the start position (and close position for previous fragment)
                        let (start_pos, start_line, start_char, close_pos, close_line, close_char) = 
                            if let Some((div_pos, div_line, div_char)) = pending_sutta_div_pos.take() {
                                // Use the <div> position
                                (div_pos, div_line, div_char, div_pos, div_line, div_char)
                            } else {
                                // Use the current tag position (normal case)
                                (event_start_pos, event_start_line, event_start_char,
                                 event_start_pos, event_start_line, event_start_char)
                            };
                        
                        // Close current sutta fragment (excluding this tag)
                        if let (Some((frag_start_pos, frag_start_line, frag_start_char)), Some(frag_type)) = 
                            (current_fragment_start, current_frag_type.as_ref()) {
                            
                            // Apply adjustments if any
                            let (end_pos, end_line, end_char) = apply_fragment_adjustment(
                                xml_content,
                                close_pos,
                                close_line,
                                close_char,
                                cst_file,
                                fragments.len(),
                                adjustments,
                            );
                            
                            let content = xml_content[frag_start_pos..end_pos].to_string();
                                 if !content.trim().is_empty() {
                                    fragments.push(XmlFragment {
                                        nikaya: nikaya_structure.nikaya.clone(),
                                        frag_type: frag_type.clone(),
                                        content,
                                        start_line: frag_start_line,
                                        end_line,
                                        start_char: frag_start_char,
                                        end_char,
                                        group_levels: current_fragment_group_levels.clone(),
                                        cst_file: cst_file.to_string(),
                                        frag_idx: fragments.len(),
                                        frag_review: None,
                                        cst_code: None,
                                        cst_vagga: None,
                                        cst_sutta: None,
                                        cst_paranum: None,
                                        sc_code: None,
                                        sc_sutta: None,
                                    });
                                    
                                    // If we adjusted the end position, start the next fragment there
                                    // to avoid gaps in XML reconstruction
                                    current_fragment_start = Some((end_pos, end_line, end_char));
                                } else {
                                    // No content was written, start from the original position
                                    current_fragment_start = Some((start_pos, start_line, start_char));
                                }
                        } else {
                            // No previous fragment to close, start from the original position
                            current_fragment_start = Some((start_pos, start_line, start_char));
                        }
                        
                        current_frag_type = Some(FragmentType::Sutta);
                        current_fragment_group_levels = hierarchy.get_current_levels();
                        
                        // Only track div depth if this is a div-based sutta marker
                        if should_track_div_depth {
                            sutta_div_depth = Some(div_depth);
                        }
                        // Stay in_sutta_content = true
                    }
                }
            },
            
            Event::Text(ref e) => {
                let text = e.unescape()
                    .context("Failed to unescape text content")?
                    .trim()
                    .to_string();
                
                // Check if this text is for a pending subhead (MN/SN style)
                if let Some((subhead_pos, subhead_line, subhead_char)) = pending_subhead_check.take() {
                    // Check if text starts with a number followed by a dot (e.g., "1. ", "10. ")
                    // Pattern: one or more digits, followed by a dot and space
                    let is_numbered = text.split_whitespace()
                        .next()
                        .and_then(|first_word| first_word.strip_suffix('.'))
                        .map_or(false, |num_part| num_part.chars().all(|c| c.is_numeric()));
                    
                    // For commentary/sub-commentary files, also check if it ends with "suttavaṇṇanā"
                    // to distinguish actual sutta commentaries from subsections
                    let is_commentary = cst_file.ends_with(".att.xml") || cst_file.ends_with(".tik.xml");
                    
                    let is_sutta_commentary = if is_commentary {
                        // In commentary files, only treat it as a sutta if it ends with "suttavaṇṇanā"
                        text.ends_with("suttavaṇṇanā")
                    } else {
                        // In base text files, any numbered subhead is a sutta
                        is_numbered
                    };
                    
                    if is_sutta_commentary {
                        // This is a sutta boundary!
                        // Check if this is the first sutta marker after <body>
                        if !seen_first_sutta && in_sutta_content {
                            // This is the FIRST sutta marker - don't close current fragment
                            seen_first_sutta = true;
                            // Clear pending_vagga_div_pos so it's not used for the next sutta
                            // The first sutta should include the preamble, so we don't split at the vagga div
                            pending_vagga_div_pos = None;
                            // Update hierarchy with sutta title
                            hierarchy.enter_level(GroupType::Sutta, text.clone(), None, None);
                            // Update group_levels to include the new Sutta level
                            current_fragment_group_levels = hierarchy.get_current_levels();
                            // Continue with the current fragment
                        } else if seen_first_sutta {
                            // This is a SUBSEQUENT sutta marker - start a new fragment
                            // For MN/SN, check if there's a pending <div type="vagga"> position
                            // If so, use that as the start position (and close position for previous fragment)
                            let (start_pos, start_line, start_char, close_pos, close_line, close_char) = 
                                if let Some((div_pos, div_line, div_char)) = pending_vagga_div_pos.take() {
                                    // Use the vagga <div> position
                                    (div_pos, div_line, div_char, div_pos, div_line, div_char)
                                } else {
                                    // Use the subhead position (normal case)
                                    (subhead_pos, subhead_line, subhead_char,
                                     subhead_pos, subhead_line, subhead_char)
                                };
                            
                            // Already in a sutta - close current and start new
                            if let (Some((frag_start_pos, frag_start_line, frag_start_char)), Some(frag_type)) = 
                                (current_fragment_start, current_frag_type.as_ref()) {
                                
                                // Apply adjustments if any
                                let (end_pos, end_line, end_char) = apply_fragment_adjustment(
                                    xml_content,
                                    close_pos,
                                    close_line,
                                    close_char,
                                    cst_file,
                                    fragments.len(),
                                    adjustments,
                                );
                                
                                let content = xml_content[frag_start_pos..end_pos].to_string();
                                if !content.trim().is_empty() {
                                    fragments.push(XmlFragment {
                                        nikaya: nikaya_structure.nikaya.clone(),
                                        frag_type: frag_type.clone(),
                                        content,
                                        start_line: frag_start_line,
                                        end_line,
                                        start_char: frag_start_char,
                                        end_char,
                                        group_levels: current_fragment_group_levels.clone(),
                                        cst_file: cst_file.to_string(),
                                        frag_idx: fragments.len(),
                                        frag_review: None,
                                        cst_code: None,
                                        cst_vagga: None,
                                        cst_sutta: None,
                                        cst_paranum: None,
                                        sc_code: None,
                                        sc_sutta: None,
                                    });
                                    
                                    // If we adjusted the end position, start the next fragment there
                                    // to avoid gaps in XML reconstruction
                                    current_fragment_start = Some((end_pos, end_line, end_char));
                                } else {
                                    // No content was written, start from the original position
                                    current_fragment_start = Some((start_pos, start_line, start_char));
                                }
                            } else {
                                // No previous fragment to close, start from the original position
                                current_fragment_start = Some((start_pos, start_line, start_char));
                            }
                            
                            // Update hierarchy with new sutta title
                            hierarchy.enter_level(GroupType::Sutta, text.clone(), None, None);
                            
                            current_frag_type = Some(FragmentType::Sutta);
                            current_fragment_group_levels = hierarchy.get_current_levels();
                        }
                    }
                    // If not numbered, it's just a section heading within a sutta - ignore
                }
                
                // If we have a pending title, update it with this text
                if let Some((group_type, _, id, number)) = pending_title.take() {
                    if !text.is_empty() {
                        hierarchy.enter_level(group_type, text, id, number);
                    }
                }
            },
            
            Event::End(ref e) => {
                let name_bytes = e.name();
                let tag_name = std::str::from_utf8(name_bytes.as_ref())
                    .context("Invalid UTF-8 in tag name")?
                    .to_string();
                
                // Track div depth - decrement when seeing closing div tags
                if tag_name == "div" {
                    // Check if this is closing the current sutta div
                    if in_sutta_content {
                        if let Some(sutta_depth) = sutta_div_depth {
                            if div_depth == sutta_depth {
                                // This closes the current sutta div
                                // DON'T close the fragment here - let the next sutta or </body> do it
                                // This allows the last sutta to include content after its </div>
                                sutta_div_depth = None;
                            }
                        }
                    }
                    
                    // Decrement div depth after processing
                    div_depth = div_depth.saturating_sub(1);
                }
                
                // Check if this closes the body tag - now we exit sutta content
                if tag_name == "body" && seen_body_tag {
                    // Close any pending sutta fragment first
                    // The sutta fragment should include ALL content up to (but not including) </body>
                    if let (Some((start_pos, start_line, start_char)), Some(frag_type)) = 
                        (current_fragment_start, current_frag_type.as_ref()) {
                        
                        // Apply adjustments if any
                        let (end_pos, end_line, end_char) = apply_fragment_adjustment(
                            xml_content,
                            event_start_pos,
                            event_start_line,
                            event_start_char,
                            cst_file,
                            fragments.len(),
                            adjustments,
                        );
                        
                        // Include everything from start up to the adjusted end position
        let content = xml_content[start_pos..end_pos].to_string();
        if !content.trim().is_empty() {
            fragments.push(XmlFragment {
                nikaya: nikaya_structure.nikaya.clone(),
                frag_type: frag_type.clone(),
                content,
                start_line,
                end_line,
                start_char,
                end_char,
                group_levels: current_fragment_group_levels.clone(),
                cst_file: cst_file.to_string(),
                frag_idx: fragments.len(),
                frag_review: None,
                cst_code: None,
                cst_vagga: None,
                cst_sutta: None,
                cst_paranum: None,
                sc_code: None,
                sc_sutta: None,
            });
            
            // Start the final Header fragment at the adjusted end position
            // to avoid gaps in XML reconstruction
            current_fragment_start = Some((end_pos, end_line, end_char));
        } else {
            // No content was written, start from the original position
            current_fragment_start = Some((event_start_pos, event_start_line, event_start_char));
        }
                    } else {
                        // No previous fragment, start from the original position
                        current_fragment_start = Some((event_start_pos, event_start_line, event_start_char));
                    }
                    
                    current_frag_type = Some(FragmentType::Header);
                    current_fragment_group_levels = hierarchy.get_current_levels();
                    in_sutta_content = false;
                }
            },
            
            Event::Eof => break,
            
            _ => {},
        }
    }
    
    // Close any remaining fragment (usually the final Header fragment)
    if let (Some((start_pos, start_line, start_char)), Some(frag_type)) = 
        (current_fragment_start, current_frag_type) {
        
        // Apply adjustments if any
        let (end_pos, end_line, end_char) = apply_fragment_adjustment(
            xml_content,
            xml_content.len(),
            reader.current_line(),
            reader.current_char(),
            cst_file,
            fragments.len(),
            adjustments,
        );
        
        let content = xml_content[start_pos..end_pos].to_string();
        if !content.trim().is_empty() {
                            fragments.push(XmlFragment {
                                nikaya: nikaya_structure.nikaya.clone(),
                                frag_type: frag_type.clone(),
                                content,
                                start_line,
                                end_line,
                                start_char,
                                end_char,
                                group_levels: current_fragment_group_levels.clone(),
                                cst_file: cst_file.to_string(),
                                frag_idx: fragments.len(),
                                frag_review: None,
                                cst_code: None,
                                cst_vagga: None,
                                cst_sutta: None,
                                cst_paranum: None,
                                sc_code: None,
                                sc_sutta: None,
                            });
        }
    }
    
    // Post-process fragments to derive CST fields
    for fragment in &mut fragments {
        let (cst_file, cst_code, cst_vagga, cst_sutta, cst_paranum) = 
            derive_cst_fields(fragment, nikaya_structure);
        
        fragment.cst_file = cst_file;
        fragment.cst_code = cst_code;
        fragment.cst_vagga = cst_vagga;
        fragment.cst_sutta = cst_sutta;
        fragment.cst_paranum = cst_paranum;
    }
    
    // Populate SC fields from embedded TSV if requested
    if populate_sc_fields {
        populate_sc_fields_from_tsv(&mut fragments)?;
    }
    
    Ok(fragments)
}

/// Populate SC fields from embedded TSV mapping
///
/// Looks up sc_code and sc_sutta from the embedded cst-vs-sc.tsv based on cst_code
///
/// # Arguments
/// * `fragments` - Mutable vector of fragments to populate
///
/// # Returns
/// Result indicating success or error
fn populate_sc_fields_from_tsv(
    fragments: &mut Vec<XmlFragment>,
) -> anyhow::Result<()> {
    // Load TSV mapping using the sutta_builder function
    use crate::tipitaka_xml_parser::sutta_builder::load_tsv_mapping;
    let tsv_records = load_tsv_mapping()
        .context("Failed to load TSV mapping")?;
    
    // Build a map from cst_code to (sc_code, sc_sutta)
    let mut tsv_map: HashMap<String, (String, String)> = HashMap::new();
    
    for record in tsv_records {
        if !record.cst_code.is_empty() && !record.sc_code.is_empty() {
            tsv_map.insert(
                record.cst_code.clone(),
                (record.sc_code.clone(), record.sc_sutta.clone())
            );
        }
    }
    
    // Populate fragments
    for fragment in fragments.iter_mut() {
        if let Some(ref cst_code) = fragment.cst_code {
            if let Some((sc_code, sc_sutta)) = tsv_map.get(cst_code) {
                fragment.sc_code = Some(sc_code.clone());
                fragment.sc_sutta = Some(sc_sutta.clone());
            }
        }
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tipitaka_xml_parser::nikaya_detector::detect_nikaya_structure;

    /// Helper to create minimal DN XML for testing
    fn create_dn_sample_xml() -> String {
        r#"<?xml version="1.0" encoding="UTF-8"?>
<TEI.2>
<teiHeader></teiHeader>
<text>
<body>
<p rend="nikaya">Dīghanikāyo</p>
<div id="dn1" type="book">
<head rend="book">Sīlakkhandhavaggapāḷi</head>
<div id="dn1_1" type="sutta">
<head rend="chapter">1. Brahmajālasutta</head>
<p rend="subhead">Paribbājakakathā</p>
<p rend="bodytext" n="1">Evaṃ me sutaṃ – ekaṃ samayaṃ bhagavā antarā ca rājagahaṃ antarā ca nālandaṃ.</p>
<p rend="bodytext" n="2">Atha kho bhagavā ambalatthikāyaṃ rājāgārake ekarattivāsaṃ upagacchi.</p>
</div>
</div>
</body>
</text>
</TEI.2>"#.to_string()
    }

    /// Helper to create minimal MN XML for testing
    fn create_mn_sample_xml() -> String {
        r#"<?xml version="1.0" encoding="UTF-8"?>
<TEI.2>
<teiHeader></teiHeader>
<text>
<body>
<p rend="nikaya">Majjhimanikāyo</p>
<div id="mn1" type="book">
<head rend="book">Mūlapaṇṇāsapāḷi</head>
<div id="mn1_1" type="vagga">
<head rend="chapter">Mūlapariyāyavaggo</head>
<div id="mn1_1_1" type="sutta">
<p rend="subhead">1. Mūlapariyāyasutta</p>
<p rend="bodytext" n="1">Evaṃ me sutaṃ – ekaṃ samayaṃ bhagavā ukkaṭṭhāyaṃ viharati.</p>
<p rend="bodytext" n="2">Tatra kho bhagavā bhikkhū āmantesi – "bhikkhavo"ti.</p>
</div>
</div>
</div>
</body>
</text>
</TEI.2>"#.to_string()
    }

    #[test]
    fn test_parse_dn_sample_basic() {
        let xml = create_dn_sample_xml();
        let structure = detect_nikaya_structure(&xml).expect("Should detect DN structure");
        
        assert_eq!(structure.nikaya, "digha");
        
        let fragments = parse_into_fragments(&xml, &structure, "test.xml", None, false).expect("Should parse fragments");
        
        // Should have at least one fragment
        assert!(!fragments.is_empty(), "Should have at least one fragment");
    }

    #[test]
    fn test_parse_dn_fragment_count() {
        let xml = create_dn_sample_xml();
        let structure = detect_nikaya_structure(&xml).unwrap();
        let fragments = parse_into_fragments(&xml, &structure, "test.xml", None, false).unwrap();
        
        // Count sutta fragments
        let sutta_fragments: Vec<_> = fragments.iter()
            .filter(|f| matches!(f.frag_type, FragmentType::Sutta))
            .collect();
        
        // Should have one sutta fragment
        assert_eq!(sutta_fragments.len(), 1, "Should have exactly one sutta fragment");
    }

    #[test]
    fn test_parse_dn_line_tracking() {
        let xml = create_dn_sample_xml();
        let structure = detect_nikaya_structure(&xml).unwrap();
        let fragments = parse_into_fragments(&xml, &structure, "test.xml", None, false).unwrap();
        
        for fragment in &fragments {
            // Line numbers should be valid (start > 0, end >= start)
            assert!(fragment.start_line > 0, "Start line should be > 0");
            assert!(fragment.end_line >= fragment.start_line, 
                    "End line should be >= start line");
        }
    }

    #[test]
    fn test_parse_mn_sample_basic() {
        let xml = create_mn_sample_xml();
        let structure = detect_nikaya_structure(&xml).expect("Should detect MN structure");
        
        assert_eq!(structure.nikaya, "majjhima");
        
        let fragments = parse_into_fragments(&xml, &structure, "test.xml", None, false).expect("Should parse fragments");
        
        assert!(!fragments.is_empty(), "Should have at least one fragment");
    }

    #[test]
    fn test_fragment_content_not_empty() {
        let xml = create_dn_sample_xml();
        let structure = detect_nikaya_structure(&xml).unwrap();
        let fragments = parse_into_fragments(&xml, &structure, "test.xml", None, false).unwrap();
        
        for fragment in &fragments {
            // Each fragment should have non-empty content
            assert!(!fragment.content.trim().is_empty(), 
                    "Fragment content should not be empty");
        }
    }

    #[test]
    fn test_character_position_tracking() {
        let xml = create_dn_sample_xml();
        let structure = detect_nikaya_structure(&xml).unwrap();
        let fragments = parse_into_fragments(&xml, &structure, "test.xml", None, false).unwrap();
        
        for fragment in &fragments {
            // Character positions should be valid
            assert!(fragment.start_char <= fragment.end_char || fragment.start_line < fragment.end_line,
                    "Character positions should be valid: start_line={}, start_char={}, end_line={}, end_char={}",
                    fragment.start_line, fragment.start_char, fragment.end_line, fragment.end_char);
            
            // If on same line, start_char should be < end_char
            if fragment.start_line == fragment.end_line {
                assert!(fragment.start_char < fragment.end_char,
                        "On same line, start_char ({}) should be < end_char ({})",
                        fragment.start_char, fragment.end_char);
            }
        }
    }

    #[test]
    fn test_same_line_multiple_elements() {
        // Create XML with multiple short elements on the same line
        let xml = r#"<?xml version="1.0"?>
<text><body><p rend="nikaya">Dīghanikāyo</p><div type="book"><head rend="book">Book1</head><div type="sutta"><head rend="chapter">Sutta1</head><p n="1">Text1</p></div></div></body></text>"#;
        
        let structure = detect_nikaya_structure(xml).unwrap();
        let fragments = parse_into_fragments(xml, &structure, "test.xml", None, false).unwrap();
        
        // Check that we can distinguish elements on the same line
        // by their character positions
        for i in 0..fragments.len() {
            for j in (i+1)..fragments.len() {
                let frag_i = &fragments[i];
                let frag_j = &fragments[j];
                
                // If both fragments are on the same line
                if frag_i.start_line == frag_j.start_line && 
                   frag_i.end_line == frag_j.end_line &&
                   frag_i.start_line == frag_i.end_line {
                    // They should have non-overlapping character ranges
                    let no_overlap = frag_i.end_char <= frag_j.start_char || 
                                    frag_j.end_char <= frag_i.start_char;
                    assert!(no_overlap,
                            "Fragments on same line should not overlap: \
                             frag[{}]: {}:{}-{}:{}, frag[{}]: {}:{}-{}:{}",
                            i, frag_i.start_line, frag_i.start_char, 
                            frag_i.end_line, frag_i.end_char,
                            j, frag_j.start_line, frag_j.start_char,
                            frag_j.end_line, frag_j.end_char);
                }
            }
        }
    }

    #[test]
    fn test_cst_fields_dn() {
        // Test CST field derivation for DN
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<TEI.2>
<text>
<body>
<p rend="nikaya">Dīghanikāyo</p>
<div id="dn1" n="dn1" type="book">
<head rend="book">Sīlakkhandhavaggapāḷi</head>
<div id="dn1_1" n="dn1_1" type="sutta">
<head rend="chapter">1. Brahmajālasuttaṃ</head>
<p rend="subhead">Paribbājakakathā</p>
<p rend="bodytext" n="1">Evaṃ me sutaṃ</p>
</div>
</div>
</body>
</text>
</TEI.2>"#;
        
        let structure = detect_nikaya_structure(xml).unwrap();
        let fragments = parse_into_fragments(xml, &structure, "s0101m.mul.xml", None, false).unwrap();
        
        // Find the sutta fragment
        let sutta_frag = fragments.iter()
            .find(|f| matches!(f.frag_type, FragmentType::Sutta))
            .expect("Should have a sutta fragment");
        
        // Check CST fields
        assert_eq!(sutta_frag.cst_file.as_str(), "s0101m.mul.xml");
        assert_eq!(sutta_frag.cst_code.as_deref(), Some("dn1.1"));
        assert_eq!(sutta_frag.cst_vagga.as_deref(), None); // DN doesn't have vaggas
        assert_eq!(sutta_frag.cst_sutta.as_deref(), Some("1. Brahmajālasuttaṃ"));
        assert_eq!(sutta_frag.cst_paranum.as_deref(), Some("1"));
    }

    #[test]
    fn test_cst_fields_mn() {
        // Test CST field derivation for MN
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<TEI.2>
<text>
<body>
<p rend="nikaya">Majjhimanikāyo</p>
<div id="mn1" type="book">
<head rend="book">Mūlapaṇṇāsapāḷi</head>
<div id="mn1_5" n="mn1_5" type="vagga">
<head rend="chapter">5. Cūḷayamakavaggo</head>
<p rend="subhead">1. Sāleyyakasuttaṃ</p>
<p rend="bodytext" n="439">Evaṃ me sutaṃ</p>
</div>
</div>
</body>
</text>
</TEI.2>"#;
        
        let structure = detect_nikaya_structure(xml).unwrap();
        let fragments = parse_into_fragments(xml, &structure, "s0201m.mul.xml", None, false).unwrap();
        
        // Find the sutta fragment
        let sutta_frag = fragments.iter()
            .find(|f| matches!(f.frag_type, FragmentType::Sutta))
            .expect("Should have a sutta fragment");
        
        // Check CST fields
        assert_eq!(sutta_frag.cst_file.as_str(), "s0201m.mul.xml");
        assert_eq!(sutta_frag.cst_code.as_deref(), Some("mn1.5.1"));
        assert_eq!(sutta_frag.cst_vagga.as_deref(), Some("5. Cūḷayamakavaggo"));
        assert_eq!(sutta_frag.cst_sutta.as_deref(), Some("1. Sāleyyakasuttaṃ"));
        assert_eq!(sutta_frag.cst_paranum.as_deref(), Some("439"));
    }
}
