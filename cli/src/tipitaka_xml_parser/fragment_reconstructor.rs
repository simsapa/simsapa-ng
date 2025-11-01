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
/// * `xml_filename` - The xml_filename to look up in the nikaya table
///
/// # Returns
/// The reconstructed XML content as a string
pub fn reconstruct_xml_from_db(
    db_path: &Path,
    xml_filename: &str,
) -> Result<String> {
    let mut conn = SqliteConnection::establish(db_path.to_str().unwrap())
        .context("Failed to connect to fragments database")?;
    
    // Get nikaya name for this filename
    let _nikaya = get_nikaya_by_filename(&mut conn, xml_filename)?;
    
    // Get all fragments for this filename, ordered by line and char position
    let fragments = get_fragments_for_filename(&mut conn, xml_filename)?;
    
    // Reconstruct XML from fragments
    reconstruct_xml_from_fragments(&fragments)
}

/// Get nikaya name by xml_filename
fn get_nikaya_by_filename(
    conn: &mut SqliteConnection,
    xml_filename: &str,
) -> Result<String> {
    #[derive(QueryableByName)]
    struct NikayaResult {
        #[diesel(sql_type = diesel::sql_types::Text)]
        nikaya: String,
    }
    
    let result: NikayaResult = diesel::sql_query(
        "SELECT DISTINCT nikaya FROM xml_fragments WHERE xml_filename = ? LIMIT 1"
    )
    .bind::<diesel::sql_types::Text, _>(xml_filename)
    .get_result(conn)
    .context(format!("No nikaya found with filename: {}", xml_filename))?;
    
    Ok(result.nikaya)
}

/// Get all fragments for a filename, ordered by position
fn get_fragments_for_filename(
    conn: &mut SqliteConnection,
    xml_filename: &str,
) -> Result<Vec<XmlFragment>> {
    #[derive(QueryableByName)]
    struct FragmentRow {
        #[diesel(sql_type = diesel::sql_types::Text)]
        fragment_type: String,
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
        xml_filename: String,
    }
    
    let rows: Vec<FragmentRow> = diesel::sql_query(
        r#"
        SELECT fragment_type, content, start_line, end_line, start_char, end_char, group_levels, xml_filename
        FROM xml_fragments
        WHERE xml_filename = ?
        ORDER BY start_line ASC, start_char ASC
        "#
    )
    .bind::<diesel::sql_types::Text, _>(xml_filename)
    .load(conn)
    .context("Failed to query fragments")?;
    
    // Convert to XmlFragment
    let mut fragments = Vec::new();
    for row in rows {
        let fragment_type = match row.fragment_type.as_str() {
            "Header" => FragmentType::Header,
            "Sutta" => FragmentType::Sutta,
            _ => continue,
        };
        
        let group_levels: Vec<GroupLevel> = serde_json::from_str(&row.group_levels)
            .context("Failed to deserialize group levels")?;
        
        fragments.push(XmlFragment {
            fragment_type,
            content: row.content,
            start_line: row.start_line as usize,
            end_line: row.end_line as usize,
            start_char: row.start_char as usize,
            end_char: row.end_char as usize,
            group_levels,
            xml_filename: row.xml_filename,
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
    use crate::tipitaka_xml_parser_tsv::encoding::read_xml_file;
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
        
        let fragments = parse_into_fragments(original_xml, &structure, "test.xml").unwrap();
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
        let fragments = parse_into_fragments(original_xml, &structure, "test.att.xml").unwrap();
        
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
