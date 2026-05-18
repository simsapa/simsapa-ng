## Relevant Files

- `tasks/prd-reorder-tabs-in-tab-list-dialog.md` — Source PRD this task list is derived from.
- `assets/keybindings.json` — Defines all keybinding actions; two new entries (`tab_list_move_tab_up`, `tab_list_move_tab_down`) will be added here.
- `assets/qml/TabListDialog.qml` — Hosts the Tabs section; needs new Up/Down buttons, reorder logic, per-group separators, and two new `Shortcut` items wired to the keybinding system.
- `assets/qml/SuttaSearchWindow.qml` — Owns `tabs_pinned_model`, `tabs_results_model`, `tabs_translations_model`, `suttas_tab_bar`, `root.keybindings`, `root.get_sequences()`, and `root.focus_on_tab_with_id_key()`. Pass keybindings into the dialog and add a `tabReorderRequested(id_key)` (or equivalent) hookup so the dialog can focus the moved tab via the existing helper.
- `assets/qml/AppSettingsWindow.qml` — Already iterates over all action IDs returned by `SuttaBridge.get_action_names_json()`. No code change expected; verify the two new actions render and are editable.
- `bridges/build.rs` — Only needs an edit if any new QML files are added. (None planned, but verify.)
- `backend/src/app_settings.rs`, `bridges/src/sutta_bridge.rs` — No source changes expected. Verified that `get_default_keybindings_json` / `get_action_names_json` already read from `keybindings.json` via `get_keybinding_definitions()`.
- `PROJECT_MAP.md` — Update to mention the new keybinding actions and the in-dialog reorder feature.
- `docs/` — Update relevant user-facing docs if tab list / keybindings are documented.

### Notes

- Per project memory: skip running tests between sub-tasks; only run after all sub-tasks of a top-level task are complete. Skip `make qml-test` unless explicitly asked. Always use `make build -B` rather than direct `cmake` calls. Don't gate integration tests behind `#[ignore]` for needing the real appdata DB. Don't `sed -i` for refactors — make per-site Edit calls.
- GUI manual verification should be done by the user; do not run the GUI in agent context.
- Webviews live in `sutta_html_view_layout` keyed by `web_item_key` and **must not** be touched by the reorder logic. Use `ListModel.move(from, to, 1)` exclusively on the source `ListModel` — never `remove` + `append`.

### Design hazards uncovered during code review

These are addressed in the sub-tasks below. Keep them in mind throughout implementation:

1. **`ListModel.move()` causes the `TabBar`'s `currentIndex` to point at the *wrong* delegate after the move**, because `currentIndex` is a plain int that does not follow row reordering. The neighbor delegate that shifts into the old `currentIndex` slot will fire `onCheckedChanged → tab_checked_changed()`, which on a never-activated neighbor will **create a webview** and push a spurious `nav_history` entry. Mitigation: a suppression guard around the move + a direct refocus (see task 3.5).
2. **QML bindings do not track property reads inside called functions.** A binding like `enabled: control.can_move_up()` will go stale after the first evaluation. The `enabled` expression must read `tab_list_view.currentIndex`, `combined_tabs_model.count`, and `control.active_column` **directly in the binding text** so dependencies are registered (see tasks 2.5 and 3.7).
3. **`id_key` is assumed unique across all three source models.** Confirmed by existing `get_tab_with_id_key` / pin-unpin behavior, but add a defensive comment in the lookup helper so a future change doesn't break the assumption (task 3.3).
4. **Defensive `from !== to` check** before calling `ListModel.move()` to avoid Qt warning logs even when `can_move_*()` guards are correct (task 3.5).

## Tasks

- [ ] 1.0 Add the two new keybinding action definitions for tab reordering
  - [ ] 1.1 Add a JSON object to `assets/keybindings.json` with `id: "tab_list_move_tab_up"`, `name: "Move Tab Up (Tab List)"`, `description: "Move the selected tab up within its group in the Tab List dialog"`, `shortcuts: ["Shift+Up"]`. Place it adjacent to other tab-related actions (e.g. near `pin_tab` / `toggle_tab_list`) to keep the file readable.
  - [ ] 1.2 Add a second JSON object with `id: "tab_list_move_tab_down"`, `name: "Move Tab Down (Tab List)"`, `description: "Move the selected tab down within its group in the Tab List dialog"`, `shortcuts: ["Shift+Down"]`.
  - [ ] 1.3 Validate the JSON parses (e.g. `cd backend && cargo build` — the `include_str!("../../assets/keybindings.json")` plus `serde_json::from_str::<Vec<KeybindingDefinition>>` will reject malformed JSON at runtime, but a quick `python -m json.tool < assets/keybindings.json > /dev/null` is cheap and catches errors early).
  - [ ] 1.4 Run `make build -B` and confirm a clean build.

