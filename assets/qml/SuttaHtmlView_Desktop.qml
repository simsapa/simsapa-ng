pragma ComponentBehavior: Bound

import QtQuick
import QtWebEngine

import com.profoundlabs.simsapa

Item {
    id: root
    anchors.fill: parent

    property string window_id
    property bool is_dark

    property string data_json

    property string item_uid
    property string table_name
    property string sutta_ref
    property string sutta_title

    // 'web' property will be defined later as an alias to web_loader.item

    signal page_loaded()

    function set_properties_from_data_json() {
        let data = JSON.parse(root.data_json);
        root.item_uid = data.item_uid;
        root.table_name = data.table_name;
        root.sutta_ref = data.sutta_ref;
        root.sutta_title = data.sutta_title;
    }

    function show_transient_message(msg: string) {
        if (!web) return;
        let js = `var msg = \`${msg}\`; document.SSP.show_transient_message(msg, "transient-messages-top");`;
        web.runJavaScript(js);
    }

    function show_find_bar() {
        if (!web) return;
        web.forceActiveFocus();
        web.runJavaScript(`document.SSP.find.show();`);
    }

    function find_next() {
        if (!web) return;
        web.runJavaScript(`document.SSP.find.nextMatch();`);
    }

    function find_previous() {
        if (!web) return;
        web.runJavaScript(`document.SSP.find.previousMatch();`);
    }

    function load_sutta_uid(uid, webview = null) {
        let target_web = webview || web;
        if (!target_web) return;
        if (uid == "Sutta") {
            // Initial blank page
            uid = "";
        }
        var html = SuttaBridge.get_sutta_html(root.window_id, uid);
        target_web.loadHtml(html);
    }

    function load_word_uid(uid, webview = null) {
        let target_web = webview || web;
        if (!target_web) return;
        if (uid == "Word") {
            // Initial blank page
            uid = "";
        }
        if (root.table_name === "dpd_headwords") {
            // Results from DPD Lookup are in the form of
            // "item_uid": "25671/dpd", "table_name": "dpd_headwords", "sutta_title":"cakka 1"
            // SuttaBridge.get_word_html() needs the uid for dict_words table in dictionaries.sqlite3
            // where the form is "uid": "cakka 1/dpd"
            uid = `${root.sutta_title}/dpd`;
        }
        var html = SuttaBridge.get_word_html(root.window_id, uid);
        target_web.loadHtml(html);
    }

    // Load the sutta or dictionary word when the Loader in SuttaHtmlView updates data_json
    onData_jsonChanged: function() {
        root.set_properties_from_data_json();
        // Only load if WebView exists (Loader is active and has created the item)
        if (root.web) {
            if (root.table_name === "dict_words" || root.table_name === "dpd_headwords") {
                root.load_word_uid(root.item_uid);
            } else {
                root.load_sutta_uid(root.item_uid);
            }
        }
        // If WebView doesn't exist yet, it will load content in its Component.onCompleted
    }

    onIs_darkChanged: function() {
        if (!web) return;
        let js = "";
        if (root.is_dark) {
            js = `
document.body.classList.add('dark');
document.documentElement.classList.add('dark');
document.documentElement.style.colorScheme = 'dark';
`;
        } else {
            js = `
document.body.classList.remove('dark');
document.documentElement.classList.remove('dark');
document.documentElement.style.colorScheme = 'light';
`;
        }
        web.runJavaScript(js);
    }

    Loader {
        id: web_loader
        anchors.fill: parent
        // Only create WebView when actually visible and has non-zero size
        active: root.visible && root.width > 0 && root.height > 0

        sourceComponent: Component {
            WebEngineView {
                id: web
                anchors.fill: parent

                Component.onCompleted: {
                    // Load content when WebView is first created
                    // Pass 'web' directly since root.web (web_loader.item) might not be set yet
                    if (root.table_name === "dict_words" || root.table_name === "dpd_headwords") {
                        root.load_word_uid(root.item_uid, web);
                    } else {
                        root.load_sutta_uid(root.item_uid, web);
                    }
                }

                onLoadingChanged: function(loadRequest) {
                    if (root.is_dark) {
                        web.runJavaScript("document.documentElement.style.colorScheme = 'dark';");
                    }
                    if (loadRequest.loadProgress === 100) {
                        root.page_loaded();
                    }
                }
            }
        }
    }

    // Provide 'web' alias for compatibility with existing code
    property var web: web_loader.item
}
