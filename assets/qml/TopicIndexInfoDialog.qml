import QtQuick
import QtQuick.Layouts
import QtQuick.Controls
import QtQuick.Window

ApplicationWindow {
    id: root

    title: "About Topic Index"
    width: is_mobile ? Screen.desktopAvailableWidth : 500
    height: is_mobile ? Screen.desktopAvailableHeight : 500
    visible: false
    color: palette.window
    flags: Qt.Dialog
    modality: Qt.ApplicationModal

    readonly property bool is_mobile: Qt.platform.os === "android" || Qt.platform.os === "ios"
    readonly property bool is_desktop: !root.is_mobile

    readonly property int pointSize: is_mobile? 14 : 12
    required property int top_bar_margin
    property bool is_dark: theme_helper.is_dark

    ThemeHelper {
        id: theme_helper
        target_window: root
    }

    Component.onCompleted: {
        theme_helper.apply();
    }

    ColumnLayout {
        spacing: 0
        anchors.fill: parent
        anchors.topMargin: root.top_bar_margin
        anchors.leftMargin: 10
        anchors.rightMargin: 10

        // Scrollable content area
        ScrollView {
            Layout.fillWidth: true
            Layout.fillHeight: true
            contentWidth: availableWidth
            clip: true

            ColumnLayout {
                width: parent.width
                spacing: 5

                Text {
                    Layout.fillWidth: true
                    text: `
<p><b>CIPS - Comprehensive Index of Pāli Suttas</b></p>

<p>An index of topics, words, people, similes, and titles found in the suttas of
the Pāli canon of the Theravada Buddhist tradition.</p>
`
                    font.pointSize: root.pointSize + 2
                    color: palette.text
                    wrapMode: Text.Wrap
                }

                Text {
                    Layout.fillWidth: true
                    text: `<a href="https://index.readingfaithfully.org">https://index.readingfaithfully.org</a>`
                    font.pointSize: root.pointSize
                    color: palette.link
                    wrapMode: Text.Wrap
                    onLinkActivated: (link) => Qt.openUrlExternally(link)

                    MouseArea {
                        anchors.fill: parent
                        cursorShape: Qt.PointingHandCursor
                        onClicked: Qt.openUrlExternally("https://index.readingfaithfully.org")
                    }
                }


                Text {
                    Layout.fillWidth: true
                    text: `
<p>This is a first draft of an index of the Sutta Piṭaka.</p>
<p>To report missing or incorrect information, please use the contact form:</p>
<p><a href="https://readingfaithfully.org/contact/">https://readingfaithfully.org/contact/</a></p>
`
                    font.pointSize: root.pointSize
                    color: palette.text
                    wrapMode: Text.Wrap
                    onLinkActivated: (link) => Qt.openUrlExternally(link)

                    MouseArea {
                        anchors.fill: parent
                        cursorShape: Qt.PointingHandCursor
                        onClicked: Qt.openUrlExternally("https://readingfaithfully.org/contact/")
                    }
                }

                Item {
                    Layout.fillHeight: true
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
                text: "Close"
                font.pointSize: root.pointSize
                onClicked: root.close()
            }

            Item { Layout.fillWidth: true }
        }
    }
}
