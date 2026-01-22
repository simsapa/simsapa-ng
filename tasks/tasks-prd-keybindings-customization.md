# Tasks: Keyboard Shortcuts Customization

## Relevant Files

### Backend (Rust)
- `backend/src/app_settings.rs` - Added KeybindingAction struct, AppKeybindings struct with Default impl and get_action_names(), integrated into AppSettings
- `backend/src/app_data.rs` - Added keybindings getter/setter methods: get_keybindings_json(), get_default_keybindings_json(), get_action_names_json(), set_keybinding(), reset_keybinding(), reset_all_keybindings()
- `bridges/src/sutta_bridge.rs` - Added bridge functions for QML access with #[qinvokable] declarations
- `bridges/build.rs` - Register new QML files

### QML Type Definitions (for qmllint)
- `assets/qml/com/profoundlabs/simsapa/SuttaBridge.qml` - Added function signatures for keybindings management

### QML Frontend
- `assets/qml/KeybindingCaptureDialog.qml` - Dialog for capturing key combinations (created)
- `assets/qml/ShortcutConflictDialog.qml` - Dialog for conflict resolution (created)
- `assets/qml/AppSettingsWindow.qml` - Added Keybindings tab with full UI for managing shortcuts
- `assets/qml/SuttaSearchWindow.qml` - Replaced hardcoded shortcuts with dynamic loading (21 actions)

### QML Type Definitions (for qmllint)
- `assets/qml/com/profoundlabs/simsapa/KeybindingCaptureDialog.qml` - Type definition (create)
- `assets/qml/com/profoundlabs/simsapa/ShortcutConflictDialog.qml` - Type definition (create)
- `assets/qml/com/profoundlabs/simsapa/SuttaBridge.qml` - Add new bridge function signatures
- `assets/qml/com/profoundlabs/simsapa/qmldir` - Register new types

### Notes

- Build command: `make build -B`
- Run Rust tests: `cd backend && cargo test`
- QML files must be registered in `bridges/build.rs` under `qml_files`
- New bridge types need corresponding qmllint type definitions in `assets/qml/com/profoundlabs/simsapa/`
- Follow existing patterns: `AboutDialog.qml` for dialog structure, `AppSettingsWindow.qml` for settings UI

## Tasks

- [x] 1.0 Create AppKeybindings data structure in Rust backend
  - [x] 1.1 Define `KeybindingAction` struct with `id: String` and `name: String` fields for action metadata
  - [x] 1.2 Define `AppKeybindings` struct with `bindings: IndexMap<String, Vec<String>>` to map action IDs to shortcut lists
  - [x] 1.3 Implement `Default` trait for `AppKeybindings` with all 21 default shortcuts from PRD table (settings, close_window, quit_app, sutta_search, focus_search, next_search_area, prev_result, next_result, find_in_page, find_next, find_prev, toggle_reading_mode, close_tab, toggle_tab_list, prev_tab, next_tab, prev_sidebar_tab, next_sidebar_tab, scroll_up, scroll_down, scroll_half_page_up)
  - [x] 1.4 Add helper function `get_action_names() -> IndexMap<String, String>` returning action_id -> human-readable name mapping
  - [x] 1.5 Add `app_keybindings: AppKeybindings` field to `AppSettings` struct
  - [x] 1.6 Verify serde serialization works correctly (existing `#[serde(default)]` on AppSettings handles backward compatibility)
  - [x] 1.7 Run `cargo test` to ensure no compilation errors

