# Tasks — Integrate StarDict Filtering Across All Dictionary Search Modes

Source PRD: [prd-integrate-stardict-filtering.md](./prd-integrate-stardict-filtering.md)

## Relevant Files

- `backend/src/types.rs` — `SearchMode::Combined` already exists in the enum (line 65); no schema change. `SearchParams.dict_source_uids` (line 108) is already wired. Reused by every dict mode handler.
- `backend/src/query_task.rs` — Per-mode dispatch + `apply_dict_source_uids_filter` (line 2084). Houses `dict_words_contains_match_fts5_full` (line 741), `lemma_1_dpd_headword_match_fts5_full` (line 1632), `dpd_lookup_full` (line 1380), `fulltext_dict` (line 1783), `results_page` (line 1998).
- `backend/src/search/searcher.rs` — `add_dict_filters` (line 574) already pushes `dict_source_uids` down to Tantivy for Fulltext. No behavioural change needed here; only add a constructor convenience if the SQL paths benefit from sharing the inclusion-set type.
- `backend/src/app_settings.rs` — Add `dict_search_last_mode: Option<String>` and accessors. Mirrors existing `dict_search_dpd_enabled` pattern.
- `backend/src/app_data.rs` — Add `get_last_dict_search_mode()` / `set_last_dict_search_mode(mode)` helpers around the new setting.
- `bridges/src/dictionary_manager.rs` — Expose two QObject methods: `get_last_dict_search_mode()` and `set_last_dict_search_mode(mode)`.
- `bridges/src/sutta_bridge.rs` — Result-cache and `results_page` entry-point are reused by the parallel Combined orchestrator without backend changes; no edits expected, but verify the cache key already includes `mode` (`SuttaBridge::results_page` line ~1302).
- `assets/qml/com/profoundlabs/simsapa/DictionaryManager.qml` — qmllint stubs for the two new methods.
- `assets/qml/SearchBarInput.qml` — `search_mode_dropdown` (line 226). Add `Combined` to the Dictionary lists and wire default + persistence.
- `assets/qml/SuttaSearchWindow.qml` — `get_search_params_from_ui` (line 526), `results_page` orchestration (line 511), `compute_dict_search_filter` (line 583), and the `FulltextResults` mount point (line 2767). The Combined orchestrator lives here so it can fan out two parallel `SuttaBridge.results_page` calls and gate render on both.
- `assets/qml/FulltextResults.qml` — Where the result list and pager are currently rendered; needs a "two-section" rendering mode for Combined (DPD section + Fulltext section, each with its own pager). Alternative: introduce a thin parent `CombinedDictResults.qml` that owns two `FulltextResults` instances side by side; preferred to keep `FulltextResults` single-section.
- `assets/qml/CombinedDictResults.qml` — **NEW** thin wrapper that holds two `FulltextResults` (DPD on top, Fulltext below) plus the orchestration state (`pending_dpd`, `pending_fulltext`, render gate).
- `bridges/build.rs` — Register `CombinedDictResults.qml` in `qml_files`.
- `backend/tests/dict_modes_filtering.rs` — **NEW** integration tests for the Contains and Headword retrieval changes (uses the local appdata DB; no `#[ignore]`).
- `backend/src/query_task.rs` (tests block at the bottom) — Unit-style tests for `apply_dict_source_uids_filter` cases the new push-down should not regress.

### Notes

- All filesystem existence checks (none expected here) must use `try_exists()` per CLAUDE.md.
- New QML files must be added to `bridges/build.rs::qml_files`.
- After each top-level task the project must compile cleanly with `make build -B`.
- Per project memory: do not run `make qml-test`; only run tests after the full top-level task is done; do not bulk-`sed` to rename symbols.
- Architecture note: **Combined is UI-orchestrated** — the backend explicitly rejects `SearchMode::Combined + SearchArea::Dictionary` so accidental backend invocations fail loudly. The two parallel sub-queries reuse the existing `DpdLookup` and `FulltextMatch` `results_page` paths and their native pagination contracts (materialise-then-slice for DPD; Tantivy push-down for Fulltext).

