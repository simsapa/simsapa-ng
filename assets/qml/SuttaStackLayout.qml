pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts

import com.profoundlabs.simsapa

StackLayout {
    id: root

    required property string window_id
    required property bool is_dark
    property string current_key: ""
    property var items_map: ({})

    function show_transient_message(msg) {
        root.items_map[root.current_key].show_transient_message(msg);
    }

    Logger { id: logger }

    Component {
        id: sutta_html_component

        SuttaHtmlView {
            id: html_view
            window_id: root.window_id
            is_dark: root.is_dark
            Layout.fillWidth: true
            Layout.fillHeight: true

            property int index
            /* Component.onCompleted: logger.log("SuttaHtmlView created at index", html_view.index) */
            /* Component.onDestruction: logger.log("SuttaHtmlView destroyed at index", html_view.index) */
        }
    }

    function add_item(tab_data: var, show_item = true) {
        /* logger.log("add_item()", "sutta_uid:", tab_data.sutta_uid, "web_item_key:", tab_data.web_item_key); */
        let key = tab_data.web_item_key;
        if (root.items_map.hasOwnProperty(key)) {
            logger.error("Item with key", key, "already exists");
            return;
        }

        let data = {
            item_key: key,
            sutta_uid: tab_data.sutta_uid,
            sutta_ref: tab_data.sutta_ref,
            sutta_title: tab_data.sutta_title,
        };
        let comp = sutta_html_component.createObject(root, data);
        root.items_map[key] = comp;
        if (show_item) {
            root.current_key = key;
        }
    }

    function get_item(key) {
        return root.items_map[key] || null;
    }

    function has_item(key) {
        return root.items_map.hasOwnProperty(key);
    }

    function delete_item(key, show_key_after_delete = null) {
        if (!root.items_map.hasOwnProperty(key)) {
            logger.error("Item with key", key, "not found")
            return;
        }

        const item = root.items_map[key];
        delete root.items_map[key];
        item.destroy();

        // Update current_key if needed
        if (root.current_key === key) {
            root.current_key = show_key_after_delete || "";
        }
    }

    onCurrent_keyChanged: update_currentIndex()
    onCurrentIndexChanged: update_current_key()

    function update_currentIndex() {
        if (!root.items_map[root.current_key]) {
            root.currentIndex = -1;
            return;
        }

        let item = root.items_map[root.current_key];
        if (item.sutta_uid !== "Sutta") {
            /* logger.log("SuttaBridge.emit_update_window_title()", item.sutta_uid, item.sutta_ref, item.sutta_title); */
            SuttaBridge.emit_update_window_title(item.sutta_uid, item.sutta_ref, item.sutta_title);
        }

        for (let i = 0; i < root.children.length; i++) {
            if (root.children[i].item_key === root.current_key) {
                root.currentIndex = i;
                break;
            }
        }
    }

    function update_current_key() {
        if (root.currentIndex >= 0 && root.currentIndex < root.children.length) {
            const item = root.children[root.currentIndex];
            root.current_key = item.item_key || "";
        } else {
            root.current_key = "";
        }
    }

    Component.onDestruction: logger.log("SuttaStackLayout destroyed, children: ", root.children.length)
}
