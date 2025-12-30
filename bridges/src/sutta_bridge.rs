use std::fs;
use std::path::{Path, PathBuf};
use std::thread;

use core::pin::Pin;
use cxx_qt_lib::{QString, QStringList, QUrl};
use cxx_qt::Threading;
use regex::{Regex, Captures};
use lazy_static::lazy_static;

use simsapa_backend::query_task::SearchQueryTask;
use simsapa_backend::types::{SearchArea, SearchMode, SearchParams, SearchResultPage};
use simsapa_backend::theme_colors::ThemeColors;
use simsapa_backend::{get_app_data, try_get_app_data, get_app_globals, get_create_simsapa_dir, is_mobile, save_to_file, check_file_exists_print_err};
use simsapa_backend::html_content::{sutta_html_page, blank_html_page};
use simsapa_backend::dir_list::{generate_html_directory_listing, generate_plain_directory_listing};
use simsapa_backend::helpers::{extract_words, normalize_query_text, query_text_to_uid_field_query};
use simsapa_backend::prompt_utils::markdown_to_html;
use simsapa_backend::logger::{info, error};

static DICTIONARY_JS: &'static str = include_str!("../../assets/js/dictionary.js");
static DICTIONARY_CSS: &'static str = include_str!("../../assets/css/dictionary.css");
static SIMSAPA_JS: &'static str = include_str!("../../assets/js/simsapa.min.js");

/// Convert a QUrl to a local file path string.
/// Handles Windows paths correctly - QUrl::path() returns "/C:/path" on Windows,
/// but we need "C:/path" for Rust's Path/PathBuf to work correctly.
fn qurl_to_local_path(url: &QUrl) -> String {
    let path_str = url.path().to_string();

    // On Windows, QUrl::path() returns "/C:/path" for local files
    // We need to remove the leading slash for Windows drive paths
    #[cfg(target_os = "windows")]
    {
        if path_str.len() >= 3 && path_str.starts_with('/') {
            let chars: Vec<char> = path_str.chars().collect();
            // Check for pattern like "/C:" where second char is a letter and third is ':'
            if chars.len() >= 3 && chars[1].is_ascii_alphabetic() && chars[2] == ':' {
                return path_str[1..].to_string();
            }
        }
    }

    path_str
}

#[cxx_qt::bridge]
pub mod qobject {

    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;

        include!("cxx-qt-lib/qstringlist.h");
        type QStringList = cxx_qt_lib::QStringList;

        include!("cxx-qt-lib/qurl.h");
        type QUrl = cxx_qt_lib::QUrl;

        include!("system_palette.h");
        fn get_system_palette_json() -> QString;

        include!("utils.h");
        fn copy_content_uri_to_temp_file(content_uri: &QString) -> QString;
    }

    impl cxx_qt::Threading for SuttaBridge{}

    extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qml_singleton]
        #[qproperty(bool, db_loaded)]
        #[qproperty(bool, sutta_references_loaded)]
        #[qproperty(bool, updates_checked)]
        #[namespace = "sutta_bridge"]
        type SuttaBridge = super::SuttaBridgeRust;

        #[qsignal]
        #[cxx_name = "updateWindowTitle"]
        fn update_window_title(self: Pin<&mut SuttaBridge>, sutta_uid: QString, sutta_ref: QString, sutta_title: QString);

        #[qsignal]
        #[cxx_name = "resultsPageReady"]
        fn results_page_ready(self: Pin<&mut SuttaBridge>, results_json: QString);

        #[qsignal]
        #[cxx_name = "allParagraphsGlossReady"]
        fn all_paragraphs_gloss_ready(self: Pin<&mut SuttaBridge>, results_json: QString);

        #[qsignal]
        #[cxx_name = "paragraphGlossReady"]
        fn paragraph_gloss_ready(self: Pin<&mut SuttaBridge>, paragraph_index: i32, results_json: QString);

        #[qsignal]
        #[cxx_name = "dpdLookupReady"]
        fn dpd_lookup_ready(self: Pin<&mut SuttaBridge>, query_id: QString, results_json: QString);

        #[qsignal]
        #[cxx_name = "ankiCsvExportReady"]
        fn anki_csv_export_ready(self: Pin<&mut SuttaBridge>, results_json: QString);

        #[qsignal]
        #[cxx_name = "ankiPreviewReady"]
        fn anki_preview_ready(self: Pin<&mut SuttaBridge>, preview_html: QString);

        #[qsignal]
        #[cxx_name = "databaseValidationResult"]
        fn database_validation_result(self: Pin<&mut SuttaBridge>, database_name: QString, is_valid: bool, message: QString);

        #[qsignal]
        #[cxx_name = "documentImportProgress"]
        fn document_import_progress(self: Pin<&mut SuttaBridge>, message: QString);

        #[qsignal]
        #[cxx_name = "documentImportCompleted"]
        fn document_import_completed(self: Pin<&mut SuttaBridge>, success: bool, message: QString);

        #[qsignal]
        #[cxx_name = "showChapterFromLibrary"]
        fn show_chapter_from_library(self: Pin<&mut SuttaBridge>, window_id: QString, result_data_json: QString);

        #[qsignal]
        #[cxx_name = "showSuttaFromReferenceSearch"]
        fn show_sutta_from_reference_search(self: Pin<&mut SuttaBridge>, window_id: QString, result_data_json: QString);

        #[qsignal]
        #[cxx_name = "bookMetadataUpdated"]
        fn book_metadata_updated(self: Pin<&mut SuttaBridge>, success: bool, message: QString);

        // Update checker signals
        #[qsignal]
        #[cxx_name = "appUpdateAvailable"]
        fn app_update_available(self: Pin<&mut SuttaBridge>, update_info_json: QString);

        #[qsignal]
        #[cxx_name = "dbUpdateAvailable"]
        fn db_update_available(self: Pin<&mut SuttaBridge>, update_info_json: QString);

        #[qsignal]
        #[cxx_name = "localDbObsolete"]
        fn local_db_obsolete(self: Pin<&mut SuttaBridge>, update_info_json: QString);

        #[qsignal]
        #[cxx_name = "noUpdatesAvailable"]
        fn no_updates_available(self: Pin<&mut SuttaBridge>);

        #[qsignal]
        #[cxx_name = "updateCheckError"]
        fn update_check_error(self: Pin<&mut SuttaBridge>, error_message: QString);

        #[qsignal]
        #[cxx_name = "releasesCheckCompleted"]
        fn releases_check_completed(self: Pin<&mut SuttaBridge>);

        #[qinvokable]
        fn emit_update_window_title(self: Pin<&mut SuttaBridge>, sutta_uid: QString, sutta_ref: QString, sutta_title: QString);

        #[qinvokable]
        fn emit_show_chapter_from_library(self: Pin<&mut SuttaBridge>, window_id: QString, result_data_json: QString);

        #[qinvokable]
        fn emit_show_sutta_from_reference_search(self: Pin<&mut SuttaBridge>, window_id: QString, result_data_json: QString);

        #[qinvokable]
        fn load_db(self: Pin<&mut SuttaBridge>);

        #[qinvokable]
        fn load_sutta_references(self: Pin<&mut SuttaBridge>);

        #[qinvokable]
        fn appdata_first_query(self: Pin<&mut SuttaBridge>);

        #[qinvokable]
        fn dpd_first_query(self: Pin<&mut SuttaBridge>);

        #[qinvokable]
        fn dictionary_first_query(self: Pin<&mut SuttaBridge>);

        #[qinvokable]
        fn userdata_first_query(self: Pin<&mut SuttaBridge>);

        #[qinvokable]
        fn reset_userdata_database(self: Pin<&mut SuttaBridge>) -> bool;

        #[qinvokable]
        fn query_text_to_uid_field_query(self: &SuttaBridge, query_text: &QString) -> QString;

        #[qinvokable]
        fn results_page(self: Pin<&mut SuttaBridge>, query: &QString, page_num: usize, search_area: &QString, params_json: &QString);

        #[qinvokable]
        fn extract_words(self: &SuttaBridge, text: &QString) -> QStringList;

        #[qinvokable]
        fn normalize_query_text(self: &SuttaBridge, text: &QString) -> QString;

        #[qinvokable]
        fn dpd_deconstructor_list(self: &SuttaBridge, query: &QString) -> QStringList;

        #[qinvokable]
        fn dpd_lookup_json(self: &SuttaBridge, query: &QString) -> QString;

        #[qinvokable]
        fn dpd_lookup_json_async(self: Pin<&mut SuttaBridge>, query_id: &QString, query: &QString);

        #[qinvokable]
        fn get_sutta_html(self: &SuttaBridge, window_id: &QString, uid: &QString) -> QString;

        #[qinvokable]
        fn get_word_html(self: &SuttaBridge, window_id: &QString, uid: &QString) -> QString;

        #[qinvokable]
        fn get_translations_data_json_for_sutta_uid(self: &SuttaBridge, sutta_uid: &QString) -> QString;

        #[qinvokable]
        fn app_data_folder_path(self: &SuttaBridge) -> QString;

        #[qinvokable]
        fn is_app_data_folder_writable(self: &SuttaBridge) -> bool;

        #[qinvokable]
        fn app_data_contents_html_table(self: &SuttaBridge) -> QString;

        #[qinvokable]
        fn app_data_contents_plain_table(self: &SuttaBridge) -> QString;

        #[qinvokable]
        fn get_theme_name(self: &SuttaBridge) -> QString;

        #[qinvokable]
        fn set_theme_name(self: Pin<&mut SuttaBridge>, theme_name: &QString);

        #[qinvokable]
        fn get_ai_models_auto_retry(self: &SuttaBridge) -> bool;

        #[qinvokable]
        fn set_ai_models_auto_retry(self: Pin<&mut SuttaBridge>, auto_retry: bool);

        #[qinvokable]
        fn get_api_key(self: &SuttaBridge, key_name: &QString) -> QString;

        #[qinvokable]
        fn set_api_keys(self: Pin<&mut SuttaBridge>, api_keys_json: &QString);

        #[qinvokable]
        fn get_system_prompt(self: &SuttaBridge, prompt_name: &QString) -> QString;

        #[qinvokable]
        fn set_system_prompts_json(self: Pin<&mut SuttaBridge>, prompts_json: &QString);

        #[qinvokable]
        fn get_system_prompts_json(self: &SuttaBridge) -> QString;

        #[qinvokable]
        fn get_providers_json(self: &SuttaBridge) -> QString;

        #[qinvokable]
        fn set_providers_json(self: Pin<&mut SuttaBridge>, providers_json: &QString);

        #[qinvokable]
        fn get_provider_api_key(self: &SuttaBridge, provider_name: &QString) -> QString;

        #[qinvokable]
        fn set_provider_api_key(self: Pin<&mut SuttaBridge>, provider_name: &QString, api_key: &QString);

        #[qinvokable]
        fn get_api_url(self: &SuttaBridge) -> QString;

        #[qinvokable]
        fn get_status_bar_height(self: &SuttaBridge) -> i32;

        #[qinvokable]
        fn open_sutta_search_window(self: &SuttaBridge);

        #[qinvokable]
        fn open_sutta_search_window_with_result(self: &SuttaBridge, result_data_json: &QString);

        #[qinvokable]
        fn open_sutta_languages_window(self: &SuttaBridge);

        #[qinvokable]
        fn open_library_window(self: &SuttaBridge);

        #[qinvokable]
        fn open_reference_search_window(self: &SuttaBridge);

        #[qinvokable]
        fn get_all_books_json(self: &SuttaBridge) -> QString;

        #[qinvokable]
        fn get_book_by_uid_json(self: &SuttaBridge, book_uid: &QString) -> QString;

        #[qinvokable]
        fn get_spine_items_for_book_json(self: &SuttaBridge, book_uid: &QString) -> QString;

        #[qinvokable]
        fn get_spine_item_uid_by_path(self: &SuttaBridge, book_uid: &QString, resource_path: &QString) -> QString;

        #[qinvokable]
        fn get_book_spine_html(self: &SuttaBridge, window_id: &QString, spine_item_uid: &QString) -> QString;

        #[qinvokable]
        fn check_book_uid_exists(self: &SuttaBridge, book_uid: &QString) -> bool;

        #[qinvokable]
        fn extract_document_metadata(self: &SuttaBridge, file_path: &QString) -> QString;

        #[qinvokable]
        fn copy_content_uri_to_temp(self: &SuttaBridge, content_uri: &QString) -> QString;

        #[qinvokable]
        fn delete_temp_import_folder(self: &SuttaBridge) -> bool;

        #[qinvokable]
        fn is_spine_item_pdf(self: &SuttaBridge, spine_item_uid: &QString) -> bool;

        #[qinvokable]
        fn get_book_uid_for_spine_item(self: &SuttaBridge, spine_item_uid: &QString) -> QString;

        #[qinvokable]
        fn import_document(self: Pin<&mut SuttaBridge>, file_path: &QString, book_uid: &QString, title: &QString, author: &QString, document_type: &QString, split_tag: &QString);

        #[qinvokable]
        fn remove_book(self: &SuttaBridge, book_uid: &QString) -> bool;

        #[qinvokable]
        fn get_book_metadata_json(self: &SuttaBridge, book_uid: &QString) -> QString;

        #[qinvokable]
        fn update_book_metadata(self: Pin<&mut SuttaBridge>, book_uid: &QString, title: &QString, author: &QString, enable_embedded_css: bool);

        #[qinvokable]
        fn set_provider_enabled(self: Pin<&mut SuttaBridge>, provider_name: &QString, enabled: bool);

        #[qinvokable]
        fn add_provider_model(self: Pin<&mut SuttaBridge>, provider_name: &QString, model_name: &QString);

        #[qinvokable]
        fn remove_provider_model(self: Pin<&mut SuttaBridge>, provider_name: &QString, model_name: &QString);

        #[qinvokable]
        fn set_provider_model_enabled(self: Pin<&mut SuttaBridge>, provider_name: &QString, model_name: &QString, enabled: bool);

        #[qinvokable]
        fn get_provider_for_model(self: &SuttaBridge, model_name: &QString) -> QString;

        #[qinvokable]
        fn get_anki_template_front(self: &SuttaBridge) -> QString;

        #[qinvokable]
        fn set_anki_template_front(self: Pin<&mut SuttaBridge>, template_str: &QString);

        #[qinvokable]
        fn get_anki_template_back(self: &SuttaBridge) -> QString;

        #[qinvokable]
        fn set_anki_template_back(self: Pin<&mut SuttaBridge>, template_str: &QString);

        #[qinvokable]
        fn get_anki_template_cloze_front(self: &SuttaBridge) -> QString;

        #[qinvokable]
        fn set_anki_template_cloze_front(self: Pin<&mut SuttaBridge>, template_str: &QString);

        #[qinvokable]
        fn get_anki_template_cloze_back(self: &SuttaBridge) -> QString;

        #[qinvokable]
        fn set_anki_template_cloze_back(self: Pin<&mut SuttaBridge>, template_str: &QString);

        #[qinvokable]
        fn get_anki_export_format(self: &SuttaBridge) -> QString;

        #[qinvokable]
        fn set_anki_export_format(self: Pin<&mut SuttaBridge>, format: &QString);

        #[qinvokable]
        fn get_anki_include_cloze(self: &SuttaBridge) -> bool;

        #[qinvokable]
        fn set_anki_include_cloze(self: Pin<&mut SuttaBridge>, include: bool);

        #[qinvokable]
        fn get_sample_vocabulary_data_json(self: &SuttaBridge) -> QString;

        #[qinvokable]
        fn get_dpd_headword_by_uid(self: &SuttaBridge, uid: &QString) -> QString;

        #[qinvokable]
        fn get_saved_theme(self: &SuttaBridge) -> QString;

        #[qinvokable]
        fn get_theme(self: &SuttaBridge, theme_name: &QString) -> QString;

        #[qinvokable]
        fn get_common_words_json(self: &SuttaBridge) -> QString;

        #[qinvokable]
        fn save_common_words_json(self: &SuttaBridge, words_json: &QString);

        #[qinvokable]
        fn get_gloss_history_json(self: &SuttaBridge) -> QString;

        #[qinvokable]
        fn update_gloss_session(self: &SuttaBridge, session_uid: &QString, gloss_data_json: &QString);

        #[qinvokable]
        fn save_new_gloss_session(self: &SuttaBridge, gloss_data_json: &QString) -> QString;

        #[qinvokable]
        fn save_anki_csv(self: &SuttaBridge, csv_content: &QString) -> QString;

        #[qinvokable]
        fn process_all_paragraphs_background(self: Pin<&mut SuttaBridge>, input_json: &QString);

        #[qinvokable]
        fn process_paragraph_background(self: Pin<&mut SuttaBridge>, paragraph_index: i32, input_json: &QString);

        #[qinvokable]
        fn save_file(self: &SuttaBridge, folder_url: &QUrl, filename: &QString, content: &QString) -> bool;

        #[qinvokable]
        fn check_file_exists_in_folder(self: &SuttaBridge, folder_url: &QUrl, filename: &QString) -> bool;

        #[qinvokable]
        fn markdown_to_html(self: &SuttaBridge, markdown_text: &QString) -> QString;

        #[qinvokable]
        fn export_anki_csv_background(self: Pin<&mut SuttaBridge>, input_json: &QString);

        #[qinvokable]
        fn render_anki_preview_background(self: Pin<&mut SuttaBridge>, front_template: &QString, back_template: &QString);

        #[qinvokable]
        fn get_search_as_you_type(self: &SuttaBridge) -> bool;

        #[qinvokable]
        fn set_search_as_you_type(self: Pin<&mut SuttaBridge>, enabled: bool);

        #[qinvokable]
        fn get_open_find_in_sutta_results(self: &SuttaBridge) -> bool;

        #[qinvokable]
        fn set_open_find_in_sutta_results(self: Pin<&mut SuttaBridge>, enabled: bool);

        #[qinvokable]
        fn get_sutta_language_labels(self: &SuttaBridge) -> QStringList;

        #[qinvokable]
        fn get_sutta_language_filter_key(self: &SuttaBridge) -> QString;

        #[qinvokable]
        fn set_sutta_language_filter_key(self: Pin<&mut SuttaBridge>, key: QString);

        #[qinvokable]
        fn get_mobile_top_bar_margin(self: &SuttaBridge) -> i32;

        #[qinvokable]
        fn is_mobile_top_bar_margin_system(self: &SuttaBridge) -> bool;

        #[qinvokable]
        fn get_mobile_top_bar_margin_custom_value(self: &SuttaBridge) -> u32;

        #[qinvokable]
        fn set_mobile_top_bar_margin_system(self: Pin<&mut SuttaBridge>);

        #[qinvokable]
        fn set_mobile_top_bar_margin_custom(self: Pin<&mut SuttaBridge>, value: u32);

        #[qinvokable]
        fn get_sutta_language_labels_with_counts(self: &SuttaBridge) -> QStringList;

        #[qinvokable]
        fn search_reference(self: &SuttaBridge, query: &QString, field: &QString) -> QString;

        #[qinvokable]
        fn extract_uid_from_url(self: &SuttaBridge, url: &QString) -> QString;

        #[qinvokable]
        fn get_full_sutta_uid(self: &SuttaBridge, partial_uid: &QString) -> QString;

        #[qinvokable]
        fn get_sutta_reference_info(self: &SuttaBridge, uid: &QString) -> QString;

        // Update checker functions
        #[qinvokable]
        fn check_for_updates(self: Pin<&mut SuttaBridge>, include_no_updates: bool, screen_size: &QString, save_stats_behaviour: &QString);

        #[qinvokable]
        fn get_notify_about_simsapa_updates(self: &SuttaBridge) -> bool;

        #[qinvokable]
        fn set_notify_about_simsapa_updates(self: Pin<&mut SuttaBridge>, enabled: bool);

        #[qinvokable]
        fn prepare_for_database_upgrade(self: &SuttaBridge);

        #[qinvokable]
        fn get_compatible_asset_version_tag(self: &SuttaBridge) -> QString;

        #[qinvokable]
        fn get_compatible_asset_github_repo(self: &SuttaBridge) -> QString;
    }
}

