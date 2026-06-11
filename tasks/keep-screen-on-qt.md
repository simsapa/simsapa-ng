# Keeping the screen on while showing media (for a Qt / CXX-Qt app)

## Goal

When our app displays a photo (or plays a video / runs a slideshow), the Android
device should **not** dim the screen or go into suspend (black screen) after the
normal display timeout. When the media is no longer shown, normal screen-timeout
behaviour must resume immediately.

This document describes the technique used by **Fossify Gallery** (a native
Kotlin Android app) and how to reproduce it in a **Qt** app written in **C++**
and **Rust (via CXX-Qt)**.

---

## The technique (reference implementation)

Fossify Gallery does **not** use a `PowerManager` `WakeLock`. Instead it sets a
**window flag** on the Activity's window:

```kotlin
// Enable: screen stays on (and at full brightness) while this window is foreground
window.addFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON)

// Disable: restore normal screen-timeout behaviour
window.clearFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON)
```

In the reference app this is tied to the view lifecycle and a user setting:

| Where (Fossify Gallery)        | Action                                  |
|--------------------------------|-----------------------------------------|
| `PhotoFragment.onResume()`     | `addFlags` if the user setting is on    |
| `PhotoFragment.onPause()`      | `clearFlags`                            |
| `VideoFragment` play / pause   | `addFlags` / `clearFlags`               |
| `ViewPagerActivity` slideshow  | `addFlags` on start, `clearFlags` stop  |

### Why `FLAG_KEEP_SCREEN_ON` and not a `WakeLock`

This matches the official Android guidance. The "Keep the screen on" guide states
that to keep the screen on in an activity you should use `FLAG_KEEP_SCREEN_ON`,
because *"unlike wake locks, it doesn't require special permission, and the
platform correctly manages the user moving between applications, without your app
needing to worry about releasing unused resources."* The wake-lock guidance is
blunter: *"you should never need to use a wake lock in an activity."*

- **No permission required.** A `WakeLock` needs the `WAKE_LOCK` permission;
  the window flag needs nothing.
- **Cannot leak.** The flag lives on the window. Per the docs, *"if an app with
  the `FLAG_KEEP_SCREEN_ON` flag goes into the background, the system allows the
  screen to turn off normally, and you don't need to explicitly clear the flag
  in this case."* A forgotten `WakeLock`, by contrast, keeps the device awake and
  drains the battery.
- **It's the behaviour we actually want.** We only care about keeping the
  *screen* on while *our window is visible*, which is exactly what the flag does.
  We don't want to keep the CPU awake in the background.

### Key facts

- Constant: `WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON`
- Numeric value: `0x00000080` (decimal `128`) — needed when calling via JNI.
- Documented behaviour: *"as long as this window is visible to the user, keep the
  device's screen turned on and bright."*
- **Activity-only.** This flag may only be set on an activity window, never in a
  service or other component.
- The XML equivalent is `android:keepScreenOn="true"` on a view, or
  `View.setKeepScreenOn(true)` in code; setting the window flag directly (as we
  do via JNI) is the equivalent for a Qt app that has no Android view hierarchy
  of its own.
- Window-flag changes **must run on the Android UI (main) thread.**

---

## Reproducing it in Qt

A Qt Android app runs inside a single `Activity` (`QtActivity`). We reach its
`Window` through JNI and call `addFlags` / `clearFlags` on it, on the UI thread.

### Option A — C++ with `QJniObject` (Qt 6)

```cpp
#include <QJniObject>
#include <QtCore/private/qandroidextras_p.h>   // QtAndroidPrivate::runOnAndroidMainThread

namespace {
// android.view.WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON
constexpr jint FLAG_KEEP_SCREEN_ON = 0x00000080;

void setKeepScreenOnFlag(bool enable)
{
    // Window-flag changes must happen on the Android UI thread.
    QtAndroidPrivate::runOnAndroidMainThread([enable] {
        QJniObject activity = QNativeInterface::QAndroidApplication::context();
        if (!activity.isValid())
            return;

        QJniObject window = activity.callObjectMethod(
            "getWindow", "()Landroid/view/Window;");
        if (!window.isValid())
            return;

        if (enable) {
            window.callMethod<void>("addFlags", "(I)V", FLAG_KEEP_SCREEN_ON);
        } else {
            window.callMethod<void>("clearFlags", "(I)V", FLAG_KEEP_SCREEN_ON);
        }
    });
}
} // namespace

void keepScreenOn(bool on) { setKeepScreenOnFlag(on); }
```

