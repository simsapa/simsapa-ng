use std::fs::File;
use std::path::{Path, PathBuf};
use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;
use std::sync::{OnceLock, Arc};

use rocket::serde::Deserialize;
use rocket::serde::json::Json;

use http;
use ureq;
use rocket::{get, post, routes, State, Shutdown};
use rocket::response::content::RawHtml;
use rocket::http::{ContentType, Status};
use rocket_cors::CorsOptions;

use simsapa_backend::{AppGlobals, get_create_simsapa_app_root, get_create_simsapa_appdata_db_path};
use simsapa_backend::html_content::html_page;
use simsapa_backend::dir_list::generate_html_directory_listing;
use simsapa_backend::db::DbManager;
use simsapa_backend::logger::{info, warn, error, profile};

pub static APP_GLOBALS_API: OnceLock<AppGlobals> = OnceLock::new();

pub fn init_app_globals_api() {
    if APP_GLOBALS_API.get().is_none() {
        let g = AppGlobals::new();
        APP_GLOBALS_API.set(g).expect("Can't set AppGlobals for API");
    }
}

pub fn get_app_globals_api() -> &'static AppGlobals {
    APP_GLOBALS_API.get().expect("AppGlobals (in API) is not initialized")
}

#[cxx_qt::bridge]
pub mod ffi {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;

        include!("utils.h");
        fn get_internal_storage_path() -> QString;
        // fn get_app_assets_path() -> QString;

        include!("gui.h");
        fn callback_run_lookup_query(query_text: QString);
        fn callback_run_summary_query(window_id: QString, query_text: QString);
        fn callback_run_sutta_menu_action(window_id: QString, action: QString, query_text: QString);
    }
}

static APP_ASSETS: include_dir::Dir<'_> = include_dir::include_dir!("$CARGO_MANIFEST_DIR/../assets/");

#[derive(Debug)]
pub struct AssetsHandler {
    files: &'static include_dir::Dir<'static>,
}

impl Default for AssetsHandler {
    fn default() -> Self {
        let files = &APP_ASSETS;
        Self { files }
    }
}

#[get("/assets/<path..>")]
fn serve_assets(path: PathBuf, assets: &State<AssetsHandler>) -> (Status, (ContentType, Vec<u8>)) {
    let path_str = path.to_str().unwrap_or("");

    let some_entry = assets.files.get_entry(path_str);

    if let Some(entry) = some_entry {
        if let Some(entry_file) = entry.as_file() {

            let p = PathBuf::from(path_str);
            let path_ext = match p.extension() {
                Some(s) => s.to_str().unwrap_or("txt"),
                None => "txt",
            };

            let content_type = ContentType::from_extension(path_ext).unwrap_or(ContentType::Plain);

            let body = Vec::from(entry_file.contents());

            (Status::Ok, (content_type, body))

        } else {
            let s = format!{"404 Not Found: {}", path_str};
            let ret = Vec::from(s.as_bytes());
            (Status::NotFound, (ContentType::Plain, ret))
        }

    } else {
        let s = format!{"404 Not Found: {}", path_str};
        let ret = Vec::from(s.as_bytes());
        (Status::NotFound, (ContentType::Plain, ret))
    }
}

#[get("/lookup_window_query/<text>")]
fn lookup_window_query(text: &str) -> Status {
    ffi::callback_run_lookup_query(ffi::QString::from(text));
    Status::Ok
}

#[get("/summary_query/<window_id>/<text>")]
fn summary_query(window_id: &str, text: &str) -> Status {
    ffi::callback_run_summary_query(ffi::QString::from(window_id),
                                    ffi::QString::from(text));
    Status::Ok
}

#[derive(Deserialize)]
struct SuttaMenuRequest {
    window_id: String,
    action: String,
    text: String,
}

#[post("/sutta_menu_action", data = "<request>")]
fn sutta_menu_action(request: Json<SuttaMenuRequest>) -> Status {
    info(&format!("sutta_menu_action(): window_id: {}, action: {}, text len: {}",
                  request.window_id, request.action, request.text.len()));

    ffi::callback_run_sutta_menu_action(ffi::QString::from(&request.window_id),
                                        ffi::QString::from(&request.action),
                                        ffi::QString::from(&request.text));
    Status::Ok
}

#[derive(Deserialize)]
struct LoggerRequest {
    log_level: String,
    msg: String,
}

#[post("/logger", data = "<req>")]
fn logger_route(req: Json<LoggerRequest>) -> Status {
    match req.log_level.as_str() {
        "info" => info(&req.msg),
        "warn" => warn(&req.msg),
        "error" => error(&req.msg),
        "profile" => profile(&req.msg),
        _ => {},
    }
    Status::Ok
}

