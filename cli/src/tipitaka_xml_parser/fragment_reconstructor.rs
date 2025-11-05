//! Reconstruct XML files from fragments database
//!
//! This module provides functionality to read fragments from the SQLite database
//! and reconstruct the original XML file.

use anyhow::{Result, Context};
use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use std::path::Path;

use crate::tipitaka_xml_parser::types::{XmlFragment, FragmentType, GroupLevel, GroupType};
use crate::tipitaka_xml_parser::nikaya_structure::NikayaStructure;

/// Reconstruct XML content from fragments database by filename
///
/// # Arguments
/// * `db_path` - Path to the fragments SQLite database
/// * `cst_file` - The cst_file to look up in the nikaya table
///
/// # Returns
/// The reconstructed XML content as a string
pub fn reconstruct_xml_from_db(
    db_path: &Path,
    cst_file: &str,
) -> Result<String> {
    let mut conn = SqliteConnection::establish(db_path.to_str().unwrap())
        .context("Failed to connect to fragments database")?;
    
    // Get nikaya name for this filename
    let _nikaya = get_nikaya_by_filename(&mut conn, cst_file)?;
    
    // Get all fragments for this filename, ordered by line and char position
    let fragments = get_fragments_for_filename(&mut conn, cst_file)?;
    
    // Reconstruct XML from fragments
    reconstruct_xml_from_fragments(&fragments)
}

/// Get nikaya name by cst_file
fn get_nikaya_by_filename(
    conn: &mut SqliteConnection,
    cst_file: &str,
) -> Result<String> {
    #[derive(QueryableByName)]
    struct NikayaResult {
        #[diesel(sql_type = diesel::sql_types::Text)]
        nikaya: String,
    }
    
    let result: NikayaResult = diesel::sql_query(
        "SELECT DISTINCT nikaya FROM xml_fragments WHERE cst_file = ? LIMIT 1"
    )
    .bind::<diesel::sql_types::Text, _>(cst_file)
    .get_result(conn)
    .context(format!("No nikaya found with filename: {}", cst_file))?;
    
    Ok(result.nikaya)
}

/// Get all fragments for a filename, ordered by position
fn get_fragments_for_filename(
    conn: &mut SqliteConnection,
    cst_file: &str,
) -> Result<Vec<XmlFragment>> {
    #[derive(QueryableByName)]
    struct FragmentRow {
        #[diesel(sql_type = diesel::sql_types::Text)]
        frag_type: String,
        #[diesel(sql_type = diesel::sql_types::Text)]
        content: String,
        #[diesel(sql_type = diesel::sql_types::Integer)]
        start_line: i32,
        #[diesel(sql_type = diesel::sql_types::Integer)]
        end_line: i32,
        #[diesel(sql_type = diesel::sql_types::Integer)]
        start_char: i32,
        #[diesel(sql_type = diesel::sql_types::Integer)]
        end_char: i32,
        #[diesel(sql_type = diesel::sql_types::Text)]
        group_levels: String,
        #[diesel(sql_type = diesel::sql_types::Text)]
        cst_file: String,
        #[diesel(sql_type = diesel::sql_types::Integer)]
        frag_idx: i32,
    }
    
    let rows: Vec<FragmentRow> = diesel::sql_query(
        r#"
        SELECT frag_type, content, start_line, end_line, start_char, end_char, group_levels, cst_file, frag_idx
        FROM xml_fragments
        WHERE cst_file = ?
        ORDER BY start_line ASC, start_char ASC
        "#
    )
    .bind::<diesel::sql_types::Text, _>(cst_file)
    .load(conn)
    .context("Failed to query fragments")?;
    
    // Convert to XmlFragment
    let mut fragments = Vec::new();
    for row in rows {
        let frag_type = match row.frag_type.as_str() {
            "Header" => FragmentType::Header,
            "Sutta" => FragmentType::Sutta,
            _ => continue,
        };
        
        let group_levels: Vec<GroupLevel> = serde_json::from_str(&row.group_levels)
            .context("Failed to deserialize group levels")?;
        
        fragments.push(XmlFragment {
            frag_type,
            content: row.content,
            start_line: row.start_line as usize,
            end_line: row.end_line as usize,
            start_char: row.start_char as usize,
            end_char: row.end_char as usize,
            group_levels,
            cst_file: row.cst_file,
            frag_idx: row.frag_idx as usize,
            frag_review: None,
            cst_code: None,
            cst_vagga: None,
            cst_sutta: None,
            cst_paranum: None,
            sc_code: None,
            sc_sutta: None,
        });
    }
    
    Ok(fragments)
}

