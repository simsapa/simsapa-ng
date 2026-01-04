//! CIPS Topic Index Parser
//!
//! Parses the CIPS (Comprehensive Index of Pāli Suttas) general-index.csv file
//! and generates a JSON file for static inclusion in the Simsapa app.
//!
//! The CSV is tab-delimited with 3 columns: headword, subheading, locator

use std::collections::HashMap;
use std::path::Path;
use std::fs;

use anyhow::{Context, Result};
use regex::Regex;
use serde::{Deserialize, Serialize};
use lazy_static::lazy_static;

use simsapa_backend::helpers::latinize;

// ============================================================================
// Data Structures for JSON Output
// ============================================================================

/// A reference within a topic entry (either a sutta reference or cross-reference)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopicIndexRef {
    /// For sutta type: lowercase sutta reference with segment ID (e.g., "dn33:1.11.0")
    /// For xref type: target headword name
    #[serde(rename = "sutta_ref", skip_serializing_if = "Option::is_none")]
    pub sutta_ref: Option<String>,

    /// For xref type: target headword reference
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ref_target: Option<String>,

    /// Pāli title of the sutta (only for sutta type)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,

    /// Type of reference: "sutta" or "xref"
    #[serde(rename = "type")]
    pub ref_type: String,
}

/// A sub-entry within a headword
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopicIndexEntry {
    /// Sub-entry text (e.g., "blemishes in oneself")
    /// Empty string for entries directly linking to a headword
    /// "—" (em-dash) for direct headword→sutta links without sub-topic
    pub sub: String,

    /// List of references (suttas or cross-references) for this sub-entry
    pub refs: Vec<TopicIndexRef>,
}

/// A headword in the topic index
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopicIndexHeadword {
    /// The headword text (e.g., "abandoning (pajahati, pahāna)")
    pub headword: String,

    /// Normalized ID for anchor navigation (e.g., "abandoning-pajahati-pahaana")
    pub headword_id: String,

    /// List of entries (sub-topics) under this headword
    pub entries: Vec<TopicIndexEntry>,
}

/// A letter section in the topic index
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopicIndexLetter {
    /// The letter (A-Z)
    pub letter: String,

    /// List of headwords under this letter
    pub headwords: Vec<TopicIndexHeadword>,
}

// ============================================================================
// Constants
// ============================================================================

/// Words to ignore when sorting headwords
const IGNORE_WORDS: &[&str] = &[
    "in", "of", "with", "from", "to", "for", "on", "the", "as", "a", "an", "vs.", "and"
];

/// Canonical book order for sorting sutta references
const BOOK_ORDER: &[&str] = &[
    "dn", "mn", "sn", "an", "kp", "dhp", "ud", "iti", "snp", "vv", "pv", "thag", "thig"
];

lazy_static! {
    /// Regex to extract book abbreviation from locator
    static ref RE_BOOK: Regex = Regex::new(r"^([a-zA-Z]+)").unwrap();

    /// Regex to extract numeric part for natural sorting
    static ref RE_NUMERIC: Regex = Regex::new(r"(\d+)").unwrap();
}

// ============================================================================
// Normalization Functions
// ============================================================================

/// Normalize a string by removing diacritics using Unicode NFD decomposition.
/// This is equivalent to the JavaScript normalizeDiacriticString function.
///
/// Example: "ānanda" → "ananda", "Ā" → "A"
pub fn normalize_diacritic_string(text: &str) -> String {
    use unicode_normalization::UnicodeNormalization;

    text.nfd()
        .filter(|c| !unicode_normalization::char::is_combining_mark(*c))
        .collect()
}

