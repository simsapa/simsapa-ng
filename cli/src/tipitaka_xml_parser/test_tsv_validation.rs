//! Test to validate parsed CST fields against cst-vs-sc.tsv
//!
//! This test ensures that the cst_code, cst_file, and cst_sutta extracted from
//! XML fragments match the expected values in the TSV mapping file.

use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;
use anyhow::{Result, Context};

use crate::tipitaka_xml_parser::{
    detect_nikaya_structure, 
    parse_into_fragments, 
    populate_sc_fields_from_tsv,
};
use crate::tipitaka_xml_parser_tsv::encoding::read_xml_file;

#[derive(Debug, Clone)]
struct TsvExpectation {
    cst_code: String,
    cst_file: String,
    cst_sutta: String,
    sc_code: String,
}

/// Load TSV expectations for a given XML file
fn load_tsv_expectations(tsv_path: &Path, xml_filename: &str) -> Result<Vec<TsvExpectation>> {
    let file = File::open(tsv_path)
        .with_context(|| format!("Failed to open TSV file: {:?}", tsv_path))?;
    
    let reader = BufReader::new(file);
    let mut lines = reader.lines();
    
    // Read header to find column indices
    let header = lines.next()
        .ok_or_else(|| anyhow::anyhow!("TSV file is empty"))?
        .context("Failed to read header")?;
    
    let columns: Vec<&str> = header.split('\t').collect();
    let cst_code_idx = columns.iter().position(|&c| c == "cst_code")
        .ok_or_else(|| anyhow::anyhow!("Missing 'cst_code' column"))?;
    let cst_file_idx = columns.iter().position(|&c| c == "cst_file")
        .ok_or_else(|| anyhow::anyhow!("Missing 'cst_file' column"))?;
    let cst_sutta_idx = columns.iter().position(|&c| c == "cst_sutta")
        .ok_or_else(|| anyhow::anyhow!("Missing 'cst_sutta' column"))?;
    let sc_code_idx = columns.iter().position(|&c| c == "code")
        .ok_or_else(|| anyhow::anyhow!("Missing 'code' column"))?;
    
    // Normalize filename for comparison (handle both with and without "romn/" prefix)
    let normalized_filename = xml_filename.trim_start_matches("romn/");
    
    // Collect matching rows
    let mut expectations = Vec::new();
    
    for line_result in lines {
        let line = line_result.context("Failed to read TSV line")?;
        if line.trim().is_empty() {
            continue;
        }
        
        let fields: Vec<&str> = line.split('\t').collect();
        
        if let Some(&file_field) = fields.get(cst_file_idx) {
            let file_normalized = file_field.trim_start_matches("romn/");
            
            if file_normalized == normalized_filename {
                if let (Some(&cst_code), Some(&cst_sutta), Some(&sc_code)) = 
                    (fields.get(cst_code_idx), fields.get(cst_sutta_idx), fields.get(sc_code_idx)) {
                    
                    expectations.push(TsvExpectation {
                        cst_code: cst_code.to_string(),
                        cst_file: file_field.to_string(),
                        cst_sutta: cst_sutta.to_string(),
                        sc_code: sc_code.to_string(),
                    });
                }
            }
        }
    }
    
    Ok(expectations)
}

