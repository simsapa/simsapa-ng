# PRD: Bookmarks Tab for SuttaSearchWindow

## 1. Introduction/Overview

The Bookmarks feature adds a new tab to `SuttaSearchWindow` that lets users save, organize, and restore open items (suttas, dictionary pages, book chapters) across sessions. Users can group bookmarks into named folders, reorder them via drag-and-drop, and restore entire folders to their original tab groups with preserved scroll position and find-bar state. On exit, the app automatically saves all open items as a "Last Session" folder for easy recovery.

## 2. Goals

1. Allow users to persistently save references to open items (suttas, dict words, book chapters) with their full viewing state.
2. Provide folder-based organization with drag-and-drop reordering.
3. Show currently open items from all `SuttaSearchWindow` instances for easy bookmarking.
4. Automatically preserve the last session's open items on app exit.
5. Enable one-click restoration of individual bookmarks or entire folders to the correct tab groups.

## 3. User Stories

- **As a student**, I want to save a set of related suttas and their translations as a folder so I can return to my study session later with everything in place.
- **As a researcher**, I want to bookmark a sutta with my find query active (e.g., match 2 of 5 for "dukkha") so I can resume exactly where I left off.
- **As a reader**, I want the app to remember what I had open when I last closed it, so I don't lose my place.
- **As a user with multiple windows**, I want to see what's open in all my windows from the Bookmarks tab and selectively save items.
- **As an organizer**, I want to drag bookmarks to reorder them within a folder and move items between folders using checkboxes.

## 4. Functional Requirements

### 4.1 Data Model

Each bookmark item stores:

| Field | Type | Description |
|-------|------|-------------|
| `id` | INTEGER | Primary key |
| `folder_id` | INTEGER | FK to bookmark folder |
| `item_uid` | VARCHAR | UID of the item (e.g., `sn56.11/pli/ms`, `pabbajita 1/dpd`) |
| `table_name` | VARCHAR | Source table: `suttas`, `dict_words`, `book_spine_items`, `dpd_headwords` |
| `title` | VARCHAR | Display title of the item |
| `tab_group` | VARCHAR | Which tab group: `pinned`, `results`, `translations` |
| `scroll_position` | REAL | Vertical scroll position (0.0–1.0 ratio or pixel offset) |
| `find_query` | VARCHAR | The text in the HTML find bar (empty string if none) |
| `find_match_index` | INTEGER | Which match was active (e.g., 2 in "2 of 5"), 0 if none |
| `sort_order` | INTEGER | Position within the folder for ordering |
| `created_at` | DATETIME | Timestamp |
| `updated_at` | DATETIME | Timestamp |

Each bookmark folder stores:

| Field | Type | Description |
|-------|------|-------------|
| `id` | INTEGER | Primary key |
| `name` | VARCHAR | Folder display name |
| `sort_order` | INTEGER | Position among folders for ordering |
| `is_last_session` | BOOLEAN | Whether this is an auto-generated "Last Session" folder |
| `created_at` | DATETIME | Timestamp |
| `updated_at` | DATETIME | Timestamp |

### 4.2 Bookmarks Tab UI

1. The Bookmarks tab must appear as a new tab in `SuttaSearchWindow`, alongside existing tabs (Search, TOC, etc.).
2. The tab has two main sections:
   - **Top section:** "Currently Open Items" — shows items from all open `SuttaSearchWindow` instances.
   - **Bottom section:** "Saved Bookmarks" — shows all bookmark folders with their items.

### 4.3 Currently Open Items Section

3. Items must be grouped by window (e.g., "Window 1", "Window 2").
4. Each item displays its `uid` and `title`.
5. Each item has a checkbox on its left side for selection.
6. Below the items list, provide:
   - A **"Save All as Folder"** button that saves all currently open items (across all windows) into a new folder. The folder name input is pre-filled with `"Saved YYYY-MM-DD HH:MM:SS"` (current date/time) and the user can edit it before confirming.
   - A **"Save Selected to Folder"** button (enabled when at least one checkbox is checked) that opens a dropdown/picker to choose an existing folder, then adds the selected items to that folder.

### 4.4 Saved Bookmarks Section

