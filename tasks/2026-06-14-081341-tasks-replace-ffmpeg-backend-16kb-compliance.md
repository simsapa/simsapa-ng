# Tasks: Replace Qt's FFmpeg Media Backend with a Pure-Rust Audio Stack (16 KB Compliance)

Based on PRD: [2026-06-14-081341-prd---replace-ffmpeg-backend-16kb-compliance.md](./2026-06-14-081341-prd---replace-ffmpeg-backend-16kb-compliance.md)
Rejected alternative (reference): [2026-06-14-081341-reference-compiled-ffmpeg-16kb-alternative.md](./2026-06-14-081341-reference---compiled-ffmpeg-16kb-alternative.md)

> Each top-level task is a stage: after completing it, the app should compile
> (`make build -B`) and relevant tests should pass. New Rust modules (1–2) and
> the bridge (3) are added *before* anything is removed; the QML swap (4) flips
> usage to the new stack; only then is Qt Multimedia removed (5).

## Relevant Files

- `backend/Cargo.toml` - Add `cpal`, `flacenc`, a resampler (`rubato`), and the Android `oboe` path; existing `symphonia` is reused for decode.
- `backend/src/audio/mod.rs` - New audio module root (declares `recorder`, `player`, shared format constants).
- `backend/src/audio/recorder.rs` - cpal input capture → canonical PCM → FLAC encode (`flacenc`).
- `backend/src/audio/player.rs` - symphonia decode → cpal output stream with sample-counted position, seek, range/loop, volume.
- `backend/src/audio/format.rs` - Canonical recording format constants (mono, 16-bit, 48 kHz) + resample/downmix helpers.
- `backend/src/lib.rs` - Register the new `audio` module.
- `backend/src/waveform.rs` - Verify FLAC decodes; no functional change expected.
- `backend/tests/test_audio_recorder.rs` - Unit/integration tests for capture→FLAC (encode a synthetic buffer, decode back with symphonia).
- `backend/tests/test_audio_player.rs` - Tests for decode/seek/position math and range/loop boundary logic.
- `bridges/src/audio_manager.rs` - New CXX-Qt bridge: instantiable `AudioManager` QObject exposing record/playback to QML, with signals. Invokables include `clear_range()` (added in Task 4 so a normal seek/stop can reset an active range).
- `bridges/build.rs` - Register `src/audio_manager.rs` in `rust_files`; remove no qml entries (RecordingPlaybackItem stays).
- `assets/qml/com/profoundlabs/simsapa/AudioManager.qml` - `qmllint` type stub for the new bridge.
- `assets/qml/com/profoundlabs/simsapa/qmldir` - Declare `AudioManager 1.0 AudioManager.qml` (non-singleton, like `AssetManager`).
- `assets/qml/RecordingPlaybackItem.qml` - Remove `QtMultimedia`; drive recording/playback via `AudioManager`; record to `.flac`.
- `CMakeLists.txt` - Remove `Multimedia` from `CXXQT_QTCOMPONENTS` and `Qt::Multimedia` from `target_link_libraries` (Task 5).
- `android/AndroidManifest.xml` - Keep `RECORD_AUDIO`; confirm no Qt-multimedia-specific entries are required.
- `android/build.gradle` - Pin `androidNdkVersion` to r28+.
- `.cargo/config.toml` - Any Android linker flags needed for cpal/oboe (if not handled by NDK default).
- `docs/pure-rust-audio-backend.md` - New doc: architecture, crates, threading, format, permissions, 16 KB verification.
- `PROJECT_MAP.md` - Update chanting/audio entries.
- `AGENTS.md` (CLAUDE.md symlink target) - Update the "Android 16 KB compatibility" note (FFmpeg gap resolved).
- `/home/gambhiro/.claude/.../memory/project_android_16kb_warning_test_deploy.md` - Update memory note (FFmpeg removed).

### Notes

