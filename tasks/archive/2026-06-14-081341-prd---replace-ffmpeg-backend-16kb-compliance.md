# PRD: Replace Qt's FFmpeg Media Backend with a Pure-Rust Audio Stack (16 KB Compliance)

- **Date:** 2026-06-14
- **Status:** Draft
- **Area:** Chanting practice — audio recording & playback (`RecordingPlaybackItem.qml`), Rust backend audio, Android packaging, build configuration

## 1. Introduction / Overview

Simsapa's chanting practice feature records and plays back audio using Qt
Multimedia. On Qt 6, Qt Multimedia ships a **bundled FFmpeg backend** by
default, including five prebuilt shared libraries (`libavcodec`, `libavformat`,
`libavutil`, `libswresample`, `libswscale`). These FFmpeg prebuilts are
**4 KB page-aligned** (`p_align = 0x1000`).

Google Play now requires apps targeting Android API 35+ to be **16 KB page-size
compatible** (`p_align = 0x4000`). The FFmpeg prebuilts are the *only* libraries
in the app that fail this requirement — Qt's own libraries are already
16 KB-aligned, and the Rust backend is compiled as a `staticlib` into the main
app `.so` (it is not a separate library). This means the app currently **cannot
be submitted to the Play Store** while the FFmpeg backend is present.

We considered switching to Qt's **native Android (MediaCodec) backend**, but Qt
**deprecated it in 6.8** and will remove it in Qt 7; new features land only on
the FFmpeg backend. To avoid taking on a backend that is already end-of-life,
this PRD instead **removes Qt Multimedia from the audio path entirely** and
replaces it with a **pure-Rust audio stack** in the existing Rust backend:

- **Capture:** `cpal` (cross-platform: ALSA/PulseAudio on Linux, CoreAudio on
  macOS, WASAPI on Windows, AAudio/Oboe on Android).
- **Encode:** `flacenc` (pure-Rust, lossless FLAC).
- **Decode / playback:** `symphonia` (already a dependency — decodes FLAC) for
  decoding, driving a `cpal` output stream (directly or via `rodio`) with
  sample-accurate position tracking.

This eliminates FFmpeg on **every** platform (not just Android), removes the
deprecation risk, shrinks the binary, and gives us one audio implementation to
maintain. The waveform path already uses Symphonia and is essentially unchanged.

**Goal:** Replace the Qt Multimedia / FFmpeg record & playback path with a
pure-Rust stack so the Android build passes Google Play's 16 KB page-size
requirement, while preserving the chanting-practice record/playback experience
on all platforms.

## 2. Goals

1. **No FFmpeg anywhere.** Qt Multimedia is no longer linked for the audio path;
   the five `libav*`/`libsw*` prebuilts are absent from every platform's build.
2. The Android APK/AAB contains **no 4 KB-aligned native libraries** — every
   bundled `.so` reports `p_align = 0x4000`, and `zipalign -c -P 16 4` passes.
3. Audio **recording** and **playback** in chanting practice work on Android,
   Linux, macOS, and Windows through the new Rust audio stack, with feature
   parity for the existing UI (play/pause/stop, seek, duration, position,
   volume, range/loop playback).
4. Recordings are written in **one format used on all platforms** (FLAC — see
   Functional Requirements), decodable by the existing waveform generator.
5. A documented **verification checklist** confirms no other library blocks Play
   Store compliance (NDK version, ELF alignment audit, `zipalign` check).

## 3. User Stories

- **As a practitioner on Android**, I want to record my chanting and play it
  back, so that I can compare it against the reference recording — without the
  app being blocked from the Play Store.
- **As the maintainer**, I want to submit the app to Google Play targeting API
  35+, so that I need a build with zero 4 KB-aligned native libraries and no
  end-of-life media backend.
- **As a practitioner on any platform**, I want recording, playback, seeking,
  range looping, and waveform display to keep working exactly as before, so that
  the feature is not visibly degraded by the backend change.
- **As the maintainer**, I want a single Rust audio implementation shared by all
  platforms, so that I don't maintain divergent per-platform media code.

## 4. Functional Requirements

### Recording format

1. New recordings **must** be encoded as **FLAC** (`.flac`), produced by the
   pure-Rust `flacenc` encoder. FLAC is chosen because it is lossless,
   compressed (~half the size of WAV), encodes in pure Rust (no C codec lib that
   would reintroduce the alignment/build problem), and is already decodable by
   the `symphonia`-based waveform path.
2. Recordings **must** be captured as **mono, 16-bit, 48 kHz** (suitable for
   voice; minimizes file size). If the input device cannot provide this exactly,
   the Rust layer resamples/downmixes to this canonical format before encoding.
3. A clean format change is acceptable — there is **no requirement** to play
   back or migrate previously recorded `.ogg` files (no installed user base).

### Rust audio backend (new)