7. Folders are displayed as collapsible lists, following the same visual pattern as the TOC tab's `BooksList` / `ChapterListItem` components.
8. Each folder header shows:
   - Expand/collapse toggle arrow
   - Folder name
   - An **"Open All"** button — restores all items in the folder to the current `SuttaSearchWindow`, each to its correct tab group
   - An **"Edit"** button (pencil icon) — opens a dialog to rename the folder
   - A **"Delete"** button (trash icon) — deletes the folder and all its bookmarks (with confirmation dialog)

9. Each bookmark item within a folder shows:
   - A checkbox on the left (for multi-select/move operations)
   - A badge indicating the tab group (`PINNED`, `RESULT`, `TRANSLATION`), styled similarly to the `EPUB`/`PDF` badges on books
   - The item `uid`
   - The item `title`
   - An **"Open"** button — opens this single item in the correct tab group of the current `SuttaSearchWindow`
   - An **"Edit"** button (pencil icon) — opens a dialog to edit the bookmark's fields (uid, title, tab_group, find_query, find_match_index)
   - A **"Delete"** button (trash icon) — removes the bookmark item (no confirmation needed for single items)

### 4.5 Drag-and-Drop Reordering

10. Bookmark items must be draggable within their folder to reorder them. Drag updates `sort_order` in the database.
11. Folders must be draggable to reorder among other folders. Drag updates folder `sort_order`.
12. Items are **not** draggable between folders. Moving between folders uses the checkbox + "Move Here" UX (see below).

### 4.6 Move Items Between Folders

13. When one or more bookmark items are selected via checkboxes, each folder header shows a **"Move Here"** button.
14. Clicking "Move Here" on a folder moves all selected items into that folder, appended at the end.
15. After the move, checkboxes are deselected and "Move Here" buttons are hidden.

### 4.7 Opening / Restoring Bookmarks

16. Opening a single bookmark item must:
    - Add the item to the correct tab group (`pinned`, `results`, or `translations`) in the current `SuttaSearchWindow`.
    - Restore the scroll position after the page loads.
    - If `find_query` is non-empty, activate the find bar with the query and navigate to `find_match_index`.

17. Opening all items in a folder ("Open All") must restore each item to its correct tab group, in the order they appear in the folder. Only the first item from the `results` group should be made the active tab (with its webview instantiated); other tabs are added but remain lazy-loaded until clicked.

### 4.8 Last Session Auto-Save

18. When the application exits, it must save all open items from every `SuttaSearchWindow` into folders named `"Last Session - Window 1"`, `"Last Session - Window 2"`, etc.
19. These folders are marked with `is_last_session = true`.
20. On each subsequent exit, all existing `is_last_session` folders are deleted and replaced with the current session's items.
21. Last Session folders appear in the Saved Bookmarks section like regular folders, and can be opened the same way.

### 4.9 Restore Last Session Setting

22. Add a **"Restore Last Session on Startup"** checkbox in `AppSettingsWindow.qml` (in the General tab).
23. Default value: `true`.
24. When enabled, on app startup, if Last Session folders exist, automatically open all their items in new `SuttaSearchWindow` instances (one window per folder).
25. Add `restore_last_session: bool` field to `AppSettings` struct.

### 4.10 Backend (Rust)

26. Create a new Diesel migration adding `bookmark_folders` and `bookmark_items` tables to the appdata database.
27. Add CRUD functions in the backend for bookmark folders and items (create, read, update, delete, reorder).
28. Expose bookmark operations through `SuttaBridge` as `#[qinvokable]` functions:
    - `get_all_bookmark_folders_json() -> QString`
    - `get_bookmark_items_for_folder_json(folder_id: i32) -> QString`
    - `create_bookmark_folder(name: QString) -> i32` (returns new folder ID)
    - `create_bookmark_item(folder_id: i32, item_json: QString) -> i32`
    - `update_bookmark_folder(folder_id: i32, name: QString)`
    - `update_bookmark_item(item_id: i32, item_json: QString)`
    - `delete_bookmark_folder(folder_id: i32)`
    - `delete_bookmark_item(item_id: i32)`
    - `reorder_bookmark_items(folder_id: i32, item_ids_json: QString)` — accepts ordered list of IDs
    - `reorder_bookmark_folders(folder_ids_json: QString)`
    - `move_bookmark_items_to_folder(item_ids_json: QString, target_folder_id: i32)`
    - `save_last_session(windows_json: QString)` — saves all open items, replacing previous Last Session folders
    - `get_last_session_json() -> QString`
    - `get_restore_last_session() -> bool`
    - `set_restore_last_session(value: bool)`