- [x] 2.0 Implement keybindings bridge functions in SuttaBridge
  - [x] 2.1 Add `get_keybindings_json(&self) -> QString` method to return current keybindings as JSON
  - [x] 2.2 Add `get_default_keybindings_json(&self) -> QString` method to return default keybindings as JSON
  - [x] 2.3 Add `get_action_names_json(&self) -> QString` method to return action ID to name mapping as JSON
  - [x] 2.4 Add `set_keybinding(self: Pin<&mut Self>, action_id: &QString, shortcuts_json: &QString)` method to set shortcuts for a single action (shortcuts_json is JSON array of strings)
  - [x] 2.5 Add `reset_keybinding(self: Pin<&mut Self>, action_id: &QString)` method to reset single action to default
  - [x] 2.6 Add `reset_all_keybindings(self: Pin<&mut Self>)` method to reset all keybindings to defaults
  - [x] 2.7 Add corresponding `#[qinvokable]` declarations in the bridge extern block
  - [x] 2.8 Update `assets/qml/com/profoundlabs/simsapa/SuttaBridge.qml` with function signatures for qmllint
  - [x] 2.9 Run `cargo test` and `make build -B` to verify compilation

- [x] 3.0 Create KeybindingCaptureDialog QML component
  - [x] 3.1 Create `assets/qml/KeybindingCaptureDialog.qml` using `ApplicationWindow` with `flags: Qt.Dialog` pattern from AboutDialog.qml
  - [x] 3.2 Add properties: `action_name` (string), `current_shortcut` (string for editing existing), `is_new_shortcut` (bool)
  - [x] 3.3 Add signal `shortcutAccepted(string shortcut)` for when user accepts a valid shortcut
  - [x] 3.4 Add signal `shortcutRemoved()` for when user clicks Remove button
  - [x] 3.5 Implement key capture area using `Item` with `focus: true` and `Keys.onPressed` handler
  - [x] 3.6 Implement `build_key_sequence(event)` function to construct shortcut string from modifiers and key
  - [x] 3.7 Implement `is_modifier_key(key)` function to detect Ctrl/Shift/Alt/Meta keys
  - [x] 3.8 Add validation to reject modifier-only shortcuts (show "..." while only modifiers held)
  - [x] 3.9 Add display area showing captured shortcut with visual feedback (different style when valid vs incomplete)
  - [x] 3.10 Add "Clear" button to reset capture area
  - [x] 3.11 Add "Accept" button (disabled when shortcut invalid) that emits `shortcutAccepted`
  - [x] 3.12 Add "Cancel" button that closes dialog without changes
  - [x] 3.13 Add "Remove" button (only visible when `!is_new_shortcut`) that emits `shortcutRemoved`
  - [x] 3.14 Add proper bottom margin for mobile compatibility
  - [x] 3.15 Register file in `bridges/build.rs` qml_files array
  - [x] 3.16 Create `assets/qml/com/profoundlabs/simsapa/KeybindingCaptureDialog.qml` type definition for qmllint
  - [x] 3.17 Add entry to `assets/qml/com/profoundlabs/simsapa/qmldir`

