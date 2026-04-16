use std::collections::HashMap;
use std::fs;
use std::io::{Read, Write};
use std::path::Path;

use anyhow::{Context, Result};
use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use diesel_migrations::MigrationHarness;
use zip::write::SimpleFileOptions;

use crate::db::appdata::AppdataDbHandle;
use crate::db::appdata_models::*;
use crate::db::APPDATA_MIGRATIONS;
use crate::get_chanting_recordings_dir;
use crate::logger::info;

/// Create a standalone SQLite database containing chanting data.
/// Runs appdata migrations to create the schema, then inserts the provided rows.
pub fn create_chanting_sqlite(
    dest_path: &Path,
    collections: &[ChantingCollection],
    chants: &[ChantingChant],
    sections: &[ChantingSection],
    recordings: &[ChantingRecording],
) -> Result<()> {
    use crate::db::appdata_schema::chanting_collections::dsl as col_dsl;
    use crate::db::appdata_schema::chanting_chants::dsl as chant_dsl;
    use crate::db::appdata_schema::chanting_sections::dsl as sec_dsl;
    use crate::db::appdata_schema::chanting_recordings::dsl as rec_dsl;

    if let Ok(true) = dest_path.try_exists() {
        fs::remove_file(dest_path)
            .with_context(|| format!("Failed to remove existing: {}", dest_path.display()))?;
    }

    let db_url = format!("sqlite://{}", dest_path.display());
    let mut conn = SqliteConnection::establish(&db_url)
        .with_context(|| format!("Failed to create export database: {}", dest_path.display()))?;

    conn.run_pending_migrations(APPDATA_MIGRATIONS)
        .map_err(|e| anyhow::anyhow!("Failed to run migrations on export database: {}", e))?;

    for col in collections {
        let new = NewChantingCollection {
            uid: &col.uid,
            title: &col.title,
            description: col.description.as_deref(),
            language: &col.language,
            sort_index: col.sort_index,
            is_user_added: col.is_user_added,
            metadata_json: col.metadata_json.as_deref(),
        };
        diesel::insert_into(col_dsl::chanting_collections)
            .values(&new)
            .execute(&mut conn)
            .with_context(|| format!("Failed to insert collection: {}", col.uid))?;
    }

    for chant in chants {
        let new = NewChantingChant {
            uid: &chant.uid,
            collection_uid: &chant.collection_uid,
            title: &chant.title,
            description: chant.description.as_deref(),
            sort_index: chant.sort_index,
            is_user_added: chant.is_user_added,
            metadata_json: chant.metadata_json.as_deref(),
        };
        diesel::insert_into(chant_dsl::chanting_chants)
            .values(&new)
            .execute(&mut conn)
            .with_context(|| format!("Failed to insert chant: {}", chant.uid))?;
    }

    for sec in sections {
        let new = NewChantingSection {
            uid: &sec.uid,
            chant_uid: &sec.chant_uid,
            title: &sec.title,
            content_pali: &sec.content_pali,
            sort_index: sec.sort_index,
            is_user_added: sec.is_user_added,
            metadata_json: sec.metadata_json.as_deref(),
        };
        diesel::insert_into(sec_dsl::chanting_sections)
            .values(&new)
            .execute(&mut conn)
            .with_context(|| format!("Failed to insert section: {}", sec.uid))?;
    }

    for rec in recordings {
        let new = NewChantingRecording {
            uid: &rec.uid,
            section_uid: &rec.section_uid,
            file_name: &rec.file_name,
            recording_type: &rec.recording_type,
            label: rec.label.as_deref(),
            duration_ms: rec.duration_ms,
            markers_json: rec.markers_json.as_deref(),
            volume: rec.volume,
            playback_position_ms: rec.playback_position_ms,
            waveform_json: rec.waveform_json.as_deref(),
            is_user_added: rec.is_user_added,
        };
        diesel::insert_into(rec_dsl::chanting_recordings)
            .values(&new)
            .execute(&mut conn)
            .with_context(|| format!("Failed to insert recording: {}", rec.uid))?;
    }

    info(&format!(
        "Created chanting SQLite: {} collections, {} chants, {} sections, {} recordings",
        collections.len(), chants.len(), sections.len(), recordings.len()
    ));

    Ok(())
}

