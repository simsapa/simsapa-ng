import QtQuick

Loader {
    id: loader
    required property string window_id
    required property bool is_dark
    required property string item_key

    property bool should_be_visible: true

    // Passing on tab_data properties as json to avoid the UI reacting to one
    // property (e.g. item_uid) while the other has not yet been set (e.g.
    // table_name).
    required property string data_json
    // NOTE: data_json properties:
    // let data = {
    //     item_uid: tab_data.item_uid,
    //     table_name: tab_data.table_name,
    //     sutta_ref: tab_data.sutta_ref,
    //     sutta_title: tab_data.sutta_title,
    // };

    function get_data_value(key: string): string {
        let data = JSON.parse(loader.data_json);
        return data[key];
    }

    function set_data_value(key: string, value: string): string {
        let data = JSON.parse(loader.data_json);
        data[key] = value;
        loader.data_json = JSON.stringify(data);
    }

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
        loader.item.is_dark = Qt.binding(() => is_dark);
        loader.item.data_json = Qt.binding(() => data_json);
        loader.item.visible = Qt.binding(() => loader.should_be_visible && loader.visible);
    }

}