pub struct SuttaBridgeRust {
    db_loaded: bool,
    sutta_references_loaded: bool,
    /// Flag to track if update check has already been performed in this session
    updates_checked: bool,
}

impl Default for SuttaBridgeRust {
    fn default() -> Self {
        Self {
            db_loaded: false,
            sutta_references_loaded: false,
            updates_checked: false,
        }
    }
}

impl qobject::SuttaBridge {
    pub fn emit_update_window_title(self: Pin<&mut Self>, sutta_uid: QString, sutta_ref: QString, sutta_title: QString) {
        // info(&format!("emit_update_window_title(): {} {} {}", &sutta_uid.to_string(), &sutta_ref.to_string(), &sutta_title.to_string()));
        self.update_window_title(sutta_uid, sutta_ref, sutta_title);
    }

    pub fn emit_show_chapter_from_library(self: Pin<&mut Self>, window_id: QString, result_data_json: QString) {
        use crate::api::ffi;
        ffi::callback_show_chapter_in_sutta_window(window_id, result_data_json);
    }

    pub fn emit_show_sutta_from_reference_search(self: Pin<&mut Self>, window_id: QString, result_data_json: QString) {
        use crate::api::ffi;
        ffi::callback_show_sutta_from_reference_search(window_id, result_data_json);
    }

    pub fn load_db(self: Pin<&mut Self>) {
        info("SuttaBridge::load_db() start");
        let qt_thread = self.qt_thread();
        thread::spawn(move || {
            // FIXME: should init AppData if not alrerady
            // let r = db::rust_backend_init_db();
            let r = true;
            qt_thread.queue(move |mut qo| {
                qo.as_mut().set_db_loaded(r);
            }).unwrap();
            info("SuttaBridge::load_db() end");
        });
    }

    pub fn load_sutta_references(self: Pin<&mut Self>) {
        info("SuttaBridge::load_sutta_references() start");
        let qt_thread = self.qt_thread();
        thread::spawn(move || {
            simsapa_backend::init_sutta_references();
            qt_thread.queue(move |mut qo| {
                qo.as_mut().set_sutta_references_loaded(true);
            }).unwrap();
            info("SuttaBridge::load_sutta_references() end");
        });
    }

