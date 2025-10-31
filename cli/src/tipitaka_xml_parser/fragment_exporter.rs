//! Export fragments and nikaya structure to SQLite database
//!
//! This module provides functionality to export parsed XML fragments and nikaya
//! structure to an SQLite database for inspection and debugging.

use anyhow::{Result, Context};
use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use std::path::Path;

use crate::tipitaka_xml_parser::types::XmlFragment;
use crate::tipitaka_xml_parser::nikaya_structure::NikayaStructure;

/// Export fragments and nikaya structure to SQLite database
///
/// Creates two tables:
/// - `nikaya`: Stores the NikayaStructure as JSON
/// - `xml_fragments`: Stores each XmlFragment with position tracking
///
/// # Arguments
/// * `fragments` - Vector of parsed XML fragments
/// * `nikaya_structure` - The nikaya structure configuration
/// * `db_path` - Path to the SQLite database file (will be created if doesn't exist)
///
/// # Returns
/// Number of fragments exported or error
pub fn export_fragments_to_db(
    fragments: &[XmlFragment],
    nikaya_structure: &NikayaStructure,
    db_path: &Path,
) -> Result<usize> {
    // Connect to database
    let mut conn = SqliteConnection::establish(db_path.to_str().unwrap())
        .context("Failed to connect to fragments database")?;
    
    // Create tables
    create_tables(&mut conn)?;
    
    // Insert nikaya structure and get the ID
    let nikaya_id = insert_nikaya_structure(&mut conn, nikaya_structure)?;
    
    // Insert fragments with nikaya_id
    let count = insert_fragments(&mut conn, fragments, nikaya_id)?;
    
    Ok(count)
}

/// Create database tables for fragments and nikaya
fn create_tables(conn: &mut SqliteConnection) -> Result<()> {
    // Create nikaya table
    diesel::sql_query(
        r#"
        CREATE TABLE IF NOT EXISTS nikaya (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            nikaya TEXT NOT NULL,
            levels TEXT NOT NULL,
            xml_filename TEXT,
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
        )
        "#
    )
    .execute(conn)
    .context("Failed to create nikaya table")?;
    
    // Create xml_fragments table with foreign key to nikaya
    diesel::sql_query(
        r#"
        CREATE TABLE IF NOT EXISTS xml_fragments (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            nikaya_id INTEGER NOT NULL,
            fragment_type TEXT NOT NULL,
            content TEXT NOT NULL,
            start_line INTEGER NOT NULL,
            end_line INTEGER NOT NULL,
            start_char INTEGER NOT NULL,
            end_char INTEGER NOT NULL,
            group_levels TEXT NOT NULL,
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (nikaya_id) REFERENCES nikaya(id)
        )
        "#
    )
    .execute(conn)
    .context("Failed to create xml_fragments table")?;
    
    Ok(())
}

/// Insert nikaya structure into database and return the ID
fn insert_nikaya_structure(
    conn: &mut SqliteConnection,
    structure: &NikayaStructure,
) -> Result<i64> {
    // Serialize levels as JSON
    let levels_json = serde_json::to_string(&structure.levels)
        .context("Failed to serialize nikaya levels")?;
    
    let xml_filename = structure.xml_filename.as_deref().unwrap_or("");
    
    diesel::sql_query(
        r#"
        INSERT INTO nikaya (nikaya, levels, xml_filename)
        VALUES (?, ?, ?)
        "#
    )
    .bind::<diesel::sql_types::Text, _>(&structure.nikaya)
    .bind::<diesel::sql_types::Text, _>(&levels_json)
    .bind::<diesel::sql_types::Text, _>(xml_filename)
    .execute(conn)
    .context("Failed to insert nikaya structure")?;
    
    // Get the last inserted row ID
    #[derive(QueryableByName)]
    struct LastInsertId {
        #[diesel(sql_type = diesel::sql_types::BigInt)]
        id: i64,
    }
    
    let result: LastInsertId = diesel::sql_query("SELECT last_insert_rowid() as id")
        .get_result(conn)
        .context("Failed to get nikaya ID")?;
    
    Ok(result.id)
}

