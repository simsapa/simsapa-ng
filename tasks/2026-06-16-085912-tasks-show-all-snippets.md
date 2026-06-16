# Tasks: Show All Snippets & Snippet Exclusion Filter

PRD: [2026-06-16-085912-prd---show-all-snippets.md](./2026-06-16-085912-prd---show-all-snippets.md)

## Relevant Files

- `backend/src/types.rs` - `SearchParams` struct (add `show_all_snippets`, `snippet_exclude` fields + defaults) and `SearchResult` (one row per snippet; record metadata reused; add an `is_snippet` marker so downstream code can tell record rows from expanded-snippet rows and count page size by record).
- `backend/src/highlight.rs` - **new** shared, range-based highlighter (`merge_ranges`, `wrap_ranges`, `literal_ranges`, focal-range variant) used by both `search::searcher` and `query_task`; emits non-nested `<span class='match'>`. (Or co-locate in `helpers.rs` if preferred — must be reachable from both modules.)
- `backend/src/query_task.rs` - `QueryTask` struct + ctor (carry the two new params); `fragment_around_text`/`fragment_around_query` (reuse for per-occurrence windows); `db_sutta_to_result`/`db_book_spine_item_to_result` (move Contains highlighting into these producers); `highlight_row`/`highlight_query_in_content`/`highlight_text` (convert central pass into a fallback — only highlight un-highlighted snippets); `suttas_contains_match_fts5` / `book_spine_items_contains_match_fts5` (FTS5 multi-snippet expansion); `results_page` (central exclusion-filter + record-drop pass; still sets `db_query_hits_count`).
- `backend/src/search/searcher.rs` - `render_snippet` (shared by all four doc-builders `sutta_/dict_/library_/bold_definition_doc_to_result`; refactor to range-based non-nested highlight using `Snippet::fragment()` + `Snippet::highlighted()` ∪ literal ranges — this also fixes Dictionary fulltext nesting); `search_single_index` / `search_indexes` (carry `(lang_key, DocAddress)` to expand per-occurrence post-slice); `#[cfg(test)] mod tests` (in-RAM index tests).
- `backend/src/search/tokenizer.rs` - the per-language analyzer used by the index; needed to re-tokenize stored content so `pajahati` highlights inflected `pajahitvā`.
- `backend/src/helpers.rs` - `normalize_plain_text` / `normalize_query_text` (reuse for diacritic-insensitive literal ranges + exclusion matching).
- `bridges/src/sutta_bridge.rs` - `ResultsPageCache` / `fetch_and_cache_page` / `cache_key` construction (verify new params flow into `params_json_text` so the cache invalidates automatically; `pages` already holds the expanded list, `total_hits` stays record count).
- `bridges/src/api.rs` - two `SearchParams { .. }` literal sites (~958, ~1049) need the new fields added so they compile (browser-extension path; defaults = off). These call `results_page` too, so the 2.0 highlight refactor (non-nested) applies to them automatically; multi-snippet/UI does not (defaults off).
- `assets/qml/SuttaSearchWindow.qml` - Advanced Options `Flow` (add checkbox + CSV input after `uid_suffix_input`, ~line 2434); `get_search_params()` (~line 615, add the two params); session-only root properties; `new_tab_data()` (~313) carries `find_query`; find-on-open block (`:1019–1027`) prefers the result's `find_query` over `last_query_text`.
- `assets/qml/FulltextResults.qml` - `update_page()` populate loop (consumes the now-longer flat `SearchResult` list; reads `is_snippet`; computes per-row `show_header` = uid differs from previous row, and per-row `find_query` parsed from the snippet HTML); delegate gains `show_header` + `find_query` roles, gates the metadata-header `RowLayout` (`:301`) on `show_header`. `total_pages` stays derived from the record-count `total_hits`.
- `docs/show-all-snippets.md` - new doc: highlight pipeline, per-mode snippet enumeration, exclusion semantics, header dedup, find-bar jump.
- `PROJECT_MAP.md` / `AGENTS.md` (the real target of the `CLAUDE.md` symlink) - update feature-doc pointers.

### Notes

