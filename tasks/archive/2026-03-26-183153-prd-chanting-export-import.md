# PRD: Chanting Practice Export / Import

## 1. Introduction / Overview

The Chanting Practice feature already supports creating collections, chants, sections, and audio recordings. This PRD adds **Export** and **Import** functionality so users can:

- Back up their chanting data (both user-created and pre-shipped selections) as a single `.zip` file
- Share chanting collections with other users
- Transfer data between devices
- Preserve all user chanting data and recordings automatically during database upgrades

The export uses a **selection mode** in the existing tree list UI, allowing users to pick specific collections, chants, or sections via checkboxes. The import reads a `.zip` file and inserts everything as new user-added items.

## 2. Goals

1. Allow users to export selected chanting data (database records + audio files) as a single `.zip` archive.
2. Allow users to import a `.zip` archive to restore or add chanting data.
3. Automatically preserve all user chanting data and recordings during database upgrades.
4. Work on both desktop and Android platforms.

## 3. User Stories

- **As a practitioner**, I want to export my chanting collections so I can back them up or share them with a friend.
- **As a practitioner**, I want to import a chanting `.zip` file from another device so I can continue my practice.
- **As a user upgrading the app database**, I want my user-created chanting data and all my recordings (including those on pre-shipped sections) to be automatically preserved without manual intervention.
- **As a mobile user**, I want to export and import chanting data on my Android device using the system file picker.

## 4. Functional Requirements

### 4.1 Export — Selection Mode UI

1. An **"Export"** button is added to the `ChantingPracticeWindow.qml` toolbar.
2. Clicking "Export" the first time enters **selection mode**:
   - A dialog appears telling the user: *"Select the items you want to export, then click the Export button again."*
   - The Export button changes to **green** background with text **"Export Selected"**.
   - Checkboxes appear next to every item in `ChantingTreeList.qml` (collections, chants, sections).
3. **Checkbox selection logic** (hierarchical):
   - Clicking a **collection** checkbox selects/unselects **all** its chants and all their sections.
   - Clicking a **chant** checkbox selects/unselects **all** its sections.
   - Clicking a **section** checkbox selects/unselects **only** that section.
   - **Upward auto-selection**: When a section is checked, its parent chant and grandparent collection are automatically checked too. This ensures exported data always has the full hierarchy (section → chant → collection).
   - **Upward auto-deselection**: When all children of a parent are unchecked, the parent is automatically unchecked. Specifically: if all sections of a chant are unchecked, uncheck the chant; if all chants of a collection are unchecked, uncheck the collection.
4. Clicking the green Export button a **second time** finalizes the export:
   - A **save-file dialog** opens with an auto-generated filename including date and time (e.g., `chanting-export-2026-03-26T143022.zip`).
   - The selected data is packaged into the `.zip` (see §4.3).
   - Selection mode is exited and the UI returns to normal.
5. If no items are selected when the user clicks the green button, show a warning dialog: *"No items selected for export."*
6. There should be a way to **cancel** selection mode (e.g., a "Cancel Export" button or pressing Escape) without exporting.

### 4.2 Import

7. An **"Import"** button is added to the `ChantingPracticeWindow.qml` toolbar.
8. Clicking "Import" opens a **file browser** (native file dialog on desktop, system file picker on Android) filtered to `.zip` files.
9. After selecting a file, the backend:
   - Extracts the `.zip` to a temporary directory.
   - Validates the archive contains `appdata-chanting.sqlite3` and optionally `chanting-recordings/`.
   - Reads all chanting data from the embedded SQLite database.
   - **Generates new UIDs** for every imported record (collections, chants, sections, recordings) to avoid conflicts. Updates all foreign key references (`collection_uid`, `chant_uid`, `section_uid`) to match the new UIDs.
   - Sets `is_user_added = true` on all imported collections, chants, and sections.
   - Inserts the records into the app's live `appdata` database.
   - Copies audio files from the archive's `chanting-recordings/` folder into the app's `chanting-recordings/` directory, renaming them to match the new recording UIDs/filenames.
   - Cleans up the temporary directory.
