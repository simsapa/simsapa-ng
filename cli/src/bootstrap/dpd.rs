use std::fs;
use std::path::{Path, PathBuf};
use anyhow::{Result, Context};

use simsapa_backend::logger;
use simsapa_backend::helpers::run_fts5_indexes_sql_script;
use simsapa_backend::dictionary_manager_core::import_user_zip;
use simsapa_backend::stardict_parse::StardictImportProgress;

use crate::import_stardict_dictionary;

pub fn dpd_bootstrap(bootstrap_assets_dir: &Path, assets_dir: &Path, limit: Option<i32>) -> Result<()> {
    // Import DPD stardict
    let dpd_stardict_path = bootstrap_assets_dir.join("dpd-db-for-bootstrap/current/dpd/");
    let limit_usize = limit.map(|l| l as usize);
    import_stardict_dictionary("dpd", &dpd_stardict_path, limit_usize)
        .map_err(|e| anyhow::anyhow!("Failed to import DPD Stardict: {}", e))?;

    // Create FTS5 indexes for dictionaries database
    create_dictionaries_fts5_indexes(assets_dir)?;

    // Migrate DPD. This requires the DPD dictionary ID already present in dictionaries.sqlite3
    // `import_migrate_dpd` internally populates bold_definitions
    // derived columns (uid, commentary_plain) before creating indexes.
    dpd_migrate(bootstrap_assets_dir, assets_dir)?;

    // Import the DPPN StarDict zip as an English dictionary.
    let dppn_zip_path = bootstrap_assets_dir.join("stardict-imports/dppn-gd.zip");
    import_stardict_zip_bootstrap(&dppn_zip_path, "dppn", "en")?;

    Ok(())
}

fn import_stardict_zip_bootstrap(zip_path: &Path, label: &str, lang: &str) -> Result<()> {
    logger::info(&format!(
        "=== Import StarDict zip: {} (label: {}, lang: {}) ===",
        zip_path.display(), label, lang
    ));

    match zip_path.try_exists() {
        Ok(true) => {}
        Ok(false) => return Err(anyhow::anyhow!("Zip file not found: {}", zip_path.display())),
        Err(e) => return Err(anyhow::anyhow!("Cannot access zip {}: {}", zip_path.display(), e)),
    }

    import_user_zip(zip_path, label, lang, &|p| {
        match p {
            StardictImportProgress::Extracting => logger::info("  extracting..."),
            StardictImportProgress::Parsing => logger::info("  parsing..."),
            StardictImportProgress::InsertingWords { done, total } => {
                if total > 0 && (done == 0 || done == total || done % 1000 == 0) {
                    logger::info(&format!("  inserting words: {}/{}", done, total));
                }
            }
            StardictImportProgress::Done => logger::info("  done."),
            StardictImportProgress::Failed { msg } => logger::error(&format!("  failed: {}", msg)),
        }
    })
    .map_err(|e| anyhow::anyhow!("Failed to import StarDict zip {}: {}", zip_path.display(), e))?;

    Ok(())
}

pub fn dpd_migrate(bootstrap_assets_dir: &Path, assets_dir: &Path) -> Result<()> {
    logger::info("=== dpd_migrate() ===");

    let source_db_path = bootstrap_assets_dir
        .join("dpd-db-for-bootstrap/current/dpd.db");
    let dest_db_path = assets_dir.join("dpd.db");

    // Check if source database exists
    if !source_db_path.exists() {
        return Err(anyhow::anyhow!(
            "Source DPD database not found at: {}",
            source_db_path.display()
        ));
    }

    // Copy the database file
    fs::copy(&source_db_path, &dest_db_path)
        .with_context(|| format!(
            "Failed to copy DPD database from {} to {}",
            source_db_path.display(),
            dest_db_path.display()
        ))?;

    logger::info("Copied dpd.db to assets directory");

    // Call the import_migrate_dpd function
    let dpd_input_path = dest_db_path;
    let dpd_output_path = assets_dir.join("dpd.sqlite3");

    simsapa_backend::db::dpd::import_migrate_dpd(&dpd_input_path, Some(dpd_output_path))
        .map_err(|e| anyhow::anyhow!("Failed to migrate DPD database: {}", e))?;

    logger::info("Successfully migrated DPD database");
    Ok(())
}

pub fn create_dictionaries_fts5_indexes(assets_dir: &Path) -> Result<()> {
    logger::info("=== create_dictionaries_fts5_indexes() ===");
    let dict_db_path = assets_dir.join("dictionaries.sqlite3");
    let sql_script_path = PathBuf::from("../scripts/dictionaries-fts5-indexes.sql");
    run_fts5_indexes_sql_script(&dict_db_path, &sql_script_path)
}
