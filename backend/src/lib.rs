pub mod types;
pub mod helpers;
pub mod export_helpers;
pub mod query_task;
pub mod html_content;
pub mod dir_list;
pub mod app_data;
pub mod stardict_parse;
pub mod pali_stemmer;

pub mod db;
pub mod models_appdata;
pub mod schema_appdata;
pub mod models_dictionaries;
pub mod schema_dictionaries;
pub mod models_dpd;
pub mod schema_dpd;

use std::fs::create_dir_all;
use std::path::PathBuf;
use std::error::Error;
use app_dirs::{get_app_root, AppDataType, AppInfo};

pub static PAGE_LEN: usize = 10;

pub static API_PORT: i32 = 4848;
pub static API_URL: &'static str = "http://localhost:4848";

pub const APP_INFO: AppInfo = AppInfo{name: "simsapa-ng", author: "profound-labs"};

pub fn get_create_simsapa_app_root() -> Result<PathBuf, Box<dyn Error>> {
    // AppDataType::UserData
    // - Android: /data/user/0/com.profoundlabs.simsapa/files/.local/share/simsapa-ng
    // AppDataType::UserConfig
    // - Android: /data/user/0/com.profoundlabs.simsapa/files/.config/simsapa-ng
    let p = get_app_root(AppDataType::UserData, &APP_INFO)?;
    if !p.exists() {
        create_dir_all(&p)?;
    }
    Ok(p)
}

pub fn get_create_simsapa_app_assets_path() -> PathBuf {
    let p = get_create_simsapa_app_root().unwrap_or(PathBuf::from(".")).join("app-assets/");
    if !p.exists() {
        let _ = create_dir_all(&p);
    }
    p
}

pub fn get_create_simsapa_appdata_db_path() -> PathBuf {
    get_create_simsapa_app_assets_path().join("appdata.sqlite3")
}

#[unsafe(no_mangle)]
pub extern "C" fn appdata_db_exists() -> bool {
    get_create_simsapa_appdata_db_path().exists()
}
