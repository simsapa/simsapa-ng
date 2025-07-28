import QtQuick
import QtWebEngine

import com.profoundlabs.simsapa

WebEngineView {
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

    // TODO: implement find_bar
    // onFindTextFinished: function(result) {
    //     if (!find_bar.visible)
    //         find_bar.visible = true;
    //
    //     find_bar.numberOfMatches = result.numberOfMatches;
    //     find_bar.activeMatch = result.activeMatch;
    // }
    //
    // onLoadingChanged: function(loadRequest) {
    //     if (loadRequest.status == WebEngineView.LoadStartedStatus)
    //         find_bar.reset();
    // }
}
