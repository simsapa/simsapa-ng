## Relevant Files

- `backend/migrations/appdata/2026-04-02-120000_create_bookmarks/up.sql` - Diesel migration creating `bookmark_folders` and `bookmark_items` tables
- `backend/migrations/appdata/2026-04-02-120000_create_bookmarks/down.sql` - Diesel migration rollback
- `backend/src/db/appdata_schema.rs` - Diesel schema (auto-generated, needs `diesel::table!` entries for new tables)
- `backend/src/db/appdata_models.rs` - Queryable and Insertable model structs for bookmark tables
- `backend/src/db/appdata.rs` - CRUD operations for bookmark folders and items
- `backend/src/app_settings.rs` - `AppSettings` struct — add `restore_last_session` field
- `backend/src/app_data.rs` - Getter/setter for `restore_last_session`, last session save logic
- `bridges/src/sutta_bridge.rs` - `#[qinvokable]` functions for all bookmark operations exposed to QML
- `bridges/build.rs` - Register new QML files in `qml_files` list
- `assets/qml/BookmarksTab.qml` - Main Bookmarks tab component (new file)
- `assets/qml/BookmarkFolderItem.qml` - Collapsible folder header with action buttons (new file)
- `assets/qml/BookmarkListItem.qml` - Individual bookmark item display (new file)
- `assets/qml/BookmarkEditDialog.qml` - Dialog for editing a bookmark item (new file)
- `assets/qml/BookmarkFolderDialog.qml` - Dialog for creating/renaming a folder (new file)
- `assets/qml/SuttaSearchWindow.qml` - Add Bookmarks tab to TabBar and StackLayout, add session state collection functions
- `assets/qml/AppSettingsWindow.qml` - Add "Restore Last Session" checkbox in General tab
- `assets/qml/com/profoundlabs/simsapa/SuttaBridge.qml` - Add qmllint type stubs for new bridge functions
- `assets/qml/com/profoundlabs/simsapa/qmldir` - No changes expected (SuttaBridge is already registered)
- `cpp/gui.cpp` - Hook into `aboutToQuit` signal to trigger last session save
- `cpp/window_manager.h` - Add method to collect open window state
- `cpp/window_manager.cpp` - Implement session state collection and restore-on-startup logic

### Notes

- Run `cd backend && cargo test` to verify Rust backend changes compile and pass.
- Run `make build -B` to verify full build after QML/bridge changes.
- Run `make qml-test` to run QML tests after UI changes.
- New QML files must be added to `qml_files` in `bridges/build.rs`.
- New bridge functions need corresponding stubs in `SuttaBridge.qml` for qmllint.
- Diesel schema is regenerated with `diesel print-schema` or by running the migration; verify `appdata_schema.rs` is updated.

## Tasks

- [ ] 1.0 Database: Create Diesel migration and models for bookmark tables
  - [ ] 1.1 Create migration directory `backend/migrations/appdata/2026-04-02-120000_create_bookmarks/`
  - [ ] 1.2 Write `up.sql` creating `bookmark_folders` table with columns: `id` (INTEGER PK), `name` (VARCHAR NOT NULL), `sort_order` (INTEGER NOT NULL DEFAULT 0), `is_last_session` (BOOLEAN NOT NULL DEFAULT 0), `created_at` (DATETIME DEFAULT CURRENT_TIMESTAMP), `updated_at` (DATETIME). Add index on `sort_order`.
  - [ ] 1.3 Write `up.sql` creating `bookmark_items` table with columns: `id` (INTEGER PK), `folder_id` (INTEGER NOT NULL, FK to `bookmark_folders` ON DELETE CASCADE), `item_uid` (VARCHAR NOT NULL), `table_name` (VARCHAR NOT NULL), `title` (VARCHAR), `tab_group` (VARCHAR NOT NULL), `scroll_position` (REAL NOT NULL DEFAULT 0.0), `find_query` (VARCHAR NOT NULL DEFAULT ''), `find_match_index` (INTEGER NOT NULL DEFAULT 0), `sort_order` (INTEGER NOT NULL DEFAULT 0), `created_at` (DATETIME DEFAULT CURRENT_TIMESTAMP), `updated_at` (DATETIME). Add indexes on `folder_id` and `sort_order`.
  - [ ] 1.4 Write `down.sql` to drop both tables (items first, then folders).
  - [ ] 1.5 Run the migration and update `backend/src/db/appdata_schema.rs` with the new `diesel::table!` entries and `joinable!`/`allow_tables_to_appear_in_same_query!` macros.
  - [ ] 1.6 Add Queryable model structs (`BookmarkFolder`, `BookmarkItem`) and Insertable structs (`NewBookmarkFolder`, `NewBookmarkItem`) to `backend/src/db/appdata_models.rs`. Add `Serialize, Deserialize` derives for JSON serialization.
  - [ ] 1.7 Verify `cd backend && cargo test` passes.

