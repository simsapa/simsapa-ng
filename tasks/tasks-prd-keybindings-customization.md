# Tasks: Keyboard Shortcuts Customization

## Relevant Files

### Backend (Rust)
- `backend/src/app_settings.rs` - Add AppKeybindings struct and integrate with AppSettings
- `backend/src/app_data.rs` - Add keybindings getter/setter methods to AppData
- `bridges/src/sutta_bridge.rs` - Add bridge functions for QML access
- `bridges/build.rs` - Register new QML files

### QML Frontend
- `assets/qml/KeybindingCaptureDialog.qml` - New dialog for capturing key combinations (create)
- `assets/qml/ShortcutConflictDialog.qml` - New dialog for conflict resolution (create)
- `assets/qml/AppSettingsWindow.qml` - Add Keybindings tab
- `assets/qml/SuttaSearchWindow.qml` - Replace hardcoded shortcuts with dynamic loading

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

- [ ] 1.0 Create AppKeybindings data structure in Rust backend
  - [ ] 1.1 Define `KeybindingAction` struct with `id: String` and `name: String` fields for action metadata
  - [ ] 1.2 Define `AppKeybindings` struct with `bindings: IndexMap<String, Vec<String>>` to map action IDs to shortcut lists
  - [ ] 1.3 Implement `Default` trait for `AppKeybindings` with all 21 default shortcuts from PRD table (settings, close_window, quit_app, sutta_search, focus_search, next_search_area, prev_result, next_result, find_in_page, find_next, find_prev, toggle_reading_mode, close_tab, toggle_tab_list, prev_tab, next_tab, prev_sidebar_tab, next_sidebar_tab, scroll_up, scroll_down, scroll_half_page_up)
  - [ ] 1.4 Add helper function `get_action_names() -> IndexMap<String, String>` returning action_id -> human-readable name mapping
  - [ ] 1.5 Add `app_keybindings: AppKeybindings` field to `AppSettings` struct
  - [ ] 1.6 Verify serde serialization works correctly (existing `#[serde(default)]` on AppSettings handles backward compatibility)
  - [ ] 1.7 Run `cargo test` to ensure no compilation errors

- [ ] 2.0 Implement keybindings bridge functions in SuttaBridge
  - [ ] 2.1 Add `get_keybindings_json(&self) -> QString` method to return current keybindings as JSON
  - [ ] 2.2 Add `get_default_keybindings_json(&self) -> QString` method to return default keybindings as JSON
  - [ ] 2.3 Add `get_action_names_json(&self) -> QString` method to return action ID to name mapping as JSON
  - [ ] 2.4 Add `set_keybinding(self: Pin<&mut Self>, action_id: &QString, shortcuts_json: &QString)` method to set shortcuts for a single action (shortcuts_json is JSON array of strings)
  - [ ] 2.5 Add `reset_keybinding(self: Pin<&mut Self>, action_id: &QString)` method to reset single action to default
  - [ ] 2.6 Add `reset_all_keybindings(self: Pin<&mut Self>)` method to reset all keybindings to defaults
  - [ ] 2.7 Add corresponding `#[qinvokable]` declarations in the bridge extern block
  - [ ] 2.8 Update `assets/qml/com/profoundlabs/simsapa/SuttaBridge.qml` with function signatures for qmllint
  - [ ] 2.9 Run `cargo test` and `make build -B` to verify compilation

- [ ] 3.0 Create KeybindingCaptureDialog QML component
  - [ ] 3.1 Create `assets/qml/KeybindingCaptureDialog.qml` using `ApplicationWindow` with `flags: Qt.Dialog` pattern from AboutDialog.qml
  - [ ] 3.2 Add properties: `action_name` (string), `current_shortcut` (string for editing existing), `is_new_shortcut` (bool)
  - [ ] 3.3 Add signal `shortcutAccepted(string shortcut)` for when user accepts a valid shortcut
  - [ ] 3.4 Add signal `shortcutRemoved()` for when user clicks Remove button
  - [ ] 3.5 Implement key capture area using `Item` with `focus: true` and `Keys.onPressed` handler
  - [ ] 3.6 Implement `build_key_sequence(event)` function to construct shortcut string from modifiers and key
  - [ ] 3.7 Implement `is_modifier_key(key)` function to detect Ctrl/Shift/Alt/Meta keys
  - [ ] 3.8 Add validation to reject modifier-only shortcuts (show "..." while only modifiers held)
  - [ ] 3.9 Add display area showing captured shortcut with visual feedback (different style when valid vs incomplete)
  - [ ] 3.10 Add "Clear" button to reset capture area
  - [ ] 3.11 Add "Accept" button (disabled when shortcut invalid) that emits `shortcutAccepted`
  - [ ] 3.12 Add "Cancel" button that closes dialog without changes
  - [ ] 3.13 Add "Remove" button (only visible when `!is_new_shortcut`) that emits `shortcutRemoved`
  - [ ] 3.14 Add proper bottom margin for mobile compatibility
  - [ ] 3.15 Register file in `bridges/build.rs` qml_files array
  - [ ] 3.16 Create `assets/qml/com/profoundlabs/simsapa/KeybindingCaptureDialog.qml` type definition for qmllint
  - [ ] 3.17 Add entry to `assets/qml/com/profoundlabs/simsapa/qmldir`

