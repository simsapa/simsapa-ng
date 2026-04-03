## Relevant Files

- `assets/qml/SuttaSearchWindow.qml` - Main window where all history stack logic, back button interception, and navigation restore will be implemented.
- `assets/qml/SuttaHtmlView.qml` - Webview abstraction; scroll position retrieval via `runJavaScript()`.
- `assets/qml/SuttaHtmlView_Mobile.qml` - Mobile-specific webview; may need scroll position getter.
- `assets/qml/SuttaHtmlView_Desktop.qml` - Desktop-specific webview; reference for scroll patterns.
- `assets/qml/ChantingPracticeReviewWindow.qml` - Reference for existing `onClosing` handler pattern.
- `bridges/src/sutta_bridge.rs` - Rust backend; `get_sutta_html()` and `get_word_html()` used for content reload.

### Notes

- The history stack is purely QML-side (JavaScript array), no Rust backend changes needed.
- Scroll position is accessed via `runJavaScript()` calls on WebEngineView — there is no native QML scroll property.
- Existing scroll restore pattern at SuttaSearchWindow.qml:2375-2382 uses `window.scrollTo(0, ratio * scrollHeight)` with a `setTimeout` wrapper.
- Existing dialog pattern (e.g. `related_sutta_not_found_dialog` at line ~1736) uses `Dialog` with `Dialog.Ok | Dialog.Cancel` standardButtons.
- Tab data structure already includes `item_uid`, `table_name`, `sutta_ref`, `sutta_title` fields which align with history entry needs.
- Run QML tests with: `make qml-test`
- Run Rust tests with: `cd backend && cargo test`

## Tasks

- [ ] 1.0 Research Android back button handling in Qt6/QML
  - [ ] 1.1 Research the recommended Qt6 approach for intercepting the Android system back button in a QML `ApplicationWindow`. Evaluate `onClosing` (with `close.accepted = false`), `Keys.onBackPressed`, and `Qt.Key_Back` key event handling. Check Qt6 documentation and known issues.
  - [ ] 1.2 Determine whether `onClosing` fires reliably on Android back button press for `ApplicationWindow` in Qt 6.x (the version used by this project). Check CMakeLists.txt for the exact Qt version.
  - [ ] 1.3 Document the chosen approach and any caveats in a brief note at the top of this task file under a "Research Findings" section. Include code snippet showing the interception pattern.
  - [ ] 1.4 Verify that the chosen approach does not interfere with desktop close behavior (the `onClosing` handler must allow normal close on desktop, i.e. when `is_mobile === false`).

- [ ] 2.0 Define the navigation history stack data structure and helper functions
  - [ ] 2.1 Add a `property var nav_history: []` JavaScript array property to `SuttaSearchWindow.qml` (near the other window-level properties, around line 19-50).
  - [ ] 2.2 Add a `property bool is_navigating_back: false` flag to prevent history recording during back navigation restores.
  - [ ] 2.3 Implement `function nav_history_push(entry)` — pushes a history entry object onto the stack. The entry object structure: `{ type: "tab_switch"|"content_replace", tab_model: "pinned"|"results"|"translations", id_key: string, scroll_position: real, item_uid: string, table_name: string, sutta_ref: string, sutta_title: string }`. Should be a no-op when `is_navigating_back` is true.
  - [ ] 2.4 Implement `function nav_history_pop()` — pops and returns the top entry, or `null` if the stack has fewer than 2 entries.
  - [ ] 2.5 Implement `function nav_history_current()` — returns the top entry without removing it, or `null` if empty.
  - [ ] 2.6 Implement `function nav_history_can_go_back()` — returns `true` if the stack has 2 or more entries (current view + at least one previous).
  - [ ] 2.7 Implement `function get_current_scroll_position(callback)` — retrieves the current scroll position from the active webview via `runJavaScript("window.scrollY")` and passes the result to the callback. This is needed because `runJavaScript` is asynchronous.

