# PRD: Tabs & History Dialog Refactor

## Introduction/Overview

The current mobile back button implementation uses a stack-based navigation history — pressing back pops entries and restores previous views. This PRD replaces that approach with a visual dialog that combines two functions: switching between open tabs (existing) and browsing view history (new). The back button on mobile will now ensure the HTML view panel is visible and open this dialog.

The existing "Select a Tab to Focus" dialog (`TabListDialog.qml`) is extended into a two-column layout: open tabs on the left and chronological view history on the right. The toggle button for this dialog is always visible (no longer hidden when tabs fit without overflow).

## Goals

1. Replace stack-based back navigation with a visual "Tabs & History" dialog triggered by the mobile back button.
2. Extend the tab list dialog with a "View History" column showing all navigation events in reverse chronological order (most recent on top).
3. Make the dialog toggle button always visible on both desktop and mobile.
4. Keep the existing navigation history tracking infrastructure but change how it's consumed — from stack-pop navigation to a browsable, append-only list.

## User Stories

1. **As a mobile user**, I want pressing the system back button to show the Tabs & History dialog so I can visually choose where to navigate.
2. **As a user**, I want to see my view history in reverse chronological order so I can find and reopen a previously viewed sutta or dictionary entry.
3. **As a user**, I want clicking a history item to open it without losing other history entries, so my full browsing trail is preserved.
4. **As a desktop user**, I want to access the same Tabs & History dialog via the toolbar button, so the feature is consistent across platforms.

## Functional Requirements

### Back Button Behavior Change (Mobile)

1. When the mobile back button is pressed, the app must ensure the HTML view panel is visible (i.e., if the sidebar is currently shown in mobile mode via `show_sidebar_btn`, toggle it so the HTML view panel is displayed).
2. After ensuring the HTML view panel is visible, open the "Tabs & History" dialog.
3. Remove the stack-based back navigation logic (`navigate_back()`, `nav_history_pop()`, `nav_history_can_go_back()`). The back button no longer pops history entries.
4. Remove the "Close the app window?" confirmation dialog (`close_window_dialog`). It is no longer needed since the back button always opens the Tabs & History dialog.

### Dialog Layout: Two-Column Design

5. Rename the dialog title from "Select a Tab to Focus" to "Tabs & History".
6. The dialog content area must have two columns side by side:
   - **Left column: "Open Tabs"** — shows the currently open tabs grouped by Pinned, Results, and Translations, exactly as the current `TabListDialog` does.
   - **Right column: "View History"** — shows the navigation history entries in reverse chronological order (most recent on top).
7. Each column must have a header label ("Open Tabs" and "View History").
8. Each column must be independently scrollable to accommodate many entries.
9. On narrow screens (mobile), the two columns should stack or use a reasonable minimum width so both remain usable. Consider using a minimum dialog width or allowing horizontal arrangement with equal splits.

### View History Column

10. The history list displays entries from the `nav_history` array, reversed (most recent first).
11. Each history item displays the `item_uid`. For DPD dictionary entries (`table_name === "dpd_headwords"`), display `sutta_title/dpd` instead.
12. Every visit is shown as a separate entry — no deduplication. This is a full chronological log.
13. Clicking a history item opens it: the app loads the item's content using the stored `item_uid` and `table_name` (same mechanism previously used by `restore_content_replace()` / `restore_tab_switch()`).
14. After opening a history item, a new entry is appended to the end of the history list as the most recent item. Entries are never discarded — the list grows for the session duration.
15. The `is_navigating_back` flag and the pop-based stack semantics are removed. Opening a history item is a regular navigation action that gets recorded.

### Toggle Button: Always Visible

16. Rename `tab_overflow_btn` to `tab_list_btn`.
17. Remove the conditional visibility (`visible: tabs_flickable.contentWidth > tabs_flickable.width`). The button must always be visible.
18. This applies to both desktop and mobile.

### History Recording (Unchanged)

19. Continue recording navigation events (tab switches, content replacements) into the `nav_history` array at the same code points as before.
20. Remove the `is_navigating_back` guard from `nav_history_push()` — all navigation events are recorded unconditionally.
21. The history array is session-only (cleared on app restart), append-only, and has no size limit.

## Non-Goals (Out of Scope)

- Persisting history across app restarts.
- Forward/backward keyboard shortcuts or toolbar buttons for history navigation.
- Search or filter within the history list.
- Grouping or categorizing history entries (they are a flat chronological list).
- Editing or deleting individual history entries.

## Design Considerations

- The two-column layout should use a `RowLayout` or `SplitView` inside the dialog's `contentItem`.
- Each column uses a `ListView` inside a `ScrollView` for independent scrolling.
- Column headers ("Open Tabs", "View History") should be `Label` elements above each list.
- The dialog width may need to increase from the current `Math.min(parent.width * 0.6, 500)` to accommodate two columns — consider `Math.min(parent.width * 0.85, 700)` or similar.
- The history column items should use the same delegate style as the open tabs column for visual consistency, but without the group label (Pinned/Results/Trans).
- Highlighted/selected item styling should match the existing pattern.

## Technical Considerations

### Code to Remove

- `navigate_back()` function
- `restore_tab_switch()` function  
- `restore_content_replace()` function
- `nav_history_pop()` function
- `nav_history_can_go_back()` function
- `is_navigating_back` property
- `close_window_dialog` Dialog component
- The `is_navigating_back` guard in `nav_history_push()`
- Back-navigation logic in the `onClosing` handler (replace with dialog-opening logic)

### Code to Modify

- `onClosing` handler in `SuttaSearchWindow.qml` — change from stack navigation to: ensure HTML view visible + open dialog.
- `tab_overflow_btn` → rename to `tab_list_btn`, remove conditional visibility.
- `TabListDialog.qml` — extend with two-column layout and history list.
- `TabListDialog` signal handling — add a new signal for history item selection, or reuse `tabSelected` with appropriate routing.

### Code to Keep

- `nav_history` array property
- `nav_history_push()` function (remove the `is_navigating_back` guard)
- `nav_history_current()` function (may still be useful)
- `build_nav_entry()` function
- `get_tab_model_name_for_id_key()` function
- `get_model_by_name()` function
- `restore_scroll_position()` function (reused when opening history items)
- All history recording call sites in `tab_checked_changed()`, `show_result_in_html_view()`, `open_related_sutta()`, `open_bookmark_in_tab_group()`, etc.

### Opening History Items

When a history item is clicked, the app needs to load its content. The logic is similar to the removed `restore_content_replace()` but without stack manipulation:
1. Find or create a tab for the item.
2. Load content via `get_sutta_html()` / `get_word_html()` using stored `item_uid` and `table_name`.
3. The navigation recording in the existing code paths will automatically append a new history entry.

### Passing History Data to the Dialog

The `TabListDialog` needs access to the `nav_history` array. Add a new required property (e.g., `required property var nav_history_model`) and pass `root.nav_history` from `SuttaSearchWindow.qml`. The dialog reads this on `onAboutToShow` to populate the history column.

## Success Metrics

- Mobile back button opens the Tabs & History dialog (no stack-based navigation).
- The HTML view panel is made visible before the dialog opens.
- The dialog shows open tabs on the left and view history on the right.
- Clicking an open tab focuses it (existing behavior preserved).
- Clicking a history item opens it and appends a new history entry.
- The toggle button is always visible on both desktop and mobile.
- No close confirmation dialog appears.
- Desktop behavior is enhanced (same dialog available via the always-visible button).

## Open Questions

None at this time.