/// Export selected chanting data to a zip archive.
///
/// The zip contains:
/// - `appdata-chanting.sqlite3` with the selected data
/// - `chanting-recordings/` with audio files for the selected sections
pub fn export_chanting_to_zip(
    appdata_db: &AppdataDbHandle,
    selected_collection_uids: Vec<String>,
    selected_chant_uids: Vec<String>,
    selected_section_uids: Vec<String>,
    dest_zip_path: &Path,
) -> Result<()> {
    // Query the selected rows from the live DB
    let collections = appdata_db.get_chanting_collections_by_uids(&selected_collection_uids)?;
    let chants = appdata_db.get_chanting_chants_by_uids(&selected_chant_uids)?;
    let sections = appdata_db.get_chanting_sections_by_uids(&selected_section_uids)?;
    let recordings = appdata_db.get_chanting_recordings_for_sections(&selected_section_uids)?;

    info(&format!(
        "Exporting: {} collections, {} chants, {} sections, {} recordings",
        collections.len(), chants.len(), sections.len(), recordings.len()
    ));

    // Create temp directory
    let temp_dir = tempfile::tempdir().context("Failed to create temp directory")?;
    let temp_path = temp_dir.path();

    // Create the SQLite database in the temp directory
    let sqlite_path = temp_path.join("appdata-chanting.sqlite3");
    create_chanting_sqlite(&sqlite_path, &collections, &chants, &sections, &recordings)?;

    // Copy audio files to temp dir
    let recordings_dir = get_chanting_recordings_dir();
    let temp_recordings_dir = temp_path.join("chanting-recordings");
    fs::create_dir_all(&temp_recordings_dir)
        .context("Failed to create temp recordings directory")?;

    for rec in &recordings {
        let src = recordings_dir.join(&rec.file_name);
        match src.try_exists() {
            Ok(true) => {
                let dest = temp_recordings_dir.join(&rec.file_name);
                fs::copy(&src, &dest)
                    .with_context(|| format!("Failed to copy recording: {}", rec.file_name))?;
            }
            _ => {
                info(&format!("Warning: recording file not found, skipping: {}", rec.file_name));
            }
        }
    }

    // Create the zip archive
    let zip_file = fs::File::create(dest_zip_path)
        .with_context(|| format!("Failed to create zip file: {}", dest_zip_path.display()))?;
    let mut zip_writer = zip::ZipWriter::new(zip_file);
    let options = SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    // Add the SQLite database
    zip_writer.start_file("appdata-chanting.sqlite3", options)?;
    let mut db_file = fs::File::open(&sqlite_path)?;
    let mut buf = Vec::new();
    db_file.read_to_end(&mut buf)?;
    zip_writer.write_all(&buf)?;

    // Add recording files
    for rec in &recordings {
        let src = temp_recordings_dir.join(&rec.file_name);
        match src.try_exists() {
            Ok(true) => {
                let zip_entry_name = format!("chanting-recordings/{}", rec.file_name);
                zip_writer.start_file(zip_entry_name, options)?;
                let mut rec_file = fs::File::open(&src)?;
                buf.clear();
                rec_file.read_to_end(&mut buf)?;
                zip_writer.write_all(&buf)?;
            }
            _ => {} // already warned above
        }
    }

    zip_writer.finish()?;

    info(&format!("Export zip created: {}", dest_zip_path.display()));

    // temp_dir is automatically cleaned up on drop
    Ok(())
}

/// Generate a new UID with the given prefix, matching the app's UID format:
/// `prefix-timestamp_base36-random_base36`
fn generate_uid(prefix: &str) -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let random: u64 = rand_u64();
    format!(
        "{}-{}-{}",
        prefix,
        radix_36(timestamp as u64),
        &radix_36(random)[..6.min(radix_36(random).len())]
    )
}

fn radix_36(mut n: u64) -> String {
    if n == 0 {
        return "0".to_string();
    }
    const CHARS: &[u8] = b"0123456789abcdefghijklmnopqrstuvwxyz";
    let mut result = Vec::new();
    while n > 0 {
        result.push(CHARS[(n % 36) as usize]);
        n /= 36;
    }
    result.reverse();
    String::from_utf8(result).unwrap()
}

/// Simple pseudo-random u64 using system time and address entropy.
fn rand_u64() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let t = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64;
    // Mix with a stack address for additional entropy
    let stack_var: u64 = 0;
    let addr = &stack_var as *const u64 as u64;
    t.wrapping_mul(6364136223846793005).wrapping_add(addr)
}