- [ ] 2.0 Add Up/Down reorder buttons to the Tabs section header in TabListDialog.qml
  - [ ] 2.1 In the header `RowLayout` (around lines 133–153 of `assets/qml/TabListDialog.qml`), insert two `Button { flat: true; font.pointSize: 9 }` controls immediately **to the left of** the existing "Clear" button: first the Up button, then the Down button.
  - [ ] 2.2 Use unicode glyphs as button text: `"▲"` for Up, `"▼"` for Down. Add `ToolTip.visible: hovered`, `ToolTip.text: "Move tab up"` / `"Move tab down"` (no shortcut in the tooltip text).
  - [ ] 2.3 Give the buttons explicit `id`s (e.g. `move_up_btn`, `move_down_btn`) so their `enabled` state can be data-bound from the helpers added in task 3.
  - [ ] 2.4 Initially wire `onClicked` to placeholder calls of stub functions `control.move_selected_tab_up()` / `control.move_selected_tab_down()` (to be implemented in task 3) so the dialog compiles in isolation.
  - [ ] 2.5 Bind `enabled` so QML's binding tracker registers the right dependencies. Critically, `tab_list_view.currentIndex`, `combined_tabs_model.count`, and `control.active_column` must be **read directly in the binding expression**, not only inside the helper function — otherwise the binding goes stale (see hazard 2 in Notes). Example:

    ```qml
    enabled: control.active_column === "tabs"
             && combined_tabs_model.count > 0
             && tab_list_view.currentIndex >= 0
             && control.can_move_up()   // boundary check (function may stay stale safely because all reactive inputs are above)
    ```

    The `can_move_up()` / `can_move_down()` helpers themselves are added in task 3; for now stub them to return `false` so the buttons stay disabled in this intermediate state.
  - [ ] 2.6 Run `make build -B`; the dialog should compile and the two buttons render as disabled flat icon buttons.

