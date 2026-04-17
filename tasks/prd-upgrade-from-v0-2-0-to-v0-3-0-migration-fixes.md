# PRD: Migration Fixes for the v0.2.0 → v0.3.0 Database Upgrade

## 1. Introduction / Overview

Manual testing of the v0.2.0 → v0.3.0 upgrade flow (the post-`userdata.sqlite3`-deprecation upgrade) surfaced several defects in the export → re-download → re-import procedure. This PRD captures those defects and the fixes that must ship before v0.3.0 is released.

The defects are concentrated in three areas:

1. The chanting export aborts with a foreign-key violation when a v0.2.0 user has *user-added recordings* attached to *seeded* sections (or has seeded reference recordings whose `is_user_added` flag was never correctly set to `false` in the v0.2.0 bootstrap). The first failing recording short-circuits the rest of the export.
2. The chanting recordings folder (`chanting-recordings/`) is never copied into `import-me/` because the audio-copy loop runs after the failing sqlite-build step.
3. The fulltext search `index/` directory survives the upgrade. The new app then runs against fresh `appdata.sqlite3` content but a stale Tantivy index whose doc IDs reference the previous DB.

In addition, the export error path is silently swallowed — the user sees no warning, the marker files are written anyway, and on restart the old DB is wiped even though the user's data was not preserved.

## 2. Goals

1. Carry every user-added chanting recording across the upgrade, regardless of whether its parent section/chant/collection is seeded or user-added.
2. Make the chanting export resilient: a failure on one row must not lose the rest of the data, and the audio files on disk must always be preserved in `import-me/` even if the sqlite step fails.
3. The re-import side intelligently rejoins each imported recording to the correct foreign-key target in the freshly-bootstrapped DB. User-created recordings must never end up in the new DB without a parent section to display them under.
4. Force a fresh fulltext index after the upgrade by deleting the stale `index/` directory in the same step that deletes the old database files.
5. Surface export failures to the user as a blocking dialog with two choices: cancel the upgrade, or continue anyway (knowing some data may be lost).

## 3. User Stories

- As a v0.2.0 alpha user with user-recorded audio against a default chant (e.g. a recording of my own *Itipiso* over the seeded `chanting-sec-itipiso` section), when I upgrade to v0.3.0 my recording survives and is shown under the seeded *Itipiso* section in the new app, with audio playable.
- As a v0.2.0 alpha user whose seeded reference recording row was incorrectly flagged `is_user_added = true`, the upgrade does not bail out on that row — it ignores the duplicate-of-seeded row gracefully and still carries my real user data across.
- As a developer reviewing logs from a failed upgrade, I can see exactly which rows failed and why, and the user's audio files are still in `import-me/chanting-recordings/` for manual recovery.
- As a Simsapa user, when I run a search after the upgrade, the search index reflects the freshly downloaded and extracted index and not the old index from the previous DB.
- As a Simsapa user, if the export step fails, I see a clear dialog telling me which part of the export didn't work, and can choose to either back out (to use my old DB and chanting as they are) or continue with the upgrade (and accept potential data loss).

## 4. Functional Requirements

### 4.1 Chanting export — include seeded ancestors and harden against per-row failure

