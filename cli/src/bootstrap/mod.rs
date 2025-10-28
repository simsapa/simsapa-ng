pub mod helpers;
pub mod appdata;
pub mod suttacentral;
pub mod dhammatalks_org;
pub mod dhammapada_munindo;
pub mod dhammapada_tipitaka;
pub mod nyanadipa;
pub mod buddha_ujja;
pub mod dpd;
pub mod completions;

use anyhow::{Result, Context};
use chrono::{DateTime, Local};

use std::path::{Path, PathBuf};
use std::{fs, env};

use diesel::prelude::*;
use diesel_migrations::MigrationHarness;

use simsapa_backend::db::APPDATA_MIGRATIONS;
use simsapa_backend::{init_app_data, get_create_simsapa_dir, get_create_simsapa_app_assets_path, logger};

pub use helpers::SuttaData;
pub use appdata::AppdataBootstrap;
pub use dhammatalks_org::DhammatalksSuttaImporter;
pub use dhammapada_munindo::DhammapadaMunindoImporter;
pub use dhammapada_tipitaka::DhammapadaTipitakaImporter;
pub use nyanadipa::NyanadipaImporter;
pub use suttacentral::SuttaCentralImporter;
pub use buddha_ujja::BuddhaUjjaImporter;

pub trait SuttaImporter {
    fn import(&mut self, conn: &mut SqliteConnection) -> Result<()>;
}

pub fn create_database_connection(db_path: &Path) -> Result<SqliteConnection> {
    let db_url = db_path.to_str()
        .ok_or_else(|| anyhow::anyhow!("Invalid database path"))?;

    let conn = SqliteConnection::establish(db_url)?;
    Ok(conn)
}

pub fn run_migrations(conn: &mut SqliteConnection) -> Result<()> {
    conn.run_pending_migrations(APPDATA_MIGRATIONS)
        .map_err(|e| anyhow::anyhow!("Failed to execute pending database migrations: {}", e))?;
    Ok(())
}

pub fn ensure_directory_exists(path: &Path) -> Result<()> {
    if !path.exists() {
        fs::create_dir_all(path)?;
    }
    Ok(())
}