/// Create a valid anchor ID from a headword.
/// Replaces long vowels with doubled letters, removes punctuation, replaces spaces with hyphens.
///
/// Examples:
/// - "nibbāna" → "nibbaana"
/// - "actions (kamma)" → "actions-kamma"
/// - "Ānanda, Ven." → "Aananda-Ven"
pub fn make_normalized_id(text: &str) -> String {
    let mut s = text.trim().to_string();

    // Replace long vowels with doubled letters
    s = s.replace('ā', "aa")
        .replace('ī', "ii")
        .replace('ū', "uu")
        .replace('Ā', "Aa")
        .replace('Ī', "Ii")
        .replace('Ū', "Uu");

    // Remove "xref " prefix if present
    s = s.replace("xref ", "");

    // Apply NFD normalization to remove remaining diacritics
    s = normalize_diacritic_string(&s);

    // Replace spaces with hyphens
    s = s.replace(' ', "-");

    // Remove punctuation (including curly quotes)
    s = s.chars()
        .filter(|c| !matches!(*c, ',' | ';' | '.' | '…' | '"' | '\'' | '/' | '(' | ')' | '\u{201C}' | '\u{201D}' | '\u{2018}' | '\u{2019}'))
        .collect();

    s
}

/// Get the sort key for a headword by stripping leading ignore words and normalizing.
fn get_headword_sort_key(headword: &str) -> String {
    let mut s = headword.trim().to_lowercase();

    // Remove leading curly/fancy quotes
    s = s.trim_start_matches('"').to_string();

    // Strip leading ignore words (repeatedly)
    loop {
        let mut stripped = false;
        for word in IGNORE_WORDS {
            let prefix = format!("{} ", word);
            if s.starts_with(&prefix) {
                s = s[prefix.len()..].to_string();
                stripped = true;
                break;
            }
        }
        if !stripped {
            break;
        }
    }

    // Latinize (remove diacritics) for case-insensitive, diacritic-insensitive comparison
    latinize(&s)
}

/// Extract the book abbreviation from a locator.
/// "DN33:1.11.0" → "dn", "AN4.159" → "an"
fn extract_book(locator: &str) -> String {
    if let Some(caps) = RE_BOOK.captures(locator) {
        caps[1].to_lowercase()
    } else {
        String::new()
    }
}

/// Get the book order index (lower = earlier in canon)
fn book_order_index(book: &str) -> usize {
    BOOK_ORDER.iter()
        .position(|&b| b == book.to_lowercase())
        .unwrap_or(999) // Unknown books sort last
}

/// Extract numeric parts from a locator for natural sorting.
/// "DN33:1.11.0" → vec![33, 1, 11, 0]
fn extract_numbers(locator: &str) -> Vec<u32> {
    RE_NUMERIC.find_iter(locator)
        .filter_map(|m| m.as_str().parse().ok())
        .collect()
}

/// Compare two locators for sorting by canonical book order and natural number sorting.
fn compare_locators(a: &str, b: &str) -> std::cmp::Ordering {
    let book_a = extract_book(a);
    let book_b = extract_book(b);

    // First compare by book order
    let order_a = book_order_index(&book_a);
    let order_b = book_order_index(&book_b);

    match order_a.cmp(&order_b) {
        std::cmp::Ordering::Equal => {
            // Same book, compare by natural number sorting
            let nums_a = extract_numbers(a);
            let nums_b = extract_numbers(b);

            for (na, nb) in nums_a.iter().zip(nums_b.iter()) {
                match na.cmp(nb) {
                    std::cmp::Ordering::Equal => continue,
                    other => return other,
                }
            }

            // If all numbers equal, shorter wins (fewer segments)
            nums_a.len().cmp(&nums_b.len())
        }
        other => other,
    }
}

// ============================================================================
// Parsing Functions
// ============================================================================

/// Raw CSV row data
#[derive(Debug)]
struct CsvRow {
    headword: String,
    subheading: String,
    locator: String,
}

/// Parse a CIPS general-index.csv file (tab-delimited).
fn parse_csv(csv_path: &Path) -> Result<Vec<CsvRow>> {
    let content = fs::read_to_string(csv_path)
        .with_context(|| format!("Failed to read CSV file: {:?}", csv_path))?;

    let mut rows = Vec::new();

    for (line_num, line) in content.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }

        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() < 3 {
            eprintln!("Warning: Line {} has fewer than 3 columns: {:?}", line_num + 1, line);
            continue;
        }

        rows.push(CsvRow {
            headword: parts[0].trim().to_string(),
            subheading: parts[1].trim().to_string(),
            locator: parts[2].trim().to_string(),
        });
    }

    Ok(rows)
}

