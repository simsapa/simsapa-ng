use simsapa_backend::helpers;

#[test]
fn test_find_sentence_start_at_beginning() {
    let text = "First sentence. Second sentence.";
    let start = helpers::find_sentence_start(text, 0);
    assert_eq!(start, 0);
}

#[test]
fn test_find_sentence_start_after_period() {
    let text = "First sentence. Second sentence.";
    let start = helpers::find_sentence_start(text, 20);
    assert_eq!(start, 16);
}

#[test]
fn test_find_sentence_start_after_question() {
    let text = "What is this? Another question.";
    let start = helpers::find_sentence_start(text, 20);
    assert_eq!(start, 14);
}

#[test]
fn test_find_sentence_start_after_exclamation() {
    let text = "Watch out! Be careful.";
    let start = helpers::find_sentence_start(text, 15);
    assert_eq!(start, 11);
}

#[test]
fn test_find_sentence_start_empty_text() {
    let text = "";
    let start = helpers::find_sentence_start(text, 0);
    assert_eq!(start, 0);
}

#[test]
fn test_find_sentence_start_no_boundary() {
    let text = "Single sentence without ending";
    let start = helpers::find_sentence_start(text, 20);
    assert_eq!(start, 0);
}

#[test]
fn test_find_sentence_end_at_period() {
    let text = "First sentence. Second sentence.";
    let end = helpers::find_sentence_end(text, 0);
    assert_eq!(end, 14);
}

#[test]
fn test_find_sentence_end_at_question() {
    let text = "What is this? Another question.";
    let end = helpers::find_sentence_end(text, 0);
    assert_eq!(end, 12);
}

#[test]
fn test_find_sentence_end_at_exclamation() {
    let text = "Watch out! Be careful.";
    let end = helpers::find_sentence_end(text, 0);
    assert_eq!(end, 9);
}

#[test]
fn test_find_sentence_end_no_boundary() {
    let text = "Single sentence without ending";
    let end = helpers::find_sentence_end(text, 0);
    assert_eq!(end, text.chars().count());
}

#[test]
fn test_find_sentence_end_beyond_text() {
    let text = "Short text.";
    let end = helpers::find_sentence_end(text, 100);
    assert_eq!(end, text.chars().count());
}

#[test]
fn test_pali_diacritics_sentence_start() {
    let text = "Katamañca bhikkhave samādhindriyaṁ? Idha bhikkhave ariyasāvako.";
    let start = helpers::find_sentence_start(text, 45);
    assert_eq!(start, 36);
}

#[test]
fn test_pali_diacritics_sentence_end() {
    let text = "Katamañca bhikkhave samādhindriyaṁ? Idha bhikkhave ariyasāvako.";
    let end = helpers::find_sentence_end(text, 0);
    assert_eq!(end, 34);
}