- [ ] 2.0 Backend: Implement bookmark CRUD operations in appdata database manager
  - [ ] 2.1 In `backend/src/db/appdata.rs`, add `get_all_bookmark_folders()` — returns `Vec<BookmarkFolder>` ordered by `sort_order`.
  - [ ] 2.2 Add `get_bookmark_items_for_folder(folder_id: i32)` — returns `Vec<BookmarkItem>` filtered by `folder_id`, ordered by `sort_order`.
  - [ ] 2.3 Add `create_bookmark_folder(name: &str) -> Result<i32>` — inserts folder, sets `sort_order` to max+1, returns new ID. Use `diesel::insert_into` with `RETURNING id` or re-query.
  - [ ] 2.4 Add `create_bookmark_item(new_item: &NewBookmarkItem) -> Result<i32>` — inserts item, sets `sort_order` to max+1 within folder, returns new ID.
  - [ ] 2.5 Add `update_bookmark_folder(id: i32, name: &str) -> Result<()>` — updates folder name and `updated_at`.
  - [ ] 2.6 Add `update_bookmark_item(id: i32, item_data: &BookmarkItemUpdate) -> Result<()>` — updates editable fields (`item_uid`, `title`, `tab_group`, `find_query`, `find_match_index`) and `updated_at`. Define a `BookmarkItemUpdate` struct for the updatable fields.
  - [ ] 2.7 Add `delete_bookmark_folder(id: i32) -> Result<()>` — deletes folder (cascade deletes items via FK).
  - [ ] 2.8 Add `delete_bookmark_item(id: i32) -> Result<()>` — deletes single bookmark item.
  - [ ] 2.9 Add `reorder_bookmark_items(folder_id: i32, item_ids: &[i32]) -> Result<()>` — updates `sort_order` for each item ID based on position in the array.
  - [ ] 2.10 Add `reorder_bookmark_folders(folder_ids: &[i32]) -> Result<()>` — updates `sort_order` for each folder ID based on position in the array.
  - [ ] 2.11 Add `move_bookmark_items_to_folder(item_ids: &[i32], target_folder_id: i32) -> Result<()>` — updates `folder_id` for each item, appends at end of target folder's sort order.
  - [ ] 2.12 Add `delete_last_session_folders() -> Result<()>` — deletes all folders where `is_last_session = true` (cascade deletes their items).
  - [ ] 2.13 Add `get_last_session_folders() -> Vec<BookmarkFolder>` — returns folders where `is_last_session = true`, ordered by `sort_order`.
  - [ ] 2.14 Write unit tests for CRUD operations: create folder, create items, reorder, move, delete, last session lifecycle.
  - [ ] 2.15 Verify `cd backend && cargo test` passes.

- [ ] 3.0 Backend: Add `restore_last_session` to AppSettings
  - [ ] 3.1 Add `restore_last_session: bool` field to `AppSettings` struct in `backend/src/app_settings.rs` with `#[serde(default = "default_true")]` (default `true`). Add the `default_true` helper function if it doesn't exist.
  - [ ] 3.2 Add `set_restore_last_session(enabled: bool)` method in `backend/src/app_data.rs` following the existing `set_notify_about_simsapa_updates` pattern: update cache, serialize to JSON, write to DB.
  - [ ] 3.3 Verify `cd backend && cargo test` passes.

