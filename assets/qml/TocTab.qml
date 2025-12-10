pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import QtQuick.Controls

import com.profoundlabs.simsapa

ColumnLayout {
    id: root

    required property string window_id
    required property bool is_dark
    readonly property bool is_mobile: Qt.platform.os === "android" || Qt.platform.os === "ios"
    readonly property bool is_desktop: !root.is_mobile

    property string current_book_uid: ""
    property var current_book_data: null

    readonly property int pointSize: is_mobile ? 16 : 12

    Logger { id: logger }

    // Function to update the TOC when a new chapter is shown
    function update_for_spine_item(spine_item_uid: string) {
        if (!spine_item_uid || spine_item_uid === "") {
            root.current_book_uid = "";
            root.current_book_data = null;
            return;
        }

        // Get the book UID for this spine item
        const book_uid = SuttaBridge.get_book_uid_for_spine_item(spine_item_uid);

        if (!book_uid || book_uid === "") {
            root.current_book_uid = "";
            root.current_book_data = null;
            return;
        }

        // If it's the same book, don't reload
        if (root.current_book_uid === book_uid && root.current_book_data !== null) {
            return;
        }

        root.current_book_uid = book_uid;

        // Get the book data
        const book_json = SuttaBridge.get_book_by_uid_json(book_uid);
        try {
            root.current_book_data = JSON.parse(book_json);
        } catch (e) {
            logger.error("Failed to parse book JSON:", e);
            root.current_book_data = null;
        }
    }

    // Empty state when no book is selected
    Label {
        visible: root.current_book_data === null
        text: "No book chapter is currently displayed.\n\nOpen a book chapter to see its table of contents here."
        font.pointSize: root.pointSize
        wrapMode: Text.WordWrap
        horizontalAlignment: Text.AlignHCenter
        verticalAlignment: Text.AlignVCenter
        Layout.fillWidth: true
        Layout.fillHeight: true
        Layout.margins: 20
        color: palette.mid
    }

    // TOC display when a book is selected
    ScrollView {
        visible: root.current_book_data !== null
        Layout.fillWidth: true
        Layout.fillHeight: true
        contentWidth: availableWidth
        clip: true

        BooksList {
            id: toc_books_list
            books_list: root.current_book_data !== null ? [root.current_book_data] : []
            selected_book_uid: root.current_book_uid
            pointSize: root.pointSize
            auto_expand: true

            onSelected_book_uid_changed: function(uid) {
                // In TocTab, we don't change selection, we just show the TOC
                // So this handler can be empty
            }
        }
    }
}
