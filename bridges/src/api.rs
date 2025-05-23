use std::fs::File;
use std::path::{Path, PathBuf};
use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;
use std::sync::Arc;

use http;
use ureq;
use rocket::{get, routes};
use rocket::response::content::RawHtml;
use rocket::Shutdown;
use rocket::State;
use rocket::http::{ContentType, Status};
use rocket_cors::CorsOptions;

use simsapa_backend::{API_PORT, API_URL, get_create_simsapa_app_root, get_create_simsapa_appdata_db_path};
use simsapa_backend::html_content::html_page;
use simsapa_backend::dir_list::generate_html_directory_listing;
use simsapa_backend::db::DbManager;

#[cxx_qt::bridge]
pub mod ffi {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;

        include!("/home/gambhiro/prods/apps/simsapa-ng-project/simsapa-ng/cpp/utils.h");
        fn get_internal_storage_path() -> QString;
        // fn get_app_assets_path() -> QString;

        // FIXME: How to avoid using the full path?
        include!("/home/gambhiro/prods/apps/simsapa-ng-project/simsapa-ng/cpp/gui.h");
        fn callback_run_lookup_query(query_text: QString);
        fn callback_run_summary_query(window_id: QString, query_text: QString);
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
    ffi::callback_run_summary_query(ffi::QString::from(window_id), ffi::QString::from(text));
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

    RawHtml(html_page(&html, None, None, None))
}

#[get("/shutdown")]
fn shutdown(shutdown: Shutdown) {
    shutdown.notify();
    println!("Webserver shutting down...")
}


#[get("/get_sutta_html_by_uid/<uid..>")]
fn get_sutta_html_by_uid(uid: PathBuf, dbm: &State<Arc<DbManager>>) -> Result<RawHtml<String>, (Status, String)> {
    let uid_str = uid.to_string_lossy();
    println!("get_sutta_html_by_uid(): {}", uid_str);

    match dbm.appdata.get_sutta(&uid_str) {
        Some(sutta) => Ok(RawHtml(format!("<p>Found: {}</p>", &sutta.uid))),
        None => Err((Status::NotFound, format!("Sutta Not Found"))),
    }
}

#[rocket::main]
#[unsafe(no_mangle)]
pub async extern "C" fn start_webserver() {
    println!("start_webserver()");
    let assets_files: AssetsHandler = AssetsHandler::default();

    let dbm = DbManager::new().expect("Api: Can't create DbManager");
    let db_manager = Arc::new(dbm);

    let cors = CorsOptions::default().to_cors().expect("Cors options error");

    let config = rocket::Config::figment()
        .merge(("log_level", rocket::config::LogLevel::Off))
        .merge(("address", "127.0.0.1"))
        .merge(("port", API_PORT));

    let _ = rocket::build()
        .configure(config)
        .attach(cors)
        .mount("/", routes![
            index,
            shutdown,
            serve_assets,
            lookup_window_query,
            summary_query,
            get_sutta_html_by_uid,
        ])
        .manage(assets_files)
        .manage(db_manager)
        .launch().await;
}

#[unsafe(no_mangle)]
pub extern "C" fn shutdown_webserver() {
    match ureq::get(format!("{}/shutdown", API_URL)).call() {
        Ok(mut resp) => {
            match resp.body_mut().read_to_string() {
                Ok(body) => { println!("{}", body); }
                Err(_) => { println!("Response error."); }
            }
        },
        Err(ureq::Error::StatusCode(code)) => {
            println!("Error {}", code);
        }
        Err(_) => { println!("Error response from webserver shutdown."); }
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
        eprintln!("{}", dir_error);
        return;
    }

    match ureq::get(url).call() {
        Ok(mut response) => {
            if response.status() != http::StatusCode::OK {
                eprintln!("HTTP request failed with status {}", response.status());
                return;
            }

            // The testing database is small, read it all to memory.
            match response.body_mut().read_to_vec() {
                Ok(buffer) => {
                    let resp = save_to_file(&buffer, &save_path);
                    println!("{}", resp);
                    return;
                },
                Err(e) => {
                    eprintln!("Failed to read to vec: {}", e);
                    return;
                }
            }
        },
        Err(e) => {
            eprintln!("HTTP request failed: {}", e);
            return;
        },
    };
}

#[unsafe(no_mangle)]
pub extern "C" fn shutdown_webserver_tcp() {
    match TcpStream::connect(format!("localhost:{}", API_PORT)) {
        Ok(mut connection) => {
            // Set a timeout of 5 seconds
            if let Err(e) = connection.set_read_timeout(Some(Duration::from_secs(5))) {
                eprintln!("Error setting timeout: {}", e);
            }

            // Construct and send the HTTP GET request
            let request = format!("GET /shutdown HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n");
            if let Err(e) = connection.write_all(request.as_bytes()) {
                eprintln!("Error sending request: {}", e);
            }

            // Read the response
            let mut response = String::new();
            if let Err(e) = connection.read_to_string(&mut response) {
                eprintln!("Error reading response: {}", e);
            }

            println!("{}", response);
        }
        Err(e) => {
            eprintln!("Error connecting to server: {}", e);
        }
    }
}
