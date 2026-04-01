//! Custom Tantivy tokenizer filters for Pāli text.
//!
//! Provides two filters and a registration function:
//! - `NiggahitaNormalizer`: normalizes ṃ→ṁ, strips √
//! - `PaliStemmerFilter`: applies the Snowball Pāli stemmer
//! - `register_tokenizers()`: registers `pali_stem` and `simple_fold` analyzers

use std::mem;

use tantivy::tokenizer::{
    AsciiFoldingFilter, LowerCaser, RemoveLongFilter, SimpleTokenizer, TextAnalyzer, Token,
    TokenFilter, TokenStream, Tokenizer,
};

use crate::snowball::{self, Algorithm, Stemmer};

// ---------------------------------------------------------------------------
// NiggahitaNormalizer
// ---------------------------------------------------------------------------

/// Normalizes niggahīta (ṃ U+1E43 → ṁ U+1E41) and strips root marker (√ U+221A).
#[derive(Clone)]
pub struct NiggahitaNormalizer;

impl TokenFilter for NiggahitaNormalizer {
    type Tokenizer<T: Tokenizer> = NiggahitaWrapper<T>;

    fn transform<T: Tokenizer>(self, tokenizer: T) -> Self::Tokenizer<T> {
        NiggahitaWrapper {
            inner: tokenizer,
            buffer: String::new(),
        }
    }
}

#[derive(Clone)]
pub struct NiggahitaWrapper<T> {
    inner: T,
    buffer: String,
}

impl<T: Tokenizer> Tokenizer for NiggahitaWrapper<T> {
    type TokenStream<'a> = NiggahitaStream<'a, T::TokenStream<'a>>;

    fn token_stream<'a>(&'a mut self, text: &'a str) -> Self::TokenStream<'a> {
        self.buffer.clear();
        NiggahitaStream {
            tail: self.inner.token_stream(text),
            buffer: &mut self.buffer,
        }
    }
}

pub struct NiggahitaStream<'a, T> {
    tail: T,
    buffer: &'a mut String,
}

impl<T: TokenStream> NiggahitaStream<'_, T> {
    fn needs_normalization(text: &str) -> bool {
        // Ṃ = U+1E42
        // ṃ = U+1E43
        // √ = U+221A
        text.contains('ṃ') || text.contains('Ṃ') || text.contains('√')
    }

    fn normalize(src: &str, dst: &mut String) {
        dst.clear();
        for ch in src.chars() {
            match ch {
                'ṃ' => dst.push('ṁ'),
                'Ṃ' => dst.push('Ṁ'),
                '√' => {} // strip
                _ => dst.push(ch),
            }
        }
    }
}

impl<T: TokenStream> TokenStream for NiggahitaStream<'_, T> {
    fn advance(&mut self) -> bool {
        if !self.tail.advance() {
            return false;
        }
        if Self::needs_normalization(&self.tail.token().text) {
            Self::normalize(&self.tail.token().text, self.buffer);
            mem::swap(&mut self.tail.token_mut().text, self.buffer);
        }
        true
    }

    fn token(&self) -> &Token {
        self.tail.token()
    }

    fn token_mut(&mut self) -> &mut Token {
        self.tail.token_mut()
    }
}

// ---------------------------------------------------------------------------
// StemmerFilter
// ---------------------------------------------------------------------------

/// Applies a Snowball stemmer (for any language) to each token.
#[derive(Clone)]
pub struct StemmerFilter {
    algorithm: Algorithm,
}

impl StemmerFilter {
    pub fn new(algorithm: Algorithm) -> Self {
        Self { algorithm }
    }
}

impl TokenFilter for StemmerFilter {
    type Tokenizer<T: Tokenizer> = StemmerWrapper<T>;

    fn transform<T: Tokenizer>(self, tokenizer: T) -> Self::Tokenizer<T> {
        StemmerWrapper {
            inner: tokenizer,
            stemmer: Stemmer::create(self.algorithm),
            buffer: String::new(),
        }
    }
}

#[derive(Clone)]
pub struct StemmerWrapper<T> {
    inner: T,
    stemmer: Stemmer,
    buffer: String,
}

impl<T: Tokenizer> Tokenizer for StemmerWrapper<T> {
    type TokenStream<'a> = StemmerStream<'a, T::TokenStream<'a>>;

    fn token_stream<'a>(&'a mut self, text: &'a str) -> Self::TokenStream<'a> {
        self.buffer.clear();
        StemmerStream {
            tail: self.inner.token_stream(text),
            stemmer: &self.stemmer,
            buffer: &mut self.buffer,
        }
    }
}

pub struct StemmerStream<'a, T> {
    tail: T,
    stemmer: &'a Stemmer,
    buffer: &'a mut String,
}

