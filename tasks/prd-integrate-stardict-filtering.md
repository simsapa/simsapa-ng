# PRD: Integrate User-Imported StarDict Filtering Across All Dictionary Search Modes

Source feature this builds on: [prd-stardict-dictionary-import.md](./prd-stardict-dictionary-import.md) and its task list [tasks-prd-stardict-dictionary-import.md](./tasks-prd-stardict-dictionary-import.md).

## 1. Background

Task 8.0 of the StarDict-import feature wired the per-dictionary checkbox / lock selection (built-in **DPD**, built-in **Commentary Definitions**, and each **user-imported** dictionary) into the dictionary search query layer. As shipped, the wiring works correctly only for `SearchMode::FulltextMatch` on `SearchArea::Dictionary` — i.e. `fulltext_dict` in `backend/src/query_task.rs`. The other dictionary search modes either ignore user-imported rows entirely or only partially honour the inclusion set.

This PRD plans the work needed to extend the same selection / solo logic to every dictionary search mode, with one deliberate exception: **DPD Lookup must remain DPD-only**. It also introduces a new default mode, **Combined**, which is a **UI-level orchestration** that fires `DpdLookup` and `FulltextMatch` in parallel and renders both blocks together once both complete. Combined replaces `DPD Lookup` as the default selection in the search-mode dropdown.

## 2. Findings — How filtering works today

### 2.1 The selection contract (already in place)

- `SearchParams.dict_source_uids: Option<Vec<String>>` (`backend/src/types.rs:108`) carries the inclusion set; `None` = no constraint, `Some([])` = no `dict_words` rows match, `Some([labels…])` = those `dict_label`s only.
- `SearchParams.include_comm_bold_definitions: bool` independently gates the bold-definition rows.
- `SuttaSearchWindow.qml::compute_dict_search_filter()` (`assets/qml/SuttaSearchWindow.qml:583`) builds these two values from the Dictionaries panel state (DPD checkbox, Commentary Definitions checkbox, per-user-dict checkboxes, lock target).
- The selection-set is therefore already correctly assembled in QML before any backend call.

### 2.2 Where it is honoured today

- **Tantivy / Fulltext path** — `backend/src/search/searcher.rs::add_dict_filters` (line 574) consumes both `filters.dict_source_uids` and `filters.include_bold_definitions` and pushes the constraint down to the dict index. This is what makes Fulltext work: the inclusion set decides which `source_uid` terms are MUST'd into the query, and bold rows are gated independently.
- **Generic SQL post-filter** — `query_task.rs::apply_dict_source_uids_filter` (line 2084) is invoked by `results_page` (line 2065) for every non-Fulltext dictionary mode. It drops any `dict_words` row whose `source_uid` (== `dict_label`) is not in the inclusion set. Bold-definition rows are passed through untouched.

### 2.3 Why other modes don't behave as the user expects

The post-filter works on whatever rows the per-mode handler **returned**. The handlers themselves push only `self.source` (the legacy single-string source filter) into SQL — they do not retrieve user-imported rows that happen to lie outside the legacy filter, and several of them are structurally DPD-only:

1. **`ContainsMatch` + Dictionary** (`dict_words_contains_match_fts5_full`, line 741):
   - Phases 1, 2, 4 look up matches in `dpd_headwords` first, then resolve to `dict_words` only via `dict_words.word == headword.lemma_1`. User-imported dicts whose words do not exist in DPD are invisible to these phases.
   - Phase 3 does match user dicts (it scans `dict_words_fts.definition_plain`), so user dicts contribute a partial subset of results.
   - Net effect: the inclusion set is honoured *as a filter* but user-dict matches are systematically under-recalled.

2. **`HeadwordMatch` + Dictionary** (`lemma_1_dpd_headword_match_fts5_full`, line 1632):
   - Drives the search entirely from `dpd_headwords_fts.lemma_1`, then resolves each hit to a single `dict_words` row by exact `word == lemma_1`. User-imported headwords that are not DPD lemmas never appear.
   - Net effect: a user-imported dictionary checkbox toggles nothing — the mode is structurally DPD-only.

3. **`DpdLookup` + Dictionary** (`dpd_lookup_full`, line 1380):
   - Pure DPD. Per the user's directive, this is intentional and **must remain so**.