## Tasks

- [ ] 1.0 Backend: extend `ContainsMatch` + Dictionary to retrieve user-imported `dict_words` rows via `dict_label IN (set)`, restoring `total` accuracy
  - [ ] 1.1 Re-read `dict_words_contains_match_fts5_full` (`backend/src/query_task.rs:741`) and locate Phase 3 (the `dict_words_fts.definition_plain LIKE` block, ~line 889). Confirm `dict_label` is in the FTS table by checking `scripts/dictionaries-fts5-indexes.sql`. If `word` is not in `dict_words_fts`, plan to use a `dict_words` table scan filtered by `dict_label IN (...)` instead — note this in the function-level doc comment.
  - [ ] 1.2 Add a private helper `fn dict_label_in_clause(set: &[String]) -> Option<(String, Vec<String>)>` returning the placeholder string `"?, ?, …"` and the bind values, or `None` when the set is empty (caller skips the phase entirely). Place it near the top of `query_task.rs` impl alongside `normalized_filter`.
  - [ ] 1.3 Replace Phase 3 (definition_plain) with a unified `dict_label IN (set)`-aware path:
        - When `self.dict_source_uids` is `Some(set)` and `set.is_empty()`, skip the phase (no rows).
        - When `self.dict_source_uids` is `Some(set)` non-empty, build the SQL with `f.dict_label IN (?, ?, …)` AND `f.definition_plain LIKE ?` (plus existing `d.uid LIKE ?` push-downs).
        - When `None`, fall back to today's behaviour (no `dict_label` clause).
        - Honour `self.source` / `self.source_include` as before — apply both clauses additively.
  - [ ] 1.4 Add a new **Phase 5: user-headword (`dict_words.word`) substring** — directly against `dict_words` filtered by `dict_label IN (set)` and `word LIKE '%term%'` (case-insensitive via SQLite `COLLATE NOCASE` on `word`, or by ANDing on a normalised token if available). Cap at `SAFETY_LIMIT_SQL`. Skip when the set is empty. This phase plugs the user-headword retrieval gap that today only flowed through DPD lemmas.
  - [ ] 1.5 Deduplicate across phases by `dict_words.id` (existing dedup uses `result.word` as a key; switch to `id` for correctness across multi-label collisions). Preserve order: DPD-driven Phases 1+2+4 first, then unified Phase 3, then Phase 5.
  - [ ] 1.6 Because retrieval is now restricted to the inclusion set, `apply_dict_source_uids_filter` (line 2084) becomes a no-op for this mode but must remain in the dispatcher as a safety net. Add a debug-level log `info!("dict_source_uids post-filter dropped {} rows", dropped)` so any non-zero drop on Contains is loud.
  - [ ] 1.7 Update the function-level rustdoc on `dict_words_contains_match_fts5_full` to document the new phase numbering and the `dict_source_uids` push-down. Keep the `total = full.len()` materialise-then-slice contract documented.
  - [ ] 1.8 Run `make build -B`. Compilation must succeed with no new warnings introduced by Phases 3/5.

