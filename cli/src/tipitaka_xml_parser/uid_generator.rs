//! UID generation for suttas using CST-to-SuttaCentral mapping
//!
//! Uses the cst-vs-sc.tsv file to map CST file and sutta information
//! to SuttaCentral-style codes, generating UIDs in the form: {code}/pli/cst4

use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::Path;
use std::fs;

/// Sutta boundary information from TSV
#[derive(Debug, Clone)]
pub struct SuttaBoundary {
    pub cst_code: String,
    pub title: String,
    pub sc_code: String,
    pub start_paranum: i32,
    pub book: String,       // e.g., "Mūlapaṇṇāsapāḷi"
    pub vagga: String,      // e.g., "5. Cūḷayamakavaggo"
}

/// Mapping from CST identifiers to SuttaCentral codes
#[derive(Debug, Clone)]
pub struct CstMapping {
    /// Map from (cst_file, cst_code) -> code
    /// Example: ("s0201m.mul.xml", "mn1.1.1") -> "mn1"
    file_code_map: HashMap<(String, String), String>,
    
    /// Map from filename -> list of sutta boundaries (sorted by paranum)
    /// Example: "s0201m.mul.xml" -> [(1, "1. Mūlapariyāyasuttaṃ", "mn1"), (14, "2. Sabbāsavasuttaṃ", "mn2"), ...]
    file_boundaries: HashMap<String, Vec<SuttaBoundary>>,
}

impl CstMapping {
    /// Load the CST-to-SC mapping from TSV file
    pub fn load_from_tsv(tsv_path: &Path) -> Result<Self> {
        let content = fs::read_to_string(tsv_path)
            .context(format!("Failed to read TSV file: {:?}", tsv_path))?;
        
        let mut file_code_map = HashMap::new();
        let mut file_boundaries: HashMap<String, Vec<SuttaBoundary>> = HashMap::new();
        
        for (line_num, line) in content.lines().enumerate().skip(1) { // Skip header
            let fields: Vec<&str> = line.split('\t').collect();
            
            if fields.len() < 13 {
                tracing::warn!("Line {}: Not enough fields, skipping", line_num + 1);
                continue;
            }
            
            let cst_code = fields[0].to_string();
            let cst_book = fields[2].to_string();
            let cst_vagga = fields[4].to_string();
            let cst_sutta = fields[5].to_string();
            let cst_paranum_str = fields[6];
            let cst_file = fields[11].to_string();
            let code = fields[12].to_string();
            
            // Extract filename from path (e.g., "romn/s0201m.mul.xml" -> "s0201m.mul.xml")
            let filename = cst_file
                .split('/')
                .last()
                .unwrap_or(&cst_file)
                .to_string();
            
            file_code_map.insert((filename.clone(), cst_code.clone()), code.clone());
            
            // Parse paranum and add sutta boundary
            if let Ok(paranum) = cst_paranum_str.parse::<i32>() {
                let boundary = SuttaBoundary {
                    cst_code: cst_code.clone(),
                    title: cst_sutta,
                    sc_code: code,
                    start_paranum: paranum,
                    book: cst_book,
                    vagga: cst_vagga,
                };
                
                file_boundaries
                    .entry(filename)
                    .or_insert_with(Vec::new)
                    .push(boundary);
            }
        }
        
        // Sort boundaries by paranum for each file
        for boundaries in file_boundaries.values_mut() {
            boundaries.sort_by_key(|b| b.start_paranum);
        }
        
        tracing::info!("Loaded {} CST mappings from TSV", file_code_map.len());
        tracing::info!("Loaded sutta boundaries for {} files", file_boundaries.len());
        
        Ok(Self { file_code_map, file_boundaries })
    }
    
    /// Generate UID for a sutta
    ///
    /// # Arguments
    /// * `xml_filename` - The XML filename (e.g., "s0201m.mul.xml")
    /// * `cst_code` - The CST code (e.g., "mn1.1")
    ///
    /// # Returns
    /// UID in format: {code}/pli/cst4 (e.g., "mn1/pli/cst4")
    pub fn generate_uid(&self, xml_filename: &str, cst_code: &str) -> Option<String> {
        let key = (xml_filename.to_string(), cst_code.to_string());
        
        self.file_code_map.get(&key).map(|code| {
            format!("{}/pli/cst4", code)
        })
    }
    
