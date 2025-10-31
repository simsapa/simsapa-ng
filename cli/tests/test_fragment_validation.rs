//! Tests to validate fragment parsing against TSV data

use std::path::PathBuf;
use std::collections::HashMap;

fn load_tsv_suttas(file_pattern: &str) -> Vec<String> {
    use std::fs;
    
    let tsv_path = PathBuf::from("assets/cst-vs-sc.tsv");
    let content = fs::read_to_string(&tsv_path).expect("Failed to read TSV file");
    
    let mut suttas = Vec::new();
    for line in content.lines().skip(1) { // Skip header
        let fields: Vec<&str> = line.split('\t').collect();
        if fields.len() > 11 {
            let cst_file = fields[11];
            let cst_sutta = fields[5]; // Column 6 (0-indexed 5)
            
            if cst_file == file_pattern {
                suttas.push(cst_sutta.to_string());
            }
        }
    }
    
    suttas
}

#[test]
fn test_dn_s0101m_sutta_count() {
    use simsapa_cli::tipitaka_xml_parser::{detect_nikaya_structure, parse_into_fragments};
    use simsapa_cli::tipitaka_xml_parser_tsv::encoding::read_xml_file;
    use simsapa_cli::tipitaka_xml_parser::FragmentType;
    
    // Load expected suttas from TSV
    let expected_suttas = load_tsv_suttas("romn/s0101m.mul.xml");
    println!("Expected DN suttas: {}", expected_suttas.len());
    for (i, sutta) in expected_suttas.iter().enumerate() {
        println!("  {}. {}", i+1, sutta);
    }
    
    // Parse the XML file
    let xml_path = PathBuf::from("tests/data/s0101m.mul.xml");
    let xml_content = read_xml_file(&xml_path).expect("Failed to read XML");
    
    let mut structure = detect_nikaya_structure(&xml_content).expect("Failed to detect nikaya");
    structure = structure.with_xml_filename("s0101m.mul.xml".to_string());
    
    let fragments = parse_into_fragments(&xml_content, &structure).expect("Failed to parse fragments");
    
    // Count sutta fragments
    let sutta_fragments: Vec<_> = fragments.iter()
        .filter(|f| matches!(f.fragment_type, FragmentType::Sutta))
        .collect();
    
    println!("Parsed sutta fragments: {}", sutta_fragments.len());
    
    assert_eq!(
        sutta_fragments.len(),
        expected_suttas.len(),
        "DN: Expected {} suttas but found {} sutta fragments",
        expected_suttas.len(),
        sutta_fragments.len()
    );
}

#[test]
fn test_mn_s0201m_sutta_count() {
    use simsapa_cli::tipitaka_xml_parser::{detect_nikaya_structure, parse_into_fragments};
    use simsapa_cli::tipitaka_xml_parser_tsv::encoding::read_xml_file;
    use simsapa_cli::tipitaka_xml_parser::FragmentType;
    
    // Load expected suttas from TSV
    let expected_suttas = load_tsv_suttas("romn/s0201m.mul.xml");
    println!("Expected MN suttas: {}", expected_suttas.len());
    for (i, sutta) in expected_suttas.iter().take(10).enumerate() {
        println!("  {}. {}", i+1, sutta);
    }
    if expected_suttas.len() > 10 {
        println!("  ... and {} more", expected_suttas.len() - 10);
    }
    
    // Parse the XML file
    let xml_path = PathBuf::from("tests/data/s0201m.mul.xml");
    let xml_content = read_xml_file(&xml_path).expect("Failed to read XML");
    
    let mut structure = detect_nikaya_structure(&xml_content).expect("Failed to detect nikaya");
    structure = structure.with_xml_filename("s0201m.mul.xml".to_string());
    
    let fragments = parse_into_fragments(&xml_content, &structure).expect("Failed to parse fragments");
    
    // Count sutta fragments
    let sutta_fragments: Vec<_> = fragments.iter()
        .filter(|f| matches!(f.fragment_type, FragmentType::Sutta))
        .collect();
    
    println!("Parsed sutta fragments: {}", sutta_fragments.len());
    
    assert_eq!(
        sutta_fragments.len(),
        expected_suttas.len(),
        "MN: Expected {} suttas but found {} sutta fragments",
        expected_suttas.len(),
        sutta_fragments.len()
    );
}
