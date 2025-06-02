use std::fs;
use std::path::PathBuf;
use std::collections::HashMap;
use std::thread;

use core::pin::Pin;
use cxx_qt_lib::{QString, QStringList};
use cxx_qt::Threading;

use simsapa_backend::query_task::SearchQueryTask;
use simsapa_backend::types::{SearchArea, SearchMode, SearchParams};
use simsapa_backend::app_data::AppData;
use simsapa_backend::{db, API_URL, get_create_simsapa_app_root};
use simsapa_backend::html_content::html_page;
use simsapa_backend::export_helpers::render_sutta_content;
use simsapa_backend::dir_list::{generate_html_directory_listing, generate_plain_directory_listing};

#[cxx_qt::bridge]
pub mod qobject {

    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;

        include!("cxx-qt-lib/qstringlist.h");
        type QStringList = cxx_qt_lib::QStringList;
    }

    extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qproperty(bool, db_loaded)]
        #[namespace = "sutta_bridge"]
        type SuttaBridge = super::SuttaBridgeRust;
    }

    impl cxx_qt::Threading for SuttaBridge{}

    unsafe extern "RustQt" {
        #[qinvokable]
        #[cxx_name = "load_db"]
        fn load_db(self: Pin<&mut SuttaBridge>);

        #[qinvokable]
        #[cxx_name = "search"]
        fn search(self: &SuttaBridge, query: &QString) -> QString;

        #[qinvokable]
        #[cxx_name = "dpd_deconstructor_list"]
        fn dpd_deconstructor_list(self: &SuttaBridge, query: &QString) -> QStringList;

        #[qinvokable]
        #[cxx_name = "dpd_lookup_list"]
        fn dpd_lookup_list(self: &SuttaBridge, query: &QString) -> QStringList;

        #[qinvokable]
        #[cxx_name = "get_sutta_html"]
        fn get_sutta_html(self: &SuttaBridge, window_id: &QString, query: &QString) -> QString;

        #[qinvokable]
        #[cxx_name = "get_translations_for_sutta_uid"]
        fn get_translations_for_sutta_uid(self: &SuttaBridge, sutta_uid: &QString) -> QStringList;

        #[qinvokable]
        #[cxx_name = "app_data_folder_path"]
        fn app_data_folder_path(self: &SuttaBridge) -> QString;

        #[qinvokable]
        #[cxx_name = "is_app_data_folder_writable"]
        fn is_app_data_folder_writable(self: &SuttaBridge) -> bool;

        #[qinvokable]
        #[cxx_name = "app_data_contents_html_table"]
        fn app_data_contents_html_table(self: &SuttaBridge) -> QString;

        #[qinvokable]
        #[cxx_name = "app_data_contents_plain_table"]
        fn app_data_contents_plain_table(self: &SuttaBridge) -> QString;
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
        println!("SuttaBridge::load_db()");
        let qt_thread = self.qt_thread();
        thread::spawn(move || {
            let r = db::rust_backend_init_db();
            qt_thread.queue(move |mut qo| {
                qo.as_mut().set_db_loaded(r);
            }).unwrap();
        });
    }

    pub fn search(&self, query: &QString) -> QString {
        let dbm = db::get_dbm();

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
            dbm,
            "en".to_string(),
            query.to_string(),
            params,
            SearchArea::Suttas,
        );

        let results = match query_task.results_page(0) {
            Ok(x) => x,
            Err(s) => {
                println!("{}", s);
                return QString::from("");
            }
        };

        let json = serde_json::to_string(&results).unwrap_or_default();
        QString::from(json)
    }

    pub fn dpd_deconstructor_list(&self, query: &QString) -> QStringList {
        let dbm = db::get_dbm();
        let list = dbm.dpd.dpd_deconstructor_list(&query.to_string());
        let mut res = QStringList::default();
        for i in list {
            res.append(QString::from(i));
        }
        res
    }

    pub fn dpd_lookup_list(&self, query: &QString) -> QStringList {
        let dbm = db::get_dbm();
        let list = dbm.dpd.dpd_lookup_list(&query.to_string());
        let mut res = QStringList::default();
        for i in list {
            res.append(QString::from(i));
        }
        res
    }

    pub fn get_sutta_html(&self, window_id: &QString, query: &QString) -> QString {
        let blank_page_html = html_page("", None, None, None);
        if query.trimmed().is_empty() {
            return QString::from(blank_page_html);
        }

        let dbm = db::get_dbm();
        let sutta = dbm.appdata.get_sutta(&query.to_string());

        let html = match sutta {
            Some(sutta) => {
                let settings = HashMap::new();
                let db_conn = dbm.appdata.get_conn().expect("get_sutta_html(): No appdata conn");
                let mut app_data = AppData::new(db_conn, settings, API_URL.to_string());

                let js_extra = format!("const WINDOW_ID = '{}';", &window_id.to_string());

                render_sutta_content(&mut app_data, &sutta, None, Some(js_extra))
                .unwrap_or(html_page("Rendering error", None, None, None))
            },
            None => blank_page_html,
        };

        QString::from(html)
    }

    pub fn get_translations_for_sutta_uid(&self, sutta_uid: &QString) -> QStringList {
        let dbm = db::get_dbm();
        let translations: Vec<String> = dbm.appdata.get_translations_for_sutta_uid(&sutta_uid.to_string());
        let mut res = QStringList::default();
        for t in translations {
            res.append(QString::from(t));
        }
        res
    }

    pub fn app_data_folder_path(&self) -> QString {
        let p = get_create_simsapa_app_root().unwrap_or(PathBuf::from("."));
        let app_data_path = p.as_os_str();
        let s = match app_data_path.to_str() {
            Some(x) => x,
            None => "Path error",
        };
        QString::from(s)
    }

    pub fn is_app_data_folder_writable(&self) -> bool {
        let p = get_create_simsapa_app_root().unwrap_or(PathBuf::from("."));
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
        let p = get_create_simsapa_app_root().unwrap_or(PathBuf::from("."));
        let app_data_path = p.to_string_lossy();
        let app_data_folder_contents = generate_html_directory_listing(&app_data_path, 3).unwrap_or(String::from("Error"));
        QString::from(app_data_folder_contents)
    }

    pub fn app_data_contents_plain_table(&self) -> QString {
        let p = get_create_simsapa_app_root().unwrap_or(PathBuf::from("."));
        let app_data_path = p.to_string_lossy();
        let app_data_folder_contents = generate_plain_directory_listing(&app_data_path, 3).unwrap_or(String::from("Error"));
        QString::from(app_data_folder_contents)
    }
}