4. **`UidMatch` + Dictionary**: searches across SQL by uid prefix; the post-filter handles the inclusion set correctly, no change required.

5. **Bold-definition rows** are folded in via the per-mode `*_with_bold` wrappers (`dict_contains_with_bold` / `headword_match_with_bold` / `dpd_lookup_with_bold`). They are gated by `include_comm_bold_definitions`, which `compute_dict_search_filter` already drives correctly.

### 2.4 The `Combined` mode

`SearchMode::Combined` exists in `backend/src/types.rs:65` but is not yet implemented (`results_page` returns `(Vec::new(), 0)` for it, line 2049) and is not exposed in the `assets/qml/SearchBarInput.qml` dropdown.

### 2.5 Pagination today (per mode)

How `total` and `page_len` are produced today, since changes to retrieval interact directly with pagination:

- **`fulltext_dict`** (Tantivy): `searcher.search_dict_with_count(query, filters, page_len, page_num)` pushes pagination *down* — Tantivy returns the requested page along with a true `total_hits`. No materialisation. The `dict_source_uids` MUST'd into the query already restricts the corpus, so `total` is post-filter accurate.
- **`fulltext_suttas` / `fulltext_library`**: same shape, pagination down to Tantivy.
- **`dict_words_contains_match_fts5`** (Contains + Dictionary): `dict_words_contains_match_fts5_full()` materialises the union of phases 1–4 into `Vec<DictWord>` (capped at `SAFETY_LIMIT_SQL`), then the per-page wrapper slices `[offset .. offset+page_len]`. `total = full.len()`.
- **`lemma_1_dpd_headword_match_fts5`** (Headword + Dictionary): same shape — materialise `_full`, slice. `total = full.len()`.
- **`dpd_lookup`** (DPD Lookup + Dictionary): same shape — `dpd_lookup_full` materialises the multi-phase fallback union, slice. `total = full.len()`.
- **`*_with_bold` wrappers**: these compose the regular `_full` with bold rows, pagewise. They preserve the materialise-then-slice pattern.
- **`uid_match`**: materialised list, slice.
- **Generic post-filter `apply_dict_source_uids_filter`** (`query_task.rs:2084`): runs *after* the per-mode handler returned its page-sliced output. It decrements `total` only by the number of dropped rows that happened to land on the current page — it does **not** know how many dropped rows exist across all pages. Concretely:
  - The page-sliced result is correct (correct rows shown).
  - `total` returned to the UI is **inaccurate** when filtering drops rows: the page count / pager controls based on `total / page_len` will overestimate the number of pages, and later pages may be empty after filtering. This is a pre-existing issue for non-Tantivy dict modes whenever the user toggles off any dictionary with results.

### 2.6 Pagination implications of the §3 changes

- Adding a `dict_words_fts` retrieval phase to `ContainsMatch` and `HeadwordMatch` increases `_full` length but keeps the materialise-then-slice contract. `total` becomes the new union length, which is consistent with the page contents because the union is built **with** `dict_label IN (set)` pushed into the SQL — so the post-filter has nothing to drop and the `total` mismatch from §2.5 disappears for these modes.
- For `DpdLookup` (unchanged), the post-filter becomes a no-op when `"dpd"` is in the set; when the user solos a non-DPD dictionary the page is empty but `total` is still DPD's full length — this is the same pre-existing inconsistency. We accept it: soloing a non-DPD row under DPD-only mode is an unusual configuration, and the user-facing fix is to switch modes rather than expect DPD Lookup to honour user dicts.
- `Combined` is redesigned (see §5.4) to orchestrate two parallel queries from the UI, each carrying its own pagination cursor — there is no single `total` for the union.

## 3. Goals

1. Make the per-dictionary checkbox + lock state genuinely affect results in `ContainsMatch` and `HeadwordMatch` for `SearchArea::Dictionary` — not only as a post-filter on a DPD-driven candidate set, but by retrieving matches from each enabled dictionary in the first place.
2. Keep `DpdLookup` strictly DPD-only — it is the user's "DPD canonical view" mode and must not be diluted with user-imported entries.
3. Add `Combined` as the new default dictionary search mode. Combined is **UI-orchestrated**: the dictionary results view fires a `DpdLookup` query and a `FulltextMatch` query in parallel, waits for both to complete, and renders DPD results first followed by Fulltext results in a single render pass — no progressive jumping. Each block keeps its own pagination cursor.
4. Preserve current Fulltext behaviour exactly (it already works).
5. Keep bold-definition gating independent and consistent across every mode.