- Run Rust tests with `cd backend && cargo test` (or `cargo test test_name`). Per project guidance, only run tests after all sub-tasks of a top-level task are done; skip `make qml-test` unless explicitly asked.
- Build with `make build -B` (not direct cmake).
- The real appdata DB for integration tests is at the SIMSAPA_DIR path in CLAUDE.md; do not gate DB-backed tests behind `#[ignore]`.
- This feature edits existing QML only (no new QML files to register in `bridges/build.rs`).
- Session-only state = plain QML root properties (no `SuttaBridge` getter/setter, no settings-DB persistence), unlike the persisted include-checkboxes.
- **Highlight invariant (all tasks):** a snippet must never contain nested `<span class='match'>`. Several tests assert `!snippet.contains("class='match'><span")` and that the span count equals the expected occurrence count.

## Instructions for Completing Tasks

As you complete each sub-task, change `- [ ]` to `- [x]` in this file, updating after each sub-task (not just per parent task).

## Tasks

### Specs & dependencies for 1.0

- **Param shape:** `show_all_snippets: bool` (default `false`), `snippet_exclude: Option<Vec<String>>` (default `None`; `Some([])` treated as none). Both `#[serde(default)]` so older serialized params and the browser-extension literals stay valid.
- **Flow:** QML `get_search_params()` → JSON → `SearchParams` (deserialized in `sutta_bridge.rs::fetch_and_cache_page` / combined path) → `QueryTask`. Mirror the existing `uid_suffix` plumbing exactly.
- **Cache dependency:** `cache_key = format!("{}|{}|{}", query_text, search_area_text, params_json_text)` (`sutta_bridge.rs:1793`, and the `|combined` variant at `:1688`). Because the params are inside `params_json_text`, no cache-key code changes are required — only verify.

- [ ] 1.0 Thread `show_all_snippets` and `snippet_exclude` search params through the backend and QML param builder (no behaviour change yet)
  - [ ] 1.1 Add `show_all_snippets: bool` and `snippet_exclude: Option<Vec<String>>` (both `#[serde(default)]`) to `SearchParams` in `backend/src/types.rs`, and set them in the `Default` impl (`false` / `None`).
  - [ ] 1.1a Add an `is_snippet: bool` field (`#[serde(default)]`, default `false`) to `SearchResult` in `backend/src/types.rs`, set `false` in the existing `from_sutta` / `from_dict_word` / `from_book_spine_item` constructors (those are whole-record rows), and read it in `update_page()` in `FulltextResults.qml` (default `false`). Marks expanded-snippet rows so record-count logic groups rows by record (a record = the run of rows sharing one `uid`, snippet rows flagged `is_snippet: true`). (`find_query` for the find-bar jump is derived QML-side in Task 7, not stored on `SearchResult`.)
  - [ ] 1.2 Add matching fields to `QueryTask` in `backend/src/query_task.rs` (near `uid_suffix` at `:81`) and populate them from `params` in the constructor (near `:136`).
  - [ ] 1.3 Add the new fields to the two `SearchParams { .. }` literals in `bridges/src/api.rs` (~958, ~1049) so the crate compiles (default off/none).
  - [ ] 1.4 Add session-only root properties to `SuttaSearchWindow.qml` (e.g. `property bool show_all_snippets: false`) — or read directly from the new controls — and include `show_all_snippets` + parsed `snippet_exclude` (CSV → trimmed array, drop empties, `null` if empty) in the object returned by `get_search_params()` (~line 615).
  - [ ] 1.5 Confirm (read-through, add a code comment if helpful) that `RESULTS_PAGE_CACHE` and the combined cache key include the new params via `params_json_text`, so toggling them invalidates cached pages automatically.
  - [ ] 1.6 Build (`make build -B`) and confirm clean compile with no behaviour change (params accepted, ignored by query logic so far).

### Specs & dependencies for 2.0 (highlight pipeline refactor — prerequisite)

