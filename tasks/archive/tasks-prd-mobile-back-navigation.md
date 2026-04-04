## Relevant Files

- `assets/qml/SuttaSearchWindow.qml` - Main window: all history stack logic, back button interception (`onClosing`), navigation restore, and close confirmation dialog implemented here.
- `assets/qml/SuttaHtmlView.qml` - Webview abstraction; scroll position retrieval via `runJavaScript()`.
- `assets/qml/SuttaHtmlView_Mobile.qml` - Mobile-specific webview; content loads via `onData_jsonChanged`.
- `assets/qml/SuttaHtmlView_Desktop.qml` - Desktop-specific webview; content loads via `onData_jsonChanged`.
- `assets/qml/ChantingPracticeReviewWindow.qml` - Reference for existing `onClosing` handler pattern.
- `bridges/src/sutta_bridge.rs` - Rust backend; `get_sutta_html()` and `get_word_html()` used for content reload (no changes needed).

### Notes

- The history stack is purely QML-side (JavaScript array), no Rust backend changes needed.
- Scroll position is accessed via `runJavaScript()` calls on WebEngineView — there is no native QML scroll property.
- Existing scroll restore pattern at SuttaSearchWindow.qml:2375-2382 uses `window.scrollTo(0, ratio * scrollHeight)` with a `setTimeout` wrapper.
- Existing dialog pattern (e.g. `related_sutta_not_found_dialog` at line ~1736) uses `Dialog` with `Dialog.Ok | Dialog.Cancel` standardButtons.
- Tab data structure already includes `item_uid`, `table_name`, `sutta_ref`, `sutta_title` fields which align with history entry needs.
- Run QML tests with: `make qml-test`
- Run Rust tests with: `cd backend && cargo test`

## Research Findings

### Android Back Button Interception in Qt6/QML

**Chosen approach: `onClosing` handler on `ApplicationWindow`**

In Qt6 for Android, the system back button triggers the window's `closing` signal. The `onClosing` handler receives a `close` event object; setting `close.accepted = false` prevents the default close behavior and allows custom navigation logic.

This project does not pin a specific Qt6 minor version (CMakeLists.txt uses `find_package(Qt6 ...)`), but the `onClosing` approach is the standard and most reliable method across all Qt 6.x versions for top-level `ApplicationWindow` instances.

**Alternatives evaluated:**
- `Keys.onBackPressed` — Only works when the item has focus; unreliable for top-level windows since focus can be on any child widget (e.g., WebEngineView).
- `Keys.onPressed` with `Qt.Key_Back` — Same focus issue as above; not suitable for window-level interception.

**Interception pattern:**
```qml
onClosing: function(close) {
    if (root.is_mobile) {
        close.accepted = false;
        if (nav_history_can_go_back()) {
            navigate_back();
        } else {
            close_window_dialog.open();
        }
    }
    // Desktop: close.accepted defaults to true, normal close behavior
}
```

**Desktop compatibility:** When `is_mobile` is `false`, the handler does nothing — `close.accepted` defaults to `true`, so the window closes normally. The existing `onClosing` pattern in `ChantingPracticeReviewWindow.qml` (line 52) confirms this signal fires reliably and the project already uses it.

## Tasks

- [x] 1.0 Research Android back button handling in Qt6/QML
  - [x] 1.1 Research the recommended Qt6 approach for intercepting the Android system back button in a QML `ApplicationWindow`. Evaluate `onClosing` (with `close.accepted = false`), `Keys.onBackPressed`, and `Qt.Key_Back` key event handling. Check Qt6 documentation and known issues.
  - [x] 1.2 Determine whether `onClosing` fires reliably on Android back button press for `ApplicationWindow` in Qt 6.x (the version used by this project). Check CMakeLists.txt for the exact Qt version.
  - [x] 1.3 Document the chosen approach and any caveats in a brief note at the top of this task file under a "Research Findings" section. Include code snippet showing the interception pattern.
  - [x] 1.4 Verify that the chosen approach does not interfere with desktop close behavior (the `onClosing` handler must allow normal close on desktop, i.e. when `is_mobile === false`).