/// Read all chanting data from a SQLite database file.
pub fn read_chanting_from_sqlite(
    db_path: &Path,
) -> Result<(
    Vec<ChantingCollection>,
    Vec<ChantingChant>,
    Vec<ChantingSection>,
    Vec<ChantingRecording>,
)> {
    use crate::db::appdata_schema::chanting_collections::dsl as col_dsl;
    use crate::db::appdata_schema::chanting_chants::dsl as chant_dsl;
    use crate::db::appdata_schema::chanting_sections::dsl as sec_dsl;
    use crate::db::appdata_schema::chanting_recordings::dsl as rec_dsl;

    let db_url = format!("sqlite://{}", db_path.display());
    let mut conn = SqliteConnection::establish(&db_url)
        .with_context(|| format!("Failed to open import database: {}", db_path.display()))?;

    let collections = col_dsl::chanting_collections
        .select(ChantingCollection::as_select())
        .load(&mut conn)
        .context("Failed to read collections from import database")?;

    let chants = chant_dsl::chanting_chants
        .select(ChantingChant::as_select())
        .load(&mut conn)
        .context("Failed to read chants from import database")?;

    let sections = sec_dsl::chanting_sections
        .select(ChantingSection::as_select())
        .load(&mut conn)
        .context("Failed to read sections from import database")?;

    let recordings = rec_dsl::chanting_recordings
        .select(ChantingRecording::as_select())
        .load(&mut conn)
        .context("Failed to read recordings from import database")?;

    Ok((collections, chants, sections, recordings))
}

/// Remap all UIDs in the chanting data, updating foreign key references.
/// Returns the remapped data and a mapping of old filenames to new filenames for recordings.
fn remap_uids(
    collections: Vec<ChantingCollection>,
    chants: Vec<ChantingChant>,
    sections: Vec<ChantingSection>,
    recordings: Vec<ChantingRecording>,
) -> (
    Vec<ChantingCollection>,
    Vec<ChantingChant>,
    Vec<ChantingSection>,
    Vec<ChantingRecording>,
    HashMap<String, String>, // old_filename -> new_filename
) {
    let mut uid_map: HashMap<String, String> = HashMap::new();

    // Generate new UIDs for collections
    for col in &collections {
        uid_map.insert(col.uid.clone(), generate_uid("col"));
    }

    // Generate new UIDs for chants
    for chant in &chants {
        uid_map.insert(chant.uid.clone(), generate_uid("chant"));
    }

    // Generate new UIDs for sections
    for sec in &sections {
        uid_map.insert(sec.uid.clone(), generate_uid("sec"));
    }

    // Generate new UIDs for recordings
    for rec in &recordings {
        uid_map.insert(rec.uid.clone(), generate_uid("rec"));
    }

    // Remap collections
    let new_collections: Vec<ChantingCollection> = collections
        .into_iter()
        .map(|mut col| {
            col.uid = uid_map[&col.uid].clone();
            col.is_user_added = true;
            col
        })
        .collect();

    // Remap chants (uid + collection_uid FK)
    let new_chants: Vec<ChantingChant> = chants
        .into_iter()
        .map(|mut chant| {
            chant.uid = uid_map[&chant.uid].clone();
            chant.collection_uid = uid_map[&chant.collection_uid].clone();
            chant.is_user_added = true;
            chant
        })
        .collect();

    // Remap sections (uid + chant_uid FK)
    let new_sections: Vec<ChantingSection> = sections
        .into_iter()
        .map(|mut sec| {
            sec.uid = uid_map[&sec.uid].clone();
            sec.chant_uid = uid_map[&sec.chant_uid].clone();
            sec.is_user_added = true;
            sec
        })
        .collect();

    // Remap recordings (uid + section_uid FK + file_name)
    let mut filename_map: HashMap<String, String> = HashMap::new();

    let new_recordings: Vec<ChantingRecording> = recordings
        .into_iter()
        .map(|mut rec| {
            let old_filename = rec.file_name.clone();
            let new_rec_uid = uid_map[&rec.uid].clone();
            let new_section_uid = uid_map[&rec.section_uid].clone();

            // Generate new filename: replace old section UID prefix if present,
            // otherwise use new_section_uid + timestamp
            let new_filename = if let Some(rest) = old_filename.strip_prefix(&rec.section_uid) {
                // Original format: old_section_uid + rest (e.g., "_user_12345.ogg")
                format!("{}{}", new_section_uid, rest)
            } else {
                // Fallback: use new section UID + "_" + new recording UID + extension
                let ext = old_filename
                    .rsplit('.')
                    .next()
                    .unwrap_or("ogg");
                format!("{}_{}.{}", new_section_uid, new_rec_uid, ext)
            };

            filename_map.insert(old_filename, new_filename.clone());

            rec.uid = new_rec_uid;
            rec.section_uid = new_section_uid;
            rec.file_name = new_filename;
            // Reset playback position and waveform cache for imported recordings
            rec.playback_position_ms = 0;
            rec.waveform_json = None;
            rec.is_user_added = true;
            rec
        })
        .collect();

    (new_collections, new_chants, new_sections, new_recordings, filename_map)
}

