import QtQuick

// Reusable Android/ChromeOS soft-keyboard activator.
//
// Drop it as a child of any TextField / TextArea — with no arguments it targets
// its parent input field:
//
//     TextField {
//         id: my_field
//         MobileKeyboardHelper {}
//     }
//
// Rationale and the full story are in docs/android-soft-keyboard.md. In short:
// on Android (and especially ChromeOS running Android apps) a single
// Qt.inputMethod.show() issued right after a tap/focus is often ignored,
// because the focus change has not yet been committed to the platform input
// context — which is why the search bar needed a second tap. This helper
// requests the panel both on focus-in and on tap, and retries on a short Timer
// until the IME reports visible.
Item {
    id: helper

    Logger { id: logger }

    // The input field to drive. Defaults to the parent element so the helper
    // can be dropped inside a TextField/TextArea with no arguments.
    property Item field: parent

    readonly property bool is_mobile: Qt.platform.os === "android" || Qt.platform.os === "ios"

    // Keyboard diagnostics: confirm the helper is active and which platform it
    // sees. If is_mobile is false on a Chromebook, the Connections/TapHandler
    // below are disabled and the keyboard is never requested.
    Component.onCompleted: logger.info("MobileKeyboardHelper: Qt.platform.os="
        + Qt.platform.os + " is_mobile=" + helper.is_mobile
        + " field=" + helper.field)

    // Zero-size: this is a behaviour helper, not a visual element.
    width: 0
    height: 0

    // A single Qt.inputMethod.show() right after a tap/focus is unreliable on
    // Android/ChromeOS — the focus change may not be committed to the platform
    // input context yet, so the request is silently dropped. Retry on a short
    // Timer until the IME reports visible (or we give up after a few tries).
    Timer {
        id: retry_timer
        interval: 150
        repeat: true
        property int attempts: 0
        onTriggered: {
            attempts += 1;
            Qt.inputMethod.show();
            logger.info("MobileKeyboardHelper: retry attempt=" + attempts
                + " inputMethod.visible=" + Qt.inputMethod.visible);
            if (Qt.inputMethod.visible || attempts >= 5) stop();
        }
    }

    function request_keyboard() {
        logger.info("MobileKeyboardHelper: request_keyboard() called, "
            + "inputMethod.visible=" + Qt.inputMethod.visible);
        Qt.inputMethod.show();
        retry_timer.attempts = 0;
        retry_timer.restart();
    }

    // Field gained active focus (tapped, or a dialog opened onto it): request
    // the keyboard.
    Connections {
        target: helper.field
        enabled: helper.is_mobile && helper.field !== null
        function onActiveFocusChanged() {
            logger.info("MobileKeyboardHelper: field.onActiveFocusChanged activeFocus="
                + helper.field.activeFocus);
            if (helper.field.activeFocus) helper.request_keyboard();
        }
    }

    // Re-tapping an already-focused field produces no focus-change signal, so
    // also request the keyboard on every tap.
    //
    // gesturePolicy MUST stay DragThreshold (the default): the handler then
    // takes only a PASSIVE grab, so the press/move/release still reach the
    // underlying input — tap-to-position-cursor, drag-to-select and the
    // selection handles keep working, and a drag past the threshold cancels the
    // tap so it never competes with text selection. Do NOT change this to
    // WithinBounds/ReleaseWithinBounds — those take an exclusive grab and would
    // swallow the cursor tap.
    TapHandler {
        parent: helper.field
        enabled: helper.is_mobile && helper.field !== null
        gesturePolicy: TapHandler.DragThreshold
        onTapped: {
            logger.info("MobileKeyboardHelper: TapHandler onTapped");
            helper.request_keyboard();
        }
    }
}
