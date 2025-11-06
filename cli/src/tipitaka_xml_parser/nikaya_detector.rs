//! Nikaya detection from XML content
//!
//! This module provides functionality to detect which nikaya a given
//! XML file belongs to by analyzing its content.

use anyhow::Result;
use crate::tipitaka_xml_parser::nikaya_structure::NikayaStructure;

/// Detect the nikaya structure from XML content
///
/// Searches for the `<p rend="nikaya">` tag in the XML and extracts the nikaya name,
/// then normalizes it and returns the corresponding structure configuration.
///
/// # Arguments
/// * `xml_content` - The complete XML file content
///
/// # Returns
/// The detected NikayaStructure or an error if detection fails
///
/// # Errors
/// Returns an error if:
/// - No nikaya tag is found in the XML
/// - The nikaya name cannot be normalized (unknown nikaya)
/// - The nikaya name has no corresponding structure configuration
pub fn detect_nikaya_structure(xml_content: &str) -> Result<NikayaStructure> {
    // Search for the nikaya marker: <p rend="nikaya">NikayaName</p>
    let nikaya_tag_start = "<p rend=\"nikaya\">";
    let nikaya_tag_end = "</p>";
    
    // Find the start of the nikaya tag
    let start_idx = xml_content
        .find(nikaya_tag_start)
        .ok_or_else(|| anyhow::anyhow!("Nikaya tag not found in XML content"))?;
    
    // Extract content after the opening tag
    let content_start = start_idx + nikaya_tag_start.len();
    let remaining = &xml_content[content_start..];
    
    // Find the closing tag
    let end_idx = remaining
        .find(nikaya_tag_end)
        .ok_or_else(|| anyhow::anyhow!("Nikaya tag closing not found in XML content"))?;
    
    // Extract the nikaya name
    let nikaya_name = &remaining[..end_idx].trim();
    
    // Normalize the nikaya name
    let normalized_name = NikayaStructure::normalize_name(nikaya_name)
        .ok_or_else(|| anyhow::anyhow!("Unknown nikaya name: '{}'", nikaya_name))?;
    
    // Get the structure configuration
    let structure = NikayaStructure::from_nikaya_name(&normalized_name)
        .ok_or_else(|| anyhow::anyhow!("No structure configuration for nikaya: '{}'", normalized_name))?;
    
    Ok(structure)
}
