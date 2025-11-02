//! Tests for XmlFragment line number and character position tracking
//!
//! This test module validates that the position data stored in `XmlFragment` 
//! (`start_line`, `end_line`, `start_char`, `end_char`) accurately represents
//! the location of the fragment content in the original XML file, and that this
//! position data can be used to extract the exact same content from the original
//! XML file.
//!
//! # Test Files
//!
//! The tests use these XML files from the tests/data/ directory:
//! - `s0101m.mul.xml` - Dīgha Nikāya base text (Sīlakkhandhavagga)
//! - `s0101a.att.xml` - Dīgha Nikāya commentary (Aṭṭhakathā)
//! - `s0101t.tik.xml` - Dīgha Nikāya sub-commentary (Ṭīkā)
//!
//! # What These Tests Verify
//!
//! For each fragment parsed from the XML files, the tests:
//! 1. Use the stored position data (start_line, start_char, end_line, end_char)
//! 2. Extract a slice from the original XML file using these positions
//! 3. Verify the extracted slice exactly matches the fragment's content
//!
//! This ensures that:
//! - Position tracking accurately reflects where content is located
//! - Line and character positions are synchronized with byte positions
//! - Fragment content can be reliably located in the source file
//!
//! # Implementation Details
//!
//! The `extract_slice_from_xml()` helper function replicates the position tracking
//! logic from `LineTrackingReader` in fragment_parser.rs:
//! - Iterates through bytes in the XML content
//! - Tracks line numbers (1-indexed) and character positions (0-indexed byte offsets)
//! - Increments line on '\n', resets char to 0
//! - Increments char for all other bytes
//! - Returns the slice between start and end positions
//!
//! # How to Run
//!
//! ```bash
//! # Run all tests in this file
//! cargo test --test test_xml_fragment_position_tracking
//!
//! # Run with output
//! cargo test --test test_xml_fragment_position_tracking -- --nocapture
//!
//! # Run a specific test
//! cargo test test_s0101m_mul_position_tracking
//! ```

use std::path::PathBuf;

/// Helper to extract a slice from XML content using line and character positions
///
/// This function reconstructs the exact slice of content by manually tracking
/// line and character positions, matching the behavior of the LineTrackingReader
/// in the fragment parser.
///
/// **IMPORTANT**: The LineTrackingReader treats `current_char` as a BYTE offset
/// within the current line, NOT a character offset. This is because it increments
/// `current_char` for each byte that isn't a newline. This function replicates
/// that behavior.
///
/// # Arguments
/// * `xml_content` - The original XML content as a string
/// * `start_line` - Starting line number (1-indexed)
/// * `end_line` - Ending line number (1-indexed)
/// * `start_char` - Starting byte offset within start_line (0-indexed)
/// * `end_char` - Ending byte offset within end_line (0-indexed, exclusive)
///
/// # Returns
/// The extracted slice as a String
fn extract_slice_from_xml(
    xml_content: &str,
    start_line: usize,
    end_line: usize,
    start_char: usize,
    end_char: usize,
) -> String {
    let bytes = xml_content.as_bytes();
    let mut current_line = 1;
    let mut current_char = 0;
    let mut start_byte_pos: Option<usize> = None;
    let mut end_byte_pos: Option<usize> = None;
    
    // Track position BEFORE processing each byte (matching LineTrackingReader logic)
    for (byte_idx, &byte) in bytes.iter().enumerate() {
        // Check if we're at the start position BEFORE processing this byte
        if current_line == start_line && current_char == start_char && start_byte_pos.is_none() {
            start_byte_pos = Some(byte_idx);
        }
        
        // Check if we're at the end position BEFORE processing this byte
        if current_line == end_line && current_char == end_char {
            end_byte_pos = Some(byte_idx);
            break;
        }
        
        // Update line and character tracking AFTER checking positions
        // This matches the LineTrackingReader::update_position logic
        if byte == b'\n' {
            current_line += 1;
            current_char = 0;
        } else {
            current_char += 1;
        }
    }
    
    // If we haven't found the end position, check if it's at the very end
    if end_byte_pos.is_none() && current_line == end_line && current_char == end_char {
        end_byte_pos = Some(bytes.len());
    }
    
    match (start_byte_pos, end_byte_pos) {
        (Some(start), Some(end)) => {
            xml_content[start..end].to_string()
        },
        _ => {
            eprintln!("Warning: Could not find byte positions for {}:{} to {}:{}", 
                     start_line, start_char, end_line, end_char);
            eprintln!("  start_byte_pos: {:?}, end_byte_pos: {:?}", start_byte_pos, end_byte_pos);
            String::new()
        }
    }
}