- [ ] 2.0 Backend: extend `HeadwordMatch` + Dictionary to merge a user-headword path against `dict_words.word`, alongside the existing DPD `lemma_1` path
  - [ ] 2.1 Re-read `lemma_1_dpd_headword_match_fts5_full` (`backend/src/query_task.rs:1632`). The DPD-only path resolves DPD `lemma_1` hits to a single `dict_words` row by `word == lemma_1`. Plan: keep this path conditional on `"dpd"` being in the inclusion set (or `dict_source_uids` being `None`), and add a parallel user-headword path for the rest of the set.
  - [ ] 2.2 Rename the function to `headword_match_full` and add a thin `lemma_1_dpd_headword_match_fts5_full` shim that calls it, only if any external caller still references the old name. Confirm via `grep -rn lemma_1_dpd_headword_match_fts5_full backend/ bridges/`. If the only callers are inside `query_task.rs`, rename in place per project memory's no-bulk-sed rule (use targeted Edit calls).
  - [ ] 2.3 Compute the "non-DPD" subset of the inclusion set: `set.iter().filter(|s| s != "dpd").collect::<Vec<_>>()`. When `dict_source_uids` is `None`, treat both paths as enabled.
  - [ ] 2.4 Add a "Path B: user-headword" SQL query against `dict_words` filtered by `dict_label IN (non_dpd_set)` and `word LIKE '%term%'`, with the existing `uid_prefix_pat` / `uid_suffix_pat` push-downs. Skip when `non_dpd_set` is empty. Cap at `SAFETY_LIMIT_SQL`.
  - [ ] 2.5 Merge Path A (DPD) and Path B (user-headword) — deduplicate by `dict_words.id`. Sort: exact-`word == term` rows first, then contains rows; tie-break by `dict_label` then `id` so result order is stable.
  - [ ] 2.6 Confirm `headword_match_with_bold` (line 1927) still composes correctly with the renamed helper — the bold branch is independent and must not double-count.
  - [ ] 2.7 As in §1.6, the `apply_dict_source_uids_filter` post-filter is now a safety net; log non-zero drops for surveillance.
  - [ ] 2.8 Update the rustdoc on `headword_match_full` to describe both paths, the merge, and pagination (`total = full.len()` materialise-then-slice unchanged).
  - [ ] 2.9 Run `make build -B`.

- [ ] 3.0 Backend: explicitly reject `SearchMode::Combined + SearchArea::Dictionary` in `results_page`
  - [ ] 3.1 In `query_task.rs::results_page` (line 1998), replace the `SearchMode::Combined => (Vec::new(), 0)` arm with an explicit error path: when `search_area == SearchArea::Dictionary`, return `Err("SearchMode::Combined is UI-orchestrated; dictionary searches must dispatch DpdLookup and FulltextMatch separately".into())`.
  - [ ] 3.2 For `SearchMode::Combined + (SearchArea::Suttas | SearchArea::Library)`, dispatch to the existing `FulltextMatch` handler (Combined falls back to Fulltext for non-Dictionary areas per PRD §5.4.13). Implement this as a `let mode = if search_mode == Combined { FulltextMatch } else { search_mode };` shadow at the top of the dispatch when not on Dictionary.
  - [ ] 3.3 Update `bridges/src/sutta_bridge.rs::results_page` to surface the error gracefully — the existing error-string path through `results_page_ready` is sufficient; do not panic.
  - [ ] 3.4 Add a unit test in `query_task.rs` (or `backend/tests/dict_modes_filtering.rs`) asserting that `Combined + Dictionary` returns `Err`, and that `Combined + Suttas` returns the same shape as `FulltextMatch + Suttas` for a known query.
  - [ ] 3.5 Run `make build -B`.

- [ ] 4.0 Settings + bridge: persist last-used dictionary search mode (`dict_search.last_mode`, default `"Combined"`)
  - [ ] 4.1 Add `dict_search_last_mode: Option<String>` to the `AppSettings` struct in `backend/src/app_settings.rs` with `#[serde(default)]`. Default value is `None`; readers treat `None` as `"Combined"`.
  - [ ] 4.2 Add `get_last_dict_search_mode() -> String` (returns `"Combined"` when unset) and `set_last_dict_search_mode(mode: &str)` on `AppData` in `backend/src/app_data.rs`. Use the same `persist_app_settings` helper pattern used by `set_dpd_enabled`.
  - [ ] 4.3 Add `get_last_dict_search_mode(&self) -> QString` and `set_last_dict_search_mode(&self, mode: QString)` to the `DictionaryManager` bridge (`bridges/src/dictionary_manager.rs`). No mutex required — settings writes are not on the import/rename/delete critical path.
  - [ ] 4.4 Update the qmllint stub `assets/qml/com/profoundlabs/simsapa/DictionaryManager.qml` with placeholder implementations of the two new methods.
  - [ ] 4.5 Run `make build -B`.

