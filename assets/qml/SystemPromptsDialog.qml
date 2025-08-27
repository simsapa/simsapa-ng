pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import QtQuick.Controls
import QtQuick.Window

import com.profoundlabs.simsapa

ApplicationWindow {
    id: root
    title: "System Prompts"
    width: is_mobile ? Screen.desktopAvailableWidth : 800
    height: is_mobile ? Screen.desktopAvailableHeight : 600
    visible: false
    /* visible: true // for qml preview */
    color: palette.window
    flags: Qt.Dialog

    readonly property bool is_mobile: Qt.platform.os === "android" || Qt.platform.os === "ios"
    readonly property bool is_desktop: !root.is_mobile

    readonly property int pointSize: is_mobile? 14 : 12

    property var current_prompts: ({})
    property string selected_prompt_key: ""
    property bool has_unsaved_changes: false

    function load_prompts() {
        let prompts_json = SuttaBridge.get_system_prompts_json();
        try {
            root.current_prompts = JSON.parse(prompts_json);
            
            // Populate the ListView model
            prompt_names_model.clear();
            for (let key in root.current_prompts) {
                prompt_names_model.append({
                    prompt_key: key,
                });
            }
            
            // Select first item if available
            if (prompt_names_model.count > 0) {
                prompt_list_view.currentIndex = 0;
                root.selected_prompt_key = prompt_names_model.get(0).prompt_key;
                prompt_text_area.text = root.current_prompts[root.selected_prompt_key] || "";
            }
        } catch (e) {
            console.error("Failed to parse system prompts:", e);
        }
        root.has_unsaved_changes = false;
    }

    function save_prompts() {
        // Update current prompts with the text from the editor
        if (root.selected_prompt_key) {
            root.current_prompts[root.selected_prompt_key] = prompt_text_area.text;
        }
        
        let prompts_json = JSON.stringify(root.current_prompts);
        SuttaBridge.set_system_prompts_json(prompts_json);
        root.has_unsaved_changes = false;
        root.close();
    }

    Component.onCompleted: {
        load_prompts();
    }

    // FIXME: not effective, since the dialog is closed with visible: false
    onClosing: function(close) {
        if (root.has_unsaved_changes) {
            unsaved_changes_dialog.open();
            close.accepted = false;
        }
    }

    ListModel { id: prompt_names_model }

    Dialog {
        id: unsaved_changes_dialog
        title: "Unsaved Changes"
        anchors.centerIn: parent
        standardButtons: Dialog.Yes | Dialog.No | Dialog.Cancel
        
        Label {
            text: "You have unsaved changes. Do you want to save them?"
        }
        
        onAccepted: {
            root.save_prompts();
        }
        
        onRejected: {
            root.has_unsaved_changes = false;
            root.close();
        }
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
                    source: "icons/svg/settings.svg"
                    Layout.preferredWidth: 32
                    Layout.preferredHeight: 32
                }
                Label {
                    text: "System Prompts"
                    font.bold: true
                    font.pointSize: root.pointSize + 3
                }
            }

            SplitView {
                Layout.fillWidth: true
                Layout.fillHeight: true
                orientation: Qt.Horizontal

                // Left side - Prompt list
                Item {
                    SplitView.preferredWidth: 250
                    SplitView.minimumWidth: 200

                    ColumnLayout {
                        anchors.fill: parent
                        anchors.margins: 5

                        Label {
                            text: "Available Prompts:"
                            font.bold: true
                            font.pointSize: root.pointSize
                        }

                        ListView {
                            id: prompt_list_view
                            Layout.fillWidth: true
                            Layout.fillHeight: true
                            model: prompt_names_model
                            clip: true

                            delegate: ItemDelegate {
                                id: item_delegate
                                required property int index
                                required property string prompt_key

                                width: prompt_list_view.width
                                height: 40

                                highlighted: prompt_list_view.currentIndex === index

                                background: Rectangle {
                                    color: item_delegate.highlighted ? palette.highlight : 
                                           (item_delegate.hovered ? palette.alternateBase : palette.base)
                                    border.width: 1
                                    border.color: palette.mid
                                }

                                Text {
                                    anchors.left: parent.left
                                    anchors.right: parent.right
                                    anchors.verticalCenter: parent.verticalCenter
                                    anchors.leftMargin: 10
                                    anchors.rightMargin: 10
                                    text: item_delegate.prompt_key
                                    font.pointSize: root.pointSize
                                    color: item_delegate.highlighted ? palette.highlightedText : palette.text
                                    elide: Text.ElideRight
                                }

                                onClicked: {
                                    // Save current prompt text before switching
                                    if (root.selected_prompt_key) {
                                        root.current_prompts[root.selected_prompt_key] = prompt_text_area.text;
                                    }
                                    
                                    prompt_list_view.currentIndex = index;
                                    root.selected_prompt_key = item_delegate.prompt_key;
                                    prompt_text_area.text = root.current_prompts[root.selected_prompt_key] || "";
                                }
                            }
                        }
                    }
                }

                // Right side - Prompt editor
                Item {
                    SplitView.fillWidth: true

                    ColumnLayout {
                        anchors.fill: parent
                        anchors.margins: 5

                        Label {
                            text: root.selected_prompt_key ? root.selected_prompt_key : "Select a prompt to edit"
                            font.bold: true
                            font.pointSize: root.pointSize
                        }

                        ScrollView {
                            Layout.fillWidth: true
                            Layout.fillHeight: true

                            TextArea {
                                id: prompt_text_area
                                placeholderText: "Select a prompt from the list to edit..."
                                wrapMode: TextArea.Wrap
                                selectByMouse: true
                                font.pointSize: root.pointSize
                                enabled: root.selected_prompt_key !== ""

                                onTextChanged: {
                                    root.has_unsaved_changes = true;
                                }
                            }
                        }
                    }
                }
            }

            RowLayout {
                spacing: 10

                Label {
                    text: root.has_unsaved_changes ? "• Unsaved changes" : ""
                    font.pointSize: root.pointSize - 1
                    color: "orange"
                    visible: root.has_unsaved_changes
                }

                Item { Layout.fillWidth: true }

                Button {
                    text: "Cancel"
                    onClicked: root.close()
                }

                Button {
                    text: "Save"
                    enabled: root.has_unsaved_changes
                    onClicked: root.save_prompts()
                }
            }
        }
    }
}
