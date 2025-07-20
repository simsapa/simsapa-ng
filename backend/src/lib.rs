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
use std::io::{self, Read, Write};
use std::fs::{self, File, create_dir_all, remove_dir_all};
use std::path::{Path, PathBuf};
use std::error::Error;
use std::sync::OnceLock;

use app_dirs::{get_app_root, AppDataType, AppInfo};
use dotenvy::dotenv;
use walkdir::WalkDir;
use cfg_if::cfg_if;

use crate::logger::{info, warn, error};
use crate::app_data::AppData;

pub static APP_INFO: AppInfo = AppInfo{name: "simsapa-ng", author: "profound-labs"};
static APP_GLOBALS: OnceLock<AppGlobals> = OnceLock::new();
static APP_DATA: OnceLock<AppData> = OnceLock::new();

#[unsafe(no_mangle)]
pub extern "C" fn init_app_globals() {
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
    if APP_DATA.get().is_none() {
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
    pub download_temp_folder: PathBuf,
    pub extract_temp_folder: PathBuf,
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

        let simsapa_dir = if let Ok(p) = get_create_simsapa_dir() {
            p
        } else {
            PathBuf::from(".")
        };

        let download_temp_folder = simsapa_dir.join("temp-download");
        let extract_temp_folder = download_temp_folder.join("temp-extract");

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
            download_temp_folder,
            extract_temp_folder,
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

pub fn get_create_simsapa_internal_app_root() -> Result<PathBuf, Box<dyn Error>> {
    // AppDataType::UserData
    // - Android: /data/user/0/com.profoundlabs.simsapa/files/.local/share/simsapa-ng
    // AppDataType::UserConfig
    // - Android: /data/user/0/com.profoundlabs.simsapa/files/.config/simsapa-ng
    let mut p = get_app_root(AppDataType::UserData, &APP_INFO)?;

    // On Android and iOS, strip .local/share/simsapa-ng from the path, so that
    // it is consistent with the storage selection path saved by
    // storage_manager::save_storage_path().
    if is_mobile() && p.ends_with(".local/share/simsapa-ng") {
        p = p.parent().unwrap()
             .parent().unwrap()
             .parent().unwrap()
             .to_path_buf()
    }

    if !p.try_exists()? {
        create_dir_all(&p)?;
    }
    Ok(p)
}

pub fn get_create_simsapa_dir() -> Result<PathBuf, Box<dyn Error>> {
    info(&format!("get_create_simsapa_dir()"));
    let simsapa_dir = match env::var("SIMSAPA_DIR") {
        // If SIMSAPA_DIR env variable was defined, use that.
        Ok(s) => Ok(PathBuf::from(s)),
        Err(_) => {
            // Else, check if storage path was selected before.
            let internal_app_root = if let Ok(p) = get_create_simsapa_internal_app_root() {
                p
            } else {
                PathBuf::from(".")
            };

            // On desktop, always use the internal app root.
            if !is_mobile() {
                return Ok(internal_app_root);
            }

            // On mobile, if there is a file storage-path.txt, read the path from there.
            // Else, use the internal app root.

            let storage_config_path = internal_app_root.join("storage-path.txt");
            let mut file = match File::open(&storage_config_path) {
                Ok(file) => {
                    info(&format!("Found: {}", &storage_config_path.to_str().unwrap_or_default()));
                    file
                },
                Err(e) => {
                    warn(&format!("File not found: {}, Error: {}",
                                  &storage_config_path.to_str().unwrap_or_default(),
                                  e));
                    return Ok(internal_app_root);
                },
            };

            let mut contents = String::new();
            match file.read_to_string(&mut contents) {
                Ok(_) => (),
                Err(e) => {
                    error(&format!("Failed to read file: {}", e));
                    return Ok(internal_app_root);
                },
            }

            info(&format!("Contents: {}", &contents));

            // storage path
            let p = PathBuf::from(contents);
            if !p.try_exists()? {
                create_dir_all(&p)?;
            }
            Ok(p)
        }
    };

    simsapa_dir
}

pub fn get_create_simsapa_app_assets_path() -> PathBuf {
    let p = get_create_simsapa_dir().unwrap_or(PathBuf::from(".")).join("app-assets/");

    match p.try_exists() {
        Ok(r) => if !r {
            match create_dir_all(&p) {
                Ok(_) => {},
                Err(e) => error(&format!("{}", e)),
            };
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


#[unsafe(no_mangle)]
pub extern "C" fn ensure_no_empty_db_files() {
    let g = get_app_globals();
    for p in [g.appdata_db_path.clone(),
              g.userdata_db_path.clone(),
              g.dict_db_path.clone(),
              g.dpd_db_path.clone()] {
        match p.try_exists() {
            Ok(true) => {
                match fs::metadata(&p) {
                    Ok(metadata) if metadata.len() == 0 => {
                        if let Err(e) = fs::remove_file(&p) {
                            eprintln!("Failed to remove file {:?}: {}", p, e);
                        }
                    }
                    Ok(_) => {}, // File exists but is not empty
                    Err(e) => eprintln!("Failed to get metadata for {:?}: {}", p, e),
                }
            }
            Ok(false) => {}, // File doesn't exist
            Err(e) => eprintln!("Failed to check if file exists {:?}: {}", p, e),
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn remove_download_temp_folder() {
    let g = get_app_globals();

    match g.download_temp_folder.try_exists() {
        Ok(exists) => {
            if exists {
                let _ = remove_dir_all(&g.download_temp_folder);
            }
        }

        Err(e) => {
            error(&format!("{}", e));
            return;
        }
    }
}

pub fn move_folder_contents<P: AsRef<Path>>(src: P, dest: P) -> io::Result<()> {
    let src_path = src.as_ref();
    let dest_path = dest.as_ref();

    // Create destination directory if it doesn't exist
    fs::create_dir_all(dest_path)?;

    // Collect all entries and sort by depth (deepest first for proper deletion)
    let mut entries: Vec<_> = WalkDir::new(src_path)
        .into_iter()
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

    // Sort by depth (deepest first) to handle nested structures properly
    entries.sort_by(|a, b| b.depth().cmp(&a.depth()));

    // Create directory structure first
    for entry in &entries {
        if entry.file_type().is_dir() && entry.path() != src_path {
            let relative_path = entry.path().strip_prefix(src_path)
                                            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
            let dest_dir = dest_path.join(relative_path);
            fs::create_dir_all(&dest_dir)?;
        }
    }

    // Move files and remove directories
    for entry in entries {
        let entry_path = entry.path();

        if entry_path == src_path {
            continue; // Skip the root source directory itself
        }

        if entry.file_type().is_file() {
            let relative_path = entry_path.strip_prefix(src_path)
                                          .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
            let dest_file = dest_path.join(relative_path);
            fs::rename(entry_path, dest_file)?;
        } else if entry.file_type().is_dir() {
            // Remove directory after its contents have been moved
            if let Err(e) = fs::remove_dir(entry_path) {
                // Only error if it's not already empty/removed
                if e.kind() != io::ErrorKind::NotFound {
                    return Err(e);
                }
            }
        }
    }

    Ok(())
}

pub fn is_mobile() -> bool {
    cfg_if! {
        if #[cfg(any(target_os = "android", target_os = "ios"))] {
            true
        } else {
            false
        }
    }
}

pub fn create_parent_directory(path: &str) -> String {
    match Path::new(path).parent() {
        None => format!("Invalid path: {}", path),
        Some(parent) => match std::fs::create_dir_all(parent) {
            Ok(_) => String::from(""),
            Err(e) => format!("Failed to create directory: {}", e),
        },
    }
}

pub fn save_to_file(data: &[u8], path: &str) -> String {
    match File::create(path) {
        Ok(mut file) => match file.write_all(data) {
            Ok(_) => String::from(format!("File saved successfully to {}", path)),
            Err(e) => format!("Failed to write file: {}", e),
        },
        Err(e) => format!("Failed to create file: {}", e),
    }
}
