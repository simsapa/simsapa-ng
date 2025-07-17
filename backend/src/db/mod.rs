pub mod appdata;
pub mod appdata_models;
pub mod appdata_schema;
pub mod dictionaries;
pub mod dictionaries_models;
pub mod dictionaries_schema;
pub mod dpd;
pub mod dpd_models;
pub mod dpd_schema;

use std::path::PathBuf;
use std::fs;
use std::sync::OnceLock;

use diesel::prelude::*;
use diesel::r2d2::{Pool, ConnectionManager, PooledConnection};
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
// use diesel::sqlite::Sqlite;

use dotenvy::dotenv;
use parking_lot::Mutex;
use anyhow::{Context, Result, Error as AnyhowError};

use crate::logger::{info, error};
use crate::db::appdata::AppdataDbHandle;
use crate::db::appdata_models::AppSetting;
use crate::db::dictionaries::DictionariesDbHandle;
use crate::db::dpd::DpdDbHandle;
use crate::app_settings::AppSettings;
use crate::{check_file_exists_print_err, get_create_simsapa_dir, get_app_globals};

pub type SqlitePool = Pool<ConnectionManager<SqliteConnection>>;
pub type DbConn = PooledConnection<ConnectionManager<SqliteConnection>>;

pub const APPDATA_MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations/appdata/");

pub static DATABASE_MANAGER: OnceLock<DbManager> = OnceLock::new();

#[derive(Debug)]
pub struct DatabaseHandle {
    pool: SqlitePool,
    pub write_lock: Mutex<()>,
}

#[derive(Debug)]
pub struct DbManager {
    pub appdata: AppdataDbHandle,
    pub userdata: AppdataDbHandle,
    pub dictionaries: DictionariesDbHandle,
    pub dpd: DpdDbHandle,
}

impl DatabaseHandle {
    pub fn new(database_url: &str) -> Result<Self> {
        info(&format!("DatabaseHandle::new() {}", database_url));
        let manager = ConnectionManager::new(database_url);
        let pool = Pool::builder()
            .max_size(5)
            .build(manager)
            .with_context(|| format!("Failed to create pool for: {}", database_url))?;

        Ok(Self {
            pool,
            write_lock: Mutex::new(()),
        })
    }

    pub fn get_conn(&self) -> Result<DbConn> {
        self.pool.get().map_err(AnyhowError::from)
    }

    /// Performs a write operation on the database, guarded by a Mutex write_lock.
    pub fn do_write<F, T>(&self, operation: F) -> Result<T>
    where
        F: FnOnce(&mut SqliteConnection) -> Result<T, diesel::result::Error>,
    {
        let _lock = self.write_lock.lock();
        let mut db_conn = self.pool.get()
            .context("Failed to get connection from pool for write")?;
        operation(&mut db_conn).map_err(AnyhowError::from) // Convert diesel::result::Error to anyhow::Error
    }

    /// Performs a read operation on the database.
    pub fn do_read<F, T>(&self, operation: F) -> Result<T>
    where
        F: FnOnce(&mut SqliteConnection) -> Result<T, diesel::result::Error>,
    {
        let mut db_conn = self.pool.get()
            .context("Failed to get connection from pool for read")?;
        operation(&mut db_conn).map_err(AnyhowError::from)
    }
}

impl DbManager {
    pub fn new() -> Result<Self> {
        info("DbManager::new()");

        let g = get_app_globals();

        info(&format!("simsapa_dir: {}", g.simsapa_dir.to_string_lossy()));

        // PathBuf::exists() can crash on Android due to permission restrictions,
        // but no errors are reported.
        //
        // FIXME: Return the errors
        let _ = check_file_exists_print_err(&g.appdata_db_path);
        let _ = check_file_exists_print_err(&g.dict_db_path);
        let _ = check_file_exists_print_err(&g.dpd_db_path);

        // If userdata doesn't exist, create it with default settings.
        let userdata_exists = match check_file_exists_print_err(&g.userdata_db_path) {
            Ok(r) => r,
            Err(_) => false,
        };

        if !userdata_exists {
            initialize_userdata(&g.userdata_database_url)
                .with_context(|| format!("Failed to initialize database at '{}'", g.userdata_database_url))?;
        }

        Ok(Self {
            appdata: DatabaseHandle::new(&g.appdata_database_url)?,
            userdata: DatabaseHandle::new(&g.userdata_database_url)?,
            dictionaries: DatabaseHandle::new(&g.dict_database_url)?,
            dpd: DatabaseHandle::new(&g.dpd_database_url)?,
        })
    }

    pub fn get_theme_name(&self) -> String {
        let app_settings = self.userdata.get_app_settings();
        app_settings.theme_name_as_string()
    }
}

