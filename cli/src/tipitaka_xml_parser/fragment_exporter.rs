//! Export fragments and nikaya structure to SQLite database
//!
//! This module provides functionality to export parsed XML fragments and nikaya
//! structure to an SQLite database for inspection and debugging.

use anyhow::{Result, Context};
use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use std::path::Path;

use crate::tipitaka_xml_parser::types::XmlFragment;
use crate::tipitaka_xml_parser::nikaya_structure::NikayaStructure;
use crate::tipitaka_xml_parser::fragments_models::{NewNikayaStructure, NewXmlFragment};
use crate::tipitaka_xml_parser::fragments_schema::{nikaya_structures, xml_fragments};

// Embed the fragments migrations
pub const FRAGMENTS_MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations/fragments/");

/// Export fragments and nikaya structure to SQLite database
///
/// Creates two tables via diesel migrations:
/// - `nikaya_structures`: Stores the NikayaStructure with unique nikaya field
/// - `xml_fragments`: Stores each XmlFragment with cst_file and nikaya foreign key
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
    
    // Run migrations
    run_migrations(&mut conn)?;
    
    // Insert or get nikaya structure
    insert_nikaya_structure(&mut conn, nikaya_structure)?;
    
    // Insert fragments with nikaya foreign key
    let count = insert_fragments(&mut conn, fragments)?;
    
    Ok(count)
}

/// Run pending migrations for fragments database
fn run_migrations(conn: &mut SqliteConnection) -> Result<()> {
    conn.run_pending_migrations(FRAGMENTS_MIGRATIONS)
        .map_err(|e| anyhow::anyhow!("Failed to execute pending database migrations: {}", e))?;
    Ok(())
}

/// Insert nikaya structure into database using diesel model
fn insert_nikaya_structure(
    conn: &mut SqliteConnection,
    structure: &NikayaStructure,
) -> Result<()> {
    // Serialize levels as JSON
    let levels_json = serde_json::to_string(&structure.levels)
        .context("Failed to serialize nikaya levels")?;
    
    let new_nikaya = NewNikayaStructure {
        nikaya: &structure.nikaya,
        levels: &levels_json,
    };
    
    // Use INSERT OR IGNORE to handle duplicate nikayas
    diesel::insert_or_ignore_into(nikaya_structures::table)
        .values(&new_nikaya)
        .execute(conn)
        .context("Failed to insert nikaya structure")?;
    
    Ok(())
}

