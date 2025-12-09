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
                        property var expanded_items: ({}) // Track expanded state of items with children

                        // Flatten the TOC tree into a flat list with depth information
                        function flatten_toc(toc_items, depth) {
                            let result = [];
                            for (let i = 0; i < toc_items.length; i++) {
                                const item = toc_items[i];
                                const has_children = item.children && item.children.length > 0;
                                const item_key = depth + "_" + i + "_" + item.label;

                                // Add the item with metadata
                                result.push({
                                    data: item,
                                    depth: depth,
                                    has_children: has_children,
                                    item_key: item_key,
                                    is_expanded: expanded_items[item_key] || false
                                });

                                // If expanded and has children, recursively add children
                                if (has_children && expanded_items[item_key]) {
                                    const children_flat = flatten_toc(item.children, depth + 1);
                                    result = result.concat(children_flat);
                                }
                            }
                            return result;
                        }

                        function toggle_item_expanded(item_key) {
                            // Toggle the expanded state
                            const new_expanded = Object.assign({}, expanded_items);
                            new_expanded[item_key] = !new_expanded[item_key];
                            expanded_items = new_expanded;

                            // Rebuild the chapter list to reflect the change
                            rebuild_chapter_list();
                        }

                        function rebuild_chapter_list() {
                            if (!use_toc) {
                                // Spine items only - no nesting
                                chapter_list = spine_items.map((item, idx) => ({
                                    data: item,
                                    depth: 0,
                                    has_children: false,
                                    item_key: "spine_" + idx,
                                    is_expanded: false
                                }));
                                return;
                            }

                            // Get the raw TOC from modelData
                            try {
                                const toc = JSON.parse(modelData.toc_json);
                                let combined_list = [];

                                // Add first spine item as cover if it exists
                                if (spine_items.length > 0) {
                                    combined_list.push({
                                        data: spine_items[0],
                                        depth: 0,
                                        has_children: false,
                                        item_key: "cover",
                                        is_expanded: false
                                    });
                                }

                                // Add flattened TOC items
                                const toc_flat = flatten_toc(toc, 0);
                                combined_list = combined_list.concat(toc_flat);

                                // Assign the combined list to trigger property change
                                chapter_list = combined_list;
                            } catch (e) {
                                console.error("Failed to rebuild chapter list:", e);
                            }
                        }

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
                                        use_toc = true;
                                        rebuild_chapter_list();
                                        return;
                                    }
                                } catch (e) {
                                    console.error("Failed to parse TOC JSON:", e);
                                }
                            }

                            // Fall back to spine items only
                            use_toc = false;
                            rebuild_chapter_list();
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

                                delegate: ChapterListItem {
                                    required property var modelData
                                    required property int index

                                    item_data: modelData.data
                                    depth: modelData.depth
                                    has_children: modelData.has_children
                                    is_expanded: modelData.is_expanded
                                    book_uid: book_item_wrapper.modelData.uid
                                    pointSize: root.pointSize

                                    onToggle_expanded: {
                                        book_item_wrapper.toggle_item_expanded(modelData.item_key);
                                    }

                                    onChapter_clicked: (spine_item_uid, title, anchor) => {
                                        const result_data = {
                                            item_uid: spine_item_uid,
                                            table_name: "book_spine_items",
                                            sutta_title: title,
                                            sutta_ref: "",
                                            anchor: anchor
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