- **The bug being fixed:** Fulltext snippets are highlighted by tantivy in `render_snippet` (`searcher.rs:836`, `.to_html()` of a `Snippet`), then `results_page` (`query_task.rs:2311`) runs `highlight_row` → `highlight_query_in_content` → `highlight_text` (`:147`) which regex-wraps the literal query term **again** → nested `<span class='match'><span class='match'>…</span></span>`. Contains snippets are plain (`fragment_around_query`) and rely on the central pass (correct, no nesting). Nesting is visually invisible but breaks tag parsing (find-bar) and the focal-only rule.
- **Target design (PRD Req 12b–12e):** highlighting is **producer-owned** and **range-based**, non-nested by construction; the central pass becomes a **fallback** for snippets a producer left plain.
- **Per-mode semantics to preserve/establish:**
  - ContainsMatch (no stemming): highlight literal query occurrences only. Single-snippet = all occurrences in the snippet.
  - FulltextMatch (stemmer): highlight tantivy stemmed matches **and** literal occurrences. Single-snippet = all such occurrences in the snippet.
- **tantivy API:** `Snippet::fragment() -> &str` and `Snippet::highlighted() -> &[Range<usize>]` (tantivy 0.25) give the fragment text + stemmed match ranges to build our own HTML.
- **No behaviour change for plain-snippet modes** (TitleMatch, UidMatch, non-DPD dict, DPD): the fallback still highlights them; DPD path stays excluded as today (`is_dpd_result`).

- [ ] 2.0 Refactor the snippet-highlight pipeline to be producer-owned, range-based, and non-nested (fixes the existing double-highlight bug and prepares for focal snippet highlighting)
  - [ ] 2.1 Add a shared highlight module (`backend/src/highlight.rs`, registered in `lib.rs`) with: `merge_ranges(ranges) -> Vec<Range<usize>>` (sort + coalesce overlapping/adjacent), `wrap_ranges(text, &[Range]) -> String` (one `<span class='match'>` per merged range — non-nested), `literal_ranges(text, term) -> Vec<Range>` (all normalized, case-insensitive occurrences), and a focal single-occurrence helper (range around a known byte offset). All operate on char-boundary-safe byte ranges.
  - [ ] 2.1a Add an **offset-based windowing helper** alongside `fragment_around_text` (`query_task.rs:183`), e.g. `fragment_around_offset(content, match_start, match_len, chars_before, chars_after) -> (window_text, focal_range_in_window)`. Reason: `fragment_around_text` only windows around the **first** occurrence (`content.to_lowercase().find(term)`), but multi-snippet needs a window around the **Nth** occurrence at a known offset, plus the focal match's range *within that window* (to pass to `wrap_ranges`). Reused by Tasks 3 (Fulltext) and 4 (Contains). Keep the existing `fragment_around_text` for the single-snippet fast path.
  - [ ] 2.2 Refactor `render_snippet` (`searcher.rs:836`) to range-based: take `Snippet::fragment()` + `Snippet::highlighted()` (stemmed ranges), union with `literal_ranges(fragment, normalize_plain_text(query))`, `merge_ranges`, then `wrap_ranges`. Pass the query text into `render_snippet` (signature change). **`render_snippet` is shared by all four doc-builders** — `sutta_doc_to_result`, `dict_doc_to_result`, `library_doc_to_result`, `bold_definition_doc_to_result` (`searcher.rs:691/723/765/797`) — so update all four call sites; the refactor also fixes the same nested-span bug on the **Dictionary** fulltext path (out of scope for multi-snippet, but it shares `render_snippet`). Confirm dict/bold/library snippets still render correctly. Output is non-nested and covers stemmed + literal (Req 12d).
  - [ ] 2.3 Move ContainsMatch highlighting into the producers: in `db_sutta_to_result` and `db_book_spine_item_to_result`, highlight the plain fragment via `wrap_ranges(fragment, literal_ranges(fragment, normalize_plain_text(query)))` (all occurrences) so the snippet arrives already highlighted (Req 12c).
  - [ ] 2.4 Convert the central pass into a fallback: in `highlight_row` (`query_task.rs:2361`), highlight via `highlight_query_in_content` **only when** the snippet does not already contain `class='match'` (producer left it plain — e.g. TitleMatch/UidMatch/non-DPD dict). Already-highlighted snippets (Fulltext, Contains, and later the focal all-snippets rows) pass through unchanged. This removes the nested-span bug and means **no separate `is_snippet` highlight guard is needed** later.
  - [ ] 2.5 Regression tests (write now, run at 2.7):
    - In-RAM Fulltext index test: query `pajahati` on content containing `pajahati` and a stemmed form (e.g. `pajahitvā`) → snippet highlights **both**, each wrapped exactly once; assert `!snippet.contains("class='match'><span")` (no nesting) and the `class='match'` count == 2.
    - Contains-producer test: a plain fragment with two `pajahati` occurrences → `wrap_ranges`/`literal_ranges` highlights both, non-nested, and does **not** highlight `pajahitvā`.
    - Fallback test: `highlight_row` on an already-highlighted snippet is a no-op (count of spans unchanged); on a plain snippet it highlights as before.
    - `merge_ranges` unit tests: overlapping/adjacent/disjoint ranges coalesce correctly.
  - [ ] 2.6 Manually confirm no visible regression in normal search highlighting (single-snippet Fulltext + Contains still show highlighted matches) — agent verifies via tests + compile; GUI check is the user's.
  - [ ] 2.7 Build (`make build -B`) and run `cd backend && cargo test` for the new highlight tests; confirm existing highlight-related tests still pass.

