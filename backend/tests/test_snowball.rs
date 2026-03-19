use simsapa_backend::snowball::{Algorithm, Stemmer, lang_to_algorithm};

// NOTE: These are very basic tests to verify minimal stemmer funcionality.
// A more extensive stemmer test suite is found in the pali-stemmer-in-snowball/ project.

fn stems_to(input: &str, expected: &str, algo: Algorithm) {
    let stemmer = Stemmer::create(algo);
    let result = stemmer.stem(input);
    assert_eq!(result, expected, "stemming '{}' with {:?}: got '{}', expected '{}'", input, algo, result, expected);
}

#[test]
fn test_pali_a_stem_basic() {
    stems_to("dhammo", "dhamma", Algorithm::Pali);
    stems_to("dhammassa", "dhamma", Algorithm::Pali);
}

#[test]
fn test_pali_u_stem() {
    stems_to("bhikkhūnaṁ", "bhikkhu", Algorithm::Pali);
}

#[test]
fn test_pali_exception_list() {
    stems_to("nibbānaṁ", "nibbāna", Algorithm::Pali);
}

#[test]
fn test_pali_consonantal_stem() {
    stems_to("bhagavantaṁ", "bhagavant", Algorithm::Pali);
}

#[test]
fn test_pali_verb_forms() {
    let stemmer = Stemmer::create(Algorithm::Pali);
    let result = stemmer.stem("vadeyya");
    assert_eq!(result, "vadati", "vadeyya should stem to vadati");
}

#[test]
fn test_english_stemmer() {
    stems_to("suffering", "suffer", Algorithm::English);
    stems_to("running", "run", Algorithm::English);
}

#[test]
fn test_hungarian_stemmer() {
    stems_to("kutyák", "kutya", Algorithm::Hungarian);
    stems_to("látják", "látja", Algorithm::Hungarian);
}

#[test]
fn test_lang_to_algorithm_mappings() {
    assert_eq!(lang_to_algorithm("pli"), Algorithm::Pali);
    assert_eq!(lang_to_algorithm("san"), Algorithm::Pali);
    assert_eq!(lang_to_algorithm("en"), Algorithm::English);
    assert_eq!(lang_to_algorithm("de"), Algorithm::German);
}

#[test]
fn test_lang_to_algorithm_fallback() {
    // Unknown language codes should fall back to English
    assert_eq!(lang_to_algorithm("xx"), Algorithm::English);
    assert_eq!(lang_to_algorithm("zz"), Algorithm::English);
    assert_eq!(lang_to_algorithm(""), Algorithm::English);
}