- Edit `AGENTS.md`, **not** `CLAUDE.md` (symlink; writes through it are refused).
- Build with `make build -B` (not direct cmake). Run Rust tests with `cd backend && cargo test`. Skip `make qml-test` unless asked. Only run tests after all sub-tasks of a top-level task are done.
- Do **not** GUI-test the app as an agent; leave runtime smoke tests to the user.
- The `oboe` crate compiles Oboe C++ from source with the project NDK → linked into the existing `staticlib` → main app `.so` (no separate prebuilt; 16 KB-aligned with NDK r28+).

## Instructions for Completing Tasks

**IMPORTANT:** As each sub-task is completed, change `- [ ]` to `- [x]` in this file. Update after each sub-task, not just each parent task.

## Tasks

---

### Specs for Task 1 (Recorder)

- **Canonical format:** mono, 16-bit PCM, 48 kHz, encoded to FLAC (`flacenc`). All recordings normalized to this regardless of device capabilities.
- **API (Rust, backend):** `Recorder::start(output_flac_path) -> Result<Recorder>`, `Recorder::stop(self) -> Result<()>` (finalizes FLAC). Capture runs on the cpal callback thread; errors surfaced via a channel/callback.
- **Dependencies:** `cpal` (capture), `flacenc` (encode), `rubato` (resample) — `symphonia` already present (used in Task 6 verification).
- **Reuse:** recordings directory comes from the existing `get_chanting_recordings_dir()` (bridge) — recorder itself just takes a path.

- [x] 1.0 Implement the Rust audio **capture + FLAC encoding** module (recorder)
  - [x] 1.1 Add `cpal`, `flacenc`, and `rubato` to `backend/Cargo.toml` (host deps); confirm `make build -B` still links on desktop.
  - [x] 1.2 Create `backend/src/audio/mod.rs` + `backend/src/audio/format.rs` defining the canonical format constants and a `to_canonical(samples, in_rate, in_channels) -> Vec<i16>` downmix/resample helper.
  - [x] 1.3 Register the `audio` module in `backend/src/lib.rs`.
  - [x] 1.4 Implement `backend/src/audio/recorder.rs`: open default input device via cpal, capture f32/i16 frames, convert to canonical PCM, accumulate (or stream) to a `flacenc` encoder, write the `.flac` file on stop. Run capture off the caller thread; expose `start`/`stop` and an error signal channel.
  - [x] 1.5 Handle device-config edge cases (input sample rate/channel count ≠ canonical → resample/downmix; if no input device, return a clear error).
  - [x] 1.6 Add `backend/tests/test_audio_recorder.rs`: feed a synthetic sine PCM buffer through the encode path, write a `.flac`, then decode it back with `symphonia` and assert sample-rate/channels/non-empty/roughly-correct duration. (Per local-integration-test guidance, do not `#[ignore]`.)
  - [x] 1.7 Run `cd backend && cargo test` for the recorder module; confirm `make build -B` is clean.

---

### Specs for Task 2 (Player)

