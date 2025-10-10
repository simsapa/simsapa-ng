import QtQuick
import QtWebView

import com.profoundlabs.simsapa

/*
 * Mobile WebView Visibility Management
 * 
 * This component wraps QtWebView in an Item container to provide proper visibility control.
 * 
 * On mobile platforms (Android/iOS), QtWebView uses native platform views (Android WebView,
 * WKWebView) that render in a separate layer above Qt Quick content. These native views don't
 * respect QML's visibility hierarchy, so simply setting visible: false on parent items doesn't
 * reliably hide them.
 * 
 * Solution:
 * 1. Wrap WebView in an Item container that participates in QML's visibility hierarchy
 * 2. Explicitly bind WebView's visible property to the container's visibility
 * 3. Set enabled: false in addition to visible: false to stop native rendering
 * 
 * This ensures the WebView is properly hidden when it should not be visible, preventing
 * blank yellow webviews from covering the screen.
 * 
 * See docs/mobile-webview-visibility-management.md for detailed explanation.
 */

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

    function set_properties_from_data_json() {
        let data = JSON.parse(root.data_json);
        root.item_uid = data.item_uid;
        root.table_name = data.table_name;
        root.sutta_ref = data.sutta_ref;
        root.sutta_title = data.sutta_title;
    }

    function show_transient_message(msg: string) {
        let js = `var msg = \`${msg}\`; document.SSP.show_transient_message(msg, "transient-messages-top");`;
        web.runJavaScript(js);
    }

    function show_find_bar() {
        web.runJavaScript(`document.SSP.find.show();`);
    }

    function find_next() {
        web.runJavaScript(`document.SSP.find.nextMatch();`);
    }

    function find_previous() {
        web.runJavaScript(`document.SSP.find.previousMatch();`);
    }

    function load_sutta_uid(uid) {
        if (uid == "Sutta") {
            // Initial blank page
            uid = "";
        }
        var html = SuttaBridge.get_sutta_html(root.window_id, uid);
        web.loadHtml(html);
    }

    function load_word_uid(uid) {
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
        web.loadHtml(html);
    }

    // Load the sutta or dictionary word when the Loader in SuttaHtmlView updates data_json
    onData_jsonChanged: function() {
        root.set_properties_from_data_json();
        // Both "dict_words" and "dpd_headwords" should load dictionary content
        if (root.table_name === "dict_words" || root.table_name === "dpd_headwords") {
            root.load_word_uid(root.item_uid);
        } else {
            root.load_sutta_uid(root.item_uid);
        }
    }

    onIs_darkChanged: function() {
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

    WebView {
        id: web
        anchors.fill: parent
        visible: root.visible
        enabled: root.visible

        onLoadingChanged: {
            if (root.is_dark) {
                web.runJavaScript("document.documentElement.style.colorScheme = 'dark';");
            }
        }
    }
}
