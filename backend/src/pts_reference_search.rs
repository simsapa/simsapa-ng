use serde::{Serialize, Deserialize};
use crate::app_settings::SUTTA_REFERENCE_CONVERTER_JSON;
use crate::helpers::latinize;
use crate::logger::error;

/// Represents a parsed PTS reference (e.g., "D ii 20" → nikaya: "d", volume: "ii", page: 20)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PTSReference {
    pub nikaya: String,
    pub volume: String,
    pub page: u32,
}

/// Represents a single search result from the reference data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReferenceSearchResult {
    pub identifier: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub pts_reference: String,
    #[serde(default)]
    pub dpr_reference: String,
    pub url: String,
    // Parsed PTS reference fields for range matching
    // These can be null in the JSON
    pub pts_nikaya: Option<String>,
    pub pts_vol: Option<String>,
    pub pts_start_page: Option<u32>,
    pub pts_end_page: Option<u32>,
}

/// Parse a PTS reference string like "D ii 20" into components
/// Returns None if the string cannot be parsed
pub fn parse_pts_reference(pts_ref: &str) -> Option<PTSReference> {
    if pts_ref.trim().is_empty() {
        return None;
    }

    // Normalize: trim, lowercase, normalize whitespace
    let normalized = pts_ref.trim().to_lowercase();
    let normalized = normalized.split_whitespace().collect::<Vec<_>>().join(" ");

    // Match pattern: letter(s) + roman numeral + number
    // Example: "d ii 20" or "m iii 10"
    let parts: Vec<&str> = normalized.split_whitespace().collect();

    if parts.len() < 3 {
        return None;
    }

    // First part should be the nikaya (one or more letters)
    let nikaya = parts[0];
    if !nikaya.chars().all(|c| c.is_alphabetic()) {
        return None;
    }

    // Second part should be the volume (roman numerals)
    let volume = parts[1];
    if !volume.chars().all(|c| matches!(c, 'i' | 'v' | 'x')) {
        return None;
    }

    // Third part should be the page number
    let page = parts[2].parse::<u32>().ok()?;

    Some(PTSReference {
        nikaya: nikaya.to_string(),
        volume: volume.to_string(),
        page,
    })
}

/// Search by text in a specific field with normalization
/// Uses the latinize() function to remove diacritics for matching
pub fn search_by_text(query: &str, field: &str) -> Vec<ReferenceSearchResult> {
    if query.trim().is_empty() {
        return load_all_references();
    }

    // Normalize query: lowercase and remove diacritics
    let normalized_query = latinize(&query.trim().to_lowercase());

    load_all_references()
        .into_iter()
        .filter(|entry| {
            let field_value = match field {
                "identifier" => &entry.identifier,
                "name" => &entry.name,
                "pts_reference" => &entry.pts_reference,
                "dpr_reference" => &entry.dpr_reference,
                _ => "",
            };

            // Skip empty field values
            if field_value.is_empty() {
                return false;
            }

            let normalized_field = latinize(&field_value.to_lowercase());
            normalized_field.contains(&normalized_query)
        })
        .collect()
}

/// Search by PTS reference with range matching support
/// Example: searching "D ii 20" will find the sutta that starts at "D ii 1"
/// if page 20 falls within the pts_start_page to pts_end_page range
pub fn search_by_pts_reference(query: &str) -> Vec<ReferenceSearchResult> {
    if query.trim().is_empty() {
        return load_all_references();
    }

    let parsed_query = match parse_pts_reference(query) {
        Some(p) => p,
        None => {
            // Fallback to text search if can't parse
            return search_by_text(query, "pts_reference");
        }
    };

    let all_refs = load_all_references();

    all_refs
        .into_iter()
        .filter(|entry| {
            // Skip entries without parsed PTS data
            let nikaya = match &entry.pts_nikaya {
                Some(n) => n,
                None => return false,
            };
            let vol = match &entry.pts_vol {
                Some(v) => v,
                None => return false,
            };

            // Check if nikaya and volume match (case-insensitive)
            let nikaya_match = nikaya.to_lowercase() == parsed_query.nikaya;
            let volume_match = vol.to_lowercase() == parsed_query.volume;

            if !nikaya_match || !volume_match {
                return false;
            }

            // Check if query page falls within the range
            match (entry.pts_start_page, entry.pts_end_page) {
                (Some(start), Some(end)) => {
                    // Page falls within start and end range (inclusive)
                    parsed_query.page >= start && parsed_query.page <= end
                }
                (Some(start), None) => {
                    // Only start page available, match if equal
                    parsed_query.page == start
                }
                _ => {
                    // No parsed page data
                    false
                }
            }
        })
        .collect()
}

/// Universal search function that routes to appropriate search method
/// For 'pts_reference' field, uses range-based matching
/// For other fields, uses text-based matching
pub fn search(query: &str, field: &str) -> Vec<ReferenceSearchResult> {
    if field == "pts_reference" {
        search_by_pts_reference(query)
    } else {
        search_by_text(query, field)
    }
}

/// Load all reference entries from the JSON data
fn load_all_references() -> Vec<ReferenceSearchResult> {
    match serde_json::from_str::<Vec<ReferenceSearchResult>>(SUTTA_REFERENCE_CONVERTER_JSON) {
        Ok(data) => data,
        Err(e) => {
            error(&format!("Failed to parse sutta-reference-converter.json: {}", e));
            vec![]
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_pts_reference_valid() {
        let result = parse_pts_reference("D ii 20");
        assert!(result.is_some());
        let parsed = result.unwrap();
        assert_eq!(parsed.nikaya, "d");
        assert_eq!(parsed.volume, "ii");
        assert_eq!(parsed.page, 20);
    }

    #[test]
    fn test_parse_pts_reference_with_extra_spaces() {
        let result = parse_pts_reference("  M   iii   10  ");
        assert!(result.is_some());
        let parsed = result.unwrap();
        assert_eq!(parsed.nikaya, "m");
        assert_eq!(parsed.volume, "iii");
        assert_eq!(parsed.page, 10);
    }

    #[test]
    fn test_parse_pts_reference_invalid() {
        assert!(parse_pts_reference("").is_none());
        assert!(parse_pts_reference("invalid").is_none());
        assert!(parse_pts_reference("D 20").is_none()); // Missing volume
    }

    #[test]
    fn test_latinize_normalization() {
        let query = "brahmajāla";
        let normalized = latinize(&query.to_lowercase());
        assert_eq!(normalized, "brahmajala");
    }

    #[test]
    fn test_json_loading() {
        let data = load_all_references();
        assert!(data.len() > 0, "JSON data should be loaded and contain entries");

        // Print first entry for debugging
        if let Some(first) = data.first() {
            eprintln!("First entry: identifier={}, pts_ref={}", first.identifier, first.pts_reference);
        }
    }
}