/// Main bootstrap function - orchestrates the entire bootstrap process
pub fn bootstrap(write_new_dotenv: bool, skip_dpd: bool) -> Result<()> {
    logger::info("=== bootstrap() ===");
    if skip_dpd {
        logger::info("--skip-dpd flag set: DPD initialization and bootstrap will be skipped");
    }

    let start_time: DateTime<Local> = Local::now();
    let iso_date = start_time.format("%Y-%m-%d").to_string();

    let bootstrap_limit: Option<i32> = match env::var("BOOTSTRAP_LIMIT") {
        Ok(s) if !s.is_empty() => s.parse().ok(),
        _ => None,
    };

    // Running the binary with 'cargo run', the PWD is simsapa-ng/cli/.
    // The asset folders are one level above simsapa-ng/.
    let bootstrap_assets_dir = PathBuf::from("../../bootstrap-assets-resources");

    if !bootstrap_assets_dir.exists() {
        anyhow::bail!(
            "Bootstrap assets directory not found: {}",
            bootstrap_assets_dir.display()
        );
    }

    let release_dir = PathBuf::from(format!("../../releases/{}-dev/databases/", iso_date));
    let dist_dir = bootstrap_assets_dir.join("dist");

    // During bootstrap, don't touch the user's Simsapa dir (~/.local/share/simsapa-ng)
    // Create files in the dist/ folder instead.
    // Setting the env var here to override any previous value.
    unsafe { env::set_var("SIMSAPA_DIR", &dist_dir.join("simsapa-ng")); }

    let simsapa_dir = get_create_simsapa_dir()
        .map_err(|e| anyhow::anyhow!("Failed to get simsapa directory: {}", e))?;
    let assets_dir = get_create_simsapa_app_assets_path();

    logger::info(&format!("Bootstrap simsapa_dir: {:?}", simsapa_dir));

    let bootstrap_limit_str = match bootstrap_limit {
        Some(n) => n.to_string(),
        None => String::new(),
    };

    let dot_env_content = format!(
        r#"BOOTSTRAP_LIMIT={}
SIMSAPA_DIR={}
BOOTSTRAP_ASSETS_DIR={}
USE_TEST_DATA=false
DISABLE_LOG=false
ENABLE_PRINT_LOG=true
START_NEW_LOG=false
ENABLE_WIP_FEATURES=false
SAVE_STATS=false
RELEASE_CHANNEL=development
"#,
        bootstrap_limit_str,
        simsapa_dir.display(),
        bootstrap_assets_dir.display()
    );

    // Only write .env file if it doesn't exist or if explicitly requested
    let dot_env_path = Path::new(".env");
    if write_new_dotenv || !dot_env_path.exists() {
        fs::write(dot_env_path, dot_env_content)
            .context("Failed to write .env file")?;
        println!("Created .env file");
    } else {
        println!("Skipping .env file creation (already exists). Use --write-new-dotenv to overwrite.");
    }

    clean_and_create_folders(&simsapa_dir, &assets_dir, &release_dir, &dist_dir)?;

    logger::info("=== Create appdata.sqlite3 ===");

    // Create appdata.sqlite3 in the app-assets directory
    let appdata_db_path = assets_dir.join("appdata.sqlite3");
    let mut appdata_bootstrap = AppdataBootstrap::new(appdata_db_path.clone());

    // Create the appdata database, populated later.
    appdata_bootstrap.run()?;

    logger::info("=== Importing suttas from various sources ===");

    // Get database connection for sutta imports
    let mut conn = create_database_connection(&appdata_db_path)?;

    // Import suttas from SuttaCentral
    {
        let sc_data_dir = bootstrap_assets_dir.join("sc-data");
        if sc_data_dir.exists() {
            logger::info("Importing suttas from SuttaCentral");
            for lang in ["en", "pli"] {
                let mut importer = SuttaCentralImporter::new(sc_data_dir.clone(), lang);
                importer.import(&mut conn)?;
            }
        } else {
            logger::warn("SuttaCentral data directory not found, skipping");
        }
    }

    // Import from Dhammatalks.org
    {
        let dhammatalks_path = bootstrap_assets_dir.join("dhammatalks-org/www.dhammatalks.org/suttas");
        if dhammatalks_path.exists() {
            logger::info("Importing suttas from dhammatalks.org");
            let mut importer = DhammatalksSuttaImporter::new(dhammatalks_path);
            importer.import(&mut conn)?;
        } else {
            logger::warn("Dhammatalks.org resource path not found, skipping");
        }
    }

    // Import Dhammapada from Tipitaka.net (Daw Mya Tin translation)
    // Uses exported database from dhammapada_tipitaka_net_export command
    {
        let exported_db_path = bootstrap_assets_dir.join("dhammapada-tipitaka-net/dhammapada-tipitaka-net.sqlite3");
        if exported_db_path.exists() {
            logger::info("Importing suttas from dhammapada-tipitaka-net (exported DB)");
            let mut importer = DhammapadaTipitakaImporter::new(exported_db_path);
            importer.import(&mut conn)?;
        } else {
            logger::warn(&format!("Dhammapada Tipitaka.net exported database not found: {:?}", exported_db_path));
            logger::warn("Run: simsapa_cli dhammapada-tipitaka-net-export <legacy_db> <output_db>");
        }
    }

    // Import Nyanadipa translations (Sutta Nipata selections)
    {
        let nyanadipa_path = bootstrap_assets_dir.join("nyanadipa-translations");
        if nyanadipa_path.exists() {
            logger::info("Importing suttas from nyanadipa-translations");
            let mut importer = NyanadipaImporter::new(nyanadipa_path);
            importer.import(&mut conn)?;
        } else {
            logger::warn("Nyanadipa translations resource path not found, skipping");
        }
    }

    // Import Ajahn Munindo's Dhammapada
    {
        let dhammapada_munindo_path = bootstrap_assets_dir.join("dhammapada-munindo");
        if dhammapada_munindo_path.exists() {
            logger::info("Importing suttas from dhammapada-munindo");
            let mut importer = DhammapadaMunindoImporter::new(dhammapada_munindo_path);
            importer.import(&mut conn)?;
        } else {
            logger::warn("Dhammapada Munindo resource path not found, skipping");
        }
    }

    // Drop connection to close database before further operations
    drop(conn);

    logger::info("=== Create appdata.tar.bz2 ===");

    create_database_archive(&appdata_db_path, &release_dir)?;

    // Digital Pāli Dictionary
    if !skip_dpd {
        init_app_data();
        dpd::dpd_bootstrap(&bootstrap_assets_dir, &assets_dir)?;
        logger::info("=== Create dpd.tar.bz2 ===");
        let dpd_db_path = assets_dir.join("dpd.sqlite3");
        create_database_archive(&dpd_db_path, &release_dir)?;
    } else {
        logger::info("Skipping DPD initialization and bootstrap");
    }

    logger::info("=== Bootstrap Hungarian from Buddha Ujja ===");

    // Import Hungarian translations from Buddha Ujja
    {
        let bu_db_path = bootstrap_assets_dir.join("buddha-ujja-sql/bu.sqlite3");
        if bu_db_path.exists() {
            logger::info("Importing Hungarian suttas from Buddha Ujja");

            let lang = "hu";
            let lang_db_path = assets_dir.join(format!("suttas_lang_{}.sqlite3", lang));

            // Create the language-specific database with appdata schema
            let mut lang_conn = create_database_connection(&lang_db_path)?;
            run_migrations(&mut lang_conn)?;

            // Import Hungarian suttas directly into the language database
            let mut importer = BuddhaUjjaImporter::new(bu_db_path);
            importer.import(&mut lang_conn)?;
            drop(lang_conn);

            // Create archive and move to release directory
            create_database_archive(&lang_db_path, &release_dir)?;
        } else {
            logger::warn(&format!("Buddha Ujja database not found: {:?}", bu_db_path));
            logger::warn("Skipping Hungarian sutta import");
        }
    }

    logger::info("=== Bootstrap process completed successfully ===");
    logger::info(&format!("Output database: {:?}", appdata_db_path));

    Ok(())
}

