## Relevant Files

- `backend/migrations/appdata/<timestamp>_create_chanting_tables/up.sql` - Migration to create the 4 chanting tables
- `backend/migrations/appdata/<timestamp>_create_chanting_tables/down.sql` - Migration rollback
- `backend/src/db/appdata_schema.rs` - Auto-generated Diesel schema (updated by `diesel print-schema`)
- `backend/src/db/appdata_models.rs` - Rust structs for chanting tables (Queryable + Insertable)
- `backend/src/db/appdata.rs` - CRUD functions for chanting data
- `backend/src/lib.rs` - `get_chanting_recordings_dir()` helper and directory creation
- `bridges/src/sutta_bridge.rs` - Bridge functions exposed to QML (or a new `chanting_bridge.rs`)
- `bridges/src/api.rs` - FFI callback declarations for opening chanting windows
- `bridges/build.rs` - QML and Rust file registration for CXX-Qt build
- `cpp/chanting_practice_window.h` - C++ window class header
- `cpp/chanting_practice_window.cpp` - C++ window class implementation
- `cpp/chanting_review_window.h` - C++ review window class header
- `cpp/chanting_review_window.cpp` - C++ review window class implementation
- `cpp/window_manager.h` - Register new window types and creation methods
- `cpp/window_manager.cpp` - Window creation and cleanup implementations
- `cpp/gui.cpp` - C++ callback functions invoked from Rust FFI
- `assets/qml/ChantingPracticeWindow.qml` - Main chanting browser window with 3-level tree
- `assets/qml/ChantingPracticeReviewWindow.qml` - Review window with text + playback area
- `assets/qml/RecordingPlaybackItem.qml` - Reusable audio record/playback/marker component
- `assets/qml/ChantingTreeList.qml` - Tree list component for collections/chants/sections (similar to BooksList.qml)
- `assets/qml/com/profoundlabs/simsapa/SuttaBridge.qml` - qmllint type definition updates
- `assets/qml/com/profoundlabs/simsapa/qmldir` - QML module directory (if adding new bridge type)
- `assets/qml/SuttaSearchWindow.qml` - Add menu item for Chanting Practice window
- `CMakeLists.txt` - Link Qt::Multimedia module
- `android/AndroidManifest.xml` - Add RECORD_AUDIO permission

### Notes

- Use `cd backend && cargo test` to run Rust backend tests.
- Use `make qml-test` to run QML tests (offscreen platform).
- Use `make build -B` to verify compilation. Do NOT run the GUI for testing.
- After modifying `appdata_schema.rs`, run `cd backend && diesel print-schema --database-url <path> > src/db/appdata_schema.rs` or manually update.
- All file existence checks must use `try_exists()` instead of `.exists()` for Android compatibility.

## Tasks

- [ ] 1.0 Database Schema & Models
  - [ ] 1.1 Create Diesel migration directory `backend/migrations/appdata/<timestamp>_create_chanting_tables/`
  - [ ] 1.2 Write `up.sql`: CREATE TABLE for `chanting_collections` (id, uid, title, description, language, sort_index, is_user_added, metadata_json, created_at, updated_at) with UNIQUE index on uid
  - [ ] 1.3 Write `up.sql`: CREATE TABLE for `chanting_chants` (id, uid, collection_uid, title, description, sort_index, is_user_added, metadata_json, created_at, updated_at) with UNIQUE index on uid and index on collection_uid
  - [ ] 1.4 Write `up.sql`: CREATE TABLE for `chanting_sections` (id, uid, chant_uid, title, content_pali, sort_index, is_user_added, metadata_json, created_at, updated_at) with UNIQUE index on uid and index on chant_uid
  - [ ] 1.5 Write `up.sql`: CREATE TABLE for `chanting_recordings` (id, uid, section_uid, file_name, recording_type, label, duration_ms, markers_json, created_at, updated_at) with UNIQUE index on uid and index on section_uid
  - [ ] 1.6 Write `down.sql`: DROP TABLE for all 4 tables in reverse order
  - [ ] 1.7 Update `appdata_schema.rs` with Diesel table! macros for all 4 tables
  - [ ] 1.8 Add Queryable structs to `appdata_models.rs`: `ChantingCollection`, `ChantingChant`, `ChantingSection`, `ChantingRecording` with appropriate derives (Queryable, Selectable, Identifiable, PartialEq, Debug, Clone)
  - [ ] 1.9 Add Insertable structs to `appdata_models.rs`: `NewChantingCollection`, `NewChantingChant`, `NewChantingSection`, `NewChantingRecording`
  - [ ] 1.10 Add JSON serializable structs for passing data to QML: `ChantingCollectionJson`, `ChantingChantJson`, `ChantingSectionJson`, `ChantingRecordingJson` (with serde Serialize/Deserialize)
  - [ ] 1.11 Verify `cargo build` succeeds in backend crate

