# Reference: Compiling a 16 KB-Aligned FFmpeg (Rejected Alternative)

- **Date:** 2026-06-14
- **Status:** Reference only — **not** the chosen approach
- **Related PRD:** [2026-06-14-081341-prd---replace-ffmpeg-backend-16kb-compliance.md](./2026-06-14-081341-prd---replace-ffmpeg-backend-16kb-compliance.md)

## Why this document exists

The active PRD replaces Qt Multimedia / FFmpeg with a **pure-Rust audio stack**
(`cpal` + `flacenc` + `symphonia`). During planning we evaluated an alternative:
keep Qt Multimedia and instead **compile a 16 KB page-aligned FFmpeg ourselves**
so the Android build passes Google Play's 16 KB requirement.

We **rejected** this for the current PRD, but it is a legitimate fallback. This
file captures the analysis so we don't have to re-derive it later (e.g. if the
pure-Rust playback engine proves too costly, or if a future need for FFmpeg's
codec breadth arises).

## What FFmpeg actually buys you

**Not audio fidelity.** The chosen plan records **FLAC, which is lossless** —
strictly higher fidelity than anything FFmpeg would encode (Opus/AAC/MP3 are
lossy). On pure signal quality, FLAC ≥ FFmpeg.

**Where FFmpeg genuinely wins is the maturity of the *playback engine*:**

- Battle-tested seeking, gapless playback, accurate position reporting, and
  decode-error tolerance.
- Hardware-accelerated decode on Android (`mediacodec`).
- **Zero rewrite** — the existing `MediaPlayer` / `MediaRecorder` QML in
  `RecordingPlaybackItem.qml`, with seek / position / range-loop already
  working, stays untouched.

That last point is the real trade. The pure-Rust path's biggest risk
(re-implementing sample-accurate position / seek / range-loop on `cpal`) is
exactly the thing Qt's FFmpeg stack already solved. So "FFmpeg has quality
advantages" is best read as *"Qt's media stack is mature and we don't have to
rebuild the player."* The cost of keeping it is owning an FFmpeg build pipeline.

## The build process (if we ever take this path)

This is **Android-only**. Desktop keeps Qt's stock FFmpeg — 16 KB alignment is a
Google Play rule and is irrelevant on Linux/macOS/Windows.

### Step 1 — Build FFmpeg from source, once per Android ABI (×4)

Architectures: `aarch64` (arm64-v8a), `armv7` (armeabi-v7a), `x86`, `x86_64`.

Prerequisites:
- The **Android NDK that matches the Qt build** (Qt 6.10/6.11 docs reference
  NDK `26.1.10909125`; for 16 KB alignment of the *main app* `.so` we separately
  want r28+, so confirm compatibility).
- `yasm` (`apt-get install yasm`).
- An Android **OpenSSL** build (libs named `libssl.so` / `libcrypto.so`, no
  version suffix), if network/TLS codecs are needed.

Per-ABI `configure` (set `ARCH`, `TOOLCHAIN_ARCH`, `CPU` for each), with the one
flag that delivers compliance:

```bash
../configure --prefix=../install-android-${ARCH} \
    --disable-doc --enable-network --enable-shared \
    --host-os=linux-x86_64 --target-os=android \
    --enable-cross-compile --arch=${ARCH} --cpu=${CPU} \
    --enable-jni --enable-mediacodec \
    --sysroot=${ANDROID_NDK_ROOT}/toolchains/llvm/prebuilt/linux-x86_64/sysroot \
    --cc=${ANDROID_NDK_ROOT}/toolchains/llvm/prebuilt/linux-x86_64/bin/${TOOLCHAIN_ARCH}24-clang \
    --cxx=${ANDROID_NDK_ROOT}/toolchains/llvm/prebuilt/linux-x86_64/bin/${TOOLCHAIN_ARCH}24-clang++ \
    --strip=${ANDROID_NDK_ROOT}/toolchains/llvm/prebuilt/linux-x86_64/bin/llvm-strip \
    --enable-openssl \
    --extra-cflags=-I<ANDROID_OPENSSL_INCLUDE_DIR> \
    --extra-ldflags=-L<ANDROID_OPENSSL_LIBS_DIR> \
    --extra-ldflags=-Wl,-z,max-page-size=16384      # <-- the 16 KB compliance flag
```

Then `make -j install`. Output: the five aligned shared libs per ABI —
`libavcodec`, `libavformat`, `libavutil`, `libswresample`, `libswscale`.

### Step 2 — Make Qt use them (two variants)

**(a) Supported: rebuild Qt Multimedia from source** against the custom FFmpeg:

```
-DFFMPEG_DIR=.../install-android-<arch> -DQT_DEPLOY_FFMPEG=ON
```

Build the `qtmultimedia` module for Android against the aligned FFmpeg and use
*that* module in place of the one in the official Qt for Android. This is the
robust, Qt-documented path.

**(b) Lightweight gamble: swap the `.so` only.** If we rebuild the **exact
FFmpeg version Qt shipped**, changing nothing but adding the alignment link flag
(ABI / soname identical), we can potentially replace just the 5 deployed `.so`
files inside the APK and skip rebuilding Qt. Faster, but **unsupported and
fragile** — any version/ABI drift on a Qt upgrade silently breaks it.

### Step 3 — Wire into the build

- Pin `androidNdkVersion` to **r28+** (main app `.so` alignment).
- Treat the aligned FFmpeg (and, for variant a, the custom `qtmultimedia`) as a
  **cached prebuilt artifact** consumed before the gradle / Qt Creator build.
- Re-verify: `readelf -lW <lib>.so | grep LOAD` → `p_align == 0x4000`, and
  `zipalign -c -P 16 4 app.apk` → PASS.

## Honest comparison

| | Pure-Rust (cpal + FLAC) — chosen | Rebuild FFmpeg 16 KB |
|---|---|---|
| Audio fidelity | Lossless (highest) | Lossy unless also recording FLAC |
| Playback robustness | Must re-implement seek/position/loop | Already works (Qt stack untouched) |
| QML rewrite | Significant | None |
| Build complexity | New Rust deps, one toolchain | Build FFmpeg ×4 + likely build Qt Multimedia from source |
| Maintenance | All Rust, one stack | Re-run FFmpeg/Qt build on every Qt/NDK bump |
| Deprecation risk | None | None (FFmpeg is Qt's *supported* backend) |
| Binary size | Smaller (no FFmpeg) | Larger (FFmpeg stays) |

## Bottom line

The rebuild trades a recurring **build-infrastructure** burden for keeping a
proven runtime (no playback rewrite, no behavior change). The pure-Rust path
trades a one-time **code rewrite** for a simpler long-term build and a smaller,
FFmpeg-free binary on every platform. Neither carries the MediaCodec deprecation
problem (that was a *third*, separately rejected option).

**Decision:** proceed with the pure-Rust stack (see the active PRD). Keep this
document as the fallback playbook if the pure-Rust playback engine proves too
costly or a future FFmpeg need arises.

## Sources

- [Building FFmpeg from source for Android on Linux — Qt Multimedia](https://doc.qt.io/qt-6//qtmultimedia-building-ffmpeg-android-linux.html)
- [Building Qt Multimedia from sources](https://doc.qt.io/qt-6/qtmultimedia-building-from-source.html)
- [Qt Multimedia backends](https://doc.qt.io/qt-6/qtmultimedia-index.html)
- [QtMultimedia on Android — Qt Wiki](https://wiki.qt.io/QtMultimedia_on_Android)