/// Check if a locator is a cross-reference
fn is_xref(locator: &str) -> bool {
    locator.contains("xref")
}

/// Extract the target headword from an xref locator.
/// "xref abandoning (pajahati, pahāna)" → "abandoning (pajahati, pahāna)"
fn extract_xref_target(locator: &str) -> String {
    locator.replace("xref ", "").trim().to_string()
}

/// Parse a locator into a sutta reference.
/// Preserves the segment ID for navigation.
/// "DN33:1.11.0" → "dn33:1.11.0"
fn parse_sutta_ref(locator: &str) -> String {
    locator.to_lowercase()
}

/// Extract just the sutta UID (without segment) for title lookup.
/// "dn33:1.11.0" → "dn33"
fn sutta_ref_to_uid(sutta_ref: &str) -> String {
    if sutta_ref.contains(':') {
        sutta_ref.split(':').next().unwrap_or(sutta_ref).to_string()
    } else {
        sutta_ref.to_string()
    }
}

/// Intermediate structure for building the index
struct IndexBuilder {
    /// Letter → Headword → Sub-entry → (locators, xrefs)
    data: HashMap<String, HashMap<String, HashMap<String, (Vec<String>, Vec<String>)>>>,
}

impl IndexBuilder {
    fn new() -> Self {
        Self {
            data: HashMap::new(),
        }
    }

    fn add_row(&mut self, row: &CsvRow) {
        // Determine letter section
        let first_char = row.headword.chars().next().unwrap_or('?');
        let letter = normalize_diacritic_string(&first_char.to_uppercase().to_string());

        // Handle blank sub-entry
        let sub = if row.subheading.trim().is_empty() && !is_xref(&row.locator) {
            "—".to_string() // Em-dash for direct headword→sutta links
        } else {
            row.subheading.clone()
        };

        // Get or create the nested structure
        let letter_map = self.data.entry(letter).or_default();
        let headword_map = letter_map.entry(row.headword.clone()).or_default();
        let (locators, xrefs) = headword_map.entry(sub).or_default();

        // Add the locator to the appropriate list
        if is_xref(&row.locator) {
            xrefs.push(row.locator.clone());
        } else {
            locators.push(row.locator.clone());
        }
    }

    /// Build the final JSON structure with sorted data.
    /// `title_lookup` is a function that looks up Pāli titles for sutta UIDs.
    fn build<F>(self, title_lookup: F) -> Vec<TopicIndexLetter>
    where
        F: Fn(&str) -> Option<String>,
    {
        let mut letters: Vec<TopicIndexLetter> = Vec::new();

        // Sort letters A-Z
        let mut letter_keys: Vec<String> = self.data.keys().cloned().collect();
        letter_keys.sort();

        for letter in letter_keys {
            let headword_map = &self.data[&letter];

            // Sort headwords
            let mut headword_keys: Vec<String> = headword_map.keys().cloned().collect();
            headword_keys.sort_by(|a, b| get_headword_sort_key(a).cmp(&get_headword_sort_key(b)));

            let mut headwords: Vec<TopicIndexHeadword> = Vec::new();

            for headword in headword_keys {
                let sub_map = &headword_map[&headword];
                let headword_id = make_normalized_id(&headword);

                // Sort sub-entries (em-dash first, then alphabetically)
                let mut sub_keys: Vec<String> = sub_map.keys().cloned().collect();
                sub_keys.sort_by(|a, b| {
                    match (a.as_str(), b.as_str()) {
                        ("—", "—") => std::cmp::Ordering::Equal,
                        ("—", _) => std::cmp::Ordering::Less,
                        (_, "—") => std::cmp::Ordering::Greater,
                        _ => latinize(a).to_lowercase().cmp(&latinize(b).to_lowercase()),
                    }
                });

                let mut entries: Vec<TopicIndexEntry> = Vec::new();

                for sub in sub_keys {
                    let (locators, xrefs) = &sub_map[&sub];
                    let mut refs: Vec<TopicIndexRef> = Vec::new();

                    // Sort and add locators (sutta references)
                    let mut sorted_locators = locators.clone();
                    sorted_locators.sort_by(|a, b| compare_locators(a, b));

                    for locator in sorted_locators {
                        let sutta_ref = parse_sutta_ref(&locator);
                        let uid = sutta_ref_to_uid(&sutta_ref);
                        let title = title_lookup(&uid);

                        refs.push(TopicIndexRef {
                            sutta_ref: Some(sutta_ref),
                            ref_target: None,
                            title,
                            ref_type: "sutta".to_string(),
                        });
                    }

                    // Add cross-references
                    for xref in xrefs {
                        let target = extract_xref_target(xref);
                        refs.push(TopicIndexRef {
                            sutta_ref: None,
                            ref_target: Some(target),
                            title: None,
                            ref_type: "xref".to_string(),
                        });
                    }

                    entries.push(TopicIndexEntry { sub, refs });
                }

                headwords.push(TopicIndexHeadword {
                    headword,
                    headword_id,
                    entries,
                });
            }

            letters.push(TopicIndexLetter { letter, headwords });
        }

        letters
    }
}

