//! Functions for sorting by Pāḷi alphabetical order.
//! Uses ṁ for niggahita.

use std::cmp::Ordering;
use std::collections::HashMap;

use lazy_static::lazy_static;
use regex::{Regex, Captures};
// use serde::{Serialize, Deserialize};

use crate::types::SearchResult;

lazy_static! {
    /// Map of Pali letters to their sort order numbers
    static ref LETTER_TO_NUMBER: HashMap<&'static str, &'static str> = {
        let mut map = HashMap::new();
        map.insert("√", "00");
        map.insert("a", "01");
        map.insert("ā", "02");
        map.insert("i", "03");
        map.insert("ī", "04");
        map.insert("u", "05");
        map.insert("ū", "06");
        map.insert("e", "07");
        map.insert("o", "08");
        map.insert("k", "09");
        map.insert("kh", "10");
        map.insert("g", "11");
        map.insert("gh", "12");
        map.insert("ṅ", "13");
        map.insert("c", "14");
        map.insert("ch", "15");
        map.insert("j", "16");
        map.insert("jh", "17");
        map.insert("ñ", "18");
        map.insert("ṭ", "19");
        map.insert("ṭh", "20");
        map.insert("ḍ", "21");
        map.insert("ḍh", "22");
        map.insert("ṇ", "23");
        map.insert("t", "24");
        map.insert("th", "25");
        map.insert("d", "26");
        map.insert("dh", "27");
        map.insert("n", "28");
        map.insert("p", "29");
        map.insert("ph", "30");
        map.insert("b", "31");
        map.insert("bh", "32");
        map.insert("m", "33");
        map.insert("y", "34");
        map.insert("r", "35");
        map.insert("l", "36");
        map.insert("v", "37");
        map.insert("s", "38");
        map.insert("h", "39");
        map.insert("ḷ", "40");
        map.insert("ṁ", "41");
        map
    };

    /// Map of Sanskrit letters to their sort order numbers
    /// Note: keeping the original typo "sanksrit" for compatibility
    static ref SANSKRIT_LETTER_TO_NUMBER: HashMap<&'static str, &'static str> = {
        let mut map = HashMap::new();
        map.insert("√", "00");
        map.insert("a", "01");
        map.insert("ā", "02");
        map.insert("i", "03");
        map.insert("ī", "04");
        map.insert("u", "05");
        map.insert("ū", "06");
        map.insert("ṛ", "07");
        map.insert("ṝ", "08");
        map.insert("ḷ", "09");
        map.insert("ḹ", "10");
        map.insert("e", "11");
        map.insert("ai", "12");
        map.insert("o", "13");
        map.insert("au", "14");
        map.insert("ḥ", "15");
        map.insert("ṁ", "16");
        map.insert("k", "17");
        map.insert("kh", "18");
        map.insert("g", "19");
        map.insert("gh", "20");
        map.insert("ṅ", "21");
        map.insert("c", "22");
        map.insert("ch", "23");
        map.insert("j", "24");
        map.insert("jh", "25");
        map.insert("ñ", "26");
        map.insert("ṭ", "27");
        map.insert("ṭh", "28");
        map.insert("ḍ", "29");
        map.insert("ḍh", "30");
        map.insert("ṇ", "31");
        map.insert("t", "32");
        map.insert("th", "33");
        map.insert("d", "34");
        map.insert("dh", "35");
        map.insert("n", "36");
        map.insert("p", "37");
        map.insert("ph", "38");
        map.insert("b", "39");
        map.insert("bh", "40");
        map.insert("m", "41");
        map.insert("y", "42");
        map.insert("r", "43");
        map.insert("l", "44");
        map.insert("v", "45");
        map.insert("ś", "46");
        map.insert("ṣ", "47");
        map.insert("s", "48");
        map.insert("h", "49");
        map
    };
}

