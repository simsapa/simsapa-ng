# Tasks: Sutta Languages Management Window

Based on PRD: `prd-sutta-languages-window.md`

## Relevant Files

- `assets/qml/LanguageListSelector.qml` - New reusable component for language selection UI
- `assets/qml/SuttaLanguagesWindow.qml` - New main window for managing sutta languages
- `assets/qml/DownloadAppdataWindow.qml` - Modified to use LanguageListSelector component
- `assets/qml/com/profoundlabs/simsapa/LanguageListSelector.qml` - Type stub for qmllint
- `assets/qml/com/profoundlabs/simsapa/SuttaLanguagesWindow.qml` - Type stub for qmllint
- `assets/qml/com/profoundlabs/simsapa/AssetManager.qml` - Type stub updated with remove function
- `assets/qml/com/profoundlabs/simsapa/SuttaBridge.qml` - Type stub updated with counts function
- `assets/qml/com/profoundlabs/simsapa/qmldir` - Updated to register new components
- `bridges/build.rs` - Updated to include new QML files in build
- `bridges/src/asset_manager.rs` - Modified to add remove_sutta_languages function
- `bridges/src/sutta_bridge.rs` - Modified to add get_sutta_language_labels_with_counts function
- `backend/src/db/appdata.rs` - Modified to add database removal operation
- `cpp/gui.h` - Add callback for opening SuttaLanguagesWindow
- `cpp/gui.cpp` - Implement callback for opening SuttaLanguagesWindow
- `cpp/window_manager.h` - Add create_sutta_languages_window method
- `cpp/window_manager.cpp` - Implement window creation for SuttaLanguagesWindow
- `cpp/sutta_languages_window.h` - New C++ window class header
- `cpp/sutta_languages_window.cpp` - New C++ window class implementation
- `CMakeLists.txt` - Add new C++ files to build
- `backend/tests/test_remove_sutta_languages.rs` - Test file for language removal functionality

### Notes

- Rust backend tests can be run with `cd backend && cargo test`
- To run a specific test: `cd backend && cargo test test_remove_sutta_languages`
- QML tests can be run with `make qml-test`
- Full build with `make build -B`
- Component isolation tested by verifying DownloadAppdataWindow still works after refactoring

## Tasks

- [ ] 1.0 Extract LanguageListSelector reusable component from DownloadAppdataWindow
  - [ ] 1.1 Create `assets/qml/LanguageListSelector.qml` by extracting lines 390-492 from DownloadAppdataWindow.qml
  - [ ] 1.2 Add configurable properties to LanguageListSelector: `model` (language list), `selected_languages` (array), `section_title` (string), `instruction_text` (string), `placeholder_text` (string), `available_label` (string), `show_count_column` (bool, default false)
  - [ ] 1.3 Implement signal `onLanguageSelectionChanged(selected_codes: var)` that emits when selection changes
  - [ ] 1.4 Implement function `get_selected_languages()` that returns array of selected language codes
  - [ ] 1.5 Support three-column layout (code, name, count) when `show_count_column` is true, with count column right-aligned
  - [ ] 1.6 Preserve existing functionality: click-to-toggle selection, TextField input sync with ListView, wildcard "*" support, visual highlighting, alternating row colors
  - [ ] 1.7 Replace the extracted section in DownloadAppdataWindow.qml (lines 390-492) with LanguageListSelector component instance
  - [ ] 1.8 Configure LanguageListSelector in DownloadAppdataWindow with appropriate properties: `section_title: "Include Languages"`, `instruction_text` describing usage, `placeholder_text: "E.g.: it, fr, pt, th"`, `available_label: "Available languages (click to select):"`, `show_count_column: false`
  - [ ] 1.9 Wire up LanguageListSelector signals/functions in DownloadAppdataWindow: connect `onLanguageSelectionChanged` to update `root.selected_languages`, use `get_selected_languages()` in download flow
  - [ ] 1.10 Test that DownloadAppdataWindow still works correctly after refactoring (language selection, download flow)