10. During import, show a modal dialog with the text **"Importing..."** and a busy spinner. The dialog closes automatically when the import completes.
11. After import, the tree list is refreshed to show the newly imported data.
12. If the archive is invalid or corrupted, show an error dialog with a descriptive message.

### 4.3 Export Archive Format

12. The export `.zip` archive has this structure:
    ```
    chanting-export-YYYY-MM-DDTHHMMSS.zip
    ├── appdata-chanting.sqlite3
    └── chanting-recordings/
        ├── sec-abc123_1234567.ogg
        └── sec-def456_9876543.ogg
    ```
13. `appdata-chanting.sqlite3` is a standalone SQLite3 database using the same schema as the app's `appdata` chanting tables (`chanting_collections`, `chanting_chants`, `chanting_sections`, `chanting_recordings`). It contains **only** the selected items and their recordings.
14. `chanting-recordings/` contains the audio files referenced by the recording rows in the database. Only recordings belonging to the selected sections are included.
15. All recordings associated with selected sections are **automatically included** — the user does not pick individual recordings.

### 4.4 Database Upgrade — Chanting Data Preservation

16. Extend `export_user_data_to_assets()` to also:
    - Query all rows where `is_user_added = true` from `chanting_collections`, `chanting_chants`, `chanting_sections`.
    - Query **all** recordings from `chanting_recordings` (user recordings may exist for both user-added and pre-shipped sections).
    - Write these rows into an `appdata-chanting.sqlite3` file inside the `import-me/` folder.
    - Copy the **entire** `chanting-recordings/` directory into `import-me/`.
17. Extend `import_user_data_from_assets()` to also:
    - Check for `appdata-chanting.sqlite3` in the `import-me/` folder.
    - Read all chanting data from it.
    - Insert user-added collections, chants, and sections into the new database (preserving original UIDs since this is a fresh database after upgrade).
    - Insert all recordings, re-associating them with sections by UID.
    - Copy audio files from `import-me/chanting-recordings/` back to the app's `chanting-recordings/` directory.
18. During upgrade import, UIDs are **preserved** (not regenerated) because the target database is freshly created and has no conflicts. This differs from the user-facing import (§4.2) which always generates new UIDs.
19. For recordings associated with **non-user-added sections**: during export, also export the `section_uid` reference. During import, if the section already exists in the new database (because it was re-shipped), simply insert the recording rows pointing to that existing section. If the section no longer exists in the new database, the recording is skipped (the section was removed from the pre-shipped data).

## 5. Non-Goals (Out of Scope)

- Merging or syncing data between two devices in real-time.
- Exporting/importing other app data (suttas, bookmarks, etc.) via this feature — those are handled separately.
- Selective recording export (user always gets all recordings for selected sections).
- Cloud backup or remote storage integration.
- Conflict resolution UI during import — imported items always get new UIDs.

## 6. Design Considerations

### 6.1 Selection Mode UI

- The Export button in the toolbar toggles between two states:
  - **Normal state**: Default button style, text "Export".
  - **Selection state**: Green background (`#4CAF50` or similar), text "Click to Export Selected".
- A "Cancel" button appears next to the green Export button during selection mode.
- Checkboxes in `ChantingTreeList.qml` appear to the **left** of the colored indicator dots, only visible during selection mode.
- Checkbox state is tracked via a JS object/map keyed by UID (e.g., `property var checked_items: ({})`).
- The tri-state logic (all children checked, some checked, none checked) can optionally show a **partial check** indicator on parent items when only some children are selected. This is optional — a simple filled checkbox is acceptable when at least one child is selected.

### 6.2 File Dialogs

- **Desktop**: Use Qt `FileDialog` from `QtQuick.Dialogs`.
- **Android**: Use Qt `FileDialog` which maps to the system's SAF (Storage Access Framework) file picker. Ensure the dialog filter is set to `.zip` files.