### Specs & dependencies for 3.0 (Fulltext multi-snippet)

- **Depends on 2.0:** reuses `wrap_ranges` + the focal-range helper, and relies on the fallback guard (2.4) so focal snippets are not re-highlighted.
- **Where snippets are made (tantivy):** `search_single_index` (`searcher.rs:377`); page slicing in `search_indexes` (`:341`–`:373`): fetch `limit=(page_num+1)*page_len`, score-sort, `skip(page_num*page_len).take(page_len)`.
- **Multi-snippet rule:** with `show_all_snippets` on, each page record yields one `SearchResult` per matched occurrence, each highlighting **only its focal occurrence** (Req 12a). Enumerate occurrences by re-tokenizing the stored `content` with the index analyzer (`tokenizer.rs`) and taking a window around each token whose stem matches a query term — this is what surfaces `pajahitvā` for query `pajahati`. Multi-term/AND: every matched term.
- **Pagination stays record-based (mandatory):** the `skip().take()` slice operates on scored records; expansion happens **after** the slice, so a page always holds `page_len` records (expanded to more rows). Bounded enumeration cost = `page_len` records/page.
- **List shape / counting:** expanded page `Vec<SearchResult>` may exceed `page_len` rows; `total_hits` stays the record count; expanded rows carry `is_snippet: true`. `total_pages = ceil(total_hits / page_len)` unaffected.
- **Plumbing:** thread `show_all_snippets` from `fulltext_suttas`/`fulltext_library` (`query_task.rs:1927`/`:2022`) via `SearchFilters` (or an arg) → `search_*_with_count` → `search_indexes` → `search_single_index`.
- **Areas:** Suttas (`IndexType::Sutta`) + Library (`IndexType::Library`) only; Dict index stays single-snippet.

