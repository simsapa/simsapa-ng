use std::fs::File;
use std::io::Read;
use std::path::PathBuf;
use serde::{Serialize, Deserialize};

#[cxx_qt::bridge]
pub mod qobject {

    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
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
        fn get_sutta_html(self: &SuttaBridge) -> QString;
    }
}

use core::pin::Pin;
use cxx_qt_lib::QString;

use crate::db::get_sutta;
use crate::html_content::html_page;

#[derive(Default)]
pub struct SuttaBridgeRust {
    number: i32,
    string: QString,
}

#[derive(Debug, Serialize, Deserialize)]
struct SearchResult {
    uid: String,
    // database schema name (appdata or userdata)
    schema_name: String,
    // database table name (e.g. suttas or dict_words)
    table_name: String,
    source_uid: Option<String>,
    title: String,
    sutta_ref: Option<String>,
    // FIXME nikaya should be Option<String>
    nikaya: Option<Vec<String>>,
    author: Option<String>,
    // highlighted snippet
    snippet: String,
    // page number in a document
    page_number: Option<i32>,
    score: Option<f32>,
    rank: Option<i32>,
}

impl SearchResult {
    fn load_from_json(path: &PathBuf) -> Result<Vec<Self>, String> {
        let mut file = match File::open(path) {
            Ok(file) => file,
            Err(e) => return Err(format!("Failed to open file: {}", e)),
        };

        let mut contents = String::new();
        match file.read_to_string(&mut contents) {
            Ok(_) => (),
            Err(e) => return Err(format!("Failed to read file: {}", e)),
        }

        match serde_json::from_str(&contents) {
            Ok(results) => Ok(results),
            Err(e) => Err(format!("Failed to parse JSON: {}", e)),
        }
    }
}

impl qobject::SuttaBridge {
    pub fn search(&self, query: &QString) -> QString {
        let results = match SearchResult::load_from_json(&PathBuf::from("/home/gambhiro/prods/apps/simsapa-ng-project/simsapa-ng/assets/qml/data/bojjhanga.json")) {
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

    pub fn get_sutta_html(&self) -> QString {
        let sutta = get_sutta("dn22/en/thanissaro");

        let html = match sutta {
            Some(sutta) => html_page(&sutta.content_html, None, None, None),
            None => String::from("<!doctype html><html><head></head><body><h1>No sutta</h1></body></html>"),
        };

        QString::from(html)
    }
}