/// Ordered list of Pali letter patterns (longer patterns first to ensure correct matching)
static PALI_PATTERNS: &[&str] = &[
    "√", "kh", "gh", "ch", "jh", "ṭh", "ḍh", "th", "dh", "ph", "bh",
    "a", "ā", "i", "ī", "u", "ū", "e", "o", "k", "g", "ṅ", "c", "j",
    "ñ", "ṭ", "ḍ", "ṇ", "t", "d", "n", "p", "b", "m", "y", "r", "l",
    "v", "s", "h", "ḷ", "ṁ"
];

/// Ordered list of Sanskrit letter patterns (longer patterns first to ensure correct matching)
static SANSKRIT_PATTERNS: &[&str] = &[
    "√", "ai", "au", "kh", "gh", "ch", "jh", "ṭh", "ḍh", "th", "dh", "ph", "bh",
    "a", "ā", "i", "ī", "u", "ū", "ṛ", "ṝ", "ḷ", "ḹ", "e", "o", "ḥ", "ṁ",
    "k", "g", "ṅ", "c", "j", "ñ", "ṭ", "ḍ", "ṇ", "t", "d", "n", "p", "b",
    "m", "y", "r", "l", "v", "ś", "ṣ", "s", "h"
];

lazy_static! {
    /// Lazy-initialized regex for Pali pattern matching
    static ref PALI_REGEX: Regex = {
        let pattern = PALI_PATTERNS
            .iter()
            .map(|s| regex::escape(s))
            .collect::<Vec<_>>()
            .join("|");
        Regex::new(&pattern).unwrap()
    };

    /// Lazy-initialized regex for Sanskrit pattern matching
    static ref SANSKRIT_REGEX: Regex = {
        let pattern = SANSKRIT_PATTERNS
            .iter()
            .map(|s| regex::escape(s))
            .collect::<Vec<_>>()
            .join("|");
        Regex::new(&pattern).unwrap()
    };
}

/// Sort a vector of words in Pāḷi alphabetical order.
///
/// # Usage
/// ```
/// let sorted = pali_list_sorter(vec!["word1", "word2"]);
/// ```
pub fn pali_list_sorter<S: AsRef<str>>(words: Vec<S>) -> Vec<String> {
    let mut sorted_words: Vec<String> = words
        .into_iter()
        .map(|s| s.as_ref().to_string())
        .collect();

    sorted_words.sort_by_key(|word| pali_sort_key(word));
    sorted_words
}

/// A key for sorting in Pāḷi alphabetical order.
///
/// # Usage
/// ```
/// let mut list = vec!["word1", "word2"];
/// list.sort_by_key(|s| pali_sort_key(s));
/// ```
pub fn pali_sort_key(word: &str) -> String {
    PALI_REGEX.replace_all(word, |caps: &Captures| {
        LETTER_TO_NUMBER
            .get(caps.get(0).unwrap().as_str())
            .unwrap_or(&"")
            .to_string()
    }).to_string()
}

/// A key for sorting in Sanskrit alphabetical order.
///
/// # Usage
/// ```
/// let mut list = vec!["word1", "word2"];
/// list.sort_by_key(|s| sanskrit_sort_key(s));
/// ```
pub fn sanskrit_sort_key(word: &str) -> String {
    SANSKRIT_REGEX.replace_all(word, |caps: &Captures| {
        SANSKRIT_LETTER_TO_NUMBER
            .get(caps.get(0).unwrap().as_str())
            .unwrap_or(&"")
            .to_string()
    }).to_string()
}

/// Trait for types that can provide a sorting key
pub trait SortKey {
    fn to_sort_key(&self) -> String;
}

/// Enum to handle either string or integer values
#[derive(Debug, Clone)]
pub enum WordOrInt {
    Word(String),
    Integer(i32),
}

impl SortKey for WordOrInt {
    fn to_sort_key(&self) -> String {
        match self {
            WordOrInt::Word(s) => s.clone(),
            WordOrInt::Integer(i) => i.to_string(),
        }
    }
}

/// Extended version of pali_sort_key that handles WordOrInt enum
pub fn pali_sort_key_flexible(value: &WordOrInt) -> String {
    match value {
        WordOrInt::Word(word) => pali_sort_key(word),
        WordOrInt::Integer(i) => i.to_string(),
    }
}

