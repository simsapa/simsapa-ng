use std::fs;
use std::path::Path;
use anyhow::{Result, Context};

use crate::import_stardict_dictionary;

pub fn dpd_bootstrap(bootstrap_assets_dir: &Path, assets_dir: &Path) -> Result<()> {
    // Import DPD stardict
    let dpd_stardict_path = bootstrap_assets_dir.join("dpd-db-for-bootstrap/current/dpd/");
    import_stardict_dictionary("dpd", &dpd_stardict_path, None)
        .map_err(|e| anyhow::anyhow!("Failed to import DPD Stardict: {}", e))?;

    // This requires the DPD dictionary ID already present in dictionaries.sqlite3
    dpd_migrate(bootstrap_assets_dir, assets_dir)?;

    Ok(())
}

pub fn dpd_migrate(bootstrap_assets_dir: &Path, assets_dir: &Path) -> Result<()> {
    println!("=== dpd_migrate() ===");

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

    println!("Copied dpd.db to assets directory");

    // Call the import_migrate_dpd function
    let dpd_input_path = dest_db_path;
    let dpd_output_path = assets_dir.join("dpd.sqlite3");

    simsapa_backend::db::dpd::import_migrate_dpd(&dpd_input_path, Some(dpd_output_path))
        .map_err(|e| anyhow::anyhow!("Failed to migrate DPD database: {}", e))?;

    println!("Successfully migrated DPD database");
    Ok(())
}
