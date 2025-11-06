//! Sutta record assembly from fragments
//!
//! This module provides functionality to assemble database records
//! from parsed XML fragments.

use anyhow::{Result, Context};
use crate::tipitaka_xml_parser::types::{XmlFragment, FragmentType, GroupType};
use crate::tipitaka_xml_parser::nikaya_structure::NikayaStructure;
use simsapa_backend::helpers::{consistent_niggahita, compact_rich_text};
use simsapa_backend::logger;
use quick_xml::{Reader, events::Event};
use html_escape;
use serde::Deserialize;

use crate::tipitaka_xml_parser::fragment_parser::CST_VS_SC_TSV;

/// Sutta record matching appdata schema
#[derive(Debug, Clone)]
pub struct SuttaRecord {
    pub uid: String,
    pub sutta_ref: String,
    pub nikaya: String,
    pub language: String,
    pub group_path: Option<String>,
    pub group_index: Option<i32>,
    pub order_index: Option<i32>,
    pub title: Option<String>,
    pub title_pali: Option<String>,
    pub content_plain: Option<String>,
    pub content_html: Option<String>,
    pub source_uid: Option<String>,
}

/// TSV record for CST code lookup
#[derive(Debug, Clone, Deserialize)]
pub struct TsvRecord {
    pub cst_file: String,
    pub cst_code: String,
    pub cst_vagga: String,
    pub cst_sutta: String,
    pub cst_paranum: String,
    #[serde(rename = "code")]
    pub sc_code: String,
    #[serde(rename = "sutta")]
    pub sc_sutta: String,
}

/// Load TSV mapping from embedded string
pub fn load_tsv_mapping() -> Result<Vec<TsvRecord>> {
    let mut reader = csv::ReaderBuilder::new()
        .delimiter(b'\t')
        .from_reader(CST_VS_SC_TSV.as_bytes());
    
    let records: Result<Vec<TsvRecord>, csv::Error> = reader
        .deserialize()
        .collect();
    
    records.context("Failed to deserialize TSV records")
}

