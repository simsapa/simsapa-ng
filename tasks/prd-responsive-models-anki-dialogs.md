# PRD: Responsive Layouts for ModelsDialog and AnkiExportDialog

## 1. Introduction/Overview

`ModelsDialog.qml` (AI model/provider settings) and `AnkiExportDialog.qml`
(Anki card template/preview editor) currently use a horizontal `SplitView`
with two or three side-by-side panels. On narrow mobile screens (Android,
iOS portrait) the panels are squeezed to the point of being unusable.

Both dialogs must adapt to a narrow-screen layout by switching to a
vertically-stacked `SplitView` when the parent indicates the window is
narrow, following the pattern already used by `TabListDialog.qml` — which
uses a single `GridLayout` (or equivalent) with layout-only properties
flipped between wide and narrow modes, without duplicating the inner
content code.

The goal is to make both dialogs usable on mobile while preserving the
desktop experience.

## 2. Goals

1. `ModelsDialog` and `AnkiExportDialog` render correctly and remain fully
   usable on narrow mobile screens.
2. Wide and narrow layouts share a single copy of each panel's content
   (no duplicated sub-trees).
3. Layout adapts reactively to `is_wide` and `is_tall` properties passed in
   by the parent window (`SuttaSearchWindow.qml`).
4. The draggable `SplitView` divider behavior is preserved in both
   orientations.
5. Header rows (Export Format combo, Auto-retry checkbox, etc.) wrap onto
   multiple rows when narrow so controls remain readable.

## 3. User Stories

- As a mobile user, I want to open the AI Models settings on my phone so
  that I can paste an API key and enable a provider without the provider
  list and detail panel fighting for the same 3cm of width.
- As a mobile user, I want to edit Anki card templates on my phone and see
  the template editor, list, and preview stacked vertically so I can
  scroll through each section at a usable size.
- As a desktop user, I want the existing two/three-column layouts to be
  preserved unchanged, including drag-to-resize dividers.
- As a developer maintaining these dialogs, I want one copy of each
  panel's QML so that edits to e.g. the provider list don't have to be
  applied twice.

## 4. Functional Requirements

### 4.1 Shared requirements (both dialogs)

1. Each dialog MUST declare `required property bool is_wide` and
   `required property bool is_tall`.
2. `SuttaSearchWindow.qml` MUST pass `is_wide: root.is_wide` and
   `is_tall: root.is_tall` when instantiating `ModelsDialog` and
   `AnkiExportDialog` (see `SuttaSearchWindow.qml:1795` and `:1800`).
3. The main panel container MUST remain a `SplitView`, with
   `orientation` bound to `control.is_wide ? Qt.Horizontal : Qt.Vertical`.
   The draggable divider MUST continue to work in both orientations.
4. Panel content (the `Item` children of the `SplitView`) MUST NOT be
   duplicated between layout modes. Layout-affecting properties
   (`SplitView.preferredWidth`, `SplitView.preferredHeight`,
   `SplitView.minimumWidth`, `SplitView.minimumHeight`) MUST be set
   conditionally on `is_wide`.
5. When `is_wide` is `false`, the overall dialog content MUST be
   wrapped/driven such that the stacked panels can scroll vertically if
   they exceed the window height (the `SplitView` itself provides sized
   panes; see 4.1.7).
6. Header rows above the `SplitView` (e.g. title row, Export Format /
   Include-cloze row in Anki, Auto-retry row in Models) MUST remain
   unchanged — no re-layout of headers is required for this PRD.
7. In narrow mode, each stacked panel MUST have a reasonable fixed
   preferred vertical height (see per-dialog specs below) so the user
   sees all panels at once and can drag the splitter to rebalance.

### 4.2 `ModelsDialog.qml` specific

8. Wide layout: identical to current — provider list on the left
   (`preferredWidth: 250`, `minimumWidth: 200`), details panel fills
   the rest.
9. Narrow layout:
   - Provider list on top with
     `SplitView.preferredHeight: is_tall ? 240 : 180`,
     `SplitView.minimumHeight: 120`.
   - Details panel below, filling remaining height
     (`SplitView.fillHeight: true`).
   - Both panels MUST use the full dialog width in narrow mode.