4. A new Rust module (e.g. `backend/src/audio/` with `recorder.rs` and
   `player.rs`) **must** implement, using `cpal` + `flacenc` + `symphonia`:
   - **Recording:** start capture to a target file path; stop and finalize the
     FLAC file. Capture runs on its own audio thread; errors are surfaced back
     to the UI.
   - **Playback:** load a file; play / pause / stop; seek to a position in ms;
     report current position in ms and total duration in ms; set output volume;
     play a bounded **range** `[start_ms, end_ms]` with optional **looping**
     (to support the existing range-playback UI).
5. The audio backend **must** be exposed to QML through the existing bridge
   layer (e.g. `SuttaBridge` or a new dedicated bridge), with QML-callable
   functions and signals covering the operations in Req. 4. Follow the project's
   bridge conventions (register the Rust file in `bridges/build.rs`; add the
   matching `qmllint` type stub in `assets/qml/com/profoundlabs/simsapa/`).
6. Playback position **must** be reported with enough accuracy and update
   frequency to drive the existing waveform playhead and range-loop logic
   (sample-counted from the output stream rather than relying on coarse buffer
   callbacks). A periodic position signal (or polled property) suitable for the
   UI's needs is required.
7. Microphone capture **must** respect platform permissions:
   - Android: the existing `RECORD_AUDIO` permission and the in-app permission
     prompt flow.
   - macOS: the `NSMicrophoneUsageDescription` / TCC prompt must still trigger
     (CoreAudio access). Add the usage-description plist key if not already
     present.

### QML refactor

8. `assets/qml/RecordingPlaybackItem.qml` **must** be refactored to **remove all
   `QtMultimedia` usage** (`import QtMultimedia`, `MediaPlayer`, `AudioOutput`,
   `CaptureSession`, `AudioInput`, `MediaRecorder`, `MediaDevices`) and instead
   drive recording/playback through the new Rust bridge API and its signals.
9. The existing UI behaviour **must** be preserved: record/stop, play/pause/stop,
   waveform-click seek, saved playback position restore, range create, range
   playback + loop, volume, file-not-found handling, and error messages. The
   `MediaPlayer.*State` checks throughout the file map to a player-state value
   provided by the bridge.
10. `start_recording()`'s output path **must** use the `.flac` extension; the
    `actualLocation`-based `finalize_recording()` flow is replaced by the
    bridge's "recording finished" signal carrying the final file path.

### Build configuration

11. `Qt::Multimedia` **must** be removed from `CMakeLists.txt`
    (`CXXQT_QTCOMPONENTS` and `target_link_libraries`) once no QML references it,
    so the FFmpeg backend plugin and its prebuilts are no longer deployed on any
    platform.
12. New Rust dependencies (`cpal`, `flacenc`, and any resampler such as `rubato`,
    plus the Android `oboe` path for cpal) **must** be added to the appropriate
    `Cargo.toml`. The Android build **must** compile `oboe` from source with the
    project NDK (no prebuilt `.so`).
13. The Android `build.gradle` NDK version (`androidNdkVersion`) **must** be
    pinned to **r28 or newer** (defaults the main app `.so`, including the
    statically-linked Oboe, to 16 KB alignment).

### Waveform

14. The waveform generator (`backend/src/waveform.rs`, `generate_waveform_data`)
    **must** decode the new FLAC recordings. `symphonia` is already pulled in
    with `features = ["all"]` (includes FLAC) — verify it produces a non-empty
    waveform from a real recording.

### Verification (compliance)

15. The build/release process **must** include a documented verification step,
    with results recorded:
    - `readelf -lW <each lib>.so | grep LOAD` → `p_align = 0x4000` for every
      bundled library; the five `libav*`/`libsw*` libraries are **absent**.
    - `$ANDROID_SDK/build-tools/<ver>/zipalign -c -P 16 4 app.apk` → `PASS`.

### Documentation

16. `docs/` **must** gain a doc describing the pure-Rust audio architecture
    (capture/encode/decode/playback crates, threading, bridge API, FLAC format,
    permissions) and the 16 KB verification checklist.
17. `PROJECT_MAP.md` and the CLAUDE.md "Android 16 KB compatibility" note
    **must** be updated to reflect that FFmpeg / Qt Multimedia is no longer used
    (the previously-documented "real gap" is resolved). Update the FFmpeg-related
    memory note accordingly.

## 5. Non-Goals (Out of Scope)

- Switching to Qt's native (MediaCodec/`QT_MEDIA_BACKEND=android`) backend — the
  reason this PRD exists is to avoid that deprecated path.
- Building a custom 16 KB-aligned FFmpeg from source.
- Using a lossy codec (Opus/AAC/MP3/Vorbis). Their encoders are C libraries that
  would reintroduce a native-lib alignment/build burden; FLAC keeps the stack
  pure-Rust.
- Migrating or converting any pre-existing `.ogg` recordings.
- Changing chanting-practice UI/markers/ranges/waveform *interaction* beyond what
  the backend swap requires.