- [ ] 4.0 Bridge: Expose bookmark operations as `#[qinvokable]` functions on SuttaBridge
  - [ ] 4.1 Add `get_all_bookmark_folders_json(&self) -> QString` — calls `get_all_bookmark_folders()`, serializes to JSON, returns `QString`. Follow the `get_all_books_json` pattern.
  - [ ] 4.2 Add `get_bookmark_items_for_folder_json(&self, folder_id: i32) -> QString` — same pattern.
  - [ ] 4.3 Add `create_bookmark_folder(self: Pin<&mut Self>, name: &QString) -> i32` — calls backend, returns new folder ID (or -1 on error).
  - [ ] 4.4 Add `create_bookmark_item(self: Pin<&mut Self>, folder_id: i32, item_json: &QString) -> i32` — parses JSON into `NewBookmarkItem`, calls backend, returns new item ID.
  - [ ] 4.5 Add `update_bookmark_folder(self: Pin<&mut Self>, folder_id: i32, name: &QString)` — calls backend.
  - [ ] 4.6 Add `update_bookmark_item(self: Pin<&mut Self>, item_id: i32, item_json: &QString)` — parses JSON into `BookmarkItemUpdate`, calls backend.
  - [ ] 4.7 Add `delete_bookmark_folder(self: Pin<&mut Self>, folder_id: i32)` — calls backend.
  - [ ] 4.8 Add `delete_bookmark_item(self: Pin<&mut Self>, item_id: i32)` — calls backend.
  - [ ] 4.9 Add `reorder_bookmark_items(self: Pin<&mut Self>, folder_id: i32, item_ids_json: &QString)` — parses JSON array of IDs, calls backend.
  - [ ] 4.10 Add `reorder_bookmark_folders(self: Pin<&mut Self>, folder_ids_json: &QString)` — parses JSON array of IDs, calls backend.
  - [ ] 4.11 Add `move_bookmark_items_to_folder(self: Pin<&mut Self>, item_ids_json: &QString, target_folder_id: i32)` — calls backend.
  - [ ] 4.12 Add `save_last_session(self: Pin<&mut Self>, windows_json: &QString)` — parses JSON (array of windows, each with array of tab items), calls `delete_last_session_folders()`, then creates new folders and items for each window.
  - [ ] 4.13 Add `get_last_session_json(&self) -> QString` — returns Last Session folders with their items as JSON.
  - [ ] 4.14 Add `get_restore_last_session(&self) -> bool` and `set_restore_last_session(self: Pin<&mut Self>, value: bool)` — read/write from AppSettings cache.
  - [ ] 4.15 Add corresponding function stubs for all new bridge functions in `assets/qml/com/profoundlabs/simsapa/SuttaBridge.qml` with correct signatures and simple return values.
  - [ ] 4.16 Verify `make build -B` compiles successfully.

- [ ] 5.0 QML: Create BookmarksTab with saved bookmarks section (folders + items, collapsible lists)
  - [ ] 5.1 Create `assets/qml/BookmarksTab.qml` with the main layout: a `ColumnLayout` with two sections separated by a visual divider — "Currently Open Items" (top, placeholder for now) and "Saved Bookmarks" (bottom).
  - [ ] 5.2 In the Saved Bookmarks section, add a `ListView` backed by a `ListModel` (`bookmark_folders_model`). Each delegate is a `BookmarkFolderItem`.
  - [ ] 5.3 Add a `load_bookmarks()` function that calls `SuttaBridge.get_all_bookmark_folders_json()`, parses the JSON, and populates `bookmark_folders_model`. For each folder, also load its items via `SuttaBridge.get_bookmark_items_for_folder_json(folder_id)`.
  - [ ] 5.4 Create `assets/qml/BookmarkFolderItem.qml` — collapsible folder header following the `BooksList.qml` pattern. Show expand/collapse arrow, folder name, "Open All" button, edit (pencil) icon button, delete (trash) icon button. Track `is_expanded` state.
  - [ ] 5.5 When expanded, show a nested `Repeater` or `Column` of `BookmarkListItem` delegates for the folder's items.
  - [ ] 5.6 Create `assets/qml/BookmarkListItem.qml` — shows checkbox (left), tab group badge (`PINNED`/`RESULT`/`TRANSLATION`), item uid, title, "Open" button, edit (pencil) icon, delete (trash) icon. Style badges like the `EPUB`/`PDF` badges in `ChapterListItem`.
  - [ ] 5.7 Wire delete buttons: folder delete shows a confirmation dialog, then calls `SuttaBridge.delete_bookmark_folder(folder_id)` and refreshes the list. Item delete calls `SuttaBridge.delete_bookmark_item(item_id)` directly.
  - [ ] 5.8 Register `BookmarksTab.qml`, `BookmarkFolderItem.qml`, `BookmarkListItem.qml` in `bridges/build.rs` `qml_files` list.
  - [ ] 5.9 Verify `make build -B` compiles successfully.