/// Import chanting data from a zip archive.
///
/// Extracts the zip, reads chanting data from the embedded SQLite database,
/// generates new UIDs for all records, inserts into the live database,
/// and copies audio files with new filenames.
pub fn import_chanting_from_zip(
    appdata_db: &AppdataDbHandle,
    zip_path: &Path,
    recordings_dir: &Path,
) -> Result<ImportResult> {
    // Extract zip to temp directory
    let temp_dir = tempfile::tempdir().context("Failed to create temp directory")?;
    let temp_path = temp_dir.path();

    let zip_file = fs::File::open(zip_path)
        .with_context(|| format!("Failed to open zip file: {}", zip_path.display()))?;
    let mut archive = zip::ZipArchive::new(zip_file)
        .with_context(|| format!("Failed to read zip archive: {}", zip_path.display()))?;

    archive.extract(temp_path)
        .with_context(|| format!("Failed to extract zip archive: {}", zip_path.display()))?;

    // Validate the archive contains the SQLite database
    let sqlite_path = temp_path.join("appdata-chanting.sqlite3");
    match sqlite_path.try_exists() {
        Ok(true) => {}
        _ => {
            return Err(anyhow::anyhow!(
                "Invalid chanting archive: missing appdata-chanting.sqlite3"
            ));
        }
    }

    // Read all data from the embedded database
    let (collections, chants, sections, recordings) =
        read_chanting_from_sqlite(&sqlite_path)?;

    info(&format!(
        "Import archive contains: {} collections, {} chants, {} sections, {} recordings",
        collections.len(), chants.len(), sections.len(), recordings.len()
    ));

    // Remap UIDs
    let (new_collections, new_chants, new_sections, new_recordings, filename_map) =
        remap_uids(collections, chants, sections, recordings);

    // Insert into the live database
    for col in &new_collections {
        let data = ChantingCollectionJson {
            uid: col.uid.clone(),
            title: col.title.clone(),
            description: col.description.clone(),
            language: col.language.clone(),
            sort_index: col.sort_index,
            is_user_added: true,
            metadata_json: col.metadata_json.clone(),
            chants: Vec::new(),
        };
        appdata_db.create_chanting_collection(&data)
            .with_context(|| format!("Failed to import collection: {}", col.uid))?;
    }

    for chant in &new_chants {
        let data = ChantingChantJson {
            uid: chant.uid.clone(),
            collection_uid: chant.collection_uid.clone(),
            title: chant.title.clone(),
            description: chant.description.clone(),
            sort_index: chant.sort_index,
            is_user_added: true,
            metadata_json: chant.metadata_json.clone(),
            sections: Vec::new(),
        };
        appdata_db.create_chanting_chant(&data)
            .with_context(|| format!("Failed to import chant: {}", chant.uid))?;
    }

    for sec in &new_sections {
        let data = ChantingSectionJson {
            uid: sec.uid.clone(),
            chant_uid: sec.chant_uid.clone(),
            title: sec.title.clone(),
            content_pali: sec.content_pali.clone(),
            sort_index: sec.sort_index,
            is_user_added: true,
            metadata_json: sec.metadata_json.clone(),
            recordings: Vec::new(),
        };
        appdata_db.create_chanting_section(&data)
            .with_context(|| format!("Failed to import section: {}", sec.uid))?;
    }

    for rec in &new_recordings {
        let data = ChantingRecordingJson {
            uid: rec.uid.clone(),
            section_uid: rec.section_uid.clone(),
            file_name: rec.file_name.clone(),
            recording_type: rec.recording_type.clone(),
            label: rec.label.clone(),
            duration_ms: rec.duration_ms,
            markers_json: rec.markers_json.clone(),
            volume: rec.volume,
            playback_position_ms: rec.playback_position_ms,
            waveform_json: rec.waveform_json.clone(),
            is_user_added: true,
        };
        appdata_db.create_chanting_recording(&data)
            .with_context(|| format!("Failed to import recording: {}", rec.uid))?;
    }

    // Copy audio files with new filenames
    let extracted_recordings_dir = temp_path.join("chanting-recordings");
    if let Ok(true) = extracted_recordings_dir.try_exists() {
        // Ensure target recordings directory exists
        fs::create_dir_all(recordings_dir)
            .context("Failed to create recordings directory")?;

        for (old_filename, new_filename) in &filename_map {
            let src = extracted_recordings_dir.join(old_filename);
            match src.try_exists() {
                Ok(true) => {
                    let dest = recordings_dir.join(new_filename);
                    fs::copy(&src, &dest).with_context(|| {
                        format!("Failed to copy recording: {} -> {}", old_filename, new_filename)
                    })?;
                }
                _ => {
                    info(&format!(
                        "Warning: audio file not found in archive, skipping: {}",
                        old_filename
                    ));
                }
            }
        }
    }

    let result = ImportResult {
        collections: new_collections.len(),
        chants: new_chants.len(),
        sections: new_sections.len(),
        recordings: new_recordings.len(),
    };

    info(&format!(
        "Import complete: {} collections, {} chants, {} sections, {} recordings",
        result.collections, result.chants, result.sections, result.recordings
    ));

    // temp_dir is automatically cleaned up on drop
    Ok(result)
}

