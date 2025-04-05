use std::env;
use cxx_qt_build::{CxxQtBuilder, QmlModule};

fn main() {
    let s = env::var("CXX_QT_QT_MODULES").expect("Env var not found");
    let mobile_build = s.contains("Qt::WebView");

    let mut qml_files = Vec::new();
    qml_files.push("../qml/word_lookup_window.qml");

    if mobile_build {
        qml_files.push("../qml/sutta_search_window_mobile.qml");
    } else {
        qml_files.push("../qml/sutta_search_window_desktop.qml");
    }

    let builder = CxxQtBuilder::new()
        // Link Qt's Network library
        // - Qt Core is always linked
        // - Qt Gui is linked by enabling the qt_gui Cargo feature of cxx-qt-lib.
        // - Qt Qml is linked by enabling the qt_qml Cargo feature of cxx-qt-lib.
        // - Qt Qml requires linking Qt Network on macOS
        .qt_module("Network")
        .qt_module("Widgets")
        .qt_module("Quick")
        .qml_module(QmlModule {
                uri: "com.profoundlabs.simsapa",
                rust_files: &[
                        "src/sutta_bridge.rs",
                        "src/api.rs",
                ],
                qml_files: &qml_files,
                ..Default::default()
        });

    if mobile_build {
        builder.qt_module("WebView").build();
    } else {
        builder.qt_module("WebEngineQuick").build();
    }

}