Notes:
- `QNativeInterface::QAndroidApplication::context()` returns the current
  `Activity` as a `QJniObject` (Qt 6). On Qt 5 use
  `QtAndroid::androidActivity()`.
- `runOnAndroidMainThread` returns a `QFuture<void>`; you can ignore it for a
  fire-and-forget call, or `.waitForFinished()` if you need to block.
- On non-Android platforms these symbols don't exist — guard the file with
  `#ifdef Q_OS_ANDROID` (see the cross-platform section).

### Option B — Rust via CXX-Qt

There are two clean approaches. **The recommended one is to keep the JNI call in
C++** (where `QJniObject` lives) and expose a tiny invokable to Rust through the
CXX-Qt bridge. This avoids duplicating Qt's JNI plumbing in Rust.

**1. C++ helper** (`screen.cpp` / `screen.h`) — same body as Option A, wrapped in
a class or a free function declared to CXX-Qt.

**2. CXX-Qt bridge (Rust):**

```rust
#[cxx_qt::bridge]
mod ffi {
    unsafe extern "RustQt" {
        #[qobject]
        #[qml_element]
        type ScreenController = super::ScreenControllerRust;
    }

    unsafe extern "RustQt" {
        // Callable from QML/Rust; implemented in Rust, delegates to C++ helper.
        #[qinvokable]
        fn set_keep_screen_on(self: Pin<&mut ScreenController>, on: bool);
    }

    // Free function implemented in C++ (Option A body).
    unsafe extern "C++" {
        include!("screen.h");
        #[namespace = "app"]
        fn keep_screen_on(on: bool);
    }
}

use core::pin::Pin;

#[derive(Default)]
pub struct ScreenControllerRust;

impl qobject::ScreenController {
    pub fn set_keep_screen_on(self: Pin<&mut Self>, on: bool) {
        #[cfg(target_os = "android")]
        ffi::keep_screen_on(on);
        #[cfg(not(target_os = "android"))]
        let _ = on; // no-op on desktop, or call a desktop inhibitor
    }
}
```

**Pure-Rust alternative:** call JNI directly with the `jni` crate using the
`JavaVM` / activity obtained from `ndk-context` (or
`android_activity`). This works but you must marshal onto the UI thread
yourself and manage `JNIEnv` lifetimes — more error-prone than reusing
`QJniObject`. Prefer Option B unless you have a reason to avoid C++.

### Wiring it to the UI lifecycle

Mirror the reference app: enable when the media view is shown, disable when it
is hidden. In QML, for example:

```qml
MediaViewer {
    property bool keepScreenOn: settings.keepScreenOn   // user setting

    Component.onCompleted: if (keepScreenOn) screenController.setKeepScreenOn(true)
    Component.onDestruction: screenController.setKeepScreenOn(false)

    onVisibleChanged: screenController.setKeepScreenOn(visible && keepScreenOn)
}
```

Also clear the flag when the app is backgrounded if you want to be strict,
though the OS already ignores the flag for non-foreground windows.

---

## Cross-platform note (desktop)

`FLAG_KEEP_SCREEN_ON` is Android-only. If the same Qt app also runs on desktop
and needs to inhibit the screensaver/suspend while showing media, use the
platform inhibitors behind the same `keep_screen_on(bool)` interface:

| Platform | API |
|----------|-----|
| Linux (Wayland/X11) | D-Bus `org.freedesktop.ScreenSaver.Inhibit` / `org.freedesktop.login1` |
| Windows  | `SetThreadExecutionState(ES_CONTINUOUS \| ES_DISPLAY_REQUIRED)` |
| macOS    | `IOPMAssertionCreateWithName(kIOPMAssertionTypeNoDisplaySleep, ...)` |

Keep a single abstraction (`keepScreenOn(bool)`) and switch implementation by
platform with `#ifdef` (C++) or `#[cfg(...)]` (Rust).

