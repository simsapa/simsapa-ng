//! Database insertion functionality
//!
//! This module provides functionality to insert sutta records into
//! the appdata database.

use std::path::Path;
use anyhow::{Result, Context};
use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use simsapa_backend::db::appdata_schema::suttas;
use simsapa_backend::db::appdata_models::NewSutta;
use crate::tipitaka_xml_parser::sutta_builder::SuttaRecord;

/// Initialize database with schema if it doesn't exist
pub fn initialize_database(db_path: &Path) -> Result<()> {
    use std::fs;
    
    // Check if database exists
    let db_exists = db_path.exists();
    
    if !db_exists {
        // Create parent directory if needed
        if let Some(parent) = db_path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directory: {:?}", parent))?;
        }
        
        // Connect to database (this creates the file)
        let mut conn = establish_connection(db_path)?;
        
        // Run migration SQL manually using rusqlite since diesel doesn't support batch_execute
        use rusqlite::Connection;
        
        let migration_sql = include_str!("../../../backend/migrations/appdata/2025-03-18-165332_create_tables/up.sql");
        
        // Use rusqlite to execute the migration
        let rusqlite_conn = Connection::open(db_path)
            .context("Failed to open database with rusqlite")?;
        
        rusqlite_conn.execute_batch(migration_sql)
            .context("Failed to run database migrations")?;
    }
    
    Ok(())
}

/// Establish database connection
fn establish_connection(db_path: &Path) -> Result<SqliteConnection> {
    let db_url = db_path.to_str()
        .ok_or_else(|| anyhow::anyhow!("Invalid database path"))?;
    
    SqliteConnection::establish(db_url)
        .with_context(|| format!("Failed to connect to database: {}", db_url))
}

/// Insert sutta records into the database
///
/// # Arguments
/// * `suttas` - Vector of sutta records to insert
/// * `db_path` - Path to the database file
///
/// # Returns
/// Number of records inserted or error if insertion fails
pub fn insert_suttas(sutta_records: Vec<SuttaRecord>, db_path: &Path) -> Result<usize> {
    let mut conn = establish_connection(db_path)?;
    let mut inserted_count = 0;
    
    // Use a transaction for batch insertion
    conn.transaction::<_, anyhow::Error, _>(|conn| {
        for record in &sutta_records {
            // Convert SuttaRecord to NewSutta
            let new_sutta = NewSutta {
                uid: &record.uid,
                sutta_ref: &record.sutta_ref,
                nikaya: &record.nikaya,
                language: &record.language,
                group_path: record.group_path.as_deref(),
                group_index: record.group_index,
                order_index: record.order_index,
                sutta_range_group: None,
                sutta_range_start: None,
                sutta_range_end: None,
                title: record.title.as_deref(),
                title_ascii: None,
                title_pali: record.title_pali.as_deref(),
                title_trans: None,
                description: None,
                content_plain: record.content_plain.as_deref(),
                content_html: record.content_html.as_deref(),
                content_json: None,
                content_json_tmpl: None,
                source_uid: record.source_uid.as_deref(),
                source_info: None,
                source_language: Some("pli"),
                message: None,
                copyright: None,
                license: None,
            };
            
            // Check if UID already exists
            use simsapa_backend::db::appdata_schema::suttas::dsl::*;
            let existing: Option<i32> = suttas
                .filter(uid.eq(&record.uid))
                .select(diesel::dsl::sql::<diesel::sql_types::Integer>("1"))
                .first(conn)
                .optional()
                .context("Failed to check for existing sutta")?;
            
            if existing.is_some() {
                // Update existing record
                diesel::update(suttas.filter(uid.eq(&record.uid)))
                    .set((
                        sutta_ref.eq(&record.sutta_ref),
                        nikaya.eq(&record.nikaya),
                        language.eq(&record.language),
                        group_path.eq(record.group_path.as_deref()),
                        group_index.eq(record.group_index),
                        order_index.eq(record.order_index),
                        title.eq(record.title.as_deref()),
                        title_pali.eq(record.title_pali.as_deref()),
                        content_plain.eq(record.content_plain.as_deref()),
                        content_html.eq(record.content_html.as_deref()),
                        source_uid.eq(record.source_uid.as_deref()),
                        source_language.eq(Some("pli")),
                    ))
                    .execute(conn)
                    .context("Failed to update existing sutta")?;
            } else {
                // Insert new record
                diesel::insert_into(suttas)
                    .values(&new_sutta)
                    .execute(conn)
                    .context("Failed to insert sutta")?;
            }
            
            inserted_count += 1;
        }
        
        Ok(())
    })?;
    
    Ok(inserted_count)
}
