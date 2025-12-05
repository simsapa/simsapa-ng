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

    Component.onCompleted: {
        load_library_books();
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
                text: "Remove"
                enabled: root.selected_book_uid !== ""
                onClicked: {
                    // TODO: Show confirmation dialog and remove book
                    console.log("Remove clicked for:", root.selected_book_uid);
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

                    delegate: Frame {
                        id: book_item
                        Layout.fillWidth: true
                        Layout.margins: 5

                        required property var modelData
                        property bool is_selected: root.selected_book_uid === modelData.uid
                        property bool is_expanded: false
                        property var spine_items: []

                        function load_spine_items() {
                            const json_str = SuttaBridge.get_spine_items_for_book_json(modelData.uid);
                            try {
                                spine_items = JSON.parse(json_str);
                            } catch (e) {
                                console.error("Failed to parse spine items JSON:", e);
                                spine_items = [];
                            }
                        }

                        background: Rectangle {
                            color: book_item.is_selected ? palette.highlight : palette.base
                            border.color: palette.mid
                            border.width: 1
                            radius: 4
                        }

                        ColumnLayout {
                            width: parent.width
                            spacing: 5

                            // Book header with click area
                            Item {
                                Layout.fillWidth: true
                                Layout.preferredHeight: header_row.implicitHeight

                                RowLayout {
                                    id: header_row
                                    anchors.fill: parent
                                    spacing: 10

                                    // Expand/collapse indicator
                                    Label {
                                        text: book_item.is_expanded ? "▼" : "▶"
                                        font.pointSize: root.pointSize - 2
                                        color: palette.text
                                    }

                                    // Document type badge
                                    Rectangle {
                                        Layout.preferredWidth: 50
                                        Layout.preferredHeight: 24
                                        color: {
                                            if (book_item.modelData.document_type === "epub") return "#4A90E2"
                                            if (book_item.modelData.document_type === "pdf") return "#E24A4A"
                                            return "#4AE290"
                                        }
                                        radius: 4

                                        Label {
                                            anchors.centerIn: parent
                                            text: book_item.modelData.document_type.toUpperCase()
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
                                            text: book_item.modelData.title || "Untitled"
                                            font.pointSize: root.pointSize
                                            font.bold: true
                                            color: book_item.is_selected ? palette.highlightedText : palette.text
                                            wrapMode: Text.WordWrap
                                            Layout.fillWidth: true
                                        }

                                        Label {
                                            visible: book_item.modelData.author
                                            text: "by " + (book_item.modelData.author || "")
                                            font.pointSize: root.pointSize - 2
                                            color: book_item.is_selected ? palette.highlightedText : palette.mid
                                            wrapMode: Text.WordWrap
                                            Layout.fillWidth: true
                                        }
                                    }
                                }

                                // Mouse area for selection and expansion
                                MouseArea {
                                    anchors.fill: parent
                                    cursorShape: Qt.PointingHandCursor

                                    onClicked: {
                                        root.selected_book_uid = book_item.modelData.uid;
                                        const was_expanded = book_item.is_expanded;
                                        book_item.is_expanded = !book_item.is_expanded;
                                        
                                        // Load spine items when expanding
                                        if (!was_expanded && book_item.spine_items.length === 0) {
                                            book_item.load_spine_items();
                                        }
                                    }
                                }
                            }

                            // Spine items list (chapters)
                            ColumnLayout {
                                visible: book_item.is_expanded
                                Layout.fillWidth: true
                                Layout.leftMargin: 30
                                spacing: 5

                                Label {
                                    visible: book_item.spine_items.length > 0
                                    text: "Chapters:"
                                    font.pointSize: root.pointSize - 1
                                    font.italic: true
                                    color: palette.mid
                                }

                                Label {
                                    visible: book_item.spine_items.length === 0
                                    text: "No chapters available"
                                    font.pointSize: root.pointSize - 2
                                    color: palette.mid
                                }

                                Repeater {
                                    model: book_item.spine_items

                                    delegate: ItemDelegate {
                                        id: spine_item
                                        Layout.fillWidth: true
                                        
                                        required property var modelData

                                        background: Rectangle {
                                            color: spine_item.hovered ? palette.midlight : "transparent"
                                            radius: 2
                                        }

                                        contentItem: Label {
                                            text: spine_item.modelData.title || "Chapter " + (spine_item.modelData.spine_index + 1)
                                            font.pointSize: root.pointSize - 1
                                            color: palette.text
                                            wrapMode: Text.WordWrap
                                            elide: Text.ElideRight
                                        }

                                        onClicked: {
                                            // TODO: Open chapter in reading view
                                            console.log("Opening chapter:", spine_item.modelData.spine_item_uid);
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
