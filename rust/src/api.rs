use std::path::PathBuf;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;

use ureq;
use rocket::{get, routes};
use rocket::response::content;
use rocket::Shutdown;
use rocket::State;
use rocket::http::{Status, ContentType};
use rocket_cors::CorsOptions;

use ffi::get_internal_storage_path;
use crate::{API_PORT, API_URL};
use crate::html_content::html_page;
use crate::dir_list::generate_html_directory_listing;

#[cxx_qt::bridge]
pub mod ffi {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;

        // FIXME: How to avoid using the full path?
        include!("/home/gambhiro/prods/apps/simsapa-project/simsapa-cxx-qt/simsapa/cpp/utils.h");
        fn get_internal_storage_path() -> QString;
        fn get_app_assets_path() -> QString;

        include!("/home/gambhiro/prods/apps/simsapa-project/simsapa-cxx-qt/simsapa/cpp/gui.h");
        fn callback_run_lookup_query(query_text: QString);
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

#[get("/lookup_window_query/<word>")]
fn lookup_window_query(word: &str) -> Status {
    ffi::callback_run_lookup_query(ffi::QString::from(word));
    Status::Ok
}

#[get("/")]
fn index() -> content::RawHtml<String> {
    let storage_path = get_internal_storage_path().to_string();

    let folder_contents = generate_html_directory_listing(&storage_path, 3).unwrap_or(String::from("Error"));

    let html = format!("
<h1>Simsapa Dhamma Reader</h1>
<img src='/assets/icons/simsapa-logo-horizontal-gray-w600.png'>
<p>Internal storage path: {}</p>
<p>Contents:</p>
<pre>{}</pre>", storage_path, folder_contents);

    content::RawHtml(html_page(&html, None, None, None))
}

#[get("/shutdown")]
fn shutdown(shutdown: Shutdown) {
    shutdown.notify();
    println!("Webserver shutting down...")
}

#[rocket::main]
#[unsafe(no_mangle)]
pub async extern "C" fn start_webserver() {
    let assets_files: AssetsHandler = AssetsHandler::default();

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
        ])
        .manage(assets_files)
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
