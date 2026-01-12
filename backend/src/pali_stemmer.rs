
pub fn pali_stem(word_orig: &str, _replace_vowel: bool) -> String {
    // FIXME: implement pali_stem()
    word_orig.trim_end_matches("ṁ").to_string()
}
