# Tasks: Combined Mode Section Headers

Source PRD: `tasks/prd-combined-mode-section-headers.md`

## Relevant Files

- `backend/src/types.rs` — `SearchResult` struct definition; add the `is_section_header` field and a `from_section_header(title)` constructor; update every `from_*` constructor literal to set the new field to `false`.
- `backend/src/search/searcher.rs` — Four direct `SearchResult { ... }` struct literals at lines ~693, ~724, ~765, ~796 must add the new field.
- `backend/src/query_task.rs` — `bold_definition_to_search_result()` (line ~47) and other `SearchResult { ... }` literal sites; check `db_sutta_to_result`, `db_word_to_result`, `db_book_spine_item_to_result`.
- `bridges/src/sutta_bridge.rs` — `fetch_combined_page()` returns the merged Combined page; this is where header rows are inserted into `merged` after slicing the real-result buffers, using `dpd_total_final` and `ft_total_final` to build the `(n)` count labels. Skip header insertion when `dpd_total_final + ft_total_final == 0`.
- `bridges/src/api.rs` — `/dict_combined_search` route uses `SearchResult` via `ApiSearchResult { results: Vec<SearchResult>, ... }`; verify the new field serializes correctly (no struct-literal change needed there).
- `assets/qml/FulltextResults.qml` — `update_page()` (propagate the new role into `results_model` from `item.is_section_header`), `search_result_delegate` (render header style when `is_section_header` is true), `Keys.onPressed` handlers and `select_previous_result()` / `select_next_result()` (skip header rows). Empty-state Text needs no change since FR 13 ensures no headers when `total_hits === 0`.

### Notes

- Per CLAUDE.md: build with `make build -B`; do not run `make qml-test` unless explicitly asked; only run tests after **all** sub-tasks of a top-level task are done.
- Per CLAUDE.md: don't use bulk `sed` for refactors — make targeted per-site Edit calls when adding the new field to all `SearchResult` constructors.
- Use `#[serde(default)]` on the new field so older cached JSON payloads (in `RESULTS_PAGE_CACHE` / `COMBINED_CACHE` if persisted, or in any disk-cached fixtures) continue to deserialize.
- Header row title strings are constructed in Rust as `format!("DPD Lookup Results ({})", dpd_total_final)` and `format!("Fulltext Results ({})", ft_total_final)`.
- Rust will not let any `SearchResult { ... }` literal compile until every site sets `is_section_header`. Lean on the compiler to find all sites — fix them targeted, not via sed.

## Tasks

- [ ] 1.0 Add `is_section_header` field to the `SearchResult` type and propagate it through all construction sites
  - [ ] 1.1 In `backend/src/types.rs`, add `pub is_section_header: bool` to the `SearchResult` struct (after `rank`), with `#[serde(default)]` so older payloads still deserialize.
  - [ ] 1.2 Update each `SearchResult::from_*` constructor in `backend/src/types.rs` (`from_sutta`, `from_dict_word`, `from_title_str`, `from_dpd_headword`, `from_dpd_root`, `from_book_spine_item`) to set `is_section_header: false`.
  - [ ] 1.3 Add a new `pub fn from_section_header(title: String) -> SearchResult` constructor in `backend/src/types.rs` that returns a `SearchResult` with `is_section_header: true`, the given `title`, and all string fields empty / numeric/option fields `None`/`0`. This is the canonical way other code creates header rows.
  - [ ] 1.4 In `backend/src/search/searcher.rs`, add `is_section_header: false` to each direct `SearchResult { ... }` struct literal (4 sites near lines 693, 724, 765, 796). Run `cargo build` to surface any literal sites missed.
  - [ ] 1.5 In `backend/src/query_task.rs`, add `is_section_header: false` to each `SearchResult { ... }` struct literal (start with `bold_definition_to_search_result()` at line ~47; lean on the compiler for the rest).
  - [ ] 1.6 Run `make build -B` and confirm the workspace compiles with no errors. Behavior on the wire is unchanged: every existing result has `is_section_header: false`.

- [ ] 2.0 Insert section-header rows in `fetch_combined_page` after the merged-page slice
  - [ ] 2.1 In `bridges/src/sutta_bridge.rs::fetch_combined_page`, locate the final assembly block (lines ~440–467) where `merged` is built from `dpd_buffer[dpd_lo..dpd_hi]` and extended with `ft_buffer[ft_lo..ft_hi]`. This is the insertion point.
  - [ ] 2.2 After computing `dpd_total_final` and `ft_total_final` but before assembling `merged`, compute a boolean `emit_headers = dpd_total_final + ft_total_final > 0` (FR 13: skip headers entirely when `total_hits === 0`).
  - [ ] 2.3 Rebuild `merged` so its row order is: `[DPD header (if emit_headers)] + dpd_buffer[dpd_lo..dpd_hi] + [Fulltext header (if emit_headers)] + ft_buffer[ft_lo..ft_hi]`. Use `SearchResult::from_section_header(format!("DPD Lookup Results ({})", dpd_total_final))` and `SearchResult::from_section_header(format!("Fulltext Results ({})", ft_total_final))`. Both headers appear on every page when `emit_headers` is true, regardless of whether that page contains rows from that section (FR 4).
  - [ ] 2.4 Confirm by inspection that `combined_total = (dpd_total_final + ft_total_final) as i64` is unchanged — headers do **not** contribute (FR 15).
  - [ ] 2.5 Confirm by inspection that the per-side offset arithmetic (`dpd_lo`, `dpd_hi`, `ft_lo`, `ft_hi`, top-up loop conditions on `dpd_needed`/`ft_needed`) only reads from the cached real-result buffers — header insertion happens after this and does not touch those values (FR 17).
  - [ ] 2.6 Run `make build -B`. Manual smoke test (user runs the app): a Combined Mode dictionary search now shows the two header rows in the result list (rendering will look like a normal item with empty snippet at this stage — full styling comes in task 3).

