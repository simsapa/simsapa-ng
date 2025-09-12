import QtQuick

Loader {
    id: loader
    required property string window_id
    required property string item_key
    required property string item_uid
    required property string table_name
    required property string sutta_ref
    required property string sutta_title

    required property bool is_dark

    function active_focus() {
        loader.item.forceActiveFocus(); // qmllint disable missing-property
    }

    function show_transient_message(msg) {
        loader.item.show_transient_message(msg); // qmllint disable missing-property
    }

    function show_find_bar() {
        loader.item.show_find_bar(); // qmllint disable missing-property
    }

    function find_next() {
        loader.item.find_next(); // qmllint disable missing-property
    }

    function find_previous() {
        loader.item.find_previous(); // qmllint disable missing-property
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
        loader.item.item_uid = Qt.binding(() => item_uid);
        loader.item.table_name = Qt.binding(() => table_name);
        loader.item.sutta_ref = Qt.binding(() => sutta_ref);
        loader.item.sutta_title = Qt.binding(() => sutta_title);
        loader.item.is_dark = Qt.binding(() => is_dark);
        /* loader.item.loadingChanged.connect(loader.loadingChanged); */
    }

}
