/// Sample vocabulary data for Anki template preview
/// 
/// This module provides hardcoded sample data for the word "abhivādetvā"
/// to be used in the Anki export dialog preview functionality.

use serde_json::json;

/// Get sample vocabulary data as JSON string for preview
/// 
/// Returns a JSON object containing:
/// - word_stem: cleaned word stem
/// - context_snippet: example sentence with word highlighted
/// - original_word: the original word form
/// - vocab: vocabulary lookup result (uid, word, summary)
/// - dpd: full DPD headword data
pub fn get_sample_vocabulary_data_json() -> String {
    let sample_data = json!({
        "word_stem": "abhivādeti",
        "context_snippet": "upasaṅkamitvā bhagavantaṁ <b>abhivādetvā</b> ekamantaṁ nisīdi.",
        "original_word": "abhivādetvā",
        "vocab": {
            "uid": "157/dpd",
            "word": "abhivādeti",
            "summary": "<i>(pr)</i> greets respectfully; pays respect (to); salutes  <b>[abhi + vādeti]</b>  <i>pr</i>"
        },
        "dpd": {
            "id": 157,
            "uid": "157/dpd",
            "lemma_1": "abhivādeti",
            "lemma_2": "",
            "pos": "pr",
            "grammar": "",
            "derived_from": "",
            "neg": "",
            "verb": "",
            "trans": "",
            "plus_case": "",
            "meaning_1": "greets respectfully; pays respect (to); salutes",
            "meaning_lit": "",
            "meaning_2": "",
            "non_ia": "",
            "sanskrit": "",
            "root_key": "",
            "root_sign": "",
            "root_base": "",
            "family_root_fk": "",
            "family_word_fk": "",
            "family_compound_fk": "",
            "family_idioms_fk": "",
            "family_set_fk": "",
            "construction": "abhi + vādeti",
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
            "inflections": "",
            "inflections_api_ca_eva_iti": "",
            "inflections_sinhala": "",
            "inflections_devanagari": "",
            "inflections_thai": "",
            "inflections_html": "",
            "freq_data": "",
            "freq_html": "",
            "ebt_count": 0,
            "dictionary_id": 1,
            "word_ascii": "abhivadeti",
            "lemma_clean": "abhivādeti"
        }
    });
    
    serde_json::to_string(&sample_data).unwrap_or_else(|_| "{}".to_string())
}
