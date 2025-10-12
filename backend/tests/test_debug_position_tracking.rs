use simsapa_backend::helpers;

#[test]
fn test_position_tracking_with_sandhi() {
    // This mimics what happens in the problematic passage
    let text = r#""iti jānāmi, iti passāmī"ti, tato aparena"#;
    
    let original_normalized = text.replace("\n", " ");
    let preprocessed = helpers::preprocess_text_for_word_extraction(&original_normalized);
    let clean_words = helpers::extract_clean_words(&preprocessed);
    
    println!("Original: '{}'", original_normalized);
    println!("Preprocessed: '{}'", preprocessed);
    println!("Clean words: {:?}", clean_words);
    
    let original_chars: Vec<char> = original_normalized.chars().collect();
    let original_lower = original_normalized.to_lowercase();
    let original_lower_chars: Vec<char> = original_lower.chars().collect();
    
    println!("\nSearching for words sequentially:");
    let mut current_pos = 0;
    
    for (i, word) in clean_words.iter().enumerate() {
        println!("\n[{}] Searching for '{}' starting at pos {}", i, word, current_pos);
        
        if let Some(pos) = helpers::find_word_position_char_based(
            &original_chars,
            &original_lower_chars,
            word,
            current_pos,
        ) {
            println!("  Found at char positions {} to {}", pos.char_start, pos.char_end);
            println!("  Original word: '{}'", pos.original_word);
            current_pos = pos.char_end;
        } else {
            println!("  NOT FOUND!");
            println!("  Remaining text from pos {}: '{}'", 
                current_pos,
                &original_normalized[original_normalized.char_indices().nth(current_pos)
                    .map(|(i, _)| i)
                    .unwrap_or(original_normalized.len())..]);
            break;
        }
    }
}

#[test]
fn test_ti_word_boundary_detection() {
    // The word "ti" appears after quote marks in the original
    let text = r#"passāmī"ti, tato"#;
    
    let chars: Vec<char> = text.chars().collect();
    let lower_chars: Vec<char> = text.to_lowercase().chars().collect();
    
    println!("Text: '{}'", text);
    println!("Chars: {:?}", chars);
    
    // Search for "ti"
    if let Some(pos) = helpers::find_word_position_char_based(&chars, &lower_chars, "ti", 0) {
        println!("Found 'ti' at positions {} to {}", pos.char_start, pos.char_end);
        println!("Original word: '{}'", pos.original_word);
        
        // Check what's before and after
        if pos.char_start > 0 {
            println!("Char before: '{}'", chars[pos.char_start - 1]);
        }
        if pos.char_end < chars.len() {
            println!("Char after: '{}'", chars[pos.char_end]);
        }
    } else {
        println!("'ti' NOT FOUND");
    }
    
    // Also search for "tato"
    if let Some(pos) = helpers::find_word_position_char_based(&chars, &lower_chars, "tato", 0) {
        println!("\nFound 'tato' at positions {} to {}", pos.char_start, pos.char_end);
    } else {
        println!("\n'tato' NOT FOUND");
    }
}

#[test]
fn test_full_problematic_sequence() {
    let text = r#"Yo pana bhikkhu anabhijānaṁ uttarimanussadhammaṁ attupanāyikaṁ alamariyañāṇadassanaṁ samudācareyya "iti jānāmi, iti passāmī"ti, tato aparena samayena samanuggāhīyamāno vā asamanuggāhīyamāno vā āpanno visuddhāpekkho evaṁ vadeyya "ajānamevaṁ āvuso avacaṁ jānāmi, apassaṁ passāmi, tucchaṁ musā vilapi"nti, aññatra adhimānā, ayampi pārājiko hoti asaṁvāso."#;
    
    let words = helpers::extract_words_with_context(text);
    
    println!("Total words extracted: {}", words.len());
    
    // Check words around the problem area (positions 12-15)
    for i in 10..16.min(words.len()) {
        let w = &words[i];
        let found = if w.context_snippet.contains("<b>") { "✓" } else { "✗" };
        println!("[{}] {} clean='{}' original='{}' has_bold={}", 
            i, found, w.clean_word, w.original_word, w.context_snippet.contains("<b>"));
    }
    
    // Check second jānāmi
    let janami_positions: Vec<_> = words.iter()
        .enumerate()
        .filter(|(_, w)| w.clean_word == "jānāmi")
        .collect();
    
    println!("\nFound {} occurrences of 'jānāmi':", janami_positions.len());
    for (idx, word) in janami_positions {
        let has_bold = word.context_snippet.contains("<b>");
        let context_preview: String = word.context_snippet.chars().take(60).collect();
        println!("  [{}] has_bold={} context='{}'", idx, has_bold, context_preview);
    }
}
