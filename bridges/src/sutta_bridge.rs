use std::fs;
use std::path::PathBuf;
use std::thread;

use core::pin::Pin;
use cxx_qt_lib::{QString, QStringList, QUrl};
use cxx_qt::Threading;
use regex::{Regex, Captures};
use lazy_static::lazy_static;

use simsapa_backend::query_task::SearchQueryTask;
use simsapa_backend::types::{SearchArea, SearchMode, SearchParams, SearchResultPage};
use simsapa_backend::theme_colors::ThemeColors;
use simsapa_backend::{get_app_data, get_create_simsapa_dir, is_mobile, save_to_file, check_file_exists_print_err};
use simsapa_backend::html_content::{sutta_html_page, blank_html_page};
use simsapa_backend::dir_list::{generate_html_directory_listing, generate_plain_directory_listing};
use simsapa_backend::helpers::{extract_words, normalize_query_text, query_text_to_uid_field_query};

use simsapa_backend::logger::{info, error};

static DICTIONARY_JS: &'static str = include_str!("../../assets/js/dictionary.js");
static DICTIONARY_CSS: &'static str = include_str!("../../assets/css/dictionary.css");

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
    }

    impl cxx_qt::Threading for SuttaBridge{}

    extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qml_singleton]
        #[qproperty(bool, db_loaded)]
        #[namespace = "sutta_bridge"]
        type SuttaBridge = super::SuttaBridgeRust;

        #[qsignal]
        #[cxx_name = "updateWindowTitle"]
        fn update_window_title(self: Pin<&mut SuttaBridge>, sutta_uid: QString, sutta_ref: QString, sutta_title: QString);

        #[qinvokable]
        fn emit_update_window_title(self: Pin<&mut SuttaBridge>, sutta_uid: QString, sutta_ref: QString, sutta_title: QString);

        #[qinvokable]
        fn load_db(self: Pin<&mut SuttaBridge>);

        #[qinvokable]
        fn appdata_first_query(self: Pin<&mut SuttaBridge>);

        #[qinvokable]
        fn dpd_first_query(self: Pin<&mut SuttaBridge>);

        #[qinvokable]
        fn query_text_to_uid_field_query(self: &SuttaBridge, query_text: &QString) -> QString;

        #[qinvokable]
        fn results_page(self: &SuttaBridge, query: &QString, page_num: usize, params_json: &QString) -> QString;

        #[qinvokable]
        fn extract_words(self: &SuttaBridge, text: &QString) -> QStringList;

        #[qinvokable]
        fn normalize_query_text(self: &SuttaBridge, text: &QString) -> QString;

        #[qinvokable]
        fn dpd_deconstructor_list(self: &SuttaBridge, query: &QString) -> QStringList;

        #[qinvokable]
        fn dpd_lookup_json(self: &SuttaBridge, query: &QString) -> QString;

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
        fn save_file(self: &SuttaBridge, folder_url: &QUrl, filename: &QString, content: &QString) -> bool;

        #[qinvokable]
        fn check_file_exists_in_folder(self: &SuttaBridge, folder_url: &QUrl, filename: &QString) -> bool;
    }
}

pub struct SuttaBridgeRust {
    db_loaded: bool,
}

impl Default for SuttaBridgeRust {
    fn default() -> Self {
        Self {
            db_loaded: false,
        }
    }
}

