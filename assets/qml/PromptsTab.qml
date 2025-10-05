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

    Logger { id: logger }
    PromptManager { id: pm }

    property alias prompt_connections: prompt_connections

    Connections {
        id: prompt_connections
        target: pm

        function onPromptResponseForMessages(sender_message_idx: int, model_name: string, response: string) {
            logger.log(`🤖 onPromptResponseForMessages received: sender_message_idx=${sender_message_idx}, model_name=${model_name}`);
            logger.log(`📝 Response content: "${response.substring(0, 100)}..."`);

            root.waiting_for_response = false;

            // Find the assistant message that should receive this response
            // The assistant message will be after the sender message
            let assistant_message_idx = sender_message_idx + 1;
            if (assistant_message_idx >= messages_model.count) {
                logger.error(`❌ Assistant message index ${assistant_message_idx} is out of bounds (count: ${messages_model.count})`);
                return;
            }

            let assistant_message = messages_model.get(assistant_message_idx);
            if (!assistant_message || assistant_message.role !== "assistant") {
                logger.error(`❌ No assistant message found at index ${assistant_message_idx}`);
                return;
            }

            // Parse current responses
            let responses = [];
            if (assistant_message.responses_json) {
                try {
                    responses = JSON.parse(assistant_message.responses_json);
                    logger.log(`📚 Parsed ${responses.length} existing responses`);
                } catch (e) {
                    logger.error("Failed to parse responses_json:", e);
                    return;
                }
            }

            // Update the specific model's response
            for (var i = 0; i < responses.length; i++) {
                if (responses[i].model_name === model_name) {
                    let is_error = root.is_error_response(response);
                    let current_retry_count = responses[i].retry_count || 0;

                    logger.log(`🔄 Updating response for ${model_name}: is_error=${is_error}, retry_count=${current_retry_count}`);

                    responses[i].response = response;
                    responses[i].status = is_error ? "error" : "completed";
                    responses[i].last_updated = Date.now();

                    // Handle automatic retry for errors (up to 5 times)
                    if (is_error && current_retry_count < 5 && root.ai_models_auto_retry && !root.is_rate_limit_error(response)) {
                        logger.log(`🔁 Scheduling automatic retry for ${model_name}`);
                        Qt.callLater(function() {
                            root.handle_retry_request(assistant_message_idx, model_name, root.generate_request_id());
                        });
                    } else if (is_error && root.is_rate_limit_error(response)) {
                        logger.log(`⏸️  Skipping auto-retry for rate limit error: ${model_name}`);
                    } else if (is_error && !root.ai_models_auto_retry) {
                        logger.log(`⏸️  Auto-retry disabled, not retrying: ${model_name}`);
                    }

                    logger.log(`✅ Updated response data:`, JSON.stringify(responses[i]));
                    break;
                }
            }

            // Update the assistant message with new responses
            messages_model.setProperty(assistant_message_idx, "responses_json", JSON.stringify(responses));
            logger.log(`💾 Saved responses_json to message model`);
        }
    }

    property bool waiting_for_response: false
    required property bool ai_models_auto_retry

    property alias messages_model: messages_model
    property alias available_models: available_models

    ListModel { id: messages_model }
    ListModel { id: available_models }

    function load_available_models() {
        logger.log(`🔄 Loading available models from all providers...`);
        available_models.clear();
        let providers_json = SuttaBridge.get_providers_json();
        logger.log(`📥 Raw providers JSON: "${providers_json}"`);
        try {
            let providers_array = JSON.parse(providers_json);
            logger.log(`📊 Parsing ${providers_array.length} providers`);
            for (var i = 0; i < providers_array.length; i++) {
                var provider = providers_array[i];
                logger.log(`  Provider ${provider.name}: enabled=${provider.enabled}`);

                // Only load models from enabled providers
                if (provider.enabled) {
                    for (var j = 0; j < provider.models.length; j++) {
                        var model = provider.models[j];
                        logger.log(`    [${j}] ${model.model_name}: enabled=${model.enabled}`);
                        available_models.append({
                            model_name: model.model_name,
                            enabled: model.enabled,
                            removable: model.removable
                        });
                    }
                } else {
                    logger.log(`    Skipping disabled provider ${provider.name}`);
                }
            }
            logger.log(`🎯 Total models loaded: ${available_models.count}`);
        } catch (e) {
            logger.error("Failed to parse providers JSON:", e);
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
                                    logger.error("Failed to parse assistant responses_json:", e);
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
                        let provider_name = SuttaBridge.get_provider_for_model(model_name);
                        pm.prompt_request_with_messages(user_message_idx, provider_name, model_name, messages_json);
                    }
                    break;
                }
            }
        } catch (e) {
            logger.error("Failed to handle retry request:", e);
        }
    }

    function update_tab_selection(message_idx, tab_index, model_name) {
        // Update the selected tab index for this message
        var message = messages_model.get(message_idx);
        if (message) {
            messages_model.setProperty(message_idx, "selected_ai_tab", tab_index);
        }
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
                msg_dialog_ok.text = "Exported as: " + save_file_name;
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

    MessageDialog {
        id: msg_dialog_cancel_ok
        buttons: MessageDialog.Cancel | MessageDialog.Ok
        property var accept_fn: {}
        onAccepted: accept_fn() // qmllint disable use-proper-function
    }

    MessageDialog {
        id: no_models_dialog
        title: "No AI Models"
        text: "There are no enabled models. See Prompts menu > AI Models"
        buttons: MessageDialog.Ok
    }

    function chat_export_data(): var {
        let chat_data = {
            messages: []
        };

        for (var i = 0; i < messages_model.count; i++) {
            var message = messages_model.get(i);
            if (!message) continue;

            var msg_data = {
                role: message.role,
                content: message.content ? message.content.trim() : "",
                responses: []
            };

            if (message.role === "user" && (!msg_data.content || msg_data.content === "")) {
                continue;
            }

            if (message.role === "assistant" && message.responses_json) {
                try {
                    var responses = JSON.parse(message.responses_json);
                    var selected_tab_index = message.selected_ai_tab || 0;
                    var selected_response = null;
                    var other_responses = [];

                    for (var j = 0; j < responses.length; j++) {
                        var resp = responses[j];
                        if (resp.status === "completed" && resp.response && resp.response.trim()) {
                            var isSelected = (j === selected_tab_index);
                            if (isSelected) {
                                selected_response = {
                                    model_name: resp.model_name,
                                    response: resp.response,
                                    is_selected: true
                                };
                            } else {
                                other_responses.push({
                                    model_name: resp.model_name,
                                    response: resp.response,
                                    is_selected: false
                                });
                            }
                        }
                    }

                    if (selected_response) {
                        msg_data.responses.push(selected_response);
                    }
                    msg_data.responses = msg_data.responses.concat(other_responses);

                } catch (e) {
                    logger.error("Failed to parse responses_json:", e);
                }
            }

            chat_data.messages.push(msg_data);
        }

        return chat_data;
    }

    function chat_as_html(): string {
        let chat_data = root.chat_export_data();

        let out = `
<!doctype html>
<html>
<head>
    <meta charset="utf-8">
    <meta http-equiv="x-ua-compatible" content="ie=edge">
    <title>Chat Export</title>
    <meta name="viewport" content="width=device-width, initial-scale=1">
</head>
<body>
<h1>Chat Export</h1>
`;

        for (var i = 0; i < chat_data.messages.length; i++) {
            var msg = chat_data.messages[i];
            
            if (msg.role === "system") {
                out += `\n<h2>System</h2>\n`;
                out += `<blockquote>${msg.content.replace(/\n/g, "<br>\n")}</blockquote>\n`;
            } else if (msg.role === "user") {
                out += `\n<h2>User</h2>\n`;
                out += `<blockquote>${msg.content.replace(/\n/g, "<br>\n")}</blockquote>\n`;
            } else if (msg.role === "assistant") {
                out += `\n<h2>Assistant</h2>\n`;
                
                for (var j = 0; j < msg.responses.length; j++) {
                    var resp = msg.responses[j];
                    var resp_html = SuttaBridge.markdown_to_html(resp.response || "");
                    var selected_indicator = resp.is_selected ? " (selected)" : "";
                    out += `<h3>${resp.model_name}${selected_indicator}</h3>\n`;
                    out += `<blockquote>${resp_html}</blockquote>\n`;
                }
            }
        }

        out += "\n</body>\n</html>";
        return out.trim().replace(/\n\n\n+/g, "\n\n");
    }

    function chat_as_markdown(): string {
        let chat_data = root.chat_export_data();

        let out = `# Chat Export\n`;

        for (var i = 0; i < chat_data.messages.length; i++) {
            var msg = chat_data.messages[i];
            
            if (msg.role === "system") {
                out += `\n## System\n\n`;
                out += `> ${msg.content.replace(/\n/g, "\n> ")}\n`;
            } else if (msg.role === "user") {
                out += `\n## User\n\n`;
                out += `> ${msg.content.replace(/\n/g, "\n> ")}\n`;
            } else if (msg.role === "assistant") {
                out += `\n## Assistant\n`;
                
                for (var j = 0; j < msg.responses.length; j++) {
                    var resp = msg.responses[j];
                    var selected_indicator = resp.is_selected ? " (selected)" : "";
                    out += `\n### ${resp.model_name}${selected_indicator}\n\n`;
                    out += `> ${resp.response.replace(/\n/g, "\n> ")}\n`;
                }
            }
        }

        return out.trim().replace(/\n\n\n+/g, "\n\n");
    }

    function chat_as_orgmode(): string {
        let chat_data = root.chat_export_data();

        let out = `* Chat Export\n`;

        for (var i = 0; i < chat_data.messages.length; i++) {
            var msg = chat_data.messages[i];
            
            if (msg.role === "system") {
                out += `\n** System\n\n`;
                out += `#+begin_quote\n${msg.content}\n#+end_quote\n`;
            } else if (msg.role === "user") {
                out += `\n** User\n\n`;
                out += `#+begin_quote\n${msg.content}\n#+end_quote\n`;
            } else if (msg.role === "assistant") {
                out += `\n** Assistant\n`;
                
                for (var j = 0; j < msg.responses.length; j++) {
                    var resp = msg.responses[j];
                    // Convert asterisk lists to dash lists
                    var resp_md = resp.response.split('\n').map(function(line) {
                        return line.replace(/^\* /, '- ');
                    }).join('\n');
                    var selected_indicator = resp.is_selected ? " (selected)" : "";
                    out += `\n*** ${resp.model_name}${selected_indicator}\n\n`;
                    out += `#+begin_src markdown\n${resp_md}\n#+end_src\n`;
                }
            }
        }

        return out.trim().replace(/\n\n\n+/g, "\n\n");
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
                        id: msg_role
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
                                    logger.log(`🔍 AssistantResponses for message ${message_item.index}: role=${message_item.role}, responses_json="${message_item.responses_json}"`);
                                    try {
                                        let data = JSON.parse(message_item.responses_json || "[]");
                                        logger.log(`📊 Parsed translations_data:`, JSON.stringify(data));
                                        return data;
                                    } catch (e) {
                                        logger.error(`❌ Error parsing responses_json for message ${message_item.index}:`, e);
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

                                        logger.log(`🚀 Send button clicked for message ${message_item.index}`);

                                        // Load enabled models
                                        root.load_available_models();
                                        logger.log(`📋 Loaded ${available_models.count} available models`);

                                        if (available_models.count === 0) {
                                            no_models_dialog.open();
                                            return;
                                        }

                                        // Create responses array for each enabled model
                                        let responses = [];
                                        for (var i = 0; i < available_models.count; i++) {
                                            var model = available_models.get(i);
                                            if (model.enabled) {
                                                logger.log(`✅ Adding enabled model: ${model.model_name}`);
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
                                                logger.log(`⏭️  Skipping disabled model: ${model.model_name}`);
                                            }
                                        }

                                        logger.log(`📊 Created ${responses.length} response entries`);

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
                                                        logger.log(`📝 Added assistant message from ${assistant_responses[selected_idx].model_name}`);
                                                    }
                                                    // Skip assistant messages that don't have completed selected responses
                                                } catch (e) {
                                                    logger.error("Failed to parse assistant responses_json:", e);
                                                }
                                            } else {
                                                // For user/system messages
                                                messages.push({
                                                    role: msg.role,
                                                    content: msg.content
                                                });
                                                logger.log(`📝 Added ${msg.role} message`);
                                            }
                                        }
                                        let messages_json = JSON.stringify(messages);
                                        logger.log(`📤 Composed message history with ${messages.length} messages`);

                                        // Send requests to all enabled models using the same message history
                                        for (var j = 0; j < responses.length; j++) {
                                            logger.log(`🎯 Sending request to ${responses[j].model_name}`);
                                            let provider_name = SuttaBridge.get_provider_for_model(responses[j].model_name);
                                            pm.prompt_request_with_messages(
                                                message_item.index, // sender message index (user message that triggered this)
                                                provider_name,
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
