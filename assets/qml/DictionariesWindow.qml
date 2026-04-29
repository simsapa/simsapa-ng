pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import QtQuick.Controls
import QtQuick.Window
import QtQuick.Dialogs

import com.profoundlabs.simsapa

ApplicationWindow {
    id: root

    title: "Dictionaries"
    width: is_mobile ? Screen.desktopAvailableWidth : 700
    height: is_mobile ? Screen.desktopAvailableHeight : Math.min(800, Screen.desktopAvailableHeight)
    visible: true
    color: palette.window
    flags: Qt.Dialog

    readonly property bool is_mobile: Qt.platform.os === "android" || Qt.platform.os === "ios"
    readonly property bool is_desktop: !root.is_mobile

    readonly property int pointSize: is_mobile ? 16 : 12
    readonly property int largePointSize: pointSize + 5
    property int top_bar_margin: is_mobile ? 24 : 0

    property var user_dictionaries: []
    property bool busy: false
    property string progress_stage: ""
    property real progress_value: 0
    property bool progress_indeterminate: false
    property bool is_dark: theme_helper.is_dark

    ThemeHelper {
        id: theme_helper
        target_window: root
    }

    DictionaryManager { id: dict_manager }

    Component.onCompleted: {
        theme_helper.apply();
        root.top_bar_margin = root.is_mobile ? SuttaBridge.get_mobile_top_bar_margin() : 0;
        root.refresh_list();
    }

    function refresh_list() {
        const json_str = dict_manager.list_user_dictionaries();
        try {
            root.user_dictionaries = JSON.parse(json_str);
        } catch (e) {
            console.log("DictionariesWindow.refresh_list parse error:", e);
            root.user_dictionaries = [];
        }
    }

    Connections {
        target: dict_manager

        function onImportProgress(stage: string, done: int, total: int) {
            root.progress_stage = stage;
            if (total > 0) {
                root.progress_indeterminate = false;
                root.progress_value = done / total;
            } else {
                root.progress_indeterminate = true;
                root.progress_value = 0;
            }
        }

        function onImportFinished(dictionary_id: int, label: string) {
            root.busy = false;
            root.progress_stage = "Import completed";
            root.progress_value = 1.0;
            root.progress_indeterminate = false;
            root.refresh_list();
            restart_dialog.dialog_text = `Dictionary "${label}" was imported. Please close and re-open Simsapa so it can be indexed for searching.`;
            restart_dialog.open();
        }

        function onImportFailed(message: string) {
            root.busy = false;
            root.progress_stage = "";
            root.progress_value = 0;
            root.progress_indeterminate = false;
            error_dialog.dialog_text = `Import failed: ${message}`;
            error_dialog.open();
        }
    }

    MessageDialog {
        id: restart_dialog
        title: "Restart required"
        property string dialog_text: ""
        text: dialog_text
        buttons: MessageDialog.Ok
    }

    MessageDialog {
        id: error_dialog
        title: "Error"
        property string dialog_text: ""
        text: dialog_text
        buttons: MessageDialog.Ok
    }

    MessageDialog {
        id: confirm_delete_dialog
        title: "Delete dictionary?"
        property int target_id: 0
        property string target_label: ""
        text: `Delete dictionary "${target_label}" and all its entries? This cannot be undone.`
        buttons: MessageDialog.Yes | MessageDialog.No

        onButtonClicked: function(button) {
            if (button === MessageDialog.Yes) {
                const result = dict_manager.delete_dictionary(confirm_delete_dialog.target_id);
                if (result === "ok") {
                    root.refresh_list();
                    restart_dialog.dialog_text = `Dictionary "${confirm_delete_dialog.target_label}" was removed. Please close and re-open Simsapa so the search index is updated.`;
                    restart_dialog.open();
                } else {
                    error_dialog.dialog_text = `Delete failed: ${result}`;
                    error_dialog.open();
                }
            }
        }
    }

    DictionaryImportDialog {
        id: import_dialog
        point_size: root.pointSize

        onImport_requested: function(zip_path, label, lang) {
            root.busy = true;
            root.progress_stage = "Starting import...";
            root.progress_value = 0;
            root.progress_indeterminate = true;
            const result = dict_manager.import_zip(zip_path, label, lang);
            if (result !== "ok") {
                root.busy = false;
                root.progress_stage = "";
                root.progress_indeterminate = false;
                error_dialog.dialog_text = `Import could not start: ${result}`;
                error_dialog.open();
            }
        }

        onReplace_requested: function(existing_id, zip_path, label, lang) {
            const del_result = dict_manager.delete_dictionary(existing_id);
            if (del_result !== "ok") {
                error_dialog.dialog_text = `Could not replace existing dictionary: ${del_result}`;
                error_dialog.open();
                return;
            }
            root.refresh_list();
            // Now start the new import
            root.busy = true;
            root.progress_stage = "Starting import...";
            root.progress_value = 0;
            root.progress_indeterminate = true;
            const result = dict_manager.import_zip(zip_path, label, lang);
            if (result !== "ok") {
                root.busy = false;
                root.progress_stage = "";
                root.progress_indeterminate = false;
                error_dialog.dialog_text = `Import could not start: ${result}`;
                error_dialog.open();
            }
        }

        onCanceled: {
            // No-op
        }
    }

    DictionaryEditDialog {
        id: edit_dialog
        point_size: root.pointSize

        onRenamed: function(dictionary_id, new_label) {
            root.refresh_list();
            restart_dialog.dialog_text = `Dictionary was renamed to "${new_label}". Please close and re-open Simsapa so its entries are re-indexed.`;
            restart_dialog.open();
        }

        onFailed: function(message) {
            error_dialog.dialog_text = `Rename failed: ${message}`;
            error_dialog.open();
        }
    }

    ColumnLayout {
        anchors.fill: parent
        anchors.topMargin: root.top_bar_margin
        anchors.margins: 12
        spacing: 12

        RowLayout {
            Layout.fillWidth: true

            Label {
                text: "Imported Dictionaries"
                font.pointSize: root.largePointSize
                font.bold: true
                Layout.fillWidth: true
            }

            Button {
                text: "Import StarDict..."
                enabled: !root.busy
                onClicked: import_dialog.start()
            }
        }

        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: progress_layout.implicitHeight + 16
            visible: root.busy || root.progress_stage.length > 0
            color: palette.alternateBase
            border.color: palette.mid
            radius: 4

            ColumnLayout {
                id: progress_layout
                anchors.fill: parent
                anchors.margins: 8
                spacing: 4

                Label {
                    text: root.progress_stage
                    font.pointSize: root.pointSize - 1
                    Layout.fillWidth: true
                }

                ProgressBar {
                    Layout.fillWidth: true
                    indeterminate: root.progress_indeterminate
                    from: 0
                    to: 1
                    value: root.progress_value
                }
            }
        }

        ScrollView {
            id: scroll_view
            Layout.fillWidth: true
            Layout.fillHeight: true
            contentWidth: availableWidth
            clip: true

            ColumnLayout {
                width: scroll_view.availableWidth
                spacing: 6

                Label {
                    visible: root.user_dictionaries.length === 0
                    text: "No imported dictionaries yet."
                    font.pointSize: root.pointSize
                    color: palette.mid
                    Layout.alignment: Qt.AlignHCenter
                    Layout.topMargin: 30
                }

                Repeater {
                    model: root.user_dictionaries

                    delegate: DictionaryListItem {
                        required property var modelData

                        dictionary_id: modelData.id
                        title_text: modelData.title
                        label_text: modelData.label
                        language_text: modelData.language || ""
                        entry_count: modelData.entry_count
                        busy: root.busy
                        point_size: root.pointSize

                        onEdit_clicked: {
                            edit_dialog.dictionary_id = modelData.id;
                            edit_dialog.original_label = modelData.label;
                            edit_dialog.open();
                        }

                        onDelete_clicked: {
                            confirm_delete_dialog.target_id = modelData.id;
                            confirm_delete_dialog.target_label = modelData.label;
                            confirm_delete_dialog.open();
                        }
                    }
                }
            }
        }

        RowLayout {
            Layout.fillWidth: true

            Item { Layout.fillWidth: true }

            Button {
                text: "Close"
                onClicked: root.close()
            }
        }
    }
}