- **API (Rust, backend):** `Player::load(path) -> Result<Player>`; `play()`, `pause()`, `stop()`, `seek_ms(pos)`, `set_volume(v: f32)`, `position_ms() -> i64`, `duration_ms() -> i64`, `play_range(start_ms, end_ms, looping: bool)`. Output via a cpal stream fed by symphonia decode.
- **Position/seek:** sample-counted from the output stream (interpolated between callbacks) for sub-100 ms accuracy — needed for the waveform playhead and range-loop end detection. (Decide: `rodio_playback_position`-style wrapper vs. custom frame-counting cpal output — Open Question #2.)
- **State:** expose a player-state enum (`Stopped`/`Playing`/`Paused`) — the QML has many `MediaPlayer.PlayingState` checks that will map to this.
- **Decode coverage (IMPORTANT):** the player must decode **MP3** as well as FLAC. The app **ships reference recordings as `.mp3`** (`cli/src/bootstrap/chanting_practice.rs` → `recordings/namo-tassa.mp3`, and `app-assets/chanting-recordings/itipiso-sumedharama-2022.mp3`). Only *user* recordings are FLAC; reference playback must keep working. `symphonia` `features = ["all"]` already decodes MP3 (the existing waveform renders for these reference files), so no new decode dependency — but the player must not assume FLAC input.
- **Reuse:** decoding reuses the existing `symphonia` setup pattern from `backend/src/waveform.rs` (probe by extension, default track, decoder).

- [x] 2.0 Implement the Rust audio **playback engine** (decode + cpal output, position, seek, duration, volume, range/loop)
  - [x] 2.1 Implement `backend/src/audio/player.rs` skeleton: load file (symphonia probe/decoder reusing the `waveform.rs` pattern), open a cpal output stream, define the state enum and a shared playback state (`Arc<Mutex<…>>` / atomics for position + volume + range).
  - [x] 2.2 Implement decode→output feeding loop: pull decoded frames, apply volume, write to the output stream; track played frames as the position source of truth.
  - [x] 2.3 Implement `play`/`pause`/`stop` and `seek_ms` (seek the symphonia stream / reset the frame counter; guard against premature end-detection right after a seek, mirroring the existing `range_seek_pending` concept).
  - [x] 2.4 Implement `play_range(start_ms, end_ms, looping)`: start playback at `start_ms`, stop or loop back to `start_ms` when `position >= end_ms`.
  - [x] 2.5 Implement `position_ms()` / `duration_ms()` and a mechanism to push periodic position updates (poll target for the bridge's position signal).
  - [x] 2.6 Add `backend/tests/test_audio_player.rs`: cover position/duration math, seek target rounding, and range/loop boundary logic against a short generated FLAC fixture (created via the Task 1 encode path).
  - [x] 2.7 Run `cd backend && cargo test`; confirm `make build -B` is clean.

---

### Specs for Task 3 (Bridge)

- **Type:** a new **instantiable** (non-singleton) QObject `AudioManager`, one instance per `RecordingPlaybackItem` (mirrors per-item `MediaPlayer`+`MediaRecorder`). Registered like `AssetManager` (`rust_files` in `build.rs`, `qmldir` line, `qmllint` stub).
- **Invokables:** `start_recording(output_path)`, `stop_recording()`, `load(path)`, `play()`, `pause()`, `stop()`, `seek(position_ms)`, `set_volume(v)`, `play_range(start_ms, end_ms, looping)`.
- **Properties:** `state` (int/enum), `position_ms` (int), `duration_ms` (int).
- **Signals:** `state_changed`, `position_changed(position_ms)`, `duration_changed(duration_ms)`, `recording_finished(file_path)`, `error_occurred(message)`.
- **Threading:** audio threads marshal to Qt via `self.qt_thread().queue(move |qo| qo.emit_…())` — the exact pattern used by `generate_waveform_data`/`waveform_data_ready` in `sutta_bridge.rs`. `AssetManager` is the template: `#[qml_element]` + `impl cxx_qt::Threading`.
- **Lifecycle (IMPORTANT):** the chanting review window holds **multiple** `RecordingPlaybackItem`s at once (a `Repeater` of new recordings + reference + user loaders), so multiple `AudioManager` instances coexist. Each must release its cpal stream(s) and audio thread on destruction (Rust `Drop`), and should create the output stream lazily on first play (not at construction) to avoid many idle streams. Recording uses the exclusive mic input — only one item records at a time (UI-enforced via sibling `cleanup()`/`pause_playback()`).
- **Depends on:** Tasks 1 & 2 (backend `audio::recorder` / `audio::player`).

- [x] 3.0 Expose the recorder and player to QML through the **CXX-Qt bridge**
  - [x] 3.1 Create `bridges/src/audio_manager.rs` with the `AudioManager` QObject: properties, `#[qsignal]`s, and `#[qinvokable]`s per the spec; hold the backend `Recorder`/`Player` instances.
  - [x] 3.2 Wire backend callbacks → Qt signals using the `qt_thread().queue()` marshalling pattern (position ticks, state changes, recording-finished, errors). Ensure no QObject access happens on the audio thread.
  - [x] 3.3 Register `src/audio_manager.rs` in the `rust_files` list in `bridges/build.rs`.
  - [x] 3.4 Create `assets/qml/com/profoundlabs/simsapa/AudioManager.qml` `qmllint` stub with matching function signatures, the state-enum constants (`Stopped`/`Playing`/`Paused`), and dummy return values; add `AudioManager 1.0 AudioManager.qml` to `qmldir`.
  - [x] 3.5 Implement `Drop` for the recorder/player so cpal streams and audio threads are torn down when an `AudioManager` is destroyed; create the output stream lazily on first `play()`/`load()`.
  - [x] 3.6 Confirm `make build -B` is clean (bridge compiles and the QML module registers) without yet using `AudioManager` from any window.

---

### Specs for Task 4 (QML refactor)

- **Target file:** `assets/qml/RecordingPlaybackItem.qml` (only audio UI surface; hosted by `ChantingPracticeReviewWindow.qml` / `ChantingPracticeWindow.qml`).
- **Remove:** `import QtMultimedia`; `MediaPlayer`, `AudioOutput`, `CaptureSession`, `AudioInput`, `MediaRecorder`, `MediaDevices`.
- **Add:** one `AudioManager { id: audio }` instance; map `player.playbackState === MediaPlayer.PlayingState` → `audio.state === AudioManager.Playing` (define matching enum values in the stub), `player.position` → `audio.position_ms`, `player.duration` → `audio.duration_ms`.
- **Behaviour to preserve:** record/stop, play/pause/stop, waveform-click seek, saved-position restore, range create, range playback + loop, volume, file-not-found handling, error messages.
- **Simplify, don't blindly port (IMPORTANT):** several QML constructs exist **only** to work around Qt/FFmpeg `MediaPlayer` quirks and should be *removed*, not reimplemented, because the Rust player seeks deterministically and owns range/loop:
  - `seek_to()`'s pause→seek→`seek_resume_timer`→resume dance (Qt "ignores a bare `position = X`") → replace with a direct `audio.seek()`.
  - `visual_position_override` / `effective_position` (compensating for unreliable `player.position` after async seek while paused) → the Rust player reports an accurate position immediately after seek, so this can likely be dropped.
  - `range_playback_timer` (50 ms QML polling for `position >= end_ms`) and `range_seek_timer` / `range_seek_pending` → the Rust `play_range(start, end, looping)` handles boundary + loop and emits on stop/loop; QML only needs the position for the playhead.
- **Logging:** use the `Logger` module, not `console` (per project rules).

- [x] 4.0 Refactor **`RecordingPlaybackItem.qml`** to the Rust audio stack
  - [x] 4.1 Replace the `MediaPlayer`/`AudioOutput` block with `AudioManager` playback; rewire `effective_position`, `onMediaStatusChanged` position-restore, and `onPlaybackStateChanged` → `playback_started()` to `AudioManager` signals/properties. (Position restore now via `load_audio()` → `audio.seek()` after `audio.load()`; `playback_started()` emitted from the `audio.onStateChanged` handler when state becomes Playing.)
  - [x] 4.2 Replace the `CaptureSession`/`AudioInput`/`MediaRecorder` block + `start_recording()`/`stop_recording()`/`finalize_recording()` with `AudioManager.start_recording()`/`stop_recording()` and the `recordingFinished(path)` signal; change the output extension to `.flac`. (Removed `MediaDevices`; cpal re-detects the default input at `start()`.)
  - [x] 4.3 Update every `player.playbackState === MediaPlayer.*State` reference to the player-state values. The Rust `AudioManager` QObject has no `Q_ENUM`, so `AudioManager.Playing` would not resolve at runtime — used local `readonly property int player_stopped/playing/paused` constants instead.
  - [x] 4.4 Replace seek + range/loop with `AudioManager.seek()` / `play_range()` / `clear_range()`, **removing** the Qt-quirk workarounds (`seek_resume_timer`, `range_seek_timer`, `range_playback_timer`, `visual_position_override`/`effective_position`, `range_seek_pending`); map errors → `AudioManager.errorOccurred`. Kept the `volume_save_timer`/`position_save_timer` debounce persistence. **Added a `clear_range()` `#[qinvokable]` to the bridge** (not in the original Task 3 spec) so normal seek/stop can reset an active range.
  - [x] 4.5 Removed the `import QtMultimedia`; `make build -B` is clean with no residual `QtMultimedia` symbols (only comments mention it).
  - [x] 4.6 Fixes from the first runtime smoke test (backend/bridge, found during Task 4 verification):
    - **ALSA `snd_pcm_avail_delay` I/O error flood + idle device:** the cpal output stream was kept running continuously. Now built **paused** and toggled with playback state (`play()`/`play_range()` start it; `pause()`/`stop()`/`halt()` pause it). The output error callback is throttled to once / 2 s. (`backend/src/audio/player.rs`)
    - **Slow window open / no background waveform:** `Player::load()` decoded the whole MP3/FLAC synchronously on the GUI thread. Split into public `decode_to_mono` (run off-thread in the bridge) + `Player::from_samples` (cheap stream build, back on the Qt thread). (`backend/src/audio/player.rs`, `bridges/src/audio_manager.rs`)
    - **Position not advancing in the UI:** consequence of the blocked GUI thread above; resolved by the async decode. The `AudioManager` now remembers `pending_volume`/`pending_seek_ms` so `set_volume`/`seek` called right after the async `load()` (before the player exists) are applied on player creation — preserving saved-position restore. (`bridges/src/audio_manager.rs`)
    - On a non-looping natural end the poll thread now also `halt()`s the stream so the device stops feeding silence.
  - [x] 4.7 Fixes from the second runtime smoke test:
    - **Playback position never advanced in the UI:** `load_audio()` ran twice (`onFile_pathChanged` + `Component.onCompleted`), so two players/cores were built but `ensure_polling()` early-returned and the poll thread stayed bound to the *first* (discarded) core. Now the bridge holds a `current_core: Arc<Mutex<Option<Arc<Mutex<PlaybackCore>>>>>` slot that `load()` updates each time; the single poll thread reads the current core, so a re-load rebinds it. Also added a `loaded_path` guard in QML so the same file isn't decoded twice. (`bridges/src/audio_manager.rs`, `assets/qml/RecordingPlaybackItem.qml`)
    - **`MouseArea ... anchors on an item managed by a layout` warning** (`ChantingPracticeReviewWindow.qml`, both reference & user delegates): the `MouseArea` was `contentData` of a `Frame` whose `contentItem` is an explicit `RowLayout`, so it got reparented into that layout. Replaced with `TapHandler` + `HoverHandler` (pointer handlers attach to the `Frame`, need no anchors).
  - [x] 4.8 Fixes from the third runtime smoke test:
    - **First play click ignored:** because `load()` decodes asynchronously, a `play()` pressed before the player exists was a no-op. The bridge now records a `pending_play` flag and auto-starts the player when it becomes ready (cleared by `pause`/`stop`). (`bridges/src/audio_manager.rs`)
    - **Every position marker's button showed "pause" during normal playback:** `is_playing_this` for position markers keyed off the global play state. Added `active_position_marker_id` in QML so only the marker that started playback shows "pause"; main play / seek (`stop_range_playback`, ±5 s) / `play_range` / any stop clear it. (`assets/qml/RecordingPlaybackItem.qml`)
  - [x] 4.9 Loading state for the async decode: added a `loading` (bool) qproperty to `AudioManager`, set true at the start of `load()` and false on ready/error. The QML shows a "Loading…" placeholder instead of the waveform and disables the playback/seek/marker controls (via `enabled: !audio.loading` on the control containers) while decoding, so the user knows to wait. (`bridges/src/audio_manager.rs`, `assets/qml/com/profoundlabs/simsapa/AudioManager.qml`, `assets/qml/RecordingPlaybackItem.qml`)

---

### Specs for Task 5 (Build / Android)

- **Goal:** with QtMultimedia no longer referenced by any QML, drop the module so its FFmpeg backend + the five `libav*`/`libsw*` prebuilts stop being deployed on all platforms.
- **Android capture:** cpal selects the `oboe` backend automatically; ensure the Android target builds `oboe` from source via the project NDK. Pin NDK r28+ so the main app `.so` (incl. statically-linked Oboe) is 16 KB-aligned.
- **Depends on:** Task 4 (no remaining `Qt::Multimedia` QML usage).

- [ ] 5.0 Remove **Qt Multimedia / FFmpeg** from the build and wire up **Android** capture
  - [ ] 5.1 Remove `Multimedia` from `CXXQT_QTCOMPONENTS` and `Qt::Multimedia` from `target_link_libraries` in `CMakeLists.txt`; rebuild desktop (`make build -B`) and confirm the app links without Qt Multimedia.
  - [ ] 5.2 Ensure the Android build compiles `oboe`/cpal (add any required `.cargo/config.toml` target flags or `oboe` features); confirm the Android Cargo target builds.
  - [ ] 5.3 Pin `androidNdkVersion` to r28+ in `android/build.gradle`; verify the Qt-for-Android kit is compatible (Open Question #5).
  - [ ] 5.4 Confirm `android/AndroidManifest.xml` keeps `RECORD_AUDIO` and needs no QtMultimedia-specific service/permission entries.

---

### Specs for Task 6 (Verification & docs)

- **Compliance checks:** `readelf -lW <lib>.so | grep LOAD` → `p_align == 0x4000` for every bundled `.so`; the five `libav*`/`libsw*` libs absent; `zipalign -c -P 16 4 app.apk` → PASS.
- **Format/waveform:** a real recorded `.flac` decodes in `generate_waveform_data` and renders a non-empty waveform.
- **Permissions (already native — survives Qt Multimedia removal):** the mic permission flow uses `AssetManager.check_microphone_permission()` / `request_microphone_permission()`, backed by `android_helpers.h` C++ (`check_microphone_permission_impl` / `request_microphone_permission_impl`) — **not** Qt Multimedia. macOS `NSMicrophoneUsageDescription` is already patched in `CMakeLists.txt:276`, and `QMicrophonePermission` lives in QtCore/Gui (not Multimedia). So removing `Qt::Multimedia` does not break permissions — just confirm.
- **Docs/memory:** record the architecture and the verification results.

- [ ] 6.0 **Verify compliance, format, permissions** and update **documentation**
  - [ ] 6.1 Build a release Android package and run the ELF `p_align` audit + `zipalign -c -P 16 4`; record results (which libs audited) and confirm the five FFmpeg libs are gone.
  - [ ] 6.2 Verify the waveform path: produce a real `.flac` recording, run `generate_waveform_data`, confirm a non-empty waveform (add/extend a backend test against a `.flac` fixture if helpful).
  - [ ] 6.3 Confirm microphone permissions: Android `RECORD_AUDIO` request flow and macOS mic plist key (already present) still work for the new capture path.
  - [ ] 6.4 Write `docs/pure-rust-audio-backend.md` (architecture, crates, threading, FLAC format, permissions, 16 KB verification checklist + results).
  - [ ] 6.5 Update `PROJECT_MAP.md` (chanting/audio), `AGENTS.md` "Android 16 KB compatibility" note (FFmpeg gap resolved), and the `project_android_16kb_warning_test_deploy` memory note.
  - [ ] 6.6 Run the full backend test suite (`cd backend && cargo test`) and `make build -B`; confirm clean (ignore pre-existing unrelated failures).