- [ ] 3.0 Implement the in-group reorder logic (combined-row ↔ source-row mapping, `ListModel.move`, post-move selection sync, spurious-activation suppression)
  - [ ] 3.1 Add a signal `tabReorderRequested(string id_key)` to `TabListDialog.qml`. Wire it in `SuttaSearchWindow.qml` (where the `TabListDialog` is instantiated, around line 2914) to call `root.focus_on_tab_with_id_key(id_key)` **after** the guard is cleared in step 3.5.7. (`focus_on_tab_with_id_key → tab.click()` may be a no-op if the moved tab is already at `suttas_tab_bar.currentIndex`; that is fine.)
  - [ ] 3.2 In `TabListDialog.qml`, add a helper `function get_source_model_for_group(group_label) { ... }` returning the matching `ListModel` (`tabs_pinned_model`, `tabs_results_model`, or `tabs_translations_model`).
  - [ ] 3.3 Add a helper `function find_source_index_by_id_key(source_model, id_key) { ... }` that linear-scans the source model and returns the index of the row whose `id_key` matches, or `-1`. Add a one-line comment noting the unique-`id_key` invariant relied on here.
  - [ ] 3.4 Add `function can_move_up() { ... }` and `function can_move_down() { ... }`. Both return `false` if `tab_list_view.currentIndex < 0` or `combined_tabs_model.count === 0`. Otherwise they check the neighbor row in `combined_tabs_model` (index − 1 / index + 1, with bounds) and return `true` iff that neighbor exists **and** has the same `group_label` as the current row.
  - [ ] 3.5 Add a suppression guard to `SuttaSearchWindow.qml` to neutralise the `TabBar.currentIndex` mismatch hazard (hazard 1):
    1. Add `property bool suppress_tab_checked_changed: false` on `root` in `SuttaSearchWindow.qml`.
    2. At the very top of `suttas_tab_bar.tab_checked_changed(...)` (line ~2497), add `if (root.suppress_tab_checked_changed) return;` so neither the activation path nor the lazy-webview-creation path nor the `nav_history_push` runs while a reorder is in progress.
    3. Expose a way for the dialog to set this — simplest: have the dialog connect directly via the `Connections` pattern or set `root.suppress_tab_checked_changed = true` through a callback. Cleanest: expose two signals on the dialog — `reorderStarting()` and `reorderFinished(string moved_id_key)` — and in `SuttaSearchWindow.qml`'s `TabListDialog { ... }` block, wire `onReorderStarting: root.suppress_tab_checked_changed = true` and `onReorderFinished: function(id_key) { root.suppress_tab_checked_changed = false; root.focus_on_tab_with_id_key(id_key); }`. This replaces / subsumes the `tabReorderRequested` signal from step 3.1 — use these two finer-grained signals instead.
  - [ ] 3.6 Implement `function move_selected_tab(direction) { ... }` where `direction` is `-1` (Up) or `+1` (Down):
    1. Bail out early if the corresponding `can_move_*()` guard returns false.
    2. Read the current and neighbor rows from `combined_tabs_model`. Remember `moved_id_key = current.id_key`.
    3. Resolve both rows to `{ source_model, source_index }` using the helpers from 3.2 / 3.3. If either lookup returns `-1`, log an error via `logger` and abort.
    4. Defensive: confirm both rows map to the same `source_model` (they always should because their `group_label`s match), and confirm `current_source_index !== neighbor_source_index`. If either check fails, log and abort.
    5. Emit `control.reorderStarting()` (sets the suppression guard).
    6. Call `source_model.move(current_source_index, neighbor_source_index, 1)`. The webview in `sutta_html_view_layout` is unaffected; the `Repeater` reorders delegates without recreating them; `tab_checked_changed` calls during this re-layout are no-ops due to the guard.
    7. Call `control.populate_model()` to rebuild `combined_tabs_model`.
    8. Re-find the moved tab in the rebuilt `combined_tabs_model` by `id_key` and set `tab_list_view.currentIndex` to that row.
    9. Emit `control.reorderFinished(moved_id_key)`. The slot in `SuttaSearchWindow.qml` clears the guard and calls `focus_on_tab_with_id_key(moved_id_key)`, which sets `suttas_tab_bar.currentIndex` to the moved tab's new global TabBar index via `tab.click()`. Because the moved tab's `web_item_key` is still in `sutta_html_view_layout` (or still `""` if it was never activated), no webview creation or switch is needed for the moved tab itself; if `tab.click()` does trigger `tab_checked_changed`, the existing early-return on `sutta_html_view_layout.current_key === tab.web_item_key` will skip the activation path for the common case.
  - [ ] 3.7 Define `function move_selected_tab_up()` / `function move_selected_tab_down()` as thin wrappers around `move_selected_tab(-1)` / `move_selected_tab(+1)`. Confirm the `onClicked` wiring from task 2.4 now invokes real logic.
  - [ ] 3.8 Replace the stubbed `can_move_up()` / `can_move_down()` from task 2.5 with the real implementations from 3.4. Verify the `enabled` binding written in task 2.5 reads `tab_list_view.currentIndex`, `combined_tabs_model.count`, and `control.active_column` directly in the expression so changes to those properties re-evaluate the binding. Functions called inside the binding (`can_move_up()` / `can_move_down()`) do **not** establish dependencies on properties they read internally — that is by design and OK as long as the explicit-read properties cover every input that can change.
  - [ ] 3.9 Run `make build -B` and confirm clean build. (Per project memory: do not run other tests until end-of-feature.)

