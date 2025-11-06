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
};
use crate::tipitaka_xml_parser::encoding::read_xml_file;

#[derive(Debug, Clone)]
struct TsvExpectation {
    cst_code: String,
    cst_file: String,
    cst_sutta: String,
    sc_code: String,
}

/// Load TSV expectations for a given XML file
fn load_tsv_expectations(tsv_path: &Path, cst_file: &str) -> Result<Vec<TsvExpectation>> {
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
    let normalized_filename = cst_file.trim_start_matches("romn/");
    
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
    
    let fragments = parse_into_fragments(&xml_content, &nikaya_structure, "s0101m.mul.xml", None, true)
        .expect("Failed to parse fragments");
    
    // Filter to Sutta fragments only
    let sutta_fragments: Vec<_> = fragments.iter()
        .filter(|f| matches!(f.frag_type, crate::tipitaka_xml_parser::types::FragmentType::Sutta))
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
                let fragment_file = fragment.cst_file.trim_start_matches("romn/");
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
    
    let fragments = parse_into_fragments(&xml_content, &nikaya_structure, "s0201m.mul.xml", None, true)
        .expect("Failed to parse fragments");
    
    // Filter to Sutta fragments only
    let sutta_fragments: Vec<_> = fragments.iter()
        .filter(|f| matches!(f.frag_type, crate::tipitaka_xml_parser::types::FragmentType::Sutta))
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
                let fragment_file = fragment.cst_file.trim_start_matches("romn/");
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
fn test_s0201m_first_sutta_fragment() {
    // This test specifically verifies that fragment index 1 (the first sutta fragment)
    // from s0201m.mul.xml has the correct cst_code and cst_sutta values.
    // This ensures the preamble is correctly included with the first sutta.
    
    let xml_path = Path::new("tests/data/s0201m.mul.xml");
    let tsv_path = Path::new("assets/cst-vs-sc.tsv");
    
    // Parse XML file
    let xml_content = read_xml_file(xml_path)
        .expect("Failed to read XML file");
    
    let nikaya_structure = detect_nikaya_structure(&xml_content)
        .expect("Failed to detect nikaya structure");
    
    let fragments = parse_into_fragments(&xml_content, &nikaya_structure, "s0201m.mul.xml", None, true)
        .expect("Failed to parse fragments");
    
    // Fragment 0 should be Header, fragment 1 should be the first Sutta
    assert!(fragments.len() > 1, "Expected at least 2 fragments");
    
    let first_sutta_fragment = &fragments[1];
    
    // Verify fragment type
    assert!(
        matches!(first_sutta_fragment.frag_type, crate::tipitaka_xml_parser::types::FragmentType::Sutta),
        "Fragment 1 should be a Sutta fragment, got: {:?}", first_sutta_fragment.frag_type
    );
    
    // Verify cst_code
    assert_eq!(
        first_sutta_fragment.cst_code.as_deref(),
        Some("mn1.1.1"),
        "Fragment 1 should have cst_code 'mn1.1.1', got: {:?}",
        first_sutta_fragment.cst_code
    );
    
    // Verify cst_sutta
    assert_eq!(
        first_sutta_fragment.cst_sutta.as_deref(),
        Some("1. Mūlapariyāyasuttaṃ"),
        "Fragment 1 should have cst_sutta '1. Mūlapariyāyasuttaṃ', got: {:?}",
        first_sutta_fragment.cst_sutta
    );
    
    // Verify frag_idx
    assert_eq!(
        first_sutta_fragment.frag_idx,
        1,
        "Fragment should have frag_idx 1, got: {}",
        first_sutta_fragment.frag_idx
    );
}

#[test]
fn test_s0201a_att_vagga_zero_fragment() {
    // This test specifically verifies that fragment index 1 from s0201a.att.xml
    // (the commentary file) correctly gets cst_code "mn1.0.0" for the introduction
    // section (vagga 0) which includes <div id="mn1_0" n="mn1_0" type="vagga">
    
    let xml_path = Path::new("tests/data/s0201a.att.xml");
    let xml_content = read_xml_file(xml_path)
        .expect("Failed to read XML file");
    
    let nikaya_structure = detect_nikaya_structure(&xml_content)
        .expect("Failed to detect nikaya structure");
    
    let fragments = parse_into_fragments(&xml_content, &nikaya_structure, "s0201a.att.xml", None, false)
        .expect("Failed to parse fragments");
    
    // Fragment 0 should be Header, fragment 1 should be the introduction (vagga 0)
    assert!(fragments.len() > 2, "Expected at least 3 fragments");
    
    let intro_fragment = &fragments[1];
    
    // Verify fragment type
    assert!(
        matches!(intro_fragment.frag_type, crate::tipitaka_xml_parser::types::FragmentType::Sutta),
        "Fragment 1 should be a Sutta fragment, got: {:?}", intro_fragment.frag_type
    );
    
    // Verify cst_code for vagga 0 (introduction)
    assert_eq!(
        intro_fragment.cst_code.as_deref(),
        Some("mn1.0.0"),
        "Fragment 1 should have cst_code 'mn1.0.0' for vagga 0, got: {:?}",
        intro_fragment.cst_code
    );
    
    // Verify frag_idx
    assert_eq!(
        intro_fragment.frag_idx,
        1,
        "Fragment should have frag_idx 1, got: {}",
        intro_fragment.frag_idx
    );
    
    // Verify that fragment 2 has the correct cst_code for the first real sutta
    let first_sutta_fragment = &fragments[2];
    assert_eq!(
        first_sutta_fragment.cst_code.as_deref(),
        Some("mn1.1.1"),
        "Fragment 2 should have cst_code 'mn1.1.1', got: {:?}",
        first_sutta_fragment.cst_code
    );
}


#[test]
fn test_s0201a_att_kakacupama_sutta() {
    // This test verifies that "1. Kakacūpamasuttavaṇṇanā" in s0201a.att.xml
    // correctly gets cst_code "mn1.3.1" derived from vagga id "mn1_3" and sutta title "1. ..."
    
    let xml_path = Path::new("tests/data/s0201a.att.xml");
    let xml_content = read_xml_file(xml_path)
        .expect("Failed to read XML file");
    
    let nikaya_structure = detect_nikaya_structure(&xml_content)
        .expect("Failed to detect nikaya structure");
    
    let fragments = parse_into_fragments(&xml_content, &nikaya_structure, "s0201a.att.xml", None, false)
        .expect("Failed to parse fragments");
    
    // Find the fragment containing Kakacūpamasuttavaṇṇanā
    let kakacupama_fragment = fragments.iter()
        .find(|f| f.content.contains("Kakacūpamasuttavaṇṇanā"))
        .expect("Should find Kakacūpamasuttavaṇṇanā fragment");
    
    // Verify cst_code
    assert_eq!(
        kakacupama_fragment.cst_code.as_deref(),
        Some("mn1.3.1"),
        "Kakacūpamasuttavaṇṇanā should have cst_code 'mn1.3.1', got: {:?}",
        kakacupama_fragment.cst_code
    );
}

#[test]
fn test_s0201t_tik_culasihanada_sutta() {
    // This test verifies that "1. Cūḷasīhanādasuttavaṇṇanā" in s0201t.tik.xml
    // correctly gets cst_code "mn1.2.1" derived from vagga id "mn1_2" and sutta title "1. ..."
    
    let xml_path = Path::new("tests/data/s0201t.tik.xml");
    let xml_content = read_xml_file(xml_path)
        .expect("Failed to read XML file");
    
    let nikaya_structure = detect_nikaya_structure(&xml_content)
        .expect("Failed to detect nikaya structure");
    
    let fragments = parse_into_fragments(&xml_content, &nikaya_structure, "s0201t.tik.xml", None, false)
        .expect("Failed to parse fragments");
    
    // Find the fragment containing Cūḷasīhanādasuttavaṇṇanā
    let culasihanada_fragment = fragments.iter()
        .find(|f| f.content.contains("Cūḷasīhanādasuttavaṇṇanā"))
        .expect("Should find Cūḷasīhanādasuttavaṇṇanā fragment");
    
    // Verify cst_code
    assert_eq!(
        culasihanada_fragment.cst_code.as_deref(),
        Some("mn1.2.1"),
        "Cūḷasīhanādasuttavaṇṇanā should have cst_code 'mn1.2.1', got: {:?}",
        culasihanada_fragment.cst_code
    );
}
