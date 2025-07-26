use std::fs;
use std::path::PathBuf;
use std::thread;

use core::pin::Pin;
use cxx_qt_lib::{QString, QStringList};
use cxx_qt::Threading;

use simsapa_backend::query_task::SearchQueryTask;
use simsapa_backend::types::{SearchArea, SearchMode, SearchParams, SearchResultPage};
use simsapa_backend::theme_colors::ThemeColors;
use simsapa_backend::{get_app_data, get_create_simsapa_dir};
use simsapa_backend::html_content::html_page;
use simsapa_backend::dir_list::{generate_html_directory_listing, generate_plain_directory_listing};

use simsapa_backend::logger::{info, error};

#[cxx_qt::bridge]
pub mod qobject {

    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;

        include!("cxx-qt-lib/qstringlist.h");
        type QStringList = cxx_qt_lib::QStringList;

        include!("system_palette.h");
        fn get_system_palette_json() -> QString;
    }

    impl cxx_qt::Threading for SuttaBridge{}

    extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qproperty(bool, db_loaded)]
        #[namespace = "sutta_bridge"]
        type SuttaBridge = super::SuttaBridgeRust;

        #[qinvokable]
        fn load_db(self: Pin<&mut SuttaBridge>);

        #[qinvokable]
        fn appdata_first_query(self: Pin<&mut SuttaBridge>);

        #[qinvokable]
        fn dpd_first_query(self: Pin<&mut SuttaBridge>);

        #[qinvokable]
        fn results_page(self: &SuttaBridge, query: &QString, page_num: usize) -> QString;

        #[qinvokable]
        fn dpd_deconstructor_list(self: &SuttaBridge, query: &QString) -> QStringList;

        #[qinvokable]
        fn dpd_lookup_list(self: &SuttaBridge, query: &QString) -> QStringList;

        #[qinvokable]
        fn get_sutta_html(self: &SuttaBridge, window_id: &QString, query: &QString) -> QString;

        #[qinvokable]
        fn get_translations_for_sutta_uid(self: &SuttaBridge, sutta_uid: &QString) -> QStringList;

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
            let _list = app_data.dbm.dpd.dpd_lookup_list("dhamma");
            info("SuttaBridge::dpd_first_query() end");
        });
    }

    pub fn results_page(&self, query: &QString, page_num: usize) -> QString {
        // FIXME: Can't store the query_task on SuttaBridgeRust
        // because it SearchQueryTask includes &'a DbManager reference.
        // Store only a connection pool?
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

    pub fn dpd_deconstructor_list(&self, query: &QString) -> QStringList {
        let app_data = get_app_data();
        let list = app_data.dbm.dpd.dpd_deconstructor_list(&query.to_string());
        let mut res = QStringList::default();
        for i in list {
            res.append(QString::from(i));
        }
        res
    }

    pub fn dpd_lookup_list(&self, query: &QString) -> QStringList {
        let app_data = get_app_data();
        let list = app_data.dbm.dpd.dpd_lookup_list(&query.to_string());
        let mut res = QStringList::default();
        for i in list {
            res.append(QString::from(i));
        }
        res
    }

    pub fn get_sutta_html(&self, window_id: &QString, query: &QString) -> QString {
        let app_data = get_app_data();
        let app_settings = app_data.app_settings_cache.read().expect("Failed to read app settings");
        let body_class = app_settings.theme_name_as_string();

        let blank_page_html = html_page("", None, None, None, Some(body_class.clone()));
        if query.trimmed().is_empty() {
            return QString::from(blank_page_html);
        }

        let sutta = app_data.dbm.appdata.get_sutta(&query.to_string());

        let html = match sutta {
            Some(sutta) => {
                let js_extra = format!("const WINDOW_ID = '{}';", &window_id.to_string());

                app_data.render_sutta_content(&sutta, None, Some(js_extra))
                .unwrap_or(html_page("Rendering error", None, None, None, Some(body_class)))
            },
            None => blank_page_html,
        };

        QString::from(html)
    }

    pub fn get_translations_for_sutta_uid(&self, sutta_uid: &QString) -> QStringList {
        let app_data = get_app_data();
        let translations: Vec<String> = app_data.dbm.appdata.get_translations_for_sutta_uid(&sutta_uid.to_string());
        let mut res = QStringList::default();
        for t in translations {
            res.append(QString::from(t));
        }
        res
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
}