## 4. Non-Goals

- Changing how the Dictionaries panel is built or how `compute_dict_search_filter` decides what to send. Backend is the only changing surface.
- Adding new search modes beyond `Combined`.
- Cross-mode UID/result ranking changes beyond what `Combined` requires.
- Indexer / reconciliation changes — user-imported dictionaries are already indexed into both Tantivy and `dict_words_fts`.

## 5. Functional Requirements

### 5.1 `ContainsMatch` + Dictionary

1. Retrieval must include matches from every dictionary in `dict_source_uids` (or every dictionary, when `None`), not only those whose `word` matches a DPD `lemma_1`.
2. Add a new retrieval phase backed by `dict_words_fts` filtered on `dict_label IN (set)` (or unconstrained if `None`):
   - Match `dict_words_fts.word LIKE '%term%'` and `dict_words_fts.definition_plain LIKE '%term%'`, deduplicated by `dict_words.id`.
   - Push `dict_label IN (...)` down into the SQL `WHERE` clause as a parameterised list. When the list is empty, skip this phase entirely (zero contribution).
3. The existing DPD-driven phases (1, 2, 4) remain — they still surface DPD lemma hits — but their result rows must continue to flow through `apply_dict_source_uids_filter`. When the user solos a non-DPD dictionary, DPD-driven phases will yield zero rows after filtering, which is the correct behaviour.
4. Result ordering within the page: keep the current "exact lemma → contains lemma → definition" precedence for DPD; append user-dict word/definition hits after them, ordered by `dict_words.id`.
5. The bold-definition append (`dict_contains_with_bold`) is unchanged.

### 5.2 `HeadwordMatch` + Dictionary

6. Headword matching must consider the **headword field of every selected dictionary**, not only DPD's `lemma_1`. For user-imported StarDict dictionaries, the headword is `dict_words.word`.
7. Add an SQL path that selects `dict_words` rows where `dict_label IN (set)` and `word LIKE '%term%'` (case-insensitive). Skip when the set is empty. Cap at `SAFETY_LIMIT_SQL`.
8. Merge with the existing DPD `lemma_1` path:
   - DPD hits (via `dpd_headwords_fts`) resolve to `dict_words` rows whose `dict_label = "dpd"` — only when `"dpd"` is in the inclusion set or the set is `None`.
   - User-dict hits come straight from `dict_words.word` for the rest of the inclusion set.
   - Deduplicate by `dict_words.id`. Sort: exact-word hits first, then contains hits, each block tie-broken by `dict_label` then `dict_words.id`.
9. The bold-definition append (`headword_match_with_bold`) is unchanged.

### 5.3 `DpdLookup` + Dictionary

10. **No change**. `dpd_lookup_full` continues to return DPD-only matches. The post-filter `apply_dict_source_uids_filter` already drops nothing because every DPD result has `source_uid = "dpd"` and DPD is ordinarily in the inclusion set; in the rare case where the user solos a non-DPD dictionary, the post-filter correctly returns zero rows.
11. The Dictionaries panel still drives the `include_comm_bold_definitions` flag, which `dpd_lookup_with_bold` honours.

### 5.4 `Combined` mode (UI-orchestrated, two parallel queries)

`Combined` is **not a backend mode that materialises a union**. It is a UI-level orchestration that fires two independent queries — `DpdLookup` and `FulltextMatch` — at the same time, each through the existing single-mode `results_page` bridge entry-point, and renders them as two stacked sections in the dictionary results view. This keeps each sub-query's pagination native to its own backend handler (Tantivy push-down for fulltext, materialise-and-slice for DPD), avoids re-paginating across a deduped union, and exploits both backends in parallel for time-to-first-page.

