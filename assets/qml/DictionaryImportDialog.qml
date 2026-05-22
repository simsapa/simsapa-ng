pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import QtQuick.Controls
import QtQuick.Window
import QtQuick.Dialogs

import com.profoundlabs.simsapa

// StarDict import dialog. Standalone ApplicationWindow with a
// three-frame StackLayout:
//   Idx 0 — Source selection: four radio options + OK/Cancel.
//   Idx 1 — Scanning: indeterminate progress while the discovery probe runs.
//   Idx 2 — Checklist: one DictionaryImportRow per discovered dictionary,
//            Select-All / Clear-Selection, and OK/Cancel.
// On OK it emits `import_batch_requested(items_json)` (ordered list of checked
// rows) and hides; the actual import runs in DictionariesWindow's frames.
ApplicationWindow {
    id: root

    Logger { id: logger }

    title: "Import StarDict Dictionaries"
    width: is_mobile ? Screen.desktopAvailableWidth : 640
    height: is_mobile ? Screen.desktopAvailableHeight : Math.min(700, Screen.desktopAvailableHeight)
    visible: false
    color: palette.window
    flags: Qt.Dialog
    modality: Qt.ApplicationModal

    readonly property bool is_mobile: Qt.platform.os === "android" || Qt.platform.os === "ios"
    readonly property bool is_desktop: !root.is_mobile

    readonly property int pointSize: is_mobile ? 16 : 12
    readonly property int largePointSize: pointSize + 5
    property int top_bar_margin: is_mobile ? 24 : 0

    // On a narrow window the checklist header buttons would crowd the "Found N"
    // title, so it collapses to two rows.
    readonly property bool narrow_layout: width < 480

    // Kept named `point_size` for parity with the previous component API; the
    // parent (DictionariesWindow) sets it, but internal sizing uses pointSize.
    property int point_size: 12

    // Emitted on OK with the ordered list of checked rows, each
    // `{path, kind: "zip"|"dir", label, lang}`. Replaces the old
    // import_requested / replace_requested pair.
    signal import_batch_requested(string items_json)
    signal canceled()

    // Parsed candidate metadata from `scan_source` (the Repeater model).
    property var scanned_items: []
    // OK-enablement, recomputed by `recompute()` across all checked rows.
    property bool can_import: false
    // Shown on the scanning/checklist frames when discovery yields nothing or
    // fails; surfaced as a message on Idx 0.
    property string scan_message: ""

    ThemeHelper {
        id: theme_helper
        target_window: root
    }

    DictionaryManager { id: dict_manager }

    Component.onCompleted: {
        theme_helper.apply();
        root.top_bar_margin = root.is_mobile ? SuttaBridge.get_mobile_top_bar_margin() : 0;
    }

    // Public entry point: reset to the source-selection frame and show.
    function start() {
        root.scanned_items = [];
        root.can_import = false;
        root.scan_message = "";
        frames.currentIndex = 0;
        root.show();
        root.raise();
        root.requestActivate();
    }

    function strip_file_scheme(url: string): string {
        let path = String(url);
        if (path.startsWith("file://")) {
            path = path.substring(7);
        }
        return path;
    }

    // Begin discovery for the chosen source kind + path: switch to the
    // scanning frame and call the worker-threaded probe.
    function begin_scan(kind: string, path: string) {
        root.scan_message = "";
        frames.currentIndex = 1;
        const result = dict_manager.scan_source(kind, path);
        if (result !== "ok") {
            root.scan_message = "Could not scan source: " + result;
            frames.currentIndex = 0;
        }
    }

    // Re-aggregate intra-batch duplicate labels and OK-enablement across rows.
    // Called on every row `changed()` (checkbox, label edit, async status).
    function recompute() {
        const counts = {};
        for (let i = 0; i < checklist_repeater.count; i++) {
            const it = checklist_repeater.itemAt(i);
            if (it && it.checked) {
                counts[it.label] = (counts[it.label] || 0) + 1;
            }
        }
        let any_checked = false;
        let any_blocking = false;
        for (let i = 0; i < checklist_repeater.count; i++) {
            const it = checklist_repeater.itemAt(i);
            if (!it) continue;
            it.duplicate_in_batch = it.checked && counts[it.label] > 1;
            if (it.checked) {
                any_checked = true;
                if (it.blocking) any_blocking = true;
            }
        }
        root.can_import = any_checked && !any_blocking;
    }

    Connections {
        target: dict_manager

        function onScanFinished(items_json: string) {
            let arr = [];
            try {
                arr = JSON.parse(items_json);
            } catch (e) {
                logger.error("DictionaryImportDialog scanFinished parse error: " + e);
                arr = [];
            }
            if (!arr || arr.length === 0) {
                root.scan_message = "No StarDict dictionaries were found in the chosen source.";
                frames.currentIndex = 0;
                return;
            }
            root.scanned_items = arr;
            frames.currentIndex = 2;
            // Rows recompute their own status on completion; aggregate after.
            Qt.callLater(root.recompute);
        }

        function onScanFailed(message: string) {
            root.scan_message = "Scan failed: " + message;
            frames.currentIndex = 0;
        }
    }

    FileDialog {
        id: file_dialog
        title: "Choose StarDict .zip"
        nameFilters: ["StarDict archives (*.zip)"]
        onAccepted: root.begin_scan("single_zip", root.strip_file_scheme(selectedFile))
        onRejected: root.canceled()
    }

    // Shared folder picker for options 2–4; `pending_kind` selects which scan
    // kind to run when a folder is chosen.
    FolderDialog {
        id: folder_dialog
        title: "Choose folder"
        property string pending_kind: "single_dir"
        onAccepted: root.begin_scan(folder_dialog.pending_kind, root.strip_file_scheme(selectedFolder))
        onRejected: root.canceled()
    }

    StackLayout {
        id: frames
        anchors.fill: parent
        anchors.topMargin: root.top_bar_margin
        currentIndex: 0

        // -------------------------------------------------------------------
        // Idx 0 — Source selection
        // -------------------------------------------------------------------
        Frame {
            Layout.fillWidth: true
            Layout.fillHeight: true

            ColumnLayout {
                anchors.fill: parent
                anchors.margins: 16
                spacing: 14

                Label {
                    text: "Import StarDict"
                    font.pointSize: root.largePointSize
                    font.bold: true
                    Layout.fillWidth: true
                }

                Label {
                    visible: root.scan_message.length > 0
                    text: root.scan_message
                    color: "#a06800"
                    font.pointSize: root.pointSize
                    wrapMode: Text.WordWrap
                    Layout.fillWidth: true
                }

                Label {
                    text: "Choose what to import:"
                    font.pointSize: root.pointSize
                    Layout.fillWidth: true
                }

                ButtonGroup { id: source_group }

                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: 8

                    RadioButton {
                        id: opt_single_zip
                        text: "A single dictionary .zip archive"
                        font.pointSize: root.pointSize
                        checked: true
                        ButtonGroup.group: source_group
                        property string kind: "single_zip"
                        Layout.fillWidth: true
                        contentItem: Text {
                            text: opt_single_zip.text
                            font: opt_single_zip.font
                            color: opt_single_zip.palette.windowText
                            verticalAlignment: Text.AlignVCenter
                            wrapMode: Text.WordWrap
                            leftPadding: opt_single_zip.indicator.width + opt_single_zip.spacing
                        }
                    }

                    RadioButton {
                        id: opt_single_dir
                        text: "A single folder of an extracted dictionary"
                        font.pointSize: root.pointSize
                        ButtonGroup.group: source_group
                        property string kind: "single_dir"
                        Layout.fillWidth: true
                        contentItem: Text {
                            text: opt_single_dir.text
                            font: opt_single_dir.font
                            color: opt_single_dir.palette.windowText
                            verticalAlignment: Text.AlignVCenter
                            wrapMode: Text.WordWrap
                            leftPadding: opt_single_dir.indicator.width + opt_single_dir.spacing
                        }
                    }

                    RadioButton {
                        id: opt_zip_folder
                        text: "A folder of multiple .zip archives"
                        font.pointSize: root.pointSize
                        ButtonGroup.group: source_group
                        property string kind: "zip_folder"
                        Layout.fillWidth: true
                        contentItem: Text {
                            text: opt_zip_folder.text
                            font: opt_zip_folder.font
                            color: opt_zip_folder.palette.windowText
                            verticalAlignment: Text.AlignVCenter
                            wrapMode: Text.WordWrap
                            leftPadding: opt_zip_folder.indicator.width + opt_zip_folder.spacing
                        }
                    }

                    RadioButton {
                        id: opt_dir_folder
                        text: "A folder of multiple extracted dictionary folders"
                        font.pointSize: root.pointSize
                        ButtonGroup.group: source_group
                        property string kind: "dir_folder"
                        Layout.fillWidth: true
                        contentItem: Text {
                            text: opt_dir_folder.text
                            font: opt_dir_folder.font
                            color: opt_dir_folder.palette.windowText
                            verticalAlignment: Text.AlignVCenter
                            wrapMode: Text.WordWrap
                            leftPadding: opt_dir_folder.indicator.width + opt_dir_folder.spacing
                        }
                    }
                }

                Item { Layout.fillHeight: true }

                RowLayout {
                    Layout.fillWidth: true

                    Item { Layout.fillWidth: true }

                    Button {
                        text: "Cancel"
                        font.pointSize: root.pointSize
                        onClicked: {
                            root.canceled();
                            root.hide();
                        }
                    }

                    Button {
                        text: "OK"
                        font.pointSize: root.pointSize
                        onClicked: {
                            const kind = source_group.checkedButton.kind;
                            if (kind === "single_zip") {
                                file_dialog.open();
                            } else {
                                folder_dialog.pending_kind = kind;
                                folder_dialog.open();
                            }
                        }
                    }
                }
            }
        }

        // -------------------------------------------------------------------
        // Idx 1 — Scanning
        // -------------------------------------------------------------------
        Frame {
            Layout.fillWidth: true
            Layout.fillHeight: true

            Item {
                anchors.fill: parent

                ColumnLayout {
                    anchors.centerIn: parent
                    width: parent.width * 0.9
                    spacing: 16

                    Label {
                        text: "Scanning…"
                        font.pointSize: root.largePointSize
                        font.bold: true
                        color: palette.text
                        Layout.alignment: Qt.AlignCenter
                        horizontalAlignment: Text.AlignHCenter
                    }

                    Label {
                        text: "Looking for StarDict dictionaries in the chosen source."
                        font.pointSize: root.pointSize
                        color: palette.mid
                        wrapMode: Text.WordWrap
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

        // -------------------------------------------------------------------
        // Idx 2 — Checklist
        // -------------------------------------------------------------------
        Frame {
            Layout.fillWidth: true
            Layout.fillHeight: true

            ColumnLayout {
                anchors.fill: parent
                anchors.margins: 12
                spacing: 10

                GridLayout {
                    Layout.fillWidth: true
                    columnSpacing: 12
                    rowSpacing: 8
                    // 2 columns when wide (title | buttons); 1 column when
                    // narrow (title over buttons).
                    columns: root.narrow_layout ? 1 : 2

                    Label {
                        text: `Found ${root.scanned_items.length} ${root.scanned_items.length === 1 ? "dictionary" : "dictionaries"}`
                        font.pointSize: root.largePointSize
                        font.bold: true
                        wrapMode: Text.WordWrap
                        Layout.fillWidth: true
                    }

                    RowLayout {
                        spacing: 8
                        Layout.fillWidth: root.narrow_layout
                        Layout.alignment: root.narrow_layout ? Qt.AlignRight : (Qt.AlignRight | Qt.AlignVCenter)

                        Item {
                            visible: root.narrow_layout
                            Layout.fillWidth: true
                        }

                        Button {
                            text: "Select All"
                            font.pointSize: root.pointSize
                            onClicked: {
                                for (let i = 0; i < checklist_repeater.count; i++) {
                                    const it = checklist_repeater.itemAt(i);
                                    if (it) it.checked = true;
                                }
                                root.recompute();
                            }
                        }

                        Button {
                            text: "Clear Selection"
                            font.pointSize: root.pointSize
                            onClicked: {
                                for (let i = 0; i < checklist_repeater.count; i++) {
                                    const it = checklist_repeater.itemAt(i);
                                    if (it) it.checked = false;
                                }
                                root.recompute();
                            }
                        }
                    }
                }

                ScrollView {
                    id: checklist_scroll
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    contentWidth: availableWidth
                    clip: true

                    ColumnLayout {
                        width: checklist_scroll.availableWidth
                        spacing: 8

                        Repeater {
                            id: checklist_repeater
                            model: root.scanned_items

                            delegate: DictionaryImportRow {
                                required property var modelData

                                Layout.fillWidth: true
                                point_size: root.pointSize

                                source_path: modelData.source_path
                                source_kind: modelData.source_kind
                                title_text: modelData.title
                                entry_count: modelData.entry_count
                                label: modelData.suggested_label
                                lang: "pli"

                                onChanged: root.recompute()
                            }
                        }
                    }
                }

                RowLayout {
                    Layout.fillWidth: true

                    Button {
                        text: "Cancel"
                        font.pointSize: root.pointSize
                        onClicked: {
                            root.canceled();
                            root.hide();
                        }
                    }

                    Item { Layout.fillWidth: true }

                    Button {
                        text: "Import"
                        font.pointSize: root.pointSize
                        enabled: root.can_import
                        onClicked: {
                            const items = [];
                            for (let i = 0; i < checklist_repeater.count; i++) {
                                const it = checklist_repeater.itemAt(i);
                                if (it && it.checked) {
                                    items.push({
                                        path: it.source_path,
                                        kind: it.source_kind,
                                        label: it.label,
                                        lang: it.lang
                                    });
                                }
                            }
                            root.import_batch_requested(JSON.stringify(items));
                            root.hide();
                        }
                    }
                }
            }
        }
    }
}