1. `export_user_chanting_data()` in `backend/src/app_data.rs:2305` must, after loading user-added recordings, also load every section referenced by those recordings (whether `is_user_added` is true or false) and every chant + collection ancestor of those sections. The exported `appdata-chanting.sqlite3` therefore always contains a complete FK-consistent subgraph for every recording.
2. Sections/chants/collections that come into the export *only* because they are ancestors of a user recording must be written to the export sqlite **with their original `is_user_added` value preserved**. Do not flip seeded rows to `true` — the import step uses this flag to decide whether to insert or skip.
3. `create_chanting_sqlite()` in `backend/src/db/chanting_export.rs:20` must execute `PRAGMA foreign_keys = OFF;` on the export connection before the insert loops. The export file is a transport format; foreign-key integrity is re-validated by the importer against the live DB.
4. The four insert loops in `create_chanting_sqlite()` must not abort the whole function on a single insert failure. Replace `?` with per-row `match`: log a `warn` with the row's uid and the error, increment a counter, and continue. After all loops, log a one-line summary of how many rows succeeded and how many failed per table. Return `Ok(())` if the sqlite file was written at all; only return `Err` for I/O failures (failure to create the file, run migrations, etc.).
5. `export_user_chanting_data()` must copy the `chanting-recordings/` audio files into `import-me/chanting-recordings/` **before** calling `create_chanting_sqlite()`. If the sqlite build later fails or partially fails, the user's audio is still preserved and recoverable.
6. The audio-copy step copies every file referenced by any user-added recording (the existing per-recording loop). Missing source files continue to log `warn` and skip — that behaviour stays.

Keep in mind successive upgrade attempts by the user. If they cancel the upgrade, and keep using the app for a while, then on a successive upgrade attempt, any previous import-me folder should be cleared to not conflict with the user data export procedure.

### 4.2 Chanting re-import — intelligent FK reconciliation

7. `import_user_chanting_data()` in `backend/src/app_data.rs:2409` must change its insertion strategy:
    - For each **collection** in the import file: if a collection with the same `uid` already exists in the live appdata DB, skip it (it is a seeded ancestor that the new bundle already shipped). Do not error.
    - For each **chant** in the import file: same skip-if-exists rule.
    - For each **section** in the import file: same skip-if-exists rule.
    - For each **recording** in the import file:
        - If a recording with the same `uid` already exists, skip it (handles the `chanting-rec-itipiso-reference` duplicate-of-seeded case where the old DB had the seeded recording flagged `is_user_added = true`).
        - Otherwise, resolve the parent `section_uid`: (a) if the section_uid exists in the live DB, insert the recording as-is; (b) if the section_uid does not exist, log a `warn` listing the recording uid and orphan section_uid, and create a new section using the metadata from the exported db. 
        - There must be no orphaned recordings, create parent items for them if needed.
8. The skip-if-exists check must use the table's `uid` column (which is unique). Use a single existence query per row rather than a `INSERT OR IGNORE` so we can log "skipped existing seeded row" vs. "inserted user row" distinctly.
9. The audio-file copy step in `import_user_chanting_data()` is unchanged in behaviour: it copies every file present under `import-me/chanting-recordings/` into the live `chanting-recordings/` directory, overwriting on conflict. Files associated with a recording should not be orphaned, we create parent items for them in the db if needed.

Document the upgrade and migration logic and flow with relevant code locations by writing a markdown file in the docs/ folder, and referencing it in CLAUDE.md

### 4.3 Index cleanup on upgrade

10. `check_delete_files_for_upgrade()` in `backend/src/lib.rs:686` must, when the marker file is present, also delete the entire `paths.index_dir` directory tree using `std::fs::remove_dir_all` after a successful `try_exists() == Ok(true)` check. The new asset bundle ships a fresh index, so removing the stale one is sufficient — no explicit rebuild call is required at startup.
11. The doc comment on `check_delete_files_for_upgrade()` must be updated to list `index/` alongside the database files it removes.

The index folder must only be removed after the user data export succeeded. If it is removed earlier, and user cancels the upgrade, the app is left without an index.

### 4.4 Export-failure user dialog

12. `prepare_for_database_upgrade()` in `bridges/src/sutta_bridge.rs:3338` must change from "log-and-continue on export error" to a two-step flow:
    - Step A: call `export_user_data_to_assets()` and capture error messages grouped by data categories (e.g. chantings, bookmarks, etc).
    - Step B: only if export succeeded, write `delete_files_for_upgrade.txt` and `auto_start_download.txt`.
    - If export failed, do **not** write the marker files. Instead, emit a new Qt signal (e.g. `export_failed(QString reason)`) to the QML layer and return without queuing the upgrade.