    /// Runs a db query so that db is cached from the disk. It should finish by
    /// the time the user types in the first actual query, and that will respond
    /// faster.
    pub fn appdata_first_query(self: Pin<&mut Self>) {
        info("SuttaBridge::appdata_first_query() start");

        let qt_thread = self.qt_thread();

        thread::spawn(move || {
            let mut error_message = String::new();

            // Check 1: Database file exists (using try_exists() to avoid Android permission crashes)
            let db_path = get_app_globals().paths.appdata_db_path.clone();
            match db_path.try_exists() {
                Ok(true) => {}, // File exists, continue
                Ok(false) => {
                    error_message = "Database file not found".to_string();
                    error("Database validation FAILED: Appdata - Database file not found");
                },
                Err(e) => {
                    error_message = format!("Error checking file existence: {}", e);
                    error(&format!("Database validation FAILED: Appdata - Error checking file existence: {}", e));
                }
            }

            if error_message.is_empty() {
                // Check 2 & 3: Query executes and returns results
                let app_data = get_app_data();
                let params = SearchParams {
                    mode: SearchMode::ContainsMatch,
                    page_len: None,
                    lang: None,
                    lang_include: true,
                    source: None,
                    source_include: true,
                    enable_regex: false,
                    fuzzy_distance: 0,
                };

                let mut query_task = SearchQueryTask::new(
                    &app_data.dbm,
                    "dhamma".to_string(),
                    params,
                    SearchArea::Suttas,
                );

                match query_task.results_page(0) {
                    Ok(results) => {
                        if results.is_empty() {
                            error_message = "Query returned 0 results".to_string();
                            error("Database validation FAILED: Appdata - Query returned 0 results");
                        } else {
                            info("Database validation: Appdata OK");
                        }
                    },
                    Err(e) => {
                        error_message = format!("Query error: {}", e);
                        error(&format!("Database validation FAILED: Appdata - Query error: {}", e));
                    }
                };
            }

            // Always emit signal with result (success or failure)
            let is_valid = error_message.is_empty();
            let message = if is_valid {
                QString::from("OK")
            } else {
                QString::from(error_message)
            };

            qt_thread.queue(move |mut qo| {
                qo.as_mut().database_validation_result(QString::from("appdata"), is_valid, message);
            }).unwrap();

            info("SuttaBridge::appdata_first_query() end");
        });
    }

    pub fn dpd_first_query(self: Pin<&mut Self>) {
        info("SuttaBridge::dpd_first_query() start");

        let qt_thread = self.qt_thread();

        thread::spawn(move || {
            let mut error_message = String::new();

            // Check 1: Database file exists (using try_exists() to avoid Android permission crashes)
            let db_path = get_app_globals().paths.dpd_db_path.clone();
            match db_path.try_exists() {
                Ok(true) => {}, // File exists, continue
                Ok(false) => {
                    error_message = "Database file not found".to_string();
                    error("Database validation FAILED: DPD - Database file not found");
                },
                Err(e) => {
                    error_message = format!("Error checking file existence: {}", e);
                    error(&format!("Database validation FAILED: DPD - Error checking file existence: {}", e));
                }
            }

            if error_message.is_empty() {
                // Check 2 & 3: Query executes and returns results
                let app_data = get_app_data();
                let json = app_data.dbm.dpd.dpd_lookup_json("dhamma");

                // dpd_lookup_json returns a JSON array string, check if it contains results
                if json == "[]" || json.is_empty() {
                    error_message = "Query returned 0 results".to_string();
                    error("Database validation FAILED: DPD - Query returned 0 results");
                } else {
                    info("Database validation: DPD OK");
                }
            }

            // Always emit signal with result (success or failure)
            let is_valid = error_message.is_empty();
            let message = if is_valid {
                QString::from("OK")
            } else {
                QString::from(error_message)
            };

            qt_thread.queue(move |mut qo| {
                qo.as_mut().database_validation_result(QString::from("dpd"), is_valid, message);
            }).unwrap();

            info("SuttaBridge::dpd_first_query() end");
        });
    }

    pub fn dictionary_first_query(self: Pin<&mut Self>) {
        info("SuttaBridge::dictionary_first_query() start");

        let qt_thread = self.qt_thread();

        thread::spawn(move || {
            let mut error_message = String::new();

            // Check 1: Database file exists (using try_exists() to avoid Android permission crashes)
            let db_path = get_app_globals().paths.dict_db_path.clone();
            match db_path.try_exists() {
                Ok(true) => {}, // File exists, continue
                Ok(false) => {
                    error_message = "Database file not found".to_string();
                    error("Database validation FAILED: Dictionaries - Database file not found");
                },
                Err(e) => {
                    error_message = format!("Error checking file existence: {}", e);
                    error(&format!("Database validation FAILED: Dictionaries - Error checking file existence: {}", e));
                }
            }

            if error_message.is_empty() {
                // Check 2 & 3: Query executes and returns results
                let app_data = get_app_data();
                let word = app_data.dbm.dictionaries.get_word("anidassana/dpd");

                match word {
                    Some(_) => {
                        info("Database validation: Dictionaries OK");
                    },
                    None => {
                        error_message = "Query returned 0 results".to_string();
                        error("Database validation FAILED: Dictionaries - Query returned 0 results");
                    },
                }
            }

            // Always emit signal with result (success or failure)
            let is_valid = error_message.is_empty();
            let message = if is_valid {
                QString::from("OK")
            } else {
                QString::from(error_message)
            };

            qt_thread.queue(move |mut qo| {
                qo.as_mut().database_validation_result(QString::from("dictionaries"), is_valid, message);
            }).unwrap();

            info("SuttaBridge::dictionary_first_query() end");
        });
    }

    pub fn userdata_first_query(self: Pin<&mut Self>) {
        info("SuttaBridge::userdata_first_query() start");

        let qt_thread = self.qt_thread();

        thread::spawn(move || {
            let mut error_message = String::new();

            // Check 1: Database file exists (using try_exists() to avoid Android permission crashes)
            let db_path = get_app_globals().paths.userdata_db_path.clone();
            match db_path.try_exists() {
                Ok(true) => {}, // File exists, continue
                Ok(false) => {
                    error_message = "Database file not found".to_string();
                    error("Database validation FAILED: Userdata - Database file not found");
                },
                Err(e) => {
                    error_message = format!("Error checking file existence: {}", e);
                    error(&format!("Database validation FAILED: Userdata - Error checking file existence: {}", e));
                }
            }

            if error_message.is_empty() {
                // Check 2 & 3: Try to get app_settings - if this succeeds without error,
                // the database is valid. We can't easily distinguish between default values
                // and actual database values without more complex queries, but calling
                // get_app_settings will fail if the database is corrupt or inaccessible.
                let app_data = get_app_data();

                // Try a simple database operation to validate connectivity
                match app_data.dbm.userdata.get_conn() {
                    Ok(_) => {
                        // Successfully connected, now try to read app_settings
                        let _settings = app_data.dbm.userdata.get_app_settings();
                        info("Database validation: Userdata OK");
                    },
                    Err(e) => {
                        error_message = format!("Query error: {}", e);
                        error(&format!("Database validation FAILED: Userdata - Query error: {}", e));
                    }
                }
            }

            // Always emit signal with result (success or failure)
            let is_valid = error_message.is_empty();
            let message = if is_valid {
                QString::from("OK")
            } else {
                QString::from(error_message)
            };

            qt_thread.queue(move |mut qo| {
                qo.as_mut().database_validation_result(QString::from("userdata"), is_valid, message);
            }).unwrap();

            info("SuttaBridge::userdata_first_query() end");
        });
    }

    pub fn reset_userdata_database(self: Pin<&mut Self>) -> bool {
        info("SuttaBridge::reset_userdata_database() start");
        use simsapa_backend::db::initialize_userdata;

        let g = get_app_globals();
        let userdata_path = g.paths.userdata_db_path.clone();
        let userdata_url = g.paths.userdata_database_url.clone();

        // Step 1: Remove the corrupt userdata database
        match userdata_path.try_exists() {
            Ok(true) => {
                info(&format!("Removing userdata database at: {}", userdata_path.display()));
                match fs::remove_file(&userdata_path) {
                    Ok(_) => {
                        info("Userdata database removed successfully");
                    },
                    Err(e) => {
                        error(&format!("Failed to remove userdata database: {}", e));
                        return false;
                    }
                }
            },
            Ok(false) => {
                info("Userdata database doesn't exist, will create new one");
            },
            Err(e) => {
                error(&format!("Error checking userdata database existence: {}", e));
                return false;
            }
        }

        // Step 2: Re-initialize with defaults
        info("Re-initializing userdata database with defaults...");
        match initialize_userdata(&userdata_url) {
            Ok(_) => {
                info("SuttaBridge::reset_userdata_database() reset complete");
                true
            },
            Err(e) => {
                error(&format!("Failed to re-initialize userdata database: {}", e));
                false
            }
        }
    }

    pub fn query_text_to_uid_field_query(&self, query_text: &QString) -> QString {
        QString::from(query_text_to_uid_field_query(&query_text.to_string()))
    }

    pub fn results_page(self: Pin<&mut Self>, query: &QString, page_num: usize, search_area: &QString, params_json: &QString) {
        info(&format!("SuttaBridge::results_page() start - query='{}', page_num={}, search_area='{}'", query.to_string(), page_num, search_area.to_string()));
        let qt_thread = self.qt_thread();

        let query_text = query.to_string();
        let search_area_text = search_area.to_string();
        let params_json_text = params_json.to_string();
        info(&format!("params_json: {}", params_json_text));

        // Spawn a thread so Qt event loop is not blocked
        thread::spawn(move || {
            // FIXME: Can't store the query_task on SuttaBridgeRust
            // because it SearchQueryTask includes &'a DbManager reference.
            // Store only a connection pool?
            let app_data = get_app_data();

            let params: SearchParams = serde_json::from_str(&params_json_text).unwrap_or_default();

            let search_area_enum = match search_area_text.as_str() {
                "Dictionary" => SearchArea::Dictionary,
                "Library" => SearchArea::Library,
                _ => SearchArea::Suttas, // Default to Suttas for any other value
            };

            // FIXME: We have to create a SearchQueryTask for each search until we
            // can store it on SuttaBridgeRust.
            let mut query_task = SearchQueryTask::new(
                &app_data.dbm,
                query_text,
                params,
                search_area_enum,
            );

            let results = match query_task.results_page(page_num) {
                Ok(x) => x,
                Err(e) => {
                    error(&format!("{}", e));
                    let error_json = serde_json::json!({"error": format!("{}", e)}).to_string();
                    qt_thread.queue(move |mut qo| {
                        qo.as_mut().results_page_ready(QString::from(error_json));
                    }).unwrap();
                    return;
                }
            };

            let results_page = SearchResultPage {
                total_hits: query_task.total_hits() as usize,
                page_len: query_task.page_len as usize,
                page_num,
                results,
            };

            let json = serde_json::to_string(&results_page).unwrap_or_default();

            // Emit signal with the results
            qt_thread.queue(move |mut qo| {
                qo.as_mut().results_page_ready(QString::from(json));
            }).unwrap();

            info("SuttaBridge::results_page() end");
        });
    }

    pub fn extract_words(&self, text: &QString) -> QStringList {
        let words = extract_words(&text.to_string());
        let mut res = QStringList::default();
        for i in words {
            res.append(QString::from(i));
        }
        res
    }

    pub fn normalize_query_text(&self, text: &QString) -> QString {
        QString::from(normalize_query_text(Some(text.to_string())))
    }

    pub fn dpd_deconstructor_list(&self, query: &QString) -> QStringList {
        let app_data = get_app_data();
        let list = app_data.dbm.dpd.dpd_deconstructor_list(&query.to_string());
        let mut res = QStringList::default();
        for i in list {
            res.append(QString::from(i));
        }
        res
    }

    pub fn dpd_lookup_json(&self, query: &QString) -> QString {
        let app_data = get_app_data();
        let s = app_data.dbm.dpd.dpd_lookup_json(&query.to_string());
        QString::from(s)
    }

    pub fn dpd_lookup_json_async(self: Pin<&mut Self>, query_id: &QString, query: &QString) {
        info("SuttaBridge::dpd_lookup_json_async() start");
        let qt_thread = self.qt_thread();
        let query_id_string = query_id.to_string();
        let query_text = query.to_string();

        // Spawn a thread so Qt event loop is not blocked
        thread::spawn(move || {
            let app_data = get_app_data();
            let s = app_data.dbm.dpd.dpd_lookup_json(&query_text);
            let results_json = QString::from(s);
            let query_id_qstring = QString::from(query_id_string);

            // Emit signal with the query_id and results
            qt_thread.queue(move |mut qo| {
                qo.as_mut().dpd_lookup_ready(query_id_qstring, results_json);
            }).unwrap();

            info("SuttaBridge::dpd_lookup_json_async() end");
        });
    }

