import QtQuick
import QtWebView

import com.profoundlabs.simsapa

WebView {
    id: web
    anchors.fill: parent

    property string window_id
    property string item_key
    property string sutta_uid
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

    // Load the sutta when the Loader in SuttaHtmlView updates sutta_uid.
    onSutta_uidChanged: function() {
        load_sutta_uid(web.sutta_uid);
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

}