- [ ] 4.0 Wire customizable keyboard shortcuts in TabListDialog.qml using the new keybinding actions
  - [ ] 4.1 Add a `required property var keybindings` to `TabListDialog.qml`. Pass `keybindings: root.keybindings` from the `TabListDialog { ... }` instantiation in `SuttaSearchWindow.qml`.
  - [ ] 4.2 Add a helper `function get_sequences(action_id) { return control.keybindings[action_id] || []; }` inside `TabListDialog.qml`, mirroring the `root.get_sequences()` pattern in `SuttaSearchWindow.qml:182`.
  - [ ] 4.3 Add two new `Shortcut` items inside the dialog. As in task 2.5, the `enabled` binding must read all reactive inputs **directly** so it doesn't go stale:

    ```qml
    Shortcut {
        sequences: control.get_sequences("tab_list_move_tab_up")
        enabled: control.visible
                 && control.active_column === "tabs"
                 && combined_tabs_model.count > 0
                 && tab_list_view.currentIndex >= 0
                 && control.can_move_up()
        onActivated: control.move_selected_tab_up()
    }
    Shortcut {
        sequences: control.get_sequences("tab_list_move_tab_down")
        enabled: control.visible
                 && control.active_column === "tabs"
                 && combined_tabs_model.count > 0
                 && tab_list_view.currentIndex >= 0
                 && control.can_move_down()
        onActivated: control.move_selected_tab_down()
    }
    ```

  - [ ] 4.4 Confirm the new actions appear in the keybindings list of `AppSettingsWindow.qml` (manual user check). No code change in `AppSettingsWindow.qml` should be needed because it iterates over all IDs from `SuttaBridge.get_action_names_json()`.
  - [ ] 4.5 Verify conflict detection: in `AppSettingsWindow.qml`, attempting to bind one of the new actions to a sequence already used by another action should be rejected by the existing `find_conflict` logic (lines ~96–106).
  - [ ] 4.6 Update the QML type stub at `assets/qml/com/profoundlabs/simsapa/SuttaBridge.qml` only if any new bridge function was added (none planned here — keybindings are read via the existing `get_keybindings_json()`).
  - [ ] 4.7 Run `make build -B` and confirm clean build.

- [ ] 5.0 Add horizontal divider separators between tab groups in `tab_list_view`
  - [ ] 5.1 Inside the `tab_item_delegate` (around lines 175–229 of `TabListDialog.qml`), add a top-anchored `Rectangle` overlay with `height: 1`, `color: control.palette.mid`, `anchors.left/right: parent.left/right`, `anchors.top: parent.top`.
  - [ ] 5.2 Set the rectangle's `visible` to `true` iff the delegate's `index > 0` **and** the previous combined row has a different `group_label`. Implement this by looking up `combined_tabs_model.get(index - 1).group_label` inside the delegate (safe because `combined_tabs_model` is the `model` of `tab_list_view`).
  - [ ] 5.3 Verify the divider does not interfere with the highlight rectangle, hover, or click area of the delegate.
  - [ ] 5.4 Run `make build -B` and confirm clean build.

- [ ] 6.0 Final verification and documentation
  - [ ] 6.1 Run `make build -B` from project root; confirm zero errors and warnings.
  - [ ] 6.2 Run `cd backend && cargo test` for the Rust backend. Ignore any pre-existing unrelated test failures (per project memory) — just confirm the build is clean and no newly introduced tests fail.
  - [ ] 6.3 Update `PROJECT_MAP.md` to mention: (a) the new in-dialog tab reorder feature in `TabListDialog.qml`, and (b) the two new keybinding actions in `assets/keybindings.json`.
  - [ ] 6.4 If `docs/` contains user-facing documentation for the tab list or keybindings, add a short section describing the Up/Down reorder buttons and the customizable shortcuts.
  - [ ] 6.5 Hand off to the user for manual GUI verification, with particular attention to the design hazards uncovered during review:
    - Move within each group (Pinned / Results / Translations); group boundaries disable Up/Down correctly.
    - Selection in the dialog follows the moved tab (repeat presses keep moving the same tab).
    - `suttas_tab_bar` order updates live; the moved tab keeps its **scroll position and rendered HTML content** (proves no webview recreation).
    - Moving a tab adjacent to a **never-activated** neighbor must **not** silently create a webview for that neighbor (proves the suppression guard works). Easiest check: open a Results tab to populate it, add several more without clicking them (they stay blank/never activated), then move the populated one. Inspect logs or use the app's debug output to confirm no `web_item_key` was generated for the bystander tabs.
    - `nav_history` gets no spurious entry from the move operation itself.
    - Rebinding the two new actions in `AppSettingsWindow.qml` takes effect without restart, and conflict detection rejects collisions with other actions.