// ============================================================================
// Validation Functions
// ============================================================================

/// Validation result containing warnings and errors
#[derive(Debug, Default)]
pub struct ValidationResult {
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
}

impl ValidationResult {
    pub fn is_valid(&self) -> bool {
        self.errors.is_empty()
    }
}

/// Validate the parsed index data
pub fn validate_index(index: &[TopicIndexLetter]) -> ValidationResult {
    let mut result = ValidationResult::default();

    // Collect all headword names for xref validation
    let mut all_headwords: HashMap<String, bool> = HashMap::new();
    for letter in index {
        for headword in &letter.headwords {
            all_headwords.insert(headword.headword.to_lowercase(), true);
        }
    }

    // Validate each entry
    for letter in index {
        for headword in &letter.headwords {
            for entry in &headword.entries {
                for ref_item in &entry.refs {
                    // Validate xref targets exist
                    if ref_item.ref_type == "xref" {
                        if let Some(target) = &ref_item.ref_target {
                            if !all_headwords.contains_key(&target.to_lowercase()) {
                                result.warnings.push(format!(
                                    "Cross-reference target not found: '{}' -> '{}'",
                                    headword.headword, target
                                ));
                            }
                        }
                    }

                    // Validate sutta reference format
                    if ref_item.ref_type == "sutta" {
                        if let Some(sutta_ref) = &ref_item.sutta_ref {
                            let book = extract_book(sutta_ref);
                            if book.is_empty() {
                                result.warnings.push(format!(
                                    "Invalid sutta reference format: '{}' in '{}'",
                                    sutta_ref, headword.headword
                                ));
                            }
                        }
                    }
                }
            }
        }
    }

    result
}

// ============================================================================
// Main Public API
// ============================================================================

/// Parse a CIPS general-index.csv file and return the topic index data structure.
///
/// # Arguments
/// * `csv_path` - Path to the CSV file
/// * `title_lookup` - Function to look up Pāli titles for sutta UIDs
///
/// # Returns
/// The parsed topic index as a vector of letter sections
pub fn parse_cips_index<F>(csv_path: &Path, title_lookup: F) -> Result<Vec<TopicIndexLetter>>
where
    F: Fn(&str) -> Option<String>,
{
    let rows = parse_csv(csv_path)?;

    let mut builder = IndexBuilder::new();
    for row in &rows {
        builder.add_row(row);
    }

    Ok(builder.build(title_lookup))
}

