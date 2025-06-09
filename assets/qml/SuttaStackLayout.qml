pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts

StackLayout {
    id: root
    required property string window_id
    property string current_key: ""
    property var items_map: ({})
    property bool is_drawer_menu_open: false

    Component {
        id: sutta_html_component

        SuttaHtmlView {
            id: html_view
            window_id: root.window_id
            Layout.fillWidth: true
            Layout.fillHeight: true
            // Hide the webview when the drawer menu is open. The mobile webview
            // is always on top, obscuring the drawer menu.
            visible: !root.is_drawer_menu_open

            property int index
            /* Component.onCompleted: console.log("SuttaHtmlView created at index", html_view.index) */
            /* Component.onDestruction: console.log("SuttaHtmlView destroyed at index", html_view.index) */
        }
    }

    function add_item(key, uid, show_item = true) {
        if (root.items_map.hasOwnProperty(key)) {
            console.warn("Item with key", key, "already exists");
            return;
        }

        let comp = sutta_html_component.createObject(root, {item_key: key, sutta_uid: uid});
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
            console.warn("Item with key", key, "not found")
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

    Component.onDestruction: console.log("SuttaStackLayout destroyed, children: ", root.children.length)
}