- [ ] 6.0 QML: Create currently open items section with multi-window awareness
  - [ ] 6.1 In `SuttaSearchWindow.qml`, add a function `get_open_items_json() -> string` that iterates `tabs_pinned_model`, `tabs_results_model`, and `tabs_translations_model`, collects each item's `{item_uid, table_name, sutta_title, sutta_ref, tab_group}` (where `tab_group` is `"pinned"`, `"results"`, or `"translations"` respectively), and returns a JSON array string.
  - [ ] 6.2 Add a `#[qinvokable]` function `get_all_windows_open_items_json()` on `SuttaBridge` that iterates all `SuttaSearchWindow` instances (via C++ callback or by having each window register its items). Alternatively, handle this entirely in QML by having `BookmarksTab` request items from all windows via a signal/callback pattern.
  - [ ] 6.3 In `BookmarksTab.qml`, implement the "Currently Open Items" top section. Group items by window (e.g., "Window 1", "Window 2" headers). Each item shows uid, title, and a checkbox.
  - [ ] 6.4 Add a "Save All as Folder" button below the open items list. On click, open `BookmarkFolderDialog` pre-filled with `"Saved YYYY-MM-DD HH:MM:SS"`. On confirm, call `SuttaBridge.create_bookmark_folder(name)`, then `SuttaBridge.create_bookmark_item(...)` for each open item, then refresh the saved bookmarks list.
  - [ ] 6.5 Add a "Save Selected to Folder" button (enabled when any checkbox is checked). On click, show a dropdown/picker listing existing folders. On selection, call `SuttaBridge.create_bookmark_item(...)` for each checked item, then refresh.
  - [ ] 6.6 Verify `make build -B` compiles successfully.

- [ ] 7.0 QML: Implement bookmark edit/create dialogs
  - [ ] 7.1 Create `assets/qml/BookmarkFolderDialog.qml` — a `Dialog` with a `TextField` for folder name. Used for both creating new folders and renaming existing ones. Accept a `folder_id` prop (0 for new). On accept, call `SuttaBridge.create_bookmark_folder(name)` or `SuttaBridge.update_bookmark_folder(folder_id, name)`.
  - [ ] 7.2 Create `assets/qml/BookmarkEditDialog.qml` — a `Dialog` with fields for: `item_uid` (TextField), `title` (TextField), `tab_group` (ComboBox with options: pinned, results, translations), `find_query` (TextField), `find_match_index` (SpinBox). Populate from existing item data. On accept, call `SuttaBridge.update_bookmark_item(item_id, item_json)`.
  - [ ] 7.3 Wire the edit button on `BookmarkFolderItem` to open `BookmarkFolderDialog` in rename mode.
  - [ ] 7.4 Wire the edit button on `BookmarkListItem` to open `BookmarkEditDialog` populated with that item's data.
  - [ ] 7.5 Register `BookmarkEditDialog.qml` and `BookmarkFolderDialog.qml` in `bridges/build.rs` `qml_files` list.
  - [ ] 7.6 Verify `make build -B` compiles successfully.

- [ ] 8.0 QML: Implement drag-and-drop reordering for items and folders
  - [ ] 8.1 Add `DragHandler` (or use `drag.target` with `Drag` and `DropArea`) to `BookmarkListItem.qml` for reordering items within a folder. Show a drag handle grip icon on the left side of each item.
  - [ ] 8.2 On drop, compute the new order of item IDs in the folder's list and call `SuttaBridge.reorder_bookmark_items(folder_id, item_ids_json)`.
  - [ ] 8.3 Add `DragHandler` to `BookmarkFolderItem.qml` for reordering folders. Show a drag handle grip icon on the folder header.
  - [ ] 8.4 On drop, compute the new order of folder IDs and call `SuttaBridge.reorder_bookmark_folders(folder_ids_json)`.
  - [ ] 8.5 Verify drag-and-drop visuals: show a drag indicator/ghost while dragging, highlight drop zones.
  - [ ] 8.6 Verify `make build -B` compiles successfully.

- [ ] 9.0 QML: Implement checkbox multi-select and "Move Here" cross-folder move UX
  - [ ] 9.1 Add a shared state property (e.g., `property var selected_item_ids: []`) on `BookmarksTab` that tracks which bookmark item IDs are currently checked.
  - [ ] 9.2 Wire each `BookmarkListItem` checkbox to add/remove its item ID from `selected_item_ids`.
  - [ ] 9.3 When `selected_item_ids.length > 0`, show a "Move Here" button on each `BookmarkFolderItem` header (next to the folder name or as an overlay). Style it distinctly (e.g., highlighted color).
  - [ ] 9.4 On "Move Here" click, call `SuttaBridge.move_bookmark_items_to_folder(item_ids_json, target_folder_id)`, then clear `selected_item_ids`, uncheck all checkboxes, hide "Move Here" buttons, and refresh the bookmark lists.
  - [ ] 9.5 Verify `make build -B` compiles successfully.