- [ ] 3.0 Fulltext Match (tantivy): expand each page record into one focal-highlighted `SearchResult` per matched occurrence, for Suttas and Library
  - [ ] 3.1 Thread `show_all_snippets` into the searcher (extend `SearchFilters` or add a param) from `fulltext_suttas`/`fulltext_library` down to `search_single_index`.
  - [ ] 3.2 Restructure `search_indexes` so per-record multi-snippet work happens **after** the record-level cross-language `skip().take()` slice. Today `search_single_index` builds the `SearchResult` (and renders its snippet) per doc inside its loop (`searcher.rs:441–465`) and returns `(score, SearchResult)`; `search_indexes` merges across languages, sorts by score, then slices. To expand post-slice, carry a doc handle alongside each scored result — **`(lang_key, DocAddress)`** — because after the cross-language merge each sliced record must be re-associated with its own index/reader to re-fetch the doc + content. For the `page_len` sliced records (flag on), fetch each doc via its `(index, reader)` and expand; leave the flag-off path exactly as today. Mandatory for pagination correctness; cost bounded to `page_len` records.
  - [ ] 3.3 Implement the occurrence enumerator: given a doc's stored `content`, the query terms, and the index analyzer (`tokenizer.rs`), return all match byte ranges (re-tokenize content; match tokens whose stem equals a query term's stem; cover every term for AND). Tokenizing a string with the registered analyzer can be done without the index.
  - [ ] 3.4 Per-occurrence rendering: for each match range, build a window via `fragment_around_offset` (2.1a) and highlight **only the focal occurrence** via `wrap_ranges` with the focal range; emit one `SearchResult` per occurrence with the record's shared metadata and `is_snippet: true`. **Zero-occurrence fallback:** if the enumerator finds no ranges for a matched record (analyzer/normalization edge case), emit the single best snippet (current `render_snippet`) as one row so the record still appears. (No separate highlight guard — the 2.4 fallback skips already-highlighted snippets.)
  - [ ] 3.5 Apply the same expansion for `IndexType::Library` (book spine items), preserving per-snippet `page_number`.
  - [ ] 3.6 Confirm count code doesn't assume `results.len() == record_count` in the expanded case (group by `uid`/`is_snippet`); keep `total` = record count.
  - [ ] 3.7 Tests (write now, run at 3.8): in-RAM index test — a doc with multiple occurrences yields one `SearchResult` per occurrence with `is_snippet: true` when on (each focal-highlighted, single span, non-nested), and exactly one row when off. Assert each focal snippet highlights only its own occurrence (other matched terms in the window un-highlighted).
  - [ ] 3.8 Build (`make build -B`) + `cargo test`; sanity-check a Suttas fulltext query expands a multi-occurrence record.

### Specs & dependencies for 4.0 (Contains multi-snippet)

- **Depends on 2.0/2.3:** reuses the focal-range helper; central fallback skips these (already highlighted).
- **Where:** `suttas_contains_match_fts5` (`query_task.rs:590`) and `book_spine_items_contains_match_fts5` (`:1171`) fetch page rows (`s.*`) and map each via `db_sutta_to_result` / book equivalent.
- **Multi-snippet rule (contains):** literal substring matching (`content_plain LIKE '%query%'`), so occurrences are all literal positions of the normalized query in the row content. Emit one `SearchResult` per occurrence, highlighting **only the focal** occurrence at its offset (not every occurrence in the window). Inflected forms are out of scope for contains (fulltext-only) — note in doc/test.

- [ ] 4.0 Contains Match (SQLite FTS5): expand each page record into one focal-highlighted `SearchResult` per literal occurrence, for Suttas and Library
  - [ ] 4.1 Add a `QueryTask` helper mapping one `Sutta` row → `Vec<SearchResult>` (one per literal occurrence of the normalized query in the same content used for the snippet, `content_plain`; window each via `fragment_around_offset` (2.1a); focal-highlight via the 2.1 helper; `is_snippet: true`; shared record metadata). Use the same normalization as the highlight/snippet path so occurrence offsets line up. **Zero-occurrence fallback:** if no literal occurrence is found, emit one `db_sutta_to_result` snippet so the record still appears. Falls back to single-snippet `db_sutta_to_result` (`is_snippet: false`) when the flag is off.
  - [ ] 4.2 In `suttas_contains_match_fts5`, swap the `.map(|s| db_sutta_to_result(s))` collect for the flag-aware expansion (`flat_map` when on); keep `total` = record `COUNT(*)`.
  - [ ] 4.3 Add the equivalent expansion in `book_spine_items_contains_match_fts5` (book content + `page_number`), same flag.
  - [ ] 4.4 Verify Dictionary contains-match paths are untouched (out of scope, single-snippet).
  - [ ] 4.5 Tests (write now, run at 4.6): a row with two literal occurrences expands to two `SearchResult`s, each focal-highlighted (single non-nested span), `is_snippet: true`; `pajahitvā` is **not** highlighted for query `pajahati`. Flag-off path returns one row, highlighted by the fallback.
  - [ ] 4.6 Build (`make build -B`) + `cargo test`; sanity-check a Suttas contains query expands a multi-occurrence record.

