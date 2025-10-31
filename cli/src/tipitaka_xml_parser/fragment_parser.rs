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
        let position = self.reader.buffer_position();
        self.update_position(position);
        
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
    // Track: (byte_pos, line_num, char_pos)
    let mut current_fragment_start: Option<(usize, usize, usize)> = None;
    let mut current_fragment_type: Option<FragmentType> = None;
    let mut pending_title: Option<(GroupType, String)> = None;
    let mut in_sutta_content = false;
    
    // Start with a Header fragment at the beginning of the file
    current_fragment_start = Some((0, 1, 0));
    current_fragment_type = Some(FragmentType::Header);
    
    loop {
        let event = reader.read_event()?;
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
                
                // Check for boundary
                if let Some((group_type, _, _id, _number)) = detector.check_boundary(&tag_name, &attributes) {
                    // We'll get the title from the text content, so store it as pending
                    pending_title = Some((group_type.clone(), String::new()));
                }
                
                // Check for sutta content start
                if !in_sutta_content && detector.is_sutta_start(&tag_name, &attributes) {
                    // Close the current Header fragment
                    if let (Some((start_pos, start_line, start_char)), Some(frag_type)) = 
                        (current_fragment_start, current_fragment_type.as_ref()) {
                        
                        let content = xml_content[start_pos..current_pos].to_string();
                        if !content.trim().is_empty() {
                            fragments.push(XmlFragment {
                                fragment_type: frag_type.clone(),
                                content,
                                start_line,
                                end_line: current_line,
                                start_char,
                                end_char: current_char,
                                group_levels: hierarchy.get_current_levels(),
                            });
                        }
                    }
                    
                    // Start new sutta fragment
                    current_fragment_start = Some((current_pos, current_line, current_char));
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
                    // Close current sutta fragment
                    if let (Some((start_pos, start_line, start_char)), Some(frag_type)) = 
                        (current_fragment_start, current_fragment_type.as_ref()) {
                        
                        let content = xml_content[start_pos..current_pos].to_string();
                        if !content.trim().is_empty() {
                            fragments.push(XmlFragment {
                                fragment_type: frag_type.clone(),
                                content,
                                start_line,
                                end_line: current_line,
                                start_char,
                                end_char: current_char,
                                group_levels: hierarchy.get_current_levels(),
                            });
                        }
                        
                        // Start a new Header fragment after the sutta
                        current_fragment_start = Some((current_pos, current_line, current_char));
                        current_fragment_type = Some(FragmentType::Header);
                        in_sutta_content = false;
                    }
                }
            },
            
            Event::Eof => break,
            
            _ => {},
        }
    }
    
    // Close any remaining fragment (usually the final Header fragment)
    if let (Some((start_pos, start_line, start_char)), Some(frag_type)) = 
        (current_fragment_start, current_fragment_type) {
        
        let content = xml_content[start_pos..].to_string();
        if !content.trim().is_empty() {
            fragments.push(XmlFragment {
                fragment_type: frag_type,
                content,
                start_line,
                end_line: reader.current_line(),
                start_char,
                end_char: reader.current_char(),
                group_levels: hierarchy.get_current_levels(),
            });
        }
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

    #[test]
    fn test_character_position_tracking() {
        let xml = create_dn_sample_xml();
        let structure = detect_nikaya_structure(&xml).unwrap();
        let fragments = parse_into_fragments(&xml, &structure).unwrap();
        
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
        let fragments = parse_into_fragments(xml, &structure).unwrap();
        
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
}