**Backend.**
12. `SearchMode::Combined` continues to be **rejected** by `results_page` for `SearchArea::Dictionary` — the bridge returns an explicit error for that combination so any accidental call surfaces immediately. It is not implemented as a per-mode handler.
13. For `SearchArea::Suttas` and `SearchArea::Library`, `Combined` falls back to `FulltextMatch` at the QML layer (we are not introducing a Suttas-side combined mode in this PRD).

**UI orchestration (QML).**
14. When the user issues a Dictionary search with mode `Combined`, the dictionary results view fires **two** `SuttaBridge.results_page` calls in parallel:
    1. `params.mode = "DPD Lookup"` — sub-query A.
    2. `params.mode = "Fulltext Match"` — sub-query B.
    Both calls receive the **same** `dict_source_uids` and `include_comm_bold_definitions` from `compute_dict_search_filter`, the same query text, and the same `page_num`. (DPD Lookup ignores `dict_source_uids` for retrieval but the post-filter still runs; Fulltext honours it natively.)
15. The view **must wait for both sub-queries to resolve** before rendering. There is no progressive render. The point is to avoid the result list "jumping" when the slower query lands.
    - Use `Promise.all`-equivalent in QML: track two pending request ids and a small reducer that fires `render_combined()` only when both have returned.
    - If either sub-query errors, render the one that succeeded and surface a non-fatal warning bar describing which side failed.
16. The two result blocks are concatenated **DPD first, Fulltext second**, separated by a section header (a small `Label` row "DPD" / "Fulltext", styled like existing section dividers).
17. Each block carries its own pager footer with its own `total` and `page_len`:
    - DPD pager controls `dpd_page_num`; Fulltext pager controls `fulltext_page_num`. They advance independently.
    - When the user changes the search query, both `page_num`s reset to 0.
    - When the user changes mode away from `Combined`, both pagers are discarded.
18. There is **no global "total"** for Combined. The combined view shows two pager labels: `DPD: page X of Y` and `Fulltext: page X of Y`. This avoids the page-count rounding lies that would otherwise come from summing a count from a post-filtered handler with one from a push-down handler.
19. Dedup across the two blocks is **not** performed. The two sections are conceptually different lookups; a row appearing in both is informative ("DPD has it as a lemma; Fulltext also matched the definition body"). If user feedback later asks for dedup we can add a post-render filter — but not in v1.
20. Caching: the existing per-call result cache in `bridges/src/sutta_bridge.rs` keys by `(query, mode, area, params)` so the two parallel calls naturally cache as two separate entries. Re-clicking the same combined query hits cache for both.

**Why parallel rather than sequential.**

The naïve "render DPD first, then fetch Fulltext" path would show a partial first page that grows when Fulltext returns. The user explicitly does not want that — the contract is "first page shown fast" via two concurrent backends, then a single render of both blocks together. Time-to-first-page is `max(dpd_latency, fulltext_latency)` rather than `dpd_latency + fulltext_latency`.

**Pagination semantics, summarised.**