#[test]
fn test_parse_matches_tsv_s0101m() {
    let xml_path = Path::new("tests/data/s0101m.mul.xml");
    let tsv_path = Path::new("assets/cst-vs-sc.tsv");
    
    // Load expectations from TSV
    let expectations = load_tsv_expectations(tsv_path, "s0101m.mul.xml")
        .expect("Failed to load TSV expectations");
    
    assert!(!expectations.is_empty(), "No TSV expectations found for s0101m.mul.xml");
    
    // Parse XML file
    let xml_content = read_xml_file(xml_path)
        .expect("Failed to read XML file");
    
    let nikaya_structure = detect_nikaya_structure(&xml_content)
        .expect("Failed to detect nikaya structure");
    
    let mut fragments = parse_into_fragments(&xml_content, &nikaya_structure, "s0101m.mul.xml", None)
        .expect("Failed to parse fragments");
    
    populate_sc_fields_from_tsv(&mut fragments, tsv_path)
        .expect("Failed to populate SC fields");
    
    // Filter to Sutta fragments only
    let sutta_fragments: Vec<_> = fragments.iter()
        .filter(|f| matches!(f.fragment_type, crate::tipitaka_xml_parser::types::FragmentType::Sutta))
        .collect();
    
    // Build a map of expected data by cst_sutta for easier lookup
    let expected_map: HashMap<String, &TsvExpectation> = expectations.iter()
        .map(|e| (e.cst_sutta.clone(), e))
        .collect();
    
    // Validate each sutta fragment
    let mut errors = Vec::new();
    
    for fragment in &sutta_fragments {
        if let Some(ref cst_sutta) = fragment.cst_sutta {
            if let Some(expected) = expected_map.get(cst_sutta) {
                // Check cst_code
                if fragment.cst_code.as_deref() != Some(&expected.cst_code) {
                    errors.push(format!(
                        "Sutta '{}': expected cst_code '{}', got '{:?}'",
                        cst_sutta, expected.cst_code, fragment.cst_code
                    ));
                }
                
                // Check cst_file
                let fragment_file = fragment.xml_filename.trim_start_matches("romn/");
                let expected_file = expected.cst_file.trim_start_matches("romn/");
                if fragment_file != expected_file {
                    errors.push(format!(
                        "Sutta '{}': expected cst_file '{}', got '{}'",
                        cst_sutta, expected_file, fragment_file
                    ));
                }
                
                // Check sc_code
                if fragment.sc_code.as_deref() != Some(&expected.sc_code) {
                    errors.push(format!(
                        "Sutta '{}': expected sc_code '{}', got '{:?}'",
                        cst_sutta, expected.sc_code, fragment.sc_code
                    ));
                }
            } else {
                errors.push(format!(
                    "Sutta '{}' not found in TSV expectations",
                    cst_sutta
                ));
            }
        }
    }
    
    // Report all errors
    if !errors.is_empty() {
        panic!("TSV validation failed with {} errors:\n{}", 
               errors.len(), errors.join("\n"));
    }
}

#[test]
fn test_parse_matches_tsv_s0201m() {
    let xml_path = Path::new("tests/data/s0201m.mul.xml");
    let tsv_path = Path::new("assets/cst-vs-sc.tsv");
    
    // Load expectations from TSV
    let expectations = load_tsv_expectations(tsv_path, "s0201m.mul.xml")
        .expect("Failed to load TSV expectations");
    
    assert!(!expectations.is_empty(), "No TSV expectations found for s0201m.mul.xml");
    
    // Parse XML file
    let xml_content = read_xml_file(xml_path)
        .expect("Failed to read XML file");
    
    let nikaya_structure = detect_nikaya_structure(&xml_content)
        .expect("Failed to detect nikaya structure");
    
    let mut fragments = parse_into_fragments(&xml_content, &nikaya_structure, "s0201m.mul.xml", None)
        .expect("Failed to parse fragments");
    
    populate_sc_fields_from_tsv(&mut fragments, tsv_path)
        .expect("Failed to populate SC fields");
    
    // Filter to Sutta fragments only
    let sutta_fragments: Vec<_> = fragments.iter()
        .filter(|f| matches!(f.fragment_type, crate::tipitaka_xml_parser::types::FragmentType::Sutta))
        .collect();
    
    // Build a map of expected data by cst_sutta for easier lookup
    let expected_map: HashMap<String, &TsvExpectation> = expectations.iter()
        .map(|e| (e.cst_sutta.clone(), e))
        .collect();
    
    // Validate each sutta fragment
    let mut errors = Vec::new();
    
    for fragment in &sutta_fragments {
        if let Some(ref cst_sutta) = fragment.cst_sutta {
            if let Some(expected) = expected_map.get(cst_sutta) {
                // Check cst_code
                if fragment.cst_code.as_deref() != Some(&expected.cst_code) {
                    errors.push(format!(
                        "Sutta '{}': expected cst_code '{}', got '{:?}'",
                        cst_sutta, expected.cst_code, fragment.cst_code
                    ));
                }
                
                // Check cst_file
                let fragment_file = fragment.xml_filename.trim_start_matches("romn/");
                let expected_file = expected.cst_file.trim_start_matches("romn/");
                if fragment_file != expected_file {
                    errors.push(format!(
                        "Sutta '{}': expected cst_file '{}', got '{}'",
                        cst_sutta, expected_file, fragment_file
                    ));
                }
                
                // Check sc_code
                if fragment.sc_code.as_deref() != Some(&expected.sc_code) {
                    errors.push(format!(
                        "Sutta '{}': expected sc_code '{}', got '{:?}'",
                        cst_sutta, expected.sc_code, fragment.sc_code
                    ));
                }
            } else {
                errors.push(format!(
                    "Sutta '{}' not found in TSV expectations",
                    cst_sutta
                ));
            }
        }
    }
    
    // Report all errors
    if !errors.is_empty() {
        panic!("TSV validation failed with {} errors:\n{}", 
               errors.len(), errors.join("\n"));
    }
}
