pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import QtQuick.Controls
import QtQuick.Window

import com.profoundlabs.simsapa

ApplicationWindow {
    id: root
    title: "AI Models"
    width: is_mobile ? Screen.desktopAvailableWidth : 600
    height: is_mobile ? Screen.desktopAvailableHeight : 500
    visible: false
    /* visible: true // for qml preview */
    color: palette.window
    flags: Qt.Dialog

    readonly property bool is_mobile: Qt.platform.os === "android" || Qt.platform.os === "ios"
    readonly property bool is_desktop: !root.is_mobile

    readonly property int pointSize: is_mobile? 14 : 12

    property var current_models: []
    property bool has_unsaved_changes: false

    function load_models() {
        let models_json = SuttaBridge.get_models_json();
        try {
            root.current_models = JSON.parse(models_json);
            
            models_list_model.clear();
            for (let i = 0; i < root.current_models.length; i++) {
                let model = root.current_models[i];
                models_list_model.append({
                    model_name: model.model_name,
                    enabled: model.enabled,
                    list_index: i
                });
            }
        } catch (e) {
            console.error("Failed to parse models JSON:", e);
        }
        root.has_unsaved_changes = false;
    }

    function save_models() {
        let models_json = JSON.stringify(root.current_models);
        SuttaBridge.set_models_json(models_json);
        root.has_unsaved_changes = false;
        root.close();
    }

    function refresh_list_indices() {
        models_list_model.clear();
        for (let i = 0; i < root.current_models.length; i++) {
            let model = root.current_models[i];
            models_list_model.append({
                model_name: model.model_name,
                enabled: model.enabled,
                list_index: i
            });
        }
    }

    function add_model() {
        let model_name = new_model_input.text.trim();
        if (model_name.length === 0) {
            return;
        }

        for (let i = 0; i < root.current_models.length; i++) {
            if (root.current_models[i].model_name === model_name) {
                return;
            }
        }

        let new_model = {
            model_name: model_name,
            enabled: true
        };

        root.current_models.unshift(new_model);
        models_list_model.insert(0, {
            model_name: model_name,
            enabled: true,
            list_index: 0
        });

        refresh_list_indices();
        new_model_input.text = "";
        root.has_unsaved_changes = true;
    }

    function remove_model(list_index) {
        root.current_models.splice(list_index, 1);
        refresh_list_indices();
        root.has_unsaved_changes = true;
    }

    function toggle_model_enabled(list_index, enabled) {
        root.current_models[list_index].enabled = enabled;
        root.has_unsaved_changes = true;
    }

    Component.onCompleted: {
        load_models();
    }

    ListModel { id: models_list_model }

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

            ColumnLayout {
                spacing: 10
                Layout.fillWidth: true
                Layout.fillHeight: true

                RowLayout {
                    spacing: 10
                    Layout.fillWidth: true

                    TextField {
                        id: new_model_input
                        Layout.fillWidth: true
                        placeholderText: "Enter model name..."
                        font.pointSize: root.pointSize
                        onAccepted: root.add_model()
                    }

                    Button {
                        text: "Add Model"
                        enabled: new_model_input.text.trim().length > 0
                        onClicked: root.add_model()
                    }
                }

                Text {
                    text: "See available models at <a href='https://openrouter.ai/models'>https://openrouter.ai/models</a>"
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

                ScrollView {
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    clip: true

                    ListView {
                        id: models_list_view
                        model: models_list_model
                        spacing: 2

                        delegate: ItemDelegate {
                            id: item_delegate
                            required property int index
                            required property string model_name
                            required property bool enabled
                            required property int list_index

                            width: models_list_view.width
                            height: 50

                            background: Rectangle {
                                color: {
                                    if (!enabled_checkbox.checked) {
                                        return Qt.darker(palette.base, 1.1);
                                    }
                                    return item_delegate.hovered ? palette.alternateBase : palette.base;
                                }
                                border.width: 1
                                border.color: palette.mid
                            }

                            onClicked: {
                                enabled_checkbox.checked = !enabled_checkbox.checked;
                                root.toggle_model_enabled(item_delegate.list_index, enabled_checkbox.checked);
                            }

                            RowLayout {
                                anchors.left: parent.left
                                anchors.right: parent.right
                                anchors.verticalCenter: parent.verticalCenter
                                anchors.leftMargin: 10
                                anchors.rightMargin: 10
                                spacing: 10

                                CheckBox {
                                    id: enabled_checkbox
                                    checked: item_delegate.enabled
                                    onToggled: {
                                        root.toggle_model_enabled(item_delegate.list_index, enabled_checkbox.checked);
                                    }
                                }

                                Text {
                                    text: item_delegate.model_name
                                    font.pointSize: root.pointSize
                                    color: palette.text
                                    elide: Text.ElideRight
                                    Layout.fillWidth: true
                                }

                                Button {
                                    text: "Remove"
                                    font.pointSize: root.pointSize - 1
                                    onClicked: root.remove_model(item_delegate.list_index)
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
                    onClicked: root.save_models()
                }
            }
        }
    }
}