- [ ] 5.0 QML dropdown: add `Combined` to the Dictionary search-mode dropdown, default-select it, and restore/persist via `DictionaryManager`
  - [ ] 5.1 In `assets/qml/SearchBarInput.qml::search_mode_dropdown` (line 226), update both `search_mode_label_wide.Dictionary` (line 241) and `search_mode_label_narrow.Dictionary` (line 263) lists:
        - Wide: `["Combined", "DPD Lookup", "Fulltext Match", "Contains Match", "Headword Match"]`
        - Narrow: `["Combined", "Lookup", "Fulltext", "Contains", "Headword"]`
  - [ ] 5.2 On `search_area === "Dictionary"`, set `currentIndex` to the index of `DictionaryManager.get_last_dict_search_mode()` in the wide list (default `"Combined"` → index 0). Do this in a `Connections { target: root; function onSearch_areaChanged() { ... } }` block or the existing search-area handler.
  - [ ] 5.3 On `currentIndex` change while `search_area === "Dictionary"`, call `DictionaryManager.set_last_dict_search_mode(get_text())`. Fold this into the existing `onCurrentIndexChanged` handler (line 279) without disturbing the `handle_query_fn` call.
  - [ ] 5.4 Verify other areas (Suttas, Library) keep their existing default (`currentIndex = 0`, "Fulltext Match") — adding the persisted setting must not affect them.
  - [ ] 5.5 Run `make build -B`. Manual sanity check via build only (no GUI): the QML registers and qmllint passes.

- [ ] 6.0 QML Combined orchestrator: parallel DPD + Fulltext queries, single render gate, two stacked pagers
  - [ ] 6.1 Create `assets/qml/CombinedDictResults.qml` with:
        - `property string query_text`
        - `property var dict_filter` (object: `{ dict_source_uids, include_comm_bold_definitions }`)
        - `property int dpd_page_num: 0`
        - `property int fulltext_page_num: 0`
        - `property bool pending_dpd: false`
        - `property bool pending_fulltext: false`
        - `property var dpd_results: []` (cached page)
        - `property var fulltext_results: []` (cached page)
        - `property int dpd_total: 0`
        - `property int fulltext_total: 0`
        - `property int page_len: 10`
        - Two `FulltextResults` children stacked DPD-first / Fulltext-second, each with its own section header label.
  - [ ] 6.2 Add `function start_combined_search(query_text, page_num_dpd, page_num_fulltext)` that:
        1. Sets `pending_dpd = true; pending_fulltext = true; visible_results = []` (so nothing is shown until both complete).
        2. Builds two `params` objects, one with `mode: "DPD Lookup"` and one with `mode: "Fulltext Match"`, both carrying the same `dict_source_uids` / `include_comm_bold_definitions` as the parent's `dict_filter`.
        3. Fires both `SuttaBridge.results_page(query_text, dpd_page_num, "Dictionary", JSON.stringify(dpd_params))` and the equivalent fulltext call. Use the existing async result-ready signal channel.
  - [ ] 6.3 Wire two `Connections { target: SuttaBridge }` listeners (or one with mode-disambiguation), recording `dpd_results` / `fulltext_results` and clearing the corresponding `pending_*` flag. The existing `results_page_ready` signal must carry enough context to disambiguate which sub-query returned — confirm and, if not, plumb an opaque `request_id` through the bridge (or piggyback on the cached params blob, since the bridge cache keys by `(query, mode, area, params)`).
  - [ ] 6.4 Render gate: a `function maybe_render()` checks `!pending_dpd && !pending_fulltext` and only then assigns `dpd_results` / `fulltext_results` to the two `FulltextResults` children. Until then, the visible state is unchanged (no jumping). On error from one side, render the other and surface a warning bar.
  - [ ] 6.5 Two independent pagers: each `FulltextResults` instance binds its `page_num` and `total` properties to `dpd_*` and `fulltext_*` respectively. Pager "next/prev" buttons call `start_combined_search(query_text, new_dpd_page, fulltext_page_num)` (and the symmetric variant) so re-fetches still go through the gate. Re-fetching one side still triggers `pending_*` for that side only — but render still waits until both sides are idle to avoid mid-page redraws.
  - [ ] 6.6 In `SuttaSearchWindow.qml`, mount `CombinedDictResults` as a sibling of the current `FulltextResults` (line 2767). Visibility logic: when `search_area === "Dictionary"` and the current dict mode is `"Combined"`, show `CombinedDictResults` and hide `FulltextResults`; otherwise hide it.
  - [ ] 6.7 Update `get_search_params_from_ui` (line 526) so when `mode === "Combined"` it does **not** dispatch through `results_page` at all — instead invoke `combined_dict_results.start_combined_search(query_text, 0, 0)`. Adjust `handle_search` / `results_page` paths accordingly.
  - [ ] 6.8 Register `CombinedDictResults.qml` in `bridges/build.rs::qml_files`.
  - [ ] 6.9 Verify cache interplay: re-running the same Combined query within session must hit the bridge result-cache for both sub-queries (no re-execution). Confirm by reading `bridges/src/sutta_bridge.rs` cache key construction.
  - [ ] 6.10 Run `make build -B`. Compilation + qmllint must succeed.

