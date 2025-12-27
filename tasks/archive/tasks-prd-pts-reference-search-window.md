# Tasks: PTS Reference Search Window

## Relevant Files

- `backend/src/pts_reference_search.rs` - New module containing all PTS reference search functions
- `backend/tests/test_pts_reference_search.rs` - Comprehensive test suite matching JavaScript implementation
- `bridges/src/sutta_bridge.rs` - Extended with reference search bridge functions
- `assets/qml/com/profoundlabs/simsapa/SuttaBridge.qml` - QML type definition for qmllint
- `assets/qml/ReferenceSearchWindow.qml` - New search window component
- `assets/qml/SuttaSearchWindow.qml` - Modified to add menu item for reference search
- `bridges/build.rs` - Updated to register new QML files
- `reference-coverter-html/search_functions.js` - Reference implementation for test case validation
- `assets/sutta-reference-converter.json` - JSON data file with reference mappings

### Notes

- **Testing:** Run `cd backend && cargo test test_pts_reference_search` for Rust tests
- **Build:** Use `make build -B` to rebuild after adding new QML components
- **All Tests:** Use `make test` to run all tests (Rust + QML + JavaScript)
- The JavaScript reference implementation in `reference-coverter-html/search_functions.js` provides the expected behavior that Rust tests should replicate

## Tasks

- [x] 1.0 Implement Backend Search Functions
  - [x] 1.1 Create `backend/src/pts_reference_search.rs` module file
  - [x] 1.2 Define `PTSReference` struct with nikaya, volume, and page fields
  - [x] 1.3 Define `ReferenceSearchResult` struct with identifier, name, pts_reference, dpr_reference, and url fields (with Serialize/Deserialize)
  - [x] 1.4 Implement `parse_pts_reference(pts_ref: &str) -> Option<PTSReference>` function to parse PTS reference strings like "D ii 20" into components
  - [x] 1.5 Implement `search_by_text(query: &str, field: &str) -> Vec<ReferenceSearchResult>` function with normalization using `helpers::latinize()`
  - [x] 1.6 Implement `search_by_pts_reference(query: &str) -> Vec<ReferenceSearchResult>` function with range-based matching algorithm
  - [x] 1.7 Implement `search(query: &str, field: &str) -> Vec<ReferenceSearchResult>` universal search function that routes to appropriate search method
  - [x] 1.8 Load JSON data from `app_settings::SUTTA_REFERENCE_CONVERTER_JSON` constant and parse into vector of reference entries
  - [x] 1.9 Add module declaration to `backend/src/lib.rs` to expose `pts_reference_search` module
  - [x] 1.10 Verify compilation with `cd backend && cargo build`

- [x] 2.0 Create and Verify Comprehensive Test Suite
  - [x] 2.1 Create `backend/tests/test_pts_reference_search.rs` test file
  - [x] 2.2 Add test case: Search DN 1 by identifier "DN 1" in 'identifier' field should find DN 1
  - [x] 2.3 Add test case: Search DN 2 by PTS ref (exact) "D i 47" in 'pts_reference' field should find D i 47
  - [x] 2.4 Add test case: Search DN 2 by PTS ref (in-between) "D i 50" in 'pts_reference' field should find D i 47
  - [x] 2.5 Add test case: Search DN 14 by PTS ref (exact at volume boundary) "D ii 1" in 'pts_reference' field should find D ii 1
  - [x] 2.6 Add test case: Search DN 14 by PTS ref (in-between) "D ii 20" in 'pts_reference' field should find D ii 1
  - [x] 2.7 Add test case: Search MN by PTS ref (in-between) "M iii 10" in 'pts_reference' field should find M iii 7
  - [x] 2.8 Add test case: Search by name (case insensitive) "brahmajala" in 'name' field should find "Brahmajāla"
  - [x] 2.9 Add test case: Search KN by DPR reference "KN 1" in 'dpr_reference' field should find KN 1
  - [x] 2.10 Run all tests with `cd backend && cargo test test_pts_reference_search` and verify 100% pass rate
  - [x] 2.11 Compare test results with JavaScript reference implementation to ensure behavioral parity

