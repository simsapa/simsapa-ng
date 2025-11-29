# PRD: Sutta Languages Management Window

## Introduction/Overview

This feature adds a new window to the Simsapa application that allows users to manage sutta language databases. Users can download additional language translations after initial setup and remove languages they no longer need (except for the core English and Pāli databases). This provides users flexibility to customize their offline content library according to their reading preferences without requiring a complete reinstall.

The implementation involves:
1. Extracting a reusable `LanguageListSelector.qml` component from `DownloadAppdataWindow.qml`
2. Creating `SuttaLanguagesWindow.qml` with download and removal sections
3. Using existing backend infrastructure (`AssetManager`, `SuttaBridge`) for database operations

## Goals

1. Enable users to download additional sutta language databases after initial application setup
2. Enable users to remove unwanted language databases to free disk space
3. Provide a clear, intuitive UI for language management operations
4. Allow users to update/refresh existing language databases by re-downloading
5. Maintain application stability by protecting core languages (English, Pāli) from removal
6. Display information about installed languages (language name, number of suttas)

## User Stories

1. **As a user who only reads English translations**, I want to remove other language databases to save disk space on my device.

2. **As a multilingual user**, I want to download additional language suttas after initial setup so I can read translations in my preferred languages.

3. **As a user**, I want to see which languages are currently installed in my database so I know what content is available offline.

4. **As a user updating my content**, I want to re-download existing language databases to get the latest translations and corrections.

## Functional Requirements

### 1. Component Extraction: LanguageListSelector.qml

1.1. Extract the language selection UI (currently `id: language_list_selector` in `DownloadAppdataWindow.qml` lines 390-492) into a reusable component `LanguageListSelector.qml`.

1.2. The component must accept the following inputs:
   - `model`: List of languages in format `["code|Name", ...]` or `["code|Name|Count", ...]`
   - `selected_languages`: Array of currently selected language codes
   - `section_title`: Customizable section title (e.g., "Include Languages", "Installed Languages")
   - `instruction_text`: Customizable instruction text
   - `placeholder_text`: TextField placeholder text
   - `available_label`: Label for the list (e.g., "Available languages (click to select)")
   - `show_count_column`: Boolean to show third column with sutta counts (default: false)

1.3. The component must provide the following outputs:
   - `onLanguageSelectionChanged(selected_codes: Array)`: Signal emitted when selection changes
   - `get_selected_languages()`: Function returning array of selected language codes

1.4. The component must preserve existing functionality:
   - Click-to-toggle selection in ListView
   - TextField input synchronization with ListView selection
   - Support for wildcard "*" to select all languages
   - Visual highlighting of selected items
   - Alternating row colors for readability

### 2. SuttaLanguagesWindow.qml Implementation

2.1. Create `SuttaLanguagesWindow.qml` with the following window properties:
   - Title: "Sutta Languages"
   - Dimensions: 600px width, max 900px height (matching DownloadAppdataWindow)
   - Window flag: `Qt.Dialog`
   - Mobile/desktop responsive layout

2.2. Window layout must include two main sections in a scrollable area:

#### Section A: Download Languages
   - Label: "Download Languages" (bold, larger font)
   - Help text explaining the purpose
   - Instance of `LanguageListSelector.qml` configured for downloads
   - "Download" button (disabled when no languages selected)
   - Show which languages are already installed (visual indicator optional in initial version)

#### Section B: Remove Languages  
   - Label: "Remove Languages" (bold, larger font)
   - Help text explaining that English and Pāli cannot be removed, with explanation why
   - Instance of `LanguageListSelector.qml` showing installed languages
   - Display sutta count for each installed language (third column)
   - "Remove" button (disabled when no languages selected)

2.3. Button layout:
   - Desktop: Horizontal layout at bottom with "Close" button on left, action buttons on right
   - Mobile: Vertical button stack at bottom

### 3. Download Flow

3.1. When "Download" button is clicked:
   - Collect selected language codes from LanguageListSelector
   - Build array of download URLs in format: `https://github.com/simsapa/simsapa-ng-assets/releases/download/v0.1.5/suttas_lang_{code}.tar.bz2`
   - Open `DownloadAppdataWindow.qml` with:
     - `is_initial_setup: false`
     - Pre-initialized with language database URLs
     - Automatically start download
   - `SuttaLanguagesWindow` remains open but inactive during download

