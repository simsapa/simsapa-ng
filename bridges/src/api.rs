use std::path::PathBuf;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;
use std::sync::{OnceLock, Arc};

use rocket::serde::{Deserialize, Serialize};
use rocket::serde::json::Json;

use http;
use ureq;
use rocket::{get, post, routes, State, Shutdown};
use rocket::response::content::RawHtml;
use rocket::http::{ContentType, Status};
use rocket_cors::CorsOptions;

use simsapa_backend::{AppGlobals, get_app_data, get_create_simsapa_dir, get_create_simsapa_appdata_db_path, save_to_file, create_parent_directory};
use simsapa_backend::html_content::sutta_html_page;
use simsapa_backend::dir_list::generate_html_directory_listing;
use simsapa_backend::db::DbManager;
use simsapa_backend::helpers::{create_or_update_linux_desktop_icon_file, query_text_to_uid_field_query};
use simsapa_backend::logger::{info, warn, error, profile};
use simsapa_backend::types::{SearchResult, SearchParams, SearchMode, SearchArea};
use simsapa_backend::query_task::SearchQueryTask;

// ============================================================================
// Browser Extension API Data Structures
// ============================================================================

/// Response structure for search endpoints (suttas and dictionary)
/// Matches the Python `ApiSearchResult` TypedDict for browser extension compatibility
#[derive(Debug, Clone, Serialize)]
pub struct ApiSearchResult {
    pub hits: i32,
    pub results: Vec<SearchResult>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deconstructor: Option<Vec<String>>,
}

/// Request body for POST search endpoints
#[derive(Debug, Clone, Deserialize)]
pub struct ApiSearchRequest {
    pub query_text: String,
    pub page_num: Option<i32>,
    pub suttas_lang: Option<String>,
    pub suttas_lang_include: Option<bool>,
    pub dict_lang: Option<String>,
    pub dict_lang_include: Option<bool>,
    pub dict_dict: Option<String>,
    pub dict_dict_include: Option<bool>,
}

/// Response structure for /sutta_and_dict_search_options endpoint
#[derive(Debug, Clone, Serialize)]
pub struct SearchOptions {
    pub sutta_languages: Vec<String>,
    pub dict_languages: Vec<String>,
    pub dict_sources: Vec<String>,
}

/// Request body for POST /lookup_window_query endpoint
#[derive(Debug, Clone, Deserialize)]
pub struct LookupWindowRequest {
    pub query_text: String,
}

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

/// Convert a PathBuf to a string using forward slashes as separators.
/// On Windows, PathBuf uses backslashes, but URLs and database paths use forward slashes.
/// This ensures consistent path handling across all platforms.
fn pathbuf_to_forward_slash_string(path: &PathBuf) -> String {
    path.iter()
        .map(|s| s.to_str().unwrap_or(""))
        .collect::<Vec<_>>()
        .join("/")
}

#[cxx_qt::bridge]
pub mod ffi {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;

        include!("utils.h");
        fn get_internal_storage_path() -> QString;
        fn get_status_bar_height() -> i32;
        // fn get_app_assets_path() -> QString;

        include!("gui.h");
        fn callback_run_lookup_query(query_text: QString);
        fn callback_run_summary_query(window_id: QString, query_text: QString);
        fn callback_run_sutta_menu_action(window_id: QString, action: QString, query_text: QString);
        fn callback_open_sutta_search_window(show_result_data_json: QString);
        fn callback_open_sutta_tab(window_id: QString, show_result_data_json: QString);
        fn callback_open_sutta_languages_window();
        fn callback_open_library_window();
        fn callback_open_reference_search_window();
        fn callback_show_chapter_in_sutta_window(window_id: QString, result_data_json: QString);
        fn callback_show_sutta_from_reference_search(window_id: QString, result_data_json: QString);
        fn callback_toggle_reading_mode(window_id: QString, is_active: bool);
        fn callback_open_in_lookup_window(result_data_json: QString);
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
    // Convert path to forward slashes for cross-platform consistency
    let path_str = pathbuf_to_forward_slash_string(&path);
    // Also log the raw PathBuf for debugging Windows path issues
    // info(&format!("serve_assets: path_str='{}', raw_path='{:?}'", path_str, path));

    let some_entry = assets.files.get_entry(&path_str);

