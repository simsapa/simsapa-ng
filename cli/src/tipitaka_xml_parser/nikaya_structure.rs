//! Nikaya structure definitions and normalization
//!
//! This module provides hard-coded nikaya hierarchy configurations
//! and name normalization logic.

use crate::tipitaka_xml_parser::types::GroupType;
use serde::{Deserialize, Serialize};

/// Represents the hierarchical structure of a nikaya
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NikayaStructure {
    /// Name of the nikaya (normalized)
    pub nikaya: String,
    /// Ordered list of group types in this nikaya's hierarchy
    pub levels: Vec<GroupType>,
    /// Source XML filename (optional, for tracking)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub xml_filename: Option<String>,
}

impl NikayaStructure {
    /// Normalize a nikaya name from various Pali forms to standard form
    ///
    /// Handles both "yo" and "ye" endings and various diacritic forms:
    /// - "Dīghanikāyo" or "Dīghanikāye" → Some("digha")
    /// - "Majjhimanikāyo" or "Majjhimanikāye" → Some("majjhima")
    /// - "Saṃyuttanikāyo" or "Saṃyuttanikāye" → Some("samyutta")
    /// - "Aṅguttaranikāyo" or "Aṅguttaranikāye" → Some("anguttara")
    /// - "Khuddakanikāyo" or "Khuddakanikāye" → Some("khuddaka")
    ///
    /// # Arguments
    /// * `name` - The raw nikaya name from XML
    ///
    /// # Returns
    /// Normalized name (e.g., "digha", "majjhima") or None if unknown
    pub fn normalize_name(name: &str) -> Option<String> {
        let normalized = name.trim().to_lowercase();
        
        // Handle various forms of nikaya names
        if normalized.contains("dīgha") || normalized.contains("digha") {
            Some("digha".to_string())
        } else if normalized.contains("majjhima") {
            Some("majjhima".to_string())
        } else if normalized.contains("saṃyutta") || normalized.contains("samyutta") {
            Some("samyutta".to_string())
        } else if normalized.contains("aṅguttara") || normalized.contains("anguttara") {
            Some("anguttara".to_string())
        } else if normalized.contains("khuddaka") {
            Some("khuddaka".to_string())
        } else {
            None
        }
    }

    /// Get the nikaya structure configuration for a given normalized nikaya name
    ///
    /// Returns hard-coded hierarchical structures for each nikaya:
    /// - DN (Dīgha): [Nikaya, Book, Sutta]
    /// - MN (Majjhima): [Nikaya, Book, Vagga, Sutta]
    /// - SN (Saṃyutta): [Nikaya, Book, Samyutta, Vagga, Sutta]
    /// - AN (Aṅguttara): [Nikaya, Book, Sutta] (simplified, may need adjustment)
    /// - KN (Khuddaka): [Nikaya, Book, Sutta] (simplified, varies by text)
    ///
    /// # Arguments
    /// * `name` - Normalized nikaya name (e.g., "digha")
    ///
    /// # Returns
    /// NikayaStructure configuration or None if unknown
    pub fn from_nikaya_name(name: &str) -> Option<NikayaStructure> {
        match name {
            "digha" => Some(NikayaStructure {
                nikaya: "digha".to_string(),
                levels: vec![
                    GroupType::Nikaya,
                    GroupType::Book,
                    GroupType::Sutta,
                ],
                xml_filename: None,
            }),
            "majjhima" => Some(NikayaStructure {
                nikaya: "majjhima".to_string(),
                levels: vec![
                    GroupType::Nikaya,
                    GroupType::Book,
                    GroupType::Vagga,
                    GroupType::Sutta,
                ],
                xml_filename: None,
            }),
            "samyutta" => Some(NikayaStructure {
                nikaya: "samyutta".to_string(),
                levels: vec![
                    GroupType::Nikaya,
                    GroupType::Book,
                    GroupType::Samyutta,
                    GroupType::Vagga,
                    GroupType::Sutta,
                ],
                xml_filename: None,
            }),
            "anguttara" => Some(NikayaStructure {
                nikaya: "anguttara".to_string(),
                levels: vec![
                    GroupType::Nikaya,
                    GroupType::Book,
                    GroupType::Sutta,
                ],
                xml_filename: None,
            }),
            "khuddaka" => Some(NikayaStructure {
                nikaya: "khuddaka".to_string(),
                levels: vec![
                    GroupType::Nikaya,
                    GroupType::Book,
                    GroupType::Sutta,
                ],
                xml_filename: None,
            }),
            _ => None,
        }
    }
    