    pub fn get_sutta_html(&self, window_id: &QString, uid: &QString) -> QString {
        let app_data = get_app_data();
        let app_settings = app_data.app_settings_cache.read().expect("Failed to read app settings");
        let body_class = app_settings.theme_name_as_string();

        let blank_page_html = blank_html_page(Some(body_class.clone()));
        if uid.trimmed().is_empty() {
            return QString::from(blank_page_html);
        }

        let sutta = app_data.dbm.appdata.get_sutta(&uid.to_string());

        let html = match sutta {
            Some(sutta) => {
                let js_extra = format!("const WINDOW_ID = '{}'; window.WINDOW_ID = WINDOW_ID;", &window_id.to_string());

                app_data.render_sutta_content(&sutta, None, Some(js_extra))
                .unwrap_or(sutta_html_page("Rendering error", None, None, None, Some(body_class)))
            },
            None => blank_page_html,
        };

        QString::from(html)
    }

    pub fn get_word_html(&self, window_id: &QString, uid: &QString) -> QString {
        let app_data = get_app_data();
        let app_settings = app_data.app_settings_cache.read().expect("Failed to read app settings");
        let body_class = app_settings.theme_name_as_string();

        let blank_page_html = blank_html_page(Some(body_class.clone()));
        if uid.trimmed().is_empty() {
            return QString::from(blank_page_html);
        }

        let word = app_data.dbm.dictionaries.get_word(&uid.to_string());

        lazy_static! {
            // (<link href=")(main.js)(") class="load_js" rel="preload" as="script">
            static ref RE_LINK_HREF: Regex = Regex::new(r#"(<link +[^>]*href=['"])([^'"]+)(['"])"#).unwrap();
            // Match <html> tag with optional attributes
            static ref RE_HTML_TAG: Regex = Regex::new(r#"<html[^>]*>"#).unwrap();
            // Match <body> tag with optional attributes
            static ref RE_BODY_TAG: Regex = Regex::new(r#"<body[^>]*>"#).unwrap();
        }

        let html = match word {
            Some(word) => match word.definition_html {
                Some(ref definition_html) => {
                    let mut js_extra = "".to_string();
                    js_extra.push_str(&format!(" const API_URL = '{}'; window.API_URL = API_URL;", &app_data.api_url));
                    js_extra.push_str(&format!(" const WINDOW_ID = '{}'; window.WINDOW_ID = WINDOW_ID;", &window_id.to_string()));
                    js_extra.push_str(&format!(" const IS_MOBILE = {};", is_mobile()));
                    js_extra.push_str(DICTIONARY_JS);
                    js_extra.push_str(SIMSAPA_JS);

                    let mut word_html = definition_html.clone();

                    word_html = word_html.replace(
                        "</head>",
                        &format!(r#"<style>{}</style><script>{}</script></head>"#, DICTIONARY_CSS, js_extra));

                    // Replace <html> tag to include dark mode class
                    word_html = RE_HTML_TAG.replace(&word_html, &format!(r#"<html class="{}">"#, body_class)).to_string();

                    // Replace <body> tag to include dark mode class and word heading
                    word_html = RE_BODY_TAG.replace(&word_html, &format!(r#"
<body class="{}">
    <div class='word-heading'>
        <div class='word-title'>
            <h1>{}</h1>
        </div>
    </div>"#, body_class, word.word())).to_string();

                    word_html = RE_LINK_HREF.replace_all(&word_html, |caps: &Captures| {
                        format!("{}{}{}{}",
                                &caps[1],
                                &format!("{}/assets/dpd-res/", &app_data.api_url),
                                &caps[2],
                                &caps[3])
                    }).to_string();

                    word_html
                },
                None => blank_page_html,
            },
            None => blank_page_html,
        };

        QString::from(html)
    }

    pub fn get_translations_data_json_for_sutta_uid(&self, sutta_uid: &QString) -> QString {
        let app_data = get_app_data();
        let r = app_data.dbm.appdata.get_translations_data_json_for_sutta_uid(&sutta_uid.to_string());
        QString::from(r)
    }

    pub fn app_data_folder_path(&self) -> QString {
        let p = get_create_simsapa_dir().unwrap_or(PathBuf::from("."));
        let app_data_path = p.as_os_str();
        let s = match app_data_path.to_str() {
            Some(x) => x,
            None => "Path error",
        };
        QString::from(s)
    }

    pub fn is_app_data_folder_writable(&self) -> bool {
        let p = get_create_simsapa_dir().unwrap_or(PathBuf::from("."));
        let md = match fs::metadata(p) {
            Ok(x) => x,
            Err(_) => return false,
        };
        let permissions = md.permissions();
        let read_only = permissions.readonly();
        if read_only {
            false
        } else {
            true
        }
    }

    pub fn app_data_contents_html_table(&self) -> QString {
        let p = get_create_simsapa_dir().unwrap_or(PathBuf::from("."));
        let app_data_path = p.to_string_lossy();
        let app_data_folder_contents = generate_html_directory_listing(&app_data_path, 3).unwrap_or(String::from("Error"));
        QString::from(app_data_folder_contents)
    }

    pub fn app_data_contents_plain_table(&self) -> QString {
        let p = get_create_simsapa_dir().unwrap_or(PathBuf::from("."));
        let app_data_path = p.to_string_lossy();
        let app_data_folder_contents = generate_plain_directory_listing(&app_data_path, 3).unwrap_or(String::from("Error"));
        QString::from(app_data_folder_contents)
    }

    /// Get the current theme setting, 'system', 'light', or 'dark'
    pub fn get_theme_name(&self) -> QString {
        let app_data = get_app_data();
        let app_settings = app_data.app_settings_cache.read().expect("Failed to read app settings");
        QString::from(app_settings.theme_name_as_string())
    }

    /// Save the theme setting in the db
    pub fn set_theme_name(self: Pin<&mut Self>, theme_name: &QString) {
        let app_data = get_app_data();
        app_data.set_theme_name(&theme_name.to_string());
    }

    /// Get the AI models auto retry setting
    pub fn get_ai_models_auto_retry(&self) -> bool {
        let app_data = get_app_data();
        let app_settings = app_data.app_settings_cache.read().expect("Failed to read app settings");
        app_settings.ai_models_auto_retry
    }

    /// Save the AI models auto retry setting in the db
    pub fn set_ai_models_auto_retry(self: Pin<&mut Self>, auto_retry: bool) {
        let app_data = get_app_data();
        app_data.set_ai_models_auto_retry(auto_retry);
    }

    /// Get a specific API key by name
    pub fn get_api_key(&self, key_name: &QString) -> QString {
        let app_data = get_app_data();
        let key = app_data.get_api_key(&key_name.to_string());
        QString::from(key)
    }

    /// Save API keys in the db as JSON
    pub fn set_api_keys(self: Pin<&mut Self>, api_keys_json: &QString) {
        let app_data = get_app_data();
        app_data.set_api_keys(&api_keys_json.to_string());
    }

    /// Get a specific system prompt by name
    pub fn get_system_prompt(&self, prompt_name: &QString) -> QString {
        let app_data = get_app_data();
        let prompt = app_data.get_system_prompt(&prompt_name.to_string());
        QString::from(prompt)
    }

    /// Save system prompts in the db as JSON
    pub fn set_system_prompts_json(self: Pin<&mut Self>, prompts_json: &QString) {
        let app_data = get_app_data();
        app_data.set_system_prompts_json(&prompts_json.to_string());
    }

    /// Get all system prompts as JSON
    pub fn get_system_prompts_json(&self) -> QString {
        let app_data = get_app_data();
        let prompts_json = app_data.get_system_prompts_json();
        QString::from(prompts_json)
    }

    /// Get all providers as JSON
    pub fn get_providers_json(&self) -> QString {
        let app_data = get_app_data();
        let providers_json = app_data.get_providers_json();
        QString::from(providers_json)
    }

    /// Save providers in the db as JSON
    pub fn set_providers_json(self: Pin<&mut Self>, providers_json: &QString) {
        let app_data = get_app_data();
        app_data.set_providers_json(&providers_json.to_string());
    }

    /// Get API key for a specific provider
    pub fn get_provider_api_key(&self, provider_name: &QString) -> QString {
        let app_data = get_app_data();
        let app_settings = app_data.app_settings_cache.read().expect("Failed to read app settings");

        // First check environment variable
        let provider_name_str = provider_name.to_string();
        if let Some(provider) = app_settings.providers.iter().find(|p| format!("{:?}", p.name) == provider_name_str) {
            // Check environment variable first
            if let Ok(env_key) = std::env::var(&provider.api_key_env_var_name) {
                return QString::from(env_key);
            }
            // Fall back to stored value
            if let Some(ref stored_key) = provider.api_key_value {
                return QString::from(stored_key.clone());
            }
        }

        QString::from("")
    }

    /// Set API key for a specific provider
    pub fn set_provider_api_key(self: Pin<&mut Self>, provider_name: &QString, api_key: &QString) {
        let app_data = get_app_data();
        let mut app_settings = app_data.app_settings_cache.write().expect("Failed to write app settings");

        let provider_name_str = provider_name.to_string();
        if let Some(provider) = app_settings.providers.iter_mut().find(|p| format!("{:?}", p.name) == provider_name_str) {
            provider.api_key_value = if api_key.is_empty() { None } else { Some(api_key.to_string()) };

            // Save via backend function
            let providers_json = serde_json::to_string(&app_settings.providers).expect("Can't encode providers JSON");
            drop(app_settings); // Release the lock before saving
            app_data.set_providers_json(&providers_json);
        }
    }

    /// Get the API URL for the localhost server
    pub fn get_api_url(&self) -> QString {
        let app_data = get_app_data();
        QString::from(&app_data.api_url)
    }

    /// Get the status bar height in density-independent pixels (dp)
    /// Returns 0 on non-mobile platforms, actual height on Android
    pub fn get_status_bar_height(&self) -> i32 {
        use crate::api::ffi;
        ffi::get_status_bar_height()
    }

    /// Enable or disable a provider
    pub fn set_provider_enabled(self: Pin<&mut Self>, provider_name: &QString, enabled: bool) {
        let app_data = get_app_data();
        let mut app_settings = app_data.app_settings_cache.write().expect("Failed to write app settings");

        let provider_name_str = provider_name.to_string();
        if let Some(provider) = app_settings.providers.iter_mut().find(|p| format!("{:?}", p.name) == provider_name_str) {
            provider.enabled = enabled;

            // Save via backend function
            let providers_json = serde_json::to_string(&app_settings.providers).expect("Can't encode providers JSON");
            drop(app_settings); // Release the lock before saving
            app_data.set_providers_json(&providers_json);
        }
    }

    /// Add a new model to a provider
    pub fn add_provider_model(self: Pin<&mut Self>, provider_name: &QString, model_name: &QString) {
        use simsapa_backend::app_settings::ModelEntry;

        let app_data = get_app_data();
        let mut app_settings = app_data.app_settings_cache.write().expect("Failed to write app settings");

        let provider_name_str = provider_name.to_string();
        if let Some(provider) = app_settings.providers.iter_mut().find(|p| format!("{:?}", p.name) == provider_name_str) {
            // Check if model already exists
            if !provider.models.iter().any(|m| m.model_name == model_name.to_string()) {
                let new_model = ModelEntry {
                    model_name: model_name.to_string(),
                    enabled: true,
                    removable: true,
                };
                // Add the new model to the top of the list, where the user can
                // more easily see it.
                provider.models.insert(0, new_model);

                // Save via backend function
                let providers_json = serde_json::to_string(&app_settings.providers).expect("Can't encode providers JSON");
                drop(app_settings); // Release the lock before saving
                app_data.set_providers_json(&providers_json);
            }
        }
    }

    /// Remove a model from a provider
    pub fn remove_provider_model(self: Pin<&mut Self>, provider_name: &QString, model_name: &QString) {
        let app_data = get_app_data();
        let mut app_settings = app_data.app_settings_cache.write().expect("Failed to write app settings");

        let provider_name_str = provider_name.to_string();
        if let Some(provider) = app_settings.providers.iter_mut().find(|p| format!("{:?}", p.name) == provider_name_str) {
            // Only remove if the model is removable
            provider.models.retain(|m| !(m.model_name == model_name.to_string() && m.removable));

            // Save via backend function
            let providers_json = serde_json::to_string(&app_settings.providers).expect("Can't encode providers JSON");
            drop(app_settings); // Release the lock before saving
            app_data.set_providers_json(&providers_json);
        }
    }

    /// Set the enabled status of a specific model for a provider
    pub fn set_provider_model_enabled(self: Pin<&mut Self>, provider_name: &QString, model_name: &QString, enabled: bool) {
        let app_data = get_app_data();
        let mut app_settings = app_data.app_settings_cache.write().expect("Failed to write app settings");

        let provider_name_str = provider_name.to_string();
        if let Some(provider) = app_settings.providers.iter_mut().find(|p| format!("{:?}", p.name) == provider_name_str) {
            // Find the model and update its enabled status
            if let Some(model) = provider.models.iter_mut().find(|m| m.model_name == model_name.to_string()) {
                model.enabled = enabled;

                // Save via backend function
                let providers_json = serde_json::to_string(&app_settings.providers).expect("Can't encode providers JSON");
                drop(app_settings); // Release the lock before saving
                app_data.set_providers_json(&providers_json);
            }
        }
    }

    /// Get the provider name for a given model name
    pub fn get_provider_for_model(&self, model_name: &QString) -> QString {
        // NOTE: This matches model_name in any provider, so two providers should not have the model_name.
        // However it shouldn't be a problem because model names are quite specific to the providers.
        let app_data = get_app_data();
        let app_settings = app_data.app_settings_cache.read().expect("Failed to read app settings");

        let model_name_str = model_name.to_string();
        for provider in &app_settings.providers {
            if provider.models.iter().any(|m| m.model_name == model_name_str) {
                return QString::from(format!("{:?}", provider.name));
            }
        }

        QString::from("")
    }

    pub fn get_saved_theme(&self) -> QString {
        self.get_theme(&self.get_theme_name())
    }

    /// Get theme colors as JSON string
    pub fn get_theme(&self, theme_name: &QString) -> QString {
        let theme = theme_name.to_string();

        let theme_json = match theme.as_str() {
            "system" => qobject::get_system_palette_json(),
            "light" => QString::from(&ThemeColors::light_json()),
            "dark" => QString::from(&ThemeColors::dark_json()),
            _ => QString::from(serde_json::json!({}).to_string()),
        };

        theme_json
    }

    pub fn get_common_words_json(&self) -> QString {
        let app_data = get_app_data();
        let s = app_data.dbm.userdata.get_common_words_json();
        QString::from(s)
    }

    pub fn save_common_words_json(&self, words_json: &QString) {
        let app_data = get_app_data();
        match app_data.dbm.userdata.save_common_words_json(&words_json.to_string()) {
            Ok(_) => {},
            Err(e) => error(&format!("{}", e))
        }
    }

    pub fn get_gloss_history_json(&self) -> QString {
        QString::from("[]")
    }

    pub fn update_gloss_session(&self, _session_uid: &QString, _gloss_data_json: &QString) {
        return
    }

    pub fn save_new_gloss_session(&self, _gloss_data_json: &QString) -> QString {
        QString::from("session-uid")
    }

    pub fn save_anki_csv(&self, _csv_content: &QString) -> QString {
        QString::from("file_name.csv")
    }

    pub fn save_file(&self,
                     folder_url: &QUrl,
                     filename: &QString,
                     content: &QString) -> bool {
        let folder_path = PathBuf::from(qurl_to_local_path(folder_url));
        let output_path = folder_path.join(&filename.to_string());
        match output_path.to_str() {
            Some(p) => {
                save_to_file(content.to_string().as_bytes(), p);
                return true;
            },
            None => return false,
        }
    }

    pub fn check_file_exists_in_folder(&self,
                                       folder_url: &QUrl,
                                       filename: &QString) -> bool {
        let folder_path = PathBuf::from(qurl_to_local_path(folder_url));
        let output_path = folder_path.join(&filename.to_string());
        let exists = match check_file_exists_print_err(&output_path) {
            Ok(r) => r,
            Err(_) => false,
        };
        exists
    }

    pub fn markdown_to_html(&self, markdown_text: &QString) -> QString {
        QString::from(markdown_to_html(&markdown_text.to_string()))
    }

    pub fn open_sutta_search_window(&self) {
        use crate::api::ffi;
        ffi::callback_open_sutta_search_window(QString::from(""));
    }

    pub fn open_sutta_search_window_with_result(&self, result_data_json: &QString) {
        use crate::api::ffi;
        ffi::callback_open_sutta_search_window(result_data_json.clone());
    }

    pub fn open_sutta_languages_window(&self) {
        use crate::api::ffi;
        ffi::callback_open_sutta_languages_window();
    }

    pub fn open_library_window(&self) {
        use crate::api::ffi;
        ffi::callback_open_library_window();
    }

    pub fn open_reference_search_window(&self) {
        use crate::api::ffi;
        ffi::callback_open_reference_search_window();
    }

    pub fn get_all_books_json(&self) -> QString {
        let app_data = get_app_data();
        match app_data.dbm.appdata.get_all_books() {
            Ok(books) => {
                let json = serde_json::to_string(&books).unwrap_or_else(|_| "[]".to_string());
                QString::from(json)
            }
            Err(e) => {
                error(&format!("Failed to get books: {}", e));
                QString::from("[]")
            }
        }
    }

    pub fn get_book_by_uid_json(&self, book_uid: &QString) -> QString {
        let app_data = get_app_data();
        let uid = book_uid.to_string();
        match app_data.dbm.appdata.get_book_by_uid(&uid) {
            Ok(Some(book)) => {
                let json = serde_json::to_string(&book).unwrap_or_else(|_| "{}".to_string());
                QString::from(json)
            }
            Ok(None) => {
                error(&format!("Book not found: {}", uid));
                QString::from("{}")
            }
            Err(e) => {
                error(&format!("Failed to get book {}: {}", uid, e));
                QString::from("{}")
            }
        }
    }

    pub fn get_spine_items_for_book_json(&self, book_uid: &QString) -> QString {
        let app_data = get_app_data();
        let uid = book_uid.to_string();
        match app_data.dbm.appdata.get_spine_items_for_book(&uid) {
            Ok(items) => {
                let json = serde_json::to_string(&items).unwrap_or_else(|_| "[]".to_string());
                QString::from(json)
            }
            Err(e) => {
                error(&format!("Failed to get spine items for book {}: {}", uid, e));
                QString::from("[]")
            }
        }
    }

    pub fn get_spine_item_uid_by_path(&self, book_uid: &QString, resource_path: &QString) -> QString {
        let app_data = get_app_data();
        let book_uid_str = book_uid.to_string();
        let resource_path_str = resource_path.to_string();

        match app_data.dbm.appdata.get_book_spine_item_by_path(&book_uid_str, &resource_path_str) {
            Ok(Some(item)) => QString::from(&item.spine_item_uid),
            Ok(None) => {
                info(&format!("No spine item found for book {} at path {}", book_uid_str, resource_path_str));
                QString::from("")
            }
            Err(e) => {
                error(&format!("Failed to get spine item by path for book {} at path {}: {}", book_uid_str, resource_path_str, e));
                QString::from("")
            }
        }
    }

    pub fn get_book_spine_html(&self, window_id: &QString, spine_item_uid: &QString) -> QString {
        let app_data = get_app_data();
        let app_settings = app_data.app_settings_cache.read().expect("Failed to read app settings");
        let body_class = app_settings.theme_name_as_string();

        let blank_page_html = blank_html_page(Some(body_class.clone()));
        if spine_item_uid.trimmed().is_empty() {
            return QString::from(blank_page_html);
        }

        let uid = spine_item_uid.to_string();
        let spine_item = match app_data.dbm.appdata.get_book_spine_item(&uid) {
            Ok(Some(item)) => item,
            Ok(None) => return QString::from(blank_page_html),
            Err(e) => {
                error(&format!("Failed to get spine item {}: {}", uid, e));
                return QString::from(blank_page_html);
            }
        };

        let html = app_data.render_book_spine_item_html(&spine_item, Some(window_id.to_string()), None)
            .unwrap_or(sutta_html_page("Rendering error", None, None, None, Some(body_class)));

        QString::from(html)
    }

    pub fn check_book_uid_exists(&self, book_uid: &QString) -> bool {
        let app_data = get_app_data();
        let uid = book_uid.to_string();
        match app_data.dbm.appdata.get_book_by_uid(&uid) {
            Ok(Some(_)) => true,
            Ok(None) => false,
            Err(e) => {
                error(&format!("Failed to check book UID {}: {}", uid, e));
                false
            }
        }
    }

    /// Extract metadata (title and author) from a document file
    /// Returns a JSON string with "title" and "author" fields
    pub fn extract_document_metadata(&self, file_path: &QString) -> QString {
        use simsapa_backend::document_metadata;

        let path_str = file_path.to_string();
        let path = Path::new(&path_str);

        match document_metadata::extract_document_metadata(path) {
            Ok(metadata) => {
                let json = serde_json::json!({
                    "title": metadata.title,
                    "author": metadata.author
                });
                QString::from(json.to_string())
            }
            Err(e) => {
                error(&format!("Failed to extract metadata: {}", e));
                // Return empty metadata on error
                let json = serde_json::json!({
                    "title": "",
                    "author": ""
                });
                QString::from(json.to_string())
            }
        }
    }

    /// Copy content from a content:// URI to a temporary file (Android only)
    /// Returns the path to the temporary file, or empty string on error
    pub fn copy_content_uri_to_temp(&self, content_uri: &QString) -> QString {
        let uri_str = content_uri.to_string();

        // Only handle content:// URIs
        if !uri_str.starts_with("content://") {
            return QString::from("");
        }

        info(&format!("Copying content URI to temp file: {}", uri_str));

        // Call the C++ function to handle the actual copying
        let temp_path = qobject::copy_content_uri_to_temp_file(content_uri);

        if temp_path.is_empty() {
            error("Failed to copy content URI to temp file");
        } else {
            info(&format!("Successfully copied to: {}", temp_path.to_string()));
        }

        temp_path
    }

    /// Delete the temporary import folder and all its contents
    /// Returns true if successful, false otherwise
    pub fn delete_temp_import_folder(&self) -> bool {
        let temp_dir = std::env::temp_dir().join("simsapa-imports");

        // Use try_exists() instead of exists() to avoid Android permission crashes
        match temp_dir.try_exists() {
            Ok(true) => {
                // Folder exists, try to remove it
                match fs::remove_dir_all(&temp_dir) {
                    Ok(_) => {
                        info(&format!("Deleted temp import folder: {}", temp_dir.display()));
                        true
                    }
                    Err(e) => {
                        error(&format!("Failed to delete temp import folder {}: {}", temp_dir.display(), e));
                        false
                    }
                }
            }
            Ok(false) => {
                // Folder doesn't exist, consider success
                true
            }
            Err(e) => {
                // Error checking if folder exists, log and return false
                error(&format!("Failed to check if temp import folder exists: {}", e));
                false
            }
        }
    }

    pub fn is_spine_item_pdf(&self, spine_item_uid: &QString) -> bool {
        let app_data = get_app_data();
        let uid = spine_item_uid.to_string();

        // Get the spine item
        match app_data.dbm.appdata.get_book_spine_item(&uid) {
            Ok(Some(spine_item)) => {
                // Get the book to check document_type
                match app_data.dbm.appdata.get_book_by_uid(&spine_item.book_uid) {
                    Ok(Some(book)) => book.document_type == "pdf",
                    _ => false,
                }
            },
            _ => false,
        }
    }

    /// Get the book_uid for a given spine_item_uid
    pub fn get_book_uid_for_spine_item(&self, spine_item_uid: &QString) -> QString {
        let app_data = get_app_data();
        let uid = spine_item_uid.to_string();

        // Get the spine item and return its book_uid
        match app_data.dbm.appdata.get_book_spine_item(&uid) {
            Ok(Some(spine_item)) => QString::from(&spine_item.book_uid),
            _ => QString::from(""),
        }
    }

    pub fn import_document(self: Pin<&mut Self>, file_path: &QString, book_uid: &QString, title: &QString, author: &QString, document_type: &QString, split_tag: &QString) {
        let path_str = file_path.to_string();
        let uid_str = book_uid.to_string();
        let title_str = title.to_string();
        let author_str = author.to_string();
        let doc_type = document_type.to_string();
        let _split_tag_str = split_tag.to_string();

        info(&format!("import_document: {} as {} ({})", &path_str, &uid_str, &doc_type));

        let qt_thread = self.qt_thread();

        // Spawn thread for import
        thread::spawn(move || {
            let app_data = get_app_data();
            let path = Path::new(&path_str);

            // Convert custom title and author to Option<&str>
            let custom_title = if title_str.trim().is_empty() {
                None
            } else {
                Some(title_str.as_str())
            };
            let custom_author = if author_str.trim().is_empty() {
                None
            } else {
                Some(author_str.as_str())
            };

            let result = match doc_type.as_str() {
                "epub" => {
                    let progress_msg = QString::from("Importing EPUB...");
                    qt_thread.queue(move |mut qo| {
                        qo.as_mut().document_import_progress(progress_msg);
                    }).unwrap();

                    app_data.import_epub_to_db(path, &uid_str, custom_title, custom_author, None, None)
                }
                "pdf" => {
                    let progress_msg = QString::from("Importing PDF...");
                    qt_thread.queue(move |mut qo| {
                        qo.as_mut().document_import_progress(progress_msg);
                    }).unwrap();

                    app_data.import_pdf_to_db(path, &uid_str, custom_title, custom_author, None, None)
                }
                "html" => {
                    let progress_msg = QString::from("Importing HTML...");
                    qt_thread.queue(move |mut qo| {
                        qo.as_mut().document_import_progress(progress_msg);
                    }).unwrap();

                    // TODO: Pass split_tag parameter when html_import supports it
                    // For now, HTML is imported as a single spine item
                    app_data.import_html_to_db(path, &uid_str, custom_title, custom_author, None, None)
                }
                _ => {
                    let error_msg = format!("Unknown document type: {}", doc_type);
                    error(&error_msg);
                    let error_qstr = QString::from(&error_msg);
                    qt_thread.queue(move |mut qo| {
                        qo.as_mut().document_import_completed(false, error_qstr);
                    }).unwrap();
                    return;
                }
            };

            match result {
                Ok(_) => {
                    info(&format!("Successfully imported {}", &uid_str));
                    let success_msg = QString::from(format!("Successfully imported '{}'", &title_str));
                    qt_thread.queue(move |mut qo| {
                        qo.as_mut().document_import_completed(true, success_msg);
                    }).unwrap();
                }
                Err(e) => {
                    let error_msg = format!("Failed to import: {}", e);
                    error(&error_msg);
                    let error_qstr = QString::from(&error_msg);
                    qt_thread.queue(move |mut qo| {
                        qo.as_mut().document_import_completed(false, error_qstr);
                    }).unwrap();
                }
            }
        });
    }

    pub fn remove_book(&self, book_uid: &QString) -> bool {
        let uid = book_uid.to_string();
        info(&format!("remove_book: {}", &uid));

        let app_data = get_app_data();
        match app_data.dbm.appdata.delete_book_by_uid(&uid) {
            Ok(_) => {
                info(&format!("Successfully removed book: {}", &uid));
                true
            }
            Err(e) => {
                error(&format!("Failed to remove book {}: {}", &uid, e));
                false
            }
        }
    }

    pub fn get_book_metadata_json(&self, book_uid: &QString) -> QString {
        let app_data = get_app_data();
        let uid = book_uid.to_string();

        match app_data.dbm.appdata.get_book_by_uid(&uid) {
            Ok(Some(book)) => {
                let json = serde_json::json!({
                    "title": book.title,
                    "author": book.author,
                    "document_type": book.document_type,
                    "enable_embedded_css": book.enable_embedded_css
                });
                QString::from(json.to_string())
            }
            Ok(None) => {
                error(&format!("Book not found: {}", uid));
                let json = serde_json::json!({
                    "title": "",
                    "author": "",
                    "document_type": "",
                    "enable_embedded_css": true
                });
                QString::from(json.to_string())
            }
            Err(e) => {
                error(&format!("Failed to get book metadata {}: {}", uid, e));
                let json = serde_json::json!({
                    "title": "",
                    "author": "",
                    "document_type": "",
                    "enable_embedded_css": true
                });
                QString::from(json.to_string())
            }
        }
    }

    pub fn update_book_metadata(self: Pin<&mut Self>, book_uid: &QString, title: &QString, author: &QString, enable_embedded_css: bool) {
        let uid = book_uid.to_string();
        let title_str = title.to_string();
        let author_str = author.to_string();

        let qt_thread = self.qt_thread();

        thread::spawn(move || {
            let app_data = get_app_data();

            match app_data.dbm.appdata.update_book_metadata(&uid, &title_str, &author_str, enable_embedded_css) {
                Ok(_) => {
                    let success_msg = QString::from(format!("Successfully updated metadata for '{}'", &title_str));
                    qt_thread.queue(move |mut qo| {
                        qo.as_mut().book_metadata_updated(true, success_msg);
                    }).unwrap();
                }
                Err(e) => {
                    let error_msg = format!("Failed to update book metadata: {}", e);
                    error(&error_msg);
                    let error_qstr = QString::from(&error_msg);
                    qt_thread.queue(move |mut qo| {
                        qo.as_mut().book_metadata_updated(false, error_qstr);
                    }).unwrap();
                }
            }
        });
    }

    /// Helper function to create error response JSON for background processing
    fn create_error_response(error_message: &str) -> String {
        let error_response = simsapa_backend::types::BackgroundProcessingError {
            success: false,
            error: error_message.to_string(),
        };

        match serde_json::to_string(&error_response) {
            Ok(json) => json,
            Err(_) => {
                // Fallback to simple JSON if serialization fails
                format!(r#"{{"success":false,"error":"{}"}}"#, error_message.replace('"', r#"\""#))
            }
        }
    }

    /// Process all paragraphs in background thread
    pub fn process_all_paragraphs_background(self: Pin<&mut Self>, input_json: &QString) {
        let input_json = input_json.to_string();
        let self_ = self.qt_thread();

        thread::spawn(move || {
            // Parse input JSON directly into typed struct
            let input_data: simsapa_backend::types::AllParagraphsProcessingInput = match serde_json::from_str(&input_json) {
                Ok(data) => data,
                Err(e) => {
                    let error_response = Self::create_error_response(&format!("Failed to parse input JSON: {}", e));
                    self_.queue(move |mut qo| {
                        qo.as_mut().all_paragraphs_gloss_ready(QString::from(error_response));
                    }).unwrap();
                    return;
                }
            };

            // Get app data for DPD database access
            let app_data = simsapa_backend::get_app_data();

            let mut paragraph_results: Vec<simsapa_backend::types::ParagraphProcessingResult> = Vec::new();
            let mut global_unrecognized_words = input_data.options.existing_global_unrecognized.clone();
            let mut global_stems = input_data.options.existing_global_stems.clone();
            let mut paragraph_unrecognized_words = input_data.options.existing_paragraph_unrecognized.clone();

            // Process each paragraph
            for (paragraph_idx, paragraph_text) in input_data.paragraphs.iter().enumerate() {

                // Extract words with context from paragraph
                let words_with_context = simsapa_backend::helpers::extract_words_with_context(paragraph_text);
                let mut paragraph_shown_stems = std::collections::HashMap::new();
                let mut processed_words = Vec::new();

                // Process each word
                for word_context in words_with_context {
                    let word_info = simsapa_backend::types::WordInfo {
                        word: word_context.clean_word.clone(),
                        sentence: word_context.context_snippet.clone(),
                    };

                    match simsapa_backend::helpers::process_word_for_glossing(
                        &word_info,
                        &mut paragraph_shown_stems,
                        &mut global_stems,
                        input_data.options.no_duplicates_globally,
                        &input_data.options,
                        &app_data.dbm.dpd,
                    ) {
                        Ok(result) => processed_words.push(result),
                        Err(e) => {
                            let error_response = Self::create_error_response(&format!("Word processing error: {}", e));
                            self_.queue(move |mut qo| {
                                qo.as_mut().all_paragraphs_gloss_ready(QString::from(error_response));
                            }).unwrap();
                            return;
                        }
                    }
                }

                // Collect unrecognized words for this paragraph
                simsapa_backend::helpers::collect_unrecognized_words(
                    &processed_words,
                    paragraph_idx,
                    &mut paragraph_unrecognized_words,
                    &mut global_unrecognized_words,
                );

                // Collect recognized words data
                let words_data: Vec<simsapa_backend::types::ProcessedWord> = processed_words
                    .into_iter()
                    .filter_map(|result| {
                        if let Some(simsapa_backend::types::WordProcessingResult::Recognized(word)) = result {
                            Some(word)
                        } else {
                            None
                        }
                    })
                    .collect();

                let paragraph_unrecognized = paragraph_unrecognized_words
                    .get(&paragraph_idx.to_string())
                    .cloned()
                    .unwrap_or_default();

                paragraph_results.push(simsapa_backend::types::ParagraphProcessingResult {
                    paragraph_index: paragraph_idx,
                    words_data,
                    unrecognized_words: paragraph_unrecognized,
                });
            }

            // Create success response
            let response = simsapa_backend::types::AllParagraphsProcessingResult {
                success: true,
                paragraphs: paragraph_results,
                global_unrecognized_words,
                updated_global_stems: global_stems,
            };

            let response_json = match serde_json::to_string(&response) {
                Ok(json) => json,
                Err(e) => {
                    let error_response = Self::create_error_response(&format!("Failed to serialize response: {}", e));
                    self_.queue(move |mut qo| {
                        qo.as_mut().all_paragraphs_gloss_ready(QString::from(error_response));
                    }).unwrap();
                    return;
                }
            };

            self_.queue(move |mut qo| {
                qo.as_mut().all_paragraphs_gloss_ready(QString::from(response_json));
            }).unwrap();
        });
    }

    /// Process a single paragraph in background thread
    pub fn process_paragraph_background(self: Pin<&mut Self>, paragraph_index: i32, input_json: &QString) {
        let input_json = input_json.to_string();
        let self_ = self.qt_thread();

        thread::spawn(move || {
            // Parse input JSON directly into typed struct
            let input_data: simsapa_backend::types::SingleParagraphProcessingInput = match serde_json::from_str(&input_json) {
                Ok(data) => data,
                Err(e) => {
                    let error_response = Self::create_error_response(&format!("Failed to parse input JSON: {}", e));
                    self_.queue(move |mut qo| {
                        qo.as_mut().paragraph_gloss_ready(paragraph_index, QString::from(error_response));
                    }).unwrap();
                    return;
                }
            };

            // Get app data for DPD database access
            let app_data = simsapa_backend::get_app_data();

            // Extract words with context from paragraph
            let words_with_context = simsapa_backend::helpers::extract_words_with_context(&input_data.paragraph_text);
            let mut paragraph_shown_stems = std::collections::HashMap::new();
            let mut global_stems = input_data.options.existing_global_stems.clone();
            let mut processed_words = Vec::new();

            // Process each word
            for word_context in words_with_context {
                let word_info = simsapa_backend::types::WordInfo {
                    word: word_context.clean_word.clone(),
                    sentence: word_context.context_snippet.clone(),
                };

                match simsapa_backend::helpers::process_word_for_glossing(
                    &word_info,
                    &mut paragraph_shown_stems,
                    &mut global_stems,
                    input_data.options.no_duplicates_globally,
                    &input_data.options,
                    &app_data.dbm.dpd,
                ) {
                    Ok(result) => processed_words.push(result),
                    Err(e) => {
                        let error_response = Self::create_error_response(&format!("Word processing error: {}", e));
                        self_.queue(move |mut qo| {
                            qo.as_mut().paragraph_gloss_ready(paragraph_index, QString::from(error_response));
                        }).unwrap();
                        return;
                    }
                }
            }

            // Collect unrecognized words
            let mut paragraph_unrecognized_words = std::collections::HashMap::new();
            let mut global_unrecognized_words = input_data.options.existing_global_unrecognized.clone();
            simsapa_backend::helpers::collect_unrecognized_words(
                &processed_words,
                paragraph_index as usize,
                &mut paragraph_unrecognized_words,
                &mut global_unrecognized_words,
            );

            // Collect recognized words data
            let words_data: Vec<simsapa_backend::types::ProcessedWord> = processed_words
                .into_iter()
                .filter_map(|result| {
                    if let Some(simsapa_backend::types::WordProcessingResult::Recognized(word)) = result {
                        Some(word)
                    } else {
                        None
                    }
                })
                .collect();

            let paragraph_unrecognized = paragraph_unrecognized_words
                .get(&paragraph_index.to_string())
                .cloned()
                .unwrap_or_default();

            // Create success response
            let response = simsapa_backend::types::SingleParagraphProcessingResult {
                success: true,
                paragraph_index: paragraph_index as usize,
                words_data,
                unrecognized_words: paragraph_unrecognized,
                updated_global_stems: global_stems,
            };

            let response_json = match serde_json::to_string(&response) {
                Ok(json) => json,
                Err(e) => {
                    let error_response = Self::create_error_response(&format!("Failed to serialize response: {}", e));
                    self_.queue(move |mut qo| {
                        qo.as_mut().paragraph_gloss_ready(paragraph_index, QString::from(error_response));
                    }).unwrap();
                    return;
                }
            };

            self_.queue(move |mut qo| {
                qo.as_mut().paragraph_gloss_ready(paragraph_index, QString::from(response_json));
            }).unwrap();
        });
    }

    /// Get Anki template for Front side
    pub fn get_anki_template_front(&self) -> QString {
        let app_data = get_app_data();
        let template = app_data.get_anki_template_front();
        QString::from(template)
    }

    /// Set Anki template for Front side
    pub fn set_anki_template_front(self: Pin<&mut Self>, template_str: &QString) {
        let app_data = get_app_data();
        app_data.set_anki_template_front(&template_str.to_string());
    }

    /// Get Anki template for Back side
    pub fn get_anki_template_back(&self) -> QString {
        let app_data = get_app_data();
        let template = app_data.get_anki_template_back();
        QString::from(template)
    }

    /// Set Anki template for Back side
    pub fn set_anki_template_back(self: Pin<&mut Self>, template_str: &QString) {
        let app_data = get_app_data();
        app_data.set_anki_template_back(&template_str.to_string());
    }

    /// Get Anki template for Cloze Front side
    pub fn get_anki_template_cloze_front(&self) -> QString {
        let app_data = get_app_data();
        let template = app_data.get_anki_template_cloze_front();
        QString::from(template)
    }

    /// Set Anki template for Cloze Front side
    pub fn set_anki_template_cloze_front(self: Pin<&mut Self>, template_str: &QString) {
        let app_data = get_app_data();
        app_data.set_anki_template_cloze_front(&template_str.to_string());
    }

    /// Get Anki template for Cloze Back side
    pub fn get_anki_template_cloze_back(&self) -> QString {
        let app_data = get_app_data();
        let template = app_data.get_anki_template_cloze_back();
        QString::from(template)
    }

    /// Set Anki template for Cloze Back side
    pub fn set_anki_template_cloze_back(self: Pin<&mut Self>, template_str: &QString) {
        let app_data = get_app_data();
        app_data.set_anki_template_cloze_back(&template_str.to_string());
    }

    /// Get Anki export format (Simple, Templated, DataCsv)
    pub fn get_anki_export_format(&self) -> QString {
        let app_data = get_app_data();
        let format = app_data.get_anki_export_format();
        QString::from(format)
    }

    /// Set Anki export format
    pub fn set_anki_export_format(self: Pin<&mut Self>, format: &QString) {
        let app_data = get_app_data();
        app_data.set_anki_export_format(&format.to_string());
    }

    /// Get whether to include cloze format in Anki export
    pub fn get_anki_include_cloze(&self) -> bool {
        let app_data = get_app_data();
        app_data.get_anki_include_cloze()
    }

    /// Set whether to include cloze format in Anki export
    pub fn set_anki_include_cloze(self: Pin<&mut Self>, include: bool) {
        let app_data = get_app_data();
        app_data.set_anki_include_cloze(include);
    }

    /// Get sample vocabulary data for preview (hardcoded abhivādetvā)
    pub fn get_sample_vocabulary_data_json(&self) -> QString {
        let sample_json = simsapa_backend::anki_sample_data::get_sample_vocabulary_data_json();
        QString::from(sample_json)
    }

    /// Get DPD headword data by UID
    pub fn get_dpd_headword_by_uid(&self, uid: &QString) -> QString {
        let app_data = get_app_data();
        let uid_str = uid.to_string();

        match app_data.get_dpd_headword_by_uid(&uid_str) {
            Some(json) => QString::from(json),
            None => QString::from("{}"),
        }
    }

    pub fn export_anki_csv_background(self: Pin<&mut Self>, input_json: &QString) {
        info("SuttaBridge::export_anki_csv_background() start");
        let qt_thread = self.qt_thread();
        let input_json_str = input_json.to_string();

        thread::spawn(move || {
            let app_data = get_app_data();

            let input: simsapa_backend::types::AnkiCsvExportInput = match serde_json::from_str(&input_json_str) {
                Ok(data) => data,
                Err(e) => {
                    let error_response = simsapa_backend::types::AnkiCsvExportResult {
                        success: false,
                        files: vec![],
                        error: Some(format!("Failed to parse input JSON: {}", e)),
                    };
                    let error_json = serde_json::to_string(&error_response).unwrap_or_default();
                    qt_thread.queue(move |mut qo| {
                        qo.as_mut().anki_csv_export_ready(QString::from(error_json));
                    }).unwrap();
                    return;
                }
            };

            let result = match simsapa_backend::anki_export::export_anki_csv(input, &app_data) {
                Ok(res) => res,
                Err(e) => simsapa_backend::types::AnkiCsvExportResult {
                    success: false,
                    files: vec![],
                    error: Some(format!("Export failed: {}", e)),
                },
            };

            let result_json = match serde_json::to_string(&result) {
                Ok(json) => json,
                Err(e) => {
                    let error_response = simsapa_backend::types::AnkiCsvExportResult {
                        success: false,
                        files: vec![],
                        error: Some(format!("Failed to serialize result: {}", e)),
                    };
                    serde_json::to_string(&error_response).unwrap_or_default()
                }
            };

            qt_thread.queue(move |mut qo| {
                qo.as_mut().anki_csv_export_ready(QString::from(result_json));
            }).unwrap();

            info("SuttaBridge::export_anki_csv_background() end");
        });
    }

    pub fn render_anki_preview_background(self: Pin<&mut Self>, front_template: &QString, back_template: &QString) {
        info("SuttaBridge::render_anki_preview_background() start");
        let qt_thread = self.qt_thread();
        let front_template_str = front_template.to_string();
        let back_template_str = back_template.to_string();

        thread::spawn(move || {
            let app_data = get_app_data();
            let sample_json = simsapa_backend::anki_sample_data::get_sample_vocabulary_data_json();

            let preview_html = match simsapa_backend::anki_export::render_anki_preview(
                &sample_json,
                &front_template_str,
                &back_template_str,
                &app_data,
            ) {
                Ok(html) => html,
                Err(e) => format!("<span style='color: red;'>Preview error: {}</span>", e),
            };

            qt_thread.queue(move |mut qo| {
                qo.as_mut().anki_preview_ready(QString::from(preview_html));
            }).unwrap();

            info("SuttaBridge::render_anki_preview_background() end");
        });
    }

    pub fn get_search_as_you_type(&self) -> bool {
        let app_data = get_app_data();
        app_data.get_search_as_you_type()
    }

    pub fn set_search_as_you_type(self: Pin<&mut Self>, enabled: bool) {
        let app_data = get_app_data();
        app_data.set_search_as_you_type(enabled);
    }

    pub fn get_open_find_in_sutta_results(&self) -> bool {
        let app_data = get_app_data();
        app_data.get_open_find_in_sutta_results()
    }

    pub fn set_open_find_in_sutta_results(self: Pin<&mut Self>, enabled: bool) {
        let app_data = get_app_data();
        app_data.set_open_find_in_sutta_results(enabled);
    }

    pub fn get_sutta_language_labels(&self) -> QStringList {
        let app_data = get_app_data();
        let languages = app_data.dbm.get_sutta_languages();

        let mut res = QStringList::default();
        for lang in languages {
            res.append(QString::from(lang));
        }
        res
    }

    /// Get sutta languages with their counts in format "code|Name|Count"
    pub fn get_sutta_language_labels_with_counts(&self) -> QStringList {
        let app_data = get_app_data();
        let labels = app_data.dbm.get_sutta_language_labels_with_counts();

        let mut res = QStringList::default();
        for label in labels {
            res.append(QString::from(label));
        }
        res
    }

    pub fn get_sutta_language_filter_key(&self) -> QString {
        let app_data = get_app_data();
        let app_settings = app_data.app_settings_cache.read().expect("Failed to read app settings");
        QString::from(&app_settings.sutta_language_filter_key)
    }

    pub fn set_sutta_language_filter_key(self: Pin<&mut Self>, key: QString) {
        let app_data = get_app_data();
        app_data.set_sutta_language_filter_key(key.to_string());
    }

    /// Get the mobile top bar margin value
    /// Returns either system value (from get_status_bar_height) or custom value
    /// Returns a default value of 24 if APP_DATA is not yet initialized
    pub fn get_mobile_top_bar_margin(&self) -> i32 {
        // Return default value if APP_DATA is not yet initialized
        // This can happen when QML components load before init_app_data() is called
        let app_data = match try_get_app_data() {
            Some(data) => data,
            None => return 24,
        };

        let app_settings = app_data.app_settings_cache.read().expect("Failed to read app settings");

        use simsapa_backend::app_settings::MobileTopBarMargin;
        match app_settings.mobile_top_bar_margin {
            MobileTopBarMargin::SystemValue => {
                use crate::api::ffi;
                ffi::get_status_bar_height()
            }
            MobileTopBarMargin::CustomValue(value) => value as i32,
        }
    }

    pub fn is_mobile_top_bar_margin_system(&self) -> bool {
        // Return default (true for system value) if APP_DATA is not yet initialized
        let app_data = match try_get_app_data() {
            Some(data) => data,
            None => return true,
        };

        let app_settings = app_data.app_settings_cache.read().expect("Failed to read app settings");
        app_settings.is_mobile_top_bar_margin_system()
    }

    pub fn get_mobile_top_bar_margin_custom_value(&self) -> u32 {
        // Return default custom value of 24 if APP_DATA is not yet initialized
        let app_data = match try_get_app_data() {
            Some(data) => data,
            None => return 24,
        };

        let app_settings = app_data.app_settings_cache.read().expect("Failed to read app settings");
        app_settings.get_mobile_top_bar_margin_custom_value()
    }

    pub fn set_mobile_top_bar_margin_system(self: Pin<&mut Self>) {
        let app_data = get_app_data();
        app_data.set_mobile_top_bar_margin_system();
    }

    pub fn set_mobile_top_bar_margin_custom(self: Pin<&mut Self>, value: u32) {
        let app_data = get_app_data();
        app_data.set_mobile_top_bar_margin_custom(value);
    }

    pub fn search_reference(&self, query: &QString, field: &QString) -> QString {
        use simsapa_backend::pts_reference_search;

        let results = pts_reference_search::search(
            &query.to_string(),
            &field.to_string()
        );

        match serde_json::to_string(&results) {
            Ok(json) => QString::from(json),
            Err(e) => {
                error(&format!("Failed to serialize reference search results: {}", e));
                QString::from("[]")
            }
        }
    }

    pub fn extract_uid_from_url(&self, url: &QString) -> QString {
        let url_str = url.to_string();

        // Extract UID from SuttaCentral URL
        // Examples:
        // - https://suttacentral.net/sn56.102 -> sn56.102
        // - https://suttacentral.net/dn1/en/sujato -> dn1
        // - sn56.102 -> sn56.102 (pass through if already just UID)

        if let Some(path_start) = url_str.find("suttacentral.net/") {
            let path = &url_str[path_start + 17..]; // Skip "suttacentral.net/"

            // Take everything up to the first '/' or end of string
            if let Some(slash_pos) = path.find('/') {
                QString::from(&path[..slash_pos])
            } else {
                QString::from(path)
            }
        } else {
            // If it's not a URL, return as-is (might already be a UID)
            QString::from(url_str)
        }
    }

    pub fn get_full_sutta_uid(&self, partial_uid: &QString) -> QString {
        let partial_str = partial_uid.to_string();
        let app_data = get_app_data();

        match app_data.dbm.appdata.get_full_sutta_uid(&partial_str) {
            Some(full_uid) => QString::from(full_uid),
            None => QString::from(""), // Return empty string if not found
        }
    }

    pub fn get_sutta_reference_info(&self, uid: &QString) -> QString {
        let uid_str = uid.to_string();
        let app_data = get_app_data();

        if let Some(sutta) = app_data.dbm.appdata.get_sutta(&uid_str) {
            let info = serde_json::json!({
                "uid": sutta.uid,
                "sutta_ref": sutta.sutta_ref,
                "title": sutta.title.clone().unwrap_or_default(),
                // FIXME: bootstrap issue: 'title_pali' is empty in the db, 'title' is used
                "title_pali": sutta.title.unwrap_or_default(),
            });
            QString::from(serde_json::to_string(&info).unwrap_or_else(|_| "{}".to_string()))
        } else {
            QString::from("{}")
        }
    }

    // ========================================================================
    // Update Checker Functions
    // ========================================================================

    /// Check for application and database updates.
    ///
    /// Spawns a background thread to perform the update check and emits
    /// appropriate signals when complete.
    ///
    /// # Arguments
    ///
    /// * `include_no_updates` - If true, emits noUpdatesAvailable signal when no updates found
    /// * `screen_size` - Screen resolution string (e.g., "1920 x 1080") for analytics, can be empty
    /// * `save_stats_behaviour` - Controls stats saving: "enabled", "disabled", or "determine"
    pub fn check_for_updates(self: Pin<&mut Self>, include_no_updates: bool, screen_size: &QString, save_stats_behaviour: &QString) {
        use simsapa_backend::update_checker::{self, SaveStatsBehaviour};

        info("SuttaBridge::check_for_updates() start");
        let qt_thread = self.qt_thread();

        // Convert screen_size to Option<String> for the background thread
        let screen_size_str = screen_size.to_string();
        let screen_size_opt: Option<String> = if screen_size_str.is_empty() {
            None
        } else {
            Some(screen_size_str)
        };

        // Parse save_stats_behaviour from string
        let stats_behaviour = SaveStatsBehaviour::from_str(&save_stats_behaviour.to_string());

        thread::spawn(move || {
            // Get current app and db versions
            let app_version = update_checker::get_app_version();
            let db_version = update_checker::get_db_version();

            // First check if local db is obsolete (incompatible with app)
            if let Some(obsolete_info) = update_checker::is_local_db_obsolete(
                &app_version,
                db_version.as_deref(),
            ) {
                let json = serde_json::to_string(&obsolete_info).unwrap_or_default();
                qt_thread.queue(move |mut qo| {
                    qo.as_mut().local_db_obsolete(QString::from(json));
                    qo.as_mut().releases_check_completed();
                }).unwrap();
                info("SuttaBridge::check_for_updates() - local db obsolete");
                return;
            }

            // Try to fetch release information
            let releases_info = match update_checker::fetch_releases_info(screen_size_opt.as_deref(), stats_behaviour) {
                Ok(info) => {
                    // Save the successfully fetched releases info to the global
                    simsapa_backend::set_releases_info(info.clone());
                    info
                },
                Err(e) => {
                    let error_msg = format!("Failed to fetch updates: {}", e);
                    error(&error_msg);
                    qt_thread.queue(move |mut qo| {
                        qo.as_mut().update_check_error(QString::from(error_msg));
                        qo.as_mut().releases_check_completed();
                    }).unwrap();
                    info("SuttaBridge::check_for_updates() - fetch error");
                    return;
                }
            };

            // Check for app update
            if let Some(mut app_update) = update_checker::has_app_update(&releases_info, &app_version) {
                // Convert release_notes from markdown to HTML
                if let Some(ref notes) = app_update.release_notes {
                    app_update.release_notes = Some(markdown_to_html(notes));
                }
                let json = serde_json::to_string(&app_update).unwrap_or_default();
                qt_thread.queue(move |mut qo| {
                    qo.as_mut().app_update_available(QString::from(json));
                    qo.as_mut().releases_check_completed();
                }).unwrap();
                info("SuttaBridge::check_for_updates() - app update available");
                return;
            }

            // Check for db update
            if let Some(mut db_update) = update_checker::has_db_update(
                &releases_info,
                &app_version,
                db_version.as_deref(),
            ) {
                // Convert release_notes from markdown to HTML
                if let Some(ref notes) = db_update.release_notes {
                    db_update.release_notes = Some(markdown_to_html(notes));
                }
                let json = serde_json::to_string(&db_update).unwrap_or_default();
                qt_thread.queue(move |mut qo| {
                    qo.as_mut().db_update_available(QString::from(json));
                    qo.as_mut().releases_check_completed();
                }).unwrap();
                info("SuttaBridge::check_for_updates() - db update available");
                return;
            }

            // No updates available
            if include_no_updates {
                qt_thread.queue(move |mut qo| {
                    qo.as_mut().no_updates_available();
                    qo.as_mut().releases_check_completed();
                }).unwrap();
            } else {
                qt_thread.queue(move |mut qo| {
                    qo.as_mut().releases_check_completed();
                }).unwrap();
            }

            info("SuttaBridge::check_for_updates() - no updates available");
        });
    }

    /// Get whether to notify about Simsapa updates.
    pub fn get_notify_about_simsapa_updates(&self) -> bool {
        let app_data = get_app_data();
        app_data.get_notify_about_simsapa_updates()
    }

    /// Set whether to notify about Simsapa updates.
    pub fn set_notify_about_simsapa_updates(self: Pin<&mut Self>, enabled: bool) {
        let app_data = get_app_data();
        app_data.set_notify_about_simsapa_updates(enabled);
    }

    /// Prepare for database upgrade by exporting user data and creating marker files.
    ///
    /// First exports user data (app_settings, download_languages, user-imported books)
    /// to the import-me folder for restoration after the upgrade.
    ///
    /// Can't delete the db and index without triggering file-lock problems on Windows.
    /// Write a file to assets to signal deleting them on next start.
    ///
    /// This creates two marker files in the simsapa directory:
    /// - `delete_files_for_upgrade.txt`: Signals the app to delete old database files on next startup
    /// - `auto_start_download.txt`: Signals the app to automatically start the download on next startup
    ///
    /// The user should quit the app after calling this function and restart it
    /// to begin the database download process.
    pub fn prepare_for_database_upgrade(&self) {
        let app_data = get_app_data();

        // Export user data to import-me folder
        if let Err(e) = app_data.export_user_data_to_assets() {
            error(&format!("Failed to export user data: {}", e));
            // Continue anyway - we still want to trigger the upgrade even if export fails
        }

        let globals = get_app_globals();

        // Create delete_files_for_upgrade.txt marker file
        let delete_marker_path = &globals.paths.delete_files_for_upgrade_marker;
        if let Err(e) = std::fs::write(delete_marker_path, "") {
            error(&format!("Failed to create delete_files_for_upgrade.txt: {}", e));
        } else {
            info(&format!("Created marker file: {}", delete_marker_path.display()));
        }

        // Create auto_start_download.txt marker file
        let auto_download_path = &globals.paths.auto_start_download_marker;
        if let Err(e) = std::fs::write(auto_download_path, "") {
            error(&format!("Failed to create auto_start_download.txt: {}", e));
        } else {
            info(&format!("Created marker file: {}", auto_download_path.display()));
        }
    }

    /// Get the version_tag of the highest compatible assets release.
    /// Returns empty string if releases info has not been fetched or no compatible release found.
    pub fn get_compatible_asset_version_tag(&self) -> QString {
        use simsapa_backend::update_checker;

        // Try to get the releases info from the global
        if let Some(releases_info) = simsapa_backend::try_get_releases_info() {
            let app_version_str = update_checker::get_app_version();
            if let Ok(app_version) = update_checker::to_version(&app_version_str) {
                // Find the latest compatible assets release
                if let Some(release) = update_checker::get_latest_app_compatible_assets_release(&releases_info, &app_version) {
                    return QString::from(&release.version_tag);
                }
            }
        }

        // Return empty string when releases info is not available
        QString::from("")
    }

    /// Get the github_repo of the highest compatible assets release.
    /// Returns empty string if releases info has not been fetched or no compatible release found.
    pub fn get_compatible_asset_github_repo(&self) -> QString {
        use simsapa_backend::update_checker;

        // Try to get the releases info from the global
        if let Some(releases_info) = simsapa_backend::try_get_releases_info() {
            let app_version_str = update_checker::get_app_version();
            if let Ok(app_version) = update_checker::to_version(&app_version_str) {
                // Find the latest compatible assets release
                if let Some(release) = update_checker::get_latest_app_compatible_assets_release(&releases_info, &app_version) {
                    return QString::from(&release.github_repo);
                }
            }
        }

        // Return empty string when releases info is not available
        QString::from("")
    }
}