impl qobject::SuttaBridge {
    pub fn emit_update_window_title(self: Pin<&mut Self>, sutta_uid: QString, sutta_ref: QString, sutta_title: QString) {
        // info(&format!("emit_update_window_title(): {} {} {}", &sutta_uid.to_string(), &sutta_ref.to_string(), &sutta_title.to_string()));
        self.update_window_title(sutta_uid, sutta_ref, sutta_title);
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

    /// Runs a db query so that db is cached from the disk. It should finish by
    /// the time the user types in the first actual query, and that will respond
    /// faster.
    pub fn appdata_first_query(self: Pin<&mut Self>) {
        info("SuttaBridge::appdata_first_query() start");
        thread::spawn(move || {
            let app_data = get_app_data();

            let params = SearchParams {
                mode: SearchMode::ContainsMatch,
                page_len: None,
                lang: Some("en".to_string()),
                lang_include: true,
                source: None,
                source_include: true,
                enable_regex: false,
                fuzzy_distance: 0,
            };

            let mut query_task = SearchQueryTask::new(
                &app_data.dbm,
                "en".to_string(),
                "dhamma".to_string(),
                params,
                SearchArea::Suttas,
            );

            let _results = match query_task.results_page(0) {
                Ok(_) => {},
                Err(e) => {
                    error(&format!("{}", e));
                }
            };

            info("SuttaBridge::appdata_first_query() end");
        });
    }

    pub fn dpd_first_query(self: Pin<&mut Self>) {
        info("SuttaBridge::dpd_first_query() start");
        thread::spawn(move || {
            let app_data = get_app_data();
            let _json = app_data.dbm.dpd.dpd_lookup_json("dhamma");
            info("SuttaBridge::dpd_first_query() end");
        });
    }

    pub fn query_text_to_uid_field_query(&self, query_text: &QString) -> QString {
        QString::from(query_text_to_uid_field_query(&query_text.to_string()))
    }

    pub fn results_page(&self, query: &QString, page_num: usize, params_json: &QString) -> QString {
        // FIXME: Can't store the query_task on SuttaBridgeRust
        // because it SearchQueryTask includes &'a DbManager reference.
        // Store only a connection pool?
        let app_data = get_app_data();

        let params: SearchParams = serde_json::from_str(&params_json.to_string()).unwrap_or_default();

        // FIXME: We have to create a SearchQueryTask for each search until we
        // can store it on SuttaBridgeRust.
        let mut query_task = SearchQueryTask::new(
            &app_data.dbm,
            "en".to_string(),
            query.to_string(),
            params,
            SearchArea::Suttas,
        );

        let results = match query_task.results_page(page_num) {
            Ok(x) => x,
            Err(e) => {
                error(&format!("{}", e));
                return QString::from("");
            }
        };

        let results_page = SearchResultPage {
            total_hits: query_task.total_hits() as usize,
            page_len: query_task.page_len as usize,
            page_num,
            results,
        };

        let json = serde_json::to_string(&results_page).unwrap_or_default();
        QString::from(json)
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
                let js_extra = format!("const WINDOW_ID = '{}';", &window_id.to_string());

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
        }

        let html = match word {
            Some(word) => match word.definition_html {
                Some(ref definition_html) => {
                    let mut js_extra = "".to_string();
                    js_extra.push_str(&format!(" const API_URL = '{}';", &app_data.api_url));
                    js_extra.push_str(&format!(" const WINDOW_ID = '{}';", &window_id.to_string()));
                    js_extra.push_str(&format!(" const IS_MOBILE = {};", is_mobile()));
                    js_extra.push_str(DICTIONARY_JS);

                    let mut word_html = definition_html.clone();

                    word_html = word_html.replace(
                        "</head>",
                        &format!(r#"<style>{}</style><script>{}</script></head>"#, DICTIONARY_CSS, js_extra));

                    word_html = word_html.replace(
                        "<body>",
                        &format!(r#"
<body>
    <div class='word-heading'>
        <div class='word-title'>
            <h1>{}</h1>
        </div>
    </div>"#, word.word()));

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

    pub fn update_gloss_session(&self, session_uid: &QString, gloss_data_json: &QString) {
        return
    }

    pub fn save_new_gloss_session(&self, gloss_data_json: &QString) -> QString {
        QString::from("session-uid")
    }

    pub fn save_anki_csv(&self, csv_content: &QString) -> QString {
        QString::from("file_name.csv")
    }

    pub fn save_file(&self,
                     folder_url: &QUrl,
                     filename: &QString,
                     content: &QString) -> bool {
        let folder_path = PathBuf::from(folder_url.path().to_string());
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
        let folder_path = PathBuf::from(folder_url.path().to_string());
        let output_path = folder_path.join(&filename.to_string());
        let exists = match check_file_exists_print_err(&output_path) {
            Ok(r) => r,
            Err(_) => false,
        };
        exists
    }
}
