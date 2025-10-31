//! Fragment parsing with line tracking
//!
//! This module provides functionality to parse XML files into fragments
//! while tracking line numbers and hierarchy.

use anyhow::{Result, Context};
use quick_xml::Reader;
use quick_xml::events::{Event, BytesStart};
use crate::tipitaka_xml_parser::types::{XmlFragment, FragmentType, GroupType, GroupLevel};
use crate::tipitaka_xml_parser::nikaya_structure::NikayaStructure;
use std::collections::HashMap;

/// Line-tracking XML reader wrapper
///
/// Wraps quick_xml::Reader and tracks the current line number by counting
/// newlines in the processed content.
struct LineTrackingReader<'a> {
    reader: Reader<&'a [u8]>,
    current_line: usize,
    last_position: usize,
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
            last_position: 0,
            content,
        }
    }
    
    /// Get the current line number
    fn current_line(&self) -> usize {
        self.current_line
    }
    
    /// Update line count based on position
    fn update_line_count(&mut self, position: usize) {
        if position <= self.last_position {
            return;
        }
        
        // Count newlines between last_position and current position
        let slice = &self.content.as_bytes()[self.last_position..position.min(self.content.len())];
        let newlines = slice.iter().filter(|&&b| b == b'\n').count();
        self.current_line += newlines;
        self.last_position = position;
    }
    
    /// Read the next event and update line tracking
    fn read_event(&mut self) -> Result<Event<'a>> {
        let position = self.reader.buffer_position();
        self.update_line_count(position);
        
        self.reader
            .read_event()
            .context("Failed to read XML event")
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
}

impl<'a> FragmentBoundaryDetector<'a> {
    fn new(nikaya_structure: &'a NikayaStructure) -> Self {
        Self { nikaya_structure }
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
        // Sutta content starts with first content paragraph or sutta div
        match tag_name {
            "div" if attributes.get("type") == Some(&"sutta".to_string()) => true,
            "p" if attributes.get("rend") == Some(&"bodytext".to_string()) => true,
            "p" if attributes.get("rend") == Some(&"gatha1".to_string()) => true,
            "p" if attributes.get("rend") == Some(&"gatha2".to_string()) => true,
            "p" if attributes.get("rend") == Some(&"gatha3".to_string()) => true,
            _ => false,
        }
    }
}

