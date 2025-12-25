use std::path::{Path, PathBuf};
use std::error::Error;

use diesel::prelude::*;
use diesel::RunQueryDsl;

use crate::logger::{info, error};
use crate::db::appdata_models::Sutta;
use crate::db::appdata_schema::suttas;
use crate::normalize_path_for_sqlite;
use crate::helpers::sutta_range_from_ref;

/// Import suttas from language database files into appdata
///
/// Finds all suttas_lang_*.sqlite3 files in extract_temp_dir and imports them to appdata.
pub fn import_suttas_lang_to_appdata(extract_temp_dir: &Path, target_database_url: &str) -> Result<(), Box<dyn Error>> {
    info(&format!("import_suttas_lang_to_appdata() to: {}", target_database_url));

    // Find all suttas_lang_*.sqlite3 files in extract_temp_dir
    let entries = match std::fs::read_dir(extract_temp_dir) {
        Ok(e) => e,
        Err(e) => {
            error(&format!("Failed to read extract_temp_dir: {}", e));
            return Err(Box::new(e));
        }
    };

    for entry in entries {
        let entry = match entry {
            Ok(e) => e,
            Err(e) => {
                error(&format!("Failed to read directory entry: {}", e));
                continue;
            }
        };

        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        let file_name = match path.file_name().and_then(|n| n.to_str()) {
            Some(n) => n,
            None => continue,
        };

        if file_name.starts_with("suttas_lang_") && file_name.ends_with(".sqlite3") {
            info(&format!("Importing suttas from: {}", file_name));

            match import_suttas_from_db(&path, target_database_url) {
                Ok(_) => {
                    info(&format!("Successfully imported {}", file_name));
                    // Remove the language db file after successful import
                    let _ = std::fs::remove_file(&path);
                }
                Err(e) => {
                    error(&format!("Failed to import {}: {}", file_name, e));
                }
            }
        }
    }

    Ok(())
}

/// Import suttas from a language database into target database (appdata)
///
/// Reads suttas from import_db_path and inserts them into target database,
/// replacing any existing suttas with the same uid.
pub fn import_suttas_from_db(import_db_path: &PathBuf, target_database_url: &str) -> Result<(), Box<dyn Error>> {
    info(&format!("import_suttas_from_db(): {:?} -> {}", import_db_path, target_database_url));

    // Establish connection to import database
    let import_abs_path = normalize_path_for_sqlite(std::fs::canonicalize(import_db_path)?);
    let import_database_url = format!("sqlite://{}", import_abs_path.to_str().ok_or("Path conversion error")?);

    let mut import_conn = SqliteConnection::establish(&import_database_url)
        .map_err(|e| format!("Failed to connect to import database: {}", e))?;

    // Establish connection to target database
    let mut target_conn = SqliteConnection::establish(target_database_url)
        .map_err(|e| format!("Failed to connect to target database: {}", e))?;

    // Read all suttas from import database
    let suttas_to_import: Vec<Sutta> = suttas::table
        .load::<Sutta>(&mut import_conn)
        .map_err(|e| format!("Failed to load suttas from import database: {}", e))?;

    let count = suttas_to_import.len();
    info(&format!("Importing {} suttas", count));

    // Import each sutta into target database
    for sutta in suttas_to_import {
        // Delete any existing sutta with the same uid
        diesel::delete(suttas::table.filter(suttas::uid.eq(&sutta.uid)))
            .execute(&mut target_conn)
            .map_err(|e| format!("Failed to delete existing sutta: {}", e))?;

        // If range fields are missing (NULL), calculate them from the UID
        let (range_group, range_start, range_end) = if sutta.sutta_range_group.is_none()
            || sutta.sutta_range_start.is_none()
            || sutta.sutta_range_end.is_none() {
            if let Some(range) = sutta_range_from_ref(&sutta.uid) {
                let start = range.start.map(|s| s as i32);
                let end = range.end.map(|e| e as i32);
                (Some(range.group), start, end)
            } else {
                (None, None, None)
            }
        } else {
            (sutta.sutta_range_group.clone(), sutta.sutta_range_start, sutta.sutta_range_end)
        };

        // Insert the new sutta
        // We need to create a new insert without the id field
        diesel::insert_into(suttas::table)
            .values((
                suttas::uid.eq(&sutta.uid),
                suttas::sutta_ref.eq(&sutta.sutta_ref),
                suttas::nikaya.eq(&sutta.nikaya),
                suttas::language.eq(&sutta.language),
                suttas::group_path.eq(&sutta.group_path),
                suttas::group_index.eq(&sutta.group_index),
                suttas::order_index.eq(&sutta.order_index),
                suttas::sutta_range_group.eq(&range_group),
                suttas::sutta_range_start.eq(&range_start),
                suttas::sutta_range_end.eq(&range_end),
                suttas::title.eq(&sutta.title),
                suttas::title_ascii.eq(&sutta.title_ascii),
                suttas::title_pali.eq(&sutta.title_pali),
                suttas::title_trans.eq(&sutta.title_trans),
                suttas::description.eq(&sutta.description),
                suttas::content_plain.eq(&sutta.content_plain),
                suttas::content_html.eq(&sutta.content_html),
                suttas::content_json.eq(&sutta.content_json),
                suttas::content_json_tmpl.eq(&sutta.content_json_tmpl),
                suttas::source_uid.eq(&sutta.source_uid),
                suttas::source_info.eq(&sutta.source_info),
                suttas::source_language.eq(&sutta.source_language),
                suttas::message.eq(&sutta.message),
                suttas::copyright.eq(&sutta.copyright),
                suttas::license.eq(&sutta.license),
            ))
            .execute(&mut target_conn)
            .map_err(|e| format!("Failed to insert sutta {}: {}", sutta.uid, e))?;
    }

    info(&format!("Successfully imported {} suttas", count));
    Ok(())
}
