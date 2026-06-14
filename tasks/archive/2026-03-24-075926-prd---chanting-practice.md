# PRD: Chanting Practice Feature

## 1. Introduction / Overview

Simsapa is a Theravada Tipitaka reader app. This feature adds a **Chanting Practice** module that allows users to browse collections of Pali chants, record their own chanting attempts, compare against reference recordings, and use markers to review specific passages. The goal is to provide an integrated practice tool for learning and improving Pali chanting.

## 2. Goals

- Provide a browsable, hierarchical list of chanting collections, chants, and sections.
- Ship pre-populated chanting data while allowing users to add custom collections, chants, and sections.
- Allow users to record multiple audio attempts per chanting section and persist them across sessions.
- Support reference recordings (bundled or user-added) for comparison.
- Enable precise review with position markers (problem spots) and range markers (verse/line boundaries).
- Support both desktop (Linux/macOS/Windows) and Android/iOS platforms.

## 3. User Stories

- **As a Pali student**, I want to browse a list of chanting collections so that I can find the chant I want to practice.
- **As a practitioner**, I want to open a chanting section and see its Pali text so that I can read along while practicing.
- **As a practitioner**, I want to record my chanting and play it back so that I can hear how I sound.
- **As a practitioner**, I want to keep multiple recordings per section so that I can track my progress over time.
- **As a practitioner**, I want to listen to a reference recording so that I can compare my chanting against a correct version.
- **As a practitioner**, I want to place position markers on the scrubber to mark problem spots so that I can quickly jump back to them.
- **As a practitioner**, I want to place range markers to define a verse or line so that I can loop playback over just that portion.
- **As a power user**, I want to add my own chanting collections, chants, sections, and reference recordings so that I can practice material not included in the app.

## 4. Functional Requirements

### 4.1 Database Models

New tables in the appdata database (`backend/src/db/appdata_models.rs`), with corresponding Diesel schema:

1. **`chanting_collections`** — Top-level grouping (e.g., "Wat Pah Nanachat Chanting Book").
   - `id` (INTEGER PRIMARY KEY)
   - `uid` (TEXT UNIQUE NOT NULL) — stable identifier
   - `title` (TEXT NOT NULL)
   - `description` (TEXT, nullable)
   - `language` (TEXT, default "pali")
   - `sort_index` (INTEGER, default 0)
   - `is_user_added` (BOOLEAN, default false)
   - `metadata_json` (TEXT, nullable)
   - `created_at` (TEXT NOT NULL)
   - `updated_at` (TEXT NOT NULL)

2. **`chanting_chants`** — A chant within a collection (e.g., "Morning Chanting").
   - `id` (INTEGER PRIMARY KEY)
   - `uid` (TEXT UNIQUE NOT NULL)
   - `collection_uid` (TEXT NOT NULL) — FK to `chanting_collections.uid`
   - `title` (TEXT NOT NULL)
   - `description` (TEXT, nullable)
   - `sort_index` (INTEGER, default 0)
   - `is_user_added` (BOOLEAN, default false)
   - `metadata_json` (TEXT, nullable)
   - `created_at` (TEXT NOT NULL)
   - `updated_at` (TEXT NOT NULL)

3. **`chanting_sections`** — A section within a chant (e.g., "Homage to the Buddha").
   - `id` (INTEGER PRIMARY KEY)
   - `uid` (TEXT UNIQUE NOT NULL)
   - `chant_uid` (TEXT NOT NULL) — FK to `chanting_chants.uid`
   - `title` (TEXT NOT NULL)
   - `content_pali` (TEXT NOT NULL) — the Pali text to display
   - `sort_index` (INTEGER, default 0)
   - `is_user_added` (BOOLEAN, default false)
   - `metadata_json` (TEXT, nullable)
   - `created_at` (TEXT NOT NULL)
   - `updated_at` (TEXT NOT NULL)

