# PRD: Section Headers in Combined Mode Dictionary Search Results

## 1. Introduction/Overview

When the user runs a dictionary search in **Combined Mode**, the result list interleaves rows from two sub-queries: DPD Lookup and Fulltext. Today there is no visual indication of where one source ends and the next begins, so the user cannot tell at a glance which items came from DPD Lookup versus the fulltext index.

This feature inserts a non-interactive **section header row** at the top of each sub-query's block within the merged result list — one row labeled "DPD Lookup Results" before the DPD items and one row labeled "Fulltext Results" before the fulltext items. The headers are extra rows in the result list (not floating titles above the list) and are inserted by the Rust backend so that frontend pagination and offset logic remain unchanged.

## 2. Goals

1. The user can visually distinguish DPD Lookup results from Fulltext results in Combined Mode.
2. The change does not alter pagination math: offsets, `page_len`, `page_num`, and `total_hits` continue to represent **real result counts only**.
3. The QML rendering layer treats header rows uniformly through the existing `ListModel` role schema — no new required roles, no missing-key errors.
4. Header rows are not selectable and are skipped by keyboard navigation, so they don't disrupt the existing up/down result-cycling UX.

## 3. User Stories

- As a Pāli reader using Combined Mode, I want to see a clear "DPD Lookup Results" label before the DPD entries and a "Fulltext Results" label before the fulltext entries, so that I can quickly orient myself within a mixed result list.
- As a user paging through Combined results, I want each page to show the relevant section headers above its results, so I never lose track of which section I'm currently viewing.
- As a keyboard user pressing up/down in the result list, I want the selection to skip over the header rows and land only on real results, so navigation feels natural.

## 4. Functional Requirements

### Scope

1. The feature applies **only** to dictionary search (search area = `Dictionary`) when the search mode is `Combined`. No other search area or mode is affected.

### Backend (Rust)

2. The merged Combined-mode page returned by `fetch_combined_page` (in `bridges/src/sutta_bridge.rs`) must insert section-header rows into the `results` vector returned to the frontend, **only for sections that contribute rows on the current page**:
   - When the page contains DPD rows, a `"DPD Lookup Results (start-end)"` header row is placed immediately before the first DPD row on the page.
   - When the page contains Fulltext rows, a `"Fulltext Results (start-end)"` header row is placed immediately before the first Fulltext row on the page.
   - `start` and `end` are 1-based, inclusive indices into the section's full result list, identifying the slice shown on this page (e.g. items 11..15 of DPD render as `"DPD Lookup Results (11-15)"`). `start = section_lo + 1`, `end = section_hi`, where `section_lo..section_hi` is the half-open per-section slice for the page.
3. Header rows are inserted **after** pagination/slicing of real results, not before. This means `page_len`, the per-page offsets into the DPD and Fulltext sub-buffers, and `total_hits` (= `dpd_total + ft_total`) continue to count only real results. A page may therefore deliver up to `page_len + 2` rows on the wire (when the page straddles the DPD→Fulltext transition and includes both headers).
4. A section's header is emitted **only when that section contributes at least one row on the current page**. Consequently:
   - A page wholly within DPD shows only the DPD header at the top.
   - A page wholly within Fulltext shows only the Fulltext header at the top.
   - A page that straddles the DPD→Fulltext transition shows the DPD header at the top, the remaining DPD rows, then the Fulltext header at the transition point, then the Fulltext rows.
   - When both sections are empty (`total_hits === 0`), no header rows are emitted at all — see FR 13.
5. Section ordering is fixed: **DPD Lookup header + DPD rows first**, then **Fulltext header + Fulltext rows**, matching the existing sub-query orchestration order.
6. A header row is represented as the existing `SearchResult`-like struct populated as follows (exact field names to be aligned with the current `SearchResult` shape used by the API; placeholder fields shown):
   - `is_section_header`: `true` (new boolean field on the result struct, default `false` on real rows)
   - `title`: the header label with embedded range (`"DPD Lookup Results (start-end)"` or `"Fulltext Results (start-end)"`) — reusing the existing title field consumed by the QML model as `sutta_title`
   - All other string fields (`uid`, `table_name`, `snippet`, `sutta_ref`, etc.): empty strings (no `null`/missing keys, per clarification 7)
   - Any numeric fields (e.g. score): `0` / sensible defaults
7. The new `is_section_header` boolean must be added to the result struct and serialized to JSON so it crosses the bridge into QML.
8. Non-Combined dictionary searches and all sutta searches must produce results with `is_section_header = false` and behave exactly as today.

### Frontend (QML)

