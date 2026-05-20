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
    property bool is_dark: theme_helper.is_dark

    // State carried into the shared summary / error frames.
    property string op_label: ""
    property string old_label: ""
    property string new_label: ""
    property int op_count: 0
    property int op_elapsed_ms: 0
    property string op_kind: ""          // "delete" | "import" | "import_aborted" | "rename"
    property string error_message: ""

    // Import-progress state.
    property string import_stage: ""
    property int import_done: 0
    property int import_total: 0
    property bool import_indeterminate: true

    // Replace = delete-then-import. The import is held here while the
    // async delete runs, then started in onDeleteFinished.
    property bool replace_pending: false
    property string pending_zip_path: ""
    property string pending_label: ""
    property string pending_lang: ""

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

    // Ignore close while a long op is in progress. Idx 1 = deleting,
    // Idx 2 = importing, Idx 3 = renaming.
    onClosing: function(close) {
        if (views_stack.currentIndex === 1
            || views_stack.currentIndex === 2
            || views_stack.currentIndex === 3) {
            close.accepted = false;
        }
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

    // Kick off an import: reset progress state, switch to the import
    // progress frame, and call the bridge. Quick-fail routes to Idx 5.
    function start_import(zip_path: string, label: string, lang: string) {
        root.op_label = label;
        root.import_stage = "";
        root.import_done = 0;
        root.import_total = 0;
        root.import_indeterminate = true;
        views_stack.currentIndex = 2;
        const result = dict_manager.import_zip(zip_path, label, lang);
        if (result !== "ok") {
            root.error_message = "Import could not start: " + result;
            views_stack.currentIndex = 5;
        }
    }

    function elapsed_seconds_text(ms: int): string {
        const s = Math.max(0, ms) / 1000.0;
        return s.toFixed(1) + "s";
    }

    Connections {
        target: dict_manager

        function onImportProgress(stage: string, done: int, total: int) {
            root.import_stage = stage;
            root.import_done = done;
            root.import_total = total;
            // Determinate only once the backend reports a positive total for
            // the inserting-words stage; Extracting/Parsing carry total == 0.
            root.import_indeterminate = (total <= 0);
        }

        function onImportFinished(dictionary_id: int, label: string, inserted_count: int, elapsed_ms: int) {
            root.op_label = label;
            root.op_kind = "import";
            root.op_count = inserted_count;
            root.op_elapsed_ms = elapsed_ms;
            views_stack.currentIndex = 4;
            root.refresh_list();
        }

        function onImportCancelled(message: string, inserted_count: int) {
            root.op_kind = "import_aborted";
            root.op_count = inserted_count;
            root.op_elapsed_ms = 0;
            views_stack.currentIndex = 4;
            root.refresh_list();
        }

        function onImportFailed(message: string) {
            root.error_message = "Import failed: " + message;
            views_stack.currentIndex = 5;
        }

        function onDeleteFinished(dictionary_id: int, label: string, removed_count: int, elapsed_ms: int) {
            // Replace flow: the delete was the first half of a delete-then-import.
            // Don't show the delete summary — chain straight into the import.
            if (root.replace_pending) {
                root.replace_pending = false;
                root.refresh_list();
                root.start_import(root.pending_zip_path, root.pending_label, root.pending_lang);
                return;
            }
            root.op_label = label;
            root.op_kind = "delete";
            root.op_count = removed_count;
            root.op_elapsed_ms = elapsed_ms;
            views_stack.currentIndex = 4;
            root.refresh_list();
        }

        function onDeleteFailed(message: string) {
            root.replace_pending = false;
            root.error_message = "Delete failed: " + message;
            views_stack.currentIndex = 5;
            root.refresh_list();
        }

        function onRenameFinished(dictionary_id: int, old_label: string, new_label: string, elapsed_ms: int) {
            root.op_label = new_label;
            root.op_kind = "rename";
            root.op_count = 0;
            root.op_elapsed_ms = elapsed_ms;
            views_stack.currentIndex = 4;
            root.refresh_list();
        }

        function onRenameFailed(message: string) {
            root.error_message = "Rename failed: " + message;
            views_stack.currentIndex = 5;
            root.refresh_list();
        }
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
                root.op_label = confirm_delete_dialog.target_label;
                views_stack.currentIndex = 1;
                const result = dict_manager.delete_dictionary(confirm_delete_dialog.target_id);
                if (result !== "ok") {
                    root.error_message = "Delete could not start: " + result;
                    views_stack.currentIndex = 5;
                }
            }
        }
    }

    DictionaryImportDialog {
        id: import_dialog
        point_size: root.pointSize

        onImport_requested: function(zip_path, label, lang) {
            root.start_import(zip_path, label, lang);
        }

        onReplace_requested: function(existing_id, zip_path, label, lang) {
            // Replace = delete-then-import. import_user_zip refuses a label
            // collision, so the existing dictionary must be deleted first.
            // Delete is async: stash the import and resume in onDeleteFinished.
            root.replace_pending = true;
            root.pending_zip_path = zip_path;
            root.pending_label = label;
            root.pending_lang = lang;
            root.op_label = label;
            views_stack.currentIndex = 1;
            const del_result = dict_manager.delete_dictionary(existing_id);
            if (del_result !== "ok") {
                root.replace_pending = false;
                root.error_message = "Could not replace existing dictionary: " + del_result;
                views_stack.currentIndex = 5;
            }
        }

        onCanceled: {
            // No-op
        }
    }

    DictionaryEditDialog {
        id: edit_dialog
        point_size: root.pointSize

        onRename_requested: function(dictionary_id, old_label, new_label) {
            root.old_label = old_label;
            root.new_label = new_label;
            views_stack.currentIndex = 3;
            const result = dict_manager.rename_label(dictionary_id, new_label);
            if (result !== "ok") {
                root.error_message = "Rename could not start: " + result;
                views_stack.currentIndex = 5;
            }
        }
    }

    StackLayout {
        id: views_stack
        anchors.fill: parent
        anchors.topMargin: root.top_bar_margin
        currentIndex: 0

        // -------------------------------------------------------------------
        // Idx 0 — List frame (default)
        // -------------------------------------------------------------------
        Frame {
            Layout.fillWidth: true
            Layout.fillHeight: true

            ColumnLayout {
                anchors.fill: parent
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
                        onClicked: import_dialog.start()
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
                                busy: false
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

        // -------------------------------------------------------------------
        // Idx 1 — Delete progress frame
        // -------------------------------------------------------------------
        Frame {
            Layout.fillWidth: true
            Layout.fillHeight: true

            ColumnLayout {
                anchors.fill: parent

                Item {
                    Layout.fillWidth: true
                    Layout.fillHeight: true

                    ColumnLayout {
                        anchors.centerIn: parent
                        width: parent.width * 0.9
                        spacing: 16

                        Label {
                            text: `Deleting dictionary "${root.op_label}"…`
                            font.pointSize: root.largePointSize
                            font.bold: true
                            wrapMode: Text.WordWrap
                            color: palette.text
                            Layout.fillWidth: true
                            Layout.alignment: Qt.AlignCenter
                            horizontalAlignment: Text.AlignHCenter
                        }

                        Label {
                            text: "Removing entries…"
                            font.pointSize: root.pointSize
                            color: palette.mid
                            Layout.fillWidth: true
                            horizontalAlignment: Text.AlignHCenter
                        }

                        ProgressBar {
                            Layout.fillWidth: true
                            indeterminate: true
                        }
                    }
                }
            }
        }

        // -------------------------------------------------------------------
        // Idx 2 — Import progress frame
        // -------------------------------------------------------------------
        Frame {
            Layout.fillWidth: true
            Layout.fillHeight: true

            ColumnLayout {
                anchors.fill: parent

                Item {
                    Layout.fillWidth: true
                    Layout.fillHeight: true

                    ColumnLayout {
                        anchors.centerIn: parent
                        width: parent.width * 0.9
                        spacing: 16

                        Label {
                            text: `Importing "${root.op_label}"…`
                            font.pointSize: root.largePointSize
                            font.bold: true
                            wrapMode: Text.WordWrap
                            color: palette.text
                            Layout.fillWidth: true
                            horizontalAlignment: Text.AlignHCenter
                        }

                        Label {
                            text: root.import_stage
                            font.pointSize: root.pointSize
                            color: palette.text
                            visible: root.import_stage.length > 0
                            Layout.fillWidth: true
                            horizontalAlignment: Text.AlignHCenter
                        }

                        Label {
                            text: `Inserting words: ${root.import_done} / ${root.import_total}`
                            font.pointSize: root.pointSize
                            color: palette.mid
                            visible: !root.import_indeterminate && root.import_total > 0
                            Layout.fillWidth: true
                            horizontalAlignment: Text.AlignHCenter
                        }

                        ProgressBar {
                            Layout.fillWidth: true
                            indeterminate: root.import_indeterminate
                            from: 0
                            to: root.import_total > 0 ? root.import_total : 1
                            value: root.import_done
                        }
                    }
                }

                RowLayout {
                    Layout.fillWidth: true
                    Layout.margins: 20
                    Layout.bottomMargin: root.is_mobile ? 60 : 20

                    Item { Layout.fillWidth: true }

                    Button {
                        text: "Abort"
                        font.pointSize: root.pointSize
                        onClicked: dict_manager.abort_import()
                    }
                }
            }
        }

        // -------------------------------------------------------------------
        // Idx 3 — Rename progress frame (§6 placeholder)
        // -------------------------------------------------------------------
        Frame {
            Layout.fillWidth: true
            Layout.fillHeight: true

            ColumnLayout {
                anchors.fill: parent

                Item {
                    Layout.fillWidth: true
                    Layout.fillHeight: true

                    ColumnLayout {
                        anchors.centerIn: parent
                        width: parent.width * 0.9
                        spacing: 16

                        Label {
                            text: `Renaming "${root.old_label}" → "${root.new_label}"…`
                            font.pointSize: root.largePointSize
                            font.bold: true
                            wrapMode: Text.WordWrap
                            color: palette.text
                            Layout.fillWidth: true
                            horizontalAlignment: Text.AlignHCenter
                        }

                        ProgressBar {
                            Layout.fillWidth: true
                            indeterminate: true
                        }
                    }
                }
            }
        }

        // -------------------------------------------------------------------
        // Idx 4 — Completion / summary frame (shared)
        // -------------------------------------------------------------------
        Frame {
            Layout.fillWidth: true
            Layout.fillHeight: true

            ColumnLayout {
                anchors.fill: parent

                Item {
                    Layout.fillWidth: true
                    Layout.fillHeight: true

                    ColumnLayout {
                        anchors.centerIn: parent
                        width: parent.width * 0.9
                        spacing: 16

                        Label {
                            text: {
                                if (root.op_kind === "delete") return "Deleted";
                                if (root.op_kind === "import") return "Imported";
                                if (root.op_kind === "import_aborted") return "Import aborted";
                                if (root.op_kind === "rename") return "Renamed";
                                return "Completed";
                            }
                            font.pointSize: root.largePointSize
                            font.bold: true
                            color: palette.text
                            Layout.alignment: Qt.AlignCenter
                            horizontalAlignment: Text.AlignHCenter
                        }

                        Label {
                            text: {
                                if (root.op_kind === "delete") {
                                    return `Deleted "${root.op_label}" — removed ${root.op_count} entries in ${root.elapsed_seconds_text(root.op_elapsed_ms)}.\nSimsapa will now exit. Start the application again so that the fulltext search index can be updated.`;
                                }
                                if (root.op_kind === "import") {
                                    return `Imported "${root.op_label}" — ${root.op_count} entries in ${root.elapsed_seconds_text(root.op_elapsed_ms)}.\nSimsapa will now exit. Start the application again so that the dictionary can be indexed for fulltext search.`;
                                }
                                if (root.op_kind === "import_aborted") {
                                    return `Import aborted — "${root.op_label}" was partially imported (${root.op_count} entries). The remaining entries can be added by re-running the import; already-imported entries will be indexed on next start.\nSimsapa will now exit.`;
                                }
                                if (root.op_kind === "rename") {
                                    return `Dictionary renamed to "${root.op_label}".\nSimsapa will now exit. Start the application again so that the dictionary entries can be re-indexed for fulltext search.`;
                                }
                                return "";
                            }
                            wrapMode: Text.WordWrap
                            font.pointSize: root.pointSize
                            color: palette.text
                            Layout.fillWidth: true
                            horizontalAlignment: Text.AlignHCenter
                        }
                    }
                }

                RowLayout {
                    Layout.fillWidth: true
                    Layout.margins: 20
                    Layout.bottomMargin: root.is_mobile ? 60 : 20

                    Item { Layout.fillWidth: true }

                    Button {
                        text: "Quit"
                        font.pointSize: root.pointSize
                        onClicked: Qt.quit()
                    }
                }
            }
        }

        // -------------------------------------------------------------------
        // Idx 5 — Error frame (shared)
        // -------------------------------------------------------------------
        Frame {
            Layout.fillWidth: true
            Layout.fillHeight: true

            ColumnLayout {
                anchors.fill: parent

                Item {
                    Layout.fillWidth: true
                    Layout.fillHeight: true

                    ColumnLayout {
                        anchors.centerIn: parent
                        width: parent.width * 0.9
                        spacing: 16

                        Label {
                            text: "Error"
                            font.pointSize: root.largePointSize
                            font.bold: true
                            color: palette.text
                            Layout.alignment: Qt.AlignCenter
                            horizontalAlignment: Text.AlignHCenter
                        }

                        TextArea {
                            text: root.error_message
                            readOnly: true
                            wrapMode: Text.WordWrap
                            font.pointSize: root.pointSize
                            color: palette.text
                            background: null
                            Layout.fillWidth: true
                            horizontalAlignment: Text.AlignHCenter
                        }
                    }
                }

                RowLayout {
                    Layout.fillWidth: true
                    Layout.margins: 20
                    Layout.bottomMargin: root.is_mobile ? 60 : 20

                    Item { Layout.fillWidth: true }

                    Button {
                        text: "OK"
                        font.pointSize: root.pointSize
                        onClicked: {
                            root.error_message = "";
                            views_stack.currentIndex = 0;
                            root.refresh_list();
                        }
                    }
                }
            }
        }
    }
}
