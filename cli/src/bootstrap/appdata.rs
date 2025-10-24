use anyhow::Result;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
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

    pub fn run(&mut self) -> Result<()> {
        info!("Starting appdata database bootstrap");

        self.create_database()?;

        let mut conn = create_database_connection(&self.output_path)?;

        self.initialize_app_settings(&mut conn)?;
        self.initialize_providers(&mut conn)?;

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