/// Find code for a given filename, sutta title, and vagga
fn find_code_for_sutta(
    tsv_records: &[TsvRecord],
    cst_file: &str,
    sutta_title: &str,
    vagga_title: Option<&str>,
    is_commentary: bool,
    used_codes: &std::collections::HashSet<String>,
) -> Option<String> {
    // Normalize the xml filename (remove path prefix if present)
    let normalized_filename = cst_file
        .trim_start_matches("romn/")
        .trim_start_matches("mula/");
    
    // For commentaries (.att.xml or .tik.xml), convert the filename to the base .mul.xml format
    // e.g., "s0201a.att.xml" -> "s0201m.mul.xml"
    // e.g., "s0201t.tik.xml" -> "s0201m.mul.xml"
    let base_filename = if is_commentary {
        if normalized_filename.ends_with(".att.xml") {
            // Replace "a.att.xml" with "m.mul.xml"
            normalized_filename.replace("a.att.xml", "m.mul.xml")
        } else if normalized_filename.ends_with(".tik.xml") {
            // Replace "t.tik.xml" with "m.mul.xml"
            normalized_filename.replace("t.tik.xml", "m.mul.xml")
        } else {
            normalized_filename.to_string()
        }
    } else {
        normalized_filename.to_string()
    };
    
    // For commentary titles, strip the "-vaṇṇanā" suffix and try to match with the base sutta
    let search_title = if is_commentary && sutta_title.ends_with("vaṇṇanā") {
        // Strip "vaṇṇanā" - the base should already end with "sutta"
        let base = sutta_title.trim_end_matches("vaṇṇanā");
        // The base already ends with "sutta", so we just need to try both "sutta" and "suttaṃ"
        let mut candidates = if base.ends_with("sutta") {
            vec![
                base.to_string(),  // Keep "sutta" as-is
                format!("{}ṃ", base),  // Add anusvara to make "suttaṃ"
            ]
        } else {
            // Fallback: assume we need to add "sutta"
            vec![
                format!("{}sutta", base),
                format!("{}suttaṃ", base),
            ]
        };
        
        // Edge case: Handle "Vanapatthapariyāya" → "Vanapatthasutta" mismatch
        // 
        // The commentary (s0201a.att.xml, s0201t.tik.xml) uses:
        //   <p rend="subhead">7. Vanapatthapariyāyasuttavaṇṇanā</p>
        // 
        // But the base text (s0201m.mul.xml) uses:
        //   <p rend="subhead">7. Vanapatthasuttaṃ</p>
        //
        // The commentary adds "pariyāya" (method/approach) to the title, which is not
        // present in the base text or TSV mapping. This is MN 17.
        if base.contains("Vanapatthapariyāyasutta") {
            // Extract the number prefix if present (e.g., "7. ")
            if let Some((num, _)) = base.split_once('.') {
                candidates.push(format!("{}. Vanapatthasutta", num));
                candidates.push(format!("{}. Vanapatthasuttaṃ", num));
            } else {
                candidates.push("Vanapatthasutta".to_string());
                candidates.push("Vanapatthasuttaṃ".to_string());
            }
        }
        
        // Also try with "Mahā" prefix for certain suttas (e.g., Satipaṭṭhānasutta -> Mahāsatipaṭṭhānasutta)
        // Extract just the sutta name without numbering
        if let Some((num, sutta_part)) = base.split_once('.') {
            let sutta_part = sutta_part.trim();
            if sutta_part.ends_with("sutta") {
                // Add "Mahā" prefix - lowercase the first letter of the original sutta name
                let sutta_with_maha = if let Some(first_char) = sutta_part.chars().next() {
                    let rest = &sutta_part[first_char.len_utf8()..];
                    format!("Mahā{}{}", first_char.to_lowercase(), rest)
                } else {
                    format!("Mahā{}", sutta_part)
                };
                candidates.push(format!("{}. {}", num, sutta_with_maha));
                candidates.push(format!("{}. {}ṃ", num, sutta_with_maha));
            }
        }
        
        candidates
    } else {
        vec![sutta_title.to_string()]
    };
    
    // Normalize all search titles with consistent niggahita
    let normalized_search_titles: Vec<String> = search_title.iter()
        .map(|t| consistent_niggahita(Some(t.clone())))
        .collect();
    
    // Normalize vagga title if provided
    let normalized_vagga = vagga_title.map(|v| consistent_niggahita(Some(v.to_string())));
    
    let mut fallback_match: Option<String> = None;
    
    for record in tsv_records {
        let record_filename = record.cst_file
            .trim_start_matches("romn/")
            .trim_start_matches("mula/");
        
        let record_title = consistent_niggahita(Some(record.cst_sutta.clone()));
        let record_vagga = consistent_niggahita(Some(record.cst_vagga.clone()));
        
        // Check if filename matches (using base filename for commentaries)
        if record_filename == base_filename {
            
            // Check if any of the search titles match
            for search_title in &normalized_search_titles {
                if &record_title == search_title {
                    // If vagga is provided, prefer exact vagga match
                    if let Some(ref expected_vagga) = normalized_vagga {
                        let vaggas_match = &record_vagga == expected_vagga;
                        if vaggas_match {
                            // Perfect match: title + vagga - only return if code unused
                            if !used_codes.contains(&record.sc_code) {
                                return Some(record.sc_code.clone());
                            }
                        } else {
                            // Title matches but vagga doesn't
                            // Save as fallback if code not yet used
                            if fallback_match.is_none() && !used_codes.contains(&record.sc_code) {
                                fallback_match = Some(record.sc_code.clone());
                            }
                        }
                    } else {
                        // No vagga filter - return first unused match
                        if !used_codes.contains(&record.sc_code) {
                            return Some(record.sc_code.clone());
                        }
                        // If code is already used, save as fallback to continue searching
                        // (This shouldn't happen often but handles edge cases)
                        if fallback_match.is_none() {
                            fallback_match = Some(record.sc_code.clone());
                        }
                    }
                }
            }
        }
    }
    
    // If no exact match found, return fallback (for commentaries with misaligned vaggas)
    fallback_match
}