/// Parse a CIPS CSV file and write the result to a JSON file.
///
/// # Arguments
/// * `csv_path` - Path to the input CSV file
/// * `json_path` - Path to the output JSON file
/// * `title_lookup` - Function to look up Pāli titles for sutta UIDs
/// * `minify` - If true, output minified JSON (no pretty-printing)
///
/// # Returns
/// The number of headwords processed
pub fn parse_cips_to_json<F>(
    csv_path: &Path,
    json_path: &Path,
    title_lookup: F,
    minify: bool,
) -> Result<usize>
where
    F: Fn(&str) -> Option<String>,
{
    let index = parse_cips_index(csv_path, title_lookup)?;

    // Count total headwords
    let headword_count: usize = index.iter().map(|l| l.headwords.len()).sum();

    // Validate
    let validation = validate_index(&index);
    for warning in &validation.warnings {
        eprintln!("Warning: {}", warning);
    }
    for error in &validation.errors {
        eprintln!("Error: {}", error);
    }

    // Write JSON
    let json_str = if minify {
        serde_json::to_string(&index)?
    } else {
        serde_json::to_string_pretty(&index)?
    };

    fs::write(json_path, json_str)
        .with_context(|| format!("Failed to write JSON file: {:?}", json_path))?;

    Ok(headword_count)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_diacritic_string() {
        assert_eq!(normalize_diacritic_string("ānanda"), "ananda");
        assert_eq!(normalize_diacritic_string("Ānanda"), "Ananda");
        assert_eq!(normalize_diacritic_string("nibbāna"), "nibbana");
        assert_eq!(normalize_diacritic_string("ñ"), "n");
    }

    #[test]
    fn test_make_normalized_id() {
        assert_eq!(make_normalized_id("nibbāna"), "nibbaana");
        assert_eq!(make_normalized_id("actions (kamma)"), "actions-kamma");
        assert_eq!(make_normalized_id("Ānanda, Ven."), "Aananda-Ven");
        assert_eq!(make_normalized_id("abandoning (pajahati, pahāna)"), "abandoning-pajahati-pahaana");
    }

    #[test]
    fn test_get_headword_sort_key() {
        assert_eq!(get_headword_sort_key("the Buddha"), "buddha");
        assert_eq!(get_headword_sort_key("of the aggregates"), "aggregates");
        assert_eq!(get_headword_sort_key("Ānanda"), "ananda");
    }

    #[test]
    fn test_extract_book() {
        assert_eq!(extract_book("DN33:1.11.0"), "dn");
        assert_eq!(extract_book("AN4.159"), "an");
        assert_eq!(extract_book("MN5"), "mn");
    }

    #[test]
    fn test_book_order_index() {
        assert_eq!(book_order_index("dn"), 0);
        assert_eq!(book_order_index("mn"), 1);
        assert_eq!(book_order_index("an"), 3);
        assert!(book_order_index("unknown") > 10);
    }

    #[test]
    fn test_compare_locators() {
        assert_eq!(compare_locators("DN1", "DN2"), std::cmp::Ordering::Less);
        assert_eq!(compare_locators("DN10", "DN2"), std::cmp::Ordering::Greater);
        assert_eq!(compare_locators("DN1", "MN1"), std::cmp::Ordering::Less);
        assert_eq!(compare_locators("AN4.10", "AN4.2"), std::cmp::Ordering::Greater);
    }

    #[test]
    fn test_is_xref() {
        assert!(is_xref("xref abandoning"));
        assert!(!is_xref("DN33:1.11.0"));
    }

    #[test]
    fn test_extract_xref_target() {
        assert_eq!(extract_xref_target("xref disrobing"), "disrobing");
        assert_eq!(extract_xref_target("xref abandoning (pajahati, pahāna)"), "abandoning (pajahati, pahāna)");
    }

    #[test]
    fn test_sutta_ref_to_uid() {
        assert_eq!(sutta_ref_to_uid("dn33:1.11.0"), "dn33");
        assert_eq!(sutta_ref_to_uid("mn5"), "mn5");
        assert_eq!(sutta_ref_to_uid("an4.10"), "an4.10");
    }
}
