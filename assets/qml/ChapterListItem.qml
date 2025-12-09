pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import QtQuick.Controls

import com.profoundlabs.simsapa

ItemDelegate {
    id: chapter_item
    Layout.fillWidth: true

    required property var item_data
    required property int depth
    required property bool has_children
    required property bool is_expanded
    required property string book_uid
    required property int pointSize

    signal toggle_expanded()
    signal chapter_clicked(string spine_item_uid, string title, string anchor)

    // Determine if this is a spine item or TOC item
    readonly property bool is_spine_item: item_data.hasOwnProperty('spine_item_uid')

    background: Rectangle {
        color: chapter_item.hovered ? palette.midlight : "transparent"
        radius: 2
    }

    contentItem: RowLayout {
        spacing: 5

        // Indentation spacer
        Item {
            Layout.preferredWidth: chapter_item.depth * 20
        }

        // Expand/collapse indicator for items with children
        Label {
            visible: chapter_item.has_children
            text: chapter_item.is_expanded ? "▼" : "▶"
            font.pointSize: chapter_item.pointSize - 3
            color: palette.text
            Layout.preferredWidth: 15

            MouseArea {
                anchors.fill: parent
                cursorShape: Qt.PointingHandCursor
                onClicked: {
                    chapter_item.toggle_expanded();
                }
            }
        }

        // Spacer for items without children to align with items that have children
        Item {
            visible: !chapter_item.has_children
            Layout.preferredWidth: 15
        }

        // Chapter title
        Label {
            text: chapter_item.is_spine_item
                ? (chapter_item.item_data.title || "Chapter " + (chapter_item.item_data.spine_index + 1))
                : chapter_item.item_data.label
            font.pointSize: chapter_item.pointSize - 1
            color: palette.text
            wrapMode: Text.WordWrap
            elide: Text.ElideRight
            Layout.fillWidth: true
        }
    }

    onClicked: {
        if (chapter_item.is_spine_item) {
            // Spine item: use spine_item_uid directly with no anchor
            chapter_item.chapter_clicked(
                chapter_item.item_data.spine_item_uid,
                chapter_item.item_data.title || "Chapter " + (chapter_item.item_data.spine_index + 1),
                ""
            );
        } else {
            // TOC item: need to look up spine item by resource path
            // Split the content path to separate file path from anchor
            const content_path = chapter_item.item_data.content;
            const hash_index = content_path.indexOf('#');
            const file_path = hash_index >= 0 ? content_path.substring(0, hash_index) : content_path;
            const anchor = hash_index >= 0 ? content_path.substring(hash_index) : "";

            const spine_item_uid = SuttaBridge.get_spine_item_uid_by_path(
                chapter_item.book_uid,
                file_path
            );

            if (spine_item_uid.length > 0) {
                chapter_item.chapter_clicked(
                    spine_item_uid,
                    chapter_item.item_data.label,
                    anchor
                );
            }
        }
    }
}
