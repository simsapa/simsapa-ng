pub mod db;
pub mod types;
pub mod helpers;
pub mod models;
pub mod schema;
pub mod query_task;
pub mod html_content;
pub mod dir_list;

use std::path::PathBuf;
use app_dirs::{get_app_root, AppDataType, AppDirsError, AppInfo};

pub static PAGE_LEN: usize = 10;

pub static API_PORT: i32 = 4848;
pub static API_URL: &'static str = "http://localhost:4848";

pub const APP_INFO: AppInfo = AppInfo{name: "simsapa-ng", author: "profound-labs"};

pub fn get_simsapa_app_root() -> Result<PathBuf, AppDirsError> {
    get_app_root(AppDataType::UserData, &APP_INFO)
}

pub fn get_simsapa_app_assets_path() -> PathBuf {
    get_simsapa_app_root().unwrap_or(PathBuf::from(".")).join("app_assets/")
}

pub fn get_simsapa_appdata_db_path() -> PathBuf {
    get_simsapa_app_assets_path().join("appdata.sqlite3")
}
