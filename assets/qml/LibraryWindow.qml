pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import QtQuick.Controls
import QtQuick.Window

import com.profoundlabs.simsapa

ApplicationWindow {
    id: root

    title: "Library"
    width: is_mobile ? Screen.desktopAvailableWidth : 800
    height: is_mobile ? Screen.desktopAvailableHeight : Math.min(900, Screen.desktopAvailableHeight)
    visible: true
    color: palette.window
    flags: Qt.Dialog

    readonly property bool is_mobile: Qt.platform.os === "android" || Qt.platform.os === "ios"
    readonly property bool is_desktop: !root.is_mobile

    readonly property int pointSize: is_mobile ? 16 : 12
    readonly property int largePointSize: pointSize + 5

    property var books_list: []
    property var selected_book_uid: ""
    property bool is_dark: false

    Logger { id: logger }

    Component.onCompleted: {
        apply_theme();
        load_library_books();
    }

    function apply_theme() {
        root.is_dark = SuttaBridge.get_theme_name() === "dark";
        var theme_json = SuttaBridge.get_saved_theme();
        if (theme_json.length === 0 || theme_json === "{}") {
            logger.error("Couldn't get theme JSON.")
            return;
        }

        try {
            var d = JSON.parse(theme_json);

            for (var color_group_key in d) {
                if (!root.palette.hasOwnProperty(color_group_key) || root.palette[color_group_key] === undefined) {
                    logger.error("Member not found on root.palette:", color_group_key);
                    continue;
                }
                var color_group = d[color_group_key];
                for (var color_role_key in color_group) {
                    if (!root.palette[color_group_key].hasOwnProperty(color_role_key) || root.palette[color_group_key][color_role_key] === undefined) {
                        logger.error("Member not found on root.palette:", color_group_key, color_role_key);
                        continue;
                    }
                    try {
                        root.palette[color_group_key][color_role_key] = color_group[color_role_key];
                    } catch (e) {
                        logger.error("Could not set palette property:", color_group_key, color_role_key, e);
                    }
                }
            }
        } catch (e) {
            logger.error("Failed to parse theme JSON:", e);
        }
    }

    function load_library_books() {
        const json_str = SuttaBridge.get_all_books_json();
        try {
            books_list = JSON.parse(json_str);
        } catch (e) {
            console.error("Failed to parse books JSON:", e);
            books_list = [];
        }
    }

    DocumentImportDialog {
        id: import_dialog
        parent: Overlay.overlay
        anchors.centerIn: parent

        onImport_completed: (success, message) => {
            if (success) {
                root.load_library_books();
            }
        }
    }

    DocumentMetadataEditDialog {
        id: metadata_edit_dialog
        parent: Overlay.overlay
        anchors.centerIn: parent

        onMetadata_saved: (success, message) => {
            if (success) {
                root.load_library_books();
            }
        }
    }

    Dialog {
        id: remove_confirmation_dialog
        parent: Overlay.overlay
        anchors.centerIn: parent

        width: 400

        title: "Confirm Removal"
        modal: true
        standardButtons: Dialog.Yes | Dialog.No

        property string book_title: ""
        property string book_uid: ""

        Label {
            text: "Remove '" + remove_confirmation_dialog.book_title + "' from library?"
            font.pointSize: root.pointSize
            wrapMode: Text.WordWrap
        }

        onAccepted: {
            const success = SuttaBridge.remove_book(remove_confirmation_dialog.book_uid);
            if (success) {
                // Clear selection
                root.selected_book_uid = "";
                // Refresh library display
                root.load_library_books();
            } else {
                console.error("Failed to remove book:", remove_confirmation_dialog.book_uid);
            }
        }
    }

    ColumnLayout {
        spacing: 0
        anchors.fill: parent

        // Toolbar with action buttons
        RowLayout {
            Layout.fillWidth: true
            Layout.margins: 10
            spacing: 10

            Button {
                text: "Import Document..."
                onClicked: {
                    import_dialog.open();
                }
            }

            Button {
                text: "Edit Metadata"
                enabled: root.selected_book_uid !== ""
                onClicked: {
                    metadata_edit_dialog.load_metadata(root.selected_book_uid);
                    metadata_edit_dialog.open();
                }
            }

            Button {
                text: "Remove"
                enabled: root.selected_book_uid !== ""
                onClicked: {
                    // Find the selected book to get its title
                    const selected_book = root.books_list.find(book => book.uid === root.selected_book_uid);
                    if (selected_book) {
                        remove_confirmation_dialog.book_title = selected_book.title || "Untitled";
                        remove_confirmation_dialog.book_uid = root.selected_book_uid;
                        remove_confirmation_dialog.open();
                    }
                }
            }

            Item { Layout.fillWidth: true }

            Button {
                visible: root.is_desktop
                text: "Close"
                onClicked: {
                    root.close();
                }
            }
        }

        // Main content area
        ScrollView {
            id: scroll_view
            Layout.fillWidth: true
            Layout.fillHeight: true
            contentWidth: availableWidth
            clip: true

            ColumnLayout {
                width: scroll_view.availableWidth
                spacing: 10

                // Empty state message
                Label {
                    visible: root.books_list.length === 0
                    text: "No documents in library. Click 'Import Document...' to add books."
                    font.pointSize: root.pointSize
                    wrapMode: Text.WordWrap
                    horizontalAlignment: Text.AlignHCenter
                    Layout.fillWidth: true
                    Layout.topMargin: 50
                    color: palette.mid
                }

                // Books list
                Repeater {
                    model: root.books_list

                    delegate: ColumnLayout {
                        id: book_item_wrapper
                        Layout.fillWidth: true
                        Layout.margins: 5
                        spacing: 0

                        required property var modelData
                        property bool is_selected: root.selected_book_uid === modelData.uid
                        property bool is_expanded: false
                        property var spine_items: []
                        property var chapter_list: []
                        property bool use_toc: false

                        function load_spine_items() {
                            // First get spine items - we'll need them either way
                            const json_str = SuttaBridge.get_spine_items_for_book_json(modelData.uid);
                            try {
                                spine_items = JSON.parse(json_str);
                            } catch (e) {
                                console.error("Failed to parse spine items JSON:", e);
                                spine_items = [];
                            }

                            // Check if the book has a TOC
                            if (modelData.toc_json && modelData.toc_json.length > 0) {
                                try {
                                    const toc = JSON.parse(modelData.toc_json);
                                    if (toc && toc.length > 0) {
                                        // Use TOC for chapter list, but prepend first spine item (usually cover)
                                        let combined_list = [];

                                        // Add first spine item as cover if it exists
                                        if (spine_items.length > 0) {
                                            combined_list.push(spine_items[0]);
                                        }

                                        // Add TOC items
                                        for (let i = 0; i < toc.length; i++) {
                                            combined_list.push(toc[i]);
                                        }

                                        // Assign the combined list to trigger property change
                                        chapter_list = combined_list;
                                        use_toc = true;
                                        return;
                                    }
                                } catch (e) {
                                    console.error("Failed to parse TOC JSON:", e);
                                }
                            }

                            // Fall back to spine items only
                            chapter_list = spine_items;
                            use_toc = false;
                        }

                        // Book header Frame
                        Frame {
                            id: book_item
                            Layout.fillWidth: true

                            background: Rectangle {
                                color: book_item_wrapper.is_selected ? palette.highlight : palette.base
                                border.color: palette.mid
                                border.width: 1
                                radius: 4
                            }

                            contentItem: Item {
                                implicitWidth: header_row.implicitWidth
                                implicitHeight: header_row.implicitHeight

                                MouseArea {
                                    anchors.fill: parent
                                    cursorShape: Qt.PointingHandCursor

                                    onClicked: {
                                        root.selected_book_uid = book_item_wrapper.modelData.uid;
                                        const was_expanded = book_item_wrapper.is_expanded;
                                        book_item_wrapper.is_expanded = !book_item_wrapper.is_expanded;

                                        // Load chapter list when expanding
                                        if (!was_expanded && book_item_wrapper.chapter_list.length === 0) {
                                            book_item_wrapper.load_spine_items();
                                        }
                                    }
                                }

                                RowLayout {
                                    id: header_row
                                    anchors.fill: parent
                                    spacing: 10

                                // Expand/collapse indicator
                                Label {
                                    text: book_item_wrapper.is_expanded ? "▼" : "▶"
                                    font.pointSize: root.pointSize - 2
                                    color: palette.text
                                }

                                // Document type badge
                                Rectangle {
                                    Layout.preferredWidth: 50
                                    Layout.preferredHeight: 24
                                    color: {
                                        if (book_item_wrapper.modelData.document_type === "epub") return "#4A90E2"
                                        if (book_item_wrapper.modelData.document_type === "pdf") return "#007A31"
                                        return "#FAE6B2"
                                    }
                                    radius: 4

                                    Label {
                                        anchors.centerIn: parent
                                        text: book_item_wrapper.modelData.document_type.toUpperCase()
                                        font.pointSize: root.pointSize - 4
                                        font.bold: true
                                        color: "white"
                                    }
                                }

                                // Title and author
                                ColumnLayout {
                                    Layout.fillWidth: true
                                    spacing: 2

                                    Label {
                                        text: book_item_wrapper.modelData.title || "Untitled"
                                        font.pointSize: root.pointSize
                                        font.bold: true
                                        color: book_item_wrapper.is_selected ? palette.highlightedText : palette.text
                                        wrapMode: Text.WordWrap
                                        Layout.fillWidth: true
                                    }

                                    Label {
                                        visible: book_item_wrapper.modelData.author
                                        text: "by " + (book_item_wrapper.modelData.author || "")
                                        font.pointSize: root.pointSize - 2
                                        color: book_item_wrapper.is_selected ? palette.highlightedText : palette.mid
                                        wrapMode: Text.WordWrap
                                        Layout.fillWidth: true
                                    }

                                    Label {
                                        visible: (book_item_wrapper.modelData.document_type === "epub" || book_item_wrapper.modelData.document_type === "html") && book_item_wrapper.modelData.enable_embedded_css === false
                                        text: "Embedded CSS: Off"
                                        font.pointSize: root.pointSize - 2
                                        color: book_item_wrapper.is_selected ? palette.highlightedText : palette.mid
                                        wrapMode: Text.WordWrap
                                        Layout.fillWidth: true
                                    }
                                }
                            }
                        }
                        }

                        // Spine items list (chapters) - outside the Frame
                        ColumnLayout {
                            visible: book_item_wrapper.is_expanded
                            Layout.fillWidth: true
                            Layout.leftMargin: 30
                            Layout.topMargin: 5
                            spacing: 5

                            Label {
                                visible: book_item_wrapper.chapter_list.length === 0
                                text: "No chapters available"
                                font.pointSize: root.pointSize - 2
                                color: palette.windowText
                            }

                            Repeater {
                                model: book_item_wrapper.chapter_list

                                delegate: ItemDelegate {
                                    id: chapter_item
                                    Layout.fillWidth: true

                                    required property var modelData
                                    required property int index

                                    // Determine if this is a spine item or TOC item
                                    readonly property bool is_spine_item: modelData.hasOwnProperty('spine_item_uid')

                                    background: Rectangle {
                                        color: chapter_item.hovered ? palette.midlight : "transparent"
                                        radius: 2
                                    }

                                    contentItem: Label {
                                        text: chapter_item.is_spine_item
                                            ? (chapter_item.modelData.title || "Chapter " + (chapter_item.modelData.spine_index + 1))
                                            : chapter_item.modelData.label
                                        font.pointSize: root.pointSize - 1
                                        color: palette.text
                                        wrapMode: Text.WordWrap
                                        elide: Text.ElideRight
                                    }

                                    onClicked: {
                                        if (chapter_item.is_spine_item) {
                                            // Spine item: use spine_item_uid directly
                                            const result_data = {
                                                item_uid: chapter_item.modelData.spine_item_uid,
                                                table_name: "book_spine_items",
                                                sutta_title: chapter_item.modelData.title || "Chapter " + (chapter_item.modelData.spine_index + 1),
                                                sutta_ref: ""
                                            };
                                            SuttaBridge.emit_show_chapter_from_library(JSON.stringify(result_data));
                                        } else {
                                            // TOC item: need to look up spine item by resource path
                                            const spine_item_uid = SuttaBridge.get_spine_item_uid_by_path(
                                                book_item_wrapper.modelData.uid,
                                                chapter_item.modelData.content
                                            );

                                            if (spine_item_uid.length > 0) {
                                                const result_data = {
                                                    item_uid: spine_item_uid,
                                                    table_name: "book_spine_items",
                                                    sutta_title: chapter_item.modelData.label,
                                                    sutta_ref: ""
                                                };
                                                SuttaBridge.emit_show_chapter_from_library(JSON.stringify(result_data));
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
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
                onClicked: {
                    root.close();
                }
            }
        }
    }
}