- [ ] 2.0 Add backend functions for language removal and sutta counts
  - [ ] 2.1 Add `remove_sutta_languages(language_codes: Vec<String>) -> Result<bool>` function in `backend/src/db/appdata.rs` that executes DELETE queries for each language code using Diesel ORM
  - [ ] 2.2 Implement proper error handling and logging in remove_sutta_languages using `anyhow::Result`, `tracing::info` and `tracing::error`
  - [ ] 2.3 Add `get_sutta_language_labels_with_counts() -> Vec<String>` function in `backend/src/db/appdata.rs` that queries `SELECT language, COUNT(*) FROM suttas GROUP BY language` and returns format `["code|Name|Count", ...]`
  - [ ] 2.4 Expose `remove_sutta_languages` through `backend/src/db/mod.rs` DatabaseManager: `pub fn remove_sutta_languages(&self, language_codes: Vec<String>) -> Result<bool>` that calls `self.appdata.remove_sutta_languages(language_codes)`
  - [ ] 2.5 Expose `get_sutta_language_labels_with_counts` through `backend/src/db/mod.rs` DatabaseManager
  - [ ] 2.6 Add `remove_sutta_languages(language_codes: QStringList) -> bool` to AssetManager bridge in `bridges/src/asset_manager.rs` as a `#[qinvokable]` function
  - [ ] 2.7 Implement AssetManager::remove_sutta_languages to convert QStringList to Vec<String>, call backend function, and return success status
  - [ ] 2.8 Add `get_sutta_language_labels_with_counts() -> QStringList` to SuttaBridge in `bridges/src/sutta_bridge.rs` as a `#[qinvokable]` function
  - [ ] 2.9 Implement SuttaBridge::get_sutta_language_labels_with_counts to call backend function and convert to QStringList with format `["code|Name|Count", ...]`, using LANG_CODE_TO_NAME for language names, sorted alphabetically
  - [ ] 2.10 Create test file `backend/tests/test_remove_sutta_languages.rs` to test the removal functionality with mock data
  - [ ] 2.11 Run backend tests to verify new functions work correctly: `cd backend && cargo test test_remove_sutta_languages`

- [ ] 3.0 Implement SuttaLanguagesWindow QML component with download and removal sections
  - [ ] 3.1 Create `assets/qml/SuttaLanguagesWindow.qml` as ApplicationWindow with title "Sutta Languages", width 600, max height 900, flag Qt.Dialog
  - [ ] 3.2 Add window properties: `is_mobile` (bool), `is_desktop` (bool), `pointSize` (12 desktop, 16 mobile), `largePointSize` (pointSize + 5)
  - [ ] 3.3 Create AssetManager instance `id: manager` and import SuttaBridge singleton in SuttaLanguagesWindow
  - [ ] 3.4 Add ScrollView as main content container with ColumnLayout for sections
  - [ ] 3.5 Implement Download Languages section: Label with title (bold, largePointSize), help text explaining purpose and size considerations, LanguageListSelector configured for downloads
  - [ ] 3.6 Configure download LanguageListSelector: `model` from `manager.get_available_languages()`, `section_title: "Download Languages"`, appropriate help text, `show_count_column: false`
  - [ ] 3.7 Add Download button below download LanguageListSelector, enabled only when selection is not empty: `enabled: download_selector.get_selected_languages().length > 0`
  - [ ] 3.8 Implement download button onClick: collect selected language codes, build URLs in format `https://github.com/simsapa/simsapa-ng-assets/releases/download/v0.1.5/suttas_lang_{code}.tar.bz2`, open DownloadAppdataWindow with `is_initial_setup: false` and auto-start download
  - [ ] 3.9 Implement Remove Languages section: Label with title (bold, largePointSize), help text explaining English and Pāli cannot be removed
  - [ ] 3.10 Configure removal LanguageListSelector: `model` from `SuttaBridge.get_sutta_language_labels_with_counts()` filtered to exclude "en" and "pli", `section_title: "Remove Languages"`, `show_count_column: true`
  - [ ] 3.11 Add Remove button below removal LanguageListSelector, enabled only when selection is not empty
  - [ ] 3.12 Implement confirmation Dialog that appears when Remove button is clicked: title "Confirm Language Removal", message listing selected languages, Cancel and Confirm buttons
  - [ ] 3.13 Implement removal operation after confirmation: call `manager.remove_sutta_languages(selected_codes)`, show "Removing..." message
  - [ ] 3.14 Implement completion Dialog after successful removal: title "Languages Removed", message explaining restart required, Quit button calling `Qt.quit()`
  - [ ] 3.15 Implement error Dialog if removal fails: show error details, allow retry or cancel
  - [ ] 3.16 Add button layout at bottom: Desktop horizontal (Close left, actions right), Mobile vertical stack
  - [ ] 3.17 Add Close button that closes the window without quitting application
  - [ ] 3.18 Use system palette colors throughout (palette.window, palette.text, etc.) for theme compatibility
  - [ ] 3.19 Add Component.onCompleted to populate language lists on window open

