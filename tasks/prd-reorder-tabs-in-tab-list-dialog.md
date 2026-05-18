# PRD: Reorder Tabs in TabListDialog

## 1. Introduction / Overview

The `TabListDialog.qml` currently lists all open sutta tabs (grouped as Pinned, Results, and Translations) and offers a "Clear" action. Users have no way to change the order of their open tabs from this dialog. This feature adds **Up** and **Down** buttons (plus customizable keyboard shortcuts) that let the user move the selected tab within its own group, with the change immediately reflected in the main `suttas_tab_bar` of `SuttaSearchWindow.qml`.

The goal is to give users direct control over the visual ordering of their sutta tabs without needing drag-and-drop, while keeping the operation scoped safely to each group (Pinned, Results, Translations).

## 2. Goals

1. Allow the user to reorder tabs from within `TabListDialog.qml` using two new icon buttons (▲ and ▼).
2. Reordering must only move tabs **within the same group** (Pinned, Results, or Translations) — never across groups.
3. The change must be reflected immediately in the underlying tab models and in `suttas_tab_bar` while the dialog is still open.
4. The buttons must correctly disable at group boundaries.
5. The corresponding keyboard shortcuts must be user-customizable via the keybindings section of `AppSettingsWindow.qml`.

## 3. User Stories

1. As a user with several open Results tabs, I want to move a tab up or down in the list so that the visual order in the tab bar matches the order I find most useful.
2. As a user, when I move a tab, I want it to stay selected in the dialog so I can continue reordering it with multiple presses.
3. As a user, I want the Up button to be disabled when I'm already at the top of a group so I can't accidentally move a tab into a different group.
4. As a keyboard-driven user, I want a customizable shortcut for "move tab up" and "move tab down" so I can rebind it to keys I prefer.

## 4. Functional Requirements

### 4.1 Buttons in TabListDialog.qml

1. The system must add two flat icon buttons — **Up** (▲) and **Down** (▼) — in the existing header `RowLayout` of the Tabs section, immediately to the **left** of the "Clear" button.
2. Both buttons must use icons (Qt standard arrow icons or unicode glyphs) and have tooltips: "Move tab up" / "Move tab down".
3. The buttons must only act on the currently selected item in `tab_list_view` when `active_column === "tabs"`.
4. The buttons must be disabled when `active_column !== "tabs"` or when `tab_list_view.currentIndex < 0`.

### 4.2 Disabled state at group boundaries

5. The **Up** button must be disabled when the currently selected tab is the **first** item of its own group in `combined_tabs_model` (i.e. the item above it belongs to a different group, or it is index 0).
6. The **Down** button must be disabled when the currently selected tab is the **last** item of its own group in `combined_tabs_model` (i.e. the item below it belongs to a different group, or it is the last index).
7. The disabled state must update automatically whenever `tab_list_view.currentIndex` changes or the model is repopulated.

### 4.3 Reorder behavior

