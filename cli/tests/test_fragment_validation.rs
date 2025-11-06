//! Tests to validate fragment parsing against TSV data

use std::path::PathBuf;

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
    
    let structure = detect_nikaya_structure(&xml_content).expect("Failed to detect nikaya");
    
    let fragments = parse_into_fragments(&xml_content, &structure, "s0101m.mul.xml", None).expect("Failed to parse fragments");
    
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
    
    let structure = detect_nikaya_structure(&xml_content).expect("Failed to detect nikaya");
    
    let fragments = parse_into_fragments(&xml_content, &structure, "s0201m.mul.xml", None).expect("Failed to parse fragments");
    
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

#[test]
fn test_mn_vagga_div_in_correct_fragment() {
    use simsapa_cli::tipitaka_xml_parser::{detect_nikaya_structure, parse_into_fragments};
    use simsapa_cli::tipitaka_xml_parser_tsv::encoding::read_xml_file;
    use simsapa_cli::tipitaka_xml_parser::FragmentType;
    
    // Parse the XML file
    let xml_path = PathBuf::from("tests/data/s0201m.mul.xml");
    let xml_content = read_xml_file(&xml_path).expect("Failed to read XML");
    
    let structure = detect_nikaya_structure(&xml_content).expect("Failed to detect nikaya");
    
    let fragments = parse_into_fragments(&xml_content, &structure, "s0201m.mul.xml", None).expect("Failed to parse fragments");
    
    // Find the fragment containing the second vagga (line 1667: <div id="mn1_2" n="mn1_2" type="vagga">)
    // This should be in the same fragment as the first sutta of that vagga (line 1674: "1. Cūḷasīhanādasuttaṃ")
    let sutta_fragments: Vec<_> = fragments.iter()
        .filter(|f| matches!(f.fragment_type, FragmentType::Sutta))
        .collect();
    
    // Find the fragment that contains "Cūḷasīhanādasuttaṃ" (first sutta of second vagga)
    let culasutta_fragment = sutta_fragments.iter()
        .find(|f| f.content.contains("Cūḷasīhanādasuttaṃ"))
        .expect("Should find fragment containing Cūḷasīhanādasuttaṃ");
    
    // Verify that the vagga div is in the same fragment
    assert!(
        culasutta_fragment.content.contains(r#"<div id="mn1_2" n="mn1_2" type="vagga">"#),
        "The vagga opening <div> tag should be in the same fragment as its first sutta.\n\
         Fragment start line: {}, end line: {}\n\
         Fragment content (first 200 chars): {}",
        culasutta_fragment.start_line,
        culasutta_fragment.end_line,
        &culasutta_fragment.content[..culasutta_fragment.content.len().min(200)]
    );
    
    // Also verify the vagga title is present
    assert!(
        culasutta_fragment.content.contains("Sīhanādavaggo"),
        "The vagga title should be in the same fragment"
    );
    
    // Find the previous sutta fragment (which should contain "Sallekhasammādiṭṭhisatipaṭṭhaṃ" from line 1664)
    // and verify that the vagga div is NOT in that fragment
    let previous_fragment = sutta_fragments.iter()
        .find(|f| f.content.contains("Sallekhasammādiṭṭhisatipaṭṭhaṃ"))
        .expect("Should find previous sutta fragment");
    
    assert!(
        !previous_fragment.content.contains(r#"<div id="mn1_2" n="mn1_2" type="vagga">"#),
        "The vagga opening <div> tag should NOT be in the previous sutta's fragment.\n\
         Previous fragment lines: {}-{}",
        previous_fragment.start_line,
        previous_fragment.end_line
    );
    
    println!("✓ Vagga div is correctly placed in the fragment with its first sutta");
    println!("  Fragment lines: {}-{}", culasutta_fragment.start_line, culasutta_fragment.end_line);
    println!("✓ Vagga div is NOT in the previous sutta's fragment");
    println!("  Previous fragment lines: {}-{}", previous_fragment.start_line, previous_fragment.end_line);
}
