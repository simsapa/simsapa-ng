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
    signal playback_started()
    signal label_edited(string new_label)

    function pause_playback() {
        if (player.playbackState === MediaPlayer.PlayingState) {
            player.pause();
        }
    }

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
    property int waveform_num_bars: 0  // total number of bars stored in cached waveform
    property bool waveform_loading: false
    property bool pending_recording_stop: false

    // Range creation state: "idle", "waiting_start", "waiting_end"
    property string range_create_state: "idle"
    property int range_create_start_ms: -1

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

    // Parse waveform JSON, handling both new object format and legacy plain array
    function parse_waveform_json(json_str: string) {
        let parsed = JSON.parse(json_str);
        if (Array.isArray(parsed)) {
            // Legacy format: plain array of peaks, derive num_bars from length
            root.waveform_data = parsed;
            root.waveform_num_bars = parsed.length;
        } else if (parsed && typeof parsed === "object" && Array.isArray(parsed.peaks)) {
            root.waveform_data = parsed.peaks;
            root.waveform_num_bars = parsed.num_bars || parsed.peaks.length;
        } else {
            root.waveform_data = [];
            root.waveform_num_bars = 0;
        }
    }

    function load_waveform() {
        if (root.file_path === "" || root.file_not_found) {
            root.waveform_data = [];
            root.waveform_num_bars = 0;
            root.waveform_loading = false;
            return;
        }

        // Use cached data from database if available
        if (root.waveform_json !== "" && root.waveform_json !== "[]") {
            try {
                parse_waveform_json(root.waveform_json);
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
                parse_waveform_json(waveform_json);
            } catch (e) {
                root.waveform_data = [];
                root.waveform_num_bars = 0;
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

        onPlaybackStateChanged: {
            if (playbackState === MediaPlayer.PlayingState) {
                root.playback_started();
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

        // Header row with label, duration, edit and close button (6.7)
        RowLayout {
            Layout.fillWidth: true
            spacing: 8

            Label {
                id: title_label
                text: root.label + (player.duration > 0 ? " (" + root.format_time(player.duration) + ")" : "")
                font.bold: true
                elide: Text.ElideRight
                Layout.fillWidth: true
                visible: !title_edit_field.visible
            }

            TextField {
                id: title_edit_field
                visible: false
                text: root.label
                font.bold: true
                Layout.fillWidth: true
                onAccepted: {
                    root.label = text;
                    root.label_edited(text);
                    visible = false;
                }
                Keys.onEscapePressed: {
                    text = root.label;
                    visible = false;
                }
                onActiveFocusChanged: {
                    if (!activeFocus && visible) {
                        text = root.label;
                        visible = false;
                    }
                }
            }

            Button {
                icon.source: "icons/32x32/fa_pen-to-square-solid.png"
                Layout.preferredWidth: 24
                flat: true
                visible: !title_edit_field.visible
                onClicked: {
                    title_edit_field.text = root.label;
                    title_edit_field.visible = true;
                    title_edit_field.forceActiveFocus();
                    title_edit_field.selectAll();
                }
                ToolTip.visible: hovered
                ToolTip.text: "Edit title"
            }

            Button {
                icon.source: "icons/32x32/mdi--close.png"
                Layout.preferredWidth: 24
                flat: true
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

        // Recording state indicator
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

            // Quick seek buttons
            Button {
                text: "-5s"
                enabled: !root.is_recording && root.file_path !== "" && !root.file_not_found
                implicitWidth: 40
                onClicked: {
                    let new_pos = Math.max(0, root.effective_position - 5000);
                    if (player.playbackState === MediaPlayer.PlayingState) {
                        player.position = new_pos;
                    } else {
                        root.seek_to(new_pos);
                    }
                    position_save_timer.restart();
                }
            }

            Button {
                text: "+5s"
                enabled: !root.is_recording && root.file_path !== "" && !root.file_not_found
                implicitWidth: 40
                onClicked: {
                    let max_pos = player.duration > 0 ? player.duration : root.effective_position;
                    let new_pos = Math.min(max_pos, root.effective_position + 5000);
                    if (player.playbackState === MediaPlayer.PlayingState) {
                        player.position = new_pos;
                    } else {
                        root.seek_to(new_pos);
                    }
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
            range_create_active: root.range_create_state !== "idle"
            range_create_pending_ms: root.range_create_start_ms

            onSeek_requested: function(position_ms) {
                if (root.range_create_state === "waiting_start") {
                    root.range_create_start_ms = position_ms;
                    root.range_create_state = "waiting_end";
                } else if (root.range_create_state === "waiting_end") {
                    let start = Math.min(root.range_create_start_ms, position_ms);
                    let end = Math.max(root.range_create_start_ms, position_ms);
                    root.add_range_marker(start, end);
                    root.range_create_state = "idle";
                    root.range_create_start_ms = -1;
                } else {
                    root.stop_range_playback();
                    root.seek_to(position_ms);
                    position_save_timer.restart();
                }
            }

            onRange_selected: function(start_ms, end_ms) {
                if (root.range_create_state !== "idle") {
                    // During range creation, ignore drag-based range selection
                    return;
                }
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
                id: range_create_button
                text: root.range_create_state === "idle" ? "＋ Range"
                    : root.range_create_state === "waiting_start" ? "Set Start"
                    : "Set End"
                enabled: player.duration > 0
                highlighted: root.range_create_state !== "idle"
                onClicked: {
                    if (root.range_create_state === "idle") {
                        root.range_create_state = "waiting_start";
                        root.range_create_start_ms = -1;
                    } else {
                        // Cancel range creation on button click
                        root.range_create_state = "idle";
                        root.range_create_start_ms = -1;
                    }
                }

                ToolTip.visible: hovered
                ToolTip.text: root.range_create_state === "idle"
                    ? "Click to start creating a range by clicking on the waveform"
                    : root.range_create_state === "waiting_start"
                    ? "Click on the waveform to set the range start (or click here to cancel)"
                    : "Click on the waveform to set the range end (or click here to cancel)"
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

            Button {
                id: resample_button
                text: "Resample"
                enabled: root.waveform_data.length > 0 && !root.waveform_loading && player.duration > 0
                onClicked: {
                    // Calculate current samples per second from num_bars and duration
                    let duration_secs = player.duration / 1000.0;
                    let current_sps = duration_secs > 0
                        ? Math.round(root.waveform_num_bars / duration_secs)
                        : 10;
                    resample_dialog.current_samples_per_second = current_sps;
                    resample_spinbox.value = current_sps;
                    resample_dialog.open();
                }

                ToolTip.visible: hovered
                ToolTip.text: "Regenerate waveform with a different sample rate"
            }
        }

        // Resample dialog
        Dialog {
            id: resample_dialog
            title: "Resample Waveform"
            parent: Overlay.overlay
            anchors.centerIn: parent
            modal: true
            standardButtons: Dialog.Ok | Dialog.Cancel

            property int current_samples_per_second: 0

            ColumnLayout {
                spacing: 12

                Label {
                    text: "Current: " + resample_dialog.current_samples_per_second + " samples/sec"
                        + " (" + root.waveform_num_bars + " total)"
                    wrapMode: Text.Wrap
                }

                RowLayout {
                    spacing: 8

                    Label { text: "New rate:" }

                    SpinBox {
                        id: resample_spinbox
                        from: 1
                        to: 500
                        stepSize: 1
                        value: 10
                        editable: true
                    }

                    Label { text: "samples/sec" }
                }

                Label {
                    text: "Higher values show more detail but use more memory."
                    font.pointSize: 9
                    color: palette.placeholderText
                    wrapMode: Text.Wrap
                }
            }

            onAccepted: {
                let sps = resample_spinbox.value;
                let duration_secs = player.duration / 1000.0;
                let num_bars = Math.max(10, Math.round(sps * duration_secs));
                root.waveform_loading = true;
                root.waveform_data = [];
                root.waveform_num_bars = 0;
                SuttaBridge.generate_waveform_data(root.recording_uid, root.file_path, num_bars);
            }
        }

        // Marker time edit dialog
        Dialog {
            id: marker_time_dialog
            title: marker_time_dialog.is_position ? "Edit Position" : "Edit Range"
            standardButtons: Dialog.Ok | Dialog.Cancel
            anchors.centerIn: parent
            modal: true

            property string marker_id: ""
            property bool is_position: true

            function set_time_fields(min_spin: SpinBox, sec_spin: SpinBox, ms_spin: SpinBox, total_ms: int) {
                let total_secs = Math.floor(total_ms / 1000);
                min_spin.value = Math.floor(total_secs / 60);
                sec_spin.value = total_secs % 60;
                ms_spin.value = total_ms % 1000;
            }

            function fields_to_ms(min_spin: SpinBox, sec_spin: SpinBox, ms_spin: SpinBox): int {
                return (min_spin.value * 60 + sec_spin.value) * 1000 + ms_spin.value;
            }

            ColumnLayout {
                spacing: 12

                // Position marker fields
                RowLayout {
                    visible: marker_time_dialog.is_position
                    spacing: 4

                    Label { text: "Position:" }
                    SpinBox { id: pos_min_spin; from: 0; to: 999; editable: true; implicitWidth: 80 }
                    Label { text: "m" }
                    SpinBox { id: pos_sec_spin; from: 0; to: 59; editable: true; implicitWidth: 80 }
                    Label { text: "s" }
                    SpinBox { id: pos_ms_spin; from: 0; to: 999; editable: true; implicitWidth: 90 }
                    Label { text: "ms" }
                }

                // Range marker fields
                RowLayout {
                    visible: !marker_time_dialog.is_position
                    spacing: 4

                    Label { text: "Start:" }
                    SpinBox { id: range_start_min_spin; from: 0; to: 999; editable: true; implicitWidth: 80 }
                    Label { text: "m" }
                    SpinBox { id: range_start_sec_spin; from: 0; to: 59; editable: true; implicitWidth: 80 }
                    Label { text: "s" }
                    SpinBox { id: range_start_ms_spin; from: 0; to: 999; editable: true; implicitWidth: 90 }
                    Label { text: "ms" }
                }

                RowLayout {
                    visible: !marker_time_dialog.is_position
                    spacing: 4

                    Label { text: "End:  " }
                    SpinBox { id: range_end_min_spin; from: 0; to: 999; editable: true; implicitWidth: 80 }
                    Label { text: "m" }
                    SpinBox { id: range_end_sec_spin; from: 0; to: 59; editable: true; implicitWidth: 80 }
                    Label { text: "s" }
                    SpinBox { id: range_end_ms_spin; from: 0; to: 999; editable: true; implicitWidth: 90 }
                    Label { text: "ms" }
                }

                Label { text: "Comment:" }

                ScrollView {
                    Layout.fillWidth: true
                    Layout.preferredHeight: 80

                    TextArea {
                        id: marker_comment_field
                        placeholderText: "Add a comment..."
                        wrapMode: TextEdit.Wrap
                    }
                }
            }

            onAccepted: {
                if (marker_time_dialog.is_position) {
                    let ms = marker_time_dialog.fields_to_ms(pos_min_spin, pos_sec_spin, pos_ms_spin);
                    let max_ms = player.duration > 0 ? player.duration : ms;
                    root.update_marker_time(marker_time_dialog.marker_id, "position_ms", Math.min(max_ms, Math.max(0, ms)));
                } else {
                    let start = marker_time_dialog.fields_to_ms(range_start_min_spin, range_start_sec_spin, range_start_ms_spin);
                    let end = marker_time_dialog.fields_to_ms(range_end_min_spin, range_end_sec_spin, range_end_ms_spin);
                    // Ensure correct order
                    let actual_start = Math.min(start, end);
                    let actual_end = Math.max(start, end);
                    let max_ms = player.duration > 0 ? player.duration : actual_end;
                    root.update_marker_time(marker_time_dialog.marker_id, "start_ms", Math.max(0, actual_start));
                    root.update_marker_time(marker_time_dialog.marker_id, "end_ms", Math.min(max_ms, actual_end));
                }
                root.update_marker_field(marker_time_dialog.marker_id, "comment", marker_comment_field.text);
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

                Rectangle {
                    id: marker_row_bg

                    required property int index

                    Layout.fillWidth: true
                    implicitHeight: marker_row.implicitHeight + 8
                    color: index % 2 === 0 ? "transparent" : Qt.rgba(palette.text.r, palette.text.g, palette.text.b, 0.06)
                    radius: 4

                    ColumnLayout {
                        id: marker_row

                        property var marker: marker_row_bg.index < marker_list_column.sorted_markers.length ? marker_list_column.sorted_markers[marker_row_bg.index] : null
                        property bool is_position: marker !== null && marker.type === "position"
                        property bool is_active_range: marker !== null && marker.id === root.active_range_id

                        anchors.fill: parent
                        anchors.margins: 4
                        spacing: 2

                    // Top row: play, indicator, label, time, delete
                    RowLayout {
                        Layout.fillWidth: true
                        spacing: 4

                        // Seek / Play / Pause button
                        Button {
                            property bool is_playing_this: marker_row.is_position
                                ? (player.playbackState === MediaPlayer.PlayingState && root.active_range_id === "")
                                : marker_row.is_active_range && player.playbackState === MediaPlayer.PlayingState

                            text: is_playing_this ? "⏸" : "▶"
                            implicitWidth: 32
                            implicitHeight: 28
                            font.pointSize: 10
                            flat: true
                            onClicked: {
                                if (marker_row.marker === null) return;
                                if (marker_row.is_position) {
                                    if (is_playing_this) {
                                        player.pause();
                                        position_save_timer.restart();
                                    } else {
                                        root.stop_range_playback();
                                        root.seek_to(marker_row.marker.position_ms);
                                        root.visual_position_override = -1;
                                        player.position = marker_row.marker.position_ms;
                                        player.play();
                                        position_save_timer.restart();
                                    }
                                } else {
                                    if (is_playing_this) {
                                        player.pause();
                                    } else if (marker_row.is_active_range) {
                                        // Range is active but paused — resume
                                        player.play();
                                    } else {
                                        root.play_range(marker_row.marker.id, marker_row.marker.start_ms, marker_row.marker.end_ms);
                                    }
                                }
                            }

                            ToolTip.visible: hovered
                            ToolTip.text: is_playing_this ? "Pause" : (marker_row.is_position ? "Play from this position" : "Play this range")
                        }

                        // Type indicator
                        Rectangle {
                            width: 8
                            height: 8
                            radius: marker_row.is_position ? 4 : 1
                            color: marker_row.is_position ? "red" : palette.highlight
                            Layout.alignment: Qt.AlignVCenter
                        }

                        // Editable label
                        // NOTE: Not using the editable feature now, just a plain label.
                        // TextInput {
                        //     id: label_edit
                        //     text: marker_row.marker !== null ? marker_row.marker.label : ""
                        //     Layout.preferredWidth: 80
                        //     Layout.alignment: Qt.AlignVCenter
                        //     selectByMouse: true
                        //     color: palette.text
                        //     selectionColor: palette.highlight
                        //     selectedTextColor: palette.highlightedText

                        //     onEditingFinished: {
                        //         if (marker_row.marker !== null) {
                        //             root.update_marker_label(marker_row.marker.id, text);
                        //         }
                        //     }

                        //     Rectangle {
                        //         anchors.fill: parent
                        //         anchors.margins: -2
                        //         color: "transparent"
                        //         border.color: label_edit.activeFocus ? palette.highlight : palette.mid
                        //         border.width: label_edit.activeFocus ? 1 : 0
                        //         radius: 2
                        //         z: -1
                        //     }
                        // }

                        // Mark / Range label
                        Label {
                            text: marker_row.marker !== null ? marker_row.marker.label : ""
                            Layout.preferredWidth: 80
                            Layout.alignment: Qt.AlignVCenter
                            color: palette.text
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

                        // Delete button
                        Button {
                            icon.source: "icons/32x32/ion--trash-outline.png"
                            implicitWidth: 28
                            implicitHeight: 28
                            flat: true
                            onClicked: {
                                if (marker_row.marker !== null) {
                                    root.delete_marker(marker_row.marker.id);
                                }
                            }

                            ToolTip.visible: hovered
                            ToolTip.text: "Delete this marker"
                        }
                    }

                    // Adjust buttons row
                    RowLayout {
                        Layout.fillWidth: true
                        Layout.leftMargin: 12
                        spacing: 4
                        visible: marker_row.marker !== null

                        // Edit button — opens precise time input dialog
                        Button {
                            icon.source: "icons/32x32/fa_pen-to-square-solid.png"
                            implicitWidth: 28
                            implicitHeight: 24
                            flat: true
                            onClicked: {
                                if (marker_row.marker === null) return;
                                marker_time_dialog.marker_id = marker_row.marker.id;
                                marker_time_dialog.is_position = marker_row.is_position;
                                if (marker_row.is_position) {
                                    marker_time_dialog.set_time_fields(pos_min_spin, pos_sec_spin, pos_ms_spin, marker_row.marker.position_ms);
                                } else {
                                    marker_time_dialog.set_time_fields(range_start_min_spin, range_start_sec_spin, range_start_ms_spin, marker_row.marker.start_ms);
                                    marker_time_dialog.set_time_fields(range_end_min_spin, range_end_sec_spin, range_end_ms_spin, marker_row.marker.end_ms);
                                }
                                marker_comment_field.text = marker_row.marker.comment !== undefined ? marker_row.marker.comment : "";
                                marker_time_dialog.open();
                            }
                            ToolTip.visible: hovered
                            ToolTip.text: "Edit timing and comment"
                        }

                        // Position marker: single -1s / +1s pair
                        Button {
                            icon.source: "icons/32x32/fa_angle-left-solid.png"
                            text: "-1s"
                            implicitHeight: 24
                            flat: true
                            visible: marker_row.is_position
                            onClicked: {
                                if (marker_row.marker === null) return;
                                root.update_marker_time(marker_row.marker.id, "position_ms", Math.max(0, marker_row.marker.position_ms - 1000));
                            }
                        }
                        Button {
                            icon.source: "icons/32x32/fa_angle-right-solid.png"
                            text: "+1s"
                            implicitHeight: 24
                            flat: true
                            visible: marker_row.is_position
                            LayoutMirroring.enabled: true
                            onClicked: {
                                if (marker_row.marker === null) return;
                                let max_ms = player.duration > 0 ? player.duration : marker_row.marker.position_ms;
                                root.update_marker_time(marker_row.marker.id, "position_ms", Math.min(max_ms, marker_row.marker.position_ms + 1000));
                            }
                        }

                        // Range marker: start -1s / +1s, then end -1s / +1s
                        Label {
                            text: "Start:"
                            font.pointSize: 8
                            visible: !marker_row.is_position
                            Layout.alignment: Qt.AlignVCenter
                        }
                        Button {
                            icon.source: "icons/32x32/fa_angle-left-solid.png"
                            text: "-1s"
                            implicitHeight: 24
                            flat: true
                            visible: !marker_row.is_position
                            onClicked: {
                                if (marker_row.marker === null) return;
                                root.update_marker_time(marker_row.marker.id, "start_ms", Math.max(0, marker_row.marker.start_ms - 1000));
                            }
                        }
                        Button {
                            icon.source: "icons/32x32/fa_angle-right-solid.png"
                            text: "+1s"
                            implicitHeight: 24
                            flat: true
                            visible: !marker_row.is_position
                            LayoutMirroring.enabled: true
                            onClicked: {
                                if (marker_row.marker === null) return;
                                let max_ms = marker_row.marker.end_ms - 100;
                                root.update_marker_time(marker_row.marker.id, "start_ms", Math.min(max_ms, marker_row.marker.start_ms + 1000));
                            }
                        }

                        Label {
                            text: "End:"
                            font.pointSize: 8
                            visible: !marker_row.is_position
                            Layout.leftMargin: 8
                            Layout.alignment: Qt.AlignVCenter
                        }
                        Button {
                            icon.source: "icons/32x32/fa_angle-left-solid.png"
                            text: "-1s"
                            implicitHeight: 24
                            flat: true
                            visible: !marker_row.is_position
                            onClicked: {
                                if (marker_row.marker === null) return;
                                let min_ms = marker_row.marker.start_ms + 100;
                                root.update_marker_time(marker_row.marker.id, "end_ms", Math.max(min_ms, marker_row.marker.end_ms - 1000));
                            }
                        }
                        Button {
                            icon.source: "icons/32x32/fa_angle-right-solid.png"
                            text: "+1s"
                            implicitHeight: 24
                            flat: true
                            visible: !marker_row.is_position
                            LayoutMirroring.enabled: true
                            onClicked: {
                                if (marker_row.marker === null) return;
                                let max_ms = player.duration > 0 ? player.duration : marker_row.marker.end_ms;
                                root.update_marker_time(marker_row.marker.id, "end_ms", Math.min(max_ms, marker_row.marker.end_ms + 1000));
                            }
                        }

                        Item { Layout.fillWidth: true }
                    }

                    // Comment display row
                    Label {
                        Layout.fillWidth: true
                        Layout.leftMargin: 12
                        visible: marker_row.marker !== null
                            && marker_row.marker.comment !== undefined
                            && marker_row.marker.comment !== ""
                        text: marker_row.marker !== null && marker_row.marker.comment !== undefined ? marker_row.marker.comment : ""
                        wrapMode: Text.Wrap
                        font.pointSize: 9
                        color: palette.placeholderText
                    }
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
            "comment": "",
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
            "comment": "",
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

    function update_marker_time(marker_id: string, field: string, value_ms: int) {
        let new_markers = root.markers.slice();
        let update = {};
        update[field] = value_ms;
        for (let i = 0; i < new_markers.length; i++) {
            if (new_markers[i].id === marker_id) {
                new_markers[i] = Object.assign({}, new_markers[i], update);
                break;
            }
        }
        root.markers = new_markers;
        save_markers();
    }

    function update_marker_field(marker_id: string, field: string, value: string) {
        let new_markers = root.markers.slice();
        let update = {};
        update[field] = value;
        for (let i = 0; i < new_markers.length; i++) {
            if (new_markers[i].id === marker_id) {
                new_markers[i] = Object.assign({}, new_markers[i], update);
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

    // Permission helper (instantiated once per RecordingPlaybackItem)
    AssetManager { id: permission_manager }

    // Tracks whether we're waiting for the async permission result
    property bool permission_requested: false

    // Android runtime permission check (6.5)
    function check_microphone_permission(): bool {
        let status = permission_manager.check_microphone_permission();
        if (status === "granted") {
            return true;
        }
        if (status === "undetermined" && !root.permission_requested) {
            root.permission_requested = true;
            permission_manager.request_microphone_permission();
            root.error_message = "Microphone permission requested. Please grant it and tap Record again.";
            return false;
        }
        root.error_message = "Microphone permission denied. Please enable it in Android Settings > Apps > Simsapa > Permissions.";
        return false;
    }

    function start_recording() {
        if (!check_microphone_permission()) {
            return;
        }

        root.error_message = "";

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
            // Clear cached waveform so it regenerates from the new file
            root.waveform_json = "";
            root.waveform_data = [];
            root.waveform_num_bars = 0;
            load_waveform();
            root.recording_completed(recorded_path);
        }
    }
}
