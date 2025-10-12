#[test]
fn test_sandhi_transformation() {
    let text = r#"passāmī"ti, tato"#;
    println!("Original: '{}'", text);
    
    let after = text.replace("ī\"ti", "i ti");
    println!("After sandhi: '{}'", after);
    
    println!("\nSearching for 'ti' in original:");
    if let Some(pos) = text.to_lowercase().find("ti") {
        println!("  Found at byte pos {}: '{}'", pos, &text[pos..pos+2]);
    }
    
    println!("\nSearching for 'tato' in original:");
    if let Some(pos) = text.to_lowercase().find("tato") {
        println!("  Found at byte pos {}: '{}'", pos, &text[pos..pos+4]);
    }
    
    let text2 = "passāmi ti, tato";
    println!("\n\nIf we had 'passāmi ti, tato':");
    if let Some(pos) = text2.find("ti") {
        println!("  'ti' found at byte pos {}", pos);
    }
    if let Some(pos) = text2.find("tato") {
        println!("  'tato' found at byte pos {}", pos);
    }
}
