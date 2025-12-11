pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import QtQuick.Controls

import com.profoundlabs.simsapa

Dialog {
    id: root
    title: "Mobile Top Margin"
    modal: true
    standardButtons: Dialog.Cancel | Dialog.Ok

    readonly property int pointSize: is_mobile ? 16 : 12

    readonly property bool is_mobile: Qt.platform.os === "android" || Qt.platform.os === "ios"
    readonly property bool is_desktop: !root.is_mobile

    property bool use_system_value: true
    property int custom_value: 24

    signal marginChanged()

    anchors.centerIn: parent
    width: 350

    Component.onCompleted: {
        root.load_current_settings();
    }

    function load_current_settings() {
        root.use_system_value = SuttaBridge.is_mobile_top_bar_margin_system();
        if (!root.use_system_value) {
            root.custom_value = SuttaBridge.get_mobile_top_bar_margin_custom_value();
        }
    }

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 10
        spacing: 15

        Label {
            text: "Configure top bar margin for mobile devices:"
            font.pointSize: root.pointSize
            wrapMode: Text.WordWrap
            Layout.fillWidth: true
        }

        CheckBox {
            id: system_checkbox
            text: "Use system value (" + SuttaBridge.get_status_bar_height() + " dp)"
            font.pointSize: root.pointSize
            checked: root.use_system_value
            onCheckedChanged: {
                root.use_system_value = checked;
            }
        }

        RowLayout {
            Layout.fillWidth: true
            spacing: 10
            enabled: !system_checkbox.checked

            Label {
                text: "Custom value (dp):"
                font.pointSize: root.pointSize
                opacity: system_checkbox.checked ? 0.5 : 1.0
            }

            SpinBox {
                id: custom_spinbox
                from: 0
                to: 100
                value: root.custom_value
                editable: true
                font.pointSize: root.pointSize
                opacity: system_checkbox.checked ? 0.5 : 1.0
                onValueModified: {
                    root.custom_value = value;
                }
            }
        }

        Label {
            text: "Note: This setting affects the spacing between the Android status bar and the app's top elements."
            font.pointSize: root.pointSize - 2
            wrapMode: Text.WordWrap
            Layout.fillWidth: true
            opacity: 0.7
        }

        Item { Layout.fillHeight: true }
    }

    onAccepted: {
        if (root.use_system_value) {
            SuttaBridge.set_mobile_top_bar_margin_system();
        } else {
            SuttaBridge.set_mobile_top_bar_margin_custom(root.custom_value);
        }
        root.marginChanged();
    }

    onRejected: {
        // Reset to current settings if canceled
        root.load_current_settings();
    }
}
