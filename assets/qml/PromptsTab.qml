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

        function onPromptResponseForMessages(sender_message_idx: int, model_name: string, response: string) {
            console.log(`🤖 onPromptResponseForMessages received: sender_message_idx=${sender_message_idx}, model_name=${model_name}`);
            console.log(`📝 Response content: "${response.substring(0, 100)}..."`);

            root.waiting_for_response = false;

            // Find the assistant message that should receive this response
            // The assistant message will be after the sender message
            let assistant_message_idx = sender_message_idx + 1;
            if (assistant_message_idx >= messages_model.count) {
                console.error(`❌ Assistant message index ${assistant_message_idx} is out of bounds (count: ${messages_model.count})`);
                return;
            }

            let assistant_message = messages_model.get(assistant_message_idx);
            if (!assistant_message || assistant_message.role !== "assistant") {
                console.error(`❌ No assistant message found at index ${assistant_message_idx}`);
                return;
            }

            // Parse current responses
            let responses = [];
            if (assistant_message.responses_json) {
                try {
                    responses = JSON.parse(assistant_message.responses_json);
                    console.log(`📚 Parsed ${responses.length} existing responses`);
                } catch (e) {
                    console.error("Failed to parse responses_json:", e);
                    return;
                }
            }

            // Update the specific model's response
            for (var i = 0; i < responses.length; i++) {
                if (responses[i].model_name === model_name) {
                    let is_error = root.is_error_response(response);
                    let current_retry_count = responses[i].retry_count || 0;

                    console.log(`🔄 Updating response for ${model_name}: is_error=${is_error}, retry_count=${current_retry_count}`);

                    responses[i].response = response;
                    responses[i].status = is_error ? "error" : "completed";
                    responses[i].last_updated = Date.now();

                    // Handle automatic retry for errors (up to 5 times)
                    if (is_error && current_retry_count < 5 && root.ai_models_auto_retry && !root.is_rate_limit_error(response)) {
                        console.log(`🔁 Scheduling automatic retry for ${model_name}`);
                        Qt.callLater(function() {
                            root.handle_retry_request(assistant_message_idx, model_name, root.generate_request_id());
                        });
                    } else if (is_error && root.is_rate_limit_error(response)) {
                        console.log(`⏸️  Skipping auto-retry for rate limit error: ${model_name}`);
                    } else if (is_error && !root.ai_models_auto_retry) {
                        console.log(`⏸️  Auto-retry disabled, not retrying: ${model_name}`);
                    }

                    console.log(`✅ Updated response data:`, JSON.stringify(responses[i]));
                    break;
                }
            }

            // Update the assistant message with new responses
            messages_model.setProperty(assistant_message_idx, "responses_json", JSON.stringify(responses));
            console.log(`💾 Saved responses_json to message model`);
        }
    }

    property bool waiting_for_response: false
    required property bool ai_models_auto_retry

    ListModel { id: messages_model }
    ListModel { id: available_models }

    function load_available_models() {
        console.log(`🔄 Loading available models...`);
        available_models.clear();
        let models_json = SuttaBridge.get_models_json();
        console.log(`📥 Raw models JSON: "${models_json}"`);
        try {
            let models_array = JSON.parse(models_json);
            console.log(`📊 Parsed ${models_array.length} models`);
            for (var i = 0; i < models_array.length; i++) {
                var item = models_array[i];
                console.log(`  [${i}] ${item.model_name}: enabled=${item.enabled}`);
                available_models.append(item);
            }
        } catch (e) {
            console.error("Failed to parse models JSON:", e);
        }
    }

    function generate_request_id() {
        return Date.now().toString() + "_" + Math.random().toString(36);
    }

    function is_error_response(response_text) {
        return response_text.includes("API Error:") ||
               response_text.includes("Error:") ||
               response_text.includes("Failed:");
    }

    function is_rate_limit_error(response_text) {
        return response_text.includes("API Error: Rate limit exceeded");
    }

    function handle_retry_request(message_idx, model_name, new_request_id) {
        var message = messages_model.get(message_idx);
        if (!message || !message.responses_json) return;

        try {
            var responses = JSON.parse(message.responses_json);
            for (var i = 0; i < responses.length; i++) {
                if (responses[i].model_name === model_name) {
                    // Update the response entry for retry
                    responses[i].request_id = new_request_id;
                    responses[i].status = "waiting";
                    responses[i].retry_count = (responses[i].retry_count || 0) + 1;
                    responses[i].last_updated = Date.now();

                    // Append retry message to response
                    var retry_msg = `\n\nRetrying... (${responses[i].retry_count}x)`;
                    if (responses[i].response && !responses[i].response.includes("Retrying...")) {
                        responses[i].response += retry_msg;
                    }

                    // Update the model
                    messages_model.setProperty(message_idx, "responses_json", JSON.stringify(responses));

                    // Compose message history up to the user message that triggered the original request
                    let user_message_idx = message_idx - 1; // Assistant message is after user message
                    if (user_message_idx >= 0) {
                        let messages = [];
                        for (var j = 0; j <= user_message_idx; j++) {
                            let msg = messages_model.get(j);
                            if (msg.role === "assistant" && msg.responses_json) {
                                // For multi-response assistant messages, use the currently selected response
                                try {
                                    let assistant_responses = JSON.parse(msg.responses_json);
                                    let selected_idx = msg.selected_ai_tab || 0;
                                    if (selected_idx < assistant_responses.length && assistant_responses[selected_idx].status === "completed") {
                                        messages.push({
                                            role: "assistant",
                                            content: assistant_responses[selected_idx].response
                                        });
                                    }
                                } catch (e) {
                                    console.error("Failed to parse assistant responses_json:", e);
                                }
                            } else {
                                // For user/system messages
                                messages.push({
                                    role: msg.role,
                                    content: msg.content
                                });
                            }
                        }
                        let messages_json = JSON.stringify(messages);

                        // Send new request
                        pm.prompt_request_with_messages(user_message_idx, model_name, messages_json);
                    }
                    break;
                }
            }
        } catch (e) {
            console.error("Failed to handle retry request:", e);
        }
    }

    function update_tab_selection(message_idx, tab_index, model_name) {
        // Update the selected tab index for this message
        messages_model.setProperty(message_idx, "selected_ai_tab", tab_index);
    }

    Component.onCompleted: {
        // Load system prompt dynamically from database
        let system_prompt_text = SuttaBridge.get_system_prompt("Prompts Tab: System Prompt");

        // Add a system prompt and an empty user message.
        messages_model.append({
            role: "system",
            content: system_prompt_text,
            content_html: "",
            responses_json: "[]",
            selected_ai_tab: 0
        });
        messages_model.append({
            role: "user",
            content: "",
            content_html: "",
            responses_json: "[]",
            selected_ai_tab: 0
        });

        // Initialize ScrollableHelper after initial messages
        Qt.callLater(function() {
            scroll_helper.initialize();
        });
    }

    ScrollableHelper {
        id: scroll_helper
        target_scroll_view: messages_scroll_view
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
                required property string responses_json
                required property int selected_ai_tab

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

                            // AssistantResponses for assistant messages
                            AssistantResponses {
                                id: assistant_responses_component
                                visible: message_item.role === "assistant"
                                is_dark: root.is_dark
                                Layout.fillWidth: true


                                translations_data: {
                                    console.log(`🔍 AssistantResponses for message ${message_item.index}: role=${message_item.role}, responses_json="${message_item.responses_json}"`);
                                    try {
                                        let data = JSON.parse(message_item.responses_json || "[]");
                                        console.log(`📊 Parsed translations_data:`, JSON.stringify(data));
                                        return data;
                                    } catch (e) {
                                        console.error(`❌ Error parsing responses_json for message ${message_item.index}:`, e);
                                        return [];
                                    }
                                }
                                paragraph_text: message_item.content
                                paragraph_index: message_item.index
                                selected_tab_index: message_item.selected_ai_tab || 0

                                onRetryRequest: function(model_name, request_id) {
                                    root.handle_retry_request(message_item.index, model_name, request_id);
                                }

                                onTabSelectionChanged: function(tab_index, model_name) {
                                    root.update_tab_selection(message_item.index, tab_index, model_name);
                                }
                            }

                            // TextArea for user/system messages
                            TextArea {
                                id: message_content
                                visible: message_item.role !== "assistant"
                                Layout.fillWidth: true
                                text: message_item.content
                                textFormat: Text.PlainText
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
                                            responses_json: message_item.responses_json || "",
                                            selected_ai_tab: message_item.selected_ai_tab || 0
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

                                        console.log(`🚀 Send button clicked for message ${message_item.index}`);

                                        // Load enabled models
                                        root.load_available_models();
                                        console.log(`📋 Loaded ${available_models.count} available models`);

                                        // Create responses array for each enabled model
                                        let responses = [];
                                        for (var i = 0; i < available_models.count; i++) {
                                            var model = available_models.get(i);
                                            if (model.enabled) {
                                                console.log(`✅ Adding enabled model: ${model.model_name}`);
                                                responses.push({
                                                    model_name: model.model_name,
                                                    status: "waiting",
                                                    response: "",
                                                    request_id: root.generate_request_id(),
                                                    retry_count: 0,
                                                    last_updated: Date.now(),
                                                    user_selected: responses.length === 0  // First model selected by default
                                                });
                                            } else {
                                                console.log(`⏭️  Skipping disabled model: ${model.model_name}`);
                                            }
                                        }

                                        console.log(`📊 Created ${responses.length} response entries`);

                                        if (responses.length === 0) {
                                            msg_dialog_ok.text = "No AI models are enabled. Please enable at least one model in settings.";
                                            msg_dialog_ok.open();
                                            return;
                                        }

                                        // Remove chat items after the sender message.
                                        for (var i=messages_model.count-1; i > message_item.index; i--) {
                                            messages_model.remove(i);
                                        }

                                        // Add assistant message with responses_json
                                        messages_model.append({
                                            role: "assistant",
                                            content: "",
                                            content_html: "",
                                            responses_json: JSON.stringify(responses),
                                            selected_ai_tab: 0
                                        });

                                        // Add new empty user message for next turn
                                        messages_model.append({
                                            role: "user",
                                            content: "",
                                            content_html: ""
                                        });

                                        // Compose chat message list from conversation history
                                        let messages = [];
                                        for (var i = 0; i <= message_item.index; i++) {
                                            let msg = messages_model.get(i);
                                            if (msg.role === "assistant" && msg.responses_json) {
                                                // For multi-response assistant messages, use the currently selected response
                                                try {
                                                    let assistant_responses = JSON.parse(msg.responses_json);
                                                    let selected_idx = msg.selected_ai_tab || 0;
                                                    if (selected_idx < assistant_responses.length && assistant_responses[selected_idx].status === "completed") {
                                                        messages.push({
                                                            role: "assistant",
                                                            content: assistant_responses[selected_idx].response
                                                        });
                                                        console.log(`📝 Added assistant message from ${assistant_responses[selected_idx].model_name}`);
                                                    }
                                                    // Skip assistant messages that don't have completed selected responses
                                                } catch (e) {
                                                    console.error("Failed to parse assistant responses_json:", e);
                                                }
                                            } else {
                                                // For user/system messages
                                                messages.push({
                                                    role: msg.role,
                                                    content: msg.content
                                                });
                                                console.log(`📝 Added ${msg.role} message`);
                                            }
                                        }
                                        let messages_json = JSON.stringify(messages);
                                        console.log(`📤 Composed message history with ${messages.length} messages`);

                                        // Send requests to all enabled models using the same message history
                                        for (var j = 0; j < responses.length; j++) {
                                            console.log(`🎯 Sending request to ${responses[j].model_name}`);
                                            pm.prompt_request_with_messages(
                                                message_item.index, // sender message index (user message that triggered this)
                                                responses[j].model_name,
                                                messages_json
                                            );
                                        }

                                        root.waiting_for_response = true;

                                        // Scroll to bottom to show waiting message
                                        scroll_helper.scroll_to_bottom();
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