/// Extended version of sanskrit_sort_key that handles WordOrInt enum
pub fn sanskrit_sort_key_flexible(value: &WordOrInt) -> String {
    match value {
        WordOrInt::Word(word) => sanskrit_sort_key(word),
        WordOrInt::Integer(i) => i.to_string(),
    }
}

// Natural sorting for dictionary entries

lazy_static! {
    /// Regex for splitting strings into text and numeric segments
    /// Treats each integer as a separate segment, not combining decimals
    static ref NATURAL_SORT_REGEX: Regex = Regex::new(r"(\d+|[^\d]+)").unwrap();
}

/// Enum representing a segment of a string for natural sorting
#[derive(Debug, Clone, PartialEq)]
enum StringSegment {
    Text(String),
    Number(u64),
}

/// Parse a string into segments for natural sorting
fn parse_natural_segments(s: &str) -> Vec<StringSegment> {
    NATURAL_SORT_REGEX
        .find_iter(s)
        .map(|mat| {
            let segment = mat.as_str();
            // Try to parse as an integer
            if let Ok(num) = segment.parse::<u64>() {
                StringSegment::Number(num)
            } else {
                StringSegment::Text(segment.to_lowercase())
            }
        })
        .collect()
}

/// Natural comparison of two string segments
fn compare_segments(a: &StringSegment, b: &StringSegment) -> Ordering {
    match (a, b) {
        (StringSegment::Text(s1), StringSegment::Text(s2)) => s1.cmp(s2),
        (StringSegment::Number(n1), StringSegment::Number(n2)) => n1.cmp(n2),
        // Numbers come before text in natural sorting
        (StringSegment::Number(_), StringSegment::Text(_)) => Ordering::Less,
        (StringSegment::Text(_), StringSegment::Number(_)) => Ordering::Greater,
    }
}

/// Natural sort key for strings containing numbers
///
/// This provides natural/alphanumeric sorting where embedded numbers
/// are sorted numerically rather than lexicographically. It properly handles
/// dictionary entry numbering where entries without minor numbers (e.g., "vajja 2")
/// come before entries with minor numbers (e.g., "vajja 2.1", "vajja 2.2").
///
/// # Example
/// ```
/// let mut entries = vec!["citta 1.10", "citta 1.2", "citta 2", "citta 2.1"];
/// entries.sort_by(|a, b| natural_sort_compare(a, b));
/// // Results in: ["citta 1.2", "citta 1.10", "citta 2", "citta 2.1"]
/// ```
pub fn natural_sort_compare(a: &str, b: &str) -> Ordering {
    let a_segments = parse_natural_segments(a);
    let b_segments = parse_natural_segments(b);

    for (seg_a, seg_b) in a_segments.iter().zip(b_segments.iter()) {
        match compare_segments(seg_a, seg_b) {
            Ordering::Equal => continue,
            other => return other,
        }
    }

    // If all segments are equal, compare by length
    a_segments.len().cmp(&b_segments.len())
}

/// Sort a vector of SearchResult entries by natural word order
///
/// This efficiently sorts dictionary entries considering embedded numbers,
/// so that "citta 1.2" comes before "citta 1.10", and entries without
/// minor numbers (e.g., "vajja 2") come before those with minor numbers
/// (e.g., "vajja 2.1", "vajja 2.2").
///
/// # Example
/// ```
/// let mut results = vec![
///     SearchResult { uid: "1".into(), title: "vajja 2.2".into(), snippet: "...".into() },
///     SearchResult { uid: "2".into(), title: "vajja 2".into(), snippet: "...".into() },
///     SearchResult { uid: "3".into(), title: "vajja 10.1".into(), snippet: "...".into() },
/// ];
/// sort_search_results_natural(&mut results);
/// // Results in order: vajja 2, vajja 2.2, vajja 10.1
/// ```
pub fn sort_search_results_natural(results: &mut Vec<SearchResult>) {
    results.sort_by(|a, b| natural_sort_compare(&a.title, &b.title));
}

