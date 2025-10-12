use simsapa_backend::helpers;

#[test]
fn test_stage_1_text_preprocessing() {
    let text = "dhārayāmī'ti sikkhāpadesū'ti";
    let preprocessed = helpers::preprocess_text_for_word_extraction(text);
    
    assert!(preprocessed.contains("dhārayāmi ti"));
    assert!(preprocessed.contains("sikkhāpadesu ti"));
    assert!(!preprocessed.contains("'"));
}

#[test]
fn test_stage_2_clean_word_extraction() {
    let preprocessed = "dhārayāmi ti sikkhāpadesu ti";
    let words = helpers::extract_clean_words(preprocessed);
    
    assert_eq!(words.len(), 4);
    assert_eq!(words[0], "dhārayāmi");
    assert_eq!(words[1], "ti");
    assert_eq!(words[2], "sikkhāpadesu");
    assert_eq!(words[3], "ti");
}

#[test]
fn test_stage_3_word_position_finding_simple() {
    let text = "hello world test";
    let chars: Vec<char> = text.chars().collect();
    let lower_chars: Vec<char> = text.to_lowercase().chars().collect();
    
    let pos = helpers::find_word_position_char_based(&chars, &lower_chars, "world", 0);
    assert!(pos.is_some());
    let pos = pos.unwrap();
    assert_eq!(pos.char_start, 6);
    assert_eq!(pos.char_end, 11);
    assert_eq!(pos.original_word, "world");
}

#[test]
fn test_stage_3_word_position_with_diacritics() {
    let text = "idha bhikkhave sammādiṭṭhi";
    let chars: Vec<char> = text.chars().collect();
    let lower_chars: Vec<char> = text.to_lowercase().chars().collect();
    
    let pos = helpers::find_word_position_char_based(&chars, &lower_chars, "sammādiṭṭhi", 0);
    assert!(pos.is_some());
    let pos = pos.unwrap();
    assert_eq!(pos.original_word, "sammādiṭṭhi");
    println!("Found 'sammādiṭṭhi' at char positions {} to {}", pos.char_start, pos.char_end);
}

#[test]
fn test_stage_3_repeated_word_sequential_search() {
    let text = "iti jānāmi iti passāmi";
    let chars: Vec<char> = text.chars().collect();
    let lower_chars: Vec<char> = text.to_lowercase().chars().collect();
    
    let first_iti = helpers::find_word_position_char_based(&chars, &lower_chars, "iti", 0);
    assert!(first_iti.is_some());
    let first = first_iti.unwrap();
    assert_eq!(first.char_start, 0);
    assert_eq!(first.char_end, 3);
    
    let second_iti = helpers::find_word_position_char_based(&chars, &lower_chars, "iti", first.char_end);
    assert!(second_iti.is_some());
    let second = second_iti.unwrap();
    assert_eq!(second.char_start, 11);
    assert_eq!(second.char_end, 14);
    assert_ne!(first.char_start, second.char_start, "Two 'iti' should be at different positions");
}

#[test]
fn test_stage_4_context_boundaries() {
    let text = "First sentence. Second sentence here. Third one.";
    let chars: Vec<char> = text.chars().collect();
    let lower_chars: Vec<char> = text.to_lowercase().chars().collect();
    
    let pos = helpers::find_word_position_char_based(&chars, &lower_chars, "sentence", 16);
    assert!(pos.is_some());
    let pos = pos.unwrap();
    
    let boundaries = helpers::calculate_context_boundaries(&pos, text, chars.len());
    
    assert_eq!(boundaries.context_start, 16);
    assert_eq!(boundaries.context_end, 37);
    
    println!("Word '{}' at positions {} to {}", pos.original_word, pos.char_start, pos.char_end);
    println!("Context from {} to {}", boundaries.context_start, boundaries.context_end);
}

#[test]
fn test_stage_5_context_snippet_with_bold() {
    let text = "idha bhikkhave sammādiṭṭhi";
    let chars: Vec<char> = text.chars().collect();
    
    let boundaries = helpers::ContextBoundaries {
        context_start: 0,
        context_end: chars.len(),
        word_start: 5,
        word_end: 14,
    };
    
    let snippet = helpers::build_context_snippet(&chars, &boundaries);
    assert!(snippet.contains("<b>bhikkhave</b>"));
    assert!(snippet.contains("idha"));
    assert!(snippet.contains("sammādiṭṭhi"));
    println!("Snippet: {}", snippet);
}

#[test]
fn test_integrated_extraction_simple_repeated_words() {
    let text = "word test word again";
    let words = helpers::extract_words_with_context(text);
    
    let word_occurrences: Vec<_> = words.iter()
        .enumerate()
        .filter(|(_, w)| w.clean_word == "word")
        .collect();
    
    assert_eq!(word_occurrences.len(), 2);
    
    for (idx, word) in word_occurrences {
        println!("'word' at idx {}: original='{}', context='{}'", 
            idx, word.original_word, word.context_snippet);
        assert!(!word.original_word.is_empty());
        assert!(word.context_snippet.contains("<b>"));
    }
}
