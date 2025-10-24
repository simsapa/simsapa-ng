pub mod appdata;
pub mod suttacentral;
pub mod dhammatalks_org;
pub mod dhammapada_munindo;
pub mod dhammapada_tipitaka;
pub mod nyanadipa;
pub mod buddha_ujja;
pub mod completions;

use anyhow::Result;
use diesel::prelude::*;
use diesel_migrations::MigrationHarness;
use std::path::Path;
use simsapa_backend::db::APPDATA_MIGRATIONS;

pub use appdata::AppdataBootstrap;
// TODO: Uncomment these as the modules are implemented
// pub use suttacentral::SuttaCentralImporter;
// pub use dhammatalks_org::DhammatalksSuttaImporter;
// pub use dhammapada_munindo::DhammapadaMunindoImporter;
// pub use dhammapada_tipitaka::DhammapadaTipitakaImporter;
// pub use nyanadipa::NyanadipaImporter;
// pub use buddha_ujja::BuddhaUjjaImporter;
// pub use completions::CompletionsGenerator;

pub trait SuttaImporter {
    fn import(&mut self, conn: &mut SqliteConnection) -> Result<()>;
}

pub type ProgressCallback = Box<dyn Fn(usize, usize, &str)>;

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
        std::fs::create_dir_all(path)?;
    }
    Ok(())
}

pub fn read_json_file<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T> {
    let file_content = std::fs::read_to_string(path)?;
    let data: T = serde_json::from_str(&file_content)?;
    Ok(data)
}

pub fn batch_insert<T, F>(
    conn: &mut SqliteConnection,
    items: Vec<T>,
    batch_size: usize,
    insert_fn: F,
) -> Result<()>
where
    T: Clone,
    F: Fn(&mut SqliteConnection, Vec<T>) -> Result<(), diesel::result::Error>,
{
    for chunk in items.chunks(batch_size) {
        insert_fn(conn, chunk.to_vec())?;
    }
    Ok(())
}
