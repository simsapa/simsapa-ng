pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import QtQuick.Controls

Frame {
    id: root

    required property int pointSize
    required property bool is_mobile
    property alias status_text: download_status.text
    property alias progress_value: progress_bar.value
    property bool show_cancel_button: false
    property string quit_button_text: "Quit"

    signal quit_clicked()
    signal cancel_clicked()

    Layout.fillWidth: true
    Layout.fillHeight: true

    ColumnLayout {
        spacing: 0
        anchors.fill: parent

        // Centered content area
        Item {
            Layout.fillWidth: true
            Layout.fillHeight: true

            ColumnLayout {
                anchors.centerIn: parent
                width: parent.width * 0.9
                spacing: 10

                AnimatedImage {
                    id: simsapa_loading_gif
                    source: "icons/gif/simsapa-loading.gif"
                    playing: true
                    Layout.alignment: Qt.AlignCenter
                }

                Label {
                    id: download_status
                    Layout.alignment: Qt.AlignCenter
                    text: "Downloading ..."
                    font.pointSize: root.pointSize
                    wrapMode: Text.WordWrap
                    horizontalAlignment: Text.AlignHCenter
                    Layout.fillWidth: true
                }

                ProgressBar {
                    id: progress_bar
                    Layout.alignment: Qt.AlignCenter
                    Layout.fillWidth: true
                    visible: true
                    from: 0
                    to: 1
                    value: 0
                    font.pointSize: root.pointSize
                }
            }
        }

        // Fixed button area at the bottom
        RowLayout {
            Layout.fillWidth: true
            Layout.margins: 20
            // Extra space on mobile to avoid the bottom bar covering the button.
            Layout.bottomMargin: root.is_mobile ? 60 : 20

            Item { Layout.fillWidth: true }

            Button {
                id: cancel_button
                visible: root.show_cancel_button
                text: "Cancel Downloads"
                font.pointSize: root.pointSize
                onClicked: root.cancel_clicked()
            }

            Button {
                id: download_quit_button
                text: root.quit_button_text
                font.pointSize: root.pointSize
                onClicked: root.quit_clicked()
            }

            Item { Layout.fillWidth: true }
        }
    }
}