/// Insert fragments into database with foreign key to nikaya
fn insert_fragments(
    conn: &mut SqliteConnection,
    fragments: &[XmlFragment],
    nikaya_id: i64,
) -> Result<usize> {
    let mut count = 0;
    
    for fragment in fragments {
        let fragment_type = match fragment.fragment_type {
            crate::tipitaka_xml_parser::types::FragmentType::Header => "Header",
            crate::tipitaka_xml_parser::types::FragmentType::Sutta => "Sutta",
        };
        
        let group_levels_json = serde_json::to_string(&fragment.group_levels)
            .context("Failed to serialize group levels")?;
        
        diesel::sql_query(
            r#"
            INSERT INTO xml_fragments 
            (nikaya_id, fragment_type, content, start_line, end_line, start_char, end_char, group_levels)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#
        )
        .bind::<diesel::sql_types::BigInt, _>(nikaya_id)
        .bind::<diesel::sql_types::Text, _>(fragment_type)
        .bind::<diesel::sql_types::Text, _>(&fragment.content)
        .bind::<diesel::sql_types::Integer, _>(fragment.start_line as i32)
        .bind::<diesel::sql_types::Integer, _>(fragment.end_line as i32)
        .bind::<diesel::sql_types::Integer, _>(fragment.start_char as i32)
        .bind::<diesel::sql_types::Integer, _>(fragment.end_char as i32)
        .bind::<diesel::sql_types::Text, _>(&group_levels_json)
        .execute(conn)
        .context("Failed to insert fragment")?;
        
        count += 1;
    }
    
    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    use crate::tipitaka_xml_parser::types::{FragmentType, GroupLevel, GroupType};
    
    #[test]
    fn test_export_fragments() {
        let temp_db = NamedTempFile::new().unwrap();
        let db_path = temp_db.path();
        
        // Create test data
        let structure = NikayaStructure {
            nikaya: "digha".to_string(),
            levels: vec![GroupType::Nikaya, GroupType::Book, GroupType::Sutta],
            xml_filename: Some("test.xml".to_string()),
        };
        
        let fragments = vec![
            XmlFragment {
                fragment_type: FragmentType::Header,
                content: "<p rend=\"nikaya\">Dīghanikāyo</p>".to_string(),
                start_line: 1,
                end_line: 1,
                start_char: 0,
                end_char: 34,
                group_levels: vec![],
            },
            XmlFragment {
                fragment_type: FragmentType::Sutta,
                content: "<p>Test content</p>".to_string(),
                start_line: 2,
                end_line: 2,
                start_char: 0,
                end_char: 19,
                group_levels: vec![
                    GroupLevel {
                        group_type: GroupType::Nikaya,
                        group_number: None,
                        title: "Dīghanikāyo".to_string(),
                        id: None,
                    },
                ],
            },
        ];
        
        // Export
        let count = export_fragments_to_db(&fragments, &structure, db_path).unwrap();
        assert_eq!(count, 2);
        
        // Verify by querying
        let mut conn = SqliteConnection::establish(db_path.to_str().unwrap()).unwrap();
        
        #[derive(QueryableByName)]
        struct CountResult {
            #[diesel(sql_type = diesel::sql_types::BigInt)]
            count: i64,
        }
        
        // Check nikaya table
        let nikaya_result: CountResult = diesel::sql_query("SELECT COUNT(*) as count FROM nikaya")
            .get_result(&mut conn)
            .unwrap();
        assert_eq!(nikaya_result.count, 1);
        
        // Check fragments table
        let fragment_result: CountResult = diesel::sql_query("SELECT COUNT(*) as count FROM xml_fragments")
            .get_result(&mut conn)
            .unwrap();
        assert_eq!(fragment_result.count, 2);
    }

    #[test]
    fn test_foreign_key_relationship() {
        let temp_db = NamedTempFile::new().unwrap();
        let db_path = temp_db.path();
        
        // Create test data with two different nikayas
        let structure1 = NikayaStructure {
            nikaya: "digha".to_string(),
            levels: vec![GroupType::Nikaya, GroupType::Book, GroupType::Sutta],
            xml_filename: Some("dn1.xml".to_string()),
        };
        
        let structure2 = NikayaStructure {
            nikaya: "majjhima".to_string(),
            levels: vec![GroupType::Nikaya, GroupType::Book, GroupType::Vagga, GroupType::Sutta],
            xml_filename: Some("mn1.xml".to_string()),
        };
        
        let fragments1 = vec![
            XmlFragment {
                fragment_type: FragmentType::Header,
                content: "<p rend=\"nikaya\">Dīghanikāyo</p>".to_string(),
                start_line: 1,
                end_line: 1,
                start_char: 0,
                end_char: 34,
                group_levels: vec![],
            },
        ];
        
        let fragments2 = vec![
            XmlFragment {
                fragment_type: FragmentType::Header,
                content: "<p rend=\"nikaya\">Majjhimanikāyo</p>".to_string(),
                start_line: 1,
                end_line: 1,
                start_char: 0,
                end_char: 35,
                group_levels: vec![],
            },
        ];
        
        // Export first nikaya and its fragments
        export_fragments_to_db(&fragments1, &structure1, db_path).unwrap();
        
        // Export second nikaya and its fragments
        export_fragments_to_db(&fragments2, &structure2, db_path).unwrap();
        
        // Verify relationships
        let mut conn = SqliteConnection::establish(db_path.to_str().unwrap()).unwrap();
        
        #[derive(QueryableByName)]
        struct CountResult {
            #[diesel(sql_type = diesel::sql_types::BigInt)]
            count: i64,
        }
        
        #[derive(QueryableByName)]
        struct NikayaIdResult {
            #[diesel(sql_type = diesel::sql_types::BigInt)]
            nikaya_id: i64,
        }
        
        // Check we have 2 nikayas
        let nikaya_count: CountResult = diesel::sql_query("SELECT COUNT(*) as count FROM nikaya")
            .get_result(&mut conn)
            .unwrap();
        assert_eq!(nikaya_count.count, 2);
        
        // Check we have 2 fragments total
        let fragment_count: CountResult = diesel::sql_query("SELECT COUNT(*) as count FROM xml_fragments")
            .get_result(&mut conn)
            .unwrap();
        assert_eq!(fragment_count.count, 2);
        
        // Verify each fragment has a valid nikaya_id
        let nikaya_ids: Vec<NikayaIdResult> = diesel::sql_query(
            "SELECT DISTINCT nikaya_id FROM xml_fragments ORDER BY nikaya_id"
        )
        .load(&mut conn)
        .unwrap();
        
        assert_eq!(nikaya_ids.len(), 2, "Should have fragments for 2 different nikayas");
        assert_eq!(nikaya_ids[0].nikaya_id, 1, "First nikaya should have ID 1");
        assert_eq!(nikaya_ids[1].nikaya_id, 2, "Second nikaya should have ID 2");
        
        // Verify we can query fragments by nikaya
        let digha_fragments: CountResult = diesel::sql_query(
            "SELECT COUNT(*) as count FROM xml_fragments WHERE nikaya_id = 1"
        )
        .get_result(&mut conn)
        .unwrap();
        assert_eq!(digha_fragments.count, 1, "Digha nikaya should have 1 fragment");
        
        let majjhima_fragments: CountResult = diesel::sql_query(
            "SELECT COUNT(*) as count FROM xml_fragments WHERE nikaya_id = 2"
        )
        .get_result(&mut conn)
        .unwrap();
        assert_eq!(majjhima_fragments.count, 1, "Majjhima nikaya should have 1 fragment");
    }
}