13. Define a new `#[qsignal]` `export_failed(reason: QString)` on `SuttaBridge`, and a matching stub in `assets/qml/com/profoundlabs/simsapa/SuttaBridge.qml`.
14. Both call sites that today invoke `SuttaBridge.prepare_for_database_upgrade()` (`assets/qml/UpdateNotificationDialog.qml:472`, `assets/qml/UpdateNotificationDialog.qml:557`, and `assets/qml/DatabaseValidationDialog.qml:157`) must subscribe to the new `export_failed` signal and present a confirmation dialog with the failure reason and four buttons:
    - **Cancel Upgrade** — close the dialog, do nothing further. The old DB and `import-me/` folder (whatever partial state was written) remain on disk; the user is left on the current version.
    - **Copy Error Message** — Copies the path to the `import-me/` folder to the clipboard.
    - **Copy Exported Path** — Copies the path to the `import-me/` folder to the clipboard.
    - **Continue Anyway** — call a new bridge method `force_database_upgrade()` that writes the two marker files unconditionally, then proceeds to the existing "closing message" dialog telling the user to restart.
15. `force_database_upgrade()` is a separate `#[qinvokable]` so the "force" path is explicit in code review and the success path stays narrow. It does **not** re-attempt the export and it does **not** delete `import-me/` — a partial export may still contain valid data for unaffected categories (bookmarks, books, etc.) that must be imported on the next run.
16. The dialog text must include the underlying error string (truncated if very long) so the user can report it. Recommended copy:

    > **Errors during user data export**
    >
    > Exporting user data before database upgrade reported errors:
    >
    > `<error categories and messages as text in a scrollable TextArea>`
    >
    > [Cancel Upgrade] [Copy Error Message] [Copy Exported Path] [Continue Anyway]

17. A successful export still proceeds straight to the existing "closing message" dialog — no extra confirmation is added on the happy path.

### 4.5 Logging

18. Every new branch added by this PRD must emit a `tracing` event at `info` (success paths and counts) or `warn`/`error` (failures). In particular:
    - The number of user-added recordings, the number of seeded ancestor sections/chants/collections pulled into the export, and the number of insert failures per table.
    - The import-side count of "skipped existing seeded row", "inserted user row", and "created parent section for orphan recording" per table.
    - The deletion of `index/` during `check_delete_files_for_upgrade()`.
    - The export-failure path in `prepare_for_database_upgrade()`, error message strings grouped by export data category.

## 5. Non-Goals (Out of Scope)

- Bumping `INDEX_VERSION` or otherwise changing the in-app index-staleness logic. The fix here is purely "delete the stale index during upgrade tear-down."
- A migration that retroactively flips `is_user_added` on seeded rows in v0.2.0 databases. The skip-if-exists check on the import side handles this case without touching the source DB.
- We don't need a "recover orphan recordings" UI because if original parent items can't be found, new parent items are created for recordings to avoid orphans.
- No need for atomic rollback of `import-me/` on export failure. If export fails partway, the dialog tells the user to cancel; the partial folder is left in place for inspection. Cleanup happens naturally on the next successful upgrade or manual deletion.
- Changes to the bookmarks or books export (those paths did not error in the manual test).
- Automated integration tests for the upgrade round-trip — manual testing as in the prior PRD.

## 6. Design Considerations

- The chanting import was the visible failure point, but the underlying *shape* of the bug — exporting a child without its FK ancestors and abort-on-first-error inside a tight loop — could repeat for any future user-data table that gains a parent/child relationship. The "include ancestors + per-row resilience" pattern from §4.1/§4.2 is the template.
- The `Continue Anyway` button is deliberately not a default action. The dialog should make `Cancel Upgrade` the default focus/ENTER target so an accidental keypress preserves data.
- The `index/` deletion is grouped with the existing DB-deletion list rather than introduced as a new marker file or new code path. Same trigger, same timing, fewer moving parts.

## 7. Technical Considerations