#[get("/")]
fn index() -> RawHtml<String> {
    let p = get_create_simsapa_app_root().unwrap_or(PathBuf::from("."));
    let app_data_path = p.to_string_lossy();
    let app_data_folder_contents = generate_html_directory_listing(&app_data_path, 3).unwrap_or(String::from("Error"));

    let storage_path = ffi::get_internal_storage_path().to_string();
    let storage_folder_contents = generate_html_directory_listing(&storage_path, 3).unwrap_or(String::from("Error"));

    let html = format!("
<h1>Simsapa Dhamma Reader</h1>
<img src='/assets/icons/simsapa-logo-horizontal-gray-w600.png'>
<p>App data path: {}</p>
<p>Contents:</p>
<pre>{}</pre>
<p>Internal storage path: {}</p>
<p>Contents:</p>
<pre>{}</pre>", app_data_path, app_data_folder_contents, storage_path, storage_folder_contents);

    RawHtml(html_page(&html, None, None, None, None))
}

#[get("/shutdown")]
fn shutdown(shutdown: Shutdown) {
    shutdown.notify();
    info("Webserver shutting down...")
}


#[get("/get_sutta_html_by_uid/<uid..>")]
fn get_sutta_html_by_uid(uid: PathBuf, dbm: &State<Arc<DbManager>>) -> Result<RawHtml<String>, (Status, String)> {
    let uid_str = uid.to_string_lossy();
    info(&format!("get_sutta_html_by_uid(): {}", uid_str));

    match dbm.appdata.get_sutta(&uid_str) {
        Some(sutta) => Ok(RawHtml(format!("<p>Found: {}</p>", &sutta.uid))),
        None => Err((Status::NotFound, format!("Sutta Not Found"))),
    }
}

#[rocket::main]
#[unsafe(no_mangle)]
pub async extern "C" fn start_webserver() {
    info("start_webserver()");
    init_app_globals_api();
    let assets_files: AssetsHandler = AssetsHandler::default();

    let dbm = DbManager::new().expect("Api: Can't create DbManager");
    let db_manager = Arc::new(dbm);
    let g = get_app_globals_api();

    let cors = CorsOptions::default().to_cors().expect("Cors options error");

    let config = rocket::Config::figment()
        .merge(("log_level", rocket::config::LogLevel::Off))
        .merge(("address", "127.0.0.1"))
        .merge(("port", g.api_port));

    let _ = rocket::build()
        .configure(config)
        .attach(cors)
        .mount("/", routes![
            index,
            shutdown,
            serve_assets,
            logger_route,
            lookup_window_query,
            summary_query,
            sutta_menu_action,
            get_sutta_html_by_uid,
        ])
        .manage(assets_files)
        .manage(db_manager)
        .launch().await;
}

#[unsafe(no_mangle)]
pub extern "C" fn shutdown_webserver() {
    let g = get_app_globals_api();
    match ureq::get(format!("{}/shutdown", g.api_url.clone())).call() {
        Ok(mut resp) => {
            match resp.body_mut().read_to_string() {
                Ok(body) => { info(&body); }
                Err(_) => { error("Response error."); }
            }
        },
        Err(ureq::Error::StatusCode(code)) => {
            error(&format!("Error {}", code));
        }
        Err(_) => { error("Error response from webserver shutdown."); }
    }
}

fn create_parent_directory(path: &str) -> String {
    match Path::new(path).parent() {
        None => format!("Invalid path: {}", path),
        Some(parent) => match std::fs::create_dir_all(parent) {
            Ok(_) => String::from(""),
            Err(e) => format!("Failed to create directory: {}", e),
        },
    }
}

fn save_to_file(data: &[u8], path: &str) -> String {
    match File::create(path) {
        Ok(mut file) => match file.write_all(data) {
            Ok(_) => String::from(format!("File saved successfully to {}", path)),
            Err(e) => format!("Failed to write file: {}", e),
        },
        Err(e) => format!("Failed to create file: {}", e),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn download_small_database() {
    let url = "https://github.com/simsapa/simsapa-ng-assets/releases/download/v0.1.0-alpha.1/appdata.sqlite3";
    let p = get_create_simsapa_appdata_db_path();
    let save_path = p.to_string_lossy();

    // Check and create directory
    let dir_error = create_parent_directory(&save_path);
    if !dir_error.is_empty() {
        error(&dir_error);
        return;
    }

    match ureq::get(url).call() {
        Ok(mut response) => {
            if response.status() != http::StatusCode::OK {
                error(&format!("HTTP request failed with status {}", response.status()));
                return;
            }

            // The testing database is small, read it all to memory.
            match response.body_mut().read_to_vec() {
                Ok(buffer) => {
                    let resp = save_to_file(&buffer, &save_path);
                    info(&resp);
                    return;
                },
                Err(e) => {
                    error(&format!("Failed to read to vec: {}", e));
                    return;
                }
            }
        },
        Err(e) => {
            error(&format!("HTTP request failed: {}", e));
            return;
        },
    };
}

#[unsafe(no_mangle)]
pub extern "C" fn shutdown_webserver_tcp() {
    let g = get_app_globals_api();
    match TcpStream::connect(format!("localhost:{}", g.api_port)) {
        Ok(mut connection) => {
            // Set a timeout of 5 seconds
            if let Err(e) = connection.set_read_timeout(Some(Duration::from_secs(5))) {
                error(&format!("Error setting timeout: {}", e));
            }

            // Construct and send the HTTP GET request
            let request = format!("GET /shutdown HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n");
            if let Err(e) = connection.write_all(request.as_bytes()) {
                error(&format!("Error sending request: {}", e));
            }

            // Read the response
            let mut response = String::new();
            if let Err(e) = connection.read_to_string(&mut response) {
                error(&format!("Error reading response: {}", e));
            }

            info(&response);
        }
        Err(e) => {
            error(&format!("Error connecting to server: {}", e));
        }
    }
}
