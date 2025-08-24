import QtQuick

Loader {
    id: loader
    required property string window_id
    required property string item_key
    required property string sutta_uid
    required property string sutta_ref
    required property string sutta_title
    required property bool is_dark

    function show_transient_message(msg) {
        loader.item.show_transient_message(msg); // qmllint disable missing-property
    }

    /* signal loadingChanged(var loadRequest) */

    source: {
        if (Qt.platform.os === "android" || Qt.platform.os === "ios") {
            return "SuttaHtmlView_Mobile.qml";
        } else {
            return "SuttaHtmlView_Desktop.qml";
        }
    }

    onLoaded: {
        loader.item.window_id = Qt.binding(() => window_id);
        loader.item.item_key = Qt.binding(() => item_key);
        loader.item.sutta_uid = Qt.binding(() => sutta_uid);
        loader.item.sutta_ref = Qt.binding(() => sutta_ref);
        loader.item.sutta_title = Qt.binding(() => sutta_title);
        loader.item.is_dark = Qt.binding(() => is_dark);
        /* loader.item.loadingChanged.connect(loader.loadingChanged); */
    }

}