3.2. Download window must show:
   - Progress bars for each language download (existing functionality)
   - Retry mechanism for failed downloads (existing functionality in asset_manager.rs)
   - Import status messages as databases are processed
   - Completion message

3.3. After download completion:
   - Display completion dialog in DownloadAppdataWindow
   - User clicks "Quit" to close application
   - Application must be restarted to use new languages

### 4. Remove Flow

4.1. Populate removable languages list:
   - Call `SuttaBridge.get_sutta_language_labels()` to get installed languages
   - Filter out "en" and "pli" from the list (these are protected)
   - Display remaining languages with sutta counts in LanguageListSelector

4.2. When "Remove" button is clicked:
   - Collect selected language codes from LanguageListSelector
   - Show confirmation dialog with:
     - Title: "Confirm Language Removal"
     - Message: "Are you sure you want to remove the following languages from your database?"
     - List of selected language names
     - "Cancel" and "Confirm" buttons

4.3. After confirmation:
   - Execute database removal operation via AssetManager backend
   - For each language code:
     - Delete rows from `suttas` table where `language = {code}`
     - Database triggers (appdata-fts5-index-for-suttas-content_plain.sql) handle index updates automatically
   - Show progress/status messages

4.4. After removal completion:
   - Display dialog with:
     - Title: "Languages Removed"
     - Message: "The selected languages have been removed. Please quit and restart the application for changes to take effect."
     - "Quit" button (calls `Qt.quit()`)
   - If removal fails, show error dialog with error details

### 5. Backend Integration

5.1. Use existing `AssetManager` component:
   - `id: manager` instance in SuttaLanguagesWindow
   - `manager.get_available_languages()`: Get downloadable languages
   - `manager.download_urls_and_extract()`: Handle downloads
   - Signals: `downloadProgressChanged`, `downloadShowMsg`, `downloadsCompleted`

5.2. Use existing `SuttaBridge` singleton:
   - `SuttaBridge.get_sutta_language_labels()`: Get installed languages with labels
   - Returns format: `["code|Name", ...]` in alphabetical order

5.3. Add new function to AssetManager in `asset_manager.rs`:
   - `remove_sutta_languages(language_codes: QStringList) -> bool`
   - Executes DELETE queries on appdata database
   - Returns success/failure status
   - Logs operations and errors

5.4. Add new function to SuttaBridge for sutta counts:
   - `get_sutta_language_labels_with_counts() -> QStringList`
   - Returns format: `["code|Name|Count", ...]`
   - Query: `SELECT language, COUNT(*) FROM suttas GROUP BY language`

### 6. LanguageListSelector Component Reusability

6.1. Component must be self-contained with minimal external dependencies

6.2. All labels, titles, and instruction text must be customizable via properties

6.3. Component must handle both two-column (code, name) and three-column (code, name, count) layouts

6.4. Component must emit clear signals for parent components to respond to selection changes

### 7. Error Handling

7.1. Download errors:
   - Retry mechanism in asset_manager.rs (up to 5 attempts with exponential backoff)
   - Display clear error messages in download status
   - User can close download window and retry later

7.2. Removal errors:
   - If database query fails, show error dialog with details
   - Do not close SuttaLanguagesWindow on error
   - Allow user to retry or cancel

7.3. No language selected:
   - Download button: disabled state (not clickable)
   - Remove button: disabled state (not clickable)

7.4. Protected languages:
   - English ("en") and Pāli ("pli") never appear in removal list
   - Help text explains these are required languages

### 8. Build System Integration

8.1. Add `SuttaLanguagesWindow.qml` to `qml_files` list in `bridges/build.rs`:
```rust
qml_files.push("../assets/qml/SuttaLanguagesWindow.qml");
```

8.2. Add `LanguageListSelector.qml` to `qml_files` list in `bridges/build.rs`:
```rust
qml_files.push("../assets/qml/LanguageListSelector.qml");
```

8.3. Add corresponding qmllint type definitions:
   - `assets/qml/com/profoundlabs/simsapa/SuttaLanguagesWindow.qml`
   - `assets/qml/com/profoundlabs/simsapa/LanguageListSelector.qml`

8.4. Update `assets/qml/com/profoundlabs/simsapa/qmldir` with new components

## Non-Goals (Out of Scope)

1. **Partial language downloads**: Users cannot download only specific suttas within a language; it's all or nothing per language.

2. **Automatic database updates**: The feature will not automatically check for or prompt users about language database updates.

3. **Disk space calculations**: The UI will not show how much disk space each language occupies or how much will be freed.