- Submitting the app to the Play Store (this PRD makes the build *eligible*).

## 6. Design Considerations

- The only audio UI surface is `assets/qml/RecordingPlaybackItem.qml`, hosted by
  `ChantingPracticeReviewWindow.qml` and `ChantingPracticeWindow.qml`. No new UI
  is introduced; the visible change is the recording file extension (`.flac`).
- Player-state, position, and duration become bridge-provided values/signals
  instead of `MediaPlayer` properties. Define a small state enum
  (stopped/playing/paused) so the many `player.playbackState === ...` checks map
  cleanly.
- Error handling keeps the existing `error_message` surface; capture/encode/
  decode failures from Rust are delivered via an error signal and shown in the
  same place.
- Microphone permission UX (`RECORD_AUDIO`, macOS TCC) is preserved.

## 7. Technical Considerations

- **Threading / Qt integration.** `cpal` streams run on a dedicated audio
  callback thread. Communication with QML must marshal to the Qt thread via the
  CXX-Qt bridge signals (do not touch QObject state from the audio thread). The
  bridge owns the recorder/player objects and emits `position`, `state`,
  `finished`, and `error` signals.
- **Position / seek accuracy.** Drive the waveform playhead and range-loop end
  detection from a **sample counter** on the output stream (interpolated between
  buffer callbacks), not from coarse device clocks. (`rodio` alone tracks
  position poorly; either use a `rodio_playback_position`-style wrapper or a
  custom cpal output that counts frames.)
- **Android capture via Oboe.** cpal's Android backend uses the `oboe` crate,
  which **compiles Oboe C++ from source** with the project NDK and links it into
  our `staticlib` → main app `.so`. Built with NDK r28+ this is 16 KB-aligned
  with **no separate prebuilt** — confirming the compliance win. Note the known
  cpal Android quirk of large reported min buffer sizes; pick an explicit buffer
  size rather than the reported minimum.
- **Resampling.** Input devices may not natively offer 48 kHz mono; include a
  resampler (e.g. `rubato`) / downmix step before FLAC encoding so all files are
  canonical regardless of device.
- **FLAC encode cost.** `flacenc` is pure Rust; encode the captured PCM
  (streamed or on stop). For short chanting clips this is cheap; verify it does
  not block the UI (encode off the Qt thread).
- **Symphonia decode.** `features = ["all"]` already enables FLAC, so playback
  decode and waveform decode share the existing decoder; no new decode
  dependency is required (only the cpal **output** path is new).
- **Desktop runtime deps.** Removing Qt Multimedia also removes the GStreamer/
  FFmpeg runtime requirement on Linux; cpal uses ALSA/Pulse directly. Confirm
  the AppImage still bundles/uses an available ALSA or PulseAudio path.
- **Other native libraries (audit result):** Per existing CLAUDE.md forensics,
  Qt's own libs are already `0x4000`-aligned and all libs pack as `Defl:N`; the
  Rust code (now including Oboe) links statically into the main app `.so`. After
  removing FFmpeg, the FFmpeg prebuilts were the sole blocker — the verification
  checklist (Req. 15) re-confirms this on the FFmpeg-free build.

## 8. Success Metrics

1. `zipalign -c -P 16 4 <app>.apk` returns **PASS** on a release build.
2. `readelf -lW` shows `p_align = 0x4000` for **all** bundled `.so` files; the
   five `libav*`/`libsw*` libraries are **absent**.
3. Manual smoke test on a physical Android device: record a chant → waveform
   renders → play back → audio is audible → seek and range-loop work — with no
   FFmpeg libs present.
4. Record + playback (including seek, volume, range loop) verified on at least
   one desktop platform (Linux or macOS) via the Rust stack.
5. The build is accepted by Google Play's pre-launch 16 KB check (or the
   internal-testing-track upload no longer reports the FFmpeg alignment error).

## 9. Open Questions

1. **Encoder placement / latency:** encode incrementally during capture, or
   buffer PCM and encode on stop? (Affects memory for long recordings vs. UI
   responsiveness at stop.)
2. **Position-tracking implementation:** adopt the `rodio_playback_position`
   crate, or write a minimal custom cpal output with a frame counter? Confirm
   which gives reliable sub-100 ms accuracy on Android.
3. **Resampler choice:** is `rubato` acceptable, or is a simpler linear
   downmix/resample sufficient for voice recordings?
4. **macOS plist:** confirm `NSMicrophoneUsageDescription` is present in the app
   bundle Info.plist (required once CoreAudio capture replaces Qt's path).
5. **NDK bump impact:** does pinning NDK to r28+ require Qt Creator kit / CI
   image changes, and is the installed Qt for Android compatible?
6. **WAV fallback:** if `flacenc` maturity/perf disappoints, is uncompressed WAV
   (`hound`) an acceptable fallback format?