/// Parse XML content into fragments with line tracking
///
/// # Arguments
/// * `xml_content` - The complete XML file content
/// * `nikaya_structure` - The structure configuration for this nikaya
///
/// # Returns
/// Vector of fragments or error if parsing fails
pub fn parse_into_fragments(
    xml_content: &str,
    nikaya_structure: &NikayaStructure,
) -> Result<Vec<XmlFragment>> {
    let mut reader = LineTrackingReader::new(xml_content);
    let mut hierarchy = HierarchyTracker::new(nikaya_structure.clone());
    let detector = FragmentBoundaryDetector::new(nikaya_structure);
    
    let mut fragments: Vec<XmlFragment> = Vec::new();
    let mut current_fragment_start: Option<(usize, usize)> = None; // (byte_pos, line_num)
    let mut current_fragment_type: Option<FragmentType> = None;
    let mut pending_title: Option<(GroupType, String)> = None;
    let mut in_sutta_content = false;
    
    loop {
        let event = reader.read_event()?;
        let current_line = reader.current_line();
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
                
                // Check for boundary
                if let Some((group_type, _, _id, _number)) = detector.check_boundary(&tag_name, &attributes) {
                    // We'll get the title from the text content, so store it as pending
                    pending_title = Some((group_type.clone(), String::new()));
                }
                
                // Check for sutta content start
                if !in_sutta_content && detector.is_sutta_start(&tag_name, &attributes) {
                    // Close any existing fragment
                    if let (Some((start_pos, start_line)), Some(frag_type)) = 
                        (current_fragment_start, current_fragment_type.as_ref()) {
                        
                        let content = xml_content[start_pos..current_pos].to_string();
                        fragments.push(XmlFragment {
                            fragment_type: frag_type.clone(),
                            content,
                            start_line,
                            end_line: current_line,
                            group_levels: hierarchy.get_current_levels(),
                        });
                    }
                    
                    // Start new sutta fragment
                    current_fragment_start = Some((current_pos, current_line));
                    current_fragment_type = Some(FragmentType::Sutta);
                    in_sutta_content = true;
                }
            },
            
            Event::Text(ref e) => {
                let text = e.unescape()
                    .context("Failed to unescape text content")?
                    .trim()
                    .to_string();
                
                // If we have a pending title, update it with this text
                if let Some((group_type, _)) = pending_title.take() {
                    if !text.is_empty() {
                        hierarchy.enter_level(group_type, text, None, None);
                    }
                }
            },
            
            Event::End(ref e) => {
                let name_bytes = e.name();
                let tag_name = std::str::from_utf8(name_bytes.as_ref())
                    .context("Invalid UTF-8 in tag name")?
                    .to_string();
                
                // Check if this closes a sutta div
                if tag_name == "div" && in_sutta_content {
                    // Close current fragment
                    if let (Some((start_pos, start_line)), Some(frag_type)) = 
                        (current_fragment_start, current_fragment_type.as_ref()) {
                        
                        let content = xml_content[start_pos..current_pos].to_string();
                        fragments.push(XmlFragment {
                            fragment_type: frag_type.clone(),
                            content,
                            start_line,
                            end_line: current_line,
                            group_levels: hierarchy.get_current_levels(),
                        });
                        
                        current_fragment_start = None;
                        current_fragment_type = None;
                        in_sutta_content = false;
                    }
                }
            },
            
            Event::Eof => break,
            
            _ => {},
        }
    }
    
    // Close any remaining fragment
    if let (Some((start_pos, start_line)), Some(frag_type)) = 
        (current_fragment_start, current_fragment_type) {
        
        let content = xml_content[start_pos..].to_string();
        fragments.push(XmlFragment {
            fragment_type: frag_type,
            content,
            start_line,
            end_line: reader.current_line(),
            group_levels: hierarchy.get_current_levels(),
        });
    }
    
    Ok(fragments)
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
        
        let fragments = parse_into_fragments(&xml, &structure).expect("Should parse fragments");
        
        // Should have at least one fragment
        assert!(!fragments.is_empty(), "Should have at least one fragment");
    }

    #[test]
    fn test_parse_dn_fragment_count() {
        let xml = create_dn_sample_xml();
        let structure = detect_nikaya_structure(&xml).unwrap();
        let fragments = parse_into_fragments(&xml, &structure).unwrap();
        
        // Count sutta fragments
        let sutta_fragments: Vec<_> = fragments.iter()
            .filter(|f| matches!(f.fragment_type, FragmentType::Sutta))
            .collect();
        
        // Should have one sutta fragment
        assert_eq!(sutta_fragments.len(), 1, "Should have exactly one sutta fragment");
    }

    #[test]
    fn test_parse_dn_line_tracking() {
        let xml = create_dn_sample_xml();
        let structure = detect_nikaya_structure(&xml).unwrap();
        let fragments = parse_into_fragments(&xml, &structure).unwrap();
        
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
        
        let fragments = parse_into_fragments(&xml, &structure).expect("Should parse fragments");
        
        assert!(!fragments.is_empty(), "Should have at least one fragment");
    }

    #[test]
    fn test_fragment_content_not_empty() {
        let xml = create_dn_sample_xml();
        let structure = detect_nikaya_structure(&xml).unwrap();
        let fragments = parse_into_fragments(&xml, &structure).unwrap();
        
        for fragment in &fragments {
            // Each fragment should have non-empty content
            assert!(!fragment.content.trim().is_empty(), 
                    "Fragment content should not be empty");
        }
    }
}
