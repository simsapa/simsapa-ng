use std::env;
use cxx_qt_build::{CxxQtBuilder, QmlModule};
// use cxx_qt_build::{is_ios_target, thin_generated_fat_library_with_lipo};

fn main() {
    let s = match env::var("CXX_QT_QT_MODULES") {
        Ok(s) => s,
        Err(_) => "".to_string(),
    };
    let mobile_build = s.contains("Qt::WebView");

    let mut qml_files = Vec::new();
    qml_files.push("../assets/qml/SuttaSearchWindow.qml");
    qml_files.push("../assets/qml/WordLookupWindow.qml");
    qml_files.push("../assets/qml/DownloadAppdataWindow.qml");

    qml_files.push("../assets/qml/SearchBarInput.qml");
    qml_files.push("../assets/qml/SearchBarOptions.qml");
    qml_files.push("../assets/qml/FulltextResults.qml");
    qml_files.push("../assets/qml/CMenuItem.qml");
    qml_files.push("../assets/qml/SuttaTabButton.qml");
    qml_files.push("../assets/qml/SuttaHtmlView.qml");
    qml_files.push("../assets/qml/SuttaHtmlView_Desktop.qml");
    qml_files.push("../assets/qml/SuttaHtmlView_Mobile.qml");
    qml_files.push("../assets/qml/DictionaryHtmlView.qml");
    qml_files.push("../assets/qml/DictionaryHtmlView_Desktop.qml");
    qml_files.push("../assets/qml/DictionaryHtmlView_Mobile.qml");
    qml_files.push("../assets/qml/DictionaryTab.qml");
    qml_files.push("../assets/qml/SuttaStackLayout.qml");
    qml_files.push("../assets/qml/AboutDialog.qml");
    qml_files.push("../assets/qml/ApiKeysDialog.qml");
    qml_files.push("../assets/qml/SystemPromptsDialog.qml");
    qml_files.push("../assets/qml/ModelsDialog.qml");
    qml_files.push("../assets/qml/ColorThemeDialog.qml");
    qml_files.push("../assets/qml/DrawerMenu.qml");
    qml_files.push("../assets/qml/DrawerEmptyItem.qml");
    qml_files.push("../assets/qml/ListBackground.qml");
    qml_files.push("../assets/qml/WordSummary.qml");
    qml_files.push("../assets/qml/StorageDialog.qml");
    qml_files.push("../assets/qml/GlossTab.qml");
    qml_files.push("../assets/qml/PromptsTab.qml");

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
                        "src/asset_manager.rs",
                        "src/storage_manager.rs",
                        "src/prompt_manager.rs",
                        "src/api.rs",
                ],
                qml_files: &qml_files,
                ..Default::default()
        })
        .cc_builder(|cc| {
            // Add include directory for custom headers
            cc.include("../cpp/");
            cc.file("../cpp/utils.cpp");
            cc.file("../cpp/system_palette.cpp");
            cc.file("../cpp/gui.cpp");
        });

    if mobile_build {
        builder.qt_module("WebView").build();
    } else {
        builder.qt_module("WebEngineQuick").build();
    }

    // if is_ios_target() {
    //     thin_generated_fat_library_with_lipo("libsimsapa_bridges-cxxqt-generated.a", "arm64");
    // }
}