### Specs & dependencies for 5.0 (exclusion filter)

- **Semantics:** `snippet_exclude: Option<Vec<String>>`. Remove any `SearchResult` whose snippet contains **any** listed string, matched **diacritic-insensitively** via `normalize_plain_text` (normalize both snippet text and exclude strings; strip `<span class='match'>` tags first). Applies in single- and multi-snippet modes (Req 16).
- **Record drop:** a record whose snippets are all excluded contributes no rows (disappears). `total` (record count) is **not** adjusted (Req 15 / Resolved Decision 1).
- **Placement:** in `results_page` (`query_task.rs:2215`) after the per-area handler returns and after `highlight_row`, so it covers both modes and snippets may carry highlight tags.

- [ ] 5.0 Snippet exclusion filter: drop snippets matching any CSV entry (diacritic-insensitive); omit records whose snippets are all excluded
  - [ ] 5.1 Add a helper testing one snippet against the normalized exclude list (strip highlight tags; normalize both sides; substring test) → true if it should be removed.
  - [ ] 5.2 In `results_page`, when `snippet_exclude` is non-empty, filter the page list through the helper (works for one-per-record and one-per-snippet lists).
  - [ ] 5.3 Records left with zero snippets simply have no rows; leave `total` unchanged; don't backfill the page.
  - [ ] 5.4 `None`/empty list = no-op fast path (no normalization work).
  - [ ] 5.5 Tests (write now, run at 5.6): query whose snippets include an excluded form (e.g. `pajahitvā`) — those snippets removed (diacritic-insensitive, e.g. `pajahitva` also matches); a record with all snippets excluded disappears while `total` is unchanged.
  - [ ] 5.6 Build (`make build -B`) + `cargo test`.

### Specs & dependencies for 6.0 (UI + header dedup)