- [x] 3.0 Implement Bridge Functions
  - [x] 3.1 Add `search_reference(query: String, field: String) -> String` invokable function to `bridges/src/sutta_bridge.rs` that returns JSON string of search results
  - [x] 3.2 Add `verify_sutta_uid_exists(uid: String) -> bool` invokable function to check if UID exists in appdata database
  - [x] 3.3 Add `extract_uid_from_url(url: String) -> String` invokable function to parse UID from SuttaCentral URL (e.g., extract "sn56.102" from "https://suttacentral.net/sn56.102")
  - [x] 3.4 Add corresponding QML type definition functions in `assets/qml/com/profoundlabs/simsapa/SuttaBridge.qml` with correct function signatures for qmllint
  - [x] 3.5 Verify compilation with `make build -B`

- [x] 4.0 Create Reference Search Window UI
  - [x] 4.1 Create `assets/qml/ReferenceSearchWindow.qml` as new ApplicationWindow with appropriate title, size, and theming support using ThemeHelper
  - [x] 4.2 Add window to `bridges/build.rs` qml_files list
  - [x] 4.3 Implement search input controls: TextField with placeholder "Enter PTS reference, DPR reference, or title..." and ComboBox with "PTS Ref", "DPR Ref", "Title" options
  - [x] 4.4 Implement real-time search with debouncing (~300ms) that triggers on text input changes
  - [x] 4.5 Create JSON results list component showing all matches from reference data with identifier, name, and pts_reference
  - [x] 4.6 Add UID verification logic that calls `verify_sutta_uid_exists()` and displays "(Not found in database)" in subdued color for missing entries
  - [x] 4.7 Create database results list component showing only entries that exist in database with formatted sutta_ref and title
  - [x] 4.8 Add "Open" button to each database result item that calls appropriate window manager function to display sutta in SuttaSearchWindow
  - [x] 4.9 Add "Copy URL" button to each database result item that uses ClipboardManager to copy full SuttaCentral URL
  - [x] 4.10 Implement empty states: helpful message when no query entered, "No results found" when search returns empty, and example queries
  - [x] 4.11 Implement proper error handling with error dialog for JSON parsing failures and clipboard operation failures
  - [x] 4.12 Add keyboard shortcut Ctrl+F to focus search input when window is active
  - [x] 4.13 Verify compilation with `make build -B` and test window can be instantiated

- [x] 5.0 Integrate Window with Application Menu
  - [x] 5.1 Open `assets/qml/SuttaSearchWindow.qml` and locate the menu bar definition
  - [x] 5.2 Add "Reference Search" menu item under Windows menu with optional keyboard shortcut Ctrl+Shift+R
  - [x] 5.3 Connect menu item to open ReferenceSearchWindow when clicked
  - [x] 5.4 Verify compilation with `make build -B`
  - [x] 5.5 Test that menu item opens the reference search window correctly

- [ ] 6.0 Testing and Polish
  - [ ] 6.1 Test PTS reference exact match: search "D i 47" should find DN 2 and allow opening it
  - [ ] 6.2 Test PTS reference range match: search "D i 50" should find DN 2 (which starts at D i 47)
  - [ ] 6.3 Test volume boundary: search "D ii 1" should find DN 14
  - [ ] 6.4 Test text search case insensitivity: search "brahmajala" should find "Brahmajāla Sutta"
  - [ ] 6.5 Test DPR reference search: search "KN 1" in DPR Ref field should find matching entry
  - [ ] 6.6 Test "Copy URL" button copies correct SuttaCentral URL to clipboard
  - [ ] 6.7 Test "Open" button opens sutta in SuttaSearchWindow HTML view
  - [ ] 6.8 Test that entries not in database show "(Not found in database)" indicator
  - [ ] 6.9 Test responsive layout on desktop and mobile (if applicable)
  - [ ] 6.10 Test window remains open after opening a sutta for repeated lookups
  - [ ] 6.11 Run full test suite with `make test` to ensure no regressions
  - [ ] 6.12 Verify search performance is under 200ms for typical queries