- [ ] 3.0 Instrument navigation code paths to record history entries
  - [ ] 3.1 In `Component.onCompleted` (line ~817), after the initial tab is set up, push an initial history entry representing the first view so the stack starts with the current view on top.
  - [ ] 3.2 In `tab_checked_changed()` (line ~1932), before switching to the new tab's webview, push a new history entry of type `"tab_switch"` with the newly activated tab's metadata. Capture scroll position of the outgoing webview before pushing. Guard with `if (!is_navigating_back)`.
  - [ ] 3.3 In `show_result_in_html_view()` (line ~680), when content replaces an existing tab (not a new tab), push a history entry of type `"content_replace"` with the new item's metadata. Capture scroll position of the current webview before pushing. Guard with `if (!is_navigating_back)`.
  - [ ] 3.4 In `open_related_sutta()` (line ~462), when a related sutta replaces or adds a tab, push the appropriate history entry. Guard with `if (!is_navigating_back)`.
  - [ ] 3.5 Audit for any other code paths that change the active tab or replace tab content (e.g. session restore, bookmark opening, link clicking within webview) and add history push calls where appropriate.

- [ ] 4.0 Implement the back navigation restore logic
  - [ ] 4.1 Implement `function navigate_back()` — the core back navigation function:
    - Set `is_navigating_back = true`.
    - Update the current top entry's scroll position (async via `get_current_scroll_position`).
    - Pop the top entry (discard current view).
    - Read the new top entry (the view to restore).
    - Dispatch to the appropriate restore handler based on entry type.
    - Set `is_navigating_back = false` after restore completes.
  - [ ] 4.2 Implement tab switch restore: find the tab by `id_key` in the appropriate tab model (`tab_model` field). If found, call `focus_on_tab_with_id_key(id_key)`. If not found (tab was removed), re-create the tab in the correct model with the stored metadata, then focus on it.
  - [ ] 4.3 Implement content replacement restore: locate the tab by `id_key`, then reload the previous content using `SuttaBridge.get_sutta_html(window_id, item_uid)` or `SuttaBridge.get_word_html(window_id, item_uid)` based on `table_name`. Update the tab's metadata (`item_uid`, `table_name`, `sutta_ref`, `sutta_title`) in the model.
  - [ ] 4.4 Implement scroll position restore: after content is loaded, use the existing `window.scrollTo()` pattern (with `setTimeout` delay) to restore `scroll_position` from the history entry.
  - [ ] 4.5 Handle the async flow: since scroll position retrieval is async (`runJavaScript`), ensure the pop-and-restore sequence completes correctly. Consider updating the scroll position on the current top entry synchronously with the last known value if async retrieval is too complex.

- [ ] 5.0 Intercept the Android back button and wire it to the navigation logic
  - [ ] 5.1 Add an `onClosing` handler (or the approach determined in task 1.0) to `SuttaSearchWindow.qml`.
  - [ ] 5.2 Gate the handler on `is_mobile`: if `is_mobile` is `false`, allow normal close (`close.accepted = true`, no custom logic).
  - [ ] 5.3 When `is_mobile` and `nav_history_can_go_back()` is true: set `close.accepted = false` and call `navigate_back()`.
  - [ ] 5.4 When `is_mobile` and `nav_history_can_go_back()` is false: set `close.accepted = false` and open the confirmation dialog (task 6.0).

- [ ] 6.0 Add the "close app window" confirmation dialog
  - [ ] 6.1 Add a `Dialog` component (id: `close_window_dialog`) to `SuttaSearchWindow.qml`, following the existing dialog pattern (e.g. `related_sutta_not_found_dialog` at line ~1736). Use `standardButtons: Dialog.Ok | Dialog.Cancel`. Title/text: "No previous history item. Close the app window?"
  - [ ] 6.2 In `onAccepted`: close the window (call `close()` or `Qt.quit()` as appropriate for the window type).
  - [ ] 6.3 In `onRejected`: do nothing (dialog closes, user stays on current view).
  - [ ] 6.4 Verify that opening the dialog from the `onClosing` handler works correctly (the close event is already prevented by `close.accepted = false` at that point).
