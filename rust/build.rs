use cxx_qt_build::{CxxQtBuilder, QmlModule};

fn main() {
    CxxQtBuilder::new()
        // Link Qt's Network library
        // - Qt Core is always linked
        // - Qt Gui is linked by enabling the qt_gui Cargo feature of cxx-qt-lib.
        // - Qt Qml is linked by enabling the qt_qml Cargo feature of cxx-qt-lib.
        // - Qt Qml requires linking Qt Network on macOS
        .qt_module("Network")
        .qt_module("Quick")
        .qt_module("WebView")
        .qml_module(QmlModule {
            uri: "com.profound_labs.simsapa",
            rust_files: &[
                    "src/sutta_bridge.rs",
            ],
            qml_files: &[
                    "../qml/sutta_search_window.qml",
            ],
            ..Default::default()
        })
        .build();
}
