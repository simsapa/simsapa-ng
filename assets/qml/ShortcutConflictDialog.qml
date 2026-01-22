pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import QtQuick.Controls
import QtQuick.Window

import com.profoundlabs.simsapa

ApplicationWindow {
    id: root
    title: "Shortcut Conflict"
    width: is_mobile ? Screen.desktopAvailableWidth : 400
    height: is_mobile ? Screen.desktopAvailableHeight : 200
    visible: false
    color: palette.window
    flags: Qt.Dialog

    readonly property bool is_mobile: Qt.platform.os === "android" || Qt.platform.os === "ios"
    readonly property bool is_desktop: !root.is_mobile

    readonly property int pointSize: is_mobile ? 14 : 12
    required property int top_bar_margin

    // Properties for the dialog
    property string shortcut: ""
    property string conflicting_action_name: ""

    // Signals
    signal confirmed()
    signal cancelled()

    property bool is_dark: theme_helper.is_dark

    ThemeHelper {
        id: theme_helper
        target_window: root
    }

    Component.onCompleted: {
        theme_helper.apply();
    }

    Frame {
        anchors.fill: parent

        ColumnLayout {
            spacing: 15
            anchors.fill: parent
            anchors.topMargin: root.top_bar_margin
            anchors.margins: 15

            // Warning icon and title
            RowLayout {
                spacing: 10
                Layout.fillWidth: true

                Label {
                    text: "\u26A0"
                    font.pointSize: root.pointSize + 8
                    color: "#e6a700"
                }

                Label {
                    text: "Shortcut Conflict"
                    font.bold: true
                    font.pointSize: root.pointSize + 2
                }
            }

            // Message
            Label {
                text: `The shortcut "${root.shortcut}" is already assigned to "${root.conflicting_action_name}".`
                font.pointSize: root.pointSize
                wrapMode: Text.WordWrap
                Layout.fillWidth: true
            }

            Label {
                text: "Remove it from that action?"
                font.pointSize: root.pointSize
                Layout.fillWidth: true
            }

            Item { Layout.fillHeight: true }

            // Buttons
            RowLayout {
                spacing: 10
                Layout.fillWidth: true
                Layout.bottomMargin: root.is_mobile ? 60 : 10

                Item { Layout.fillWidth: true }

                Button {
                    text: "No"
                    font.pointSize: root.pointSize
                    onClicked: {
                        root.cancelled();
                        root.close();
                    }
                }

                Button {
                    text: "Yes"
                    font.pointSize: root.pointSize
                    highlighted: true
                    onClicked: {
                        root.confirmed();
                        root.close();
                    }
                }
            }
        }
    }
}