/// Reconstruct XML content from ordered fragments
fn reconstruct_xml_from_fragments(fragments: &[XmlFragment]) -> Result<String> {
    if fragments.is_empty() {
        return Err(anyhow::anyhow!("No fragments to reconstruct"));
    }
    
    // Simply concatenate all fragment contents in order
    let mut xml = String::new();
    for fragment in fragments {
        xml.push_str(&fragment.content);
    }
    
    Ok(xml)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    use crate::tipitaka_xml_parser::{detect_nikaya_structure, parse_into_fragments, export_fragments_to_db};
    use crate::tipitaka_xml_parser::encoding::read_xml_file;
    use std::path::PathBuf;
    
    #[test]
    fn test_roundtrip_reconstruction() {
        // Create a simple XML sample
        let original_xml = r#"<?xml version="1.0"?>
<TEI.2>
<teiHeader></teiHeader>
<text>
<body>
<p rend="nikaya">Dīghanikāyo</p>
<div id="dn1" type="book">
<head rend="book">Sīlakkhandhavaggapāḷi</head>
<div id="dn1_1" type="sutta">
<head rend="chapter">1. Brahmajālasutta</head>
<p rend="bodytext" n="1">Evaṃ me sutaṃ.</p>
</div>
</div>
</body>
</text>
</TEI.2>"#;
        
        let temp_db = NamedTempFile::new().unwrap();
        let db_path = temp_db.path();
        
        // Parse and export
        let structure = detect_nikaya_structure(original_xml).unwrap();
        
        let fragments = parse_into_fragments(original_xml, &structure, "test.xml", None).unwrap();
        export_fragments_to_db(&fragments, &structure, db_path).unwrap();
        
        // Reconstruct
        let reconstructed_xml = reconstruct_xml_from_db(db_path, "test.xml").unwrap();
        
        // Verify - should be identical (whitespace may differ)
        assert_eq!(original_xml.trim(), reconstructed_xml.trim());
    }
    
    #[test]
    fn test_roundtrip_commentary_style() {
        // Test reconstruction with commentary-style XML (DN .att.xml)
        let original_xml = r#"<?xml version="1.0"?>
<TEI.2>
<teiHeader></teiHeader>
<text>
<body>
<p rend="nikaya">Dīghanikāyo</p>
<div type="book">
<head rend="book">Sīlakkhandhavaggapāḷi</head>
<p>Header content before suttas.</p>
<head rend="chapter">1. Brahmajālasuttavaṇṇanā</head>
<p>First sutta commentary.</p>
<head rend="chapter">2. Sāmaññaphalasuttavaṇṇanā</head>
<p>Second sutta commentary.</p>
</div>
</body>
</text>
</TEI.2>"#;
        
        let temp_db = NamedTempFile::new().unwrap();
        let db_path = temp_db.path();
        
        // Parse as commentary file
        let structure = detect_nikaya_structure(original_xml).unwrap();
        let fragments = parse_into_fragments(original_xml, &structure, "test.att.xml", None).unwrap();
        
        // Verify head tags are together
        for frag in &fragments {
            if frag.content.contains("<head rend=\"chapter\">") {
                assert!(frag.content.contains("</head>"),
                    "Fragment with <head rend=\"chapter\"> must also contain </head>");
            }
        }
        
        export_fragments_to_db(&fragments, &structure, db_path).unwrap();
        
        // Reconstruct
        let reconstructed_xml = reconstruct_xml_from_db(db_path, "test.att.xml").unwrap();
        
        // Verify - should be identical
        assert_eq!(original_xml.trim(), reconstructed_xml.trim(),
            "Reconstructed commentary XML should match original");
    }
}
