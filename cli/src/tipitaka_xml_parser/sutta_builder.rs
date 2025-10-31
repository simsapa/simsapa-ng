//! Sutta record assembly from fragments
//!
//! This module provides functionality to assemble database records
//! from parsed XML fragments.

use anyhow::{Result, Context, bail};
use crate::tipitaka_xml_parser::types::{XmlFragment, FragmentType, GroupType};
use crate::tipitaka_xml_parser::nikaya_structure::NikayaStructure;
use simsapa_backend::helpers::consistent_niggahita;
use std::collections::HashMap;
use std::path::Path;
use quick_xml::{Reader, events::Event};
use html_escape;

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
#[derive(Debug, Clone)]
struct TsvRecord {
    cst_file: String,
    cst_sutta: String,
    code: String,
}

/// Load TSV mapping file into memory for lookups
fn load_tsv_mapping(tsv_path: &Path) -> Result<Vec<TsvRecord>> {
    use std::fs::File;
    use std::io::{BufRead, BufReader};
    
    let file = File::open(tsv_path)
        .context("Failed to open TSV mapping file")?;
    let reader = BufReader::new(file);
    
    let mut records = Vec::new();
    let mut lines = reader.lines();
    
    // Skip header line
    if let Some(Ok(_header)) = lines.next() {
        // Process data lines
        for line in lines {
            let line = line.context("Failed to read TSV line")?;
            let fields: Vec<&str> = line.split('\t').collect();
            
            if fields.len() >= 13 {
                records.push(TsvRecord {
                    cst_file: fields[11].to_string(),      // cst_file column
                    cst_sutta: fields[5].to_string(),      // cst_sutta column
                    code: fields[12].to_string(),          // code column
                });
            }
        }
    }
    
    Ok(records)
}

/// Find code for a given filename and sutta title
fn find_code_for_sutta(
    tsv_records: &[TsvRecord],
    xml_filename: &str,
    sutta_title: &str,
) -> Option<String> {
    // Normalize the xml filename (remove path prefix if present)
    let normalized_filename = xml_filename
        .trim_start_matches("romn/")
        .trim_start_matches("mula/");
    
    // Normalize sutta title for comparison (consistent niggahita)
    let normalized_title = consistent_niggahita(Some(sutta_title.to_string()));
    
    for record in tsv_records {
        let record_filename = record.cst_file
            .trim_start_matches("romn/")
            .trim_start_matches("mula/");
        
        let record_title = consistent_niggahita(Some(record.cst_sutta.clone()));
        
        if record_filename == normalized_filename && record_title == normalized_title {
            return Some(record.code.clone());
        }
    }
    
    None
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
    
    let mut buf = Vec::new();
    let mut in_paragraph = false;
    let mut current_para_class = String::new();
    let mut pending_paranum: Option<String> = None;
    
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
                        current_para_class = class.to_string();
                        in_paragraph = true;
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
                    _ => {
                        // Ignore unknown tags but preserve their content
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
                        in_paragraph = false;
                        current_para_class.clear();
                        pending_paranum = None;
                    }
                    "hi" => {
                        html.push_str("</span>");
                    }
                    "note" => {
                        html.push_str("]</span>");
                    }
                    _ => {}
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
                eprintln!("XML parsing error at position {}: {}", reader.buffer_position(), e);
            }
            _ => {}
        }
        
        buf.clear();
    }
    
    Ok(html)
}