/// Test helper that parses an XML file and validates position tracking
fn test_position_tracking_for_file(xml_path: &str, xml_filename: &str) {
    use simsapa_cli::tipitaka_xml_parser::{detect_nikaya_structure, parse_into_fragments};
    use simsapa_cli::tipitaka_xml_parser_tsv::encoding::read_xml_file;
    
    let path = PathBuf::from(xml_path);
    let xml_content = read_xml_file(&path).expect("Failed to read XML");
    
    let structure = detect_nikaya_structure(&xml_content).expect("Failed to detect nikaya");
    
    let fragments = parse_into_fragments(&xml_content, &structure, xml_filename, None)
        .expect("Failed to parse fragments");
    
    println!("\n=== Testing {} ===", xml_filename);
    println!("Total fragments: {}", fragments.len());
    
    let mut mismatches = Vec::new();
    
    for (idx, fragment) in fragments.iter().enumerate() {
        // Extract slice using position data
        let extracted = extract_slice_from_xml(
            &xml_content,
            fragment.start_line,
            fragment.end_line,
            fragment.start_char,
            fragment.end_char,
        );
        
        // Compare with fragment content
        if extracted != fragment.content {
            mismatches.push((idx, fragment, extracted));
        }
    }
    
    // Report results
    if mismatches.is_empty() {
        println!("✓ All {} fragments have correct position tracking", fragments.len());
    } else {
        println!("✗ Found {} mismatches:", mismatches.len());
        
        for (idx, fragment, extracted) in mismatches.iter().take(5) {
            println!("\n--- Fragment {} ---", idx);
            println!("Position: {}:{} to {}:{}",
                     fragment.start_line, fragment.start_char,
                     fragment.end_line, fragment.end_char);
            println!("Fragment type: {:?}", fragment.fragment_type);
            println!("\nExpected content (first 200 chars):");
            println!("{:?}", fragment.content.chars().take(200).collect::<String>());
            println!("\nExtracted content (first 200 chars):");
            println!("{:?}", extracted.chars().take(200).collect::<String>());
            println!("\nExpected length: {}", fragment.content.len());
            println!("Extracted length: {}", extracted.len());
        }
        
        if mismatches.len() > 5 {
            println!("\n... and {} more mismatches", mismatches.len() - 5);
        }
        
        panic!("Position tracking validation failed for {}: {} out of {} fragments had mismatches",
               xml_filename, mismatches.len(), fragments.len());
    }
}

#[test]
fn test_s0101m_mul_position_tracking() {
    test_position_tracking_for_file(
        "tests/data/s0101m.mul.xml",
        "s0101m.mul.xml"
    );
}

#[test]
fn test_s0101a_att_position_tracking() {
    test_position_tracking_for_file(
        "tests/data/s0101a.att.xml",
        "s0101a.att.xml"
    );
}

#[test]
fn test_s0101t_tik_position_tracking() {
    test_position_tracking_for_file(
        "tests/data/s0101t.tik.xml",
        "s0101t.tik.xml"
    );
}

#[cfg(test)]
mod slice_extraction_tests {
    use super::*;

    #[test]
    fn test_extract_single_line() {
        let xml = "Line 1\nLine 2\nLine 3";
        let result = extract_slice_from_xml(xml, 2, 2, 0, 6);
        assert_eq!(result, "Line 2");
    }

    #[test]
    fn test_extract_single_line_partial() {
        let xml = "Line 1\nLine 2 with more text\nLine 3";
        // Line 2 starts at position 0 (after \n), "2 with " is at char 5-12
        let result = extract_slice_from_xml(xml, 2, 2, 5, 12);
        assert_eq!(result, "2 with ");
    }

    #[test]
    fn test_extract_multi_line() {
        let xml = "Line 1\nLine 2\nLine 3";
        // From start of line 1 to char 6 of line 3
        let result = extract_slice_from_xml(xml, 1, 3, 0, 6);
        assert_eq!(result, "Line 1\nLine 2\nLine 3");
    }

    #[test]
    fn test_extract_multi_line_partial() {
        let xml = "Line 1 start\nLine 2 middle\nLine 3 end";
        // From char 7 of line 1 to char 10 of line 3
        let result = extract_slice_from_xml(xml, 1, 3, 7, 10);
        assert_eq!(result, "start\nLine 2 middle\nLine 3 end");
    }

    #[test]
    fn test_extract_two_lines() {
        let xml = "First line here\nSecond line here\nThird line";
        // From char 6 of line 1 to char 11 of line 2
        let result = extract_slice_from_xml(xml, 1, 2, 6, 11);
        assert_eq!(result, "line here\nSecond line");
    }

    #[test]
    fn test_extract_at_line_boundary() {
        let xml = "ABC\nDEF\nGHI";
        // Extract the newline character between line 1 and 2
        let result = extract_slice_from_xml(xml, 1, 2, 3, 0);
        assert_eq!(result, "\n");
    }

    #[test]
    fn test_char_position_tracking() {
        // Simulate how LineTrackingReader tracks positions
        // After reading "ABC", we're at line 1, char 3
        // After reading "\n", we're at line 2, char 0
        let xml = "ABC\nDEF";
        
        // Extract from start to just before the newline
        let result = extract_slice_from_xml(xml, 1, 1, 0, 3);
        assert_eq!(result, "ABC");
        
        // Extract the newline and first char of next line
        let result = extract_slice_from_xml(xml, 1, 2, 3, 1);
        assert_eq!(result, "\nD");
    }
}