- [ ] 2.0 Rust Backend CRUD Operations
  - [ ] 2.1 In `appdata.rs`, add `get_all_chanting_collections()` — returns all collections with nested chants and sections as a JSON-serializable tree structure
  - [ ] 2.2 Add `get_chanting_section_detail(section_uid)` — returns section data with all associated recordings
  - [ ] 2.3 Add `create_chanting_collection()`, `update_chanting_collection()`, `delete_chanting_collection()` (cascade deletes chants, sections, recordings + files)
  - [ ] 2.4 Add `create_chanting_chant()`, `update_chanting_chant()`, `delete_chanting_chant()` (cascade deletes sections, recordings + files)
  - [ ] 2.5 Add `create_chanting_section()`, `update_chanting_section()`, `delete_chanting_section()` (cascade deletes recordings + files)
  - [ ] 2.6 Add `create_chanting_recording()`, `delete_chanting_recording(uid)` — delete also removes the audio file from disk using `try_exists()` check
  - [ ] 2.7 Add `update_recording_markers(uid, markers_json)` — updates the markers_json field for a recording
  - [ ] 2.8 In `backend/src/lib.rs`, add `get_chanting_recordings_dir()` that returns the resolved path (`<simsapa_dir>/app-assets/chanting-recordings/`), creating the directory if it doesn't exist (using `try_exists()`)
  - [ ] 2.9 Write unit tests for CRUD operations: create, read, update, delete for each table, and verify cascade deletes
  - [ ] 2.10 Verify `cargo test` passes

- [ ] 3.0 Rust Bridge & C++ Window Scaffold
  - [ ] 3.1 Add bridge functions to `sutta_bridge.rs` (extern "RustQt" block): `open_chanting_practice_window()`, `open_chanting_review_window(section_uid)`, `get_all_chanting_collections_json()`, `get_chanting_section_detail_json(section_uid)`, `get_chanting_recordings_dir()`
  - [ ] 3.2 Add CRUD bridge functions: `create_chanting_collection(json)`, `update_chanting_collection(json)`, `delete_chanting_collection(uid)`, and equivalent for chants, sections, and recordings
  - [ ] 3.3 Add `update_recording_markers(uid, markers_json)` bridge function
  - [ ] 3.4 Implement all bridge functions in `sutta_bridge.rs`, calling the backend functions from task 2.0
  - [ ] 3.5 Update `assets/qml/com/profoundlabs/simsapa/SuttaBridge.qml` with qmllint type definitions for all new functions
  - [ ] 3.6 Add FFI callback declarations in `bridges/src/api.rs`: `callback_open_chanting_practice_window()`, `callback_open_chanting_review_window(section_uid: &QString)`
  - [ ] 3.7 Create `cpp/chanting_practice_window.h` and `cpp/chanting_practice_window.cpp` following the LibraryWindow pattern
  - [ ] 3.8 Create `cpp/chanting_review_window.h` and `cpp/chanting_review_window.cpp` — constructor takes `section_uid` QString and passes it to QML as a context property
  - [ ] 3.9 Register both windows in `cpp/window_manager.h` and `cpp/window_manager.cpp`: add creation methods, window lists, and destructor cleanup
  - [ ] 3.10 Add callback implementations in `cpp/gui.cpp`: `callback_open_chanting_practice_window()` and `callback_open_chanting_review_window()`
  - [ ] 3.11 Create placeholder `assets/qml/ChantingPracticeWindow.qml` (minimal ApplicationWindow with title)
  - [ ] 3.12 Create placeholder `assets/qml/ChantingPracticeReviewWindow.qml` (minimal ApplicationWindow with title showing section_uid)
  - [ ] 3.13 Verify `make build -B` compiles successfully