impl<T: TokenStream> TokenStream for StemmerStream<'_, T> {
    fn advance(&mut self) -> bool {
        if !self.tail.advance() {
            return false;
        }
        let stemmed = self.stemmer.stem(&self.tail.token().text);
        self.buffer.clear();
        self.buffer.push_str(&stemmed);
        mem::swap(&mut self.tail.token_mut().text, self.buffer);
        true
    }

    fn token(&self) -> &Token {
        self.tail.token()
    }

    fn token_mut(&mut self) -> &mut Token {
        self.tail.token_mut()
    }
}

// ---------------------------------------------------------------------------
// Registration
// ---------------------------------------------------------------------------

/// Register custom tokenizers with a Tantivy index for the given language.
///
/// Registers three analyzers:
/// - `{lang}_stem`: full stemming pipeline
/// - `simple_fold`: lowercase + ASCII folding (no stemming)
/// - `{lang}_normalize`: normalization without stemming
pub fn register_tokenizers(index: &tantivy::Index, lang: &str) {
    let algorithm = snowball::lang_to_algorithm(lang);

    let lang_stem = TextAnalyzer::builder(SimpleTokenizer::default())
        .filter(RemoveLongFilter::limit(50))
        .filter(LowerCaser)
        .filter(NiggahitaNormalizer)
        .filter(AsciiFoldingFilter)
        .filter(StemmerFilter::new(algorithm))
        .build();

    index
        .tokenizers()
        .register(&format!("{lang}_stem"), lang_stem);

    let simple_fold = TextAnalyzer::builder(SimpleTokenizer::default())
        .filter(LowerCaser)
        .filter(AsciiFoldingFilter)
        .build();

    index.tokenizers().register("simple_fold", simple_fold);

    let lang_normalize = TextAnalyzer::builder(SimpleTokenizer::default())
        .filter(RemoveLongFilter::limit(50))
        .filter(LowerCaser)
        .filter(NiggahitaNormalizer)
        .filter(AsciiFoldingFilter)
        .build();

    index
        .tokenizers()
        .register(&format!("{lang}_normalize"), lang_normalize);
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use tantivy::tokenizer::Token;

    /// Helper: run text through a tokenizer and collect resulting tokens.
    fn tokenize(analyzer: &mut TextAnalyzer, text: &str) -> Vec<Token> {
        let mut tokens = Vec::new();
        let mut stream = analyzer.token_stream(text);
        while let Some(tok) = stream.next() {
            tokens.push(tok.clone());
        }
        tokens
    }

    fn pali_stem_analyzer() -> TextAnalyzer {
        let algorithm = snowball::lang_to_algorithm("pli");

        TextAnalyzer::builder(SimpleTokenizer::default())
            .filter(RemoveLongFilter::limit(50))
            .filter(LowerCaser)
            .filter(NiggahitaNormalizer)
            .filter(AsciiFoldingFilter)
            .filter(StemmerFilter::new(algorithm))
            .build()
    }

    fn simple_fold_analyzer() -> TextAnalyzer {
        TextAnalyzer::builder(SimpleTokenizer::default())
            .filter(LowerCaser)
            .filter(AsciiFoldingFilter)
            .build()
    }

    #[test]
    fn test_pali_stem_basic() {
        let mut a = pali_stem_analyzer();
        let tokens = tokenize(&mut a, "viññāṇānaṁ");
        assert_eq!(tokens.len(), 1);
        // stemmed and folded to ascii
        assert_eq!(tokens[0].text, "vinnana");
    }

    #[test]
    fn test_pali_stem_multiple_words() {
        let mut a = pali_stem_analyzer();
        let tokens = tokenize(&mut a, "bhikkhūnaṁ dhammo");
        assert_eq!(tokens.len(), 2);
        assert_eq!(tokens[0].text, "bhikkhu");
        assert_eq!(tokens[1].text, "dhamma");
    }

    #[test]
    fn test_niggahita_normalization() {
        let mut a = pali_stem_analyzer();
        // ṃ should produce the same result as ṁ
        let tokens_m_dot_below = tokenize(&mut a, "dhammaṃ");
        let tokens_m_dot_above = tokenize(&mut a, "dhammaṁ");
        assert_eq!(tokens_m_dot_below[0].text, tokens_m_dot_above[0].text);
    }

    #[test]
    fn test_root_marker_stripped() {
        let mut a = pali_stem_analyzer();
        let tokens = tokenize(&mut a, "√gam");
        assert_eq!(tokens.len(), 1);
        // √ stripped, "gam" is 3 chars, too short for stemming, passes through
        assert_eq!(tokens[0].text, "gam");
    }

    #[test]
    fn test_ascii_folding() {
        let mut a = pali_stem_analyzer();
        // After stemming "dhammā" → "dhamma", folding is a no-op since dhamma is ASCII.
        // But let's test a word where the stem retains diacritics: "vijjā" stays "vijja" after fold.
        let tokens = tokenize(&mut a, "vijjā");
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].text, "vijja");
    }

    #[test]
    fn test_short_words_unchanged() {
        let mut a = pali_stem_analyzer();
        for word in &["ca", "na", "hi", "pi", "so", "te"] {
            let tokens = tokenize(&mut a, word);
            assert_eq!(tokens.len(), 1, "expected 1 token for '{word}'");
            assert_eq!(tokens[0].text, *word, "short word '{word}' should pass through unchanged");
        }
    }

    #[test]
    fn test_simple_fold_no_stemming() {
        let mut a = simple_fold_analyzer();
        // simple_fold should lowercase and fold but NOT stem
        let tokens = tokenize(&mut a, "Dhammaṁ");
        assert_eq!(tokens.len(), 1);
        // lowercased + folded: diacritics removed but no stemming
        assert_eq!(tokens[0].text, "dhammam");
    }

    #[test]
    fn test_offsets_preserved() {
        let mut a = pali_stem_analyzer();
        let tokens = tokenize(&mut a, "dhammo bhikkhūnaṁ");
        assert_eq!(tokens[0].offset_from, 0);
        assert_eq!(tokens[0].offset_to, 6); // "dhammo" = 6 bytes
        assert_eq!(tokens[1].offset_from, 7);
        // "bhikkhūnaṁ" = 7 ASCII + ū(2 bytes) + na + ṁ(2 bytes) = 13 bytes
        assert_eq!(tokens[1].offset_to, 7 + 13);
    }

    #[test]
    fn test_ascii_input_matches_diacritical() {
        let mut a = pali_stem_analyzer();
        let ascii_tokens = tokenize(&mut a, "vinnanam");
        let diacritical_tokens = tokenize(&mut a, "viññāṇaṁ");
        assert_eq!(ascii_tokens[0].text, diacritical_tokens[0].text,
            "ASCII 'vinnanam' and diacritical 'viññāṇaṁ' should produce the same stem");
    }

    #[test]
    fn test_ascii_stem_anabhijanam() {
        let mut a = pali_stem_analyzer();
        let tokens = tokenize(&mut a, "anabhijanam");
        assert_eq!(tokens.len(), 1);
        // anam suffix (a-stem gen pl) is longest match, same as ānaṁ in original
        assert_eq!(tokens[0].text, "anabhija");
    }

    #[test]
    fn test_ascii_stem_bhikkhunam() {
        let mut a = pali_stem_analyzer();
        let tokens = tokenize(&mut a, "bhikkhunam");
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].text, "bhikkhu");
    }

    #[test]
    fn test_ascii_stem_sattanam() {
        let mut a = pali_stem_analyzer();
        let tokens = tokenize(&mut a, "sattanam");
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].text, "satta");
    }

    #[test]
    fn test_ascii_stem_nibbanam() {
        let mut a = pali_stem_analyzer();
        let tokens = tokenize(&mut a, "nibbanam");
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].text, "nibbana");
    }

    #[test]
    fn test_ascii_stem_dhammo() {
        let mut a = pali_stem_analyzer();
        let tokens = tokenize(&mut a, "dhammo");
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].text, "dhamma");
    }

    // Suffix collision tests: verify correct behavior for merged suffixes
    #[test]
    fn test_collision_ayo_uses_a_stem() {
        // 'ayo' suffix collision: i-stem (<-'i') vs a-stem (<-'a')
        // Resolved to a-stem. E.g. "kannayo" (kaññāyo, fem nom pl of kaññā)
        let mut a = pali_stem_analyzer();
        let tokens = tokenize(&mut a, "kannayo");
        assert_eq!(tokens[0].text, "kanna");
    }

    #[test]
    fn test_collision_inam_merged() {
        // 'inam' from both {ii}na{.m} and ina{.m}, both (<- 'i')
        let mut a = pali_stem_analyzer();
        let tokens = tokenize(&mut a, "agginam");
        assert_eq!(tokens[0].text, "aggi");
    }

    #[test]
    fn test_collision_usu_merged() {
        // 'usu' from both {uu}su and usu, both (<- 'u')
        let mut a = pali_stem_analyzer();
        let tokens = tokenize(&mut a, "bhikkhusu");
        assert_eq!(tokens[0].text, "bhikkhu");
    }

    #[test]
    fn test_identity_noop_prevents_verb_match() {
        // 'a' <- 'a' is a no-op but prevents verb_suffix from firing
        // "dhamma" should stay "dhamma", not be incorrectly verb-stemmed
        let mut a = pali_stem_analyzer();
        let tokens = tokenize(&mut a, "dhamma");
        assert_eq!(tokens[0].text, "dhamma");
    }
}