pub struct ImportResult {
    pub collections: usize,
    pub chants: usize,
    pub sections: usize,
    pub recordings: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn make_test_data() -> (
        Vec<ChantingCollection>,
        Vec<ChantingChant>,
        Vec<ChantingSection>,
        Vec<ChantingRecording>,
    ) {
        let collections = vec![ChantingCollection {
            id: 0,
            uid: "col-test-1".to_string(),
            title: "Test Collection".to_string(),
            description: Some("A test collection".to_string()),
            language: "pali".to_string(),
            sort_index: 0,
            is_user_added: true,
            metadata_json: None,
        }];

        let chants = vec![ChantingChant {
            id: 0,
            uid: "chant-test-1".to_string(),
            collection_uid: "col-test-1".to_string(),
            title: "Test Chant".to_string(),
            description: None,
            sort_index: 0,
            is_user_added: true,
            metadata_json: None,
        }];

        let sections = vec![ChantingSection {
            id: 0,
            uid: "sec-test-1".to_string(),
            chant_uid: "chant-test-1".to_string(),
            title: "Test Section".to_string(),
            content_pali: "Namo tassa".to_string(),
            sort_index: 0,
            is_user_added: true,
            metadata_json: None,
        }];

        let recordings = vec![ChantingRecording {
            id: 0,
            uid: "rec-test-1".to_string(),
            section_uid: "sec-test-1".to_string(),
            file_name: "sec-test-1_12345.ogg".to_string(),
            recording_type: "user".to_string(),
            label: Some("My recording".to_string()),
            duration_ms: 5000,
            markers_json: None,
            volume: 1.0,
            playback_position_ms: 0,
            waveform_json: None,
            is_user_added: true,
        }];

        (collections, chants, sections, recordings)
    }

