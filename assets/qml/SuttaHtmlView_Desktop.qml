import QtQuick
import QtWebEngine

import com.profoundlabs.simsapa

WebEngineView {
    id: web
    anchors.fill: parent

    property string item_key
    property string sutta_uid

    // Load the sutta when the Loader in SuttaHtmlView updates sutta_uid.
    onSutta_uidChanged: function() {
        load_sutta_uid(web.sutta_uid);
    }

    SuttaBridge {
        id: sb
    }

    function load_sutta_uid(uid) {
        if (uid == "Sutta") {
            // Initial blank page
            uid = "";
        }
        var html = sb.get_sutta_html(uid);
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
