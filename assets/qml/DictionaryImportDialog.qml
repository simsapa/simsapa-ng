pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import QtQuick.Controls
import QtQuick.Dialogs

import com.profoundlabs.simsapa

Item {
    id: root

    property int point_size: 12

    signal import_requested(string zip_path, string label, string lang)
    signal replace_requested(int existing_id, string zip_path, string label, string lang)
    signal canceled()

    DictionaryManager { id: dict_manager }

    function start() {
        file_dialog.open();
    }

    FileDialog {
        id: file_dialog
        title: "Choose StarDict .zip"
        nameFilters: ["StarDict archives (*.zip)"]
        onAccepted: {
            const url = String(selectedFile);
            // Strip "file://" scheme if present
            let path = url;
            if (path.startsWith("file://")) {
                path = path.substring(7);
            }
            details_dialog.zip_path = path;
            details_dialog.label_input_text = dict_manager.suggested_label_for_zip(path);
            details_dialog.lang_input_text = "pli";
            details_dialog.refresh_status();
            details_dialog.refresh_lang_warning();
            details_dialog.open();
        }
        onRejected: {
            root.canceled();
        }
    }

    Dialog {
        id: details_dialog
        title: "Import Dictionary"
        modal: true
        standardButtons: Dialog.Cancel | Dialog.Ok
        anchors.centerIn: Overlay.overlay
        width: 520

        property string zip_path: ""
        property string label_input_text: ""
        property string lang_input_text: "pli"
        property string label_status: "available"
        property bool lang_warning: false

        function refresh_status() {
            const v = label_input.text;
            if (v.length === 0) {
                details_dialog.label_status = "invalid";
            } else {
                details_dialog.label_status = dict_manager.label_status(v);
            }
        }

        function refresh_lang_warning() {
            const v = lang_input.text;
            details_dialog.lang_warning = v.length > 0 && !dict_manager.is_known_tokenizer_lang(v);
        }

        contentItem: ColumnLayout {
            spacing: 10

            Label {
                text: `Source: ${details_dialog.zip_path}`
                font.pointSize: root.point_size - 1
                wrapMode: Text.WrapAnywhere
                Layout.fillWidth: true
            }

            Label {
                text: "Label (will appear in search results):"
                font.pointSize: root.point_size
            }

            TextField {
                id: label_input
                Layout.fillWidth: true
                font.pointSize: root.point_size
                text: details_dialog.label_input_text
                onTextChanged: details_dialog.refresh_status()
            }

            Label {
                visible: details_dialog.label_status === "invalid"
                text: "Label must be ASCII alphanumeric, '_' or '-' only and non-empty."
                color: "red"
                font.pointSize: root.point_size - 1
                Layout.fillWidth: true
                wrapMode: Text.WordWrap
            }

            Label {
                visible: details_dialog.label_status === "taken_shipped"
                text: "This name is reserved by a built-in dictionary."
                color: "red"
                font.pointSize: root.point_size - 1
                Layout.fillWidth: true
                wrapMode: Text.WordWrap
            }

            Label {
                visible: details_dialog.label_status === "taken_user"
                text: "Another imported dictionary already uses this label. Submit will offer to replace it."
                color: "#a06800"
                font.pointSize: root.point_size - 1
                Layout.fillWidth: true
                wrapMode: Text.WordWrap
            }

            Label {
                text: "Language code:"
                font.pointSize: root.point_size
            }

            TextField {
                id: lang_input
                Layout.fillWidth: true
                font.pointSize: root.point_size
                text: details_dialog.lang_input_text
                onTextChanged: details_dialog.refresh_lang_warning()
            }

            Label {
                visible: details_dialog.lang_warning
                text: "Unknown tokenizer language. Indexing will use the default tokenizer."
                color: "#a06800"
                font.pointSize: root.point_size - 1
                Layout.fillWidth: true
                wrapMode: Text.WordWrap
            }
        }

        onAccepted: {
            const v = label_input.text;
            const lang_v = lang_input.text;
            if (details_dialog.label_status === "invalid" || details_dialog.label_status === "taken_shipped") {
                return;
            }
            if (details_dialog.label_status === "taken_user") {
                replace_confirm.label_to_replace = v;
                replace_confirm.lang_to_use = lang_v;
                replace_confirm.open();
                return;
            }
            root.import_requested(details_dialog.zip_path, v, lang_v);
        }

        onRejected: {
            root.canceled();
        }
    }

    MessageDialog {
        id: replace_confirm
        title: "Replace existing dictionary?"
        text: `An imported dictionary named "${label_to_replace}" already exists. Replace it with the new import?`
        buttons: MessageDialog.Yes | MessageDialog.No

        property string label_to_replace: ""
        property string lang_to_use: "pli"

        onButtonClicked: function(button) {
            if (button === MessageDialog.Yes) {
                // Find existing dictionary id
                const list_json = dict_manager.list_user_dictionaries();
                let existing_id = 0;
                try {
                    const arr = JSON.parse(list_json);
                    for (let i = 0; i < arr.length; i++) {
                        if (arr[i].label === replace_confirm.label_to_replace) {
                            existing_id = arr[i].id;
                            break;
                        }
                    }
                } catch (e) {
                    console.log("replace_confirm parse error:", e);
                }
                if (existing_id > 0) {
                    root.replace_requested(existing_id, details_dialog.zip_path, replace_confirm.label_to_replace, replace_confirm.lang_to_use);
                }
            } else {
                root.canceled();
            }
        }
    }
}
