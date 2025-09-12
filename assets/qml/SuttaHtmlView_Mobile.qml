import QtQuick
import QtWebView

import com.profoundlabs.simsapa

WebView {
    id: web
    anchors.fill: parent

    property string window_id
    property string item_key
    property string item_uid
    property string table_name
    property string sutta_ref
    property string sutta_title
    property bool is_dark

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

    // Load the sutta or dictionary word when the Loader in SuttaHtmlView updates item_uid.
    onItem_uidChanged: function() {
        if (web.table_name === "dict_words") {
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
        var html = SuttaBridge.get_word_html(web.window_id, uid);
        web.loadHtml(html);
    }
}