---

## Summary

- Set `WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON` (value `0x80`) on the
  Activity window via `addFlags`; remove it with `clearFlags`.
- No permissions, no `WakeLock`, no leak risk.
- In Qt, reach the window with `QJniObject` and run the change on the Android UI
  thread (`runOnAndroidMainThread`).
- For CXX-Qt, keep the JNI call in C++ and expose a `set_keep_screen_on(bool)`
  invokable to Rust/QML.
- Drive it from the media view's show/hide lifecycle, gated by a user setting.

**Reference (source app):** Fossify Gallery — `PhotoFragment.kt`
(`onResume`/`onPause`), `VideoFragment.kt`, `ViewPagerActivity.kt`,
`VideoPlayerActivity.kt`, setting `Config.keepScreenOn`.

---

## API reference links

### Android

- [Keep the screen on (guide)](https://developer.android.com/develop/background-work/background-tasks/awake/screen-on)
  — the canonical recommendation to use `FLAG_KEEP_SCREEN_ON` / `keepScreenOn`.
- [Choose the right API to keep the device awake](https://developer.android.com/develop/background-work/background-tasks/awake)
  — decision tree; "use the most lightweight approach possible".
- [Keep the device awake / wake locks](https://developer.android.com/training/scheduling/wakelock)
  — why wake locks are a last resort ("you should never need to use a wake lock in an activity").
- [`WindowManager.LayoutParams` (incl. `FLAG_KEEP_SCREEN_ON`)](https://developer.android.com/reference/android/view/WindowManager.LayoutParams#FLAG_KEEP_SCREEN_ON)
- [`Window.addFlags(int)`](https://developer.android.com/reference/android/view/Window#addFlags(int))
- [`Window.clearFlags(int)`](https://developer.android.com/reference/android/view/Window#clearFlags(int))
- [`View.setKeepScreenOn(boolean)`](https://developer.android.com/reference/android/view/View#setKeepScreenOn(boolean))
  — the view-level / XML equivalent.
- [`Activity.getWindow()`](https://developer.android.com/reference/android/app/Activity#getWindow())
- [`Activity.runOnUiThread(Runnable)`](https://developer.android.com/reference/android/app/Activity#runOnUiThread(java.lang.Runnable))
  — fallback for marshalling onto the UI thread if not using Qt's helper.

### Qt

- [`QJniObject`](https://doc.qt.io/qt-6/qjniobject.html) — calling Java/JNI from C++.
- [`QNativeInterface::QAndroidApplication`](https://doc.qt.io/qt-6/qnativeinterface-qandroidapplication.html)
  — `context()` returns the current activity, plus `runOnAndroidMainThread(...)`.
- [Qt JNI / Android integration overview](https://doc.qt.io/qt-6/android-jni.html)
- [`QFuture`](https://doc.qt.io/qt-6/qfuture.html) — return type of `runOnAndroidMainThread`.

### CXX-Qt

- [CXX-Qt book](https://kdab.github.io/cxx-qt/book/) — `#[cxx_qt::bridge]`,
  `#[qinvokable]`, exposing C++ to Rust and vice versa.
- [`cxx` (Rust ↔ C++ FFI)](https://cxx.rs/) — the `extern "C++"` mechanics CXX-Qt builds on.

### Desktop inhibitors (cross-platform fallback)

- Linux: [`org.freedesktop.ScreenSaver`](https://specifications.freedesktop.org/idle-inhibit-spec/latest/)
  / [`org.freedesktop.login1` Inhibit](https://www.freedesktop.org/software/systemd/man/latest/org.freedesktop.login1.html)
- Windows: [`SetThreadExecutionState`](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-setthreadexecutionstate)
- macOS: [`IOPMAssertionCreateWithName`](https://developer.apple.com/documentation/iokit/1557134-iopmassertioncreatewithname)
</content>
</invoke>

result
The file is created at /home/gambhiro/src/applications/Gallery/docs/keep-screen-on-qt.md
The file is created at /home/gambhiro/src/applications/Gallery/docs/keep-screen-on-qt.md

Wait, this got created in the Gallery repo. Let me verify that's where you want it.