### 4.11 QML Components

29. Create `BookmarksTab.qml` — the main Bookmarks tab component.
30. Create `BookmarkFolderItem.qml` — collapsible folder header with action buttons.
31. Create `BookmarkListItem.qml` — individual bookmark item display with badge, buttons, and checkbox.
32. Create `BookmarkEditDialog.qml` — dialog for editing a bookmark item's fields.
33. Create `BookmarkFolderDialog.qml` — dialog for creating/renaming a folder.
34. Register all new QML files in `bridges/build.rs` `qml_files` list.

### 4.12 Scroll Position Capture

35. When saving a bookmark (either manually or during Last Session save), the current scroll position of the item's webview must be captured. Use JavaScript evaluation on the WebEngineView to get `window.scrollY` or equivalent.
36. When restoring, after the page loads, inject JavaScript to scroll to the saved position.

### 4.13 Find Bar State Capture

37. When saving a bookmark, capture the current find bar query text and match index from the `SuttaSearchWindow` find bar state.
38. When restoring, set `pending_find_query` to the saved query and, after the find completes, advance to the saved match index using the existing find-next mechanism.

## 5. Non-Goals (Out of Scope)

- Cloud sync or cross-device bookmark sharing.
- Nested folders (folders within folders) — only one level of folders.
- Import/export of bookmarks to external formats.
- Bookmark tagging or search within bookmarks.
- Keyboard shortcuts for bookmark operations (can be added later).

## 6. Design Considerations

- **Collapsible lists** should reuse the visual patterns from `BooksList.qml` and `ChapterListItem.qml` in the TOC tab — same expand/collapse arrows, indentation, and styling.
- **Tab group badges** (`PINNED`, `RESULT`, `TRANSLATION`) should use the same badge styling as `EPUB`/`PDF` badges on book items.
- The Bookmarks tab icon could be a star or bookmark ribbon icon.
- The "Currently Open Items" section should have a subtle visual separator from the "Saved Bookmarks" section.
- Drag handles (grip dots icon) should appear on the left of items and folders to indicate draggability.
- The "Move Here" buttons on folders should be visually distinct (e.g., highlighted/pulsing) when items are selected, to guide the user.

## 7. Technical Considerations

- **Database:** New tables in the appdata SQLite database via Diesel migration. Follows the existing migration pattern in `backend/migrations/appdata/`.
- **Bridge pattern:** Follow existing `sutta_bridge.rs` patterns — `#[qinvokable]` functions returning `QString` JSON for complex data, simple types for scalars.
- **Multi-window state:** `SuttaSearchWindow` instances need a way to report their open items. The bridge likely needs a function that accepts a `window_id` and returns the items for that window. Use the existing `window_id` property on `SuttaSearchWindow`.
- **Tab data extraction:** The currently open items can be read from the `tabs_pinned_model`, `tabs_results_model`, and `tabs_translations_model` ListModels in each `SuttaSearchWindow`.
- **Scroll position JS:** Store scroll position as a ratio (`window.scrollY / document.documentElement.scrollHeight`). Restore with `window.scrollTo(0, ratio * document.documentElement.scrollHeight)`. Ratio-based storage works reliably across mobile and desktop window sizes.
- **`qmllint` type definitions:** Create corresponding `.qml` type stubs for any new bridge methods, per project conventions.
- **`qmldir` updates:** If any new singleton or type is added, update `assets/qml/com/profoundlabs/simsapa/qmldir`.

## 8. Success Metrics

- Users can save and restore bookmarks across app sessions without data loss.
- Last Session restore correctly reopens all items in the right tab groups with scroll positions preserved.
- Drag-and-drop reordering persists correctly after app restart.
- No performance degradation with up to 100 folders and 1000 total bookmark items.

## 9. Resolved Questions

1. **Scroll position format:** Use ratio (`scrollY / scrollHeight`). Works reliably across mobile and desktop.
2. **Find match index reliability:** Not a concern — accept that the index may shift if content changes.
3. **Maximum folder/item limits:** No limits imposed; rely on practical performance.
4. **Confirmation on "Open All":** No confirmation needed. Only the active tab instantiates a webview; the rest are lazy-loaded. When opening a folder, the first `results` group item becomes active.
5. **Duplicate handling:** Allow duplicates — the same uid can appear in multiple folders or multiple times in the same folder.
