# Tasks: CIPS Topic Index Window

Based on PRD: `tasks/prd-cips-topic-index-window.md`

## Relevant Files

- `cli/src/bootstrap/parse_cips_index.rs` - CLI module to parse CIPS general-index.csv and generate JSON output (structs, normalization, sorting, validation)
- `cli/src/bootstrap/mod.rs` - Registers the parse_cips_index module
- `cli/src/main.rs` - Contains `ParseCipsIndex` subcommand and `parse_cips_index_command()` function with database title lookup
- `cli/Cargo.toml` - Added `unicode-normalization` dependency for diacritic handling
- `cli/tests/data/sample-cips-index.csv` - Sample test data for parser testing
- `cli/tests/data/sample-cips-output.json` - Sample output from parser for verification
- `assets/general-index.json` - Generated JSON data file containing the parsed topic index
- `backend/src/app_settings.rs` - Add static JSON inclusion constant `CIPS_GENERAL_INDEX_JSON`
- `backend/src/topic_index.rs` - New module for topic index data structures, parsing, and search functions
- `backend/src/lib.rs` - Register the new topic_index module
- `bridges/src/sutta_bridge.rs` - Add bridge functions for topic index: load, search, get by letter, get by headword
- `assets/qml/com/profoundlabs/simsapa/SuttaBridge.qml` - Add qmllint type definitions for new bridge functions
- `assets/qml/TopicIndexWindow.qml` - Main window component for the topic index feature
- `assets/qml/TopicIndexInfoDialog.qml` - Info dialog explaining CIPS and usage instructions
- `bridges/build.rs` - Register new QML files
- `cpp/topic_index_window.cpp` - C++ window class
- `cpp/topic_index_window.h` - C++ window class header
- `cpp/window_manager.cpp` - Add function to open TopicIndexWindow
- `cpp/window_manager.h` - Declare function to open TopicIndexWindow
- `cpp/gui.cpp` - Add callback for topic index window
- `cpp/gui.h` - Declare callback
- `bridges/src/api.rs` - Add callback_open_topic_index_window
- `assets/qml/SuttaSearchWindow.qml` - Add menu item "Windows > Topic Index..."
- `backend/tests/test_topic_index.rs` - Unit tests for topic index parsing and search functions

### Notes

