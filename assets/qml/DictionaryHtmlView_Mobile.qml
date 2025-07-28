import QtQuick
import QtWebView

import com.profoundlabs.simsapa

WebView {
    id: web
    anchors.fill: parent

    property string window_id
    property string word_uid
    property bool is_dark

    onLoadingChanged: {
        if (web.is_dark) {
            runJavaScript("document.documentElement.style.colorScheme = 'dark';");
        }
    }

    onWord_uidChanged: function() {
        load_word_uid(web.word_uid);
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

    SuttaBridge {
        id: sb
    }

    function load_word_uid(uid) {
        if (uid == "Word") {
            // Initial blank page
            uid = "";
        }
        var html = sb.get_word_html(web.window_id, uid);
        web.loadHtml(html);
    }

}
