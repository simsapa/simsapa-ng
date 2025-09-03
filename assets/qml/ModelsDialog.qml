pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import QtQuick.Controls
import QtQuick.Window

import com.profoundlabs.simsapa

ApplicationWindow {
    id: root

    title: "AI Models"
    width: is_mobile ? Screen.desktopAvailableWidth : 800
    height: is_mobile ? Screen.desktopAvailableHeight : 600
    visible: false
    /* visible: true // for qml preview */
    color: palette.window
    flags: Qt.Dialog

    readonly property bool is_mobile: Qt.platform.os === "android" || Qt.platform.os === "ios"
    readonly property bool is_desktop: !root.is_mobile

    readonly property int pointSize: is_mobile? 14 : 12

    property var current_providers: []
    property string selected_provider: ""
    property int selected_provider_index: -1

    Logger { id: logger }

    function load_providers() {
        let providers_json = SuttaBridge.get_providers_json();
        try {
            root.current_providers = JSON.parse(providers_json);

            provider_list_model.clear();
            for (let i = 0; i < root.current_providers.length; i++) {
                let provider = root.current_providers[i];
                provider_list_model.append({
                    provider_name: provider.name,
                    provider_enabled: provider.enabled,
                    provider_index: i
                });
            }

            // Select first provider by default
            if (root.current_providers.length > 0) {
                root.selected_provider = root.current_providers[0].name;
                root.selected_provider_index = 0;
                provider_list_view.currentIndex = 0;
                load_provider_details();
            }
        } catch (e) {
            logger.error("Failed to parse providers JSON:", e);
        }
    }

    function load_provider_details() {
        if (root.selected_provider_index >= 0 && root.selected_provider_index < root.current_providers.length) {
            let provider = root.current_providers[root.selected_provider_index];

            // Load API key
            api_key_input.text = SuttaBridge.get_provider_api_key(provider.name);

            // Load models
            model_list_model.clear();
            for (let i = 0; i < provider.models.length; i++) {
                let model = provider.models[i];
                model_list_model.append({
                    model_name: model.model_name,
                    model_enabled: model.enabled,
                    model_removable: model.removable,
                    model_index: i
                });
            }
        }
    }

    function save_provider_api_key() {
        if (root.selected_provider_index >= 0) {
            let provider = root.current_providers[root.selected_provider_index];
            SuttaBridge.set_provider_api_key(provider.name, api_key_input.text);
        }
    }

    function toggle_provider_enabled(provider_index, enabled) {
        root.current_providers[provider_index].enabled = enabled;
        SuttaBridge.set_provider_enabled(root.current_providers[provider_index].name, enabled);

        // Update the list model
        provider_list_model.setProperty(provider_index, "provider_enabled", enabled);
    }

    function add_model() {
        let model_name = new_model_input.text.trim();

        if (model_name.length === 0) {
            return;
        }

        if (root.selected_provider_index >= 0) {
            let provider = root.current_providers[root.selected_provider_index];

            // Check if model already exists
            for (let i = 0; i < provider.models.length; i++) {
                if (provider.models[i].model_name === model_name) {
                    return;
                }
            }

            SuttaBridge.add_provider_model(provider.name, model_name);

            // Reload provider data to get updated model list
            load_providers();
            provider_list_view.currentIndex = root.selected_provider_index;
            load_provider_details();

            new_model_input.text = "";
        }
    }

    function toggle_model_enabled(model_index, enabled) {
        if (root.selected_provider_index >= 0) {
            let provider = root.current_providers[root.selected_provider_index];
            provider.models[model_index].enabled = enabled;

            // Save via providers JSON
            let providers_json = JSON.stringify(root.current_providers);
            SuttaBridge.set_providers_json(providers_json);

            // Update the model list display
            model_list_model.setProperty(model_index, "model_enabled", enabled);
        }
    }

    function remove_model_with_confirmation(model_index) {
        if (root.selected_provider_index >= 0) {
            let provider = root.current_providers[root.selected_provider_index];
            let model = provider.models[model_index];

            if (!model.removable) {
                return; // Can't remove non-removable models
            }

            confirmation_dialog.model_name = model.model_name;
            confirmation_dialog.model_index = model_index;
            confirmation_dialog.open();
        }
    }

    function remove_model(model_index) {
        if (root.selected_provider_index >= 0) {
            let provider = root.current_providers[root.selected_provider_index];
            let model = provider.models[model_index];

            SuttaBridge.remove_provider_model(provider.name, model.model_name);

            // Reload provider data to get updated model list
            load_providers();
            provider_list_view.currentIndex = root.selected_provider_index;
            load_provider_details();
        }
    }

    Component.onCompleted: {
        load_providers();
    }

    onVisibilityChanged: {
        // When the dialog is closed, reset the state of key visibility.
        if (!root.visible) {
            show_key.checked = false;
        }
    }

    ListModel { id: provider_list_model }
    ListModel { id: model_list_model }

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
                    source: "icons/32x32/fa_gear-solid.png"
                    Layout.preferredWidth: 32
                    Layout.preferredHeight: 32
                }
                Label {
                    text: "AI Models"
                    font.bold: true
                    font.pointSize: root.pointSize + 3
                }
            }

            SplitView {
                Layout.fillWidth: true
                Layout.fillHeight: true
                orientation: Qt.Horizontal

                // Left side - Provider list
                Item {
                    SplitView.preferredWidth: 250
                    SplitView.minimumWidth: 200

                    ColumnLayout {
                        anchors.fill: parent
                        anchors.margins: 5

                        Label {
                            text: "AI Providers:"
                            font.bold: true
                            font.pointSize: root.pointSize
                        }

                        ListView {
                            id: provider_list_view
                            Layout.fillWidth: true
                            Layout.fillHeight: true
                            model: provider_list_model
                            clip: true

                            delegate: ItemDelegate {
                                id: provider_item
                                required property int index
                                required property string provider_name
                                required property bool provider_enabled
                                required property int provider_index

                                width: provider_list_view.width
                                height: 60

                                highlighted: provider_list_view.currentIndex === index

                                background: Rectangle {
                                    color: provider_item.highlighted ? palette.highlight :
                                           (provider_item.hovered ? palette.alternateBase : palette.base)
                                    opacity: provider_item.provider_enabled ? 1.0 : 0.6
                                    border.width: 1
                                    border.color: palette.mid
                                }

                                ColumnLayout {
                                    anchors.left: parent.left
                                    anchors.right: parent.right
                                    anchors.verticalCenter: parent.verticalCenter
                                    anchors.leftMargin: 10
                                    anchors.rightMargin: 10
                                    spacing: 2

                                    RowLayout {
                                        Layout.fillWidth: true
                                        spacing: 5

                                        Text {
                                            text: provider_item.provider_name
                                            font.pointSize: root.pointSize
                                            font.bold: true
                                            color: provider_item.highlighted ? palette.highlightedText : palette.text
                                            elide: Text.ElideRight
                                            Layout.fillWidth: true
                                        }

                                        Switch {
                                            id: provider_switch
                                            checked: provider_item.provider_enabled
                                            onToggled: {
                                                root.toggle_provider_enabled(provider_item.provider_index, provider_switch.checked);
                                            }
                                        }
                                    }

                                    Text {
                                        text: provider_item.provider_enabled ? "Enabled" : "Disabled"
                                        font.pointSize: root.pointSize - 2
                                        color: provider_item.highlighted ? palette.highlightedText : palette.windowText
                                        opacity: 0.7
                                    }
                                }

                                onClicked: {
                                    provider_list_view.currentIndex = index;
                                    root.selected_provider = provider_item.provider_name;
                                    root.selected_provider_index = provider_item.provider_index;
                                    root.load_provider_details();
                                }
                            }
                        }
                    }
                }

                // Right side - Provider details
                Item {
                    SplitView.fillWidth: true

                    ColumnLayout {
                        anchors.fill: parent
                        anchors.margins: 5
                        spacing: 15

                        Label {
                            text: root.selected_provider ? root.selected_provider + " Settings" : "Select a provider"
                            font.bold: true
                            font.pointSize: root.pointSize + 1
                        }

                        // API Key Section
                        GroupBox {
                            title: "API Key"
                            Layout.fillWidth: true

                            background: Rectangle {
                                anchors.fill: parent
                                border.width: 0
                                color: palette.window
                            }

                            ColumnLayout {
                                anchors.fill: parent
                                spacing: 10

                                RowLayout {
                                    Layout.fillWidth: true
                                    spacing: 5

                                    TextField {
                                        id: api_key_input
                                        Layout.fillWidth: true
                                        placeholderText: "Enter API key..."
                                        echoMode: show_key.checked ? TextInput.Normal : TextInput.Password
                                        font.pointSize: root.pointSize
                                        enabled: root.selected_provider !== ""
                                        onTextChanged: {
                                            if (root.visible && root.selected_provider !== "") {
                                                root.save_provider_api_key();
                                            }
                                        }
                                    }

                                    Button {
                                        id: show_key
                                        icon.source: show_key.checked ? "icons/32x32/mdi--eye-off-outline.png" : "icons/32x32/mdi--eye-outline.png"
                                        checkable: true
                                        Layout.preferredHeight: api_key_input.height
                                        Layout.preferredWidth: api_key_input.height
                                        enabled: root.selected_provider !== ""
                                    }
                                }
                            }
                        }

                        // Models Section
                        GroupBox {
                            title: "Models"
                            Layout.fillWidth: true
                            Layout.fillHeight: true

                            background: Rectangle {
                                anchors.fill: parent
                                border.width: 0
                                color: palette.window
                            }

                            ColumnLayout {
                                anchors.fill: parent
                                spacing: 10

                                // Add Model Section
                                ColumnLayout {
                                    Layout.fillWidth: true
                                    spacing: 5

                                    Label {
                                        text: "Add New Model:"
                                        font.pointSize: root.pointSize
                                        font.bold: true
                                    }

                                    TextField {
                                        id: new_model_input
                                        Layout.fillWidth: true
                                        placeholderText: "Enter model name..."
                                        font.pointSize: root.pointSize
                                        enabled: root.selected_provider !== ""
                                        onAccepted: root.add_model()
                                    }



                                    Button {
                                        text: "Add Model"
                                        enabled: root.selected_provider !== "" && new_model_input.text.trim().length > 0
                                        onClicked: root.add_model()
                                    }
                                }

                                // Model List
                                ScrollView {
                                    Layout.fillWidth: true
                                    Layout.fillHeight: true
                                    clip: true

                                    ListView {
                                        id: model_list_view
                                        model: model_list_model
                                        spacing: 2

                                        delegate: ItemDelegate {
                                            id: model_item
                                            required property int index
                                            required property string model_name
                                            required property bool model_enabled
                                            required property bool model_removable
                                            required property int model_index

                                            width: model_list_view.width
                                            height: 50

                                            background: Rectangle {
                                                color: {
                                                    if (!model_enabled_checkbox.checked) {
                                                        return Qt.darker(palette.base, 1.1);
                                                    }
                                                    return model_item.hovered ? palette.alternateBase : palette.base;
                                                }
                                                border.width: 1
                                                border.color: palette.mid
                                            }

                                            onClicked: {
                                                model_enabled_checkbox.checked = !model_enabled_checkbox.checked;
                                                root.toggle_model_enabled(model_item.model_index, model_enabled_checkbox.checked);
                                            }

                                            RowLayout {
                                                anchors.left: parent.left
                                                anchors.right: parent.right
                                                anchors.verticalCenter: parent.verticalCenter
                                                anchors.leftMargin: 10
                                                anchors.rightMargin: 10
                                                spacing: 10

                                                CheckBox {
                                                    id: model_enabled_checkbox
                                                    checked: model_item.model_enabled
                                                    onToggled: {
                                                        root.toggle_model_enabled(model_item.model_index, model_enabled_checkbox.checked);
                                                    }
                                                }

                                                Text {
                                                    text: model_item.model_name
                                                    font.pointSize: root.pointSize
                                                    color: palette.text
                                                    elide: Text.ElideRight
                                                    Layout.fillWidth: true
                                                }

                                                Button {
                                                    id: remove_btn
                                                    Layout.preferredHeight: remove_btn.height
                                                    Layout.preferredWidth: remove_btn.height
                                                    icon.source: "icons/32x32/ion--trash-outline.png"
                                                    font.pointSize: root.pointSize - 1
                                                    enabled: model_item.model_removable
                                                    visible: model_item.model_removable
                                                    onClicked: root.remove_model_with_confirmation(model_item.model_index)
                                                    ToolTip.visible: hovered
                                                    ToolTip.text: "Remove this model"
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

    // Confirmation Dialog
    Dialog {
        id: confirmation_dialog
        title: "Confirm Removal"
        anchors.centerIn: parent
        modal: true

        property string model_name: ""
        property int model_index: -1

        ColumnLayout {
            spacing: 20

            Text {
                text: "Are you sure you want to remove the model '" + confirmation_dialog.model_name + "'?"
                font.pointSize: root.pointSize
                wrapMode: Text.WordWrap
                Layout.preferredWidth: 300
            }

            RowLayout {
                spacing: 10
                Layout.alignment: Qt.AlignRight

                Button {
                    text: "Cancel"
                    onClicked: confirmation_dialog.close()
                }

                Button {
                    text: "Remove"
                    icon.source: "icons/32x32/ion--trash-outline.png"
                    onClicked: {
                        root.remove_model(confirmation_dialog.model_index);
                        confirmation_dialog.close();
                    }
                }
            }
        }
    }
}
