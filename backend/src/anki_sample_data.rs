use serde_json::json;

/// Sample vocabulary data for Anki template preview
/// 
/// This module provides hardcoded sample data for the word "abhivādetvā"
/// to be used in the Anki export dialog preview functionality.
///
/// Get sample vocabulary data as JSON string for preview
/// 
/// Returns a JSON object containing:
/// - word_stem: cleaned word stem
/// - context_snippet: example sentence with word highlighted
/// - original_word: the original word form
/// - vocab: vocabulary lookup result (uid, word, summary)
/// - dpd: full DPD headword data
/// - root: DPD root data (if word has a root)
pub fn get_sample_vocabulary_data_json() -> String {
    let sample_data = json!({
        "word_stem": "karoti",
        "context_snippet": "idha, bhikkhave, bhikkhu puññāni kammāni <b>karoti</b>",
        "original_word": "karoti",
        "vocab": {
            "uid": "20558/dpd",
            "word": "karoti 1",
            "summary": "<i>(pr)</i> does; acts; performs  <b>[√kar + o]</b>"
        },
        "dpd": {
            "id": 20558,
            "uid": "20558/dpd",
            "lemma_1": "karoti 1",
            "lemma_2": "",
            "pos": "pr",
            "grammar": "",
            "derived_from": "",
            "neg": "",
            "verb": "",
            "trans": "",
            "plus_case": "",
            "meaning_1": "does; acts; performs",
            "meaning_lit": "",
            "meaning_2": "",
            "non_ia": "",
            "sanskrit": "",
            "root_key": "√kar",
            "root_sign": "o, yira",
            "root_base": "",
            "family_root_fk": "",
            "family_word_fk": "",
            "family_compound_fk": "",
            "family_idioms_fk": "",
            "family_set_fk": "",
            "construction": "√kar + o",
            "derivative": "",
            "suffix": "",
            "phonetic": "",
            "compound_type": "",
            "compound_construction": "",
            "non_root_in_comps": "",
            "source_1": "",
            "sutta_1": "",
            "example_1": "",
            "source_2": "",
            "sutta_2": "",
            "example_2": "",
            "antonym": "",
            "synonym": "",
            "variant": "",
            "var_phonetic": "",
            "var_text": "",
            "commentary": "",
            "notes": "",
            "cognate": "",
            "link": "",
            "origin": "",
            "stem": "",
            "pattern": "",
            "dictionary_id": 1,
            "word_ascii": "karoti",
            "lemma_clean": "karoti"
        },
        "root": {
            "root": "√kar",
            "root_in_comps": "",
            "root_has_verb": "･",
            "root_group": 7,
            "root_sign": "o, yira",
            "root_meaning": "do, make",
            "sanskrit_root": "√kṛ",
            "sanskrit_root_meaning": "make",
            "sanskrit_root_class": "1, 2, 5, 8",
            "root_example": "karoti, kara, kamma",
            "dhatupatha_num": "526",
            "dhatupatha_root": "kara",
            "dhatupatha_pali": "karaṇe",
            "dhatupatha_english": "doing",
            "dhatumanjusa_num": 740,
            "dhatumanjusa_root": "kara",
            "dhatumanjusa_pali": "karaṇasmiṁ",
            "dhatumanjusa_english": "doing, making",
            "dhatumala_root": "kara",
            "dhatumala_pali": "karaṇe",
            "dhatumala_english": "doing, making",
            "panini_root": "ḍu kṛ ñ",
            "panini_sanskrit": "karaṇe",
            "panini_english": "doing, making",
            "note": "",
            "dictionary_id": 1,
            "uid": "√kar/dpd",
            "word_ascii": "kar",
            "root_clean": "√kar",
            "root_no_sign": "kar"
        }
    });
    
    serde_json::to_string(&sample_data).unwrap_or_else(|_| "{}".to_string())
}