- [ ] 4.0 ChantingPracticeWindow QML
  - [ ] 4.1 Create `assets/qml/ChantingTreeList.qml` — a reusable tree list component (following BooksList.qml pattern) that takes a JSON model of collections/chants/sections and renders 3-level expand/collapse tree
  - [ ] 4.2 Implement ChantingPracticeWindow.qml: ApplicationWindow with ThemeHelper, mobile support (is_mobile, top_bar_margin), standard window flags
  - [ ] 4.3 Add Component.onCompleted to load data via `SuttaBridge.get_all_chanting_collections_json()` and parse into the tree model
  - [ ] 4.4 Wire section item click to call `SuttaBridge.open_chanting_review_window(section_uid)`
  - [ ] 4.5 Add toolbar with "Add Collection" button — opens dialog with title and description fields, calls `SuttaBridge.create_chanting_collection()`, refreshes list
  - [ ] 4.6 Add "Add Chant" toolbar button — enabled when a collection is selected, opens dialog, calls `SuttaBridge.create_chanting_chant()`, refreshes list
  - [ ] 4.7 Add "Add Section" toolbar button — enabled when a chant is selected, opens dialog with title and content_pali (multiline) fields, calls `SuttaBridge.create_chanting_section()`, refreshes list
  - [ ] 4.8 Add "Edit" toolbar button — opens dialog pre-filled with selected item's data, calls appropriate update function
  - [ ] 4.9 Add "Remove" toolbar button — shows confirmation dialog, calls appropriate delete function, refreshes list
  - [ ] 4.10 Register `ChantingTreeList.qml` in `bridges/build.rs` qml_files list
  - [ ] 4.11 Verify `make build -B` compiles successfully

- [ ] 5.0 Menu Integration & Build Configuration
  - [ ] 5.1 In `CMakeLists.txt`, add `Qt::Multimedia` to the qt_modules list and `Multimedia` to CXXQT_QTCOMPONENTS
  - [ ] 5.2 In `android/AndroidManifest.xml`, add `<uses-permission android:name="android.permission.RECORD_AUDIO" />`
  - [ ] 5.3 Register all new QML files in `bridges/build.rs` qml_files list: `ChantingPracticeWindow.qml`, `ChantingPracticeReviewWindow.qml`, `RecordingPlaybackItem.qml`, `ChantingTreeList.qml`
  - [ ] 5.4 In `SuttaSearchWindow.qml`, add "Chanting Practice..." menu item in the Windows menu after action_topic_index, triggering `SuttaBridge.open_chanting_practice_window()`
  - [ ] 5.5 Add new C++ files to CMakeLists.txt sources: `chanting_practice_window.cpp`, `chanting_review_window.cpp`
  - [ ] 5.6 Verify `make build -B` compiles and the full window chain is wired (menu → bridge → FFI → C++ → QML)

- [ ] 6.0 RecordingPlaybackItem — Basic Record & Playback
  - [ ] 6.1 Create `assets/qml/RecordingPlaybackItem.qml` with required properties: `recording_uid`, `file_path`, `label`, `recording_type`, `is_new_recording` (bool)
  - [ ] 6.2 Add audio playback using MediaPlayer + AudioOutput: play/pause/stop buttons, source bound to file_path
  - [ ] 6.3 Add scrubber Slider bound to `player.position` / `player.duration` with seek-on-drag, and time display (`MM:SS / MM:SS`)
  - [ ] 6.4 Add audio recording using CaptureSession + AudioInput + MediaRecorder: record/stop button, OGG/Opus format, output to chanting-recordings directory
  - [ ] 6.5 Add Android runtime permission check using MicrophonePermission (Qt 6.5+) before starting recording; show user-facing message if denied
  - [ ] 6.6 Add recording state UI: pulsing red indicator and elapsed time during recording
  - [ ] 6.7 Add header row with recording label and close button (x), emit `closed()` signal
  - [ ] 6.8 On recording stop, emit signal with file path so the parent can call `SuttaBridge.create_chanting_recording()` to persist to database
  - [ ] 6.9 Verify `make build -B` compiles successfully