    /// Set the XML filename for this structure
    pub fn with_xml_filename(mut self, filename: String) -> Self {
        self.xml_filename = Some(filename);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_name_digha_yo() {
        assert_eq!(
            NikayaStructure::normalize_name("Dīghanikāyo"),
            Some("digha".to_string())
        );
    }

    #[test]
    fn test_normalize_name_digha_ye() {
        assert_eq!(
            NikayaStructure::normalize_name("Dīghanikāye"),
            Some("digha".to_string())
        );
    }

    #[test]
    fn test_normalize_name_majjhima_yo() {
        assert_eq!(
            NikayaStructure::normalize_name("Majjhimanikāyo"),
            Some("majjhima".to_string())
        );
    }

    #[test]
    fn test_normalize_name_majjhima_ye() {
        assert_eq!(
            NikayaStructure::normalize_name("Majjhimanikāye"),
            Some("majjhima".to_string())
        );
    }

    #[test]
    fn test_normalize_name_samyutta_yo() {
        assert_eq!(
            NikayaStructure::normalize_name("Saṃyuttanikāyo"),
            Some("samyutta".to_string())
        );
    }

    #[test]
    fn test_normalize_name_samyutta_ye() {
        assert_eq!(
            NikayaStructure::normalize_name("Saṃyuttanikāye"),
            Some("samyutta".to_string())
        );
    }

    #[test]
    fn test_normalize_name_anguttara() {
        assert_eq!(
            NikayaStructure::normalize_name("Aṅguttaranikāyo"),
            Some("anguttara".to_string())
        );
    }

    #[test]
    fn test_normalize_name_khuddaka() {
        assert_eq!(
            NikayaStructure::normalize_name("Khuddakanikāyo"),
            Some("khuddaka".to_string())
        );
    }

    #[test]
    fn test_normalize_name_unknown() {
        assert_eq!(NikayaStructure::normalize_name("Unknown"), None);
    }

    #[test]
    fn test_from_nikaya_name_digha() {
        let structure = NikayaStructure::from_nikaya_name("digha").unwrap();
        assert_eq!(structure.nikaya, "digha");
        assert_eq!(structure.levels.len(), 3);
        assert!(matches!(structure.levels[0], GroupType::Nikaya));
        assert!(matches!(structure.levels[1], GroupType::Book));
        assert!(matches!(structure.levels[2], GroupType::Sutta));
    }

    #[test]
    fn test_from_nikaya_name_majjhima() {
        let structure = NikayaStructure::from_nikaya_name("majjhima").unwrap();
        assert_eq!(structure.nikaya, "majjhima");
        assert_eq!(structure.levels.len(), 4);
        assert!(matches!(structure.levels[0], GroupType::Nikaya));
        assert!(matches!(structure.levels[1], GroupType::Book));
        assert!(matches!(structure.levels[2], GroupType::Vagga));
        assert!(matches!(structure.levels[3], GroupType::Sutta));
    }

    #[test]
    fn test_from_nikaya_name_samyutta() {
        let structure = NikayaStructure::from_nikaya_name("samyutta").unwrap();
        assert_eq!(structure.nikaya, "samyutta");
        assert_eq!(structure.levels.len(), 5);
        assert!(matches!(structure.levels[0], GroupType::Nikaya));
        assert!(matches!(structure.levels[1], GroupType::Book));
        assert!(matches!(structure.levels[2], GroupType::Samyutta));
        assert!(matches!(structure.levels[3], GroupType::Vagga));
        assert!(matches!(structure.levels[4], GroupType::Sutta));
    }

    #[test]
    fn test_from_nikaya_name_unknown() {
        assert!(NikayaStructure::from_nikaya_name("unknown").is_none());
    }
}