- [x] 4.0 Add Keybindings tab to AppSettingsWindow
  - [x] 4.1 Add `TabButton { text: "Keybindings"; padding: 5 }` to TabBar after "Find" tab
  - [x] 4.2 Add property `var keybindings_data: ({})` to store loaded keybindings
  - [x] 4.3 Add property `var default_keybindings: ({})` to store default keybindings for comparison
  - [x] 4.4 Add property `var action_names: ({})` to store action ID to name mapping
  - [x] 4.5 Create new `ScrollView` in StackLayout for Keybindings tab content
  - [x] 4.6 Add `ColumnLayout` inside ScrollView with section Label "Keyboard Shortcuts"
  - [x] 4.7 Create `Repeater` with model from `Object.keys(keybindings_data)`
  - [x] 4.8 Create delegate `RowLayout` for each action showing: action name Label, shortcut Button(s), [+] Button, Reset Button
  - [x] 4.9 Style shortcut buttons: use different background color when shortcut differs from default (compare with `default_keybindings`)
  - [x] 4.10 Implement `onClicked` for shortcut buttons to open KeybindingCaptureDialog for editing
  - [x] 4.11 Implement `onClicked` for [+] button to open KeybindingCaptureDialog for adding new shortcut
  - [x] 4.12 Implement `onClicked` for Reset button to call `SuttaBridge.reset_keybinding(action_id)` and reload
  - [x] 4.13 Add "Reset All to Defaults" button at bottom of keybindings list
  - [x] 4.14 Create `ShortcutConflictDialog.qml` for conflict handling (simpler dialog with Yes/No buttons)
  - [x] 4.15 Implement conflict detection: when shortcut accepted, check if it exists in another action
  - [x] 4.16 Show ShortcutConflictDialog when conflict detected, handle Yes (remove from other action) / No (cancel)
  - [x] 4.17 Implement `save_shortcut(action_id, shortcut_index, new_shortcut)` function to update keybindings via bridge
  - [x] 4.18 Implement `add_shortcut(action_id, new_shortcut)` function to add new shortcut to action
  - [x] 4.19 Implement `remove_shortcut(action_id, shortcut_index)` function to remove shortcut from action
  - [x] 4.20 Load keybindings in `Component.onCompleted`: parse JSON from `SuttaBridge.get_keybindings_json()`, `get_default_keybindings_json()`, `get_action_names_json()`
  - [x] 4.21 Register ShortcutConflictDialog in `bridges/build.rs` and create qmllint type definition

- [x] 5.0 Integrate dynamic shortcuts in SuttaSearchWindow
  - [x] 5.1 Add property `var keybindings: ({})` at top of SuttaSearchWindow to store loaded shortcuts
  - [x] 5.2 Add `load_keybindings()` function that parses `SuttaBridge.get_keybindings_json()` into the property
  - [x] 5.3 Call `load_keybindings()` in `Component.onCompleted`
  - [x] 5.4 Create helper function `get_sequences(action_id)` that returns array from `keybindings[action_id]` or empty array
  - [x] 5.5 Replace hardcoded `sequences: ["Ctrl+,"]` with `sequences: root.get_sequences("settings")` for Settings action
  - [x] 5.6 Replace hardcoded sequences for all 21 actions (close_window, quit_app, sutta_search, focus_search, next_search_area, prev_result, next_result, find_in_page, find_next, find_prev, toggle_reading_mode, close_tab, toggle_tab_list, prev_tab, next_tab, prev_sidebar_tab, next_sidebar_tab, scroll_up, scroll_down, scroll_half_page_up)
  - [x] 5.7 Ensure menu items remain accessible even when shortcuts array is empty (menu text should not show shortcut in this case)
  - [x] 5.8 Optional: Add signal handler to reload keybindings when settings change (for immediate effect without window restart)

- [x] 6.0 Build verification and manual testing
  - [x] 6.1 Run `make build -B` and fix any compilation errors
  - [x] 6.2 Run `cd backend && cargo test` and fix any test failures (note: test_pts_search_vol failure is pre-existing and unrelated to keybindings)
  - [ ] 6.3 Manual test: Open app, verify all default shortcuts work as before
  - [ ] 6.4 Manual test: Open Settings > Keybindings tab, verify all actions listed with correct defaults
  - [ ] 6.5 Manual test: Click a shortcut button, capture dialog opens, capture new shortcut, accept
  - [ ] 6.6 Manual test: Verify customized shortcut shows different background color
  - [ ] 6.7 Manual test: Verify new shortcut works in SuttaSearchWindow
  - [ ] 6.8 Manual test: Assign conflicting shortcut, verify conflict dialog appears
  - [ ] 6.9 Manual test: Click Reset on single action, verify it returns to default
  - [ ] 6.10 Manual test: Click Reset All, verify all shortcuts return to defaults
  - [ ] 6.11 Manual test: Close and reopen app, verify custom shortcuts persisted
  - [ ] 6.12 Manual test: Add multiple shortcuts to same action using [+] button
  - [ ] 6.13 Manual test: Remove a shortcut using Remove button in capture dialog
