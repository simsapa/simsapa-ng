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
        if (root.is_recording) {
            stop_recording();
        }
        if (player.playbackState === MediaPlayer.PlayingState) {
            player.stop();
        }
        save_position();
        if (root.recording_uid !== "") {
            SuttaBridge.update_recording_volume(root.recording_uid, root.volume);
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
                    player.stop();
                    position_save_timer.restart();
                }
            }

            Item { Layout.fillWidth: true }

            // Time display (6.3)
            Label {
                text: root.format_time(player.position) + " / " + root.format_time(player.duration)
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
            playback_position_ms: player.position
            is_playing: player.playbackState === MediaPlayer.PlayingState
            markers: root.markers

            onSeek_requested: function(position_ms) {
                player.position = position_ms;
                position_save_timer.restart();
            }

            onRange_selected: function(start_ms, end_ms) {
                // Add a new range marker
                let new_markers = root.markers.slice();
                let new_id = "range_" + Date.now();
                new_markers.push({
                    "id": new_id,
                    "type": "range",
                    "label": "Range",
                    "start_ms": start_ms,
                    "end_ms": end_ms
                });
                root.markers = new_markers;
                root.markers_json = JSON.stringify(new_markers);
                if (root.recording_uid !== "") {
                    SuttaBridge.update_recording_markers(root.recording_uid, root.markers_json);
                }
            }
        }

        // Scrubber slider (6.3) — hidden when file not found
        Slider {
            id: scrubber
            Layout.fillWidth: true
            visible: !root.is_recording && root.file_path !== "" && !root.file_not_found
            from: 0
            to: player.duration > 0 ? player.duration : 1
            value: player.position

            onMoved: {
                player.position = value;
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