/// Extract plain text from XML content (strip all tags)
fn xml_to_plain_text(xml_content: &str) -> Result<String> {
    let mut text = String::new();
    let mut reader = Reader::from_str(xml_content);
    reader.trim_text(false);
    
    let mut buf = Vec::new();
    let mut skip_note = false;
    
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                let name_bytes = e.name();
                let name = std::str::from_utf8(name_bytes.as_ref()).unwrap_or("");
                if name == "note" || name == "pb" {
                    skip_note = true;
                }
            }
            Ok(Event::End(ref e)) => {
                let name_bytes = e.name();
                let name = std::str::from_utf8(name_bytes.as_ref()).unwrap_or("");
                if name == "note" || name == "pb" {
                    skip_note = false;
                } else if name == "p" {
                    text.push('\n');
                }
            }
            Ok(Event::Text(e)) => {
                if !skip_note {
                    let content = e.unescape().unwrap_or_default();
                    text.push_str(&content);
                }
            }
            Ok(Event::Eof) => break,
            _ => {}
        }
        
        buf.clear();
    }
    
    // Normalize whitespace and apply consistent niggahita
    let normalized = text
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    
    Ok(consistent_niggahita(Some(normalized)))
}

/// Build sutta database records from fragments
///
/// # Arguments
/// * `fragments` - Vector of parsed fragments
/// * `nikaya_structure` - The structure configuration for this nikaya
/// * `tsv_path` - Path to cst-vs-sc.tsv mapping file
///
/// # Returns
/// Vector of sutta records or error if assembly fails
pub fn build_suttas(
    fragments: Vec<XmlFragment>,
    nikaya_structure: &NikayaStructure,
    tsv_path: &Path,
) -> Result<Vec<SuttaRecord>> {
    // Load TSV mapping
    let tsv_records = load_tsv_mapping(tsv_path)
        .context("Failed to load TSV mapping file")?;
    
    let mut suttas = Vec::new();
    
    // Group sutta fragments
    let sutta_fragments: Vec<&XmlFragment> = fragments.iter()
        .filter(|f| matches!(f.fragment_type, FragmentType::Sutta))
        .collect();
    
    for (idx, fragment) in sutta_fragments.iter().enumerate() {
        // Extract sutta title from group_levels
        let sutta_level = fragment.group_levels.iter()
            .find(|level| matches!(level.group_type, GroupType::Sutta));
        
        let title = if let Some(level) = sutta_level {
            level.title.clone()
        } else {
            format!("Sutta {}", idx + 1)
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
            .map(|level| consistent_niggahita(Some(level.title.clone())))
            .collect::<Vec<_>>()
            .join(" / ");
        
        let group_path_opt = if group_path.is_empty() {
            None
        } else {
            Some(group_path)
        };
        
        // Get XML filename (with proper extension handling)
        let xml_filename = nikaya_structure.xml_filename.clone()
            .unwrap_or_else(|| "unknown.xml".to_string());
        
        // Determine if this is a commentary or subcommentary
        let is_commentary = xml_filename.ends_with(".att.xml");
        let is_subcommentary = xml_filename.ends_with(".tik.xml");
        
        // Find code from TSV
        let code = find_code_for_sutta(&tsv_records, &xml_filename, &title)
            .context(format!(
                "Failed to find code for sutta '{}' in file '{}'", 
                title, xml_filename
            ))?;
        
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
        let full_html = format!(
            "<div class=\"cst4\">\n<header>\n<h3>{} {}</h3>\n<h1>{}</h1>\n</header>\n{}</div>",
            nikaya_name,
            sutta_number,
            html_escape::encode_text(&normalized_title),
            content_html
        );
        
        // Extract plain text
        let content_plain = xml_to_plain_text(&fragment.content)
            .context("Failed to extract plain text")?;
        
        // Build sutta record
        let sutta = SuttaRecord {
            uid,
            sutta_ref,
            nikaya: nikaya_structure.nikaya.clone(),
            language: "pli".to_string(),
            group_path: group_path_opt,
            group_index: Some(idx as i32),
            order_index: Some(idx as i32),
            title: Some(normalized_title.clone()),
            title_pali: Some(normalized_title),
            content_plain: Some(content_plain),
            content_html: Some(full_html),
            source_uid: Some("cst4".to_string()),
        };
        
        suttas.push(sutta);
    }
    
    Ok(suttas)
}