4. **`chanting_recordings`** — Audio recordings (user attempts and reference recordings).
   - `id` (INTEGER PRIMARY KEY)
   - `uid` (TEXT UNIQUE NOT NULL)
   - `section_uid` (TEXT NOT NULL) — FK to `chanting_sections.uid`
   - `file_name` (TEXT NOT NULL) — relative path within the chanting-recordings directory
   - `recording_type` (TEXT NOT NULL) — `"user"` or `"reference"`
   - `label` (TEXT, nullable) — user-facing name (e.g., "Attempt 3", "Ajahn Example")
   - `duration_ms` (INTEGER, default 0)
   - `markers_json` (TEXT, nullable) — JSON array of marker objects (see 4.5)
   - `created_at` (TEXT NOT NULL)
   - `updated_at` (TEXT NOT NULL)

### 4.2 ChantingPracticeWindow

**File:** `assets/qml/ChantingPracticeWindow.qml`

A window showing a hierarchical tree of chanting material, similar to `LibraryWindow.qml`.

- **Structure:** Three-level tree view:
  - **Collection** (top level, expandable)
    - **Chant** (second level, expandable)
      - **Section** (leaf items, clickable to open review window)
- Data is loaded from the Rust backend via a bridge function returning JSON.
- Each level shows the title. Collections and chants are expandable tree nodes.
- Clicking a section opens `ChantingPracticeReviewWindow` for that section.
- **Toolbar actions:**
  - "Add Collection" — dialog to create a new collection (title, description).
  - "Add Chant" — dialog to add a chant to the selected collection (title, description).
  - "Add Section" — dialog to add a section to the selected chant (title, Pali text).
  - "Edit" — edit the selected item's title/description/text.
  - "Remove" — delete the selected item (with confirmation dialog).

### 4.3 ChantingPracticeReviewWindow

**File:** `assets/qml/ChantingPracticeReviewWindow.qml`

A window for practicing a specific chanting section. Opened from ChantingPracticeWindow when a section is clicked.

**Layout (top to bottom):**

1. **Header:** Section title and parent chant/collection names.
2. **Pali Text Area:** Scrollable area displaying the section's `content_pali` text in a readable font size.
3. **Recording List:** A list of all recordings for this section, grouped by type:
   - **Reference recordings** — labeled, non-deletable if bundled.
   - **User recordings** — labeled with creation date, deletable.
   - Each recording has an "Open" button that adds a `RecordingPlaybackItem` to the playback area below.
   - A "New Recording" button to start recording a new attempt.
4. **Playback Area:** A scrollable vertical list of open `RecordingPlaybackItem` components. Multiple items can be open simultaneously for side-by-side comparison (e.g., a reference recording and a user recording). Each item can be closed individually.

### 4.3.1 RecordingPlaybackItem (Reusable QML Component)

**File:** `assets/qml/RecordingPlaybackItem.qml`

A self-contained, reusable QML component for audio playback, recording, and marker management. Multiple instances can be displayed in the review window at the same time.

**Each instance contains:**

1. **Header row:** Recording label (e.g., "Reference — Ajahn Example" or "Attempt 3 — 2026-03-24"), a close button (x).
2. **Audio Controls:**
   - **Record button** (only for new/user recordings) — starts/stops recording. When recording, shows elapsed time and a pulsing indicator.
   - **Play/Pause button** — plays/pauses the recording.
   - **Stop button** — stops playback and resets position to start.
3. **Scrubber / Timeline:**
   - A horizontal slider showing playback position within the recording.
   - Displays current time and total duration (e.g., `01:23 / 03:45`).
   - User can drag to seek.
   - Markers are displayed as visual indicators on the scrubber (see 4.5).
4. **Marker Controls:**
   - "Add Position Marker" button — places a point marker at the current playback position.
   - "Add Range Marker" button — enters range-marking mode: first click sets start, second click sets end.
   - "Loop" checkbox — when checked, A-B range playback repeats automatically instead of stopping.
   - List of existing markers with labels, showing type (position/range) and time(s).
   - Click a position marker to seek to it.
   - Click a range marker to play only that range.
   - Delete button per marker.