fn initialize_userdata(database_url: &str) -> Result<()> {
    info(&format!("initialize_userdata(): {}", database_url));

    // Create initial connection to create the database file
    let mut db_conn = SqliteConnection::establish(database_url)
        .with_context(|| format!("Failed to create initial database connection to '{}'", database_url))?;

    run_appdata_migrations(&mut db_conn)
        .context("Failed to run database migrations")?;

    insert_default_settings(&mut db_conn)
        .context("Failed to insert default application settings")?;

    info(&format!("initialize_userdata(): end"));
    Ok(())
}

fn insert_default_settings(db_conn: &mut SqliteConnection) -> Result<()> {
    info("insert_default_settings()");
    use crate::db::appdata_schema::app_settings;

    let settings_json = serde_json::to_string(&AppSettings::default()).expect("Can't encode JSON");

    let value = appdata_models::NewAppSetting {
        key: "app_settings",
        value: Some(&settings_json),
    };

    diesel::insert_into(app_settings::table)
        .values(value)
        .execute(db_conn)
        .context("Failed to insert default settings into database")?;

    Ok(())
}

pub fn get_app_settings() -> AppSettings {
    info("get_app_settings()");
    use crate::db::appdata_schema::app_settings;

    let g = get_app_globals();

    let _ = check_file_exists_print_err(&g.userdata_db_path);

    let db_conn = &mut SqliteConnection::establish(&g.userdata_database_url)
        .unwrap_or_else(|_| panic!("Error connecting to {}", g.userdata_database_url));

    let json = app_settings::table
        .select(AppSetting::as_select())
        .filter(app_settings::key.eq("app_settings"))
        .first(db_conn)
        .optional();

    match json {
        Ok(x) => {
            // FIXME simplify this expression
            if let Some(setting) = x {
                if let Some(val) = setting.value {
                    let res: AppSettings = serde_json::from_str(&val).expect("Can't decode JSON");
                    res
                } else {
                    AppSettings::default()
                }
            } else {
                AppSettings::default()
            }
        },
        Err(e) => {
            error(&format!("{}", e));
            AppSettings::default()
        }
    }
}

fn run_appdata_migrations(db_conn: &mut SqliteConnection) -> Result<()> {
    info("run_appdata_migrations()");
    db_conn.run_pending_migrations(APPDATA_MIGRATIONS)
           .map_err(|e| anyhow::anyhow!("Failed to execute pending database migrations: {}", e))?;
    Ok(())
}

/// Returns connections as a tuple to appdata.sqlite3, dictionaries.sqlite3, dpd.sqlite3
pub fn establish_connection() -> (SqliteConnection, SqliteConnection, SqliteConnection) {
    info("establish_connection()");
    dotenv().ok();

    let simsapa_dir = if let Ok(p) = get_create_simsapa_dir() {
        p
    } else {
        PathBuf::from(".")
    };

    let app_assets_dir = simsapa_dir.join("app-assets");

    let appdata_db_path = app_assets_dir.join("appdata.sqlite3");
    let dict_db_path = app_assets_dir.join("dictionaries.sqlite3");
    let dpd_db_path = app_assets_dir.join("dpd.sqlite3");

    // PathBuf::exists() can crash on Android due to permission restrictions,
    // but no errors are reported.
    let _ = check_file_exists_print_err(&appdata_db_path);
    let _ = check_file_exists_print_err(&dict_db_path);
    let _ = check_file_exists_print_err(&dpd_db_path);

    let appdata_abs_path = fs::canonicalize(appdata_db_path.clone()).unwrap_or(appdata_db_path);
    let appdata_database_url = format!("sqlite://{}", appdata_abs_path.as_os_str().to_str().expect("os_str Error!"));
    let appdata_conn = SqliteConnection::establish(&appdata_database_url)
        .unwrap_or_else(|_| panic!("Error connecting to {}", appdata_database_url));

    let dict_abs_path = fs::canonicalize(dict_db_path.clone()).unwrap_or(dict_db_path);
    let dict_database_url = format!("sqlite://{}", dict_abs_path.as_os_str().to_str().expect("os_str Error!"));
    let dict_conn = SqliteConnection::establish(&dict_database_url)
        .unwrap_or_else(|_| panic!("Error connecting to {}", dict_database_url));

    let dpd_abs_path = fs::canonicalize(dpd_db_path.clone()).unwrap_or(dpd_db_path);
    let dpd_database_url = format!("sqlite://{}", dpd_abs_path.as_os_str().to_str().expect("os_str Error!"));
    let dpd_conn = SqliteConnection::establish(&dpd_database_url)
        .unwrap_or_else(|_| panic!("Error connecting to {}", dpd_database_url));

    (appdata_conn, dict_conn, dpd_conn)
}

