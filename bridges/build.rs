use std::env;
use cxx_qt_build::{CxxQtBuilder, QmlModule};
use cxx_qt_build::{is_ios_target, thin_generated_fat_library_with_lipo};

fn main() {
    let s = match env::var("CXX_QT_QT_MODULES") {
        Ok(s) => s,
        Err(_) => "".to_string(),
    };
    let mobile_build = s.contains("Qt::WebView");

    let mut qml_files = Vec::new();
    qml_files.push("../assets/qml/SuttaSearchWindow.qml");
    qml_files.push("../assets/qml/WordLookupWindow.qml");

    qml_files.push("../assets/qml/SearchBarInput.qml");
    qml_files.push("../assets/qml/SearchBarOptions.qml");
    qml_files.push("../assets/qml/FulltextResults.qml");
    qml_files.push("../assets/qml/CMenuItem.qml");
    qml_files.push("../assets/qml/SuttaTabButton.qml");

    qml_files.push("../assets/qml/SuttaHtmlView.qml");
    qml_files.push("../assets/qml/SuttaHtmlView_Desktop.qml");
    qml_files.push("../assets/qml/SuttaHtmlView_Mobile.qml");

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

    if is_ios_target() {
        thin_generated_fat_library_with_lipo("libsimsapa_bridges-cxxqt-generated.a", "arm64");
    }
}
