pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import QtQuick.Controls

Rectangle {
    id: root

    property int dictionary_id: 0
    property string title_text: ""
    property string label_text: ""
    property string language_text: ""
    property int entry_count: 0
    property bool busy: false
    property int point_size: 12

    signal edit_clicked()
    signal delete_clicked()

    color: "transparent"
    border.color: palette.mid
    border.width: 1
    radius: 4
    Layout.fillWidth: true
    implicitHeight: row.implicitHeight + 16

    RowLayout {
        id: row
        anchors.fill: parent
        anchors.margins: 8
        spacing: 12

        ColumnLayout {
            Layout.fillWidth: true
            spacing: 2

            Label {
                text: root.title_text
                font.pointSize: root.point_size
                font.bold: true
                elide: Text.ElideRight
                Layout.fillWidth: true
            }

            Label {
                text: `${root.label_text}  ·  ${root.language_text || "—"}  ·  ${root.entry_count} entries`
                font.pointSize: root.point_size - 2
                color: palette.mid
                elide: Text.ElideRight
                Layout.fillWidth: true
            }
        }

        Button {
            text: "Edit"
            enabled: !root.busy
            onClicked: root.edit_clicked()
        }

        Button {
            icon.source: "icons/32x32/ion--trash-outline.png"
            ToolTip.visible: hovered
            ToolTip.text: "Delete dictionary"
            enabled: !root.busy
            onClicked: root.delete_clicked()
        }
    }
}
