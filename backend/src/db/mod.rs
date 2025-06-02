pub mod appdata;
pub mod appdata_models;
pub mod appdata_schema;
pub mod dictionaries;
pub mod dictionaries_models;
pub mod dictionaries_schema;
pub mod dpd;
pub mod dpd_models;
pub mod dpd_schema;

use std::env;
use std::path::PathBuf;
use std::fs;
use std::sync::OnceLock;

use diesel::prelude::*;
use diesel::r2d2::{Pool, ConnectionManager, PooledConnection};
// use diesel::sqlite::Sqlite;

use dotenvy::dotenv;
use parking_lot::Mutex;
use anyhow::{Context, Result, Error as AnyhowError};

use crate::db::appdata::AppdataDbHandle;
use crate::db::dictionaries::DictionariesDbHandle;
use crate::db::dpd::DpdDbHandle;
use crate::get_create_simsapa_app_root;

pub type SqlitePool = Pool<ConnectionManager<SqliteConnection>>;
pub type DbConn = PooledConnection<ConnectionManager<SqliteConnection>>;

#[derive(Debug)]
pub struct DatabaseHandle {
    pool: SqlitePool,
    pub write_lock: Mutex<()>,
}

#[derive(Debug)]
pub struct DbManager {
    pub appdata: AppdataDbHandle,
    pub dictionaries: DictionariesDbHandle,
    pub dpd: DpdDbHandle,
}

pub static DATABASE_MANAGER: OnceLock<DbManager> = OnceLock::new();

impl DatabaseHandle {
    pub fn new(database_url: &str) -> Result<Self> {
        println!("DatabaseHandle::new() {}", database_url);
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
        println!("DbManager::new()");

        dotenv().ok();

        let simsapa_dir = match env::var("SIMSAPA_DIR") {
            Ok(s) => PathBuf::from(s),
            Err(_) => {
                if let Ok(p) = get_create_simsapa_app_root() {
                    p
                } else {
                    PathBuf::from(".")
                }
            }
        };

        let app_assets_dir = simsapa_dir.join("app-assets");

        let appdata_db_path = app_assets_dir.join("appdata.sqlite3");
        let dict_db_path = app_assets_dir.join("dictionaries.sqlite3");
        let dpd_db_path = app_assets_dir.join("dpd.sqlite3");

        if !appdata_db_path.exists() {
            panic!("Appdata database file not found at expected location: {:?}", appdata_db_path);
        }

        if !dict_db_path.exists() {
            panic!("Dictionary database file not found at expected location: {:?}", dict_db_path);
        }

        if !dpd_db_path.exists() {
            panic!("Dictionary database file not found at expected location: {:?}", dpd_db_path);
        }

        let appdata_abs_path = fs::canonicalize(appdata_db_path.clone()).unwrap_or(appdata_db_path);
        let appdata_database_url = format!("sqlite://{}", appdata_abs_path.as_os_str().to_str().expect("os_str Error!"));

        let dict_abs_path = fs::canonicalize(dict_db_path.clone()).unwrap_or(dict_db_path);
        let dict_database_url = format!("sqlite://{}", dict_abs_path.as_os_str().to_str().expect("os_str Error!"));

        let dpd_abs_path = fs::canonicalize(dpd_db_path.clone()).unwrap_or(dpd_db_path);
        let dpd_database_url = format!("sqlite://{}", dpd_abs_path.as_os_str().to_str().expect("os_str Error!"));

        Ok(Self {
            appdata: DatabaseHandle::new(&appdata_database_url)?,
            dictionaries: DatabaseHandle::new(&dict_database_url)?,
            dpd: DatabaseHandle::new(&dpd_database_url)?,
        })
    }
}

pub fn rust_backend_init_db() -> bool {
    println!("rust_backend_init_db() start");
    let manager = DbManager::new().expect("Can't create DbManager");
    DATABASE_MANAGER.set(manager).unwrap();
    println!("rust_backend_init_db() end");
    true
}

pub fn get_dbm() -> &'static DbManager {
    DATABASE_MANAGER.get().expect("DbManager is not initialized")
}

/// Returns connections as a tuple to appdata.sqlite3, dictionaries.sqlite3, dpd.sqlite3
pub fn establish_connection() -> (SqliteConnection, SqliteConnection, SqliteConnection) {
    println!("establish_connection()");
    dotenv().ok();

    let simsapa_dir = match env::var("SIMSAPA_DIR") {
        Ok(s) => PathBuf::from(s),
        Err(_) => {
            if let Ok(p) = get_create_simsapa_app_root() {
                p
            } else {
                PathBuf::from(".")
            }
        }
    };

    let app_assets_dir = simsapa_dir.join("app-assets");

    let appdata_db_path = app_assets_dir.join("appdata.sqlite3");
    let dict_db_path = app_assets_dir.join("dictionaries.sqlite3");
    let dpd_db_path = app_assets_dir.join("dpd.sqlite3");

    if !appdata_db_path.exists() {
        panic!("Appdata database file not found at expected location: {:?}", appdata_db_path);
    }

    if !dict_db_path.exists() {
        panic!("Dictionary database file not found at expected location: {:?}", dict_db_path);
    }

    if !dpd_db_path.exists() {
        panic!("Dictionary database file not found at expected location: {:?}", dpd_db_path);
    }

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

