use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Result, Context};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

use simsapa_backend::db::appdata_models::NewAppSetting;
use simsapa_backend::db::appdata_schema::app_settings;

use crate::bootstrap::{create_database_connection, run_migrations, ensure_directory_exists, read_json_file};

#[derive(Debug, Deserialize, Serialize)]
struct ProviderModel {
    model_name: String,
    enabled: bool,
    removable: bool,
}

#[derive(Debug, Deserialize, Serialize)]
struct Provider {
    name: String,
    description: String,
    enabled: bool,
    api_key_env_var_name: String,
    api_key_value: Option<String>,
    models: Vec<ProviderModel>,
}

pub struct AppdataBootstrap {
    output_path: PathBuf,
}

impl AppdataBootstrap {
    pub fn new(output_path: PathBuf) -> Self {
        Self { output_path }
    }

    pub fn create_database(&self) -> Result<()> {
        info!("Creating appdata database at: {:?}", self.output_path);

        if self.output_path.exists() {
            info!("Deleting existing database file");
            std::fs::remove_file(&self.output_path)?;
        }

        ensure_directory_exists(
            self.output_path
                .parent()
                .ok_or_else(|| anyhow::anyhow!("Invalid database path"))?
        )?;

        info!("Creating new SQLite database file");
        let mut conn = create_database_connection(&self.output_path)?;

        info!("Running diesel migrations to create schema");
        run_migrations(&mut conn)?;

        info!("Database created successfully");
        Ok(())
    }

    pub fn initialize_app_settings(&self, conn: &mut SqliteConnection) -> Result<()> {
        info!("Initializing app settings");

        let settings = vec![
            NewAppSetting {
                key: "app_version",
                value: Some("0.1.0"),
            },
            NewAppSetting {
                key: "db_version",
                value: Some("1"),
            },
            NewAppSetting {
                key: "first_run",
                value: Some("true"),
            },
        ];

        diesel::insert_into(app_settings::table)
            .values(&settings)
            .execute(conn)?;

        info!("App settings initialized with {} entries", settings.len());
        Ok(())
    }

    pub fn initialize_providers(&self, conn: &mut SqliteConnection) -> Result<()> {
        info!("Initializing providers from providers.json");

        let providers_json_path = Path::new("assets/providers.json");
        
        if !providers_json_path.exists() {
            warn!("providers.json not found at {:?}, skipping provider initialization", providers_json_path);
            return Ok(());
        }

        let providers: Vec<Provider> = read_json_file(providers_json_path)?;
        
        for provider in &providers {
            let provider_json = serde_json::to_string(provider)?;
            let setting = NewAppSetting {
                key: &format!("provider_{}", provider.name.to_lowercase().replace(" ", "_")),
                value: Some(&provider_json),
            };

            diesel::insert_into(app_settings::table)
                .values(&setting)
                .execute(conn)?;
        }

        info!("Providers initialized with {} entries", providers.len());
        Ok(())
    }

    pub fn create_fts5_indexes(&self) -> Result<()> {
        let appdata_db_path = self.output_path.clone();

        // NOTE: Make sure appdata db connections are closed before running this.

        // NOTE: Running the SQL script with the sqlite3 cli, it creates the fts5 index data.
        // But executing it with a Diesel db connection from Rust, the fts5 tables are created but there is no index data in them.
        // Perhaps the trigram tokenizer is missing from Diesel SQLite?

        // Get the absolute path to the SQL script
        let sql_script_path = PathBuf::from("../scripts/appdata-fts5-index-for-suttas-content_plain.sql");

        // Check if the SQL script exists
        if !sql_script_path.exists() {
            return Err(anyhow::anyhow!(
                "SQL script not found at: {}",
                sql_script_path.display()
            ));
        }

        // Get absolute path to the destination database
        let appdata_db_abs_path = fs::canonicalize(&appdata_db_path)
            .with_context(|| format!("Failed to get absolute path for database: {}", appdata_db_path.display()))?;

        // Execute sqlite3 CLI command with input redirection
        let mut child = Command::new("sqlite3")
            .arg(&appdata_db_abs_path)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .with_context(|| "Failed to spawn sqlite3 command")?;

        // Read the SQL script content and write it to sqlite3's stdin
        let sql_content = fs::read_to_string(&sql_script_path)
            .with_context(|| format!("Failed to read SQL script: {}", sql_script_path.display()))?;

        if let Some(stdin) = child.stdin.take() {
            use std::io::Write;
            let mut stdin = stdin;
            stdin.write_all(sql_content.as_bytes())
                .with_context(|| "Failed to write SQL content to sqlite3 stdin")?;
            // Close stdin to signal end of input
            drop(stdin);
        }

        // Wait for the command to complete
        let output = child.wait_with_output()
            .with_context(|| "Failed to execute sqlite3 command")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!(
                "sqlite3 command failed with exit code {}: {}",
                output.status.code().unwrap_or(-1),
                stderr
            ));
        }

        println!("Successfully created FTS5 indexes and triggers using sqlite3 CLI");
        Ok(())
    }

    pub fn run(&mut self) -> Result<()> {
        info!("Starting appdata database bootstrap");

        self.create_database()?;

        {
            let mut conn = create_database_connection(&self.output_path)?;
            self.initialize_app_settings(&mut conn)?;
            self.initialize_providers(&mut conn)?;
            // Connection will be automatically dropped and closed at the end of this scope
        }
        // NOTE: Db connection to appdata must be closed before create_fts5_indexes()
        self.create_fts5_indexes()?;

        info!("Appdata database bootstrap completed successfully");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_database_creation() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test_appdata.sqlite3");
        
        let bootstrap = AppdataBootstrap::new(db_path.clone());
        
        assert!(bootstrap.create_database().is_ok());
        assert!(db_path.exists());
    }

    #[test]
    fn test_app_settings_initialization() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test_appdata.sqlite3");
        
        let mut bootstrap = AppdataBootstrap::new(db_path.clone());
        bootstrap.create_database().unwrap();
        
        let mut conn = create_database_connection(&db_path).unwrap();
        
        assert!(bootstrap.initialize_app_settings(&mut conn).is_ok());
        
        use simsapa_backend::db::appdata_schema::app_settings::dsl::*;
        let count: i64 = app_settings.count().get_result(&mut conn).unwrap();
        assert!(count >= 3);
    }

    #[test]
    fn test_complete_appdata_creation() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test_appdata.sqlite3");
        
        let mut bootstrap = AppdataBootstrap::new(db_path.clone());
        
        assert!(bootstrap.run().is_ok());
        assert!(db_path.exists());
        
        let mut conn = create_database_connection(&db_path).unwrap();
        use simsapa_backend::db::appdata_schema::app_settings::dsl::*;
        let count: i64 = app_settings.count().get_result(&mut conn).unwrap();
        assert!(count >= 3);
    }
}