- `create_chanting_sqlite` uses Diesel models that derive `Insertable` against `appdata_schema`. Running `PRAGMA foreign_keys = OFF` on the connection before inserts is straightforward via `diesel::sql_query("PRAGMA foreign_keys = OFF").execute(&mut conn)`.
- The "include ancestors of user recordings" query can be a single `IN (uid, uid, …)` filter on the existing schema; no new index is needed.
- The skip-if-exists check on import is one `SELECT 1 FROM <table> WHERE uid = ? LIMIT 1` per row. With 3 collections / a few chants / dozens of sections / a few recordings per typical user, this is negligible.
- The Qt signal must be declared in the `cxx_qt::bridge` block alongside the existing `#[qsignal]` entries and connected via `Connections { target: SuttaBridge }` in QML, matching the pattern used by `app_settings_reset` (added in the previous PRD).
- The `index_dir` removal already has `try_exists()` precedent in the same file (`backend/src/lib.rs`) — follow the CLAUDE.md guidance and never use `.exists()`.
- `DatabaseValidationDialog.qml` already has dialog-styling utilities for confirmation prompts; reuse those for consistency rather than inventing a new dialog component.

## 8. Success Metrics

1. A simulated v0.2.0 upgrade where a user has one user-added recording attached to the seeded `chanting-sec-itipiso` section completes without error, the recording is visible under the seeded section in the new app, and its audio plays.
2. A simulated v0.2.0 upgrade where the seeded `chanting-rec-itipiso-reference` row is flagged `is_user_added = true` completes without error: the duplicate-of-seeded row is skipped on import, and other user data is carried across.
3. After any export run (success or failure), `import-me/chanting-recordings/` contains every audio file referenced by a user-added recording in the source DB.
4. After a successful upgrade, the `index/` directory does not exist between the old-DB-deletion step and the new bundle's extraction. After extraction, the index reflects only the fresh DB.
5. Triggering an export failure (e.g. by simulating a write error in `create_chanting_sqlite`) surfaces a dialog with `Cancel Upgrade` / `Continue Anyway`. Choosing Cancel leaves the old DB intact and no marker files written; choosing Continue Anyway writes the markers and proceeds.
6. `make build -B` succeeds and `cd backend && cargo test` passes.

When running the tests, don't worry about old failing tests that are not concerned with the migration and upgrade.

## 9. Implementation Stages

Each stage is small enough to build + test before moving on. After every top-level stage: `make build -B` and `cd backend && cargo test`. Skip `make qml-test` unless explicitly asked.

**Stage 1 — Chanting export: include seeded ancestors and harden inserts.**
- Update `export_user_chanting_data()` to additionally collect (a) the set of section_uids referenced by user-added recordings, (b) their sections (preserving original `is_user_added`), (c) their parent chants and collections.
- Reorder the audio-copy step to run before `create_chanting_sqlite()`.
- Update `create_chanting_sqlite()` to run `PRAGMA foreign_keys = OFF` on its connection and to convert the four insert loops from abort-on-first-error to per-row warn-and-continue with summary counters.
- Verify (manually): an export from a v0.2.0-style DB with a user recording on a seeded section produces an `appdata-chanting.sqlite3` containing the user recording **and** its seeded ancestor section/chant/collection, plus the audio file in `import-me/chanting-recordings/`.

**Stage 2 — Chanting import: skip-if-exists + orphan tolerance.**
- In `import_user_chanting_data()`, replace the unconditional inserts for collections/chants/sections with skip-if-exists checks (by `uid`).
- For recordings, add the same skip-if-exists check, then check that the recording's `section_uid` exists in the live DB before inserting; if not, create the missing section (and its chant/collection ancestors if also missing) from the exported metadata, log a warn, and then insert the recording. No recording is dropped.
- Emit per-table counters: inserted, skipped-existing, created-parent-for-orphan.
- Verify (manually): re-import of a Stage-1 export cleanly produces the expected live-DB state with no FK errors and no duplicate seeded rows.

