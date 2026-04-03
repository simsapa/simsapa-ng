# PRD: Mobile Back Button Navigation History

## Introduction/Overview

When the Android system back button is pressed, the app currently closes the SuttaSearchWindow. Instead, it should behave like a browser's back button — navigating to the previously viewed item (sutta, dictionary word, book chapter, etc.) within that window.

This requires two things:
1. Intercepting the mobile OS back button press in the QML layer.
2. Maintaining a per-window navigation history stack that tracks tab switches and content changes.

## Goals

- Intercept the Android system back button and override the default close behavior with custom navigation logic.
- Maintain a navigation history stack per SuttaSearchWindow that records what the user has viewed.
- Navigate backward through the history when the back button is pressed, restoring the previous view (tab activation or content reload).
- When history is exhausted, prompt the user before closing the window.

## User Stories

1. **As a mobile user**, I want pressing the system back button to show the previous sutta I was reading, so I can quickly return to where I was.
2. **As a mobile user**, I want pressing back after switching tabs to re-activate the previous tab, so that tab switching is reversible.
3. **As a mobile user**, I want to be asked before the app window closes when I press back with no history, so I don't accidentally lose my session.

## Functional Requirements

### Back Button Interception

1. The app must intercept the Android system back button press on `SuttaSearchWindow`.
2. In Qt6/QML, this is done by handling the `onClosing` event on the `ApplicationWindow` and setting `close.accepted = false` when there is history to navigate, or by using `Keys.onBackPressed`. Research the recommended Qt6 approach for Android back button handling.
3. This feature applies only to mobile platforms (`is_mobile === true`). Desktop behavior is unchanged.

### Navigation History Stack

4. Each `SuttaSearchWindow` must maintain its own independent navigation history stack (QML-side, in-memory).
5. The history stack is session-only — it is cleared when the app restarts.
6. The history stack has unlimited depth for the duration of the session.
7. Only the main content area (sutta/result tabs) is tracked. Sidebar tab switches (Dictionary, Glossary, Prompts, etc.) are not tracked.

### History Entry Types

8. A history entry records the state needed to restore a view. There are two types of navigation events to track:

   **a) Tab switch:** The user clicks a different tab in the tab bar.
   - History entry stores: the previously active tab's `id_key` (and which tab model it belongs to: pinned, results, or translations).

   **b) Content replacement:** The user opens a new item (from search results, related suttas, etc.) which replaces the content of the current tab.
   - History entry stores: the tab's `id_key`, the previous item's `uid`, `table_name`, `sutta_ref`, `sutta_title`, and enough metadata to re-render the content.

### Back Navigation Behavior

9. The history stack operates so that the top entry always represents the current view. When the user presses the system back button and there are at least 2 entries in the stack:
   - Pop the top entry (the current view) — it is discarded.
   - The new top entry is now the view to restore.
   - If it was a **tab switch**: re-activate the previous tab. If that tab was removed in the meantime, re-add it to the appropriate tab model and activate it.
   - If it was a **content replacement**: reload the previous item's content into the same tab (by calling `get_sutta_html` / `get_word_html` or equivalent using the stored `uid` and `table_name`).
   - Restore the scroll position from the entry.

10. When the user presses the system back button and the history stack has 0 or 1 entries (no previous view):
    - Show a dialog: "No previous history item. Close the app window?" with Cancel and OK buttons.
    - If OK: close the window.
    - If Cancel: do nothing.

### History Recording

11. Every navigation action (tab switch, content replacement) pushes a new entry representing the **new** view onto the stack. On the very first view shown in a window, an initial entry must also be pushed so the stack always has the current view on top.
12. History entries must be recorded in these existing code paths:
    - `tab_checked_changed()` — when the user switches tabs (lines ~1932-1990 in SuttaSearchWindow.qml).
    - `show_result_in_html_view()` — when a search result replaces content in an existing tab (lines ~680-730).
    - `open_related_sutta()` — when opening commentary/root text (lines ~462-475).
    - Any other path that changes the active tab or replaces tab content.

## Non-Goals (Out of Scope)

- **Forward navigation** is not included. This is back-only.
- **Desktop back button** or in-app back/forward toolbar buttons are not included. This feature is triggered exclusively by the mobile OS system back button.
- **Sidebar tab history** (Dictionary, Glossary, Prompts, Table of Contents, Bookmarks, Query panel) is not tracked.
- **Persisting history** across app restarts is not included.
- **Cross-window history** is not included. Each window's history is independent.

## Design Considerations

- The confirmation dialog ("No previous history item. Close the app window?") should use a standard Qt Quick `Dialog` or `MessageDialog` component, consistent with existing dialogs in the app.
- No visible UI changes are needed — the feature is entirely driven by the system back button.

## Technical Considerations

### Qt6 Android Back Button

- In Qt6 for Android, the system back button can be intercepted via the `onClosing` handler on `ApplicationWindow`. Set `close.accepted = false` to prevent the default close, then run custom logic.
- Alternative: `Keys.onBackPressed` or handling `Qt.Key_Back` in a key handler. The `onClosing` approach is generally more reliable for top-level windows.
- This should be gated on `is_mobile` so desktop close behavior is unaffected.

### History Stack Data Structure

- A simple JavaScript array in QML acting as a stack (push/pop) is sufficient.
- Each entry is an object like:
  ```
  {
    type: "tab_switch" | "content_replace",
    tab_model: "pinned" | "results" | "translations",
    id_key: "...",
    scroll_position: 0,  // vertical scroll offset to restore
    // For content_replace only:
    item_uid: "...",
    table_name: "...",
    sutta_ref: "...",
    sutta_title: "..."
  }
  ```

### Restoring Removed Tabs

- When navigating back to a tab that was closed/removed, the tab must be re-added to its original tab model with the stored metadata, then activated and its content loaded.

### Existing Functions to Leverage

- `focus_on_tab_with_id_key(id_key)` — for re-activating a tab.
- `get_sutta_html(window_id, uid)` / `get_word_html(window_id, uid)` — for reloading content.
- `add_results_tab()` — for re-adding a removed tab.

## Success Metrics

- Pressing the Android back button navigates to the previous view instead of closing the window.
- Tab switches and content replacements are correctly restored.
- The confirmation dialog appears when history is empty.
- Desktop behavior is completely unaffected.

## Resolved Questions

1. **Scroll position:** Yes — save the vertical scroll position in each history entry and restore it when navigating back.
2. **Stack semantics after going back:** The history is a simple stack. Going back pops the top entry, so the new top is the currently displayed view. If the user then performs a new navigation action (opens a search result, switches tabs), that action pushes a new entry onto the stack after the current position. There is no forward history — going back and then navigating somewhere new simply continues the stack from the current point.

## Open Questions

None at this time.