### 4.4 Audio Recording & Playback

**Qt Multimedia QML components used:**

- **Recording:** `CaptureSession` + `AudioInput` + `MediaRecorder`
- **Playback:** `MediaPlayer` + `AudioOutput`

**Format:** OGG/Opus (`MediaFormat.Ogg` container, `MediaFormat.Opus` codec). Falls back to best available if unsupported.

**File storage:**
- Desktop: `~/.local/share/simsapa-ng/app-assets/chanting-recordings/`
- Android: `<app-files-dir>/.local/share/simsapa-ng/app-assets/chanting-recordings/`
- File naming convention: `{section_uid}_{timestamp}.ogg`
- The Rust backend provides the resolved directory path to QML via a bridge function.
- Use `try_exists()` for all file/directory existence checks (Android compatibility).

**Android permissions:**
- `RECORD_AUDIO` permission in AndroidManifest.xml.
- Runtime permission request via QML `MicrophonePermission` (Qt 6.5+) before recording.

**Reference recordings:**
- Can be bundled in the app assets or added by the user.
- User can add reference recordings via a file picker dialog.
- Reference recordings are copied into the `chanting-recordings/` directory and tracked in the database.

### 4.5 Markers

Markers are stored per-recording in `chanting_recordings.markers_json` as a JSON array:

```json
[
  {
    "id": "uuid-1",
    "type": "position",
    "label": "Problem spot",
    "position_ms": 15200
  },
  {
    "id": "uuid-2",
    "type": "range",
    "label": "Verse 1",
    "start_ms": 0,
    "end_ms": 8500
  }
]
```

**Position markers:**
- Displayed as a vertical line/dot on the scrubber.
- Clicking jumps playback to that position.

**Range markers:**
- Displayed as a highlighted region on the scrubber.
- Clicking plays only the audio between `start_ms` and `end_ms`.
- When the "Loop" checkbox is unchecked, playback pauses automatically when reaching `end_ms`.
- When the "Loop" checkbox is checked, playback seeks back to `start_ms` and repeats.
- End-detection is implemented via a polling Timer checking `player.position` (~50ms interval).

### 4.6 Rust Bridge & C++ Integration

Following the established window creation pattern:

1. **Rust bridge functions** on `SuttaBridge` (or a new `ChantingBridge`):
   - `get_all_chanting_collections_json()` → JSON of the full tree
   - `get_section_detail_json(section_uid)` → section data with recordings
   - `create_chanting_collection(json)` / `update_*` / `delete_*`
   - `create_chanting_chant(json)` / `update_*` / `delete_*`
   - `create_chanting_section(json)` / `update_*` / `delete_*`
   - `create_chanting_recording(json)` / `delete_chanting_recording(uid)`
   - `update_recording_markers(uid, markers_json)`
   - `get_chanting_recordings_dir()` → resolved absolute path
   - `open_chanting_practice_window()`
   - `open_chanting_review_window(section_uid)`

2. **C++ classes:** `ChantingPracticeWindow` and `ChantingPracticeReviewWindow` in `cpp/`, following the pattern of existing window classes.

3. **WindowManager:** Add `create_chanting_practice_window()` and `create_chanting_review_window(section_uid)` methods.

4. **FFI callbacks** registered in `bridges/src/api.rs`.

5. **Build registration:** Add QML files (`ChantingPracticeWindow.qml`, `ChantingPracticeReviewWindow.qml`, `RecordingPlaybackItem.qml`) and bridge files to `bridges/build.rs`.

6. **QML type definitions:** Create `assets/qml/com/profoundlabs/simsapa/ChantingBridge.qml` (if using a separate bridge) and update `qmldir`.

### 4.7 Menu Integration

In `SuttaSearchWindow.qml`, add a menu item in the Windows menu after `action_topic_index`:

```qml
CMenuItem {
    action: Action {
        id: action_chanting_practice
        text: "&Chanting Practice..."
        onTriggered: {
            SuttaBridge.open_chanting_practice_window()
        }
    }
}
```

## 5. Non-Goals (Out of Scope)

- **Search/filter** in the chanting list (may be added later).
- **Pitch analysis** or automated feedback on chanting quality.
- **Video recording** — audio only.
- **Synced text highlighting** (karaoke-style) during playback.
- **Cloud sync** of recordings or chanting data.
- **Sharing recordings** with other users.
- **Importing chanting data from external formats** (e.g., SuttaCentral) — manual entry only for v1.
- **Translation/transliteration** display alongside Pali text.

## 6. Design Considerations

- Follow the existing app's visual style: use `palette.window`, `ThemeHelper`, `CMenuItem`, and standard Qt Quick Controls.
- The tree view in ChantingPracticeWindow should follow the same pattern as `BooksList` in LibraryWindow.
- The scrubber should be a standard `Slider` with marker overlays rendered as colored rectangles/lines on top.
- Position markers: thin vertical lines (e.g., 2px wide, accent color).
- Range markers: semi-transparent colored rectangles spanning the range.
- Recording indicator: pulsing red dot next to the record button when active.
- Mobile (Android): responsive layout, larger touch targets for buttons and markers, `top_bar_margin` for status bar.

## 7. Technical Considerations

- **Qt Multimedia dependency:** The project already includes Qt Multimedia. Ensure `find_package(Qt6 REQUIRED COMPONENTS Multimedia)` is in CMakeLists.txt and linked to the target.
- **CXX-Qt bridge:** Decide whether to add chanting functions to `SuttaBridge` or create a dedicated `ChantingBridge`. A separate bridge is cleaner but adds more boilerplate. Recommendation: use a separate `ChantingBridge` to keep concerns separated.
- **Database:** All chanting data lives in `appdata.sqlite3` (userdata.sqlite3 is being deprecated). User-added items are distinguished by the `is_user_added` flag.
- **Default data:** Pre-populate chanting collections from the Bhikkhu Manual (already in the app's Library) and Patimokkha chanting during database setup/migration.
- **Diesel migrations:** New tables require a Diesel migration. Add migration files under `backend/migrations/`.
- **File cleanup:** When a recording is deleted from the database, also delete the corresponding audio file from disk.
- **Marker accuracy:** The polling Timer for range marker end-detection should run at ~50ms intervals. This gives acceptable accuracy for chanting practice (±50ms).
- **Android permissions:** Must request `RECORD_AUDIO` at runtime before the first recording attempt. Handle the "denied" case gracefully with a user-facing message.
- **Large file handling:** OGG/Opus at normal quality produces ~1MB/minute. A 5-minute recording is ~5MB. This is acceptable for local storage.

## 8. Success Metrics

- Users can browse, open, and read the Pali text of pre-shipped chanting sections.
- Users can add custom collections, chants, and sections.
- Users can record, play back, and persist multiple recordings per section.
- Users can add and play reference recordings.
- Markers (position and range) work correctly for navigation and A-B loop playback.
- Feature works on both desktop (Linux/macOS) and Android.

## 9. Resolved Questions

1. **Default chanting collections:** Chants from the Bhikkhu Manual (already in the app's Library) and Patimokkha chanting.
2. **Database:** All chanting data (pre-shipped and user-added) stored in `appdata.sqlite3` only. `userdata.sqlite3` is being deprecated.
3. **Maximum recording length:** No limit.
4. **Range marker A-B loop:** Plays once by default, with an option checkbox to enable automatic looping.
5. **Playback interface:** The audio playback UI is a **reusable QML component** (`RecordingPlaybackItem.qml`) so that the review window can display multiple playback items simultaneously — e.g., a reference recording and one or more user recordings open at the same time for easy comparison.

## 10. Open Questions

(None remaining.)