**Stage 3 — Index cleanup on upgrade.**
- Add `paths.index_dir` to the deletion list in `check_delete_files_for_upgrade()` (use `remove_dir_all` guarded by `try_exists() == Ok(true)`). Update the doc comment.
- Verify (manually): after the marker file fires on next startup, the `index/` directory is gone before the asset download begins.

**Stage 4 — Export-failure dialog plumbing.**
- Add `#[qsignal] export_failed(reason: QString)` to `SuttaBridge` and the matching stub in `SuttaBridge.qml`.
- Add `#[qinvokable] force_database_upgrade()` that writes the two marker files unconditionally and emits no signal (the QML side already navigates to "closing").
- Change `prepare_for_database_upgrade()` so it (a) does **not** write the marker files on export error, and (b) emits `export_failed` with the underlying error string instead.
- Verify: the bridge compiles and the QML stub loads cleanly under qmllint.

**Stage 5 — Export-failure dialog UI.**
- Add a new dialog (or new section in an existing dialog component) with the copy from §4.4 #16, default focus on `Cancel Upgrade`.
- Wire the three current call sites of `SuttaBridge.prepare_for_database_upgrade()` (`UpdateNotificationDialog.qml:472`, `UpdateNotificationDialog.qml:557`, `DatabaseValidationDialog.qml:157`) to subscribe to `export_failed` and show the new dialog.
- The `Continue Anyway` button calls `SuttaBridge.force_database_upgrade()` and then transitions to the existing "closing" dialog state.
- Add the new QML file (if any) to `bridges/build.rs::qml_files` per CLAUDE.md.
- Verify (manually): simulate a failure (e.g. temporarily make `create_chanting_sqlite` return an `Err` for I/O reasons before the per-row loop), trigger the upgrade — confirm the dialog appears, Cancel preserves state, Continue Anyway proceeds.

**Stage 6 — Documentation.**
- Update `PROJECT_MAP.md` "Database Upgrade Flow" to mention (a) seeded-ancestor inclusion in chanting export, (b) skip-if-exists import strategy, (c) `index/` deletion in `check_delete_files_for_upgrade`, (d) the `export_failed` signal and the user-facing dialog.
- If the chanting export/import section in `tasks/archive/prd-chanting-export-import.md` is referenced elsewhere, leave the archive alone but note the new behaviour in `PROJECT_MAP.md`.

## 10. Open Questions (Resolved)

