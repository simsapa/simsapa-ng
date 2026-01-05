import QtQuick
import QtQuick.Layouts
import QtQuick.Controls

Dialog {
    id: root

    title: "About Topic Index"
    modal: true
    standardButtons: Dialog.Close

    width: Math.min(500, parent.width - 40)
    height: Math.min(400, parent.height - 40)

    x: (parent.width - width) / 2
    y: (parent.height - height) / 2

    readonly property int pointSize: Qt.platform.os === "android" || Qt.platform.os === "ios" ? 16 : 12

    ScrollView {
        anchors.fill: parent

        ColumnLayout {
            width: parent.width
            spacing: 15

            Text {
                Layout.fillWidth: true
                text: "<b>CIPS - Comprehensive Index of Pali Suttas</b>"
                font.pointSize: root.pointSize + 2
                color: palette.text
                wrapMode: Text.Wrap
            }

            Text {
                Layout.fillWidth: true
                text: "This topic index provides a comprehensive guide to subjects and themes found in the Pali suttas. Browse by letter or search for specific topics to discover relevant sutta passages."
                font.pointSize: root.pointSize
                color: palette.text
                wrapMode: Text.Wrap
            }

            Text {
                Layout.fillWidth: true
                text: "<b>How to Use</b>"
                font.pointSize: root.pointSize + 1
                color: palette.text
                wrapMode: Text.Wrap
            }

            Text {
                Layout.fillWidth: true
                text: "• Click letter buttons (A-Z) to browse topics alphabetically\n• Use the search field to find topics by keyword\n• Click on sutta references to open them in the reader\n• Cross-references (\"see:\") link to related topics"
                font.pointSize: root.pointSize
                color: palette.text
                wrapMode: Text.Wrap
            }

            Text {
                Layout.fillWidth: true
                text: "<b>Credits</b>"
                font.pointSize: root.pointSize + 1
                color: palette.text
                wrapMode: Text.Wrap
            }

            Text {
                Layout.fillWidth: true
                text: "The CIPS index is created and maintained by the Dhamma Talks community."
                font.pointSize: root.pointSize
                color: palette.text
                wrapMode: Text.Wrap
            }

            Text {
                Layout.fillWidth: true
                text: "<a href=\"https://cips.dhammatalks.net/\">https://cips.dhammatalks.net/</a>"
                font.pointSize: root.pointSize
                color: palette.link
                wrapMode: Text.Wrap
                onLinkActivated: (link) => Qt.openUrlExternally(link)

                MouseArea {
                    anchors.fill: parent
                    cursorShape: Qt.PointingHandCursor
                    onClicked: Qt.openUrlExternally("https://cips.dhammatalks.net/")
                }
            }

            Item {
                Layout.fillHeight: true
            }
        }
    }
}