- [ ] 4.0 Add Keybindings tab to AppSettingsWindow
  - [ ] 4.1 Add `TabButton { text: "Keybindings"; padding: 5 }` to TabBar after "Find" tab
  - [ ] 4.2 Add property `var keybindings_data: ({})` to store loaded keybindings
  - [ ] 4.3 Add property `var default_keybindings: ({})` to store default keybindings for comparison
  - [ ] 4.4 Add property `var action_names: ({})` to store action ID to name mapping
  - [ ] 4.5 Create new `ScrollView` in StackLayout for Keybindings tab content
  - [ ] 4.6 Add `ColumnLayout` inside ScrollView with section Label "Keyboard Shortcuts"
  - [ ] 4.7 Create `Repeater` with model from `Object.keys(keybindings_data)`
  - [ ] 4.8 Create delegate `RowLayout` for each action showing: action name Label, shortcut Button(s), [+] Button, Reset Button
  - [ ] 4.9 Style shortcut buttons: use different background color when shortcut differs from default (compare with `default_keybindings`)
  - [ ] 4.10 Implement `onClicked` for shortcut buttons to open KeybindingCaptureDialog for editing
  - [ ] 4.11 Implement `onClicked` for [+] button to open KeybindingCaptureDialog for adding new shortcut
  - [ ] 4.12 Implement `onClicked` for Reset button to call `SuttaBridge.reset_keybinding(action_id)` and reload
  - [ ] 4.13 Add "Reset All to Defaults" button at bottom of keybindings list
  - [ ] 4.14 Create `ShortcutConflictDialog.qml` for conflict handling (simpler dialog with Yes/No buttons)
  - [ ] 4.15 Implement conflict detection: when shortcut accepted, check if it exists in another action
  - [ ] 4.16 Show ShortcutConflictDialog when conflict detected, handle Yes (remove from other action) / No (cancel)
  - [ ] 4.17 Implement `save_shortcut(action_id, shortcut_index, new_shortcut)` function to update keybindings via bridge
  - [ ] 4.18 Implement `add_shortcut(action_id, new_shortcut)` function to add new shortcut to action
  - [ ] 4.19 Implement `remove_shortcut(action_id, shortcut_index)` function to remove shortcut from action
  - [ ] 4.20 Load keybindings in `Component.onCompleted`: parse JSON from `SuttaBridge.get_keybindings_json()`, `get_default_keybindings_json()`, `get_action_names_json()`
  - [ ] 4.21 Register ShortcutConflictDialog in `bridges/build.rs` and create qmllint type definition

- [ ] 5.0 Integrate dynamic shortcuts in SuttaSearchWindow
  - [ ] 5.1 Add property `var keybindings: ({})` at top of SuttaSearchWindow to store loaded shortcuts
  - [ ] 5.2 Add `load_keybindings()` function that parses `SuttaBridge.get_keybindings_json()` into the property
  - [ ] 5.3 Call `load_keybindings()` in `Component.onCompleted`
  - [ ] 5.4 Create helper function `get_sequences(action_id)` that returns array from `keybindings[action_id]` or empty array
  - [ ] 5.5 Replace hardcoded `sequences: ["Ctrl+,"]` with `sequences: root.get_sequences("settings")` for Settings action
  - [ ] 5.6 Replace hardcoded sequences for all 21 actions (close_window, quit_app, sutta_search, focus_search, next_search_area, prev_result, next_result, find_in_page, find_next, find_prev, toggle_reading_mode, close_tab, toggle_tab_list, prev_tab, next_tab, prev_sidebar_tab, next_sidebar_tab, scroll_up, scroll_down, scroll_half_page_up)
  - [ ] 5.7 Ensure menu items remain accessible even when shortcuts array is empty (menu text should not show shortcut in this case)
  - [ ] 5.8 Optional: Add signal handler to reload keybindings when settings change (for immediate effect without window restart)

- [ ] 6.0 Build verification and manual testing
  - [ ] 6.1 Run `make build -B` and fix any compilation errors
  - [ ] 6.2 Run `cd backend && cargo test` and fix any test failures
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
