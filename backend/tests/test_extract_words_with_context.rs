use simsapa_backend::helpers;

#[test]
fn test_mid_sentence_word() {
    let text = "Katamañca bhikkhave samādhindriyaṁ?";
    let words = helpers::extract_words_with_context(text);
    
    assert!(words.len() > 0);
    
    let samadhi_word = words.iter().find(|w| w.clean_word.contains("samādhindriya"));
    assert!(samadhi_word.is_some(), "Should find samādhindriya word");
    
    let word = samadhi_word.unwrap();
    assert!(!word.context_snippet.is_empty(), "Should have context");
    assert!(word.context_snippet.contains("<b>"), "Should have bold tags");
    assert!(word.context_snippet.contains("</b>"), "Should close bold tags");
}

#[test]
fn test_word_at_sentence_boundary() {
    let text = "First word. Second sentence here.";
    let words = helpers::extract_words_with_context(text);
    
    assert!(words.len() >= 4);
    
    let second_word = words.iter().find(|w| w.clean_word.to_lowercase() == "second");
    assert!(second_word.is_some(), "Should find 'second'");
    
    let word = second_word.unwrap();
    assert!(word.context_snippet.contains("Second"), "Context should contain 'Second'");
}

#[test]
fn test_first_word_in_text() {
    let text = "Bhikkhave, listen carefully to my words.";
    let words = helpers::extract_words_with_context(text);
    
    let first_word = &words[0];
    assert_eq!(first_word.clean_word.to_lowercase(), "bhikkhave");
    assert!(first_word.context_snippet.contains("<b>Bhikkhave</b>"));
}

#[test]
fn test_last_word_in_text() {
    let text = "This is the last word here.";
    let words = helpers::extract_words_with_context(text);
    
    let last_word = &words[words.len() - 1];
    assert_eq!(last_word.clean_word, "here");
    assert!(last_word.context_snippet.contains("<b>here</b>"));
}

#[test]
fn test_word_near_start() {
    let text = "Short intro. Then a longer sentence with more content.";
    let words = helpers::extract_words_with_context(text);
    
    let intro_word = words.iter().find(|w| w.clean_word == "intro");
    assert!(intro_word.is_some());
    
    let word = intro_word.unwrap();
    assert!(word.context_snippet.len() < 100, "Context should be limited near start");
}

#[test]
fn test_word_near_end() {
    let text = "A longer sentence with some content before. End word.";
    let words = helpers::extract_words_with_context(text);
    
    let end_word = words.iter().find(|w| w.clean_word.to_lowercase() == "end");
    assert!(end_word.is_some());
}

#[test]
fn test_multiple_occurrences_only_first_bolded() {
    let text = "The word appears here and word appears there too.";
    let words = helpers::extract_words_with_context(text);
    
    let word_entries: Vec<_> = words.iter().filter(|w| w.clean_word == "word").collect();
    
    for word in word_entries {
        let bold_count = word.context_snippet.matches("<b>").count();
        assert_eq!(bold_count, 1, "Should only have one <b> tag");
    }
}

#[test]
fn test_pali_diacritics_preservation() {
    let text = "Idha bhikkhave ariyasāvako sammādiṭṭhisampanno hoti.";
    let words = helpers::extract_words_with_context(text);
    
    let sammadithi = words.iter().find(|w| w.clean_word.contains("sammādiṭṭhi"));
    assert!(sammadithi.is_some(), "Should find sammādiṭṭhi");
    
    let word = sammadithi.unwrap();
    assert!(word.context_snippet.contains("ṭ"), "Should preserve ṭ diacritic");
    assert!(word.original_word.contains("ṭ"), "Original word should have ṭ");
}

#[test]
fn test_sandhi_transformation_iiti() {
    let text = "dhārayāmī'ti";
    let words = helpers::extract_words_with_context(text);
    
    assert!(words.len() >= 2, "Should split into dhārayāmi and ti");
    
    let dharayami = words.iter().find(|w| w.clean_word.contains("dhārayāmi"));
    assert!(dharayami.is_some(), "Should find dhārayāmi");
}

#[test]
fn test_sandhi_transformation_aati() {
    let text = "dassanāyā'ti";
    let words = helpers::extract_words_with_context(text);
    
    assert!(words.len() >= 2, "Should split into dassanāya and ti");
}

#[test]
fn test_sandhi_transformation_uuti() {
    let text = "sikkhāpadesū'ti";
    let words = helpers::extract_words_with_context(text);
    
    assert!(words.len() >= 2, "Should split into sikkhāpadesu and ti");
    
    let sikkha = words.iter().find(|w| w.clean_word.contains("sikkhāpadesu"));
    assert!(sikkha.is_some(), "Should find sikkhāpadesu");
}

#[test]
fn test_sandhi_transformation_nti() {
    let text = "gantun'ti";
    let words = helpers::extract_words_with_context(text);
    
    assert!(words.len() >= 2, "Should split into gantuṁ and ti");
    
    let gantu = words.iter().find(|w| w.clean_word.contains("gantuṁ"));
    assert!(gantu.is_some(), "Should find gantuṁ");
}

#[test]
fn test_empty_text() {
    let text = "";
    let words = helpers::extract_words_with_context(text);
    assert_eq!(words.len(), 0);
}

#[test]
fn test_whitespace_only() {
    let text = "   \n  \t  ";
    let words = helpers::extract_words_with_context(text);
    assert_eq!(words.len(), 0);
}
