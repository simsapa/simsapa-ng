import QtQuick
import QtWebView

import com.profoundlabs.simsapa

WebView {
    id: web
    anchors.fill: parent

    property string window_id
    property bool is_dark

    property string data_json

    property string item_uid
    property string table_name
    property string sutta_ref
    property string sutta_title

    function set_properties_from_data_json() {
        let data = JSON.parse(web.data_json);
        web.item_uid = data.item_uid;
        web.table_name = data.table_name;
        web.sutta_ref = data.sutta_ref;
        web.sutta_title = data.sutta_title;
    }

    function show_transient_message(msg: string) {
        let js = `var msg = \`${msg}\`; document.SSP.show_transient_message(msg, "transient-messages-top");`;
        runJavaScript(js);
    }

    function show_find_bar() {
        runJavaScript(`document.SSP.find.show();`);
    }

    function find_next() {
        runJavaScript(`document.SSP.find.nextMatch();`);
    }

    function find_previous() {
        runJavaScript(`document.SSP.find.previousMatch();`);
    }

    onLoadingChanged: {
        if (web.is_dark) {
            runJavaScript("document.documentElement.style.colorScheme = 'dark';");
        }
    }

    // Load the sutta or dictionary word when the Loader in SuttaHtmlView updates data_json
    onData_jsonChanged: function() {
        web.set_properties_from_data_json();
        // Both "dict_words" and "dpd_headwords" should load dictionary content
        if (web.table_name === "dict_words" || web.table_name === "dpd_headwords") {
            load_word_uid(web.item_uid);
        } else {
            load_sutta_uid(web.item_uid);
        }
    }

    onIs_darkChanged: function() {
        let js = "";
        if (web.is_dark) {
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
        runJavaScript(js);
    }

    function load_sutta_uid(uid) {
        if (uid == "Sutta") {
            // Initial blank page
            uid = "";
        }
        var html = SuttaBridge.get_sutta_html(web.window_id, uid);
        web.loadHtml(html);
    }

    function load_word_uid(uid) {
        if (uid == "Word") {
            // Initial blank page
            uid = "";
        }
        if (web.table_name === "dpd_headwords") {
            // Results from DPD Lookup are in the form of
            // "item_uid": "25671/dpd", "table_name": "dpd_headwords", "sutta_title":"cakka 1"
            // SuttaBridge.get_word_html() needs the uid for dict_words table in dictionaries.sqlite3
            // where the form is "uid": "cakka 1/dpd"
            uid = `${web.sutta_title}/dpd`;
        }
        var html = SuttaBridge.get_word_html(web.window_id, uid);
        web.loadHtml(html);
    }
}