/// Convert UID code to sutta reference (e.g., "dn1" -> "DN 1")
fn uid_to_ref(uid_code: &str) -> String {
    // Extract letters and numbers
    let mut letters = String::new();
    let mut numbers = String::new();
    
    for ch in uid_code.chars() {
        if ch.is_alphabetic() {
            letters.push(ch);
        } else if ch.is_numeric() {
            numbers.push(ch);
        }
    }
    
    // Convert to uppercase and add space
    let collection = letters.to_uppercase();
    
    if numbers.is_empty() {
        collection
    } else {
        format!("{} {}", collection, numbers)
    }
}

/// Transform raw XML fragment content to HTML
fn xml_to_html(xml_content: &str) -> Result<String> {
    let mut html = String::new();
    let mut reader = Reader::from_str(xml_content);
    reader.trim_text(false); // Preserve whitespace
    reader.check_end_names(false); // Don't validate end tag names strictly
    
    let mut buf = Vec::new();
    let mut pending_paranum: Option<String> = None;
    let mut unknown_tags = std::collections::HashSet::new();
    let mut tag_stack: Vec<String> = Vec::new(); // Track opened tags to close them properly
    
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                let name_bytes = e.name();
                let name = std::str::from_utf8(name_bytes.as_ref())
                    .unwrap_or("");
                
                match name {
                    "p" => {
                        // Get rend attribute for class
                        let mut rend = String::from("bodytext");
                        let mut paranum: Option<String> = None;
                        
                        for attr in e.attributes() {
                            if let Ok(attr) = attr {
                                let key = std::str::from_utf8(attr.key.as_ref()).unwrap_or("");
                                let value = attr.unescape_value().unwrap_or_default();
                                
                                match key {
                                    "rend" => rend = value.to_string(),
                                    "n" => paranum = Some(value.to_string()),
                                    _ => {}
                                }
                            }
                        }
                        
                        // Map rend types to CSS classes
                        let class = match rend.as_str() {
                            "centre" => "centered",
                            "center" => "centered",
                            _ => &rend,
                        };
                        
                        html.push_str(&format!("<p class=\"{}\">", class));
                        pending_paranum = paranum;
                    }
                    "hi" => {
                        // Highlighted text - get rend attribute
                        let mut rend = String::from("bold");
                        
                        for attr in e.attributes() {
                            if let Ok(attr) = attr {
                                let key = std::str::from_utf8(attr.key.as_ref()).unwrap_or("");
                                if key == "rend" {
                                    rend = attr.unescape_value().unwrap_or_default().to_string();
                                }
                            }
                        }
                        
                        // Special handling for paranum
                        if rend == "paranum" {
                            html.push_str("<span class=\"paranum\">");
                        } else {
                            html.push_str(&format!("<span class=\"{}\">", rend));
                        }
                    }
                    "note" => {
                        html.push_str("<span class=\"note\">[");
                    }
                    "pb" => {
                        // Page break
                        let mut ed = String::new();
                        let mut n = String::new();
                        
                        for attr in e.attributes() {
                            if let Ok(attr) = attr {
                                let key = std::str::from_utf8(attr.key.as_ref()).unwrap_or("");
                                let value = attr.unescape_value().unwrap_or_default();
                                
                                match key {
                                    "ed" => ed = value.to_string(),
                                    "n" => n = value.to_string(),
                                    _ => {}
                                }
                            }
                        }
                        
                        html.push_str(&format!("<span class=\"pagebreak\" data-ed=\"{}\" data-n=\"{}\"></span>", ed, n));
                    }
                    "head" => {
                        // Head tags - convert to appropriate HTML heading based on rend attribute
                        let mut rend = String::new();
                        for attr in e.attributes() {
                            if let Ok(attr) = attr {
                                let key = std::str::from_utf8(attr.key.as_ref()).unwrap_or("");
                                if key == "rend" {
                                    rend = attr.unescape_value().unwrap_or_default().to_string();
                                }
                            }
                        }
                        
                        let html_tag = match rend.as_str() {
                            "chapter" => {
                                html.push_str("<h2 class=\"chapter\">");
                                "h2"
                            },
                            "book" => {
                                html.push_str("<h2 class=\"book\">");
                                "h2"
                            },
                            "subhead" => {
                                html.push_str("<h3 class=\"subhead\">");
                                "h3"
                            },
                            "subsubhead" => {
                                html.push_str("<h4 class=\"subsubhead\">");
                                "h4"
                            },
                            _ => {
                                html.push_str(&format!("<h3 class=\"{}\">", rend));
                                "h3"
                            }
                        };
                        tag_stack.push(html_tag.to_string());
                    }
                    "div" => {
                        // Div tags - convert to HTML div with appropriate class
                        let mut div_type = String::new();
                        let mut id = String::new();
                        
                        for attr in e.attributes() {
                            if let Ok(attr) = attr {
                                let key = std::str::from_utf8(attr.key.as_ref()).unwrap_or("");
                                let value = attr.unescape_value().unwrap_or_default();
                                
                                match key {
                                    "type" => div_type = value.to_string(),
                                    "id" => id = value.to_string(),
                                    _ => {}
                                }
                            }
                        }
                        
                        if !div_type.is_empty() {
                            if !id.is_empty() {
                                html.push_str(&format!("<div class=\"{}\" id=\"{}\">", div_type, id));
                            } else {
                                html.push_str(&format!("<div class=\"{}\">", div_type));
                            }
                        } else {
                            html.push_str("<div>");
                        }
                        tag_stack.push("div".to_string());
                    }
                    "trailer" => {
                        // Trailer - typically end markers like "Suttavaṇṇanā niṭṭhitā"
                        let mut rend = String::from("trailer");
                        for attr in e.attributes() {
                            if let Ok(attr) = attr {
                                let key = std::str::from_utf8(attr.key.as_ref()).unwrap_or("");
                                if key == "rend" {
                                    rend = attr.unescape_value().unwrap_or_default().to_string();
                                }
                            }
                        }
                        html.push_str(&format!("<p class=\"{}\">", rend));
                        tag_stack.push("p".to_string());
                    }
                    _ => {
                        // Log unknown tags for future handling
                        if unknown_tags.insert(name.to_string()) {
                            logger::warn(&format!("Unknown XML tag in content: <{}>", name));
                        }
                    }
                }
            }
            Ok(Event::End(ref e)) => {
                let name_bytes = e.name();
                let name = std::str::from_utf8(name_bytes.as_ref())
                    .unwrap_or("");
                
                match name {
                    "p" => {
                        html.push_str("</p>\n");
                        pending_paranum = None;
                    }
                    "hi" => {
                        html.push_str("</span>");
                    }
                    "note" => {
                        html.push_str("]</span>");
                    }
                    "head" | "div" | "trailer" => {
                        // Close the tag using the tag stack
                        if let Some(tag) = tag_stack.pop() {
                            html.push_str(&format!("</{}>\n", tag));
                        } else {
                            // Fallback if stack is empty (shouldn't happen with well-formed XML)
                            match name {
                                "head" => html.push_str("</h3>\n"),
                                "div" => html.push_str("</div>\n"),
                                "trailer" => html.push_str("</p>\n"),
                                _ => {}
                            }
                        }
                    }
                    _ => {
                        // Unknown closing tags - skip silently (already logged on open)
                    }
                }
            }
            Ok(Event::Text(e)) => {
                let text = e.unescape().unwrap_or_default();
                
                // Add paranum if pending and this is the first text in paragraph
                if let Some(num) = pending_paranum.take() {
                    html.push_str(&format!("<span class=\"paranum\">{}</span> ", num));
                }
                
                // HTML escape the text content
                html.push_str(&html_escape::encode_text(&text));
            }
            Ok(Event::Empty(ref e)) => {
                let name_bytes = e.name();
                let name = std::str::from_utf8(name_bytes.as_ref())
                    .unwrap_or("");
                
                if name == "pb" {
                    // Handle self-closing page break
                    let mut ed = String::new();
                    let mut n = String::new();
                    
                    for attr in e.attributes() {
                        if let Ok(attr) = attr {
                            let key = std::str::from_utf8(attr.key.as_ref()).unwrap_or("");
                            let value = attr.unescape_value().unwrap_or_default();
                            
                            match key {
                                "ed" => ed = value.to_string(),
                                "n" => n = value.to_string(),
                                _ => {}
                            }
                        }
                    }
                    
                    html.push_str(&format!("<span class=\"pagebreak\" data-ed=\"{}\" data-n=\"{}\"></span>", ed, n));
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                // Log error but continue
                logger::error(&format!("XML parsing error at position {}: {}", reader.buffer_position(), e));
            }
            _ => {}
        }
        
        buf.clear();
    }
    
    Ok(html)
}

