use std::fs;
use std::path::PathBuf;
use std::collections::HashMap;

use core::pin::Pin;
use cxx_qt_lib::{QString, QStringList};

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
        #[qproperty(i32, number)]
        #[qproperty(QString, string)]
        #[namespace = "sutta_bridge"]
        type SuttaBridge = super::SuttaBridgeRust;
    }

    unsafe extern "RustQt" {
        #[qinvokable]
        #[cxx_name = "search"]
        fn search(self: &SuttaBridge, query: &QString) -> QString;

        #[qinvokable]
        #[cxx_name = "incrementNumber"]
        fn increment_number(self: Pin<&mut SuttaBridge>);

        #[qinvokable]
        #[cxx_name = "sayHi"]
        fn say_hi(self: &SuttaBridge, string: &QString, number: i32);

        #[qinvokable]
        #[cxx_name = "get_sutta_html"]
        fn get_sutta_html(self: &SuttaBridge, query: &QString) -> QString;

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
    number: i32,
    string: QString,
}

impl Default for SuttaBridgeRust {
    fn default() -> Self {
        Self {
            number: 0,
            string: QString::from(""),
        }
    }
}

impl qobject::SuttaBridge {
    pub fn search(&self, query: &QString) -> QString {
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

    pub fn increment_number(self: Pin<&mut Self>) {
        let previous = *self.number();
        self.set_number(previous + 1);
    }

    pub fn say_hi(&self, string: &QString, number: i32) {
        println!("Hi from Rust! String is '{string}' and number is {number}");
    }

    pub fn get_sutta_html(&self, query: &QString) -> QString {
        let sutta = db::get_sutta(&query.to_string());

        let html = match sutta {
            Some(sutta) => {
                let (db_conn, _) = db::establish_connection();
                let settings = HashMap::new();
                let mut app_data = AppData::new(db_conn, settings, API_URL.to_string());

                render_sutta_content(&mut app_data, &sutta, None)
                .unwrap_or(html_page("Rendering error", None, None, None))
            },
            None => String::from("<!doctype html><html><head></head><body><h1>No sutta</h1></body></html>"),
        };

        QString::from(html)
    }

    pub fn get_translations_for_sutta_uid(&self, sutta_uid: &QString) -> QStringList {
        let translations: Vec<String> = db::get_translations_for_sutta_uid(&sutta_uid.to_string());
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