| Sub-query  | Pagination source                        | `total` accuracy |
|------------|------------------------------------------|------------------|
| DPD Lookup | `dpd_lookup_full` materialise + slice    | exact (modulo the post-filter caveat from §2.5, which doesn't bite here because `"dpd"` is in the set when DPD is enabled) |
| Fulltext   | Tantivy `search_dict_with_count`         | exact (push-down) |

Each sub-query honours its native pagination contract; nothing in `Combined` re-paginates a union.

### 5.5 QML wiring

14. `assets/qml/SearchBarInput.qml::search_mode_dropdown`:
    - For `search_area === "Dictionary"`, the model becomes:
      ```
      ["Combined", "DPD Lookup", "Fulltext Match", "Contains Match", "Headword Match"]
      ```
      with narrow-screen labels:
      ```
      ["Combined", "Lookup", "Fulltext", "Contains", "Headword"]
      ```
    - The default `currentIndex` for Dictionary becomes `0` (Combined). Today the dropdown remembers its own `currentIndex` only by position; switching to a new default requires adjusting the initial `currentIndex` when `search_area === "Dictionary"` and there is no persisted user choice yet.
15. The JSON sent to the backend uses the wide label string (existing `get_text()` contract). The new value is the literal `"Combined"`, which already matches `#[serde(rename = "Combined")]` is unnecessary because the Rust enum variant `Combined` deserialises by name.
16. No change to `compute_dict_search_filter` — its output already feeds through `dict_source_uids` and `include_comm_bold_definitions` which every backend handler will read.

### 5.6 Persisted default

17. Persist the user's last-used dictionary search mode under a new `app_settings` key `dict_search.last_mode` (default `"Combined"`). On `SuttaSearchWindow` open, when `search_area === "Dictionary"`, restore that index. When the user switches to Dictionary from another area, restore as well. Switching the dropdown updates the setting.
18. Migration: existing installs do not have the key. The default kicks in on first read. No data migration required.

## 6. Behavioural Matrix (target state)

| Mode            | DPD checkbox | Commentary checkbox | User-imported checkboxes | Solo any row | Notes                                    |
|-----------------|--------------|---------------------|--------------------------|--------------|------------------------------------------|
| Combined        | ✔ filters    | ✔ filters bold       | ✔ filters                | ✔            | Default. UI fires DPD + Fulltext in parallel; renders both blocks once both return; independent pagers per block. |
| DPD Lookup      | (always on)  | ✔ filters bold       | (ignored)                | ✔ (DPD-only) | Stays DPD-only.                          |
| Fulltext Match  | ✔ filters    | ✔ filters bold       | ✔ filters                | ✔            | Already works. No change.                |
| Contains Match  | ✔ filters    | ✔ filters bold       | ✔ filters (NEW retrieval) | ✔            | Adds dict_words_fts retrieval phase.     |
| Headword Match  | ✔ filters    | ✔ filters bold       | ✔ filters (NEW retrieval) | ✔            | Adds dict_words.word retrieval path.     |
| Uid Match       | ✔ filters    | n/a                  | ✔ filters                | ✔            | Post-filter is sufficient.               |

"Solo X" means: when `compute_dict_search_filter` returns a single-element `dict_source_uids` set, that mode returns matches only from that dictionary (or zero matches if the mode is structurally incompatible — e.g. soloing a user dictionary under DPD Lookup yields zero results, which is the documented behaviour).

## 7. Technical Considerations

- **No schema or index changes.** `dict_words_fts` is auto-synced via the SQLite triggers in `scripts/dictionaries-fts5-indexes.sql`; user-imported rows are already indexed in both Tantivy and FTS5 after the startup reconciliation pass.
- `dict_words_fts` exposes `word`, `definition_plain`, and `dict_label` — sufficient for both the Contains and Headword retrieval paths. Verify in `scripts/dictionaries-fts5-indexes.sql` before implementing; if `word` is not in the FTS table, fall back to a `dict_words` table scan filtered on `dict_label IN (...)` with a `LIKE` predicate (still bounded by `SAFETY_LIMIT_SQL`).
- Use parameterised `IN (?, ?, …)` lists. Diesel boxed queries support `eq_any`; for raw `sql_query` paths, build the placeholder string from `set.len()` and bind each label individually. Refuse to issue the SQL when the set is empty (return `(Vec::new(), 0)` for that phase).
- **`Combined` is UI-only**: no backend handler, no full-Fulltext materialisation, no dedup pass. The two parallel sub-queries reuse the existing `DpdLookup` and `FulltextMatch` bridges as-is.
- The QML orchestrator must use distinct request ids per sub-query so the bridge's result-cache (`bridges/src/sutta_bridge.rs`, line ~28) and the `results_page_ready` signal routing remain unambiguous. Concretely: the dictionary results view holds `pending = { dpd: bool, fulltext: bool }` and only renders when both flip to false.
- The bridge's existing per-call cache keys by `(query, mode, area, params)` so the two parallel calls cache independently — re-clicking the same Combined query hits cache for both sides.
- Keep `apply_dict_source_uids_filter` as the final safety net even when the per-mode handler has already restricted retrieval. It is a no-op when the rows are already in-set, and it prevents regressions if a future change broadens retrieval.
- All filesystem existence checks (none expected here, but for any new code paths) must use `try_exists()` per CLAUDE.md.

## 8. Test Plan

1. **ContainsMatch + user dict**: import a small fixture StarDict whose `word` column contains a token absent from DPD (e.g. `xyzzy`). Search `xyzzy`. With the user dict checkbox **on**, expect ≥1 hit; with it **off**, expect 0 hits. Repeat with the term appearing only in `definition_plain` (not in `word`) — Phase 3 already covers this; ensure the count is unchanged after the change.
2. **HeadwordMatch + user dict**: same fixture. Search `xyzzy`. With the user dict checkbox **on**, expect the headword to appear; with it **off**, expect 0 hits. Solo the user dict → only that headword.
3. **DpdLookup + user dict checkbox toggling**: confirm toggling user-imported checkboxes does not change the result set. Solo a user dict → expect 0 results.
4. **Combined**: search a term that exists both in DPD lemmas and in a user dictionary's content. Expect two stacked sections — DPD on top, Fulltext below — rendered in a single pass after both sub-queries return. Verify that the result list never visibly "jumps" between renders (e.g. by artificially slowing one backend and confirming nothing renders until the slower one returns). Each section's pager advances independently.
5. **Default mode**: open Dictionary tab on a fresh install → search-mode dropdown shows `Combined`. Switch to `DPD Lookup`, restart → opens on `DPD Lookup`.
6. **Lock interaction across modes**: in each of Combined / Fulltext / Contains / Headword, lock each row in turn (DPD, Commentary, each user dict). Verify only that row contributes.
7. **No-imports baseline**: with zero user dictionaries imported, verify result counts in Contains and Headword for several known queries are unchanged from current behaviour.
8. **`make build -B`** must succeed. `cd backend && cargo test` must not introduce new failures.

## 9. Implementation Sketch (file map, not a task list)

- `backend/src/types.rs` — no change; `SearchMode::Combined` already exists in the enum but is unused at the backend layer.
- `backend/src/query_task.rs`:
  - Add `dict_words_user_phase()` helper used by Contains and Headword.
  - Extend `dict_words_contains_match_fts5_full` to call it after Phase 3 (or replace Phase 3 with a unified `dict_label IN (set)` path that already covers DPD's `dict_label = "dpd"`).
  - Extend `lemma_1_dpd_headword_match_fts5_full` to merge a user-headword path; rename to `headword_match_full` for accuracy (keep the old fn as a thin alias for one release if anything calls it externally — checking first).
  - **Do not** add a backend `combined_dict` handler. `SearchMode::Combined` + `SearchArea::Dictionary` should return an explicit `Err` from `results_page` so that any accidental backend invocation fails loudly and the QML orchestrator stays the sole entry point.
- `backend/src/search/searcher.rs` — no behavioural change required; the existing `add_dict_filters` already handles the Fulltext side.
- `backend/src/app_settings.rs` — add `dict_search_last_mode: Option<String>` (default `None`, treated as `"Combined"` on read).
- `bridges/src/dictionary_manager.rs` — add `get_last_dict_search_mode()` / `set_last_dict_search_mode(mode)` for the dropdown.
- `assets/qml/com/profoundlabs/simsapa/DictionaryManager.qml` — stubs for the two new methods.
- `assets/qml/DictionaryTab.qml` (or whichever component renders the dictionary results view): add the Combined-mode orchestrator — fire two parallel `SuttaBridge.results_page` calls, gate render on both completing, render two stacked sections with independent pagers.
- `assets/qml/SearchBarInput.qml` — add `Combined` to the Dictionary model lists; restore + persist via `DictionaryManager`.
- Tests:
  - `backend/src/query_task.rs` (or a new `backend/tests/dict_modes.rs`) — fixture-driven tests for the four scenarios in §8.

## 10. Open Questions

- Should `Combined` interleave or visually separate DPD vs Fulltext blocks beyond a section header? Default for v1: two clearly labelled sections stacked DPD-first. Each block's pager is independent. Re-evaluate after dogfooding.
- Should the two parallel sub-queries each have a per-block "page size" different from the main mode's page size (e.g. show only the top 5 DPD hits and 5 Fulltext hits on the first page)? Default for v1: same `page_len = 10` for both sides, since the existing bridge contract is global. If this proves cluttered, revisit.
- For `HeadwordMatch`, do we want to expose a separate "exact only" sub-toggle now that user dictionaries can swamp the result list with contains-style matches? Default for v1: keep the existing `LIKE '%term%'` semantics for symmetry with DPD's behaviour. Revisit if the result set becomes noisy.