4. **Bookmark/notes preservation warnings**: No warnings about losing bookmarks or notes associated with removed languages (assumed to be rare edge case).

5. **In-place language updates without restart**: After adding/removing languages, application restart is required.

6. **Sanskrit language removal**: While "san" is also a base language, the initial focus is on protecting "en" and "pli" only.

7. **Import progress bars per language**: Downloads show progress, but the import step shows a simple status message rather than granular progress.

## Design Considerations

### UI/UX Layout

1. **Window structure**: Follow DownloadAppdataWindow.qml patterns:
   - Use `StackLayout` if multiple states needed
   - ScrollView for main content area
   - Fixed button area at bottom
   - Mobile vs desktop responsive sizing

2. **LanguageListSelector appearance**:
   - Simple checklist-style interaction (click to toggle)
   - Visual feedback: highlighted background for selected items
   - Alternating row colors (palette.base / palette.alternateBase)
   - Two columns: language code (50px width), language name (flexible)
   - Optional third column for sutta count (right-aligned)

3. **Typography**:
   - Section headers: `font.bold: true`, `font.pointSize: root.pointSize`
   - Help text: Regular weight, slightly smaller point size
   - Use `root.pointSize` and `root.largePointSize` properties for consistency

4. **Color theme**:
   - Use system palette colors throughout (palette.window, palette.text, etc.)
   - Respect dark/light theme settings
   - No hardcoded colors

5. **Button states**:
   - Disabled state when no selection: `enabled: selected_languages.length > 0`
   - Visual feedback with hover states (automatic with Qt Controls)

### Language Display Order

- All language lists must be sorted alphabetically by language code
- Sorting happens in backend (Rust) before returning to QML
- QML displays in the order received

### Help Text Content

**Download section help text:**
```
"Select additional languages to download. Each language includes all available sutta translations. Downloads may be large (50-200 MB per language). You can re-download existing languages to update to the latest versions."
```

**Remove section help text:**
```
"Remove languages you no longer need to free up disk space. English and Pāli cannot be removed as they are core languages required for the application to function properly."
```

## Technical Considerations

### Component Communication

1. **LanguageListSelector to parent**:
   - Use signals (`onLanguageSelectionChanged`) for reactive updates
   - Provide getter function for explicit queries
   - Parent manages selected state, component is stateless where possible

2. **SuttaLanguagesWindow to DownloadAppdataWindow**:
   - Use window instantiation pattern (like existing window management)
   - Pass initialization parameters via properties
   - No direct coupling; windows communicate via backend signals

3. **Backend database operations**:
   - All database writes happen in Rust backend
   - Use Diesel ORM for type-safe queries
   - Proper error propagation with `anyhow::Result`
   - Transaction support for multi-row operations

### Performance

1. **Language list loading**: 
   - Query happens once on window open
   - Cache in QML property if needed
   - For typical 30-40 languages, performance is not a concern

2. **Sutta count queries**:
   - Single GROUP BY query, should be fast with indexes
   - Execute in Rust thread to avoid UI blocking
   - Display "Loading..." state if needed

3. **Database removal**:
   - DELETE operations can be slow for large datasets
   - Execute in background thread
   - Show progress indicator (indeterminate spinner)

### Code Style

Follow existing project conventions:

**Rust:**
- snake_case for functions and variables
- Use `anyhow::Result` for error handling
- Use `tracing::info` and `tracing::error` for logging (or project's logger module)
- Document public functions with doc comments

**QML:**
- snake_case for id names: `id: remove_languages_list`
- camelCase for properties: `property bool showRemoveSection`
- PascalCase for component files: `LanguageListSelector.qml`
- Use `function` keyword for JavaScript functions
- Arrow functions for inline callbacks

**Naming:**
- Use domain terms: "sutta", "language", "appdata"
- Be explicit: `get_sutta_language_labels_with_counts` over `get_langs_counted`

## Success Metrics

1. **Functional success**: Users can successfully download and remove languages without errors or data corruption.

2. **Stability success**: The application remains stable after language management operations and restarts cleanly.

3. **UX success**: Users understand the workflow without confusion:
   - Clear understanding that restart is required
   - No attempts to remove protected languages (they never see them in removal list)
   - Confirmation dialogs prevent accidental deletions

4. **Code quality success**:
   - LanguageListSelector component is successfully reused in multiple contexts (download, removal, potentially elsewhere)
   - No code duplication between windows
   - Clean separation of concerns (UI in QML, logic in Rust)

## Open Questions

1. **Window opening pattern**: How is `SuttaBridge.open_sutta_languages_window()` implemented? Does it follow the same pattern as other window management code in the C++ layer?

See `SuttaBridge.open_sutta_search_window()` for an example.

2. **Asset version handling**: Should the download URLs use hardcoded version `v0.1.5` or should this be configurable? Where is this version string currently managed?

Leave the version number hardcoded for now.

3. **Database transaction scope**: Should language removal be wrapped in a single transaction, or is per-language deletion acceptable?

Per-language deletion is good.

4. **Progress indication for removal**: Should removal show a progress bar or is an indeterminate spinner sufficient?

Removal doesn't take long, no need for a progress bar, just a "Removing..." message.

5. **Undo functionality**: Is there any need for an "undo" feature for language removal, or is the confirmation dialog sufficient?

No need for undo.

6. **Download window reuse**: When opening DownloadAppdataWindow from SuttaLanguagesWindow, should we use the same window instance or create a new one?

Can open a new DownloadAppdataWindow.

7. **Legacy Python code**: The file `simsapa-legacy/simsapa/layouts/sutta_languages.py` exists—should this be used as additional reference, or is it superseded by this Qt6/QML implementation?

`sutta_languages.py` is legacy, and superseded by the QML + Rust implementation.

## Implementation Notes for Developers

### Step 1: Extract LanguageListSelector Component
- Copy lines 390-492 from DownloadAppdataWindow.qml
- Parameterize hardcoded strings and values
- Test component isolation by re-integrating into DownloadAppdataWindow
- Verify existing download functionality still works

### Step 2: Add Backend Functions
- Implement `remove_sutta_languages` in asset_manager.rs
- Implement `get_sutta_language_labels_with_counts` in sutta_bridge.rs
- Add corresponding function declarations in bridge QML type definitions
- Write unit tests for backend functions

### Step 3: Implement SuttaLanguagesWindow
- Create window skeleton with layout structure
- Integrate LanguageListSelector instances
- Wire up button actions to backend functions
- Implement confirmation and completion dialogs

### Step 4: Test and Refine
- Test download flow with multiple languages
- Test removal flow with confirmation
- Test error cases (network failures, database errors)
- Test mobile and desktop layouts
- Verify application restart behavior

### Key Files to Modify/Create

**New files:**
- `assets/qml/LanguageListSelector.qml`
- `assets/qml/SuttaLanguagesWindow.qml`
- `assets/qml/com/profoundlabs/simsapa/LanguageListSelector.qml` (type stub)
- `assets/qml/com/profoundlabs/simsapa/SuttaLanguagesWindow.qml` (type stub)

**Modified files:**
- `bridges/build.rs` (add new QML files to build)
- `bridges/src/asset_manager.rs` (add remove_sutta_languages function)
- `bridges/src/sutta_bridge.rs` (add get_sutta_language_labels_with_counts function)
- `assets/qml/DownloadAppdataWindow.qml` (replace language_list_selector with component)
- `assets/qml/SuttaSearchWindow.qml` (already has menu action, verify it works)
- `assets/qml/com/profoundlabs/simsapa/qmldir` (register new components)
- `assets/qml/com/profoundlabs/simsapa/AssetManager.qml` (add type stub for new function)
- `assets/qml/com/profoundlabs/simsapa/SuttaBridge.qml` (add type stub for new function)

### Testing Checklist

- [ ] LanguageListSelector works in DownloadAppdataWindow (existing behavior preserved)
- [ ] LanguageListSelector works in SuttaLanguagesWindow download section
- [ ] LanguageListSelector works in SuttaLanguagesWindow removal section
- [ ] Download button opens DownloadAppdataWindow and starts download
- [ ] Download progress shows correctly for language databases
- [ ] Download completion shows restart prompt
- [ ] Removal list shows installed languages minus en/pli
- [ ] Removal list shows sutta counts
- [ ] Removal confirmation dialog appears with correct language names
- [ ] Removal operation completes successfully
- [ ] Removal error handling shows appropriate messages
- [ ] Application restarts cleanly after language changes
- [ ] New languages appear in search results after restart
- [ ] Removed languages do not appear in search results after restart
- [ ] Mobile layout renders correctly
- [ ] Desktop layout renders correctly
- [ ] Buttons are disabled when no selection made
- [ ] Help text is clear and visible