/// Insert fragments into database using diesel models
fn insert_fragments(
    conn: &mut SqliteConnection,
    fragments: &[XmlFragment],
) -> Result<usize> {
    let mut count = 0;
    
    for fragment in fragments {
        let frag_type = match fragment.frag_type {
            crate::tipitaka_xml_parser::types::FragmentType::Header => "Header",
            crate::tipitaka_xml_parser::types::FragmentType::Sutta => "Sutta",
        };
        
        let group_levels_json = serde_json::to_string(&fragment.group_levels)
            .context("Failed to serialize group levels")?;
        
        let new_fragment = NewXmlFragment {
            cst_file: &fragment.cst_file,
            frag_idx: fragment.frag_idx as i32,
            frag_type,
            frag_review: fragment.frag_review.as_deref(),
            nikaya: &fragment.nikaya,
            cst_code: fragment.cst_code.as_deref(),
            sc_code: fragment.sc_code.as_deref(),
            content: &fragment.content,
            cst_vagga: fragment.cst_vagga.as_deref(),
            cst_sutta: fragment.cst_sutta.as_deref(),
            cst_paranum: fragment.cst_paranum.as_deref(),
            sc_sutta: fragment.sc_sutta.as_deref(),
            start_line: fragment.start_line as i32,
            start_char: fragment.start_char as i32,
            end_line: fragment.end_line as i32,
            end_char: fragment.end_char as i32,
            group_levels: &group_levels_json,
        };
        
        diesel::insert_into(xml_fragments::table)
            .values(&new_fragment)
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
        };
        
        let fragments = vec![
            XmlFragment {
                nikaya: "digha".to_string(),
                frag_type: FragmentType::Header,
                content: "<p rend=\"nikaya\">Dīghanikāyo</p>".to_string(),
                start_line: 1,
                end_line: 1,
                start_char: 0,
                end_char: 34,
                group_levels: vec![],
                cst_file: "test.xml".to_string(),
                frag_idx: 0,
                frag_review: None,
                cst_code: None,
                cst_vagga: None,
                cst_sutta: None,
                cst_paranum: None,
                sc_code: None,
                sc_sutta: None,
            },
            XmlFragment {
                nikaya: "digha".to_string(),
                frag_type: FragmentType::Sutta,
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
                cst_file: "test.xml".to_string(),
                frag_idx: 1,
                frag_review: None,
                cst_code: None,
                cst_vagga: None,
                cst_sutta: None,
                cst_paranum: None,
                sc_code: None,
                sc_sutta: None,
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
        
        // Check nikaya_structures table
        let nikaya_result: CountResult = diesel::sql_query("SELECT COUNT(*) as count FROM nikaya_structures")
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
        };
        
        let structure2 = NikayaStructure {
            nikaya: "majjhima".to_string(),
            levels: vec![GroupType::Nikaya, GroupType::Book, GroupType::Vagga, GroupType::Sutta],
        };
        
        let fragments1 = vec![
            XmlFragment {
                nikaya: "digha".to_string(),
                frag_type: FragmentType::Header,
                content: "<p rend=\"nikaya\">Dīghanikāyo</p>".to_string(),
                start_line: 1,
                end_line: 1,
                start_char: 0,
                end_char: 34,
                group_levels: vec![],
                cst_file: "dn1.xml".to_string(),
                frag_idx: 0,
                frag_review: None,
                cst_code: None,
                cst_vagga: None,
                cst_sutta: None,
                cst_paranum: None,
                sc_code: None,
                sc_sutta: None,
            },
        ];
        
        let fragments2 = vec![
            XmlFragment {
                nikaya: "majjhima".to_string(),
                frag_type: FragmentType::Header,
                content: "<p rend=\"nikaya\">Majjhimanikāyo</p>".to_string(),
                start_line: 1,
                end_line: 1,
                start_char: 0,
                end_char: 35,
                group_levels: vec![],
                cst_file: "mn1.xml".to_string(),
                frag_idx: 0,
                frag_review: None,
                cst_code: None,
                cst_vagga: None,
                cst_sutta: None,
                cst_paranum: None,
                sc_code: None,
                sc_sutta: None,
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
        let nikaya_count: CountResult = diesel::sql_query("SELECT COUNT(*) as count FROM nikaya_structures")
            .get_result(&mut conn)
            .unwrap();
        assert_eq!(nikaya_count.count, 2);
        
        // Check we have 2 fragments total
        let fragment_count: CountResult = diesel::sql_query("SELECT COUNT(*) as count FROM xml_fragments")
            .get_result(&mut conn)
            .unwrap();
        assert_eq!(fragment_count.count, 2);
        
        // Verify each fragment has a valid nikaya
        #[derive(QueryableByName)]
        struct NikayaResult {
            #[diesel(sql_type = diesel::sql_types::Text)]
            nikaya: String,
        }
        
        let nikaya_ids: Vec<NikayaResult> = diesel::sql_query(
            "SELECT DISTINCT nikaya FROM xml_fragments ORDER BY nikaya"
        )
        .load(&mut conn)
        .unwrap();
        
        assert_eq!(nikaya_ids.len(), 2, "Should have fragments for 2 different nikayas");
        assert_eq!(nikaya_ids[0].nikaya, "digha", "First nikaya should be digha");
        assert_eq!(nikaya_ids[1].nikaya, "majjhima", "Second nikaya should be majjhima");
        
        // Verify we can query fragments by nikaya
        let digha_fragments: CountResult = diesel::sql_query(
            "SELECT COUNT(*) as count FROM xml_fragments WHERE nikaya = 'digha'"
        )
        .get_result(&mut conn)
        .unwrap();
        assert_eq!(digha_fragments.count, 1, "Digha nikaya should have 1 fragment");
        
        let majjhima_fragments: CountResult = diesel::sql_query(
            "SELECT COUNT(*) as count FROM xml_fragments WHERE nikaya = 'majjhima'"
        )
        .get_result(&mut conn)
        .unwrap();
        assert_eq!(majjhima_fragments.count, 1, "Majjhima nikaya should have 1 fragment");
    }
}
