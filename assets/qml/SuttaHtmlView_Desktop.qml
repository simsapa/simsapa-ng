import QtQuick
import QtWebEngine

import com.profoundlabs.simsapa

WebEngineView {
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