1. **Should `Continue Anyway` delete `import-me/` before writing the marker files?** **No.** Export failure may only affect one data category (e.g. chantings) while the rest of `import-me/` still holds valid user data (bookmarks, books, etc.) that must still be imported. `force_database_upgrade()` must leave `import-me/` untouched — it only writes the two marker files. Successive upgrade attempts still clear `import-me/` at the start of the export step per §4.1, so stale artifacts from a cancelled attempt are not a concern on the happy path.
2. **Orphan recordings.** Not applicable: per §4.2 #7 the importer creates a parent section (using the exported metadata) whenever a recording's `section_uid` is missing in the live DB. No recording is ever skipped as an orphan, so no sidecar file or recovery tool is needed. The "skipped-orphan" counter referenced in §4.5 / Stage 2 should be renamed to "created-parent-for-orphan" (or equivalent) to reflect the actual behaviour.
3. **`index/` deletion timing.** The `index/` directory must be removed only on the upgrade-proceed path, never on cancel. Because deletion happens inside `check_delete_files_for_upgrade()` which is gated on the `delete_files_for_upgrade.txt` marker, and because the marker is only written by the happy path (§4.4 #12) or by `force_database_upgrade()` (§4.4 #15), a cancelled upgrade leaves the marker absent and the index intact — the existing gate already gives us the required behaviour. Stage 3 must still audit that no other code path writes the marker or removes `index/` unconditionally.

## 11. Post-Implementation Review — Issues and Planned Fixes

After Stages 1–5 were implemented, a code review surfaced the following defects. They must be fixed before release; see §12 tasks 6.0.

### 11.1 Duplicate dialog reactions across sibling dialogs

Both `UpdateNotificationDialog` and `DatabaseValidationDialog` are instantiated as siblings inside `assets/qml/SuttaSearchWindow.qml` (around lines 1796 and 1806). Each defines its own global `Connections { target: SuttaBridge }` block handling `onExportFailed` and `onExportSucceeded`. Because both live in the QML tree at the same time, **every** emission of `exportFailed` / `exportSucceeded` fires **both** handlers — regardless of which dialog initiated the upgrade. Symptoms:

- Triggering the upgrade from `DatabaseValidationDialog` ("Remove All & Re-Download") also forces `UpdateNotificationDialog` to `show() + raise() + requestActivate()` and display its own failure frame.
- On success, the DB-validation dialog opens `remove_all_success_dialog` **and** the update dialog transitions to the `closing` state.

**Fix.** Each dialog tracks whether it initiated the current upgrade and only reacts to the bridge signals when it did.

- Add a boolean property `upgrade_initiated_here` (default `false`) to each dialog.
- Set it to `true` immediately before calling `SuttaBridge.prepare_for_database_upgrade()` at every call site.
- In the `Connections` handlers for `onExportFailed` / `onExportSucceeded`, return early when `!upgrade_initiated_here`.
- Reset the flag to `false` once the dialog has acted on the signal (opened the failure dialog, transitioned to `closing`, etc.), or on user cancel.
- Add a QML comment at the top of each `Connections` block explaining why the guard is required ("both dialogs are siblings in SuttaSearchWindow and both receive the signal; only the initiator should react").

### 11.2 UI dead-time between click and async export signal

In `UpdateNotificationDialog.qml` (the Yes / Download Now buttons at lines ~477 and ~561) the previous eager `root.dialog_type = "closing"` transition was removed so that the dialog waits for `exportSucceeded` / `exportFailed`. During that wait (several seconds for chanting export) nothing visible changes and the user may double-click.

**Fix.**

- While the export is in progress, disable the button and change its label to "Exporting user data…" (or similar). Apply the same pattern to the `DatabaseValidationDialog` "Remove All and Re-Download" button.
- Use a local QML property `export_in_progress` (default `false`) flipped to `true` on click and flipped back to `false` when either `onExportFailed` or `onExportSucceeded` fires (or on dialog close).
- Add a QML comment explaining the reason: "we wait for `exportSucceeded`/`exportFailed` from the bridge before transitioning; disable the button and change its label so the user knows work is in progress and cannot re-trigger the export."

### 11.3 Silent marker-file I/O failure falsely reports success

`write_upgrade_marker_files()` in `bridges/src/sutta_bridge.rs` logs marker-write failures but does not return them. `prepare_for_database_upgrade()` then unconditionally emits `export_succeeded` after calling it, so the QML layer proceeds to the "Restart Required" closing dialog even though the marker files are missing — on restart the old DB is not deleted and the upgrade silently does not happen.

**Fix.**

- Change `write_upgrade_marker_files()` to return `Result<(), Vec<(String, String)>>` using the same category-error shape as `export_user_data_to_assets()`. Categories: `marker_delete_files`, `marker_auto_start_download`. Each failed write pushes one entry.
- `prepare_for_database_upgrade()`:
  - On successful export, call `write_upgrade_marker_files()`. If **that** fails, emit `export_failed` with the formatted marker errors (not `export_succeeded`). If it succeeds, emit `export_succeeded`.
  - On failed export, behaviour unchanged (emit `export_failed` without attempting marker writes).
- `force_database_upgrade()`: if `write_upgrade_marker_files()` returns an error, emit `export_failed` with the marker errors so the user sees the failure instead of restarting into a no-op upgrade.
- Add a Rust doc comment on `write_upgrade_marker_files` explaining why propagating I/O failure matters ("a silently-missing marker file produces a silently-failed upgrade on restart").

### 11.4 Orphan-repair still drops recordings when ancestors are missing from the exported DB

In `import_user_chanting_data()` the orphan-repair branch sets `orphan_repair_ok = false` and then `continue`s when the *exported* DB lacks the needed section / chant / collection (`warn("Cannot repair orphan recording …")`). This contradicts PRD §10.2 which mandates "no recording is ever skipped as an orphan."

**Fix.** Never drop a recording. When an ancestor is missing from both the live DB and the exported DB, synthesise a placeholder using deterministic values derived from the recording / section uid so repeated imports converge:

- Synthetic collection: `uid = format!("col-orphan-recovery")`, `title = "Orphan Recovery"`, `language = "pali"`, `is_user_added = true`, `sort_index = 9999`.
- Synthetic chant: `uid = format!("chant-orphan-recovery")`, `collection_uid` pointing at the synthetic or real collection, `title = "Orphan Recovery"`, `is_user_added = true`.
- Synthetic section: `uid = <the original recording's section_uid>` so the recording's FK still resolves, `chant_uid` pointing at the (real or synthetic) chant, `title = format!("Recovered section {}", section_uid)`, `content_pali = ""`, `is_user_added = true`.
- Before synthesising, check the live DB for the synthetic uids (skip-if-exists) so subsequent imports reuse the same placeholders.

Count these as `created_parent_for_orphan` and log at `warn` listing the recording uid, missing ancestor uid(s), and the synthetic placeholder(s) used. Remove the `continue` on `!orphan_repair_ok`; every recording must proceed to the insert step.

### 11.5 User-added chants/sections without a user collection get silently dropped on export

`export_user_chanting_data()` early-returns on `user_collections.is_empty() && user_recordings.is_empty()`. A user who has user-added chants or sections (e.g. added under a seeded collection) but zero user-added collections **and** zero user-added recordings would see their data silently dropped.

More generally, the same orphan-tolerance rule from §11.4 must apply on the **export** side: every user-added row must carry its full ancestor chain into the export sqlite, creating placeholders where the live DB lacks ancestors.

**Fix.**

- Relax the early-return condition to `user_collections.is_empty() && user_chants.is_empty() && user_sections.is_empty() && user_recordings.is_empty()`.
- Extend the seeded-ancestor inclusion logic (§4.1 #1) so that the ancestor load-and-merge is driven not just by `user_recordings.section_uid` but also by:
  - `user_sections.chant_uid` (to pull ancestor chants)
  - `user_chants.collection_uid` (to pull ancestor collections)
- When a live-DB lookup for an ancestor uid returns no row (rare, but possible on a damaged DB), synthesise a placeholder using the same deterministic rule as §11.4 so the exported sqlite always has a complete FK subgraph. Log at `warn`.
- Apply the same principle on the import side (§11.4): any level of missing ancestor is synthesised so no user-added row is ever dropped.

### 11.6 `force_database_upgrade` does not log the errors the user chose to bypass

Today `force_database_upgrade()` logs only "writing upgrade marker files after user opted to continue past export failure". The category errors emitted by `prepare_for_database_upgrade()` are not re-logged, so a post-mortem from a user bug report cannot recover them.

**Fix.**

- Hold the most recent export-failure reason string on the `SuttaBridge` struct (e.g. `last_export_failure: Mutex<Option<String>>`), set it when `prepare_for_database_upgrade()` emits `export_failed`, and clear it on successful `prepare_for_database_upgrade()`.
- `force_database_upgrade()` logs the stored reason at `error` level before writing the markers, prefixed with "force_database_upgrade(): user is bypassing the following export errors:". This way the bypass decision and its motivation appear together in a single log file.

## 12. Post-Implementation Stages

**Stage 6 — Apply fixes from §11.** Each fix must be implemented, built, and tested before moving on; see §13 tasks 6.0 in the task list file.