    if let Some(entry) = some_entry {
        if let Some(entry_file) = entry.as_file() {

            let p = PathBuf::from(&path_str);
            let path_ext = match p.extension() {
                Some(s) => s.to_str().unwrap_or("txt"),
                None => "txt",
            };

            let content_type = match path_ext {
                "css" => ContentType::CSS,
                "js" | "mjs" => ContentType::JavaScript,
                "json" => ContentType::JSON,
                "svg" => ContentType::SVG,
                "png" => ContentType::PNG,
                "jpg" | "jpeg" => ContentType::JPEG,
                "gif" => ContentType::GIF,
                "woff" | "woff2" => ContentType::WOFF,
                "ttf" => ContentType::TTF,
                "otf" => ContentType::OTF,
                "html" | "htm" => ContentType::HTML,
                "wasm" => ContentType::WASM,
                "pdf" => ContentType::PDF,
                "map" => ContentType::JSON, // Source maps
                _ => ContentType::from_extension(path_ext).unwrap_or(ContentType::Plain),
            };

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

#[get("/toggle_reading_mode/<window_id>/<is_active>")]
fn toggle_reading_mode(window_id: &str, is_active: &str) -> Status {
    let active = is_active == "true";
    // info(&format!("toggle_reading_mode(): window_id: {}, is_active: {}", window_id, active));
    ffi::callback_toggle_reading_mode(ffi::QString::from(window_id), active);
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
struct BookPageRequest {
    book_page_url: String,
}

#[post("/open_book_page_tab/<window_id>", data = "<request>")]
fn open_book_page_tab(window_id: &str, request: Json<BookPageRequest>, dbm: &State<Arc<DbManager>>) -> Status {
    let book_page_url = &request.book_page_url;
    info(&format!("open_book_page_tab(): window_id: {}, url: {}", window_id, book_page_url));

    // Parse the URL to extract book_uid and resource_path
    // Format: /book_pages/<book_uid>/<resource_path>
    if !book_page_url.starts_with("/book_pages/") {
        error(&format!("Invalid book page URL format: {}", book_page_url));
        return Status::BadRequest;
    }

    let path_part = &book_page_url[12..]; // Remove "/book_pages/" prefix
    let parts: Vec<&str> = path_part.splitn(2, '/').collect();

    if parts.len() < 2 {
        error(&format!("Invalid book page URL format: {}", book_page_url));
        return Status::BadRequest;
    }

    let book_uid = parts[0];

    // Extract anchor fragment if present
    let path_and_anchor: Vec<&str> = parts[1].splitn(2, '#').collect();
    let resource_path = path_and_anchor[0];
    let anchor = if path_and_anchor.len() > 1 {
        path_and_anchor[1]
    } else {
        ""
    };

    // info(&format!("Parsed: book_uid={}, resource_path={}, anchor={}", book_uid, resource_path, anchor));

    // Get the book spine item
    let item = match dbm.appdata.get_book_spine_item_by_path(book_uid, resource_path) {
        Ok(Some(item)) => item,
        Ok(None) => {
            error(&format!("BookSpineItem not found: {}/{}", book_uid, resource_path));
            return Status::NotFound;
        }
        Err(e) => {
            error(&format!("Database error: {}", e));
            return Status::InternalServerError;
        }
    };

    // Compose the result data JSON with anchor
    let result_data_json = serde_json::json!({
        "item_uid": item.spine_item_uid,
        "table_name": "book_spine_items",
        "sutta_title": item.title.unwrap_or_default(),
        "sutta_ref": "",
        "snippet": "",
        "anchor": anchor,
    });

    let json_string = serde_json::to_string(&result_data_json).unwrap_or_default();
    ffi::callback_open_sutta_tab(ffi::QString::from(window_id), ffi::QString::from(json_string));
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

#[derive(Deserialize)]
struct CopyToClipboardRequest {
    text: String,
}

#[post("/copy_to_clipboard", data = "<req>")]
fn copy_to_clipboard(req: Json<CopyToClipboardRequest>) -> Status {
    info(&format!("copy_to_clipboard(): text length: {}", req.text.len()));
    let text_qstring = ffi::QString::from(&req.text);
    let mime_qstring = ffi::QString::from("text/plain");
    crate::clipboard_manager::qobject::copy_with_mime_type_impl(&text_qstring, &mime_qstring);
    Status::Ok
}

#[derive(Deserialize)]
struct OpenExternalUrlRequest {
    url: String,
}

#[post("/open_external_url", data = "<req>")]
fn open_external_url(req: Json<OpenExternalUrlRequest>) -> Status {
    info(&format!("open_external_url(): {}", req.url));
    let url_qstring = ffi::QString::from(&req.url);
    let success = crate::clipboard_manager::qobject::open_external_url_impl(&url_qstring);
    if success {
        Status::Ok
    } else {
        Status::InternalServerError
    }
}

#[get("/app-assets-list")]
fn app_assets_list() -> RawHtml<String> {
    let p = get_create_simsapa_dir().unwrap_or(PathBuf::from("."));
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

    RawHtml(sutta_html_page(&html, None, None, None, None))
}

#[get("/")]
fn index() -> RawHtml<String> {
    let html = r#"
<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <title>Simsapa Dhamma Reader</title>
</head>
<body>
  <h1>Simsapa Dhamma Reader</h1>
</body>
</html>
"#.to_string();

    RawHtml(html)
}

#[get("/shutdown")]
fn shutdown(shutdown: Shutdown) {
    shutdown.notify();
    info("Webserver shutting down...")
}


#[get("/get_sutta_html_by_uid/<window_id>/<uid..>")]
fn get_sutta_html_by_uid(window_id: &str, uid: PathBuf, dbm: &State<Arc<DbManager>>) -> Result<RawHtml<String>, (Status, String)> {
    // Convert path to forward slashes for cross-platform consistency
    let uid_str = pathbuf_to_forward_slash_string(&uid);
    info(&format!("get_sutta_html_by_uid(): window_id: {}, uid: {}", window_id, uid_str));

    let app_data = get_app_data();

    match dbm.appdata.get_sutta(&uid_str) {
        Some(sutta) => {
            // Render the sutta with WINDOW_ID in the JavaScript
            let js_extra = format!("const WINDOW_ID = '{}'; window.WINDOW_ID = WINDOW_ID;", window_id);
            match app_data.render_sutta_content(&sutta, None, Some(js_extra)) {
                Ok(html) => Ok(RawHtml(html)),
                Err(e) => Err((Status::InternalServerError, format!("Rendering error: {}", e))),
            }
        },
        None => Err((Status::NotFound, format!("Sutta Not Found: {}", &uid_str))),
    }
}

#[get("/get_book_spine_item_html_by_uid/<window_id>/<spine_item_uid..>")]
fn get_book_spine_item_html_by_uid(window_id: &str, spine_item_uid: PathBuf, dbm: &State<Arc<DbManager>>) -> Result<RawHtml<String>, (Status, String)> {
    // Convert path to forward slashes for cross-platform consistency
    let uid_str = pathbuf_to_forward_slash_string(&spine_item_uid);
    info(&format!("get_book_spine_item_html_by_uid(): {}", uid_str));

    let item = match dbm.appdata.get_book_spine_item(&uid_str) {
        Ok(Some(item)) => item,
        Ok(None) => return Err((Status::NotFound, format!("BookSpineItem Not Found"))),
        Err(e) => return Err((Status::InternalServerError, format!("Database error: {}", e))),
    };

    let app_data = get_app_data();
    if let Ok(html) = app_data.render_book_spine_item_html(&item, Some(window_id.to_string()), None) {
        Ok(RawHtml(html))
    } else {
        Err((Status::InternalServerError, "HTML rendering error".to_string()))
    }
}

/// Serve PDF viewer page for a PDF book - for browser testing
/// URL: /get_pdf_viewer/<book_uid>
/// This generates the same URL that the QML view would load
#[get("/get_pdf_viewer/<book_uid>")]
fn get_pdf_viewer(book_uid: &str) -> RawHtml<String> {
    let g = get_app_globals_api();
    let api_url = format!("http://localhost:{}", g.api_port);
    let pdf_url = format!("{}/book_resources/{}/document.pdf", api_url, book_uid);
    // URL encode the pdf_url for use as query parameter
    let encoded_pdf_url = pdf_url.replace(":", "%3A").replace("/", "%2F");
    let viewer_url = format!("{}/assets/pdf-viewer/web/viewer.html?file={}", api_url, encoded_pdf_url);

    // Return a simple redirect page
    let html = format!(r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <title>PDF Viewer - {}</title>
    <meta http-equiv="refresh" content="0; url={}">
</head>
<body>
    <p>Loading PDF viewer...</p>
    <p>If not redirected, <a href="{}">click here</a></p>
    <p>Debug info:</p>
    <ul>
        <li>Book UID: {}</li>
        <li>PDF URL: {}</li>
        <li>Viewer URL: {}</li>
    </ul>
</body>
</html>"#, book_uid, viewer_url, viewer_url, book_uid, pdf_url, viewer_url);

    RawHtml(html)
}

#[get("/book_pages/<book_uid>/<resource_path..>")]
fn get_book_page_by_path(book_uid: &str, resource_path: PathBuf, dbm: &State<Arc<DbManager>>) -> Result<RawHtml<String>, (Status, String)> {
    // Convert path to forward slashes for cross-platform consistency
    let resource_path_str = pathbuf_to_forward_slash_string(&resource_path);
    info(&format!("get_book_page_by_path(): {}/{}", book_uid, resource_path_str));

    let item = match dbm.appdata.get_book_spine_item_by_path(book_uid, &resource_path_str) {
        Ok(Some(item)) => item,
        Ok(None) => return Err((Status::NotFound, format!("BookSpineItem Not Found for path: {}", resource_path_str))),
        Err(e) => return Err((Status::InternalServerError, format!("Database error: {}", e))),
    };

    let app_data = get_app_data();
    if let Ok(html) = app_data.render_book_spine_item_html(&item, None, None) {
        Ok(RawHtml(html))
    } else {
        Err((Status::InternalServerError, "HTML rendering error".to_string()))
    }
}

#[get("/open_sutta_window/<uid..>")]
fn open_sutta_window(uid: PathBuf, dbm: &State<Arc<DbManager>>) -> Status {
    // Convert path to forward slashes for cross-platform consistency
    let uid_str = pathbuf_to_forward_slash_string(&uid);
    info(&format!("open_sutta(): {}", uid_str));

    // Try to get sutta with original UID
    let sutta_option = dbm.appdata.get_sutta(&uid_str);

    // If not found and not already pli/ms, try fallback
    let final_sutta = if sutta_option.is_none() && !uid_str.ends_with("/pli/ms") {
        // Extract code (e.g., "sn47.8" from "sn47.8/en/thanissaro")
        let code = uid_str.split('/').next().unwrap_or(&uid_str);
        let fallback_uid = format!("{}/pli/ms", code);

        // Try to get fallback sutta
        if let Some(fallback_sutta) = dbm.appdata.get_sutta(&fallback_uid) {
            info(&format!("open_sutta(): Using fallback UID: {}", fallback_uid));
            Some(fallback_sutta)
        } else {
            None
        }
    } else {
        sutta_option
    };

    // If sutta is found, compose JSON and call callback
    if let Some(sutta) = final_sutta {
        let result_data_json = serde_json::json!({
            "item_uid": sutta.uid,
            "table_name": "suttas",
            "sutta_title": sutta.title,
            "sutta_ref": sutta.sutta_ref,
            "snippet": "",
        });

        let json_string = serde_json::to_string(&result_data_json).unwrap_or_default();
        ffi::callback_open_sutta_search_window(ffi::QString::from(json_string));
        Status::Ok
    } else {
        // Sutta not found - return 404 so frontend can show error dialog
        error(&format!("Sutta not found: {}", uid_str));
        Status::NotFound
    }
}

#[get("/open_sutta_tab/<window_id>/<uid..>")]
fn open_sutta_tab(window_id: &str, uid: PathBuf, dbm: &State<Arc<DbManager>>) -> Status {
    // Convert path to forward slashes for cross-platform consistency
    let uid_str = pathbuf_to_forward_slash_string(&uid);
    info(&format!("open_sutta_tab(): window_id: {}, uid: {}", window_id, uid_str));

    // Try to get sutta with original UID
    let sutta_option = dbm.appdata.get_sutta(&uid_str);

    // If not found and not already pli/ms, try fallback
    let final_sutta = if sutta_option.is_none() && !uid_str.ends_with("/pli/ms") {
        // Extract code (e.g., "sn47.8" from "sn47.8/en/thanissaro")
        let code = uid_str.split('/').next().unwrap_or(&uid_str);
        let fallback_uid = format!("{}/pli/ms", code);

        // Try to get fallback sutta
        if let Some(fallback_sutta) = dbm.appdata.get_sutta(&fallback_uid) {
            info(&format!("open_sutta_tab(): Using fallback UID: {}", fallback_uid));
            Some(fallback_sutta)
        } else {
            None
        }
    } else {
        sutta_option
    };

    // If sutta is found, compose JSON and call callback
    if let Some(sutta) = final_sutta {
        let result_data_json = serde_json::json!({
            "item_uid": sutta.uid,
            "table_name": "suttas",
            "sutta_title": sutta.title,
            "sutta_ref": sutta.sutta_ref,
            "snippet": "",
        });

        let json_string = serde_json::to_string(&result_data_json).unwrap_or_default();
        ffi::callback_open_sutta_tab(ffi::QString::from(window_id), ffi::QString::from(json_string));
        Status::Ok
    } else {
        // Sutta not found - return 404 so frontend can show error dialog
        error(&format!("Sutta not found: {}", uid_str));
        Status::NotFound
    }
}

/// Serve book resources (images, CSS, PDFs, etc.) from the database
#[get("/book_resources/<book_uid>/<path..>")]
fn serve_book_resources(book_uid: &str, path: PathBuf, db_manager: &State<Arc<DbManager>>) -> (Status, (ContentType, Vec<u8>)) {
    // Convert path to forward slashes for cross-platform consistency
    let path_str = pathbuf_to_forward_slash_string(&path);
    info(&format!("serve_book_resources: book_uid={}, path={}", book_uid, path_str));

    // Query the database for the resource
    match db_manager.appdata.get_book_resource(book_uid, &path_str) {
        Ok(Some(resource)) => {
            // Determine ContentType from MIME type
            let content_type = if let Some(ref mime) = resource.mime_type {
                match mime.as_str() {
                    "image/png" => ContentType::PNG,
                    "image/jpeg" | "image/jpg" => ContentType::JPEG,
                    "image/gif" => ContentType::GIF,
                    "image/svg+xml" => ContentType::SVG,
                    "image/webp" => ContentType::WEBP,
                    "text/css" => ContentType::CSS,
                    "application/javascript" | "text/javascript" => ContentType::JavaScript,
                    "application/pdf" => ContentType::PDF,
                    "font/woff" | "font/woff2" => ContentType::WOFF,
                    "font/ttf" => ContentType::TTF,
                    "font/otf" => ContentType::OTF,
                    _ => ContentType::Binary,
                }
            } else {
                ContentType::Binary
            };

            // Return the resource data
            let data = resource.content_data.unwrap_or_default();
            (Status::Ok, (content_type, data))
        }
        Ok(None) => {
            // Resource not found
            let msg = format!("404 Not Found: /book_resources/{}/{}", book_uid, path_str);
            warn(&msg);
            let ret = Vec::from(msg.as_bytes());
            (Status::NotFound, (ContentType::Plain, ret))
        }
        Err(e) => {
            // Database error
            let msg = format!("500 Internal Server Error: {}", e);
            error(&msg);
            let ret = Vec::from(msg.as_bytes());
            (Status::InternalServerError, (ContentType::Plain, ret))
        }
    }
}

// ============================================================================
// Browser Extension API Routes
// ============================================================================

/// GET /sutta_and_dict_search_options
/// Returns available filter options for sutta and dictionary searches
#[get("/sutta_and_dict_search_options")]
fn get_search_options(dbm: &State<Arc<DbManager>>) -> Json<SearchOptions> {
    let sutta_languages = dbm.appdata.get_sutta_languages();
    let dict_languages = dbm.dictionaries.get_distinct_languages();
    let dict_sources = dbm.dictionaries.get_distinct_sources();

    Json(SearchOptions {
        sutta_languages,
        dict_languages,
        dict_sources,
    })
}

/// POST /suttas_fulltext_search
/// Search suttas using ContainsMatch (placeholder for fulltext search)
#[post("/suttas_fulltext_search", data = "<request>")]
fn suttas_fulltext_search(request: Json<ApiSearchRequest>, dbm: &State<Arc<DbManager>>) -> Json<ApiSearchResult> {
    let query_text_orig = request.query_text.clone();
    let page_num = request.page_num.unwrap_or(0) as usize;

    // Build language filter - only apply if not "Languages" (the default placeholder)
    let lang_filter = match &request.suttas_lang {
        Some(lang) if lang != "Languages" && lang != "Language" && !lang.is_empty() => Some(lang.clone()),
        _ => None,
    };
    let lang_include = request.suttas_lang_include.unwrap_or(true);

    // Check if query is a sutta reference pattern (e.g., "sn56.11", "MN 44", "dhp182")
    // query_text_to_uid_field_query returns "uid:..." if it's a UID/reference pattern
    let uid_query = query_text_to_uid_field_query(&query_text_orig);
    let (query_text, search_mode) = if uid_query.starts_with("uid:") {
        (uid_query, SearchMode::UidMatch)
    } else {
        (query_text_orig.clone(), SearchMode::ContainsMatch)
    };

    info(&format!("suttas_fulltext_search(): query='{}', page={}, lang={:?}, include={}, mode={:?}",
                  query_text, page_num, lang_filter, lang_include, search_mode));

    // Create search params - use UidMatch for reference patterns, ContainsMatch otherwise
    let params = SearchParams {
        mode: search_mode,
        page_len: Some(20), // Browser extension uses 20 results per page
        lang: lang_filter,
        lang_include,
        source: None,
        source_include: true,
        enable_regex: false,
        fuzzy_distance: 0,
    };

    // Create and execute search task
    let mut search_task = SearchQueryTask::new(
        dbm.inner(),
        query_text,
        params,
        SearchArea::Suttas,
    );

    match search_task.results_page(page_num) {
        Ok(results) => {
            let hits = search_task.total_hits() as i32;
            Json(ApiSearchResult {
                hits,
                results,
                deconstructor: None, // Not applicable for sutta search
            })
        }
        Err(e) => {
            error(&format!("suttas_fulltext_search error: {}", e));
            Json(ApiSearchResult {
                hits: 0,
                results: Vec::new(),
                deconstructor: None,
            })
        }
    }
}

/// POST /dict_combined_search
/// Search dictionary words with language and source filtering, includes deconstructor results
#[post("/dict_combined_search", data = "<request>")]
fn dict_combined_search(request: Json<ApiSearchRequest>, dbm: &State<Arc<DbManager>>) -> Json<ApiSearchResult> {
    let query_text_orig = request.query_text.clone();
    let page_num = request.page_num.unwrap_or(0) as usize;

    // Build language filter - only apply if not "Languages" (the default placeholder)
    let lang_filter = match &request.dict_lang {
        Some(lang) if lang != "Languages" && lang != "Language" && !lang.is_empty() => Some(lang.clone()),
        _ => None,
    };
    let lang_include = request.dict_lang_include.unwrap_or(true);

    // Build source filter - only apply if not "Dictionaries" (the default placeholder)
    let source_filter = match &request.dict_dict {
        Some(source) if source != "Dictionaries" && source != "Dictionary" && !source.is_empty() => Some(source.clone()),
        _ => None,
    };
    let source_include = request.dict_dict_include.unwrap_or(true);

    // Check if query is a UID pattern (e.g., "dhamma 1.01", "dhamma 1.01/dpd", "123/dpd")
    // query_text_to_uid_field_query returns "uid:..." if it's a UID pattern
    let uid_query = query_text_to_uid_field_query(&query_text_orig);
    let (query_text, search_mode) = if uid_query.starts_with("uid:") {
        (uid_query, SearchMode::UidMatch)
    } else {
        // Use DpdLookup as the default mode for dictionary search (same as SuttaSearchWindow QML)
        // This searches DPD headwords by lemma rather than doing a broad contains search
        (query_text_orig.clone(), SearchMode::DpdLookup)
    };

    info(&format!("dict_combined_search(): query='{}', page={}, lang={:?}, source={:?}, mode={:?}",
                  query_text, page_num, lang_filter, source_filter, search_mode));

    // Get deconstructor results for the original query (not the uid: prefixed version)
    let deconstructor_results = dbm.dpd.dpd_deconstructor_list(&query_text_orig);
    let deconstructor = if deconstructor_results.is_empty() {
        None
    } else {
        Some(deconstructor_results)
    };

    // Create search params - use UidMatch for UID patterns, ContainsMatch otherwise
    let params = SearchParams {
        mode: search_mode,
        page_len: Some(20), // Browser extension uses 20 results per page
        lang: lang_filter,
        lang_include,
        source: source_filter,
        source_include,
        enable_regex: false,
        fuzzy_distance: 0,
    };

    // Create and execute search task
    let mut search_task = SearchQueryTask::new(
        dbm.inner(),
        query_text,
        params,
        SearchArea::Dictionary,
    );

    match search_task.results_page(page_num) {
        Ok(results) => {
            let hits = search_task.total_hits() as i32;
            Json(ApiSearchResult {
                hits,
                results,
                deconstructor,
            })
        }
        Err(e) => {
            error(&format!("dict_combined_search error: {}", e));
            Json(ApiSearchResult {
                hits: 0,
                results: Vec::new(),
                deconstructor,
            })
        }
    }
}

/// GET /suttas/<uid>
/// Open a sutta in the Simsapa application window (browser extension route)
/// Returns plain text message for the browser tab
#[get("/suttas/<uid..>")]
fn open_sutta_by_uid(uid: PathBuf, dbm: &State<Arc<DbManager>>) -> (Status, String) {
    // Convert path to forward slashes for cross-platform consistency
    let uid_str = pathbuf_to_forward_slash_string(&uid);
    info(&format!("open_sutta_by_uid(): {}", uid_str));

    // Try to get sutta with original UID
    let sutta_option = dbm.appdata.get_sutta(&uid_str);

    // If not found and not already pli/ms, try fallback
    let final_sutta = if sutta_option.is_none() && !uid_str.ends_with("/pli/ms") {
        // Extract code (e.g., "sn47.8" from "sn47.8/en/thanissaro")
        let code = uid_str.split('/').next().unwrap_or(&uid_str);
        let fallback_uid = format!("{}/pli/ms", code);

        // Try to get fallback sutta
        if let Some(fallback_sutta) = dbm.appdata.get_sutta(&fallback_uid) {
            info(&format!("open_sutta_by_uid(): Using fallback UID: {}", fallback_uid));
            Some(fallback_sutta)
        } else {
            None
        }
    } else {
        sutta_option
    };

    // If sutta is found, compose JSON and call callback
    if let Some(sutta) = final_sutta {
        let result_data_json = serde_json::json!({
            "item_uid": sutta.uid,
            "table_name": "suttas",
            "sutta_title": sutta.title,
            "sutta_ref": sutta.sutta_ref,
            "snippet": "",
        });

        let json_string = serde_json::to_string(&result_data_json).unwrap_or_default();
        // Use the dedicated lookup window for browser extension requests
        ffi::callback_open_in_lookup_window(ffi::QString::from(json_string));

        // Return plain text response for the browser tab
        (Status::Ok, format!("The Simsapa window should appear with '{}'. You can close this tab.", uid_str))
    } else {
        // Sutta not found
        error(&format!("Sutta not found: {}", uid_str));
        (Status::NotFound, format!("Sutta not found: {}", uid_str))
    }
}

/// POST /lookup_window_query
/// Open the word lookup window and search for a word (browser extension route)
/// If query_text is a UID (contains '/'), look up the word directly
/// Otherwise, run a search query
#[post("/lookup_window_query", data = "<request>")]
fn lookup_window_query_post(request: Json<LookupWindowRequest>, dbm: &State<Arc<DbManager>>) -> Status {
    let query_text = &request.query_text;
    info(&format!("lookup_window_query_post(): {}", query_text));

    // Check if this is a UID (contains '/') - e.g., "dhamma 1.01/dpd" or "buddhadhamma/dpd"
    if query_text.contains('/') {
        // Try to look up as a dictionary word UID
        if let Some(dict_word) = dbm.dictionaries.get_word(query_text) {
            let result_data_json = serde_json::json!({
                "item_uid": dict_word.uid,
                "table_name": "dict_words",
                "sutta_title": dict_word.word,
                "sutta_ref": "",
                "snippet": dict_word.definition_plain.unwrap_or_default(),
            });

            let json_string = serde_json::to_string(&result_data_json).unwrap_or_default();
            ffi::callback_open_in_lookup_window(ffi::QString::from(json_string));
            return Status::Ok;
        }

        // If not found in dict_words, try DPD headwords (for numeric UIDs like "34626/dpd")
        let app_data = get_app_data();
        if query_text.ends_with("/dpd") {
            if let Some(json_str) = app_data.get_dpd_headword_by_uid(query_text) {
                if let Ok(headword) = serde_json::from_str::<serde_json::Value>(&json_str) {
                    let word_title = headword.get("lemma_1")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    let meaning = headword.get("meaning_1")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");

                    let result_data_json = serde_json::json!({
                        "item_uid": query_text,
                        "table_name": "dpd_headwords",
                        "sutta_title": word_title,
                        "sutta_ref": "",
                        "snippet": meaning,
                    });

                    let json_string = serde_json::to_string(&result_data_json).unwrap_or_default();
                    ffi::callback_open_in_lookup_window(ffi::QString::from(json_string));
                    return Status::Ok;
                }
            }
        }

        // UID not found, fall through to search query
        info(&format!("lookup_window_query_post(): UID not found, running search: {}", query_text));
    }

    // Not a UID or UID not found - run a search query
    ffi::callback_run_lookup_query(ffi::QString::from(query_text.as_str()));
    Status::Ok
}

/// GET /words/<uid>.json
/// Get full dictionary word data as JSON for copying glossary information
/// The .json extension is part of the path parameter
#[get("/words/<uid_with_ext..>")]
fn get_word_json(uid_with_ext: PathBuf, dbm: &State<Arc<DbManager>>) -> Json<Vec<serde_json::Value>> {
    // Convert path to forward slashes and remove .json extension
    let uid_str = pathbuf_to_forward_slash_string(&uid_with_ext);
    let uid = uid_str.trim_end_matches(".json");

    info(&format!("get_word_json(): uid={}", uid));

    let app_data = get_app_data();

    // Determine word type based on UID pattern:
    // - DPD headwords: end with "/dpd" (e.g., "dhamma 1/dpd" or numeric id patterns)
    // - DPD roots: contain "roots/" pattern (e.g., "√kar/dpd")
    // - dict_words: everything else (e.g., "dhamma/ncped")

    if uid.ends_with("/dpd") {
        // Try DPD headword first (uses numeric IDs like "34626/dpd")
        if let Some(json_str) = app_data.get_dpd_headword_by_uid(uid) {
            if let Ok(value) = serde_json::from_str::<serde_json::Value>(&json_str) {
                return Json(vec![value]);
            }
        }

        // If not found as headword, try as root (roots have format like "√kar/dpd")
        // Extract root key by removing "/dpd" suffix
        let root_key = uid.trim_end_matches("/dpd");
        if let Some(json_str) = app_data.get_dpd_root_by_root_key(root_key) {
            if let Ok(value) = serde_json::from_str::<serde_json::Value>(&json_str) {
                return Json(vec![value]);
            }
        }

        // If not found in dpd.sqlite3, try dict_words table in dictionaries.sqlite3
        // This handles UIDs like "dhamma 1.01/dpd" which are stored in dict_words
        if let Some(dict_word) = dbm.dictionaries.get_word(uid) {
            if let Ok(value) = serde_json::to_value(&dict_word) {
                return Json(vec![value]);
            }
        }
    } else {
        // Try dict_words table for non-DPD entries (e.g., "dhamma/ncped")
        if let Some(dict_word) = dbm.dictionaries.get_word(uid) {
            if let Ok(value) = serde_json::to_value(&dict_word) {
                return Json(vec![value]);
            }
        }
    }

    // Word not found - return empty array
    Json(Vec::new())
}

/// GET /sutta_titles_flat_completion_list
/// Returns list of sutta titles for autocomplete (placeholder - returns empty array)
/// TODO: Future implementation should query sutta titles from database with Pali sort order
#[get("/sutta_titles_flat_completion_list")]
fn sutta_titles_completion() -> Json<Vec<String>> {
    // Placeholder: return empty array
    // Future implementation could query sutta titles sorted by Pali order
    Json(Vec::new())
}

/// GET /dict_words_flat_completion_list
/// Returns list of dictionary words for autocomplete (placeholder - returns empty array)
/// TODO: Future implementation could query DPD lemmas and roots
/// Note: The browser extension currently loads this from a bundled JSON file
#[get("/dict_words_flat_completion_list")]
fn dict_words_completion() -> Json<Vec<String>> {
    // Placeholder: return empty array
    // The browser extension has a fallback bundled word list
    Json(Vec::new())
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
            app_assets_list,
            serve_assets,
            serve_book_resources,
            logger_route,
            copy_to_clipboard,
            open_external_url,
            lookup_window_query,
            summary_query,
            toggle_reading_mode,
            sutta_menu_action,
            get_sutta_html_by_uid,
            get_book_spine_item_html_by_uid,
            get_pdf_viewer,
            get_book_page_by_path,
            open_sutta_window,
            open_sutta_tab,
            open_book_page_tab,
            // Browser Extension API routes
            get_search_options,
            suttas_fulltext_search,
            dict_combined_search,
            open_sutta_by_uid,
            lookup_window_query_post,
            get_word_json,
            sutta_titles_completion,
            dict_words_completion,
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

#[unsafe(no_mangle)]
pub extern "C" fn create_linux_desktop_icon_file() {
    if let Err(e) = create_or_update_linux_desktop_icon_file() {
        error(&format!("Failed to create desktop icon file: {}", e));
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