- [ ] 7.0 Tests + final `make build -B` + docs/PROJECT_MAP update
  - [ ] 7.1 Add `backend/tests/dict_modes_filtering.rs` with tests against the local appdata DB:
        - `contains_match_includes_user_dict_word_only_in_set`: search for a token present only in a user-imported dict's `word`; assert ≥1 result with the user dict in the inclusion set, 0 with it removed.
        - `contains_match_includes_user_dict_definition_only_in_set`: same for `definition_plain`.
        - `headword_match_includes_user_dict_word`: solo a user dict; expect only that dict's headword. With set `["dpd"]` only, expect zero user-dict rows.
        - `dpd_lookup_unaffected_by_user_dict_toggle`: toggling user dict checkboxes does not change DPD Lookup output.
        - `combined_mode_dictionary_returns_err`: `SearchMode::Combined + SearchArea::Dictionary` returns `Err` from `results_page`.
        - `combined_mode_suttas_falls_back_to_fulltext`: `Combined + Suttas` matches `FulltextMatch + Suttas` for a known query.
  - [ ] 7.2 Add a `query_task.rs` unit test verifying `apply_dict_source_uids_filter` is a no-op when retrieval already restricted by `dict_label IN (set)` (i.e. zero drops, no `total` decrement).
  - [ ] 7.3 Run `cd backend && cargo test`. Per project memory, ignore pre-existing failures; flag only newly introduced ones.
  - [ ] 7.4 Run `make build -B` one final time after all sub-tasks complete.
  - [ ] 7.5 Update `PROJECT_MAP.md`: new `Combined` mode (UI-orchestrated), new `CombinedDictResults.qml`, the `dict_label IN (set)` push-down on Contains/Headword, the persisted `dict_search.last_mode` setting and bridge methods, and the rename of `lemma_1_dpd_headword_match_fts5_full` → `headword_match_full` (if applicable).
  - [ ] 7.6 Update `docs/` with a brief user-facing note: "Combined" is the new default dictionary mode and shows DPD lookups followed by Fulltext matches; the two are computed in parallel and the page renders only when both are ready, so the result list never jumps. The Dictionaries panel checkboxes / lock affect Combined, Fulltext, Contains, and Headword. DPD Lookup remains DPD-only by design.
