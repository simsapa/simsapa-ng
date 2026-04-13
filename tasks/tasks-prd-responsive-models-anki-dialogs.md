# Tasks: Responsive Layouts for ModelsDialog and AnkiExportDialog

## Relevant Files

- `assets/qml/ModelsDialog.qml` — AI Models settings dialog; currently uses a horizontal `SplitView` with provider list and details. Needs `is_wide` / `is_tall` props and a responsive `SplitView`.
- `assets/qml/AnkiExportDialog.qml` — Anki template/preview dialog; currently a horizontal `SplitView` with three panels. Needs `is_wide` / `is_tall` props and a responsive `SplitView` (list → editor → preview when narrow).
- `assets/qml/SuttaSearchWindow.qml` — Parent window; declares `is_wide` (line 52) and `is_tall` (line 53), and instantiates both dialogs at lines 1795 and 1800. Must bind the two properties through.
- `assets/qml/TabListDialog.qml` — Reference implementation of the wide/narrow layout pattern (uses `GridLayout`, but technique of single content + conditional layout props applies).
- `PROJECT_MAP.md` — Update if the dialog layout story shifts materially; otherwise no change needed.

### Notes

- No new files are created, so `bridges/build.rs` does not need to change.
- `required property bool is_wide` / `is_tall` means the parent MUST pass the value in the same change or the dialog will fail to instantiate — update `SuttaSearchWindow.qml` and the dialog properties together.
- Per CLAUDE.md: do not run `make qml-test` unless asked; do not run the GUI; verify with `make build -B` only. Run tests only after all sub-tasks of a top-level task are complete.
- `SplitView` attached properties `SplitView.preferredWidth` / `SplitView.preferredHeight` both apply depending on orientation; setting both is harmless and lets a single child work in either mode.

## Tasks

- [ ] 1.0 Wire `is_wide` and `is_tall` from `SuttaSearchWindow` into `ModelsDialog` and `AnkiExportDialog`
  - [ ] 1.1 In `assets/qml/ModelsDialog.qml`, add `required property bool is_wide` and `required property bool is_tall` near the existing `required property int top_bar_margin`.
  - [ ] 1.2 In `assets/qml/AnkiExportDialog.qml`, add the same two required properties next to `top_bar_margin`.
  - [ ] 1.3 In `assets/qml/SuttaSearchWindow.qml` at the `ModelsDialog { ... }` declaration (line 1795), pass `is_wide: root.is_wide` and `is_tall: root.is_tall`.
  - [ ] 1.4 In `assets/qml/SuttaSearchWindow.qml` at the `AnkiExportDialog { ... }` declaration (line 1800), pass `is_wide: root.is_wide` and `is_tall: root.is_tall`.
  - [ ] 1.5 Run `make build -B` and confirm no QML binding / required-property errors.

- [ ] 2.0 Adapt `ModelsDialog.qml` to a responsive `SplitView`
  - [ ] 2.1 Change the `SplitView`'s `orientation` from `Qt.Horizontal` to `root.is_wide ? Qt.Horizontal : Qt.Vertical`.
  - [ ] 2.2 On the provider-list `Item` child, keep existing `SplitView.preferredWidth: 250` / `SplitView.minimumWidth: 200`; additionally set `SplitView.preferredHeight: root.is_tall ? 240 : 180` and `SplitView.minimumHeight: 120` so it sizes correctly in vertical orientation.
  - [ ] 2.3 On the details-panel `Item` child (currently `SplitView.fillWidth: true`), add `SplitView.fillHeight: true` so it fills remaining space when stacked vertically. Leave `fillWidth` in place — it remains valid in horizontal mode.
  - [ ] 2.4 Verify the details panel's inner `ScrollView` still scrolls content in both orientations (no code change expected — note from inspection).
  - [ ] 2.5 Run `make build -B`.

- [ ] 3.0 Adapt `AnkiExportDialog.qml` to a responsive `SplitView`
  - [ ] 3.1 Change the outer `SplitView` `orientation` to `root.is_wide ? Qt.Horizontal : Qt.Vertical`.
  - [ ] 3.2 On the template-list `Item` (currently `preferredWidth: 200`, `minimumWidth: 150`), add `SplitView.preferredHeight: root.is_tall ? 200 : 140` and `SplitView.minimumHeight: 100`.
  - [ ] 3.3 On the editor `Item` (currently `SplitView.fillWidth: true`), add `SplitView.fillHeight: true` and `SplitView.minimumHeight: root.is_tall ? 240 : 160`.
  - [ ] 3.4 On the preview `Item` (currently `preferredWidth: 350`, `minimumWidth: 250`), add `SplitView.preferredHeight: root.is_tall ? 280 : 200` and `SplitView.minimumHeight: 150`.
  - [ ] 3.5 Confirm the children remain in declaration order (list, editor, preview) — this matches the PRD narrow-mode stack order and the current wide-mode left-to-right order, so no reordering is needed.
  - [ ] 3.6 Verify the editor and preview inner `ScrollView`s still scroll in both orientations (no code change expected).
  - [ ] 3.7 Run `make build -B`.

- [ ] 4.0 Build verification and manual smoke test
  - [ ] 4.1 Run `make build -B` after all edits and confirm a clean build.
  - [ ] 4.2 Run `cd backend && cargo test` (per CLAUDE.md: tests run only after all sub-tasks of a top-level task are done — this satisfies that for the project as a whole).
  - [ ] 4.3 Ask the user to manually smoke-test on desktop (wide window) and on a narrow window / mobile: open Models dialog and Anki Export dialog; verify stacked layout appears below the `is_wide` breakpoint (`width <= 650` on desktop), horizontal layout above; verify the splitter divider drags in both orientations and that inner scroll areas work.
  - [ ] 4.4 If `PROJECT_MAP.md` documents dialog layout conventions, add a one-liner noting both dialogs use the `is_wide` / `is_tall` responsive `SplitView` pattern (skip if no such section exists).
