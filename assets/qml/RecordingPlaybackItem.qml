pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import QtQuick.Controls
import QtMultimedia

import com.profoundlabs.simsapa

Item {
    id: root

    // Properties (6.1)
    property string recording_uid: ""
    property string file_path: ""
    property string label: ""
    property string recording_type: "user"  // "reference" or "user"
    property bool is_new_recording: false
    property real volume: 1.0
    property int playback_position_ms: 0

    // Signals (6.7, 6.8)
    signal closed()
    signal recording_completed(string recorded_file_path)
    signal remove_requested(string recording_uid)

    // Markers data (JSON array of marker objects)
    property string markers_json: "[]"
    property var markers: []

    // Cached waveform data from database (set by parent)
    property string waveform_json: ""

    // Internal state
    property bool is_recording: false
    property int recording_elapsed_ms: 0
    property bool file_not_found: false
    property string error_message: ""
    property var waveform_data: []
    property bool waveform_loading: false
    property bool pending_recording_stop: false

    // Range playback state (8.8, 8.9)
    property bool loop_enabled: false
    property string active_range_id: ""  // ID of range marker currently being played
    property int active_range_start_ms: 0
    property int active_range_end_ms: 0
    property bool range_seek_pending: false  // Guard against premature end-detection after seek
    // Last position we explicitly seeked to. Used as the display position
    // when the player is NOT playing, because player.position is unreliable
    // after an async seek while paused.  Set to -1 when no pending seek.
    property int visual_position_override: -1
    property int effective_position: {
        // While playing, player.position updates continuously and is reliable.
        if (player.playbackState === MediaPlayer.PlayingState)
            return player.position;
        // While paused/stopped, prefer our explicit seek target if set.
        if (visual_position_override >= 0)
            return visual_position_override;
        return player.position;
    }

    implicitHeight: main_column.implicitHeight + 16

    Rectangle {
        anchors.fill: parent
        color: root.file_not_found ? Qt.rgba(1, 0.9, 0.9, 1) : palette.base
        border.color: root.file_not_found ? "red" : palette.mid
        border.width: 1
        radius: 4
    }

    function check_file() {
        if (root.file_path !== "" && !root.is_new_recording) {
            let exists = SuttaBridge.check_file_exists(root.file_path);
            console.log("RecordingPlaybackItem check_file:", root.file_path, "exists:", exists);
            root.file_not_found = !exists;
            if (!exists) {
                root.error_message = "Audio file not found:\n" + root.file_path;
            } else {
                root.error_message = "";
            }
        } else {
            root.file_not_found = false;
            root.error_message = "";
        }
    }

    onFile_pathChanged: {
        check_file();
        load_waveform();
    }

    onMarkers_jsonChanged: {
        try {
            root.markers = JSON.parse(root.markers_json);
        } catch (e) {
            root.markers = [];
        }
    }

    Component.onCompleted: {
        // Delay check slightly to ensure all properties are bound
        Qt.callLater(check_file);
        Qt.callLater(load_waveform);
        // Parse initial markers
        try {
            root.markers = JSON.parse(root.markers_json);
        } catch (e) {
            root.markers = [];
        }
    }

    function load_waveform() {
        if (root.file_path === "" || root.file_not_found) {
            root.waveform_data = [];
            root.waveform_loading = false;
            return;
        }

        // Use cached data from database if available
        if (root.waveform_json !== "" && root.waveform_json !== "[]") {
            try {
                root.waveform_data = JSON.parse(root.waveform_json);
                root.waveform_loading = false;
                return;
            } catch (e) {
                // Fall through to generate
            }
        }

        // Generate in background
        root.waveform_loading = true;
        SuttaBridge.generate_waveform_data(root.recording_uid, root.file_path, 200);
    }

    // Handle async waveform data from background thread
    Connections {
        target: SuttaBridge
        function onWaveformDataReady(recording_uid: string, waveform_json: string) {
            if (recording_uid !== root.recording_uid) return;
            root.waveform_loading = false;
            root.waveform_json = waveform_json;
            try {
                root.waveform_data = JSON.parse(waveform_json);
            } catch (e) {
                root.waveform_data = [];
            }
        }
    }

    // Audio playback (6.2) — only set source if file exists
    MediaPlayer {
        id: player
        source: root.file_path !== "" && !root.file_not_found
            ? Qt.resolvedUrl("file://" + root.file_path)
            : ""
        audioOutput: AudioOutput {
            id: audio_output
            volume: root.volume
        }

        // Restore saved playback position once media is loaded
        onMediaStatusChanged: {
            if (mediaStatus === MediaPlayer.LoadedMedia && root.playback_position_ms > 0) {
                player.position = root.playback_position_ms;
                // Stay paused — don't auto-play
            }
        }
    }

    // Used to re-detect the current default audio input when starting a recording
    MediaDevices { id: media_devices }

    // Audio recording (6.4) — no hardcoded format, let system pick best available
    CaptureSession {
        id: capture_session
        audioInput: AudioInput {
            id: audio_input
        }
        recorder: MediaRecorder {
            id: recorder
            quality: MediaRecorder.NormalQuality

            onRecorderStateChanged: {
                // File is fully finalized only when state transitions to StoppedState
                if (recorderState === MediaRecorder.StoppedState && root.pending_recording_stop) {
                    root.pending_recording_stop = false;
                    root.finalize_recording();
                }
            }
        }
    }

    // Resumes playback after a brief pause-seek cycle.
    Timer {
        id: seek_resume_timer
        interval: 80
        repeat: false
        onTriggered: {
            player.position = root.visual_position_override;
            root.visual_position_override = -1;
            player.play();
        }
    }

    // Seek to a position reliably.  While playing, Qt often ignores a bare
    // `player.position = X`, so we pause → seek → short delay → resume.
    function seek_to(position_ms: int) {
        let was_playing = (player.playbackState === MediaPlayer.PlayingState);
        if (was_playing) {
            player.pause();
        }
        root.visual_position_override = position_ms;
        player.position = position_ms;
        if (was_playing) {
            seek_resume_timer.restart();
        }
    }

    // Debounce timer for persisting volume changes
    Timer {
        id: volume_save_timer
        interval: 300
        repeat: false
        onTriggered: {
            if (root.recording_uid !== "") {
                SuttaBridge.update_recording_volume(root.recording_uid, root.volume);
            }
        }
    }

    // Debounce timer for persisting playback position
    Timer {
        id: position_save_timer
        interval: 300
        repeat: false
        onTriggered: {
            root.save_position();
        }
    }

    function save_position() {
        if (root.recording_uid !== "" && !root.is_new_recording) {
            SuttaBridge.update_recording_playback_position(root.recording_uid, Math.round(player.position));
        }
    }

    // Stop playback/recording and persist volume + position
    function cleanup() {
        seek_resume_timer.stop();
        if (root.is_recording) {
            stop_recording();
        }
        if (player.playbackState === MediaPlayer.PlayingState) {
            player.stop();
        }
        root.visual_position_override = -1;
        save_position();
        if (root.recording_uid !== "") {
            SuttaBridge.update_recording_volume(root.recording_uid, root.volume);
        }
    }

    // One-shot timer: after seeking to range start, pause briefly then play.
    // This gives Qt MediaPlayer time to honour the position change before
    // playback resumes.
    Timer {
        id: range_seek_timer
        interval: 80
        repeat: false
        onTriggered: {
            // Re-assert the seek position in case the first assignment was
            // swallowed, then start playback.
            player.position = root.active_range_start_ms;
            root.visual_position_override = -1;
            player.play();
        }
    }

    // Range playback polling timer (8.8, 8.9) — checks if player reached end_ms
    Timer {
        id: range_playback_timer
        interval: 50
        repeat: true
        running: root.active_range_id !== "" && player.playbackState === MediaPlayer.PlayingState
        onTriggered: {
            // After a seek (play_range or loop-back), wait until the player
            // position is near the range start before checking for the end.
            // "Near" = within 500 ms of start, to tolerate seek imprecision
            // while avoiding false-clear when paused in the middle of the range.
            if (root.range_seek_pending) {
                let tolerance = Math.min(500, (root.active_range_end_ms - root.active_range_start_ms) / 2);
                if (player.position >= root.active_range_start_ms
                    && player.position <= root.active_range_start_ms + tolerance) {
                    root.range_seek_pending = false;
                }
                return;
            }

            if (player.position >= root.active_range_end_ms) {
                if (root.loop_enabled) {
                    root.range_seek_pending = true;
                    root.visual_position_override = root.active_range_start_ms;
                    player.pause();
                    player.position = root.active_range_start_ms;
                    range_seek_timer.restart();
                } else {
                    let end_pos = root.active_range_end_ms;
                    player.pause();
                    player.position = end_pos;
                    stop_range_playback();
                    // Set override AFTER stop_range_playback (which clears it)
                    root.visual_position_override = end_pos;
                }
            }
        }
    }

    // Timer for recording elapsed time (6.6)
    Timer {
        id: recording_timer
        interval: 100
        repeat: true
        running: root.is_recording
        onTriggered: {
            root.recording_elapsed_ms += 100;
        }
    }

    // Helper to format ms to MM:SS
    function format_time(ms: int): string {
        let total_secs = Math.floor(ms / 1000);
        let mins = Math.floor(total_secs / 60);
        let secs = total_secs % 60;
        return String(mins).padStart(2, '0') + ":" + String(secs).padStart(2, '0');
    }

    ColumnLayout {
        id: main_column
        x: 8
        y: 8
        width: root.width - 16
        spacing: 6

        // Header row with label and close button (6.7)
        RowLayout {
            Layout.fillWidth: true
            spacing: 8

            Label {
                text: root.label
                font.bold: true
                elide: Text.ElideRight
                Layout.fillWidth: true
            }

            ToolButton {
                text: "✕"
                font.pointSize: 10
                implicitWidth: 28
                implicitHeight: 28
                onClicked: {
                    root.save_position();
                    root.closed();
                }
            }
        }

        // File not found error display
        ColumnLayout {
            Layout.fillWidth: true
            visible: root.file_not_found
            spacing: 6

            Label {
                text: root.error_message
                color: "red"
                wrapMode: Text.Wrap
                Layout.fillWidth: true
            }

            RowLayout {
                Layout.fillWidth: true
                spacing: 8

                Button {
                    text: "Remove Recording"
                    onClicked: {
                        root.remove_requested(root.recording_uid);
                    }
                }

                Button {
                    text: "Close"
                    onClicked: root.closed()
                }
            }
        }

        // Recording state indicator (6.6)
        RowLayout {
            Layout.fillWidth: true
            spacing: 6
            visible: root.is_recording

            Rectangle {
                id: recording_dot
                width: 12
                height: 12
                radius: 6
                color: "red"

                SequentialAnimation on opacity {
                    running: root.is_recording
                    loops: Animation.Infinite
                    NumberAnimation { to: 0.3; duration: 500 }
                    NumberAnimation { to: 1.0; duration: 500 }
                }
            }

            Label {
                text: "Recording  " + root.format_time(root.recording_elapsed_ms)
                color: "red"
                font.bold: true
            }
        }

        // Audio controls row (6.2) — hidden when file not found
        RowLayout {
            Layout.fillWidth: true
            spacing: 4
            visible: !root.file_not_found

            // Record button — only for new/user recordings (6.4)
            Button {
                id: record_button
                text: root.is_recording ? "⏹ Stop" : "⏺ Record"
                visible: root.is_new_recording || root.recording_type === "user"
                enabled: player.playbackState !== MediaPlayer.PlayingState
                onClicked: {
                    if (root.is_recording) {
                        stop_recording();
                    } else {
                        start_recording();
                    }
                }
            }

            // Play/Pause button
            Button {
                text: player.playbackState === MediaPlayer.PlayingState ? "⏸" : "▶"
                enabled: !root.is_recording && root.file_path !== "" && !root.file_not_found
                implicitWidth: 40
                onClicked: {
                    if (player.playbackState === MediaPlayer.PlayingState) {
                        player.pause();
                        position_save_timer.restart();
                    } else {
                        // If the user seeked while paused (waveform click or
                        // scrubber drag), re-assert that position before playing
                        // because Qt may not have processed the seek yet.
                        if (root.visual_position_override >= 0) {
                            player.position = root.visual_position_override;
                        }
                        root.visual_position_override = -1;
                        player.play();
                    }
                }
            }

            // Stop button
            Button {
                text: "⏹"
                enabled: !root.is_recording && player.playbackState !== MediaPlayer.StoppedState
                implicitWidth: 40
                onClicked: {
                    seek_resume_timer.stop();
                    root.stop_range_playback();
                    player.stop();
                    player.position = 0;
                    root.visual_position_override = 0;
                    position_save_timer.restart();
                }
            }

            Item { Layout.fillWidth: true }

            // Time display (6.3)
            Label {
                text: root.format_time(root.effective_position) + " / " + root.format_time(player.duration)
                font.family: "monospace"
                visible: !root.is_recording && root.file_path !== "" && !root.file_not_found
            }
        }

        // Waveform placeholder while loading
        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: 60
            visible: !root.is_recording && root.file_path !== "" && !root.file_not_found && root.waveform_loading && root.waveform_data.length === 0
            color: palette.base
            border.color: palette.mid
            border.width: 1
            radius: 2

            Label {
                anchors.centerIn: parent
                text: "Loading waveform..."
                color: palette.placeholderText
                font.pointSize: 10
            }
        }

        // Waveform visualization — above the scrubber
        WaveformView {
            id: waveform_view
            Layout.fillWidth: true
            Layout.preferredHeight: 60
            visible: !root.is_recording && root.file_path !== "" && !root.file_not_found && root.waveform_data.length > 0

            waveform_data: root.waveform_data
            duration_ms: player.duration
            playback_position_ms: root.effective_position
            is_playing: player.playbackState === MediaPlayer.PlayingState
            markers: root.markers

            onSeek_requested: function(position_ms) {
                root.stop_range_playback();
                root.seek_to(position_ms);
                position_save_timer.restart();
            }

            onRange_selected: function(start_ms, end_ms) {
                root.add_range_marker(start_ms, end_ms);
            }
        }

        // Scrubber slider (6.3) — hidden when file not found
        Slider {
            id: scrubber
            Layout.fillWidth: true
            visible: !root.is_recording && root.file_path !== "" && !root.file_not_found
            from: 0
            to: player.duration > 0 ? player.duration : 1
            value: root.effective_position

            onMoved: {
                root.stop_range_playback();
                root.seek_to(Math.round(value));
                position_save_timer.restart();
            }
        }

        // Volume control — hidden when file not found
        RowLayout {
            Layout.fillWidth: true
            spacing: 6
            visible: !root.is_recording && root.file_path !== "" && !root.file_not_found

            Label {
                text: "🔊"
                font.pointSize: 10
            }

            Slider {
                id: volume_slider
                Layout.fillWidth: true
                from: 0.0
                to: 1.0
                value: root.volume
                stepSize: 0.05

                onMoved: {
                    root.volume = value;
                    if (root.recording_uid !== "") {
                        volume_save_timer.restart();
                    }
                }
            }

            Label {
                text: Math.round(volume_slider.value * 100) + "%"
                font.family: "monospace"
                Layout.preferredWidth: 40
                horizontalAlignment: Text.AlignRight
            }
        }

        // Marker controls row (8.2, 8.3, 8.9)
        RowLayout {
            Layout.fillWidth: true
            spacing: 6
            visible: !root.is_recording && root.file_path !== "" && !root.file_not_found

            Button {
                text: "＋ Position"
                enabled: player.duration > 0
                onClicked: root.add_position_marker()

                ToolTip.visible: hovered
                ToolTip.text: "Add a position marker at the current playback time"
            }

            Button {
                text: "＋ Range"
                enabled: player.duration > 0
                onClicked: {
                    // Create a range marker around the current position (±2 seconds)
                    let pos = Math.round(player.position);
                    let start = Math.max(0, pos - 2000);
                    let end = Math.min(player.duration, pos + 2000);
                    root.add_range_marker(start, end);
                }

                ToolTip.visible: hovered
                ToolTip.text: "Add a range marker around the current position (drag on waveform for precise ranges)"
            }

            Item { Layout.fillWidth: true }

            CheckBox {
                id: loop_checkbox
                text: "Loop"
                checked: root.loop_enabled
                onCheckedChanged: root.loop_enabled = checked

                ToolTip.visible: hovered
                ToolTip.text: "When checked, range playback repeats automatically"
            }
        }

        // Marker list (8.6, 8.7, 8.8, 8.10, 8.11)
        ColumnLayout {
            id: marker_list_column
            Layout.fillWidth: true
            spacing: 2
            visible: !root.is_recording && root.file_path !== "" && !root.file_not_found && root.markers.length > 0

            // Sorted copy: position markers by position_ms, range markers by start_ms
            property var sorted_markers: {
                let arr = root.markers.slice();
                arr.sort(function(a, b) {
                    let a_time = a.type === "position" ? a.position_ms : a.start_ms;
                    let b_time = b.type === "position" ? b.position_ms : b.start_ms;
                    return a_time - b_time;
                });
                return arr;
            }

            Label {
                text: "Markers"
                font.bold: true
                font.pointSize: 10
            }

            Repeater {
                model: marker_list_column.sorted_markers.length

                RowLayout {
                    id: marker_row

                    required property int index

                    property var marker: index < marker_list_column.sorted_markers.length ? marker_list_column.sorted_markers[index] : null
                    property bool is_position: marker !== null && marker.type === "position"
                    property bool is_active_range: marker !== null && marker.id === root.active_range_id

                    Layout.fillWidth: true
                    spacing: 4

                    // Type indicator
                    Rectangle {
                        width: 8
                        height: 8
                        radius: marker_row.is_position ? 4 : 1
                        color: marker_row.is_position ? "red" : palette.highlight
                        Layout.alignment: Qt.AlignVCenter
                    }

                    // Editable label (8.10)
                    TextInput {
                        id: label_edit
                        text: marker_row.marker !== null ? marker_row.marker.label : ""
                        Layout.preferredWidth: 80
                        Layout.alignment: Qt.AlignVCenter
                        selectByMouse: true
                        color: palette.text
                        selectionColor: palette.highlight
                        selectedTextColor: palette.highlightedText

                        onEditingFinished: {
                            if (marker_row.marker !== null) {
                                root.update_marker_label(marker_row.marker.id, text);
                            }
                        }

                        Rectangle {
                            anchors.fill: parent
                            anchors.margins: -2
                            color: "transparent"
                            border.color: label_edit.activeFocus ? palette.highlight : palette.mid
                            border.width: label_edit.activeFocus ? 1 : 0
                            radius: 2
                            z: -1
                        }
                    }

                    // Time display
                    Label {
                        text: {
                            if (marker_row.marker === null) return "";
                            if (marker_row.is_position) {
                                return root.format_time(marker_row.marker.position_ms);
                            } else {
                                return root.format_time(marker_row.marker.start_ms) + " – " + root.format_time(marker_row.marker.end_ms);
                            }
                        }
                        font.family: "monospace"
                        font.pointSize: 9
                        Layout.alignment: Qt.AlignVCenter
                    }

                    Item { Layout.fillWidth: true }

                    // Seek / Play button (8.7, 8.8)
                    Button {
                        text: marker_row.is_position ? "⏵" : (marker_row.is_active_range ? "⏹" : "⏵")
                        implicitWidth: 32
                        implicitHeight: 28
                        font.pointSize: 10
                        onClicked: {
                            if (marker_row.marker === null) return;
                            if (marker_row.is_position) {
                                root.stop_range_playback();
                                root.seek_to(marker_row.marker.position_ms);
                                // Always start playback from the marker position
                                if (player.playbackState !== MediaPlayer.PlayingState && !seek_resume_timer.running) {
                                    root.visual_position_override = -1;
                                    player.position = marker_row.marker.position_ms;
                                    player.play();
                                }
                                position_save_timer.restart();
                            } else {
                                if (marker_row.is_active_range) {
                                    player.pause();
                                    root.stop_range_playback();
                                } else {
                                    root.play_range(marker_row.marker.id, marker_row.marker.start_ms, marker_row.marker.end_ms);
                                }
                            }
                        }

                        ToolTip.visible: hovered
                        ToolTip.text: marker_row.is_position ? "Seek to this position" : (marker_row.is_active_range ? "Stop range playback" : "Play this range")
                    }

                    // Delete button (8.11)
                    Button {
                        text: "✕"
                        implicitWidth: 28
                        implicitHeight: 28
                        font.pointSize: 9
                        onClicked: {
                            if (marker_row.marker !== null) {
                                root.delete_marker(marker_row.marker.id);
                            }
                        }

                        ToolTip.visible: hovered
                        ToolTip.text: "Delete this marker"
                    }
                }
            }
        }
    }

    // --- Marker management functions (8.2, 8.3, 8.11) ---

    function save_markers() {
        root.markers_json = JSON.stringify(root.markers);
        if (root.recording_uid !== "") {
            SuttaBridge.update_recording_markers(root.recording_uid, root.markers_json);
        }
    }

    function add_position_marker() {
        let new_markers = root.markers.slice();
        new_markers.push({
            "id": "pos_" + Date.now(),
            "type": "position",
            "label": "Mark",
            "position_ms": Math.round(player.position)
        });
        root.markers = new_markers;
        save_markers();
    }

    function add_range_marker(start_ms: int, end_ms: int) {
        let new_markers = root.markers.slice();
        new_markers.push({
            "id": "range_" + Date.now(),
            "type": "range",
            "label": "Range",
            "start_ms": start_ms,
            "end_ms": end_ms
        });
        root.markers = new_markers;
        save_markers();
    }

    function delete_marker(marker_id: string) {
        let new_markers = [];
        for (let i = 0; i < root.markers.length; i++) {
            if (root.markers[i].id !== marker_id) {
                new_markers.push(root.markers[i]);
            }
        }
        root.markers = new_markers;
        // Stop range playback if the active range was deleted
        if (root.active_range_id === marker_id) {
            stop_range_playback();
        }
        save_markers();
    }

    function update_marker_label(marker_id: string, new_label: string) {
        let new_markers = root.markers.slice();
        for (let i = 0; i < new_markers.length; i++) {
            if (new_markers[i].id === marker_id) {
                new_markers[i] = Object.assign({}, new_markers[i], {"label": new_label});
                break;
            }
        }
        root.markers = new_markers;
        save_markers();
    }

    // Range playback (8.8)
    function play_range(marker_id: string, start_ms: int, end_ms: int) {
        root.active_range_id = marker_id;
        root.active_range_start_ms = start_ms;
        root.active_range_end_ms = end_ms;
        root.range_seek_pending = true;
        // Immediately move the visual cursor so the user sees no jump
        // to the old position.
        root.visual_position_override = start_ms;

        if (player.playbackState === MediaPlayer.PlayingState) {
            player.pause();
        }
        player.position = start_ms;
        range_seek_timer.restart();
    }

    function stop_range_playback() {
        root.active_range_id = "";
        root.active_range_start_ms = 0;
        root.active_range_end_ms = 0;
        root.range_seek_pending = false;
        root.visual_position_override = -1;
        range_seek_timer.stop();
        range_playback_timer.stop();
    }

    // Android runtime permission check (6.5)
    function check_microphone_permission(): bool {
        // Qt 6.5+ MicrophonePermission is handled declaratively
        // For now, return true — the actual permission request happens
        // when CaptureSession starts on Android
        return true;
    }

    function start_recording() {
        if (!check_microphone_permission()) {
            return;
        }

        // Re-detect the current default audio input device so that OS-level
        // changes made after the app started are picked up.
        audio_input.device = media_devices.defaultAudioInput;

        // Generate output file path
        let recordings_dir = SuttaBridge.get_chanting_recordings_dir();
        let timestamp = Date.now();
        let output_file = recordings_dir + "/" + root.recording_uid + "_" + timestamp + ".ogg";

        recorder.outputLocation = Qt.resolvedUrl("file://" + output_file);
        root.recording_elapsed_ms = 0;
        root.is_recording = true;
        recorder.record();
    }

    function stop_recording() {
        root.pending_recording_stop = true;
        root.is_recording = false;
        recorder.stop();
        // File finalization happens async — see recorder.onRecorderStateChanged
    }

    function finalize_recording() {
        let recorded_path = recorder.actualLocation.toString().replace("file://", "");
        if (recorded_path !== "") {
            root.file_path = recorded_path;
            root.file_not_found = false;
            root.error_message = "";
            player.source = recorder.actualLocation;
            load_waveform();
            root.recording_completed(recorded_path);
        }
    }
}
