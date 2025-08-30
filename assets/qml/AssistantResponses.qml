pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

import com.profoundlabs.simsapa

ColumnLayout {
    id: root

    required property bool is_dark

    required property var translations_data
    required property string paragraph_text
    required property int paragraph_index
    property string title: ""
    property int selected_tab_index: 0

    readonly property int vocab_font_point_size: 10

    property string text_color: root.is_dark ? "#F0F0F0" : "#000000"
    property string bg_color: root.is_dark ? "#23272E" : "#FAE6B2"
    property string bg_color_lighter: root.is_dark ? "#2E333D" : "#FBEDC7"
    property string bg_color_darker: root.is_dark ? "#1C2025" : "#F8DA8E"
    property string border_color: root.is_dark ? "#0a0a0a" : "#ccc"

    // Debug logging when translations_data changes
    onTranslations_dataChanged: {
        console.log(`🔄 AssistantResponses: translations_data changed for paragraph ${paragraph_index}`);
        if (translations_data) {
            console.log(`📊 New data has ${translations_data.length} translations:`);
            for (var i = 0; i < translations_data.length; i++) {
                var item = translations_data[i];
                if (item) {
                    console.log(`  [${i}] ${item.model_name}: status=${item.status}, response_length=${item.response ? item.response.length : 0}`);
                } else {
                    console.log(`  [${i}] null/undefined item`);
                }
            }
        } else {
            console.log(`❌ translations_data is null/undefined`);
        }
    }

    signal retryRequest(string model_name, string request_id)
    signal tabSelectionChanged(int tab_index, string model_name)

    function retry_request(model_name) {
        // Generate new request ID and emit signal
        var request_id = generate_request_id()
        root.retryRequest(model_name, request_id)
    }

    function generate_request_id() {
        return Date.now().toString() + "_" + Math.random().toString(36)
    }

    function is_error_response(response_text) {
        return response_text.includes("API Error:") ||
               response_text.includes("Error:") ||
               response_text.includes("Failed:")
    }

    spacing: 10

    GroupBox {
        Layout.fillWidth: true
        Layout.margins: 10
        visible: root.translations_data && root.translations_data.length > 0

        background: Rectangle {
            anchors.fill: parent
            color: root.bg_color_darker
            border.width: 1
            border.color: root.border_color
            radius: 5
        }

        ColumnLayout {
            anchors.fill: parent

            Text {
                id: assistant_title
                text: root.title
                visible: root.title.length > 0
                font.bold: true
                font.pointSize: root.vocab_font_point_size
                color: root.text_color
            }

            TabBar {
                id: tab_bar
                Layout.fillWidth: true
                currentIndex: root.selected_tab_index

                onCurrentIndexChanged: {
                    if (currentIndex !== root.selected_tab_index) {
                        root.selected_tab_index = currentIndex
                        if (root.translations_data && currentIndex >= 0 && currentIndex < root.translations_data.length) {
                            var item = root.translations_data[currentIndex]
                            if (item && item.model_name) {
                                root.tabSelectionChanged(currentIndex, item.model_name)
                            }
                        }
                    }
                }

                // Synchronize when selected_tab_index changes externally
                Connections {
                    target: root
                    function onSelected_tab_indexChanged() {
                        if (tab_bar.currentIndex !== root.selected_tab_index) {
                            tab_bar.currentIndex = root.selected_tab_index
                        }
                    }
                }

                Repeater {
                    model: root.translations_data || []

                    ResponseTabButton {
                        required property int index
                        required property var modelData

                        model_name: (modelData && modelData.model_name) ? modelData.model_name : ""
                        status: (modelData && modelData.status) ? modelData.status : "waiting"
                        retry_count: (modelData && modelData.retry_count) ? modelData.retry_count : 0

                        onRetryRequested: {
                            var name = (modelData && modelData.model_name) ? modelData.model_name : ""
                            if (name) {
                                root.retry_request(name)
                            }
                        }
                    }
                }
            }

            StackLayout {
                Layout.fillWidth: true
                Layout.preferredHeight: 200
                currentIndex: tab_bar.currentIndex

                Repeater {
                    model: root.translations_data || []

                    Item {
                        id: response_content_item

                        required property int index
                        required property var modelData

                        Layout.fillWidth: true
                        Layout.fillHeight: true

                        ScrollView {
                            anchors.fill: parent
                            ScrollBar.vertical.policy: ScrollBar.AsNeeded

                            TextArea {
                                property var data: response_content_item.modelData || {}

                                text: {
                                    console.log(`🎨 TextArea rendering for item:`, JSON.stringify(data));

                                    // Handle empty or invalid data
                                    if (!data || Object.keys(data).length === 0) {
                                        console.log(`⚠️  Empty or invalid data, showing waiting message`);
                                        return `Waiting for response from ${data.model_name} (up to 3min)...`;
                                    }

                                    if (data.status === "waiting") {
                                        console.log(`⏳ Showing waiting message for ${data.model_name}`);
                                        return `Waiting for response from ${data.model_name} (up to 3min)...`
                                    } else if (data.status === "error") {
                                        console.log(`❌ Showing error message`);
                                        var error_text = data.response || "Unknown error occurred"
                                        var retry_text = data.retry_count > 0 ? `\n\nRetrying... (${data.retry_count}x)` : ""
                                        return error_text + retry_text
                                    } else if (data.status === "completed") {
                                        console.log(`✅ Showing completed response, raw content: "${data.response}"`);
                                        var html_content = SuttaBridge.markdown_to_html(data.response || "");
                                        console.log(`🎨 Converted HTML: "${html_content}"`);
                                        return html_content;
                                    } else {
                                        console.log(`❓ Unknown status: "${data.status}", showing waiting message for ${data.model_name}`);
                                        return `Waiting for response from ${data.model_name} (up to 3min)...`;
                                    }
                                }
                                font.pointSize: root.vocab_font_point_size
                                selectByMouse: true
                                readOnly: true
                                textFormat: data.status === "completed" ? Text.RichText : Text.PlainText
                                wrapMode: TextEdit.WordWrap
                                color: root.text_color

                                background: Rectangle {
                                    color: "transparent"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
