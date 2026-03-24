pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import QtQuick.Controls
import QtQuick.Dialogs
import QtQuick.Window

import com.profoundlabs.simsapa

ApplicationWindow {
    id: root
    title: "Chanting Review"
    width: is_mobile ? Screen.desktopAvailableWidth : 700
    height: is_mobile ? Screen.desktopAvailableHeight : Math.min(800, Screen.desktopAvailableHeight)
    visible: true
    color: palette.window
    flags: Qt.Dialog

    readonly property bool is_mobile: Qt.platform.os === "android" || Qt.platform.os === "ios"
    readonly property bool is_desktop: !root.is_mobile
    readonly property int pointSize: is_mobile ? 16 : 12
    property int top_bar_margin: is_mobile ? 24 : 0

    property bool is_dark: theme_helper.is_dark

    // section_uid is set as a context property by C++
    property string current_section_uid: typeof section_uid !== 'undefined' ? section_uid : ""

    // Section data parsed from JSON
    property string section_title: ""
    property string chant_title: ""
    property string collection_title: ""
    property string content_pali: ""

    // Recordings list model
    property var recordings_data: []

    ThemeHelper {
        id: theme_helper
        target_window: root
    }

    // 7.2 Load section detail on completed
    Component.onCompleted: {
        theme_helper.apply();
        root.top_bar_margin = root.is_mobile ? SuttaBridge.get_mobile_top_bar_margin() : 0;
        load_section_data();
    }

    // Stop all playback and save state when window is closed
    onClosing: {
        for (let i = 0; i < playback_repeater.count; i++) {
            let item = playback_repeater.itemAt(i);
            if (item && item.cleanup) {
                item.cleanup();
            }
        }
    }

    function load_section_data() {
        let json_str = SuttaBridge.get_chanting_section_detail_json(root.current_section_uid);
        if (json_str === "null" || json_str === "") {
            return;
        }

        let data = JSON.parse(json_str);
        root.section_title = data.title || "";
        root.content_pali = data.content_pali || "";

        // Look up parent chant and collection titles from the tree data
        let tree_json = SuttaBridge.get_all_chanting_collections_json();
        if (tree_json && tree_json !== "null") {
            let collections = JSON.parse(tree_json);
            find_parent_titles(collections, data.chant_uid);
        }

        // Load recordings
        root.recordings_data = data.recordings || [];
        update_recording_models();
    }

    function find_parent_titles(collections: var, chant_uid: string) {
        for (let i = 0; i < collections.length; i++) {
            let coll = collections[i];
            let chants = coll.chants || [];
            for (let j = 0; j < chants.length; j++) {
                if (chants[j].uid === chant_uid) {
                    root.chant_title = chants[j].title || "";
                    root.collection_title = coll.title || "";
                    return;
                }
            }
        }
    }

    function update_recording_models() {
        reference_model.clear();
        user_model.clear();
        for (let i = 0; i < root.recordings_data.length; i++) {
            let rec = root.recordings_data[i];
            let item = {
                "uid": rec.uid,
                "file_name": rec.file_name,
                "recording_type": rec.recording_type,
                "label": rec.label || rec.file_name,
                "duration_ms": rec.duration_ms,
                "markers_json": rec.markers_json || "[]",
                "volume": rec.volume !== undefined ? rec.volume : 1.0,
                "playback_position_ms": rec.playback_position_ms || 0,
                "waveform_json": rec.waveform_json || ""
            };
            if (rec.recording_type === "reference") {
                reference_model.append(item);
            } else {
                user_model.append(item);
            }
        }
    }

    function refresh_data() {
        // Clear open playback items
        playback_items_model.clear();
        load_section_data();
    }

    // Check if a recording is already open in the playback area
    function is_recording_open(uid: string): bool {
        for (let i = 0; i < playback_items_model.count; i++) {
            if (playback_items_model.get(i).model_recording_uid === uid) {
                return true;
            }
        }
        return false;
    }

    // Close an open playback item by recording uid
    function close_playback_item(uid: string) {
        for (let i = 0; i < playback_items_model.count; i++) {
            if (playback_items_model.get(i).model_recording_uid === uid) {
                playback_items_model.remove(i);
                return;
            }
        }
    }

    // Format duration in ms to a readable string like "2:06"
    function format_duration(ms: int): string {
        if (ms <= 0) return "";
        let total_secs = Math.floor(ms / 1000);
        let mins = Math.floor(total_secs / 60);
        let secs = total_secs % 60;
        return mins + ":" + String(secs).padStart(2, '0');
    }

    // Format a recording label with date and duration
    function format_recording_info(label: string, duration_ms: int): string {
        let parts = [];
        if (label) parts.push(label);
        if (duration_ms > 0) parts.push(format_duration(duration_ms));
        return parts.join("  —  ");
    }

    // Models for recording lists
    ListModel { id: reference_model }
    ListModel { id: user_model }

    // Model for open playback items in the playback area
    ListModel { id: playback_items_model }

    // Update recording list models when waveform data is generated
    Connections {
        target: SuttaBridge
        function onWaveformDataReady(recording_uid: string, waveform_json: string) {
            function update_model(model: ListModel) {
                for (let i = 0; i < model.count; i++) {
                    if (model.get(i).uid === recording_uid) {
                        model.setProperty(i, "waveform_json", waveform_json);
                        return;
                    }
                }
            }
            update_model(reference_model);
            update_model(user_model);
        }
    }

    // Confirmation dialog for deleting a recording
    MessageDialog {
        id: delete_confirm_dialog
        title: "Delete Recording"
        text: "Are you sure you want to delete this recording? This cannot be undone."
        buttons: MessageDialog.Cancel | MessageDialog.Ok
        property string target_uid: ""
        onAccepted: {
            root.close_playback_item(delete_confirm_dialog.target_uid);
            SuttaBridge.delete_chanting_recording(delete_confirm_dialog.target_uid);
            load_section_data();
        }
    }

    // File dialog for adding reference recordings (7.7)
    FileDialog {
        id: reference_file_dialog
        title: "Select Reference Recording"
        nameFilters: ["Audio files (*.ogg *.opus *.mp3 *.wav *.m4a *.flac *.aac *.wma)", "All files (*)"]
        onAccepted: {
            add_recording_from_file(selectedFile.toString(), "reference");
        }
    }

    // File dialog for adding user recordings from file
    FileDialog {
        id: user_file_dialog
        title: "Add Recording from File"
        nameFilters: ["Audio files (*.ogg *.opus *.mp3 *.wav *.m4a *.flac *.aac *.wma)", "All files (*)"]
        onAccepted: {
            add_recording_from_file(selectedFile.toString(), "user");
        }
    }

    function add_recording_from_file(file_url_str: string, rec_type: string) {
        // Strip file:// prefix, handling both file:// and file:///
        let source_path = file_url_str;
        if (source_path.startsWith("file:///")) {
            source_path = source_path.substring(7);  // file:///path -> /path
        } else if (source_path.startsWith("file://")) {
            source_path = source_path.substring(7);
        }

        console.log("add_recording_from_file: source_path =", source_path);

        let timestamp = Date.now();

        // Preserve original file extension
        let original_name = source_path.split("/").pop();
        let ext = original_name.includes(".") ? "." + original_name.split(".").pop() : ".ogg";
        let dest_filename = root.current_section_uid + "_" + rec_type + "_" + timestamp + ext;

        console.log("add_recording_from_file: dest_filename =", dest_filename);

        // Copy file to chanting-recordings directory via bridge
        let result_str = SuttaBridge.copy_file_to_chanting_recordings(source_path, dest_filename);
        console.log("add_recording_from_file: copy result =", result_str);
        let result = JSON.parse(result_str);

        if (result.error) {
            console.error("Failed to copy recording:", result.error);
            return;
        }

        let uid = root.current_section_uid + "_" + rec_type + "_" + timestamp;
        let label_prefix = rec_type === "reference" ? "Reference" : "Recording";
        let rec_json = JSON.stringify({
            "uid": uid,
            "section_uid": root.current_section_uid,
            "file_name": dest_filename,
            "recording_type": rec_type,
            "label": label_prefix + " — " + original_name,
            "duration_ms": 0,
            "markers_json": "[]"
        });
        SuttaBridge.create_chanting_recording(rec_json);
        load_section_data();
    }

    function remove_recording_and_refresh(recording_uid: string) {
        SuttaBridge.delete_chanting_recording(recording_uid);
        load_section_data();
    }

    Frame {
        anchors.fill: parent
        anchors.topMargin: root.top_bar_margin

        ColumnLayout {
            anchors.fill: parent
            spacing: 8

            // 7.3 Header showing section title, parent chant, and collection
            ColumnLayout {
                Layout.fillWidth: true
                spacing: 2

                Label {
                    text: root.collection_title
                    font.pointSize: root.pointSize - 1
                    color: palette.placeholderText
                    visible: root.collection_title !== ""
                }

                Label {
                    text: root.chant_title
                    font.pointSize: root.pointSize
                    color: palette.placeholderText
                    visible: root.chant_title !== ""
                }

                Label {
                    text: root.section_title
                    font.pointSize: root.pointSize + 4
                    font.bold: true
                    wrapMode: Text.Wrap
                    Layout.fillWidth: true
                }
            }

            // Separator
            Rectangle {
                Layout.fillWidth: true
                height: 1
                color: palette.mid
            }

            // 7.4 Scrollable Pali text area
            ScrollView {
                Layout.fillWidth: true
                Layout.preferredHeight: Math.min(200, pali_text.implicitHeight + 20)
                Layout.maximumHeight: 300
                clip: true

                TextArea {
                    id: pali_text
                    text: root.content_pali
                    readOnly: true
                    wrapMode: TextEdit.Wrap
                    font.pointSize: root.pointSize + 2
                    font.family: "serif"
                    background: Rectangle {
                        color: palette.base
                        border.color: palette.mid
                        border.width: 1
                        radius: 4
                    }
                }
            }

            // Separator
            Rectangle {
                Layout.fillWidth: true
                height: 1
                color: palette.mid
            }

            // 7.5 Recording list panel
            RowLayout {
                Layout.fillWidth: true
                spacing: 8

                Label {
                    text: "Recordings"
                    font.pointSize: root.pointSize + 2
                    font.bold: true
                    Layout.fillWidth: true
                }

                // 7.6 New Recording button
                Button {
                    text: "New Recording"
                    onClicked: {
                        let uid = root.current_section_uid + "_user_" + Date.now();
                        let recordings_dir = SuttaBridge.get_chanting_recordings_dir();
                        playback_items_model.append({
                            "model_recording_uid": uid,
                            "model_file_path": "",
                            "model_label": "Recording — " + new Date().toLocaleString(undefined, {year: 'numeric', month: 'short', day: 'numeric', hour: '2-digit', minute: '2-digit'}),
                            "model_recording_type": "user",
                            "model_is_new_recording": true,
                            "model_volume": 1.0,
                            "model_playback_position_ms": 0,
                            "model_markers_json": "[]",
                            "model_waveform_json": ""
                        });
                    }
                }

                // Add recording from existing file
                Button {
                    text: "Add from File"
                    onClicked: user_file_dialog.open()
                }

                // 7.7 Add Reference Recording button
                Button {
                    text: "Add Reference"
                    onClicked: reference_file_dialog.open()
                }
            }

            // Reference recordings group
            ColumnLayout {
                Layout.fillWidth: true
                visible: reference_model.count > 0
                spacing: 2

                Label {
                    text: "Reference"
                    font.pointSize: root.pointSize - 1
                    font.bold: true
                    color: palette.placeholderText
                }

                Repeater {
                    model: reference_model
                    delegate: Frame {
                        id: ref_delegate
                        Layout.fillWidth: true

                        required property int index
                        required property string uid
                        required property string file_name
                        required property string label
                        required property int duration_ms
                        required property string markers_json
                        required property real volume
                        required property int playback_position_ms
                        required property string waveform_json

                        property bool is_open: root.is_recording_open(ref_delegate.uid)

                        background: Rectangle {
                            color: ref_delegate.is_open ? palette.highlight : palette.base
                            border.color: palette.mid
                            border.width: 1
                            radius: 4
                        }

                        function toggle_open() {
                            if (ref_delegate.is_open) {
                                root.close_playback_item(ref_delegate.uid);
                            } else {
                                let recordings_dir = SuttaBridge.get_chanting_recordings_dir();
                                let fp = ref_delegate.file_name.startsWith("/")
                                    ? ref_delegate.file_name
                                    : recordings_dir + "/" + ref_delegate.file_name;
                                playback_items_model.append({
                                    "model_recording_uid": ref_delegate.uid,
                                    "model_file_path": fp,
                                    "model_label": ref_delegate.label,
                                    "model_recording_type": "reference",
                                    "model_is_new_recording": false,
                                    "model_volume": ref_delegate.volume,
                                    "model_playback_position_ms": ref_delegate.playback_position_ms,
                                    "model_markers_json": ref_delegate.markers_json,
                                    "model_waveform_json": ref_delegate.waveform_json
                                });
                            }
                        }

                        RowLayout {
                            anchors.fill: parent
                            spacing: 4

                            Label {
                                text: root.format_recording_info(ref_delegate.label, ref_delegate.duration_ms)
                                elide: Text.ElideRight
                                Layout.fillWidth: true
                                color: ref_delegate.is_open ? palette.highlightedText : palette.text

                                MouseArea {
                                    anchors.fill: parent
                                    cursorShape: Qt.PointingHandCursor
                                    onClicked: ref_delegate.toggle_open()
                                }
                            }

                            Button {
                                text: ref_delegate.is_open ? "Close" : "Open"
                                onClicked: ref_delegate.toggle_open()
                            }
                        }
                    }
                }
            }

            // User recordings group
            ColumnLayout {
                Layout.fillWidth: true
                visible: user_model.count > 0
                spacing: 2

                Label {
                    text: "User Recordings"
                    font.pointSize: root.pointSize - 1
                    font.bold: true
                    color: palette.placeholderText
                }

                Repeater {
                    model: user_model
                    delegate: Frame {
                        id: user_delegate
                        Layout.fillWidth: true

                        required property int index
                        required property string uid
                        required property string file_name
                        required property string label
                        required property int duration_ms
                        required property string markers_json
                        required property real volume
                        required property int playback_position_ms
                        required property string waveform_json

                        property bool is_open: root.is_recording_open(user_delegate.uid)

                        background: Rectangle {
                            color: user_delegate.is_open ? palette.highlight : palette.base
                            border.color: palette.mid
                            border.width: 1
                            radius: 4
                        }

                        function toggle_open() {
                            if (user_delegate.is_open) {
                                root.close_playback_item(user_delegate.uid);
                            } else {
                                let recordings_dir = SuttaBridge.get_chanting_recordings_dir();
                                let fp = user_delegate.file_name.startsWith("/")
                                    ? user_delegate.file_name
                                    : recordings_dir + "/" + user_delegate.file_name;
                                playback_items_model.append({
                                    "model_recording_uid": user_delegate.uid,
                                    "model_file_path": fp,
                                    "model_label": user_delegate.label,
                                    "model_recording_type": "user",
                                    "model_is_new_recording": false,
                                    "model_volume": user_delegate.volume,
                                    "model_playback_position_ms": user_delegate.playback_position_ms,
                                    "model_markers_json": user_delegate.markers_json,
                                    "model_waveform_json": user_delegate.waveform_json
                                });
                            }
                        }

                        RowLayout {
                            anchors.fill: parent
                            spacing: 4

                            Label {
                                text: root.format_recording_info(user_delegate.label, user_delegate.duration_ms)
                                elide: Text.ElideRight
                                Layout.fillWidth: true
                                color: user_delegate.is_open ? palette.highlightedText : palette.text

                                MouseArea {
                                    anchors.fill: parent
                                    cursorShape: Qt.PointingHandCursor
                                    onClicked: user_delegate.toggle_open()
                                }
                            }

                            Button {
                                id: user_open_close_btn
                                text: user_delegate.is_open ? "Close" : "Open"
                                onClicked: user_delegate.toggle_open()
                            }

                            // Delete button (user recordings only)
                            Button {
                                icon.source: "icons/32x32/ion--trash-outline.png"
                                implicitHeight: user_open_close_btn.implicitHeight
                                implicitWidth: implicitHeight
                                onClicked: {
                                    delete_confirm_dialog.target_uid = user_delegate.uid;
                                    delete_confirm_dialog.open();
                                }
                            }
                        }
                    }
                }
            }

            // Separator
            Rectangle {
                Layout.fillWidth: true
                height: 1
                color: palette.mid
                visible: playback_items_model.count > 0
            }

            // 7.8 Scrollable playback area
            ScrollView {
                Layout.fillWidth: true
                Layout.fillHeight: true
                clip: true
                visible: playback_items_model.count > 0

                ColumnLayout {
                    width: parent.width
                    spacing: 8

                    Repeater {
                        id: playback_repeater
                        model: playback_items_model
                        delegate: RecordingPlaybackItem {
                            id: playback_delegate
                            Layout.fillWidth: true

                            required property int index
                            required property string model_recording_uid
                            required property string model_file_path
                            required property string model_label
                            required property string model_recording_type
                            required property bool model_is_new_recording
                            required property real model_volume
                            required property int model_playback_position_ms
                            required property string model_markers_json
                            required property string model_waveform_json

                            recording_uid: playback_delegate.model_recording_uid
                            file_path: playback_delegate.model_file_path
                            label: playback_delegate.model_label
                            recording_type: playback_delegate.model_recording_type
                            is_new_recording: playback_delegate.model_is_new_recording
                            volume: playback_delegate.model_volume
                            playback_position_ms: playback_delegate.model_playback_position_ms
                            markers_json: playback_delegate.model_markers_json
                            waveform_json: playback_delegate.model_waveform_json

                            // 7.10 Close removes from playback area
                            onClosed: {
                                playback_items_model.remove(playback_delegate.index);
                            }

                            // Remove recording from DB when file not found
                            onRemove_requested: function(rec_uid) {
                                root.remove_recording_and_refresh(rec_uid);
                                playback_items_model.remove(playback_delegate.index);
                            }

                            // 7.11 Recording complete — persist and refresh
                            onRecording_completed: function(recorded_file_path) {
                                // Store just the filename for portability
                                let file_name = recorded_file_path.split("/").pop();
                                let rec_json = JSON.stringify({
                                    "uid": playback_delegate.model_recording_uid,
                                    "section_uid": root.current_section_uid,
                                    "file_name": file_name,
                                    "recording_type": "user",
                                    "label": playback_delegate.model_label,
                                    "duration_ms": 0,
                                    "markers_json": "[]"
                                });
                                SuttaBridge.create_chanting_recording(rec_json);
                                root.load_section_data();
                            }
                        }
                    }
                }
            }

            // Spacer when no playback items
            Item {
                Layout.fillHeight: true
                visible: playback_items_model.count === 0
            }
        }
    }
}