- [ ] 10.0 QML: Implement bookmark opening/restoration with scroll position and find state
  - [ ] 10.1 In `BookmarkListItem.qml`, wire the "Open" button to call a function on `BookmarksTab` (or directly on `SuttaSearchWindow`) that restores a single bookmark item. The function should: construct `result_data` from the bookmark's `item_uid`, `table_name`, `title`, call `show_result_in_html_view(result_data)` or the pinned/translation equivalent based on `tab_group`.
  - [ ] 10.2 For `tab_group === "pinned"`: add the item via the pinned tab path (use existing `add_pinned_tab` or equivalent function in `SuttaSearchWindow.qml`). For `"results"`: use `show_result_in_html_view()`. For `"translations"`: add to the translations model.
  - [ ] 10.3 After the page loads in the webview, inject JavaScript to restore scroll position: `window.scrollTo(0, ${scroll_ratio} * document.documentElement.scrollHeight)`. Use the webview's `onLoadingChanged` or `loadFinished` signal to trigger this after content is ready.
  - [ ] 10.4 If `find_query` is non-empty, set `pending_find_query` to the bookmark's `find_query`. After the find bar activates and results are found, advance to `find_match_index` by calling find-next the appropriate number of times.
  - [ ] 10.5 In `BookmarkFolderItem.qml`, wire the "Open All" button to iterate all items in the folder and restore each one. Set `focus_on_new = false` for all items except the first one in the `results` group, which gets `focus_on_new = true`.
  - [ ] 10.6 Verify `make build -B` compiles successfully.

- [ ] 11.0 QML: Integrate Bookmarks tab into SuttaSearchWindow TabBar
  - [ ] 11.1 In `SuttaSearchWindow.qml`, add a new `TabButton` in `rightside_tabs` TabBar for "Bookmarks" with an appropriate icon (e.g., a star or bookmark ribbon from the icons directory).
  - [ ] 11.2 Add `BookmarksTab` as a new item in the `tab_stack` StackLayout at the corresponding index.
  - [ ] 11.3 Pass necessary properties to `BookmarksTab`: `window_id`, reference to the parent `SuttaSearchWindow` (for calling `show_result_in_html_view`), and any signals needed.
  - [ ] 11.4 Call `bookmarks_tab.load_bookmarks()` when the Bookmarks tab becomes visible (on tab switch) to refresh the data.
  - [ ] 11.5 Verify `make build -B` compiles successfully.

- [ ] 12.0 Last Session: Auto-save on exit and restore on startup
  - [ ] 12.1 In `SuttaSearchWindow.qml`, add a function `get_session_data_json() -> string` that collects all open tabs from all three tab groups, including scroll position (captured via JS `window.scrollY / document.documentElement.scrollHeight` on active webviews, or 0.0 for lazy-loaded tabs) and find bar state. Returns a JSON object: `{window_id, items: [{item_uid, table_name, title, tab_group, scroll_position, find_query, find_match_index}, ...]}`.
  - [ ] 12.2 In `cpp/gui.cpp`, connect `QApplication::aboutToQuit` signal (before `app.exec()`) to a lambda that iterates all `sutta_search_windows`, calls `get_session_data_json()` on each via `QMetaObject::invokeMethod`, collects the results into a JSON array, and calls `SuttaBridge.save_last_session(windows_json)`.
  - [ ] 12.3 Alternatively, implement the exit save entirely through a QML `Component.onDestruction` handler or `Connections` to `Qt.application.aboutToQuit` in `SuttaSearchWindow.qml`, collecting data and calling the bridge function. Choose whichever approach is more reliable for capturing state before windows are destroyed.
  - [ ] 12.4 In `cpp/window_manager.cpp` (or `gui.cpp`), after creating the initial `SuttaSearchWindow` on startup, check `SuttaBridge.get_restore_last_session()`. If `true`, call `SuttaBridge.get_last_session_json()`, parse the result, and for each last-session window folder, create a new `SuttaSearchWindow` and call `show_result_in_html_view_with_json()` for each item in the folder, adding them to the correct tab groups.
  - [ ] 12.5 Verify `make build -B` compiles successfully.

- [ ] 13.0 Settings: Add "Restore Last Session" checkbox in AppSettingsWindow
  - [ ] 13.1 In `assets/qml/AppSettingsWindow.qml`, add a `CheckBox` in the General tab section with `text: "Restore Last Session on Startup"`. Wire `onCheckedChanged` to call `SuttaBridge.set_restore_last_session(checked)`. In `Component.onCompleted`, set `checked = SuttaBridge.get_restore_last_session()`.
  - [ ] 13.2 Verify `make build -B` compiles successfully.