- **UI placement:** Advanced Options `Flow`, after the `uid_suffix_input` `RowLayout` (~line 2434). Gate to Suttas + Library: `visible: search_bar_input.search_area === "Suttas" || search_bar_input.search_area === "Library"`.
- **Conventions:** `font.pointSize: root.is_mobile ? 12 : 10`; text input uses `MobileKeyboardHelper {}`, `EnterKey.type: Qt.EnterKeyDone`, `selectByMouse: true`; controls call `advanced_options_debounce_timer.restart()` (or `root.advanced_options_changed()`). No `console` API — use `Logger`.
- **Session-only:** plain QML state; no `SuttaBridge` persistence.
- **Header dedup (depends on 1.1a `is_snippet`):** delegate metadata header `RowLayout` (`FulltextResults.qml:301`) gated on a `show_header` model role computed in `update_page()` (`:163`) while appending rows (uid differs from previous appended row's uid). Single-snippet record = one-row group, header always shows. Fixed item height (`:244`) unchanged.

- [ ] 6.0 Advanced Search Options UI + header dedup
  - [ ] 6.1 Add `CheckBox { id: show_all_snippets_checkbox; text: "Show All Snippets" }` after `uid_suffix_input`, gated to Suttas + Library; `onCheckedChanged` → `root.advanced_options_changed()`.
  - [ ] 6.2 Add `TextField { id: snippet_exclude_input }` (label "Exclude snippets containing:", placeholder `e.g. pajahitvā, na upādiyati`), `MobileKeyboardHelper`, `EnterKey.type: Qt.EnterKeyDone`, `onTextChanged: advanced_options_debounce_timer.restart()`, gated to Suttas + Library.
  - [ ] 6.3 Wire both into `get_search_params()` (checkbox → `show_all_snippets`; input → CSV split/trim/drop-empties → `snippet_exclude` array or `null`).
  - [ ] 6.4 In `update_page()`, compute per-row `show_header` (uid differs from previous appended uid; first row true) and append to `results_model`; add the `show_header` role to the delegate's `required property` set. Treat `is_section_header` rows as group boundaries (reset "previous uid") and don't disturb the section-header path.
  - [ ] 6.5 Gate the metadata header `RowLayout` (`:301`) on `result_item.show_header`; keep the snippet `Text` always visible and the item height fixed.
  - [ ] 6.6 Confirm controls reset on restart (no persistence); Dictionary area hides them and is unaffected.
  - [ ] 6.7 Build (`make build -B`); confirm controls appear, re-trigger the query, and header shows once per record group (GUI check is the user's).

### Specs & dependencies for 7.0 (snippet-aware find-bar jump)

- **Goal:** clicking a result opens the record and jumps the find bar to **that snippet's** passage. Find query = matched word + following 1–2 words (e.g. `… *pajahitvā* ṭhito …` → `pajahitvā ṭhito`).
- **Derivation (QML-side, in `update_page()`):** find the first `<span class='match'>`, take to end, **strip all remaining tags** (after 2.0 there should be no nesting, but strip defensively), drop `…` and trailing punctuation, take matched word + 1–2 following words. Empty if no match span. Store as a `find_query` model role.
- **Existing flow:** find-on-open trigger `SuttaSearchWindow.qml:1019–1027` sets `root.pending_find_query = root.last_query_text` (gated on `open_find_in_sutta_results`, Suttas area, content-replace open). Result metadata flows via `new_tab_data()` (~313) → `show_result_in_html_view()` → `add_results_tab()`; `current_result_data()` (`FulltextResults.qml:71`) returns the whole model row.

- [ ] 7.0 Snippet-aware find-bar jump
  - [ ] 7.1 In `update_page()`, derive `find_query` per row from the snippet HTML (as above), append to `results_model`, add the `find_query` role to the delegate's `required property` set.
  - [ ] 7.2 Thread `find_query` into the open path: it rides along in `current_result_data()`; add a `find_query` key in `new_tab_data()` (~313) so it reaches the find-on-open block. Trace each hop from click → `show_result_in_html_view()` and confirm it survives.
  - [ ] 7.3 In the find-on-open block (`:1019–1027`), set `root.pending_find_query` to the result's `find_query` when present, else fall back to `root.last_query_text`; keep all existing gates unchanged.
  - [ ] 7.4 Factor the `find_query` derivation into a small pure JS function so it is testable; add a check: `… pajahati na upādiyati *pajahitvā* ṭhito …` → `pajahitvā ṭhito`; no match span → `""`.
  - [ ] 7.5 Confirm unchanged behaviour when `open_find_in_sutta_results` is off and for non-sutta results; a single-snippet result still jumps to its one snippet's matched word + following words.
  - [ ] 7.6 Build (`make build -B`); GUI verification of the jump is the user's.

### Specs & dependencies for 8.0 (integration acceptance + docs)

- **Acceptance case (Req 12):** fulltext `pajahati` against `cnd8/pli/ms` (content `… pajahati na upādiyati pajahitvā ṭhito …`) with `show_all_snippets` on → two snippets, snippet 1 highlights only `pajahati`, snippet 2 only `pajahitvā`.
- **Test DB:** real appdata DB path in CLAUDE.md; do not `#[ignore]` DB-backed tests.

- [ ] 8.0 Integration acceptance, full test run, and documentation
  - [ ] 8.1 Add the `cnd8/pli/ms` two-snippet acceptance test against the real appdata DB (fulltext `pajahati`, `show_all_snippets` on): exactly two snippets for that record, each focal-highlighted with the expected form, non-nested.
  - [ ] 8.2 Add an integration check combining `show_all_snippets` + `snippet_exclude` (exclude `pajahitvā` → snippet 2 gone, snippet 1 remains; record stays because snippet 1 survives).
  - [ ] 8.3 Run `cd backend && cargo test` and confirm green (ignore pre-existing unrelated failures per project guidance).
  - [ ] 8.4 Write `docs/show-all-snippets.md`: the refactored highlight pipeline (producer-owned, non-nested, per-mode semantics), fulltext stemmed vs contains literal enumeration, focal-only vs all-occurrence highlighting, exclusion semantics, header dedup, snippet-aware find-bar jump, session-only state, cache reuse.
  - [ ] 8.5 Add a pointer in `PROJECT_MAP.md` and the notable-feature-docs list (edit `AGENTS.md`, the real target of the `CLAUDE.md` symlink).