/// Build sutta database records from fragments
///
/// Uses derived CST fields (cst_code, cst_sutta, etc.) from fragments when available.
/// Falls back to TSV lookup for legacy compatibility if derived fields are not present.
///
/// # Arguments
/// * `fragments` - Vector of parsed fragments with derived CST metadata
/// * `nikaya_structure` - The structure configuration for this nikaya
/// * `tsv_path` - Path to cst-vs-sc.tsv mapping file (used for legacy fallback)
///
/// # Returns
/// Vector of sutta records or error if assembly fails
pub fn build_suttas(
    fragments: Vec<XmlFragment>,
    nikaya_structure: &NikayaStructure,
) -> Result<Vec<SuttaRecord>> {
    // Load TSV mapping from embedded data
    let tsv_records = load_tsv_mapping()
        .context("Failed to load TSV mapping")?;
    
    let mut suttas = Vec::new();
    let mut used_codes = std::collections::HashSet::new();
    
    // Group sutta fragments
    let sutta_fragments: Vec<&XmlFragment> = fragments.iter()
        .filter(|f| matches!(f.frag_type, FragmentType::Sutta))
        .collect();
    
    for (idx, fragment) in sutta_fragments.iter().enumerate() {
        // Get sutta title - prefer cst_sutta from fragment if available
        let title = if let Some(ref cst_sutta) = fragment.cst_sutta {
            cst_sutta.clone()
        } else {
            // Fall back to extracting from group_levels
            let sutta_level = fragment.group_levels.iter()
                .find(|level| matches!(level.group_type, GroupType::Sutta));
            
            if let Some(level) = sutta_level {
                level.title.clone()
            } else {
                // No sutta title in group_levels - this fragment is a subsection heading
                // (e.g., "<p rend="subhead">Uddeso</p>" meaning "Summary") that was treated
                // as a fragment boundary during parsing but is not actually a separate sutta.
                // The content is preserved in the previous sutta fragment, so we skip this.
                continue;
            }
        };
        
        // Normalize title
        let normalized_title = consistent_niggahita(Some(title.clone()));
        
        // Extract nikaya name from first level
        let nikaya_name = fragment.group_levels.first()
            .map(|level| level.title.clone())
            .unwrap_or_else(|| nikaya_structure.nikaya.clone());
        
        // Build group path from hierarchy (excluding nikaya and sutta levels)
        let group_path = fragment.group_levels.iter()
            .filter(|level| !matches!(level.group_type, GroupType::Nikaya | GroupType::Sutta))
            .map(|level| level.title.clone())
            .collect::<Vec<_>>()
            .join(" / ");
        
        let group_path_opt = if group_path.is_empty() {
            None
        } else {
            Some(group_path)
        };
        
        // Get XML filename from fragment
        let cst_file = &fragment.cst_file;
        
        // Determine if this is a commentary or subcommentary
        let is_commentary = cst_file.ends_with(".att.xml");
        let is_subcommentary = cst_file.ends_with(".tik.xml");
        let is_commentary_or_sub = is_commentary || is_subcommentary;
        
        // Extract vagga from group_levels (if present)
        // For MN/SN, vagga structure typically aligns between base text and commentary
        // For DN, commentary doesn't have vagga structure (uses chapter=sutta directly)
        let vagga_title = fragment.group_levels.iter()
            .find(|level| matches!(level.group_type, GroupType::Vagga))
            .map(|level| level.title.as_str());
        
        // Get SC code - this is the primary identifier for suttas
        // Priority: sc_code > cst_code > TSV lookup
        let code = if let Some(ref sc_code) = fragment.sc_code {
            // Use the sc_code from TSV mapping (preferred)
            sc_code.clone()
        } else if let Some(ref cst_code) = fragment.cst_code {
            // Fall back to derived cst_code
            // Try to look it up in TSV to get the sc_code
            let sc_code_from_tsv = tsv_records.iter()
                .find(|r| r.cst_code == *cst_code)
                .map(|r| r.sc_code.clone());
            
            if let Some(sc) = sc_code_from_tsv {
                sc
            } else {
                // Use cst_code as fallback
                cst_code.clone()
            }
        } else {
            // Fall back to TSV lookup (legacy path)
            match find_code_for_sutta(&tsv_records, &cst_file, &title, vagga_title, is_commentary_or_sub, &used_codes) {
                Some(c) => c,
                None => {
                    // Log warning - could not find matching code
                    logger::warn(&format!("Could not find code for sutta '{}' in file '{}', skipping",
                             title, cst_file));
                    continue;
                }
            }
        };
        
        // Check if we've already used this code
        if used_codes.contains(&code) {
            logger::error(&format!("Code '{}' already used for a previous sutta, skipping duplicate for '{}' (file: {})",
                     code, title, cst_file));
            continue;
        }
        used_codes.insert(code.clone());
        
        // Add commentary/subcommentary suffix to code
        let uid_code = if is_commentary {
            format!("{}.att", code)
        } else if is_subcommentary {
            format!("{}.tik", code)
        } else {
            code.clone()
        };
        
        // Build full UID
        let uid = format!("{}/pli/cst4", uid_code);
        
        // Generate sutta reference
        let sutta_ref = uid_to_ref(&code);
        
        // Extract sutta number from code (e.g., "dn1" -> "1")
        let sutta_number = code.chars()
            .skip_while(|c| c.is_alphabetic())
            .collect::<String>();
        
        // Transform XML content to HTML
        let content_html = xml_to_html(&fragment.content)
            .context("Failed to transform XML to HTML")?;

        // Build HTML with header
        let normalized_full_html = consistent_niggahita(Some(format!(
            "<div class=\"cst4\">\n<header>\n<h3>{} {}</h3>\n<h1>{}</h1>\n</header>\n{}</div>",
            nikaya_name,
            sutta_number,
            html_escape::encode_text(&normalized_title),
            content_html
        )));
        
        // Extract plain text
        let normalized_content_plain = compact_rich_text(&content_html);

        // Build sutta record
        let sutta = SuttaRecord {
            uid,
            sutta_ref,
            nikaya: nikaya_structure.nikaya.clone(),
            language: "pli".to_string(),
            group_path: Some(consistent_niggahita(group_path_opt)),
            group_index: Some(idx as i32),
            order_index: Some(idx as i32),
            title: Some(normalized_title.clone()),
            title_pali: Some(normalized_title),
            content_plain: Some(normalized_content_plain),
            content_html: Some(normalized_full_html),
            source_uid: Some("cst4".to_string()),
        };
        
        suttas.push(sutta);
    }
    
    Ok(suttas)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_load_tsv_mapping() {
        // Test that the embedded TSV can be loaded and deserialized correctly
        let result = load_tsv_mapping();
        assert!(result.is_ok(), "Failed to load TSV mapping: {:?}", result.err());
        
        let records = result.unwrap();
        assert!(!records.is_empty(), "TSV mapping should contain records");
        
        // Verify first record has expected fields populated
        let first = &records[0];
        assert!(!first.cst_file.is_empty(), "cst_file should not be empty");
        assert!(!first.cst_code.is_empty(), "cst_code should not be empty");
        assert!(!first.sc_code.is_empty(), "sc_code should not be empty");
        
        // Verify known record (DN1) exists with correct mapping
        let dn1 = records.iter().find(|r| r.sc_code == "dn1");
        assert!(dn1.is_some(), "Should find DN1 record");
        
        if let Some(dn1) = dn1 {
            assert_eq!(dn1.cst_file.trim_start_matches("romn/").trim_start_matches("mula/"), "s0101m.mul.xml");
            assert_eq!(dn1.cst_sutta, "1. Brahmajālasuttaṃ");
            assert_eq!(dn1.cst_vagga, "");
        }
    }
}
