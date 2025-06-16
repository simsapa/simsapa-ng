pub mod db;
pub mod types;
pub mod helpers;
pub mod query_task;
pub mod html_content;
pub mod dir_list;
pub mod app_data;
pub mod stardict_parse;
pub mod pali_stemmer;
pub mod logger;
pub mod theme_colors;
pub mod app_settings;

use std::env;
use std::fs::{self, create_dir_all};
use std::path::{Path, PathBuf};
use std::error::Error;
use std::sync::OnceLock;

use app_dirs::{get_app_root, AppDataType, AppInfo};
use dotenvy::dotenv;

use crate::logger::{info, error};
use crate::app_data::AppData;

pub static APP_INFO: AppInfo = AppInfo{name: "simsapa-ng", author: "profound-labs"};
static APP_GLOBALS: OnceLock<AppGlobals> = OnceLock::new();
static APP_DATA: OnceLock<AppData> = OnceLock::new();

pub fn init_app_globals() {
    if APP_GLOBALS.get().is_none() {
        let g = AppGlobals::new();
        APP_GLOBALS.set(g).expect("Can't set AppGlobals");
    }
}

pub fn get_app_globals() -> &'static AppGlobals {
    APP_GLOBALS.get().expect("AppGlobals is not initialized")
}

// #[unsafe(no_mangle)]
// pub extern "C" fn rust_backend_init_db() -> bool {
//     db::rust_backend_init_db()
// }

#[unsafe(no_mangle)]
pub extern "C" fn init_app_data() {
    if APP_GLOBALS.get().is_none() {
        info("init_app_data() start");
        let app_data = AppData::new();
        APP_DATA.set(app_data).expect("Can't set AppData");
        info("init_appdata() end");
    }
}

pub fn get_app_data() -> &'static AppData {
    APP_DATA.get().expect("AppData is not initialized")
}

#[derive(Debug)]
pub struct AppGlobals {
    pub page_len: usize,
    pub api_port: i32,
    pub api_url: String,
    pub simsapa_dir: PathBuf,
    pub app_assets_dir: PathBuf,

    pub appdata_db_path: PathBuf,
    pub appdata_abs_path: PathBuf,
    pub appdata_database_url: String,

    pub userdata_db_path: PathBuf,
    pub userdata_abs_path: PathBuf,
    pub userdata_database_url: String,

    pub dict_db_path: PathBuf,
    pub dict_abs_path: PathBuf,
    pub dict_database_url: String,

    pub dpd_db_path: PathBuf,
    pub dpd_abs_path: PathBuf,
    pub dpd_database_url: String,
}

impl AppGlobals {
    pub fn new() -> Self {
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
        let appdata_abs_path = fs::canonicalize(appdata_db_path.clone()).unwrap_or(appdata_db_path.clone());
        let appdata_database_url = format!("sqlite://{}", appdata_abs_path.as_os_str().to_str().expect("os_str Error!"));

        let userdata_db_path = app_assets_dir.join("userdata.sqlite3");
        let userdata_abs_path = fs::canonicalize(userdata_db_path.clone()).unwrap_or(userdata_db_path.clone());
        let userdata_database_url = format!("sqlite://{}", userdata_abs_path.as_os_str().to_str().expect("os_str Error!"));

        let dict_db_path = app_assets_dir.join("dictionaries.sqlite3");
        let dict_abs_path = fs::canonicalize(dict_db_path.clone()).unwrap_or(dict_db_path.clone());
        let dict_database_url = format!("sqlite://{}", dict_abs_path.as_os_str().to_str().expect("os_str Error!"));

        let dpd_db_path = app_assets_dir.join("dpd.sqlite3");
        let dpd_abs_path = fs::canonicalize(dpd_db_path.clone()).unwrap_or(dpd_db_path.clone());
        let dpd_database_url = format!("sqlite://{}", dpd_abs_path.as_os_str().to_str().expect("os_str Error!"));

        AppGlobals {
            page_len: 10,
            api_port: 4848,
            api_url: "http://localhost:4848".to_string(),
            simsapa_dir,
            app_assets_dir,

            appdata_db_path,
            appdata_abs_path,
            appdata_database_url,

            userdata_db_path,
            userdata_abs_path,
            userdata_database_url,

            dict_db_path,
            dict_abs_path,
            dict_database_url,

            dpd_db_path,
            dpd_abs_path,
            dpd_database_url,
        }
    }
}

/// PathBuf::exists() can crash on Android due to permission restrictions.
/// This function only returns Ok(true), false is turned into an error message.
/// If the file exists but is 0 byte length, this is also returned as an error.
fn check_file_exists_print_err<P: AsRef<Path>>(path: P) -> Result<bool, Box<dyn Error>> {
    let path_ref = path.as_ref();

    let exists = path_ref.try_exists()?;
    if !exists {
        let msg = format!("File doesn't exist: {}", path_ref.display());
        error(&msg);
        return Err(msg.into());
    }

    // Must also test for file length.
    // The file might exist but it may be 0 length.
    // This can happen if diesel::ConnectionManager::new() was called on a non-existent file.
    let metadata = fs::metadata(path_ref)?;
    if metadata.len() == 0 {
        let msg = format!("File is 0 bytes: {}", path_ref.display());
        error(&msg);
        return Err(msg.into());
    }

    Ok(true)
}

pub fn get_create_simsapa_app_root() -> Result<PathBuf, Box<dyn Error>> {
    // AppDataType::UserData
    // - Android: /data/user/0/com.profoundlabs.simsapa/files/.local/share/simsapa-ng
    // AppDataType::UserConfig
    // - Android: /data/user/0/com.profoundlabs.simsapa/files/.config/simsapa-ng
    let p = get_app_root(AppDataType::UserData, &APP_INFO)?;
    if !p.try_exists()? {
        create_dir_all(&p)?;
    }
    Ok(p)
}

pub fn get_create_simsapa_app_assets_path() -> PathBuf {
    let p = get_create_simsapa_app_root().unwrap_or(PathBuf::from(".")).join("app-assets/");

    match p.try_exists() {
        Ok(r) => if !r {
            let _ = create_dir_all(&p);
        }
        Err(e) => error(&format!("{}", e)),
    }

    p
}

pub fn get_create_simsapa_appdata_db_path() -> PathBuf {
    get_create_simsapa_app_assets_path().join("appdata.sqlite3")
}

#[unsafe(no_mangle)]
pub extern "C" fn appdata_db_exists() -> bool {
    match get_create_simsapa_appdata_db_path().try_exists() {
        Ok(r) => r,
        Err(e) => {
            error(&format!("{}", e));
            false
        },
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn dotenv_c() {
    dotenv().ok();
}