    fn make_multi_test_data() -> (
        Vec<ChantingCollection>,
        Vec<ChantingChant>,
        Vec<ChantingSection>,
        Vec<ChantingRecording>,
    ) {
        let collections = vec![ChantingCollection {
            id: 0,
            uid: "col-orig-1".to_string(),
            title: "Original Collection".to_string(),
            description: None,
            language: "pali".to_string(),
            sort_index: 0,
            is_user_added: false,
            metadata_json: None,
        }];

        let chants = vec![
            ChantingChant {
                id: 0,
                uid: "chant-orig-1".to_string(),
                collection_uid: "col-orig-1".to_string(),
                title: "First Chant".to_string(),
                description: None,
                sort_index: 0,
                is_user_added: false,
                metadata_json: None,
            },
            ChantingChant {
                id: 0,
                uid: "chant-orig-2".to_string(),
                collection_uid: "col-orig-1".to_string(),
                title: "Second Chant".to_string(),
                description: Some("Desc".to_string()),
                sort_index: 1,
                is_user_added: false,
                metadata_json: None,
            },
        ];

        let sections = vec![
            ChantingSection {
                id: 0,
                uid: "sec-orig-1".to_string(),
                chant_uid: "chant-orig-1".to_string(),
                title: "Section A".to_string(),
                content_pali: "Pali A".to_string(),
                sort_index: 0,
                is_user_added: false,
                metadata_json: None,
            },
            ChantingSection {
                id: 0,
                uid: "sec-orig-2".to_string(),
                chant_uid: "chant-orig-2".to_string(),
                title: "Section B".to_string(),
                content_pali: "Pali B".to_string(),
                sort_index: 0,
                is_user_added: false,
                metadata_json: None,
            },
        ];

        let recordings = vec![
            ChantingRecording {
                id: 0,
                uid: "rec-orig-1".to_string(),
                section_uid: "sec-orig-1".to_string(),
                file_name: "sec-orig-1_user_111.ogg".to_string(),
                recording_type: "user".to_string(),
                label: None,
                duration_ms: 3000,
                markers_json: None,
                volume: 0.8,
                playback_position_ms: 100,
                waveform_json: Some("cached".to_string()),
                is_user_added: true,
            },
            ChantingRecording {
                id: 0,
                uid: "rec-orig-2".to_string(),
                section_uid: "sec-orig-2".to_string(),
                file_name: "sec-orig-2_user_222.ogg".to_string(),
                recording_type: "guide".to_string(),
                label: Some("Guide".to_string()),
                duration_ms: 10000,
                markers_json: Some(r#"[{"ms":0}]"#.to_string()),
                volume: 1.0,
                playback_position_ms: 500,
                waveform_json: None,
                is_user_added: false,
            },
        ];

        (collections, chants, sections, recordings)
    }

    #[test]
    fn test_create_chanting_sqlite_roundtrip() {
        use crate::db::appdata_schema::chanting_collections::dsl as col_dsl;
        use crate::db::appdata_schema::chanting_chants::dsl as chant_dsl;
        use crate::db::appdata_schema::chanting_sections::dsl as sec_dsl;
        use crate::db::appdata_schema::chanting_recordings::dsl as rec_dsl;

        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test-chanting.sqlite3");
        let (collections, chants, sections, recordings) = make_test_data();

        // Write
        create_chanting_sqlite(&db_path, &collections, &chants, &sections, &recordings).unwrap();

        // Read back
        let db_url = format!("sqlite://{}", db_path.display());
        let mut conn = SqliteConnection::establish(&db_url).unwrap();

        let read_collections: Vec<ChantingCollection> = col_dsl::chanting_collections
            .select(ChantingCollection::as_select())
            .load(&mut conn)
            .unwrap();
        assert_eq!(read_collections.len(), 1);
        assert_eq!(read_collections[0].uid, "col-test-1");
        assert_eq!(read_collections[0].title, "Test Collection");

        let read_chants: Vec<ChantingChant> = chant_dsl::chanting_chants
            .select(ChantingChant::as_select())
            .load(&mut conn)
            .unwrap();
        assert_eq!(read_chants.len(), 1);
        assert_eq!(read_chants[0].uid, "chant-test-1");
        assert_eq!(read_chants[0].collection_uid, "col-test-1");

        let read_sections: Vec<ChantingSection> = sec_dsl::chanting_sections
            .select(ChantingSection::as_select())
            .load(&mut conn)
            .unwrap();
        assert_eq!(read_sections.len(), 1);
        assert_eq!(read_sections[0].uid, "sec-test-1");
        assert_eq!(read_sections[0].content_pali, "Namo tassa");

        let read_recordings: Vec<ChantingRecording> = rec_dsl::chanting_recordings
            .select(ChantingRecording::as_select())
            .load(&mut conn)
            .unwrap();
        assert_eq!(read_recordings.len(), 1);
        assert_eq!(read_recordings[0].uid, "rec-test-1");
        assert_eq!(read_recordings[0].file_name, "sec-test-1_12345.ogg");
        assert_eq!(read_recordings[0].duration_ms, 5000);
        assert_eq!(read_recordings[0].volume, 1.0);
    }

    #[test]
    fn test_create_chanting_sqlite_empty_data() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test-empty.sqlite3");

        create_chanting_sqlite(&db_path, &[], &[], &[], &[]).unwrap();

        let db_url = format!("sqlite://{}", db_path.display());
        let mut conn = SqliteConnection::establish(&db_url).unwrap();

        use crate::db::appdata_schema::chanting_collections::dsl as col_dsl;
        let count: i64 = col_dsl::chanting_collections
            .count()
            .get_result(&mut conn)
            .unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn test_create_chanting_sqlite_multiple_records() {
        use crate::db::appdata_schema::chanting_sections::dsl as sec_dsl;
        use crate::db::appdata_schema::chanting_recordings::dsl as rec_dsl;

        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test-multi.sqlite3");

        let collections = vec![ChantingCollection {
            id: 0,
            uid: "col-1".to_string(),
            title: "Collection 1".to_string(),
            description: None,
            language: "pali".to_string(),
            sort_index: 0,
            is_user_added: false,
            metadata_json: None,
        }];

        let chants = vec![ChantingChant {
            id: 0,
            uid: "chant-1".to_string(),
            collection_uid: "col-1".to_string(),
            title: "Chant 1".to_string(),
            description: None,
            sort_index: 0,
            is_user_added: false,
            metadata_json: None,
        }];

        let sections = vec![
            ChantingSection {
                id: 0,
                uid: "sec-1".to_string(),
                chant_uid: "chant-1".to_string(),
                title: "Section 1".to_string(),
                content_pali: "Content 1".to_string(),
                sort_index: 0,
                is_user_added: false,
                metadata_json: None,
            },
            ChantingSection {
                id: 0,
                uid: "sec-2".to_string(),
                chant_uid: "chant-1".to_string(),
                title: "Section 2".to_string(),
                content_pali: "Content 2".to_string(),
                sort_index: 1,
                is_user_added: false,
                metadata_json: None,
            },
        ];

        let recordings = vec![
            ChantingRecording {
                id: 0,
                uid: "rec-1".to_string(),
                section_uid: "sec-1".to_string(),
                file_name: "sec-1_111.ogg".to_string(),
                recording_type: "user".to_string(),
                label: None,
                duration_ms: 3000,
                markers_json: None,
                volume: 0.8,
                playback_position_ms: 0,
                waveform_json: None,
                is_user_added: true,
            },
            ChantingRecording {
                id: 0,
                uid: "rec-2".to_string(),
                section_uid: "sec-2".to_string(),
                file_name: "sec-2_222.ogg".to_string(),
                recording_type: "guide".to_string(),
                label: Some("Guide".to_string()),
                duration_ms: 10000,
                markers_json: Some(r#"[{"ms":0}]"#.to_string()),
                volume: 1.0,
                playback_position_ms: 500,
                waveform_json: None,
                is_user_added: false,
            },
        ];

        create_chanting_sqlite(&db_path, &collections, &chants, &sections, &recordings).unwrap();

        let db_url = format!("sqlite://{}", db_path.display());
        let mut conn = SqliteConnection::establish(&db_url).unwrap();

        let read_sections: Vec<ChantingSection> = sec_dsl::chanting_sections
            .select(ChantingSection::as_select())
            .order(sec_dsl::sort_index.asc())
            .load(&mut conn)
            .unwrap();
        assert_eq!(read_sections.len(), 2);
        assert_eq!(read_sections[0].uid, "sec-1");
        assert_eq!(read_sections[1].uid, "sec-2");

        let read_recordings: Vec<ChantingRecording> = rec_dsl::chanting_recordings
            .select(ChantingRecording::as_select())
            .load(&mut conn)
            .unwrap();
        assert_eq!(read_recordings.len(), 2);

        let rec2 = read_recordings.iter().find(|r| r.uid == "rec-2").unwrap();
        assert_eq!(rec2.recording_type, "guide");
        assert_eq!(rec2.label, Some("Guide".to_string()));
        assert_eq!(rec2.volume, 1.0);
        assert_eq!(rec2.playback_position_ms, 500);
    }

    #[test]
    fn test_remap_uids_generates_new_uids() {
        let (collections, chants, sections, recordings) = make_test_data();

        let (new_cols, new_chants, new_secs, new_recs, filename_map) =
            remap_uids(collections, chants, sections, recordings);

        // All UIDs should be different from originals
        assert_ne!(new_cols[0].uid, "col-test-1");
        assert_ne!(new_chants[0].uid, "chant-test-1");
        assert_ne!(new_secs[0].uid, "sec-test-1");
        assert_ne!(new_recs[0].uid, "rec-test-1");

        // UIDs should start with correct prefixes
        assert!(new_cols[0].uid.starts_with("col-"));
        assert!(new_chants[0].uid.starts_with("chant-"));
        assert!(new_secs[0].uid.starts_with("sec-"));
        assert!(new_recs[0].uid.starts_with("rec-"));

        // All should be marked as user_added
        assert!(new_cols[0].is_user_added);
        assert!(new_chants[0].is_user_added);
        assert!(new_secs[0].is_user_added);
    }

    #[test]
    fn test_remap_uids_foreign_keys_consistent() {
        let (collections, chants, sections, recordings) = make_multi_test_data();

        let (new_cols, new_chants, new_secs, new_recs, _) =
            remap_uids(collections, chants, sections, recordings);

        // Chants should reference the new collection UID
        let col_uid = &new_cols[0].uid;
        assert_eq!(new_chants[0].collection_uid, *col_uid);
        assert_eq!(new_chants[1].collection_uid, *col_uid);

        // Sections should reference the correct new chant UID
        let chant1_uid = &new_chants[0].uid;
        let chant2_uid = &new_chants[1].uid;
        assert_eq!(new_secs[0].chant_uid, *chant1_uid);
        assert_eq!(new_secs[1].chant_uid, *chant2_uid);

        // Recordings should reference the correct new section UID
        let sec1_uid = &new_secs[0].uid;
        let sec2_uid = &new_secs[1].uid;
        assert_eq!(new_recs[0].section_uid, *sec1_uid);
        assert_eq!(new_recs[1].section_uid, *sec2_uid);
    }

    #[test]
    fn test_remap_uids_filename_updated() {
        let (collections, chants, sections, recordings) = make_test_data();

        let (_new_cols, _new_chants, new_secs, new_recs, filename_map) =
            remap_uids(collections, chants, sections, recordings);

        // Old filename should be in the map
        assert!(filename_map.contains_key("sec-test-1_12345.ogg"));

        // New filename should start with the new section UID
        let new_filename = &new_recs[0].file_name;
        assert!(new_filename.starts_with(&new_secs[0].uid));

        // New filename should preserve the suffix after the old section UID
        assert!(new_filename.ends_with("_12345.ogg"));
    }

    #[test]
    fn test_remap_uids_resets_playback_and_waveform() {
        let (collections, chants, sections, recordings) = make_multi_test_data();

        // rec-orig-1 has playback_position_ms=100 and waveform_json=Some("cached")
        let (_new_cols, _new_chants, _new_secs, new_recs, _) =
            remap_uids(collections, chants, sections, recordings);

        for rec in &new_recs {
            assert_eq!(rec.playback_position_ms, 0);
            assert_eq!(rec.waveform_json, None);
        }
    }

    #[test]
    fn test_read_chanting_from_sqlite_roundtrip() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test-read.sqlite3");
        let (collections, chants, sections, recordings) = make_multi_test_data();

        create_chanting_sqlite(&db_path, &collections, &chants, &sections, &recordings).unwrap();

        let (read_cols, read_chants, read_secs, read_recs) =
            read_chanting_from_sqlite(&db_path).unwrap();

        assert_eq!(read_cols.len(), 1);
        assert_eq!(read_chants.len(), 2);
        assert_eq!(read_secs.len(), 2);
        assert_eq!(read_recs.len(), 2);

        assert_eq!(read_cols[0].title, "Original Collection");
        assert_eq!(read_chants[0].title, "First Chant");
        assert_eq!(read_recs[1].label, Some("Guide".to_string()));
    }

    #[test]
    fn test_radix_36() {
        assert_eq!(radix_36(0), "0");
        assert_eq!(radix_36(35), "z");
        assert_eq!(radix_36(36), "10");
        assert_eq!(radix_36(100), "2s");
    }

    #[test]
    fn test_generate_uid_format() {
        let uid = generate_uid("col");
        assert!(uid.starts_with("col-"), "UID should start with prefix: {}", uid);
        let parts: Vec<&str> = uid.splitn(3, '-').collect();
        assert_eq!(parts.len(), 3, "UID should have 3 parts: {}", uid);
    }
}