- Run Rust backend tests with `cd backend && cargo test`
- Run QML tests with `make qml-test`
- Build the application with `make build -B` to verify compilation
- The CIPS general-index.csv source file should be obtained from the CIPS project (https://cips.dhammatalks.net/)

## Tasks

- [x] 1.0 Create CLI Parser for CIPS general-index.csv
  - [x] 1.1 Create new file `cli/src/bootstrap/parse_cips_index.rs` with module structure
  - [x] 1.2 Define Rust structs for JSON output: `TopicIndexLetter`, `TopicIndexHeadword`, `TopicIndexEntry`, `TopicIndexRef`
  - [x] 1.3 Implement `normalize_diacritic_string()` function using Unicode NFD normalization (similar to existing `latinize()` in helpers.rs)
  - [x] 1.4 Implement `make_normalized_id()` function to create valid anchor IDs from headwords (ā→aa, ī→ii, ū→uu, remove punctuation, spaces→hyphens)
  - [x] 1.5 Implement CSV parsing logic to read tab-delimited file with columns: headword, subheading, locator
  - [x] 1.6 Implement ignore words stripping for sorting ("in", "of", "with", "from", "to", "for", "on", "the", "as", "a", "an", "vs.", "and")
  - [x] 1.7 Implement headword sorting algorithm (case-insensitive, diacritic-insensitive, ignore leading articles)
  - [x] 1.8 Implement citation/locator sorting by canonical book order (DN, MN, SN, AN, Kp, Dhp, Ud, Iti, Snp, Vv, Pv, Thag, Thig) with natural number sorting
  - [x] 1.9 Implement cross-reference detection (locator contains "xref") and target extraction
  - [x] 1.10 Implement sutta reference parsing to extract book, sutta_ref, and segment ID
  - [x] 1.11 Implement blank sub-entry handling (replace with "—" em-dash for direct headword→sutta links)
  - [x] 1.12 Implement sutta title lookup from database (query Pāli titles like "mn5/pli/ms")
  - [x] 1.13 Build nested data structure: Letter → Headword → Entry → Refs
  - [x] 1.14 Implement JSON serialization with minified output (no pretty-printing)
  - [x] 1.15 Register module in `cli/src/bootstrap/mod.rs`
  - [x] 1.16 Add `ParseCipsIndex` subcommand to `cli/src/main.rs` with input CSV path and output JSON path arguments
  - [x] 1.17 Implement validation: check xref targets exist, no duplicate headword+sub combinations, locator format validation
  - [x] 1.18 Test parser with sample CIPS CSV data and verify output JSON structure

- [x] 2.0 Implement Static JSON Inclusion and Backend Functions
  - [x] 2.1 Run CLI parser to generate `assets/general-index.json` from CIPS CSV
  - [x] 2.2 Add `pub static CIPS_GENERAL_INDEX_JSON: &str = include_str!("../../assets/general-index.json");` to `backend/src/app_settings.rs`
  - [x] 2.3 Create new file `backend/src/topic_index.rs` with data structures matching JSON schema
  - [x] 2.4 Implement `TopicIndex` struct with `Vec<TopicIndexLetter>` and lazy-initialized cache using `OnceLock`
  - [x] 2.5 Implement `load_topic_index()` function to parse JSON and populate cache (called once on first access)
  - [x] 2.6 Implement `is_topic_index_loaded()` function to check if data is already cached
  - [x] 2.7 Implement `get_letters()` function returning array of available letters (A-Z)
  - [x] 2.8 Implement `get_headwords_for_letter(letter: &str)` function returning all headwords for a specific letter
  - [x] 2.9 Implement `search_headwords(query: &str)` function with case-insensitive partial matching on headwords, sub-entries, and Pāli terms in parentheses
  - [x] 2.10 Implement `get_headword_by_id(headword_id: &str)` function for xref navigation lookup
  - [x] 2.11 Register module in `backend/src/lib.rs`
  - [x] 2.12 Write unit tests in `backend/tests/test_topic_index.rs` for parsing, search, and lookup functions (tests in topic_index.rs module)
  - [x] 2.13 Implement `find_headword_id_by_text(target: &str)` function for xref navigation by headword text

- [x] 3.0 Create Bridge Functions and QML Type Definitions
  - [x] 3.1 Add `topic_index_loaded` property to `SuttaBridgeRust` struct in `bridges/src/sutta_bridge.rs`
  - [x] 3.2 Implement `load_topic_index()` bridge function that calls backend and sets `topic_index_loaded` property
  - [x] 3.3 Implement `is_topic_index_loaded()` bridge function
  - [x] 3.4 Implement `get_topic_index_letters()` bridge function returning QStringList
  - [x] 3.5 Implement `get_topic_headwords_for_letter(letter: &QString)` bridge function returning JSON string
  - [x] 3.6 Implement `search_topic_headwords(query: &QString)` bridge function returning JSON string with matching headwords and their entries
  - [x] 3.7 Implement `get_topic_headword_by_id(headword_id: &QString)` bridge function returning JSON string
  - [x] 3.8 Add `topicIndexLoaded` signal to SuttaBridge for async loading notification
  - [x] 3.9 Update `assets/qml/com/profoundlabs/simsapa/SuttaBridge.qml` with qmllint type definitions for all new functions
  - [x] 3.10 Verify bridge functions compile and are accessible from QML
  - [x] 3.11 Implement `find_topic_headword_id_by_text(target: &QString)` bridge function for xref navigation

- [x] 4.0 Create TopicIndexWindow.qml with Basic Structure
  - [x] 4.1 Create new file `assets/qml/TopicIndexWindow.qml` with ApplicationWindow base (following ReferenceSearchWindow pattern)
  - [x] 4.2 Add standard properties: `is_mobile`, `is_desktop`, `pointSize`, `top_bar_margin`, `is_dark`
  - [x] 4.3 Add ThemeHelper, Logger components
  - [x] 4.4 Set window properties: title "Topic Index - Simsapa", flags Qt.Dialog, responsive width/height
  - [x] 4.5 Create header RowLayout with Info button (left) and Close button (right, desktop only)
  - [x] 4.6 Create TopicIndexInfoDialog.qml with CIPS explanation, usage instructions, website link, and attribution
  - [x] 4.7 Connect Info button to open TopicIndexInfoDialog
  - [x] 4.8 Add main ColumnLayout structure for search, alphabet nav, and content areas
  - [x] 4.9 Add file to `qml_files` list in `bridges/build.rs`
  - [x] 4.10 Verify window displays correctly with `make build -B`

- [x] 5.0 Implement Alphabet Navigation and Letter Filtering
  - [x] 5.1 Add `current_letter` property (default "A") to TopicIndexWindow
  - [x] 5.2 Add `search_active` property (true when search input has 3+ characters)
  - [x] 5.3 Create alphabet navigation Row with A-Z buttons using Repeater and model ["A","B",...,"Z"]
  - [x] 5.4 Style letter buttons as radio buttons (only one active at a time) using `checked` property bound to `current_letter`
  - [x] 5.5 Implement button click handler to set `current_letter` and trigger data reload
  - [x] 5.6 Disable alphabet buttons (gray out) when `search_active` is true
  - [x] 5.7 Re-enable alphabet buttons when search input is cleared or has fewer than 3 characters
  - [x] 5.8 Call `SuttaBridge.get_topic_headwords_for_letter(current_letter)` when letter changes
  - [x] 5.9 Store headwords data in `headwords_for_letter` property

- [x] 6.0 Implement Search Functionality
  - [x] 6.1 Add search TextField below header (following ReferenceSearchWindow pattern)
  - [x] 6.2 Add `current_query` property to store search text
  - [x] 6.3 Add `search_results` property to store filtered results
  - [x] 6.4 Create debounce Timer with 300ms interval
  - [x] 6.5 Connect TextField.onTextChanged to restart debounce timer
  - [x] 6.6 Implement `perform_search()` function that requires minimum 3 characters
  - [x] 6.7 Call `SuttaBridge.search_topic_headwords(query)` for search
  - [x] 6.8 Parse JSON results and populate `search_results`
  - [x] 6.9 Set `search_active = true` when query.length >= 3, `false` otherwise
  - [x] 6.10 Show "Enter at least 3 characters" message when query.length < 3 and > 0
  - [x] 6.11 Show "No results found" when search returns empty

- [x] 7.0 Implement Topic List Display with Sutta Links
  - [x] 7.1 Create ScrollView containing ListView for topic display
  - [x] 7.2 Create delegate component for headword items (bold/prominent styling)
  - [x] 7.3 Create delegate component for sub-entry items (indented under headword)
  - [x] 7.4 Implement depth-based indentation (similar to ChapterListItem.qml pattern, 20px per level)
  - [x] 7.5 Display sub-entries with empty sub-word as link/xref only (no sub-word text)
  - [x] 7.6 Format sutta references with space: "AN 4.10" not "AN4.10"
  - [x] 7.7 Append Pāli title after reference: "AN 4.10 Yogasutta"
  - [x] 7.8 Style sutta links with dashed underline and palette.link color
  - [x] 7.9 Style cross-references distinctly with "see:" or "see also:" prefix and italic text
  - [x] 7.10 Add "Open in new window" checkbox (following ReferenceSearchWindow pattern)
  - [x] 7.11 Implement sutta link click handler using full sutta_ref with segment ID
  - [x] 7.12 Include hashtag fragment for segment navigation (e.g., "#dn3:2.3.0")
  - [x] 7.13 Call `SuttaBridge.emit_show_sutta_from_reference_search()` or `open_sutta_search_window_with_result()` based on checkbox
  - [x] 7.14 Show list of headwords for current letter when not searching
  - [x] 7.15 Show search results when searching

- [x] 8.0 Implement Cross-Reference Navigation and Highlighting
  - [x] 8.1 Add `highlighted_headword_id` property to track which headword to highlight
  - [x] 8.2 Implement xref click handler to get target headword's letter section
  - [x] 8.3 On xref click: clear search, set `current_letter` to target's letter
  - [x] 8.4 Set `highlighted_headword_id` to target headword's ID
  - [ ] 8.5 Implement ListView scrolling to highlighted headword using `positionViewAtIndex()` - TODO
  - [x] 8.6 Apply highlight background color (palette.highlight with reduced opacity) to highlighted item
  - [x] 8.7 Keep highlight visible until user selects another item (no timeout)
  - [x] 8.8 Implement search result headword click: clear search, navigate to letter, highlight headword
  - [x] 8.9 Clear highlight when user clicks a different headword or starts new search

- [x] 9.0 Add Mobile Support and Window Management Integration
  - [x] 9.1 Set mobile dimensions: `Screen.desktopAvailableWidth` x `Screen.desktopAvailableHeight`
  - [x] 9.2 Add `top_bar_margin` spacing for mobile status bar (call `SuttaBridge.get_mobile_top_bar_margin()`)
  - [x] 9.3 Add extra bottom margin (60px) for mobile navigation bar
  - [x] 9.4 Show Close button in top-right on both desktop and mobile
  - [x] 9.5 Add `open_topic_index_window()` function to `cpp/window_manager.cpp`
  - [x] 9.6 Declare function in `cpp/window_manager.h`
  - [x] 9.7 Add callback bridge function in `bridges/src/api.rs` if needed
  - [x] 9.8 Add `open_topic_index_window()` function to SuttaBridge
  - [x] 9.9 Add menu item "Topic Index..." under Windows menu in SuttaSearchWindow.qml
  - [x] 9.10 Connect menu item to `SuttaBridge.open_topic_index_window()`

- [x] 10.0 Data Loading and Loading State
  - [x] 10.1 Add `is_loading` property to TopicIndexWindow (default true)
  - [x] 10.2 Call `SuttaBridge.load_topic_index()` in Component.onCompleted
  - [x] 10.3 Connect to `topicIndexLoaded` signal to set `is_loading = false`
  - [x] 10.4 Disable search input and alphabet buttons while `is_loading` is true
  - [x] 10.5 Show "Loading..." message in content area while loading
  - [x] 10.6 Enable search input and alphabet buttons after loading completes
  - [x] 10.7 Display initial letter "A" content after loading

- [ ] 11.0 Testing and Validation
  - [x] 11.1 Test CLI parser with full CIPS general-index.csv (~21,742 entries)
  - [x] 11.2 Verify JSON output structure matches PRD specification
  - [ ] 11.3 Test search with English terms, Pāli terms, and partial matches
  - [ ] 11.4 Test cross-reference navigation and highlighting
  - [ ] 11.5 Test sutta link opening in same window and new window
  - [ ] 11.6 Test mobile layout on Android emulator or device
  - [ ] 11.7 Test theme switching (light/dark mode)
  - [ ] 11.8 Verify performance: load time < 1 second, search results < 300ms
  - [x] 11.9 Test keyboard shortcut for search focus (Ctrl+L if implemented)
  - [x] 11.10 Run `make build -B` and verify no compilation errors
  - [x] 11.11 Run `cd backend && cargo test` and verify all tests pass
