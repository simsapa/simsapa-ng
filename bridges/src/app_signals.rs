use core::pin::Pin;
use cxx_qt_lib::{QString, QGuiApplication};

#[cxx_qt::bridge]
pub mod app_windows {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
    }

    unsafe extern "C++Qt" {
        include!(<QtWidgets/QPushButton>);
        #[qobject]
        type QPushButton;

        #[qsignal]
        fn clicked(self: Pin<&mut QPushButton>, checked: bool);
    }

    extern "RustQt" {
        #[qobject]
        #[namespace = "app_windows"]
        type AppWindows = super::AppWindowsRust;
    }

    unsafe extern "RustQt" {
        #[qsignal]
        fn open_window_signal(self: Pin<&mut AppWindows>, window_type_name: QString);
    }
}

#[derive(Default)]
pub struct AppWindowsRust;

impl app_windows::AppWindows {
    pub fn do_init(self: Pin<&mut Self>, app: &QGuiApplication, enable_tray_icon: bool) {
        let _ = self.on_open_window_signal(|_, x| {
            Self::handle_open_window_signal(&x.to_string());
        });

        if enable_tray_icon {
            Self::setup_system_tray(app);
        }
    }

    fn setup_system_tray(app: &QGuiApplication) {
        println!("setup_system_tray()");
    }

    #[allow(dead_code)]
    fn quit_app(&mut self) {
        println!("quit_app()");
        // self.app.quit();
    }

    #[allow(dead_code)]
    fn handle_system_tray_clicked(&self) {
        println!("handle_system_tray_clicked()");
    }

    fn handle_open_window_signal(window_type_name: &str) {
        println!("handle_open_window_signal(): {}", window_type_name);
        Self::open_window_type(window_type_name);
    }

    fn open_window_type(window_type: &str) {
        println!("open_window_type(): {}", window_type);
    }
}