pub fn clean_and_create_folders(
    simsapa_dir: &Path,
    assets_dir: &Path,
    release_dir: &Path,
    dist_dir: &Path
) -> Result<()> {
    println!("=== clean_and_create_folders() ===");

    // Clean and create directories
    for dir in [
        dist_dir, // remove and re-create dist/ first
        assets_dir, // app-assets is in dist/ during bootstrap
        release_dir,
    ] {
        if dir.exists() {
            fs::remove_dir_all(dir)
                .with_context(|| format!("Failed to remove directory: {}", dir.display()))?;
        }
        fs::create_dir_all(dir)
            .with_context(|| format!("Failed to create directory: {}", dir.display()))?;
    }

    // create_app_dirs(); // Not needed yet, we only need simsapa_dir and assets_dir at the moment.

    // Remove unzipped_stardict directory if it exists
    let unzipped_stardict_dir = simsapa_dir.join("unzipped_stardict");
    if unzipped_stardict_dir.exists() {
        fs::remove_dir_all(&unzipped_stardict_dir)
            .with_context(|| format!("Failed to remove unzipped_stardict directory: {}", unzipped_stardict_dir.display()))?;
    }

    // Remove .tar.bz2 files in simsapa_dir
    if simsapa_dir.exists() {
        let entries = fs::read_dir(simsapa_dir)
            .with_context(|| format!("Failed to read simsapa directory: {}", simsapa_dir.display()))?;

        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() {
                if let Some(extension) = path.extension() {
                    if extension == "bz2" && path.to_string_lossy().ends_with(".tar.bz2") {
                        fs::remove_file(&path)
                            .with_context(|| format!("Failed to remove file: {}", path.display()))?;
                        println!("Removed: {}", path.display());
                    }
                }
            }
        }
    }

    // Clear log.txt
    let log_path = simsapa_dir.join("log.txt");
    fs::write(&log_path, "")
        .with_context(|| format!("Failed to clear log file: {}", log_path.display()))?;

    println!("Bootstrap cleanup and folder creation completed");
    Ok(())
}

/// Create tar.bz2 archive from a database file and move to release directory
///
/// Takes a database path (e.g., "path/to/appdata.sqlite3") and creates a compressed
/// tar.bz2 archive (e.g., "appdata.tar.bz2") in the same directory, then moves it
/// to the release directory.
pub fn create_database_archive(db_path: &Path, release_dir: &Path) -> Result<()> {
    let db_name = db_path.file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| anyhow::anyhow!("Invalid database filename"))?;

    // Create tar name by replacing .sqlite3 with .tar.bz2
    let tar_name = db_name.replace(".sqlite3", ".tar.bz2");

    let db_dir = db_path.parent()
        .ok_or_else(|| anyhow::anyhow!("Invalid database path"))?;

    logger::info(&format!("Creating {} archive", tar_name));

    let tar_result = std::process::Command::new("tar")
        .arg("cjf")
        .arg(&tar_name)
        .arg(db_name)
        .current_dir(db_dir)
        .status()
        .context("Failed to execute tar command")?;

    if !tar_result.success() {
        anyhow::bail!("tar command failed for {}", tar_name);
    }

    // Move tar archive to release directory
    let tar_src = db_dir.join(&tar_name);
    let tar_dst = release_dir.join(&tar_name);
    fs::rename(&tar_src, &tar_dst)
        .with_context(|| format!("Failed to move {} to release directory", tar_name))?;

    logger::info(&format!("Created and moved {} to {:?}", tar_name, release_dir));

    Ok(())
}