- [x] 2.0 Define the navigation history stack data structure and helper functions
  - [x] 2.1 Add a `property var nav_history: []` JavaScript array property to `SuttaSearchWindow.qml` (near the other window-level properties, around line 19-50).
  - [x] 2.2 Add a `property bool is_navigating_back: false` flag to prevent history recording during back navigation restores.
  - [x] 2.3 Implement `function nav_history_push(entry)` — pushes a history entry object onto the stack. The entry object structure: `{ type: "tab_switch"|"content_replace", tab_model: "pinned"|"results"|"translations", id_key: string, scroll_position: real, item_uid: string, table_name: string, sutta_ref: string, sutta_title: string }`. Should be a no-op when `is_navigating_back` is true.
  - [x] 2.4 Implement `function nav_history_pop()` — pops and returns the top entry, or `null` if the stack has fewer than 2 entries.
  - [x] 2.5 Implement `function nav_history_current()` — returns the top entry without removing it, or `null` if empty.
  - [x] 2.6 Implement `function nav_history_can_go_back()` — returns `true` if the stack has 2 or more entries (current view + at least one previous).
  - [x] 2.7 Implement `function get_current_scroll_position(callback)` — retrieves the current scroll position from the active webview via `runJavaScript("window.scrollY")` and passes the result to the callback. This is needed because `runJavaScript` is asynchronous.

- [x] 3.0 Instrument navigation code paths to record history entries
  - [x] 3.1 In `Component.onCompleted` (line ~917), after the initial tab is set up, push an initial history entry representing the first view so the stack starts with the current view on top.
  - [x] 3.2 In `tab_checked_changed()` (line ~2057), after switching to the new tab's webview, push a new history entry of type `"tab_switch"` with the newly activated tab's metadata. Guard with `if (!is_navigating_back)`.
  - [x] 3.3 In `show_result_in_html_view()` (line ~768), when content replaces or adds a tab, push a history entry of type `"content_replace"` with the new item's metadata. Guard with `if (!is_navigating_back)`.
  - [x] 3.4 In `open_related_sutta()` (line ~565), when a related sutta adds a new tab, push the appropriate history entry. Guard with `if (!is_navigating_back)`.
  - [x] 3.5 Audit for any other code paths that change the active tab or replace tab content (e.g. session restore, bookmark opening, link clicking within webview) and add history push calls where appropriate. Added history push to `open_bookmark_in_tab_group()` for pinned and translations tab groups. No session restore or webview-internal link navigation paths found.

- [x] 4.0 Implement the back navigation restore logic
  - [x] 4.1 Implement `function navigate_back()` — pops current entry, reads previous entry, dispatches to restore handler based on type. Sets `is_navigating_back` flag around the whole operation.
  - [x] 4.2 Implement `restore_tab_switch(entry)` — finds tab by `id_key`, clicks it. If tab was removed, re-creates it in the correct model with stored metadata.
  - [x] 4.3 Implement `restore_content_replace(entry)` — finds tab, updates model metadata and `data_json` on the webview component to trigger content reload via `onData_jsonChanged`.
  - [x] 4.4 Implement `restore_scroll_position(scroll_pos)` — uses `window.scrollTo()` with `setTimeout(200)` delay pattern matching existing code.
  - [x] 4.5 Async flow simplified: scroll position restore is done synchronously using the stored value from the history entry. The `get_current_scroll_position` async helper is available but not needed in the restore path since we store scroll positions at push time.

- [x] 5.0 Intercept the Android back button and wire it to the navigation logic
  - [x] 5.1 Add an `onClosing` handler to `SuttaSearchWindow.qml` (line ~18).
  - [x] 5.2 Gate the handler on `is_mobile`: if `is_mobile` is `false`, `close.accepted` defaults to `true` (normal close).
  - [x] 5.3 When `is_mobile` and `nav_history_can_go_back()` is true: set `close.accepted = false` and call `navigate_back()`.
  - [x] 5.4 When `is_mobile` and `nav_history_can_go_back()` is false: set `close.accepted = false` and open `close_window_dialog`.

- [x] 6.0 Add the "close app window" confirmation dialog
  - [x] 6.1 Add a `Dialog` component (id: `close_window_dialog`) to `SuttaSearchWindow.qml`, following the existing dialog pattern. Uses `standardButtons: Dialog.Ok | Dialog.Cancel`. Title: "Close Window", text: "No previous history item. Close the app window?"
  - [x] 6.2 In `onAccepted`: calls `Qt.quit()` to close the app.
  - [x] 6.3 In `onRejected`: default behavior (dialog closes, user stays on current view).
  - [x] 6.4 The dialog opens from the `onClosing` handler after `close.accepted = false` is set, so the close event is already prevented.
