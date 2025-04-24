import QtQuick
import QtQuick.Layouts
import QtQuick.Controls

TabButton {
    id: control

    required property int index
    required property string title
    required property bool pinned

    signal pinToggled(bool pinned)
    signal closeClicked()

    /* implicitWidth: Math.min(200, Math.max(150, implicitContentWidth + 30)) */

    contentItem: RowLayout {
        Button {
            id: pin_btn
            checkable: true
            checked: control.pinned
            icon.source: checked ? "icons/32x32/material-symbols-light--push-pin.png" : "icons/32x32/material-symbols-light--push-pin-outline.png"
            Layout.preferredWidth: 24
            flat: true
            onCheckedChanged: control.pinToggled(checked)
        }

        Label {
            text: control.title
            elide: Text.ElideRight
            /* Layout.fillWidth: true */
        }

        Button {
            icon.source: "icons/32x32/mdi--close.png"
            Layout.preferredWidth: 24
            flat: true
            onClicked: control.closeClicked()
        }
    }
}