10. The details panel's internal `ScrollView` MUST continue to scroll its
    content vertically in both modes.

### 4.3 `AnkiExportDialog.qml` specific

11. Wide layout: identical to current — template list
    (`preferredWidth: 200`, `minimumWidth: 150`), editor (`fillWidth`),
    preview (`preferredWidth: 350`, `minimumWidth: 250`).
12. Narrow layout, top-to-bottom (list, editor, preview):
    - Template list: `SplitView.preferredHeight: is_tall ? 200 : 140`,
      `SplitView.minimumHeight: 100`.
    - Editor: `SplitView.fillHeight: true`,
      `SplitView.minimumHeight: is_tall ? 240 : 160`.
    - Preview: `SplitView.preferredHeight: is_tall ? 280 : 200`,
      `SplitView.minimumHeight: 150`.
    - All panels use full dialog width.
13. The editor's `TextArea` and the preview's `TextArea` MUST remain
    inside their existing `ScrollView`s so long content scrolls.

### 4.4 Integration

14. `SuttaSearchWindow.qml` MUST bind `is_wide` and `is_tall` on the two
    dialog declarations (pattern already established for `TabListDialog`
    — see `SuttaSearchWindow.qml:2048-2049`).

## 5. Non-Goals (Out of Scope)

- No changes to the business logic of either dialog (provider
  management, template saving, preview rendering).
- No redesign of the header control rows (Export Format combo,
  Auto-retry checkbox, title/icon row).
- No changes to `TabListDialog.qml`.
- No new breakpoint/property beyond reusing the existing `is_wide` and
  `is_tall` from `SuttaSearchWindow.qml`.
- No change to the confirmation/error sub-dialogs inside `ModelsDialog`.
- No tablet-specific intermediate layout — only the two modes.

## 6. Design Considerations

- Follow `TabListDialog.qml` as the reference pattern for single-content
  / two-layout structure. The key technique is a single container
  (there: `GridLayout`; here: `SplitView`) whose orientation/column
  count is bound to `is_wide`, with per-child layout properties
  conditionally set.
- `SplitView` supports both `Qt.Horizontal` and `Qt.Vertical`
  orientations; the `SplitView.preferredWidth` vs
  `SplitView.preferredHeight` attached properties apply to whichever
  axis is active. Setting both is safe — the inactive one is ignored.
- `is_wide` in `SuttaSearchWindow.qml:52` is computed as
  `is_desktop ? root.width > 650 : root.width > 800`. Both dialogs
  already know `is_mobile`; `is_wide` is the canonical signal.

## 7. Technical Considerations

- Both dialogs are `ApplicationWindow`, not `Dialog`, and receive
  `top_bar_margin` from the parent. Adding `is_wide` / `is_tall`
  follows the same pattern.
- The outer `Item { x: 10; y: 10 + top_bar_margin; implicitWidth: ... }`
  wrapper in both files remains unchanged.
- QML `required property` means the parent MUST pass the value — be
  sure to update the parent (`SuttaSearchWindow.qml`) in the same
  change, or the dialogs will fail to instantiate.
- No new QML files are created, so `bridges/build.rs` does not need
  updating.

## 8. Success Metrics

- On an Android device (or narrow desktop window ≤ 650 px), opening
  each dialog shows stacked panels with all controls reachable without
  horizontal scrolling.
- On desktop at default size, both dialogs look and behave identically
  to the current implementation (visual diff limited to nothing).
- No duplicated panel QML introduced: each panel's delegate / inner
  `ColumnLayout` appears exactly once in each file.
- `make build -B` succeeds; manual smoke test of both dialogs on
  desktop and on a narrow window confirms usability.

## 9. Open Questions

None — resolved during clarification:
- Narrow-mode heights use reasonable fixed values, switched on
  `is_tall` (see sections 4.2 and 4.3).
- AnkiExport narrow stack order is list → editor → preview.