8. When **Up** is pressed, the system must swap the selected tab with the tab immediately above it **within the same group** in the underlying source model (`tabs_pinned_model`, `tabs_results_model`, or `tabs_translations_model`, depending on the tab's `group_label`).
9. When **Down** is pressed, the system must swap the selected tab with the tab immediately below it within the same group in the underlying source model.
10. Blank placeholder tabs (those for which `is_blank_tab(item_uid)` returns true — `"Sutta"`, `"Word"`, or empty `item_uid`) must be **ignored** by the reorder operation. The swap operates on the next/previous non-blank tab within the group in the source model.
11. After a swap, the system must repopulate `combined_tabs_model` (via the existing `populate_model()` function) so the dialog reflects the new order.
12. After a swap, the system must update `tab_list_view.currentIndex` so that the **moved tab remains the selected item** in the dialog (i.e. selection follows the moved tab to its new position).
13. The reorder change must be reflected **immediately and live** in `suttas_tab_bar` in `SuttaSearchWindow.qml` (because that `TabBar` is driven by the same `tabs_pinned_model`, `tabs_results_model`, and `tabs_translations_model` instances, swapping rows in those `ListModel`s using `ListModel.move(from, to, 1)` should propagate naturally; persistence across app restarts is **not** required).

### 4.4 Keyboard shortcuts

14. The system must add two new keybinding action definitions in `assets/keybindings.json`:
    - `id: "tab_list_move_tab_up"`, `name: "Move Tab Up (Tab List)"`, `description: "Move the selected tab up within its group in the Tab List dialog"`, default `shortcuts: ["Shift+Up"]`.
    - `id: "tab_list_move_tab_down"`, `name: "Move Tab Down (Tab List)"`, `description: "Move the selected tab down within its group in the Tab List dialog"`, default `shortcuts: ["Shift+Down"]`.
15. The shortcuts must be wired up inside `TabListDialog.qml` (alongside the existing `Shortcut` items) and must only fire when `control.visible` is true and `active_column === "tabs"`.
16. The shortcuts must respect the same disabled conditions as the buttons (first/last item of group, no selection, etc.) — they must be no-ops at group boundaries rather than wrapping or jumping groups.
17. The shortcuts must appear and be editable in the keybindings section of `AppSettingsWindow.qml` automatically (no special UI work required, because the existing UI iterates over all action IDs returned by `SuttaBridge.get_action_names_json()`).
18. The shortcut values used by `TabListDialog.qml` must be read from the user's keybinding settings via the existing keybindings mechanism (the same way other in-app shortcuts are resolved), so customizations take effect without restarting.

## 5. Non-Goals (Out of Scope)

- Drag-and-drop reordering of tabs.
- Moving tabs **across** groups (e.g. promoting a Results tab to Pinned).
- Persisting tab order across application restarts.
- Reordering tabs from anywhere other than `TabListDialog.qml` (e.g. directly from the tab bar context menu).
- Reordering items in the History column.
- Multi-select reorder (moving several tabs at once).

## 6. Design Considerations

### 6.1 Up/Down buttons

- Buttons should be small `Button { flat: true }` controls consistent with the existing "Clear" button styling (`font.pointSize: 9`).
- Prefer Qt's standard arrow icons (e.g. `Qt.resolvedUrl` to an icon asset, or use unicode "▲" / "▼" as `text` if no icon assets exist for arrows in the project).
- Disabled buttons should follow standard Qt disabled appearance (greyed out).
- Tooltips are plain static text: `"Move tab up"` and `"Move tab down"` (no shortcut text).
- No visual separator between the Up/Down buttons and the Clear button.

### 6.2 Horizontal separators between groups in the tab list

19. The system must render a thin horizontal divider line (1 px tall `Rectangle` using `control.palette.mid`) **between** items that belong to different groups in `tab_list_view`. Specifically, a divider appears at the top of an item iff the previous visible item belongs to a different `group_label`.
20. The divider must be drawn inside the `ItemDelegate` (as an overlay/anchor at the top edge), so it does not affect highlight/selection behavior.
21. No divider above the first item, and none in groups that have no visible items.

## 7. Technical Considerations

### 7.1 How tabs and webviews are wired together (background)

Understanding this is essential because the reorder operation must **not** destroy or recreate the HTML view (webview) associated with any tab.

- The three source models (`tabs_pinned_model`, `tabs_results_model`, `tabs_translations_model`) are `ListModel`s declared in `SuttaSearchWindow.qml` (around lines 297–299). Each row carries fields including `id_key`, `item_uid`, `web_item_key`, `sutta_title`, `sutta_ref`, etc. They are passed into `TabListDialog.qml` as `required property var`.
- `suttas_tab_bar` (`SuttaSearchWindow.qml:2491`) contains three `Repeater`s (`tabs_pinned`, `tabs_results`, `tabs_translations`, lines ~2734–2890), each bound to one of the source models. Each delegate is a `SuttaTabButton` whose `id_key`, `item_uid`, `web_item_key`, `index` properties come from the model row.
- HTML views (webviews) live in a **separate** stack layout, `sutta_html_view_layout` (a `SuttaStackLayout`), keyed by `web_item_key` (not by tab index, and not parented to the tab buttons). A tab "has no webview yet" when its `web_item_key === ""` (lazy creation).
- On first activation of a tab, `tab_checked_changed()` (line 2497) generates a `web_item_key`, writes it back into the source model via `tab_model.set(tab.index, tab_data)`, and calls `sutta_html_view_layout.add_item(tab_data, false)`. Subsequent activations of the same tab simply call `sutta_html_view_layout.current_key = tab.web_item_key`, which switches the visible webview without recreating anything.
- Existing pin/unpin operations (e.g. `onPinToggled` at line 2739 and 2789) move a tab between groups by `append`-ing the same data (preserving `web_item_key`) into the destination model and `remove`-ing the row from the source model. The webview survives because it is keyed by `web_item_key` in the *separate* layout, not parented to the source delegate. The reorder feature must follow this same principle: do not touch `sutta_html_view_layout`.

### 7.2 Required reorder primitive: `ListModel.move()`

22. The reorder operation must be implemented using `ListModel.move(from, to, 1)` on the appropriate source model. **Do not** use a remove+insert pair, because:
    - `remove(i)` of a row whose tab has an active webview can still leave that webview alive in `sutta_html_view_layout` (it is keyed by `web_item_key`), but the corresponding delegate in the `Repeater` will be destroyed and recreated, causing `onCheckedChanged` to fire spuriously and possibly toggle focus to a different tab.
    - `append(data)` followed by lookups would also push the tab to the end of the group rather than to the desired adjacent position.
    `ListModel.move()` is the only operation that preserves row identity and minimises delegate churn.

23. `ListModel.move(from, to, 1)` preserves all field values on the moved row (including `web_item_key`, which may be `""` for never-activated tabs — that is fine; the lazy-creation path in `tab_checked_changed()` will still work when the tab is later clicked). The webview in `sutta_html_view_layout` is **not** touched, because the move operates only on the `ListModel`, not on the stack layout.

24. After the move, `Repeater` will update the `index` property on the two affected delegates. The currently-checked tab (the one being moved) **must remain checked**. Because `TabBar` tracks `currentIndex` rather than the delegate instance, the implementation must call `root.focus_on_tab_with_id_key(moved_tab.id_key)` (or an equivalent helper) after the move so that `suttas_tab_bar.currentIndex` follows the moved tab to its new position. This must also be done even if the delegate happens to still be checked, to keep dialog selection and tab-bar selection in sync.

### 7.3 Mapping combined_tabs_model rows to source-model rows

25. `combined_tabs_model` inside `TabListDialog.qml` is a derived view rebuilt by `populate_model()`. Each row carries a `group_label` ("Pinned" / "Results" / "Trans"). Because `populate_model()` filters out blank placeholder tabs (`is_blank_tab(item_uid)`), the index in `combined_tabs_model` does **not** equal the index in the source model.

26. The implementation must add a helper that, given a `combined_tabs_model` row, returns `{ source_model, source_index }`. The simplest reliable approach: find the row's source model from its `group_label`, then scan that source model for a row whose `id_key` matches the combined row's `id_key` (assuming `id_key` is unique per tab — this matches existing usage such as `root.focus_on_tab_with_id_key`).

27. To compute the swap target within the same group: look at the previous (for Up) or next (for Down) row in `combined_tabs_model`; if its `group_label` differs from the current row's, the operation is disabled; otherwise, resolve that neighbor's `{ source_model, source_index }` the same way, confirm both rows resolve to the same `source_model`, and call `source_model.move(current_source_index, neighbor_source_index, 1)`. This naturally handles the "skip over blank tabs in the source model" requirement, because the lookup is by `id_key` and blank tabs are absent from `combined_tabs_model`.

### 7.4 Refreshing the dialog after a move

28. After calling `source_model.move(...)`, the dialog must call `populate_model()` to rebuild `combined_tabs_model` and then set `tab_list_view.currentIndex` to the new combined-model row of the moved tab (looked up by its `id_key`). This satisfies requirements 11 and 12.

29. The `suttas_tab_bar` `Repeater` reacts to the underlying `ListModel` move automatically — no explicit refresh of the tab bar is required.

### 7.5 Keybindings plumbing

- Keybindings are defined in `assets/keybindings.json` and surfaced to QML via `SuttaBridge.get_keybindings_json()` / `SuttaBridge.get_action_names_json()` (see `bridges/src/sutta_bridge.rs:1129`, `backend/src/app_settings.rs:403`). Adding the two new entries (`tab_list_move_tab_up`, `tab_list_move_tab_down`) to `keybindings.json` is sufficient for them to appear in `AppSettingsWindow.qml`.
- The two new `Shortcut` items added to `TabListDialog.qml` must read their `sequences` from the live keybindings (the same pattern used by other in-app `Shortcut` items in this codebase) so user customisations take effect without restarting.
- The existing conflict-detection logic in `AppSettingsWindow.qml` (`find_conflict`, lines ~96–106) must forbid binding either of the new actions to a sequence already assigned to another keybinding action. No additional conflict logic against the hardcoded navigation `Shortcut`s in `TabListDialog.qml` is required by this feature — that is left to the user's discretion.

### 7.6 Misc

- No Rust backend changes are required beyond the two new entries in `assets/keybindings.json`.
- The feature must not regress the existing keyboard navigation in the dialog (`Up/K`, `Down/J`, `Home/G`, `End/Shift+G`, `Left/H`, `Right/L`, `Return/Enter`), the Open/Clear/Close buttons, pin/unpin, tab close, or the lazy webview creation in `tab_checked_changed`.

## 8. Success Metrics

1. The user can move any non-blank tab up or down within its group from `TabListDialog.qml`, and the change is reflected in `suttas_tab_bar` before closing the dialog.
2. The Up button is disabled exactly when the selected tab is the first non-blank tab of its group; the Down button is disabled exactly when it is the last non-blank tab of its group.
3. Selection in the dialog follows the moved tab (so repeated presses keep moving the same tab).
4. The two new actions appear in `AppSettingsWindow.qml`'s keybindings list, and rebinding them changes the active shortcut without restarting the app.
5. No regressions in existing TabListDialog navigation, the "Open" / "Clear" / "Close" buttons, or `suttas_tab_bar` behavior.

## 9. Open Questions

None at this time. (Earlier open questions resolved: tooltips are static text without shortcut, no visual separator between Up/Down and Clear, conflict detection uses existing `find_conflict` logic in `AppSettingsWindow.qml`.)
