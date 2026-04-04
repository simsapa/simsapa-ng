## Relevant Files

- `assets/qml/SuttaSearchWindow.qml` - Main window: contains all navigation history logic, back button interception (`onClosing`), `close_window_dialog`, `tab_overflow_btn`, and the `TabListDialog` instance. Most changes happen here.
- `assets/qml/TabListDialog.qml` - The dialog component to refactor into a two-column "Tabs & History" layout.

### Notes

- The navigation history array (`nav_history`) and recording call sites are all in `SuttaSearchWindow.qml`.
- The `is_navigating_back` guard appears at 6 locations in `SuttaSearchWindow.qml` (lines 68, 705, 892, 1126, 1139, 2220) plus the flag itself at line 65.
- Run `make build -B` to verify compilation after each top-level task.
- Run `cd backend && cargo test` to verify Rust tests (no Rust changes expected, but good to confirm nothing breaks).
- Do not run `make qml-test` unless explicitly asked.

## Tasks

- [x] 1.0 Remove stack-based back navigation and close confirmation dialog
  - [x] 1.1 Remove the `is_navigating_back` property declaration (line 65).
  - [x] 1.2 Remove the `if (root.is_navigating_back) return;` guard from `nav_history_push()` (line 68). The function should now unconditionally push entries.
  - [x] 1.3 Remove the `navigate_back()` function (lines 139–161).
  - [x] 1.4 Remove the `restore_tab_switch()` function (lines 163–185).
  - [x] 1.5 Remove the `restore_content_replace()` function (lines 187–225).
  - [x] 1.6 Remove the `nav_history_pop()` function (lines 72–75).
  - [x] 1.7 Remove the `nav_history_can_go_back()` function (lines 82–84).
  - [x] 1.8 Remove all `if (!root.is_navigating_back)` guards around `nav_history_push()` calls. There are 5 sites — the `nav_history_push()` call should remain, only the `if` guard is removed:
    - `open_related_sutta()` around line 705
    - `show_result_in_html_view()` around line 892
    - `open_bookmark_in_tab_group()` pinned branch around line 1126
    - `open_bookmark_in_tab_group()` translations branch around line 1139
    - `tab_checked_changed()` around line 2220
  - [x] 1.9 Remove the `close_window_dialog` Dialog component (lines 1805–1819).
  - [x] 1.10 Simplify the `onClosing` handler (lines 19–29) to a temporary stub: on mobile, set `close.accepted = false` (the dialog wiring comes in task 2.0). Remove the `nav_history_can_go_back()` and `navigate_back()` calls and the `close_window_dialog.open()` call.
  - [x] 1.11 Verify the app compiles with `make build -B`.

- [x] 2.0 Change back button and toggle button behavior
  - [x] 2.1 Update the `onClosing` handler: when `is_mobile`, set `close.accepted = false`, then ensure the HTML view panel is visible by setting `show_sidebar_btn.checked = false`, then call `tab_list_dialog.open()`.
  - [x] 2.2 Rename `tab_overflow_btn` to `tab_list_btn` (id and all references in the file — check `enabled:` bindings on Shortcuts at lines ~1607, 1624, 1705, 1722 that reference `tab_list_dialog.visible`; these don't reference the button id but verify no other references exist).
  - [x] 2.3 Remove the conditional visibility on the button: change `visible: tabs_flickable.contentWidth > tabs_flickable.width` to `visible: true` (or simply remove the `visible` property since `true` is the default).
  - [x] 2.4 Verify the app compiles with `make build -B`.

- [x] 3.0 Extend TabListDialog with two-column layout and View History
  - [x] 3.1 Add a new required property `required property var nav_history` to `TabListDialog.qml`.
  - [x] 3.2 Pass `root.nav_history` from the `TabListDialog` instance in `SuttaSearchWindow.qml` (around line 2562).
  - [x] 3.3 Change the dialog title from `"Select a Tab to Focus"` to `"Tabs & History"`.
  - [x] 3.4 Increase the dialog width to accommodate two columns, e.g. `width: Math.min(parent.width * 0.85, 700)`.
  - [x] 3.5 Refactor the dialog `contentItem` from a single `ScrollView > ListView` to a `RowLayout` containing two columns, each with a header `Label` and a `ScrollView > ListView`.
  - [x] 3.6 Left column ("Open Tabs"): move the existing `combined_tabs_model` ListView here with its current delegate, highlight, and population logic. Add a `Label { text: "Open Tabs"; font.bold: true }` header above it.
  - [x] 3.7 Right column ("View History"): add a new `ListModel` (e.g., `history_list_model`) and a `ListView` with a delegate displaying `item_uid` (or `sutta_title/dpd` for DPD entries). Add a `Label { text: "View History"; font.bold: true }` header above it.
  - [x] 3.8 Implement `populate_history_model()` function: iterate `nav_history` in reverse order, appending each entry to `history_list_model` with fields needed for display (`item_uid`, `table_name`, `sutta_title`) and for opening (`item_uid`, `table_name`, `sutta_ref`, `sutta_title`, `id_key`, `tab_model`).
  - [x] 3.9 Call `populate_history_model()` from `onAboutToShow` alongside the existing `populate_model()`.
  - [x] 3.10 Add a new signal `signal historyItemSelected(string item_uid, string table_name, string sutta_ref, string sutta_title)` to the dialog.
  - [x] 3.11 Wire history item clicks: single click selects, double click (or an "Open" action) emits `historyItemSelected` with the item's data and closes the dialog.
  - [x] 3.12 Update the footer buttons: the "Open" button should work for whichever column has a selected item. Consider tracking which column was last interacted with, or having separate selection states per column.
  - [x] 3.13 Update keyboard navigation shortcuts (Up/Down/Home/End/Enter) to work within the currently focused column.
  - [x] 3.14 Verify the app compiles with `make build -B`.

- [x] 4.0 Implement history item opening logic
  - [x] 4.1 In `SuttaSearchWindow.qml`, connect the `onHistoryItemSelected` signal from `tab_list_dialog` to a new function `open_history_item(item_uid, table_name, sutta_ref, sutta_title)`.
  - [x] 4.2 Implement `open_history_item()`: construct a `result_data` object from the parameters and call `show_result_in_html_view(result_data, true)` to open it in a tab. The existing `show_result_in_html_view` already handles finding/creating tabs and recording navigation history, so no additional history push is needed.
  - [x] 4.3 After opening the item, ensure the HTML view panel is visible (sidebar hidden on narrow screens) — `show_result_in_html_view` already does `show_sidebar_btn.checked = false` when `!is_wide`, so verify this works correctly.
  - [x] 4.4 Verify the app compiles with `make build -B`.