### 6.3 Mobile Layout

- On mobile, the Export and Import buttons may need to be placed in a secondary row or overflow menu if toolbar space is limited.

## 7. Technical Considerations

### 7.1 Rust Backend — New Bridge Functions

Add to `sutta_bridge.rs`:

```
export_chanting_data(json_selected_uids: &QString, dest_path: &QString) -> QString
    // json_selected_uids: JSON object { collections: [...], chants: [...], sections: [...] }
    // dest_path: path for the .zip file
    // Returns: { "ok": true } or { "error": "..." }

import_chanting_data(zip_path: &QString) -> QString
    // zip_path: path to the .zip file
    // Returns: { "ok": true, "imported": { "collections": N, "chants": N, "sections": N, "recordings": N } }
    //     or: { "error": "..." }
```

### 7.2 Rust Backend — Export Logic

In `app_data.rs` or a new `chanting_export.rs` module:

1. Accept the list of selected section UIDs (derive chant and collection UIDs from the hierarchy).
2. Query the selected rows from the live database.
3. Create a temporary directory.
4. Create `appdata-chanting.sqlite3` using the same Diesel migration SQL to set up the schema.
5. Insert the selected rows.
6. Copy referenced audio files to `chanting-recordings/` in the temp directory.
7. Create the `.zip` archive from the temp directory using the `zip` crate.
8. Move the `.zip` to the user's chosen destination path.
9. Clean up the temp directory.

### 7.3 Rust Backend — Import Logic

1. Extract `.zip` to a temp directory.
2. Open the embedded `appdata-chanting.sqlite3` (read-only).
3. Read all rows from the four chanting tables.
4. Generate new UIDs for every record; build a mapping of `old_uid → new_uid`.
5. Update all foreign key references using the mapping.
6. Update `file_name` fields in recordings to reflect new UIDs if the filename contains the old UID.
7. Set `is_user_added = true` on all collection/chant/section records.
8. Insert into the live database.
9. Copy and rename audio files to the app's `chanting-recordings/` directory.
10. Clean up.

### 7.4 Zip Crate

Use the `zip` crate (already commonly used in Rust) for creating and reading `.zip` archives. Add to `backend/Cargo.toml`:

```toml
zip = "2"
```

### 7.5 QML Bridge Type Definitions

Add the new bridge functions to `assets/qml/com/profoundlabs/simsapa/SuttaBridge.qml` for `qmllint` compatibility:

```qml
function export_chanting_data(json_selected_uids: string, dest_path: string): string {
    return '{"ok": true}';
}

function import_chanting_data(zip_path: string): string {
    return '{"ok": true}';
}
```

### 7.6 Upgrade Path Integration

The `export_user_data_to_assets()` and `import_user_data_from_assets()` functions in `app_data.rs` already handle settings and user books. The chanting export/import logic should be implemented as separate helper methods (e.g., `export_user_chanting_data(import_dir)` and `import_user_chanting_data(import_dir)`) called from within the existing export/import flow.

### 7.7 Build Registration

No new QML components are needed — changes are to existing files. No new Rust bridge files — functions are added to the existing `sutta_bridge.rs`.

## 8. Success Metrics

- Users can export selected chanting data and re-import it on the same or different device without data loss.
- Database upgrades preserve all user-added chanting content and all user recordings (including those on pre-shipped sections).
- Export/import works on both desktop (Linux, macOS, Windows) and Android.
- No orphaned recordings after import (every imported recording file matches a database row).

## 9. Resolved Questions

1. **Maximum archive size**: No limit or warning — let it proceed.
2. **Import progress indicator**: A simple modal dialog showing "Importing..." with a busy spinner. No progress bar needed.
3. **Export filename customization**: The save dialog allows renaming but the `.zip` extension must be enforced (append if missing).
4. **Android SAF limitations**: Not a concern — the user will select an appropriate folder.
