pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import QtQuick.Controls
import QtQuick.Window

import com.profoundlabs.simsapa

ApplicationWindow {
    id: root
    title: "Chanting Practice"
    width: is_mobile ? Screen.desktopAvailableWidth : 600
    height: is_mobile ? Screen.desktopAvailableHeight : Math.min(700, Screen.desktopAvailableHeight)
    visible: true
    color: palette.window
    flags: Qt.Dialog

    readonly property bool is_mobile: Qt.platform.os === "android" || Qt.platform.os === "ios"
    readonly property bool is_desktop: !root.is_mobile
    readonly property int pointSize: is_mobile ? 16 : 12
    property int top_bar_margin: is_mobile ? 24 : 0

    property var collections_list: []
    property string selected_uid: ""
    property string selected_type: ""
    property bool is_dark: theme_helper.is_dark

    ThemeHelper {
        id: theme_helper
        target_window: root
    }

    Component.onCompleted: {
        root.top_bar_margin = root.is_mobile ? SuttaBridge.get_mobile_top_bar_margin() : 0;
        theme_helper.apply();
        load_collections();
    }

    function load_collections() {
        const json_str = SuttaBridge.get_all_chanting_collections_json();
        try {
            collections_list = JSON.parse(json_str);
        } catch (e) {
            console.error("Failed to parse chanting collections JSON:", e);
            collections_list = [];
        }
    }

    function generate_uid(prefix) {
        return prefix + "-" + Date.now().toString(36) + "-" + Math.random().toString(36).substring(2, 8);
    }

    // --- Add Collection Dialog ---

    Dialog {
        id: add_collection_dialog
        parent: Overlay.overlay
        anchors.centerIn: parent
        width: 400
        title: "Add Collection"
        modal: true
        standardButtons: Dialog.Ok | Dialog.Cancel

        ColumnLayout {
            anchors.fill: parent
            spacing: 10

            Label { text: "Title:"; font.pointSize: root.pointSize }
            TextField {
                id: add_collection_title
                Layout.fillWidth: true
                font.pointSize: root.pointSize
                placeholderText: "Collection title"
            }

            Label { text: "Description:"; font.pointSize: root.pointSize }
            TextField {
                id: add_collection_description
                Layout.fillWidth: true
                font.pointSize: root.pointSize
                placeholderText: "Optional description"
            }
        }

        onAccepted: {
            if (add_collection_title.text.trim() === "") return;
            const data = {
                uid: root.generate_uid("col"),
                title: add_collection_title.text.trim(),
                description: add_collection_description.text.trim() || null,
                language: "pali",
                sort_index: root.collections_list.length,
                is_user_added: true
            };
            SuttaBridge.create_chanting_collection(JSON.stringify(data));
            add_collection_title.text = "";
            add_collection_description.text = "";
            root.load_collections();
        }
    }

    // --- Add Chant Dialog ---

    Dialog {
        id: add_chant_dialog
        parent: Overlay.overlay
        anchors.centerIn: parent
        width: 400
        title: "Add Chant"
        modal: true
        standardButtons: Dialog.Ok | Dialog.Cancel

        ColumnLayout {
            anchors.fill: parent
            spacing: 10

            Label { text: "Title:"; font.pointSize: root.pointSize }
            TextField {
                id: add_chant_title
                Layout.fillWidth: true
                font.pointSize: root.pointSize
                placeholderText: "Chant title"
            }

            Label { text: "Description:"; font.pointSize: root.pointSize }
            TextField {
                id: add_chant_description
                Layout.fillWidth: true
                font.pointSize: root.pointSize
                placeholderText: "Optional description"
            }
        }

        onAccepted: {
            if (add_chant_title.text.trim() === "") return;
            // Find the parent collection to count existing chants
            const parent_col = root.collections_list.find(c => c.uid === root.selected_uid);
            const chant_count = parent_col && parent_col.chants ? parent_col.chants.length : 0;
            const data = {
                uid: root.generate_uid("chant"),
                collection_uid: root.selected_uid,
                title: add_chant_title.text.trim(),
                description: add_chant_description.text.trim() || null,
                sort_index: chant_count,
                is_user_added: true
            };
            SuttaBridge.create_chanting_chant(JSON.stringify(data));
            add_chant_title.text = "";
            add_chant_description.text = "";
            root.load_collections();
        }
    }

    // --- Add Section Dialog ---

    Dialog {
        id: add_section_dialog
        parent: Overlay.overlay
        anchors.centerIn: parent
        width: 500
        title: "Add Section"
        modal: true
        standardButtons: Dialog.Ok | Dialog.Cancel

        ColumnLayout {
            anchors.fill: parent
            spacing: 10

            Label { text: "Title:"; font.pointSize: root.pointSize }
            TextField {
                id: add_section_title
                Layout.fillWidth: true
                font.pointSize: root.pointSize
                placeholderText: "Section title"
            }

            Label { text: "Pāli Text:"; font.pointSize: root.pointSize }
            ScrollView {
                Layout.fillWidth: true
                Layout.preferredHeight: 150

                TextArea {
                    id: add_section_content
                    font.pointSize: root.pointSize
                    placeholderText: "Enter Pāli text here..."
                    wrapMode: TextEdit.WordWrap
                }
            }
        }

        onAccepted: {
            if (add_section_title.text.trim() === "") return;
            // Find parent chant to count existing sections
            let section_count = 0;
            for (let i = 0; i < root.collections_list.length; i++) {
                const col = root.collections_list[i];
                if (col.chants) {
                    const chant = col.chants.find(ch => ch.uid === root.selected_uid);
                    if (chant) {
                        section_count = chant.sections ? chant.sections.length : 0;
                        break;
                    }
                }
            }
            const data = {
                uid: root.generate_uid("sec"),
                chant_uid: root.selected_uid,
                title: add_section_title.text.trim(),
                content_pali: add_section_content.text.trim(),
                sort_index: section_count,
                is_user_added: true
            };
            SuttaBridge.create_chanting_section(JSON.stringify(data));
            add_section_title.text = "";
            add_section_content.text = "";
            root.load_collections();
        }
    }

    // --- Edit Dialog ---

    Dialog {
        id: edit_dialog
        parent: Overlay.overlay
        anchors.centerIn: parent
        width: 500
        title: "Edit"
        modal: true
        standardButtons: Dialog.Ok | Dialog.Cancel

        property string edit_type: "" // "collection", "chant", "section"
        property var edit_data: ({})

        ColumnLayout {
            anchors.fill: parent
            spacing: 10

            Label { text: "Title:"; font.pointSize: root.pointSize }
            TextField {
                id: edit_title
                Layout.fillWidth: true
                font.pointSize: root.pointSize
            }

            Label {
                visible: edit_dialog.edit_type === "collection" || edit_dialog.edit_type === "chant"
                text: "Description:"
                font.pointSize: root.pointSize
            }
            TextField {
                id: edit_description
                visible: edit_dialog.edit_type === "collection" || edit_dialog.edit_type === "chant"
                Layout.fillWidth: true
                font.pointSize: root.pointSize
            }

            Label {
                visible: edit_dialog.edit_type === "section"
                text: "Pāli Text:"
                font.pointSize: root.pointSize
            }
            ScrollView {
                visible: edit_dialog.edit_type === "section"
                Layout.fillWidth: true
                Layout.preferredHeight: 150

                TextArea {
                    id: edit_content_pali
                    font.pointSize: root.pointSize
                    wrapMode: TextEdit.WordWrap
                }
            }
        }

        onAccepted: {
            if (edit_title.text.trim() === "") return;
            let data = Object.assign({}, edit_dialog.edit_data);
            data.title = edit_title.text.trim();

            if (edit_dialog.edit_type === "collection") {
                data.description = edit_description.text.trim() || null;
                SuttaBridge.update_chanting_collection(JSON.stringify(data));
            } else if (edit_dialog.edit_type === "chant") {
                data.description = edit_description.text.trim() || null;
                SuttaBridge.update_chanting_chant(JSON.stringify(data));
            } else if (edit_dialog.edit_type === "section") {
                data.content_pali = edit_content_pali.text.trim();
                SuttaBridge.update_chanting_section(JSON.stringify(data));
            }
            root.load_collections();
        }
    }

    // --- Remove Confirmation Dialog ---

    Dialog {
        id: remove_dialog
        parent: Overlay.overlay
        anchors.centerIn: parent
        width: 400
        title: "Confirm Removal"
        modal: true
        standardButtons: Dialog.Yes | Dialog.No

        property string remove_title: ""

        Label {
            text: "Remove '" + remove_dialog.remove_title + "' and all its contents?"
            font.pointSize: root.pointSize
            wrapMode: Text.WordWrap
        }

        onAccepted: {
            if (root.selected_type === "collection") {
                SuttaBridge.delete_chanting_collection(root.selected_uid);
            } else if (root.selected_type === "chant") {
                SuttaBridge.delete_chanting_chant(root.selected_uid);
            } else if (root.selected_type === "section") {
                SuttaBridge.delete_chanting_section(root.selected_uid);
            }
            root.selected_uid = "";
            root.selected_type = "";
            root.load_collections();
        }
    }

    // --- Main Layout ---

    ColumnLayout {
        spacing: 0
        anchors.fill: parent
        anchors.topMargin: root.top_bar_margin

        // Toolbar
        RowLayout {
            Layout.fillWidth: true
            Layout.margins: 10
            spacing: 10

            Button {
                text: "Add Collection"
                onClicked: add_collection_dialog.open()
            }

            Button {
                text: "Add Chant"
                enabled: root.selected_type === "collection"
                onClicked: add_chant_dialog.open()
            }

            Button {
                text: "Add Section"
                enabled: root.selected_type === "chant"
                onClicked: add_section_dialog.open()
            }

            Button {
                text: "Open"
                enabled: root.selected_type === "section"
                onClicked: {
                    SuttaBridge.open_chanting_review_window(root.selected_uid);
                }
            }

            Button {
                text: "Edit"
                enabled: root.selected_uid !== ""
                onClicked: {
                    const item = root.find_selected_item();
                    if (!item) return;
                    edit_dialog.edit_type = root.selected_type;
                    edit_dialog.edit_data = item;
                    edit_title.text = item.title || "";
                    edit_description.text = item.description || "";
                    if (root.selected_type === "section") {
                        edit_content_pali.text = item.content_pali || "";
                    }
                    edit_dialog.open();
                }
            }

            Button {
                text: "Remove"
                enabled: root.selected_uid !== ""
                onClicked: {
                    const item = root.find_selected_item();
                    if (!item) return;
                    remove_dialog.remove_title = item.title || "Untitled";
                    remove_dialog.open();
                }
            }

            Item { Layout.fillWidth: true }

            Button {
                visible: root.is_desktop
                text: "Close"
                onClicked: root.close()
            }
        }

        // Tree list
        ScrollView {
            Layout.fillWidth: true
            Layout.fillHeight: true
            contentWidth: availableWidth
            clip: true

            ChantingTreeList {
                collections_list: root.collections_list
                pointSize: root.pointSize

                onSection_clicked: function(section_uid) {
                    SuttaBridge.open_chanting_review_window(section_uid);
                }

                onSelection_changed: function(uid, item_type) {
                    root.selected_uid = uid;
                    root.selected_type = item_type;
                }
            }
        }

        // Mobile close button
        ColumnLayout {
            visible: root.is_mobile
            Layout.fillWidth: true
            Layout.margins: 10
            Layout.bottomMargin: 60
            spacing: 10

            Button {
                text: "Close"
                Layout.fillWidth: true
                onClicked: root.close()
            }
        }
    }

    function find_selected_item() {
        if (!root.selected_uid || !root.selected_type) return null;

        for (let i = 0; i < root.collections_list.length; i++) {
            const col = root.collections_list[i];
            if (root.selected_type === "collection" && col.uid === root.selected_uid) {
                return col;
            }
            if (col.chants) {
                for (let j = 0; j < col.chants.length; j++) {
                    const chant = col.chants[j];
                    if (root.selected_type === "chant" && chant.uid === root.selected_uid) {
                        return chant;
                    }
                    if (chant.sections) {
                        for (let k = 0; k < chant.sections.length; k++) {
                            const sec = chant.sections[k];
                            if (root.selected_type === "section" && sec.uid === root.selected_uid) {
                                return sec;
                            }
                        }
                    }
                }
            }
        }
        return null;
    }
}
