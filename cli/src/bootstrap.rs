use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use anyhow::{Result, Context};
use chrono::{DateTime, Local};
use simsapa_backend::{get_create_simsapa_dir, get_create_simsapa_app_assets_path};

pub fn bootstrap(write_new_dotenv: bool) -> Result<()> {
    let start_time: DateTime<Local> = Local::now();
    let iso_date = start_time.format("%Y-%m-%d").to_string();

    let bootstrap_limit: Option<i32> = match env::var("BOOTSTRAP_LIMIT") {
        Ok(s) if !s.is_empty() => s.parse().ok(),
        _ => None,
    };

    // Running the binary with 'cargo run', the PWD is simsapa-ng/cli/.
    // The asset folders are one level above simsapa-ng/.
    let bootstrap_assets_dir = PathBuf::from("../../bootstrap-assets-resources");
    let release_dir = PathBuf::from(format!("../../releases/{}-dev", iso_date));
    let dist_dir = bootstrap_assets_dir.join("dist");
    let _sc_data_dir = bootstrap_assets_dir.join("sc-data");

    // During bootstrap, don't touch the user's Simsapa dir (~/.local/share/simsapa-ng)
    // Create files in the dist/ folder instead.
    // Setting the env var here to override any previous value.
    unsafe { env::set_var("SIMSAPA_DIR", &dist_dir.join("simsapa-ng")); }

    let simsapa_dir = get_create_simsapa_dir()
        .map_err(|e| anyhow::anyhow!("Failed to get simsapa directory: {}", e))?;
    let assets_dir = get_create_simsapa_app_assets_path();

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

    Ok(())
}

fn clean_and_create_folders(
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