- [ ] 7.0 ChantingPracticeReviewWindow QML
  - [ ] 7.1 Implement ChantingPracticeReviewWindow.qml: ApplicationWindow with ThemeHelper, mobile support, receives `section_uid` context property
  - [ ] 7.2 On Component.onCompleted, call `SuttaBridge.get_chanting_section_detail_json(section_uid)` and parse response into section data + recordings list
  - [ ] 7.3 Add header showing section title, parent chant title, and collection title
  - [ ] 7.4 Add scrollable Pali text area displaying `content_pali` in a readable font
  - [ ] 7.5 Add recording list panel grouped by type (Reference / User recordings), each with an "Open" button and delete button (user recordings only, non-bundled references)
  - [ ] 7.6 Add "New Recording" button that creates a RecordingPlaybackItem in recording mode (is_new_recording: true) and adds it to the playback area
  - [ ] 7.7 Add "Add Reference Recording" button with file picker dialog — copies selected audio file to chanting-recordings dir, creates database record, refreshes list
  - [ ] 7.8 Add scrollable playback area (ColumnLayout inside ScrollView) that holds dynamically created RecordingPlaybackItem instances
  - [ ] 7.9 Wire "Open" button on each recording list item to instantiate a RecordingPlaybackItem with the recording's file path and add it to the playback area
  - [ ] 7.10 Wire RecordingPlaybackItem close signal to remove the instance from the playback area
  - [ ] 7.11 Wire RecordingPlaybackItem recording-complete signal to persist the new recording via bridge and refresh the recording list
  - [ ] 7.12 Verify `make build -B` compiles successfully

- [ ] 8.0 RecordingPlaybackItem — Markers & Enhanced Playback
  - [ ] 8.1 Add marker data model: parse `markers_json` from recording into a ListModel of position and range markers
  - [ ] 8.2 Add "Add Position Marker" button — creates a position marker at current `player.position`, assigns default label, adds to model, saves via `SuttaBridge.update_recording_markers()`
  - [ ] 8.3 Add "Add Range Marker" button with two-click mode: first click sets start_ms, visual indicator shown, second click sets end_ms, creates range marker, saves
  - [ ] 8.4 Render position markers on scrubber as thin vertical lines (2px, accent color) using Repeater over marker model
  - [ ] 8.5 Render range markers on scrubber as semi-transparent colored rectangles spanning the range
  - [ ] 8.6 Add marker list below scrubber showing all markers with label, type, time(s), and delete button
  - [ ] 8.7 Wire position marker click (in list or on scrubber) to seek player to that position
  - [ ] 8.8 Wire range marker click to play only that range: seek to start_ms, play, stop at end_ms using polling Timer (~50ms)
  - [ ] 8.9 Add "Loop" checkbox — when checked and a range marker is active, seek back to start_ms instead of pausing when end_ms is reached
  - [ ] 8.10 Add editable marker labels (inline text edit on click)
  - [ ] 8.11 Wire marker delete button to remove from model and save updated markers_json via bridge
  - [ ] 8.12 Verify `make build -B` compiles successfully

- [ ] 9.0 Default Chanting Data & Testing
  - [ ] 9.1 Identify and extract chanting text content from the Bhikkhu Manual data already in the app's Library (collections, chants, sections structure with Pali text)
  - [ ] 9.2 Identify and prepare Patimokkha chanting content (collections, chants, sections with Pali text)
  - [ ] 9.3 Create a seed data migration or Rust initialization function that inserts the default chanting collections, chants, and sections into appdata.sqlite3 (with is_user_added = false)
  - [ ] 9.4 Write Rust unit tests: CRUD for chanting_collections (create, read, update, delete)
  - [ ] 9.5 Write Rust unit tests: CRUD for chanting_chants with cascade delete verification
  - [ ] 9.6 Write Rust unit tests: CRUD for chanting_sections with cascade delete verification
  - [ ] 9.7 Write Rust unit tests: CRUD for chanting_recordings, including file cleanup on delete
  - [ ] 9.8 Write Rust unit tests: marker JSON serialization/deserialization and update_recording_markers
  - [ ] 9.9 Write QML tests for RecordingPlaybackItem: verify component loads, properties bind correctly
  - [ ] 9.10 Verify `cargo test` and `make qml-test` pass
