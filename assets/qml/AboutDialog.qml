import QtQuick
import QtQuick.Layouts
import QtQuick.Controls
import QtQuick.Window

import com.profoundlabs.simsapa

ApplicationWindow {
    id: root
    title: `About ${root.app_name}`
    width: is_mobile ? Screen.desktopAvailableWidth : 500
    height: is_mobile ? Screen.desktopAvailableHeight : 500
    visible: false
    /* visible: true // for qml preview */
    color: palette.window
    flags: Qt.Dialog

    readonly property bool is_mobile: Qt.platform.os === "android" || Qt.platform.os === "ios"
    readonly property bool is_desktop: !root.is_mobile

    readonly property int pointSize: is_mobile? 14 : 12

    // FIXME make text selectable

    // Application.displayName is simsapa-ng
    property string app_name: "Simsapa Dhamma Reader"
    // Declared in gui.cpp with app.setApplicationVersion("v0.1.0");
    property string app_version: Application.version
    // FIXME: get Qt version
    property string qt_version: ""

    property string current_platform: ""

    SuttaBridge { id: sb }

    function info_lines() {
        return [
            `App version: ${root.app_version}`,
            `Qt Version: ${root.qt_version}`,
            `Current platform: ${root.current_platform}`,
            `App data folder: ${sb.app_data_folder_path()}`,
            `App data folder is writable: ${sb.is_app_data_folder_writable()}`,
        ];
    }

    Item {
        x: 10
        y: 10
        implicitWidth: root.width - 20
        implicitHeight: root.height - 20

        ColumnLayout {
            spacing: 10
            anchors.fill: parent

            RowLayout {
                spacing: 8
                Image {
                    source: "icons/appicons/simsapa.png"
                    Layout.preferredWidth: 64
                    Layout.preferredHeight: 64
                }
                Label {
                    text: root.app_name
                    font.bold: true
                    font.pointSize: root.pointSize + 5
                }
            }

            ColumnLayout {
                spacing: 10
                Text {
                    textFormat: Text.RichText
                    font.pointSize: root.pointSize
                    wrapMode: Text.WordWrap
                    Layout.fillWidth: true
                    text: "<p>" + root.info_lines().join("</p><p>") + "</p>"
                }
                Button {
                    text: "List Contents"
                    onClicked: data_contents.text = sb.app_data_contents_html_table()
                }
                Text {
                    id: data_contents
                    textFormat: Text.RichText
                    font.pointSize: root.pointSize
                    text: ""
                    wrapMode: Text.WordWrap
                    Layout.fillWidth: true
                }
            }

            Item { Layout.fillHeight: true }

            RowLayout {
                spacing: 10
                // Invisible helper for clipboard
                TextEdit {
                    id: clip
                    visible: false
                    function copy_text(text) {
                        clip.text = text;
                        clip.selectAll();
                        clip.copy();
                    }
                }

                Item { Layout.fillWidth: true }

                Button {
                    text: "Copy"
                    onClicked: {
                        let info = root.info_lines().join("\n");
                        info += "\nContents:\n\n" + sb.app_data_contents_plain_table()
                        clip.copy_text(info);
                    }
                }

                Button {
                    text: "OK"
                    onClicked: root.close()
                }
            }
        }
    }
}
