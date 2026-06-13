# Android / ChromeOS soft keyboard for text inputs

On Android — and especially on ChromeOS running Android apps — a Qt `TextField`
/ `TextArea` does **not** reliably raise the on-screen keyboard when focused or
tapped. Two distinct problems were observed and fixed; both are handled by the
reusable [`MobileKeyboardHelper.qml`](../assets/qml/MobileKeyboardHelper.qml)
component plus a per-field `EnterKey.type`.

## The two problems

### 1. The keyboard needs two taps (or never appears)

Symptom: the first tap highlights the field and blinks the cursor (the field
**has** active focus), but the soft keyboard does not come up. A second tap
raises it.

Two compounding causes:

- **A pre-focused field swallows the first tap.** With `focus: true` the field
  already holds active focus by the time the user taps it, so the first physical
  tap is *not* a focus transition — Android's native "focus a field → raise the
  IME" never fires, and `onActiveFocusChanged` never fires either. This is why
  the persistent search bar (`SearchBarInput.qml`) needed two taps. Fix: don't
  pre-grab focus on mobile (`focus: root.is_desktop`); let the first tap be a
  real focus change.

- **A single `Qt.inputMethod.show()` right after a tap/focus is ignored.** The
  focus change has not yet been committed to the platform input context when the
  synchronous call runs, so the request is silently dropped. Fix: request the
  panel on both focus-in and tap, and **retry on a short `Timer`** until
  `Qt.inputMethod.visible` becomes true.

Confirmed from `adb logcat` on the ChromeOS device: on the working single tap,
`request_keyboard` first runs with `activeFocus=false`, then `activeFocusChanged
activeFocus=true` fires, a second `request_keyboard` runs, Qt's
`QtInputDelegate.showKeyboard` drives `InsetsController.show(ime())`, and the
retry observes `im.visible=true` on attempt 1 and stops.

### 2. The action key does not start the search

Symptom: on first show the keyboard's action key was a generic "Next" arrow that
does **not** emit `accepted`, so `onAccepted` never ran and the query didn't
start. (A later focus showed a "Done" checkmark, which *does* emit `accepted` —
hence the inconsistency.)

Fix: set `EnterKey.type` explicitly. For search fields use
`Qt.EnterKeySearch`, which maps to Android's `IME_ACTION_SEARCH`: a consistent
"search" action key that emits `accepted`. Because the IME action emits
`accepted` (not a physical Return key event), the field's submit logic must live
in `onAccepted`. A field that previously relied on `Keys.onReturnPressed` should
**move** that logic to `onAccepted` (which fires on desktop Return/Enter *and*
the mobile IME action) — do **not** keep both, or a desktop Return can run the
action twice.

## How to apply it

Drop `MobileKeyboardHelper {}` as a child of the input — with no arguments it
targets its parent field — and set an appropriate `EnterKey.type`:

```qml
// A search / lookup field whose action key should run the search:
TextField {
    id: search_input
    inputMethodHints: Qt.ImhNoAutoUppercase | Qt.ImhPreferLowercase // Pāli is lowercase
    EnterKey.type: Qt.EnterKeySearch
    onAccepted: search_btn.clicked()   // the IME search action emits `accepted`
    MobileKeyboardHelper {}
}

// A normal form field (commit & dismiss):
TextField {
    id: title_field
    EnterKey.type: Qt.EnterKeyDone
    MobileKeyboardHelper {}
}

// A multi-line field — do NOT set EnterKey.type (Enter inserts a newline):
TextArea {
    id: body_field
    wrapMode: TextEdit.WordWrap
    MobileKeyboardHelper {}
}
```

Guidelines:

- **`EnterKey.type`**: `Qt.EnterKeySearch` for search/lookup inputs;
  `Qt.EnterKeyDone` for single-line form fields; **omit** it for multi-line
  `TextArea`s (Enter must insert a newline).
- **`onAccepted`**: any field with `EnterKey.type` that triggers an action must
  handle `onAccepted` (the IME action does not produce a Return key event). If
  the field only had `Keys.onReturnPressed`, **move** that logic to `onAccepted`
  rather than keeping both (a desktop Return could otherwise run it twice).
- **Pre-focused persistent fields** (a search bar that sits focused): gate
  auto-focus to desktop (`focus: root.is_desktop`) so the first mobile tap is a
  real focus transition. Modal dialog fields that gain focus when the dialog
  opens do not need this — the open *is* the focus transition.
- **`gesturePolicy`**: the helper's `TapHandler` must stay `DragThreshold` (its
  default). That gives it a *passive* grab so taps still reach the field for
  cursor placement and text selection; a drag past the threshold cancels the tap.
  Never change it to `WithinBounds`/`ReleaseWithinBounds` — those take an
  exclusive grab and swallow the cursor tap.

The helper is mobile-gated (`Qt.platform.os` android/ios) and is a no-op on
desktop, so it is safe to add to any field.

## Where it is applied

Single-line inputs across the app carry `MobileKeyboardHelper {}` +
`EnterKey.type` (search fields: `SearchBarInput`, `ReferenceSearchWindow`,
`TopicIndexWindow`, `WordSummary` lookup → `EnterKeySearch`; form/dialog fields →
`EnterKeyDone`), and the mobile-relevant multi-line content areas
(`ChantingPracticeWindow` Pāli text) carry the helper without an `EnterKey.type`.
Read-only fields and desktop-only config/AI `TextArea`s (e.g. debug output,
keybinding capture, Anki/prompt templates) are intentionally left untouched.

When adding a **new** text input, apply this technique.

## Component registration

`MobileKeyboardHelper.qml` lives in `assets/qml/` and is listed in the
`qml_files` array in `bridges/build.rs`. New QML components must be added there
(see AGENTS.md → "New QML components").