- [ ] 4.0 Implement C++ window management and integration
  - [ ] 4.1 Create `cpp/sutta_languages_window.h` header file based on word_lookup_window.h pattern with SuttaLanguagesWindow class
  - [ ] 4.2 Add SuttaLanguagesWindow class members: `QApplication* m_app`, `QObject* m_root`, `QQmlApplicationEngine* m_engine`
  - [ ] 4.3 Add constructor `SuttaLanguagesWindow(QApplication* app, QObject* parent = nullptr)` and destructor
  - [ ] 4.4 Add private method `void setup_qml()` to load SuttaLanguagesWindow.qml
  - [ ] 4.5 Create `cpp/sutta_languages_window.cpp` implementation file based on sutta_search_window.cpp pattern
  - [ ] 4.6 Implement constructor to initialize m_app and call setup_qml()
  - [ ] 4.7 Implement setup_qml() to load QUrl "qrc:/qt/qml/com/profoundlabs/simsapa/assets/qml/SuttaLanguagesWindow.qml" and create engine
  - [ ] 4.8 Implement destructor to delete m_engine
  - [ ] 4.9 Add `#include "sutta_languages_window.h"` forward declaration in `cpp/window_manager.h`
  - [ ] 4.10 Add `QList<SuttaLanguagesWindow*> sutta_languages_windows;` member to WindowManager class in window_manager.h
  - [ ] 4.11 Add `SuttaLanguagesWindow* create_sutta_languages_window();` public method declaration in window_manager.h
  - [ ] 4.12 Implement `WindowManager::create_sutta_languages_window()` in window_manager.cpp following create_word_lookup_window pattern
  - [ ] 4.13 Add cleanup for sutta_languages_windows list in WindowManager destructor
  - [ ] 4.14 Add `void callback_open_sutta_languages_window();` function declaration in `cpp/gui.h`
  - [ ] 4.15 Implement `callback_open_sutta_languages_window()` in gui.cpp to call `AppGlobals::manager->create_sutta_languages_window()`
  - [ ] 4.16 Add extern C++ declaration in `bridges/src/sutta_bridge.rs` in unsafe extern "C++" block: `fn callback_open_sutta_languages_window();`
  - [ ] 4.17 Add `#[qinvokable]` function `open_sutta_languages_window(&self)` to SuttaBridge in sutta_bridge.rs that calls `ffi::callback_open_sutta_languages_window()`
  - [ ] 4.18 Update CMakeLists.txt to add `cpp/sutta_languages_window.cpp` to the source files list (near sutta_search_window.cpp and word_lookup_window.cpp)

- [ ] 5.0 Update build configuration and qmllint type definitions
  - [ ] 5.1 Add `qml_files.push("../assets/qml/LanguageListSelector.qml");` to `bridges/build.rs` qml_files list
  - [ ] 5.2 Add `qml_files.push("../assets/qml/SuttaLanguagesWindow.qml");` to `bridges/build.rs` qml_files list (note: already exists in build.rs from line 16, verify it's there)
  - [ ] 5.3 Create qmllint type stub `assets/qml/com/profoundlabs/simsapa/LanguageListSelector.qml` with all properties, signals, and functions matching the component interface
  - [ ] 5.4 Create qmllint type stub `assets/qml/com/profoundlabs/simsapa/SuttaLanguagesWindow.qml` as a basic Item stub
  - [ ] 5.5 Update `assets/qml/com/profoundlabs/simsapa/qmldir` to add: `LanguageListSelector 1.0 LanguageListSelector.qml` and `SuttaLanguagesWindow 1.0 SuttaLanguagesWindow.qml`
  - [ ] 5.6 Add `function remove_sutta_languages(language_codes: list<string>): bool` to `assets/qml/com/profoundlabs/simsapa/AssetManager.qml` type stub
  - [ ] 5.7 Add `function get_sutta_language_labels_with_counts(): list<string>` to `assets/qml/com/profoundlabs/simsapa/SuttaBridge.qml` type stub
  - [ ] 5.8 Add `function open_sutta_languages_window()` to `assets/qml/com/profoundlabs/simsapa/SuttaBridge.qml` type stub
  - [ ] 5.9 Run `make build -B` to verify all components compile successfully
  - [ ] 5.10 Test opening SuttaLanguagesWindow from QML or menu: verify window opens, displays language lists, download and remove operations work correctly
