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

impl qobject::SuttaBridge {
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
