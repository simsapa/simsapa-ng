pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import QtQuick.Dialogs

import com.profoundlabs.simsapa

Item {
    id: root

    required property string window_id
    required property bool is_dark
    readonly property bool is_mobile: Qt.platform.os === "android" || Qt.platform.os === "ios"
    readonly property bool is_desktop: !root.is_mobile
    readonly property bool is_qml_preview: Qt.application.name === "Qml Runtime"

    readonly property int vocab_font_point_size: 10
    readonly property TextMetrics vocab_tm1: TextMetrics { text: "#"; font.pointSize: root.vocab_font_point_size }

    property string text_color: root.is_dark ? "#F0F0F0" : "#000000"
    property string bg_color: root.is_dark ? "#23272E" : "#FAE6B2"
    property string bg_color_lighter: root.is_dark ? "#2E333D" : "#FBEDC7"
    property string bg_color_darker: root.is_dark ? "#1C2025" : "#F8DA8E"
    property string border_color: root.is_dark ? "#0a0a0a" : "#ccc"

    PromptManager { id: pm }

    Connections {
        target: pm

        function onPromptResponseForMessages(sender_message_idx: int, response: string, response_html: string) {
            root.waiting_for_response = false;

            messages_model.append({
                role: "assistant",
                content: response,
                content_html: response_html,
            });
            messages_model.append({
                role: "user",
                content: "",
                content_html: "",
            });

            // Scroll to bottom after adding new messages
            root.scroll_to_bottom();
        }
    }

    property string model_name: "tngtech/deepseek-r1t2-chimera:free"
    property bool waiting_for_response: false

    ListModel { id: messages_model }

    Component.onCompleted: {
        // Load system prompt dynamically from database
        let system_prompt_text = SuttaBridge.get_system_prompt("Prompts Tab: System Prompt");

        // Add a system prompt and an empty user message.
        messages_model.append({
            role: "system",
            content: system_prompt_text,
            content_html: "",
        });
        messages_model.append({
            role: "user",
            content: "",
            content_html: "",
        });

        // Initialize content height tracking after initial messages
        Qt.callLater(function() {
            if (messages_scroll_view.contentItem) {
                root.last_content_height = messages_scroll_view.contentItem.contentHeight;
            }
        });
    }

    function scroll_to_bottom() {
        // Only scroll if needed - check if content exceeds view
        scroll_timer.restart();
    }

    property real last_content_height: 0

    function perform_scroll_if_needed() {
        var contentHeight = messages_scroll_view.contentItem.contentHeight; // qmllint disable missing-property
        var viewHeight = messages_scroll_view.height;

        // Check if new content was added that exceeds the current view
        var contentGrew = contentHeight > root.last_content_height;
        root.last_content_height = contentHeight;

        // Only scroll if:
        // 1. Content actually grew (new messages were added), AND
        // 2. The content now exceeds the view height
        if (contentGrew && contentHeight > viewHeight) {
            messages_scroll_view.ScrollBar.vertical.position = 1.0 - messages_scroll_view.ScrollBar.vertical.size;
        }
    }

    Timer {
        id: scroll_timer
        interval: 150  // Increased delay to ensure layout completion
        repeat: false
        onTriggered: {
            root.perform_scroll_if_needed();
        }
    }

    function new_prompt(prompt: string) {
        messages_model.clear();

        // Load system prompt dynamically from database
        let system_prompt_text = SuttaBridge.get_system_prompt("Prompts Tab: System Prompt");

        messages_model.append({
            role: "system",
            content: system_prompt_text,
            content_html: "",
        });
        messages_model.append({
            role: "user",
            content: prompt,
            content_html: "",
        });
        var item = messages_repeater.itemAt(1);
        if (item && item.send_btn) { // qmllint disable missing-property
            item.send_btn.click();
        }
    }

    FolderDialog {
        id: export_folder_dialog
        acceptLabel: "Export to Folder"
        onAccepted: root.export_dialog_accepted()
    }

    function export_dialog_accepted() {
        if (export_btn.currentIndex === 0) return;
        let save_file_name = null
        let save_content = null;

        if (export_btn.currentValue === "HTML") {
            save_file_name = "chat_export.html";
            save_content = root.chat_as_html();

        } else if (export_btn.currentValue === "Markdown") {
            save_file_name = "chat_export.md";
            save_content = root.chat_as_markdown();

        } else if (export_btn.currentValue === "Org-Mode") {
            save_file_name = "chat_export.org";
            save_content = root.chat_as_orgmode();
        }

        let save_fn = function() {
            let ok = SuttaBridge.save_file(export_folder_dialog.selectedFolder, save_file_name, save_content);
            if (ok) {
                msg_dialog_ok.text = "Export completed."
                msg_dialog_ok.open();
            } else {
                msg_dialog_ok.text = "Export failed."
                msg_dialog_ok.open();
            }
        };

        if (save_file_name) {
            let exists = SuttaBridge.check_file_exists_in_folder(export_folder_dialog.selectedFolder, save_file_name);
            if (exists) {
                msg_dialog_cancel_ok.text = `${save_file_name} exists. Overwrite?`;
                msg_dialog_cancel_ok.accept_fn = save_fn;
                msg_dialog_cancel_ok.open();
            } else {
                save_fn();
            }
        }

        // set the button back to default
        export_btn.currentIndex = 0;
    }

    MessageDialog {
        id: msg_dialog_ok
        buttons: MessageDialog.Ok
    }

    function prompt_as_html(): string {
        // FIXME implement html export
        return "<h1>Prompt Messages</h1>";
    }

    function prompt_as_markdown(): string {
        // FIXME implement markdown export
        return "# Prompt Messages";
    }

    function prompt_as_orgmode(): string {
        // FIXME implement orgmode export
        return "* Prompt Messages";
    }

    TabBar {
        id: tab_bar
        anchors.top: parent.top
        anchors.left: parent.left
        anchors.right: parent.right

        TabButton {
            text: "Prompt"
        }

        TabButton {
            text: "History"
        }
    }

    StackLayout {
        anchors.top: tab_bar.bottom
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.bottom: parent.bottom
        currentIndex: tab_bar.currentIndex

        // Prompt Tab
        ScrollView {
            id: messages_scroll_view
            contentWidth: availableWidth

            background: Rectangle {
                anchors.fill: parent
                border.width: 0
                color: root.bg_color
            }

            ColumnLayout {
                id: messages_column_layout
                Layout.topMargin: 20
                width: parent.width
                spacing: 20

                RowLayout {
                    Layout.topMargin: 10
                    Layout.leftMargin: 10
                    Layout.fillWidth: true

                    ComboBox {
                        id: export_btn
                        model: ["Export As...", "HTML", "Markdown", "Org-Mode"]
                        enabled: messages_model.count > 2
                        onCurrentIndexChanged: {
                            if (export_btn.currentIndex !== 0) {
                                export_folder_dialog.open();
                            }
                        }
                    }
                }

                Repeater {
                    id: messages_repeater
                    model: messages_model
                    delegate: messages_component
                }

                RowLayout {
                    Layout.leftMargin: 10
                    Label {
                        id: waiting_msg
                        text: "Waiting for response..."
                        visible: root.waiting_for_response
                        font.pointSize: root.vocab_font_point_size
                    }
                }

                Item {
                    Layout.fillHeight: true
                }
            }
        }

        // History Tab
        ScrollView {
            Label { text: "History" }
            // ListView {
            //     anchors.fill: parent
            //     model: history_model
            //     spacing: 10
            //     delegate: historyItemDelegate
            // }
        }

        Component {
            id: messages_component

            ColumnLayout {
                id: message_item
                /* anchors.fill: parent */

                required property int index
                required property string role
                required property string content
                required property string content_html

                property bool is_collapsed: collapse_btn.checked
                property bool is_editable: ["user", "system"].includes(message_item.role)

                property alias send_btn: send_btn

                RowLayout {
                    Layout.leftMargin: 10

                    Button {
                        id: collapse_btn
                        checkable: true
                        checked: false
                        icon.source: checked ? "icons/32x32/material-symbols--expand-all.png" : "icons/32x32/material-symbols--collapse-all.png"
                        Layout.alignment: Qt.AlignLeft
                        Layout.preferredWidth: collapse_btn.height
                    }

                    Label {
                        text: message_item.role
                        font.bold: true
                        font.pointSize: root.vocab_font_point_size
                    }
                }

                ColumnLayout {
                    visible: !collapse_btn.checked

                    GroupBox {
                        Layout.fillWidth: true
                        Layout.margins: 10

                        background: Rectangle {
                            anchors.fill: parent
                            color: message_item.is_editable ? root.bg_color_darker : root.bg_color
                            border.width: message_item.is_editable ? 1 : 0
                            border.color: message_item.is_editable ? root.border_color : root.bg_color
                            radius: 5
                        }

                        ColumnLayout {
                            anchors.fill: parent

                            TextArea {
                                id: message_content
                                Layout.fillWidth: true
                                text: message_item.role === "assistant" ? message_item.content_html : message_item.content
                                textFormat: message_item.role === "assistant" ? Text.RichText : Text.PlainText
                                font.pointSize: 12
                                selectByMouse: true
                                wrapMode: TextEdit.WordWrap
                                placeholderText: "Prompt message ..."
                                readOnly: !message_item.is_editable
                                onTextChanged: {
                                    if (text !== message_item.content) {
                                        messages_model.set(message_item.index, {
                                            role: message_item.role,
                                            content: text,
                                            content_html: message_item.content_html,
                                        });
                                    }
                                }
                            }

                            RowLayout {
                                Layout.alignment: Qt.AlignRight

                                Button {
                                    id: send_btn
                                    text: "Send"
                                    visible: message_item.role === "user"
                                    Layout.alignment: Qt.AlignRight
                                    onClicked: {
                                        if (message_content.text.trim().length == 0) {
                                            msg_dialog_ok.text = "Prompt message is empty";
                                            msg_dialog_ok.open();
                                            return;
                                        }

                                        let messages = [];
                                        // Send messages up to this item, so user can change the chat conversation from this point onward
                                        for (var i=0; i <= message_item.index; i++) {
                                            messages.push(messages_model.get(i));
                                        }
                                        let messages_json = JSON.stringify(messages);
                                        pm.prompt_request_with_messages(message_item.index, root.model_name, messages_json);

                                        root.waiting_for_response = true;

                                        // Remove chat items after the sender message.
                                        for (var i=messages_model.count-1; i > message_item.index; i--) {
                                            messages_model.remove(i);
                                        }

                                        // Scroll to bottom to show waiting message
                                        root.scroll_to_bottom();
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

}
