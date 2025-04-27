import QtQuick
import QtWebView

import com.profoundlabs.simsapa

WebView {
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

}
