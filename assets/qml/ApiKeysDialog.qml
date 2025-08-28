import QtQuick
import QtQuick.Layouts
import QtQuick.Controls
import QtQuick.Window

import com.profoundlabs.simsapa

ApplicationWindow {
    id: root
    title: "API Keys"
    width: is_mobile ? Screen.desktopAvailableWidth : 500
    height: is_mobile ? Screen.desktopAvailableHeight : 300
    visible: false
    /* visible: true // for qml preview */
    color: palette.window
    flags: Qt.Dialog

    readonly property bool is_mobile: Qt.platform.os === "android" || Qt.platform.os === "ios"
    readonly property bool is_desktop: !root.is_mobile

    readonly property int pointSize: is_mobile? 14 : 12

    property string current_api_key: ""

    function load_current_api_key() {
        root.current_api_key = SuttaBridge.get_api_key("OPENROUTER_API_KEY");
        api_key_input.text = root.current_api_key;
    }

    function save_api_key_immediately() {
        let api_keys_json = JSON.stringify({
            "OPENROUTER_API_KEY": api_key_input.text.trim()
        });
        SuttaBridge.set_api_keys(api_keys_json);
    }

    Component.onCompleted: {
        load_current_api_key();
    }

    onVisibilityChanged: {
        // When the dialog is closed, reset the state of key visibility.
        if (!root.visible) {
            show_key.checked = false;
        }
    }

    Item {
        x: 10
        y: 10
        implicitWidth: root.width - 20
        implicitHeight: root.height - 20

        ColumnLayout {
            spacing: 15
            anchors.fill: parent

            RowLayout {
                spacing: 8
                Image {
                    source: "icons/32x32/mdi--key-variant.png"
                    Layout.preferredWidth: 32
                    Layout.preferredHeight: 32
                }
                Label {
                    text: "API Keys"
                    font.bold: true
                    font.pointSize: root.pointSize + 3
                }
            }

            ColumnLayout {
                spacing: 10

                Label {
                    text: "OpenRouter API Key:"
                    font.pointSize: root.pointSize
                }

                RowLayout {
                    TextField {
                        id: api_key_input
                        Layout.fillWidth: true
                        placeholderText: "Enter your OpenRouter API key..."
                        echoMode: show_key.checked ? TextInput.Normal : TextInput.Password
                        font.pointSize: root.pointSize
                        onTextChanged: {
                            if (root.visible) {
                                root.save_api_key_immediately();
                            }
                        }
                    }

                    Button {
                        id: show_key
                        icon.source: show_key.checked ? "icons/32x32/mdi--eye-off-outline.png" : "icons/32x32/mdi--eye-outline.png"
                        checkable: true
                        Layout.preferredHeight: api_key_input.height
                        Layout.preferredWidth: api_key_input.height
                    }
                }

                Text {
                    text: "Get your API key from <a href='https://openrouter.ai/keys'>https://openrouter.ai/keys</a>"
                    font.pointSize: root.pointSize - 1
                    color: palette.windowText
                    textFormat: Text.RichText
                    wrapMode: Text.WordWrap
                    Layout.fillWidth: true
                    onLinkActivated: function(link) {
                        Qt.openUrlExternally(link);
                    }
                    MouseArea {
                        anchors.fill: parent
                        acceptedButtons: Qt.NoButton
                        cursorShape: parent.hoveredLink ? Qt.PointingHandCursor : Qt.ArrowCursor
                    }
                }
            }

            Item { Layout.fillHeight: true }

            RowLayout {
                spacing: 10

                Item { Layout.fillWidth: true }

                Button {
                    text: "OK"
                    onClicked: root.close()
                }
            }
        }
    }
}