/// Alternative: Sort and return a new vector (non-mutating)
///
/// Handles entries with and without minor numbers correctly:
/// "vajja 1" < "vajja 2" < "vajja 2.2" < "vajja 3" < "vajja 4.1" < "vajja 10.1"
pub fn sorted_search_results_natural(mut results: Vec<SearchResult>) -> Vec<SearchResult> {
    results.sort_by(|a, b| natural_sort_compare(&a.title, &b.title));
    results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pali_sort_key() {
        let key = pali_sort_key("kho");
        // "kh" should become "10", "a" should become "08"
        assert_eq!(key, "1008");
    }

    #[test]
    fn test_sanskrit_sort_key() {
        let key = sanskrit_sort_key("kai");
        // "k" should become "17", "ai" should become "12"
        assert_eq!(key, "1712");
    }

    #[test]
    fn test_pali_list_sorter() {
        let words = vec!["vā", "a", "ka"];
        let sorted = pali_list_sorter(words);
        // "a" (01) < "ka" (0901) < "vā" (3702)
        assert_eq!(sorted, vec!["a", "ka", "vā"]);
    }

    #[test]
    fn test_flexible_sort_key() {
        let word = WordOrInt::Word("test".to_string());
        let int = WordOrInt::Integer(42);

        let word_key = pali_sort_key_flexible(&word);
        let int_key = pali_sort_key_flexible(&int);

        assert_eq!(word_key, "24073824");
        assert_eq!(int_key, "42");
    }

    #[test]
    fn test_natural_sort_compare() {
        // Test basic number sorting
        assert_eq!(natural_sort_compare("citta 1.2", "citta 1.10"), Ordering::Less);
        assert_eq!(natural_sort_compare("citta 2.1", "citta 1.10"), Ordering::Greater);
        assert_eq!(natural_sort_compare("citta 1.1", "citta 1.1"), Ordering::Equal);

        // Test entries without minor numbers vs with minor numbers
        assert_eq!(natural_sort_compare("vajja 2", "vajja 2.2"), Ordering::Less);
        assert_eq!(natural_sort_compare("vajja 2", "vajja 2.1"), Ordering::Less);
        assert_eq!(natural_sort_compare("vajja 10", "vajja 10.1"), Ordering::Less);
        assert_eq!(natural_sort_compare("vajja 10.1", "vajja 2"), Ordering::Greater);

        // Test mixed text and numbers
        assert_eq!(natural_sort_compare("abc 10", "abc 2"), Ordering::Greater);
        assert_eq!(natural_sort_compare("version 1.2.3", "version 1.2.10"), Ordering::Less);
    }

    #[test]
    fn test_sort_search_results() {
        let mut results = vec![
            SearchResult::from_title_str("citta 2.6"),
            SearchResult::from_title_str("citta 1.3"),
            SearchResult::from_title_str("citta 1.1"),
            SearchResult::from_title_str("citta 1.10"),
            SearchResult::from_title_str("citta 1.2"),
        ];

        sort_search_results_natural(&mut results);

        let actual_order: Vec<&str> = results.iter().map(|r| r.title.as_str()).collect();
        let expected_order = vec!["citta 1.1", "citta 1.2", "citta 1.3", "citta 1.10", "citta 2.6"];

        assert_eq!(actual_order, expected_order);
    }

    #[test]
    fn test_sort_with_missing_minor_numbers() {
        let mut results = vec![
            SearchResult::from_title_str("vajja 2.2"),
            SearchResult::from_title_str("vajja 10.1"),
            SearchResult::from_title_str("vajja 1"),
            SearchResult::from_title_str("vajja 4.1"),
            SearchResult::from_title_str("vajja 2"),
            SearchResult::from_title_str("vajja 3"),
        ];

        sort_search_results_natural(&mut results);

        let actual_order: Vec<&str> = results.iter().map(|r| r.title.as_str()).collect();
        let expected_order = vec!["vajja 1", "vajja 2", "vajja 2.2", "vajja 3", "vajja 4.1", "vajja 10.1"];

        assert_eq!(actual_order, expected_order);
    }
}
