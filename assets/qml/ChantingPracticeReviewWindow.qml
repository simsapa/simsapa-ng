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

    property string window_id
    property bool is_dark: theme_helper.is_dark

    // section_uid is set as a context property by C++
    property string current_section_uid: typeof section_uid !== 'undefined' ? section_uid : ""

    // Section data parsed from JSON
    property string section_title: ""
    property string chant_title: ""
    property string collection_title: ""
    property string content_pali: ""
    property var section_data: null  // Full section JSON for updates

    // Recordings list model
    property var recordings_data: []

    // UID of a recording that should auto-open after data reload
    property string auto_open_uid: ""

    // Emitted when a recording starts playing; other items should pause
    signal pause_other_playback(string playing_uid)

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
        function cleanup_repeater(repeater: Repeater) {
            for (let i = 0; i < repeater.count; i++) {
                let item = repeater.itemAt(i);
                if (item && item.cleanup_playback) {
                    item.cleanup_playback();
                }
            }
        }
        cleanup_repeater(ref_repeater);
        cleanup_repeater(user_repeater);
        for (let i = 0; i < new_rec_repeater.count; i++) {
            let item = new_rec_repeater.itemAt(i);
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
        root.section_data = data;
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
        new_recordings_model.clear();
        load_section_data();
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

    // Model for new recordings not yet saved to DB
    ListModel { id: new_recordings_model }

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

            // 7.4 Scrollable Pali text area (editable, auto-saves)
            ScrollView {
                Layout.fillWidth: true
                Layout.preferredHeight: Math.min(200, pali_text.implicitHeight + 20)
                Layout.maximumHeight: 300
                clip: true

                TextArea {
                    id: pali_text
                    text: root.content_pali
                    wrapMode: TextEdit.Wrap
                    font.pointSize: root.pointSize + 2
                    font.family: "serif"
                    background: Rectangle {
                        color: palette.base
                        border.color: pali_text.activeFocus ? palette.highlight : palette.mid
                        border.width: 1
                        radius: 4
                    }

                    onTextChanged: {
                        if (root.section_data !== null && text !== root.content_pali) {
                            pali_save_timer.restart();
                        }
                    }
                }
            }

            Timer {
                id: pali_save_timer
                interval: 400
                repeat: false
                onTriggered: {
                    if (root.section_data === null) return;
                    root.content_pali = pali_text.text;
                    let data = Object.assign({}, root.section_data);
                    data.content_pali = pali_text.text;
                    SuttaBridge.update_chanting_section(JSON.stringify(data));
                }
            }

            Button {
                text: "Gloss Chanting Text"
                Layout.alignment: Qt.AlignLeft
                enabled: pali_text.text.trim().length > 0
                onClicked: {
                    SuttaBridge.run_gloss_in_sutta_window(root.window_id, pali_text.text);
                }
            }

            // Separator
            Rectangle {
                Layout.fillWidth: true
                height: 1
                color: palette.mid
            }

            // 7.5 Scrollable recording list with inline playback
            ScrollView {
                id: recordings_scroll
                Layout.fillWidth: true
                Layout.fillHeight: true
                clip: true
                contentWidth: availableWidth

                ColumnLayout {
                    width: recordings_scroll.availableWidth
                    spacing: 8

                    // Recording list header
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
                                new_recordings_model.append({
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

                    // Reference recordings group with inline playback
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
                            id: ref_repeater
                            model: reference_model
                            delegate: ColumnLayout {
                                id: ref_delegate
                                Layout.fillWidth: true
                                spacing: 4

                                required property int index
                                required property string uid
                                required property string file_name
                                required property string label
                                required property int duration_ms
                                required property string markers_json
                                required property real volume
                                required property int playback_position_ms
                                required property string waveform_json

                                property bool is_open: false

                                property string computed_file_path: {
                                    if (ref_delegate.file_name === "") return "";
                                    if (ref_delegate.file_name.startsWith("/")) return ref_delegate.file_name;
                                    return SuttaBridge.get_chanting_recordings_dir() + "/" + ref_delegate.file_name;
                                }

                                function cleanup_playback() {
                                    if (ref_delegate.is_open && ref_playback_loader.item && ref_playback_loader.item.cleanup) {
                                        ref_playback_loader.item.cleanup();
                                    }
                                }

                                Frame {
                                    Layout.fillWidth: true

                                    background: Rectangle {
                                        color: ref_delegate.is_open ? palette.highlight : Qt.tint(palette.base, Qt.rgba(palette.highlight.r, palette.highlight.g, palette.highlight.b, 0.15))
                                        border.color: palette.mid
                                        border.width: 1
                                        radius: 4
                                    }

                                    contentItem: RowLayout {
                                        spacing: 4

                                        Label {
                                            text: root.format_recording_info(ref_delegate.label, ref_delegate.duration_ms)
                                            elide: Text.ElideRight
                                            Layout.fillWidth: true
                                            color: ref_delegate.is_open ? palette.highlightedText : palette.text
                                        }

                                        Button {
                                            text: ref_delegate.is_open ? "Close" : "Open"
                                            onClicked: ref_delegate.is_open = !ref_delegate.is_open
                                        }
                                    }

                                    MouseArea {
                                        anchors.fill: parent
                                        cursorShape: Qt.PointingHandCursor
                                        onClicked: ref_delegate.is_open = !ref_delegate.is_open
                                        z: -1
                                    }
                                }

                                Connections {
                                    target: root
                                    function onPause_other_playback(playing_uid: string) {
                                        if (playing_uid !== ref_delegate.uid && ref_playback_loader.item) {
                                            ref_playback_loader.item.pause_playback();
                                        }
                                    }
                                }

                                Loader {
                                    id: ref_playback_loader
                                    Layout.fillWidth: true
                                    active: ref_delegate.is_open
                                    visible: active
                                    sourceComponent: Component {
                                        RecordingPlaybackItem {
                                            width: ref_playback_loader.width

                                            recording_uid: ref_delegate.uid
                                            file_path: ref_delegate.computed_file_path
                                            label: ref_delegate.label
                                            recording_type: "reference"
                                            is_new_recording: false
                                            volume: ref_delegate.volume
                                            playback_position_ms: ref_delegate.playback_position_ms
                                            markers_json: ref_delegate.markers_json
                                            waveform_json: ref_delegate.waveform_json

                                            onClosed: {
                                                ref_delegate.is_open = false;
                                            }

                                            onRemove_requested: function(rec_uid) {
                                                root.remove_recording_and_refresh(rec_uid);
                                            }

                                            onPlayback_started: {
                                                root.pause_other_playback(ref_delegate.uid);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // User recordings group with inline playback
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
                            id: user_repeater
                            model: user_model
                            delegate: ColumnLayout {
                                id: user_delegate
                                Layout.fillWidth: true
                                spacing: 4

                                required property int index
                                required property string uid
                                required property string file_name
                                required property string label
                                required property int duration_ms
                                required property string markers_json
                                required property real volume
                                required property int playback_position_ms
                                required property string waveform_json

                                property bool is_open: false

                                Component.onCompleted: {
                                    if (root.auto_open_uid !== "" && root.auto_open_uid === user_delegate.uid) {
                                        user_delegate.is_open = true;
                                        root.auto_open_uid = "";
                                    }
                                }

                                property string computed_file_path: {
                                    if (user_delegate.file_name === "") return "";
                                    if (user_delegate.file_name.startsWith("/")) return user_delegate.file_name;
                                    return SuttaBridge.get_chanting_recordings_dir() + "/" + user_delegate.file_name;
                                }

                                function cleanup_playback() {
                                    if (user_delegate.is_open && user_playback_loader.item && user_playback_loader.item.cleanup) {
                                        user_playback_loader.item.cleanup();
                                    }
                                }

                                Frame {
                                    id: user_frame
                                    Layout.fillWidth: true

                                    background: Rectangle {
                                        color: user_delegate.is_open ? palette.highlight : Qt.tint(palette.base, Qt.rgba(palette.highlight.r, palette.highlight.g, palette.highlight.b, 0.15))
                                        border.color: palette.mid
                                        border.width: 1
                                        radius: 4
                                    }

                                    contentItem: RowLayout {
                                        spacing: 4

                                        Label {
                                            text: root.format_recording_info(user_delegate.label, user_delegate.duration_ms)
                                            elide: Text.ElideRight
                                            Layout.fillWidth: true
                                            color: user_delegate.is_open ? palette.highlightedText : palette.text
                                        }

                                        Button {
                                            id: user_open_close_btn
                                            text: user_delegate.is_open ? "Close" : "Open"
                                            onClicked: user_delegate.is_open = !user_delegate.is_open
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

                                    MouseArea {
                                        anchors.fill: parent
                                        cursorShape: Qt.PointingHandCursor
                                        onClicked: user_delegate.is_open = !user_delegate.is_open
                                        z: -1
                                    }
                                }

                                Connections {
                                    target: root
                                    function onPause_other_playback(playing_uid: string) {
                                        if (playing_uid !== user_delegate.uid && user_playback_loader.item) {
                                            user_playback_loader.item.pause_playback();
                                        }
                                    }
                                }

                                Loader {
                                    id: user_playback_loader
                                    Layout.fillWidth: true
                                    active: user_delegate.is_open
                                    visible: active
                                    sourceComponent: Component {
                                        RecordingPlaybackItem {
                                            width: user_playback_loader.width

                                            recording_uid: user_delegate.uid
                                            file_path: user_delegate.computed_file_path
                                            label: user_delegate.label
                                            recording_type: "user"
                                            is_new_recording: false
                                            volume: user_delegate.volume
                                            playback_position_ms: user_delegate.playback_position_ms
                                            markers_json: user_delegate.markers_json
                                            waveform_json: user_delegate.waveform_json

                                            onClosed: {
                                                user_delegate.is_open = false;
                                            }

                                            onRemove_requested: function(rec_uid) {
                                                root.remove_recording_and_refresh(rec_uid);
                                            }

                                            onPlayback_started: {
                                                root.pause_other_playback(user_delegate.uid);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // New recordings (not yet saved to DB)
                    Repeater {
                        id: new_rec_repeater
                        model: new_recordings_model
                        delegate: RecordingPlaybackItem {
                            id: new_rec_delegate
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

                            recording_uid: new_rec_delegate.model_recording_uid
                            file_path: new_rec_delegate.model_file_path
                            label: new_rec_delegate.model_label
                            recording_type: new_rec_delegate.model_recording_type
                            is_new_recording: new_rec_delegate.model_is_new_recording
                            volume: new_rec_delegate.model_volume
                            playback_position_ms: new_rec_delegate.model_playback_position_ms
                            markers_json: new_rec_delegate.model_markers_json
                            waveform_json: new_rec_delegate.model_waveform_json

                            onPlayback_started: {
                                root.pause_other_playback(new_rec_delegate.model_recording_uid);
                            }

                            Connections {
                                target: root
                                function onPause_other_playback(playing_uid: string) {
                                    if (playing_uid !== new_rec_delegate.model_recording_uid) {
                                        new_rec_delegate.pause_playback();
                                    }
                                }
                            }

                            onClosed: {
                                new_recordings_model.remove(new_rec_delegate.index);
                            }

                            onRemove_requested: function(rec_uid) {
                                root.remove_recording_and_refresh(rec_uid);
                                new_recordings_model.remove(new_rec_delegate.index);
                            }

                            // 7.11 Recording complete — persist and refresh
                            onRecording_completed: function(recorded_file_path) {
                                let file_name = recorded_file_path.split("/").pop();
                                let uid = new_rec_delegate.model_recording_uid;
                                let rec_json = JSON.stringify({
                                    "uid": uid,
                                    "section_uid": root.current_section_uid,
                                    "file_name": file_name,
                                    "recording_type": "user",
                                    "label": new_rec_delegate.model_label,
                                    "duration_ms": 0,
                                    "markers_json": "[]"
                                });
                                SuttaBridge.create_chanting_recording(rec_json);
                                // Remove from new recordings and auto-open in user list
                                new_recordings_model.remove(new_rec_delegate.index);
                                root.auto_open_uid = uid;
                                root.load_section_data();
                            }
                        }
                    }

                    // Bottom spacer
                    Item {
                        Layout.fillHeight: true
                    }
                }
            }
        }
    }
}