    /// Generate the base SC code for a sutta without suffixes or language/source
    ///
    /// Returns the mapped SuttaCentral code (e.g., "mn1") if found.
    pub fn generate_code(&self, xml_filename: &str, cst_code: &str) -> Option<String> {
        let key = (xml_filename.to_string(), cst_code.to_string());
        self.file_code_map.get(&key).cloned()
    }

    /// Generate fallback UID when no mapping is found
    ///
    /// Format: {filename_without_ext}/{sutta_index}/pli/cst4
    pub fn generate_fallback_uid(xml_filename: &str, sutta_index: usize) -> String {
        let filename_stem = xml_filename
            .trim_end_matches(".mul.xml")
            .trim_end_matches(".att.xml")
            .trim_end_matches(".tik.xml");
        
        format!("{}/{}/pli/cst4", filename_stem, sutta_index)
    }
    
    /// Get sutta boundaries for a file
    pub fn get_sutta_boundaries(&self, xml_filename: &str) -> Option<&Vec<SuttaBoundary>> {
        self.file_boundaries.get(xml_filename)
    }
    
    /// Determine which sutta a paragraph belongs to based on its number
    pub fn find_sutta_for_paranum(&self, xml_filename: &str, paranum: i32) -> Option<&SuttaBoundary> {
        let boundaries = self.file_boundaries.get(xml_filename)?;
        
        // Find the sutta that starts at or before this paranum
        boundaries.iter()
            .rev() // Search from end
            .find(|b| b.start_paranum <= paranum)
    }
}

/// Extract CST code from sutta title
///
/// Attempts to extract patterns like "mn1.1" from titles like "1. Mūlapariyāyasuttaṃ"
/// Combined with book/vagga context to build the full CST code
pub fn extract_cst_code_from_context(
    book_id: &str,
    vagga_id: Option<&str>,
    sutta_index: usize,
) -> String {
    // Try to extract from div IDs which often contain the CST code
    // Example: book_id="mn1", vagga_id="mn1_1" -> "mn1.{sutta_index}"
    
    if let Some(vagga) = vagga_id {
        // Extract the numeric parts
        // "mn1_1" -> book=1, vagga=1
        if let Some(book_num) = extract_number_from_id(book_id) {
            format!("{}.{}", book_id.trim_end_matches(&book_num.to_string()), sutta_index)
        } else {
            format!("{}.{}", book_id, sutta_index)
        }
    } else {
        format!("{}.{}", book_id, sutta_index)
    }
}

/// Extract trailing number from an ID string
fn extract_number_from_id(id: &str) -> Option<usize> {
    id.chars()
        .rev()
        .take_while(|c| c.is_numeric())
        .collect::<String>()
        .chars()
        .rev()
        .collect::<String>()
        .parse()
        .ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_number_from_id() {
        assert_eq!(extract_number_from_id("mn1"), Some(1));
        assert_eq!(extract_number_from_id("mn1_1"), Some(1));
        assert_eq!(extract_number_from_id("dn2"), Some(2));
        assert_eq!(extract_number_from_id("abc"), None);
    }

    #[test]
    fn test_extract_cst_code_from_context() {
        let code = extract_cst_code_from_context("mn1", Some("mn1_1"), 1);
        assert_eq!(code, "mn.1");
        
        let code = extract_cst_code_from_context("dn2", Some("dn2_3"), 5);
        assert_eq!(code, "dn.5");
    }

    #[test]
    fn test_generate_fallback_uid() {
        let uid = CstMapping::generate_fallback_uid("s0201m.mul.xml", 1);
        assert_eq!(uid, "s0201m/1/pli/cst4");
        
        let uid = CstMapping::generate_fallback_uid("s0102m.mul.xml", 15);
        assert_eq!(uid, "s0102m/15/pli/cst4");
    }
}