- [ ] 3.0 Render header rows with the section-header style in `FulltextResults.qml`
  - [ ] 3.1 In `update_page()`, extend the `result_data` object built per item to include `is_section_header: !!item.is_section_header` and `header_title: item.is_section_header ? item.title : ""`. Continue populating all existing roles with their normal values (or empty strings on header rows — `item.uid`, `item.snippet`, etc. will already be empty strings from the Rust `from_section_header` constructor).
  - [ ] 3.2 In the `search_result_delegate` Component, declare new required properties `required property bool is_section_header` and `required property string header_title`.
  - [ ] 3.3 Replace the delegate body's single rendering path with a conditional: when `is_section_header` is true, render a compact header (a `Text` with `font.bold: true`, `font.pointSize: root.font_point_size + 2`, `color: root.palette.active.text`, with a small vertical padding — no `Frame`, no `ListBackground`, no `MouseArea`); otherwise render the existing result body unchanged.
  - [ ] 3.4 Make the delegate's `height` conditional: header rows use a smaller height equal to the header text line height plus its padding (e.g. `root.tm1.height + 8`), while real results continue to use `fulltext_list.item_height`.
  - [ ] 3.5 Ensure header rows are visually non-clickable: do not attach a `MouseArea` to the header branch (so clicks on a header do nothing and never set `currentIndex`).
  - [ ] 3.6 Run `make build -B`. Manual smoke test: header rows now appear visually styled as section titles inside the Combined Mode result list.

- [ ] 4.0 Make keyboard navigation skip header rows in `FulltextResults.qml`
  - [ ] 4.1 Add a small helper function `next_selectable_index(from: int, direction: int): int` to the `root` ColumnLayout. It scans `results_model` starting at `from + direction`, in steps of `direction` (±1), and returns the first index whose row has `is_section_header === false`. If the scan reaches the bounds (`< 0` or `>= results_model.count`) without finding a non-header, return the original `from` (no movement).
  - [ ] 4.2 Update `select_previous_result()` to call `fulltext_list.currentIndex = next_selectable_index(fulltext_list.currentIndex, -1)`.
  - [ ] 4.3 Update `select_next_result()` to call `fulltext_list.currentIndex = next_selectable_index(fulltext_list.currentIndex, +1)`.
  - [ ] 4.4 Update the `Keys.onPressed` handler in `fulltext_list` to use the same helper for Up/Ctrl-K and Down/Ctrl-J branches.
  - [ ] 4.5 Update `update_page()` so that after clearing the model and repopulating it, the initial selection is set to `next_selectable_index(-1, +1)` — the first non-header row — instead of leaving `currentIndex = -1`. (Optional: leave at -1 if the model contains only headers, though FR 13 prevents that case in Combined Mode.)
  - [ ] 4.6 Run `make build -B`. Manual smoke test: pressing Down/Up through a Combined page lands on every real result exactly once and never on a header; the very first press of Down on a fresh page jumps from the (header) top to the first DPD result.

- [ ] 5.0 Verify end-to-end
  - [ ] 5.1 `make build -B` from a clean state — confirm no errors, no new warnings related to the changed sites.
  - [ ] 5.2 `cd backend && cargo test` — confirm Rust tests pass (or pre-existing failures are unrelated; per CLAUDE.md, pre-existing failures may be ignored).
  - [ ] 5.3 Manual smoke test 1 — multi-page Combined query (user-driven): both `"DPD Lookup Results (n)"` and `"Fulltext Results (n)"` headers visible on every page; `Page X of Y` and prev/next buttons behave identically to before; keyboard navigation skips both headers.
  - [ ] 5.4 Manual smoke test 2 — single-section-empty (user-driven): a query with `dpd_total > 0` but `ft_total = 0` shows `"DPD Lookup Results (n)"` followed by the DPD rows, then `"Fulltext Results (0)"` with no rows below it. And vice versa.
  - [ ] 5.5 Manual smoke test 3 — zero-hit Combined query (user-driven): no headers appear; the standard "No results found" empty-state message is shown.
  - [ ] 5.6 Regression check — non-Combined dictionary searches (DPD-only, Fulltext-only, etc.) and sutta searches show **no** headers in the result list and behave exactly as before.