9. `assets/qml/FulltextResults.qml`'s `update_page()` must read the `is_section_header` boolean from each incoming item and append it to `results_model` with that role plus a `header_title` string populated from the item's title field (per clarification 7, reusing `item.title` / `sutta_title`). Unused fields on header rows are stored as empty strings.
10. The `search_result_delegate` component must conditionally render a **header style** when `is_section_header === true`:
    - Visual style: bold text at a slightly larger point size than the regular `font_point_size`, no `Frame` border, no `ListBackground`, with modest vertical padding. Height: only enough for the title line (smaller than `fulltext_list.item_height`).
    - The header text shows `header_title`.
    - Header rows must not respond to mouse clicks (no `MouseArea` selection) and must not be selectable as the `currentIndex`.
11. Keyboard navigation (`Keys.onPressed` Up/Down/Ctrl-J/Ctrl-K in `FulltextResults.qml`) must **skip header rows**: pressing Down past the last real item before a header should advance to the next real item after the header, and likewise for Up. If the resulting index would land on a header, continue scanning in the same direction until a non-header row is found or until the bounds of the list.
12. `select_previous_result()` / `select_next_result()` must apply the same header-skipping logic.
13. The "No results found" empty state (`empty_state` Text) must be shown in Combined Mode when **both** sections are empty across the whole query (`total_hits === 0`). In that case, no header rows are emitted at all — the user sees only the standard "No results found" message. When at least one section has results, headers are emitted as specified (only for sections contributing rows on the current page), and the empty-state message is suppressed.
14. The pagination controls (`Page X of Y`, prev/next buttons) continue to use `total_hits` and `page_len` unchanged. No frontend pagination math is modified.

### Counting & Offsets (cross-cutting)

15. `total_hits` reported to the frontend remains `dpd_total + ft_total` (real results only). Headers do not contribute to `total_hits`.
16. `page_len` continues to mean "real results per page". A page payload may contain up to `page_len + 2` items on the wire due to header insertion.
17. The combined-cache offset bookkeeping (DPD/Fulltext sub-buffer offsets driving the side-aware top-up on later pages) must remain based on real-result counts only.

## 5. Non-Goals (Out of Scope)

- Adding section headers to non-Combined modes or to non-Dictionary search areas.
- Making headers collapsible/expandable.
- Sticky/floating headers that remain visible at the top of the viewport while scrolling.
- Localizing header labels (they ship as English strings; future i18n is out of scope).
- Changing the section ordering or making it user-configurable.

## 6. Design Considerations

- **Header style:** bold, slightly larger than the `font_point_size` used by results, with vertical padding only enough to fit the title line. No frame, no background fill, no divider line — keep it minimal so the list still reads as a continuous list with light typographic separation.
- **Spacing:** headers should sit flush with the items below them (no extra gap), matching the existing `ListView { spacing: 0 }` rhythm.
- **Dark/light mode:** header text should use `root.palette.active.text`, the same color as result text, so it adapts to theme automatically.
- **Mobile:** header height and font size should scale with the existing `is_mobile` font sizing pattern (`font_point_size: root.is_mobile ? 14 : 11`).

## 7. Technical Considerations

- The new `is_section_header: bool` field must be added to the Rust result struct used by `fetch_combined_page` and serialized via the existing JSON path that crosses into QML. Verify the struct used in `bridges/src/sutta_bridge.rs` and `bridges/src/api.rs` (`/dict_combined_search`) and update both if they share a type, or both if they're parallel types.
- All non-Combined call sites that produce result structs must populate `is_section_header = false` (use `#[serde(default)]` to keep deserialization stable on older payloads if applicable).
- The QML `ListModel` requires consistent role keys across appended items. Always append every role on every row (header or real), with empty strings/zeros where unused.
- Keyboard-skip logic in `FulltextResults.qml` should be implemented as a small helper (e.g. `next_selectable_index(from, direction)`) and reused by `select_previous_result()`, `select_next_result()`, and the `Keys.onPressed` handler to avoid drift.
- The Combined cache (`COMBINED_CACHE` in `sutta_bridge.rs`) should continue to store **real** sub-query results only. Header insertion happens at page-assembly time inside `fetch_combined_page`, after the merged real-result slice is computed.

## 8. Success Metrics

- Visual inspection: Combined Mode dictionary searches show the two labeled sections on every page that contains the corresponding results.
- Pagination correctness: `Page X of Y` and prev/next behavior is identical to pre-feature behavior for the same query (verified against a query that produces multiple pages).
- Keyboard navigation: pressing Down through a full Combined page lands on every real result exactly once and never on a header.
- No regressions in non-Combined dictionary searches or in sutta searches (their result lists remain header-free).

## 9. Open Questions

None remaining — all clarifying decisions have been folded into the requirements above.
