# PRD: Show All Snippets & Snippet Exclusion Filter

**Date:** 2026-06-16
**Feature area:** Sutta/Library search — Advanced Search Options
**Status:** Draft

## 1. Introduction / Overview

Today, a fulltext or contains-match search shows **one snippet per matched
record** in the results list (`FulltextResults.qml`). A search for `pajahati`
returns each matching sutta once, with a single excerpt placed at the first
spot where `pajahati` (or an inflected form such as `pajahituṁ`) was matched.
A record that contains the search term in several places — possibly in several
different inflected forms — hides all but one of those occurrences from the
user.

This feature adds a **"Show All Snippets"** option to the Advanced Search
Options area. When enabled, a matched record is "expanded" — meaning the
**number of result items grows**: each matched occurrence in the record becomes
its own result item in the results list, each with its own highlighted excerpt.
The result delegate items are rendered exactly as before (same layout, same
item height); there are simply more of them. When several snippet items belong
to the **same record**, only the **first** item shows the record's metadata
header (sutta_ref / title / uid) — the repeated header on the following snippet
items is suppressed, so the group reads as one record with several excerpts.

It also improves the **click-to-open** behaviour: clicking a result item opens
the record in the HTML view and triggers the find bar so the page jumps to the
matched text. Instead of searching for the original query (which lands on the
first occurrence in the page), the find bar now searches for the **matched word
plus the following word(s) from that specific snippet**, so the page jumps to
the exact place the clicked snippet came from. It also adds a **snippet
exclusion filter**: a
comma-separated list of strings that removes any snippet containing one of
those strings, letting the user narrow a broad search by suppressing forms or
phrases they are not interested in.

The goal is to give users studying Pāli passages a complete, per-occurrence
view of where and how a term appears across the corpus, with a quick way to
filter out unwanted contexts.

## 2. Goals

1. Add a **"Show All Snippets"** toggle to Advanced Search Options, below the
   UID suffix filter, applicable to **Fulltext Match** and **Contains Match**
   modes.
2. When enabled, every matched occurrence in a record becomes its **own flat
   result item** with its own highlighted snippet — i.e. a record with N
   occurrences contributes N result items to the list, each rendered with the
   existing delegate (unchanged layout and item height).
3. Add a **snippet exclusion filter** input (CSV) that removes snippets
   containing any of the listed strings, matched **diacritic-insensitively**.
4. Keep results **paginated by record** (10 records per page) exactly as today.
   Expansion does not change the page size or the total record count — it only
   increases how many result items those 10 records render on the page.
5. Generate per-occurrence snippets and highlighting **only for the records on
   the currently visible page**, reusing the existing highlighted-page cache so
   navigating prev/next/back to a page does not recompute.
6. Preserve existing behaviour when "Show All Snippets" is off (one snippet per
   record, unchanged).
7. Show the record metadata header (sutta_ref / title / uid) only on the
   **first** result item of each record group; suppress it on the following
   snippet items of the same record.
8. When a result item is clicked, make the find bar jump to **that snippet's**
   location in the page by searching for the snippet's matched word plus the
   following word(s), instead of the original query text.
9. Correct and unify the snippet-highlighting pipeline as a prerequisite: make
   highlighting producer-owned, range-based, and **non-nested**, fixing the
   existing Fulltext double-highlight (nested `<span class='match'>`) and
   establishing the per-mode semantics the snippet expansion relies on.

## 3. User Stories

- **As a Pāli student**, when I search `pajahati` with "Show All Snippets" on, I
  want each place the term (and its inflections) occurs to appear as its own
  highlighted result item, so I can study every usage across the corpus without
  opening each full text.
- **As a researcher**, when a broad search returns too much noise, I want to
  type `pajahitvā, na upādiyati, akusalaṁ` into a filter box so that snippets
  containing any of those strings are removed from the results, narrowing what
  I see.
- **As a user who wants a quick overview**, I want "Show All Snippets" off by
  default so the results list shows one snippet per record and stays scannable,
  and to opt in only when I need the full per-occurrence breakdown.
- **As a reader**, when I click a snippet I want the opened page to jump to the
  exact passage that snippet came from (not just the first occurrence of my
  query), so I land where I was looking — e.g. clicking
  `… pajahati na upādiyati *pajahitvā* ṭhito …` jumps to `pajahitvā ṭhito`.
- **As a reader scanning a record's occurrences**, I want the title/uid header
  shown once at the top of the record's snippets, not repeated on every snippet,
  so the group is easy to read.

## 4. Functional Requirements

### Advanced Search Options UI

1. The system must add a **"Show All Snippets"** checkbox to the Advanced
   Search Options area in `SuttaSearchWindow.qml`, positioned **after the UID
   suffix filter** row.
2. The "Show All Snippets" checkbox must be **visible/applicable** for the
   **Suttas** and **Library** search areas only. It must not appear (or must be
   inert) for the Dictionary area.
3. The system must add a **"Exclude snippets containing"** text input (CSV)
   below or beside the "Show All Snippets" checkbox, with a placeholder such as
   `e.g. pajahitvā, na upādiyati`.
4. Both controls must follow existing Advanced Options conventions:
   `MobileKeyboardHelper`, `EnterKey.type: Qt.EnterKeyDone` for the text input,
   debounced re-query via `advanced_options_debounce_timer` /
   `advanced_options_changed()`, and the project font-size/`is_mobile` styling.
5. Changing either control must re-run the current query (debounced), the same
   way the existing prefix/suffix filters do.
6. Both controls are **session-only** state: they reset to default
   (unchecked / empty) on app restart and are **not** persisted to the settings
   DB.

### Search parameters & backend

7. The system must pass two new search parameters from QML through to the Rust
   backend search request:
   - `show_all_snippets: bool`
   - `snippet_exclude: Option<Vec<String>>` (parsed from the CSV input; each
     entry trimmed; empty entries dropped; `None`/empty list = no exclusion).
   These must be threaded through `SearchParams` (`backend/src/types.rs`) and
   `QueryTask` (`backend/src/query_task.rs`) alongside the existing
   `uid_prefix` / `uid_suffix` parameters.
8. When `show_all_snippets` is **false**, the backend must behave exactly as
   today: one snippet per record (no behaviour change, no extra work).
9. When `show_all_snippets` is **true**, for each matched record **on the
   requested page**, the backend must produce a list of snippets — **one
   snippet per matched occurrence** in that record — instead of a single
   snippet. Each snippet must include the highlighted match span using the
   existing highlight markup (`<span class='match'>…</span>`).
10. **Fulltext Match** (tantivy) must use the tantivy index / its matched terms
    to identify all occurrences in a matched record's content and produce one
    highlighted snippet per occurrence.
11. **Contains Match** (sqlite FTS5) must use the FTS5 index / matched content
    to identify all occurrences in a matched record's content and produce one
    highlighted snippet per occurrence.
12. Worked example (acceptance test): a fulltext query `pajahati` matches sutta
    uid `cnd8/pli/ms`, whose content contains
    `… pajahati na upādiyati pajahitvā ṭhito …`. With "Show All Snippets" on,
    this record must yield **two** snippets:
    - `… *pajahati* na upādiyati pajahitvā ṭhito …` (first occurrence
      highlighted), and
    - `… pajahati na upādiyati *pajahitvā* ṭhito …` (second occurrence
      highlighted),
    where `*…*` denotes the highlighted match span.
12a. **Focal-only highlight (all-snippets mode):** each expanded snippet must
    highlight **only its own occurrence**, not every matched term that happens to
    fall inside the window (note snippet 1 above leaves `pajahitvā` un-highlighted
    and snippet 2 leaves `pajahati` un-highlighted). This differs from
    single-snippet mode, which highlights all occurrences in the one snippet.

### Highlighting behaviour (corrected pipeline)

12b. **No nested highlight spans.** Each highlighted occurrence is wrapped in
    exactly one `<span class='match'>…</span>`; a snippet must never contain
    nested match spans. (The current pipeline produces nested spans for the
    Fulltext literal-query term because tantivy highlights the snippet and then a
    central `highlight_row` pass re-highlights the same literal term — an
    unintended double pass. This must be corrected; see Technical Considerations.)

12c. **ContainsMatch highlighting is literal only** (no Pāli stemming): it
    highlights occurrences of the query string itself (normalized, diacritic- and
    case-insensitive). Single-snippet mode highlights **every** such occurrence in
    the snippet; all-snippets mode highlights **only the focal** occurrence of
    that snippet. (E.g. query `pajahati` highlights `pajahati`, never `pajahitvā`.)

12d. **FulltextMatch highlighting covers both** the Pāli-stemmed matches (e.g.
    `pajahati` → `pajahitvā`, via the tantivy stemmer) **and** literal
    string-match occurrences of the query. Single-snippet mode highlights **all**
    such occurrences in the snippet; all-snippets mode highlights **only the
    focal** occurrence of that snippet.

12e. **Highlighting is the snippet producer's responsibility.** Each mode
    produces its snippet already correctly highlighted (non-nested); the central
    `highlight_row` pass is reduced to a fallback that only highlights snippets a
    producer left un-highlighted (so it can never double-wrap). See Technical
    Considerations for the shared range-based highlighter.

### Snippet exclusion filter

13. When `snippet_exclude` is non-empty, the system must remove any snippet
    whose text contains **any** of the listed strings.
14. Exclusion matching must be **diacritic-insensitive** (normalized), using the
    same Pāli normalization applied elsewhere in the search path (so a typed
    `pajahitva` matches a snippet containing `pajahitvā`). Matching is a
    substring test against the snippet's plain text.
15. If, after exclusion, a record has **zero** surviving snippets, that record
    must not be displayed on the page at all (so the user sees the results
    narrowed down). This does **not** need to adjust the displayed total result
    count — the user is aware they are filtering records out, so the count may
    legitimately exceed the number of records shown.
16. The exclusion filter must be applicable whenever the input is non-empty,
    independent of the "Show All Snippets" toggle (it filters the single
    snippet in normal mode too). Its primary, most useful pairing is with
    "Show All Snippets".

### Results UI & expansion

17. Results must remain **paginated by record at 10 records per page**. Enabling
    "Show All Snippets" must **not** change the page size, the total record
    count, or the prev/next paging behaviour. It only increases the number of
    result items that page's records produce.
18. In `FulltextResults.qml`, when "Show All Snippets" is on, the page's records
    must be flattened into the existing `results_model` as **one result item per
    snippet**. No new per-snippet expand/collapse control is added — the result
    delegate, its layout, and its item height stay as they are today; the list
    simply contains more items. (Consecutive items from the same record share
    the same sutta_ref / title / uid, since each is a snippet of that record.)
19. The result delegate must show the record metadata header (sutta_ref / title
    / uid row) only on the **first** result item of each record group. The model
    must carry a `show_header` value (computed in `update_page()` as "this row's
    uid differs from the previous row's uid"), and the delegate must bind the
    header row's `visible` to it. The snippet body remains visible on every
    item. Item height stays fixed (the suppressed header simply leaves the
    snippet more room / blank space; no height change).
20. Snippet highlighting/extraction work must be performed **only for the
    records of the currently visible page**, and must reuse the **existing
    highlighted-page cache** (`RESULTS_PAGE_CACHE` in `sutta_bridge.rs`) so that
    returning to a page via prev/next does not recompute.
21. The existing list delegate and its fixed item height must remain unchanged
    (apart from the `show_header` gating in Req 19). Section-header rows and the
    single-snippet (Show All Snippets off) layout must continue to work exactly
    as today (a single-snippet record is a one-item group, so its header always
    shows).

### Click-to-open find-bar jump

22. Each `SearchResult` must carry a per-snippet **find query**: the snippet's
    matched word plus the following word(s) (e.g. for the snippet
    `… pajahati na upādiyati *pajahitvā* ṭhito …` the find query is
    `pajahitvā ṭhito`). For a single-snippet (Show All Snippets off) result, this
    is derived from that one snippet the same way.
23. When a result item is clicked and the record is opened in the HTML view, the
    find bar must be triggered with the result's **per-snippet find query**
    instead of the original `last_query_text`
    (`SuttaSearchWindow.qml:1025`). If the per-snippet find query is empty/absent,
    fall back to the current behaviour (`last_query_text`).
24. This applies only to the existing find-on-open path (the user preference
    `open_find_in_sutta_results`, Suttas area, content-replace open). It must not
    change behaviour when that preference is off or for non-sutta results.

## 5. Non-Goals (Out of Scope)

- **Dictionary** search area support for multi-snippet expansion (dictionary
  entries are short single definitions).
- Persisting "Show All Snippets" or the exclusion filter across restarts (this
  release is **session-only**).
- Changing pagination to be snippet-based (count = total snippets). Pagination
  stays record-based.
- New highlight styling/colors — reuse the existing `span.match` highlight.
- Regex or boolean logic in the exclusion filter (plain comma-separated
  substrings only).
- Localhost / browser-extension search API (`bridges/src/api.rs` Rocket routes,
  e.g. `POST /suttas_fulltext_search`) — exposing Fulltext/Contains search over
  curl with pagination, highlights, and single-/all-snippets mode is **deferred
  to a later change**, not part of this release. This release targets the in-app
  `SuttaSearchWindow` results.

  **The design is intentionally API-ready and must not block that follow-up:**
  the API endpoints already call `SearchQueryTask::results_page`, the same choke
  point the UI uses, so the new `show_all_snippets` / `snippet_exclude` params
  (which live on `SearchParams`), the producer-owned **non-nested** highlight
  refactor (Goal 9 / Reqs 12b–12e), the per-occurrence expansion, and the
  exclusion filter are all produced **backend-side** and serialized into the
  `SearchResult` JSON automatically (including the `is_snippet` marker). The only
  two values an API client would not receive directly — `show_header` and
  `find_query` — are derived **QML-side** in `update_page()` and are trivially
  recomputable by any client (uid adjacency; parsing the snippet HTML), so the
  later API exposure is request-plumbing only (add `mode` / `search_area` /
  `page_len` / `show_all_snippets` / `snippet_exclude` to the request and set
  them on `SearchParams`). Keep all data-shaping (expansion, highlight,
  exclusion) in the backend — never in `FulltextResults.update_page()` — so it
  stays on the shared path.

## 6. Design Considerations

- Place the new controls in the Advanced Options `Flow` after the existing
  `uid_suffix_input` `RowLayout` (around `SuttaSearchWindow.qml:2434`), gated to
  the Suttas + Library areas (mirror the `visible: search_bar_input.search_area
  === "Suttas"` pattern, extended to include Library).
- There is **no new expand/collapse UI control**. "Expansion" is purely the
  growth in the number of flat result items; each snippet is an ordinary result
  delegate item, identical to a single-snippet item today.
- Each snippet item carries its own record's sutta_ref / title / uid, but the
  header row is shown **only on the first item of a record group** (Req 19); the
  following snippet items of the same record show just the highlighted excerpt.
  This is driven by a `show_header` flag computed per row in `update_page()`
  (uid differs from the previous row), passed to the delegate.
- Clicking a snippet should land the reader at that snippet's passage. The find
  bar is reused as-is; only the search text changes — from the original query to
  the per-snippet "matched word + following word(s)" (Req 22–23).

## 7. Technical Considerations

### Threading the parameters
- Follow the existing `uid_prefix` / `uid_suffix` plumbing: add fields to
  `SearchParams` (`backend/src/types.rs`) and `QueryTask`
  (`backend/src/query_task.rs:81`/`:136`), default off/empty, and build them in
  the QML `get_search_params()` (`SuttaSearchWindow.qml:615`).

### One query per page — no per-record sub-queries needed
- **Tantivy (Fulltext):** the per-page `TopDocs` search already returns each
  matched document, and the `content` field is stored, so the full content for
  all 10 page records is already in hand inside `search_single_index`
  (`backend/src/search/searcher.rs:441`). Producing all snippets is a matter of
  finding every match position in that already-fetched content — **no extra
  query/round-trip per record.** The existing `SnippetGenerator`
  (`searcher.rs:431`) emits a single best fragment; for multi-snippet we need to
  enumerate all matched-term positions. The robust approach is to re-tokenize
  the stored content with the **same analyzer used by the index** and emit a
  fragment around each token whose stem matches a query term (this is what makes
  `pajahati` highlight `pajahitvā`). Reuse `fragment_around_text`
  (`query_task.rs:183`) for the per-occurrence excerpt windows and the existing
  `<span class='match'>` markup for highlight.
- **SQLite FTS5 (Contains):** the contains-match path already fetches matched
  source rows, so the record content is available without a second query
  (`suttas_contains_match_fts5` `query_task.rs:590`,
  `book_spine_items_contains_match_fts5` `query_task.rs:1171`). Enumerate all
  occurrences of the query term(s) in the fetched content and emit one fragment
  each. (FTS5's `snippet()`/`highlight()` aux functions return a single
  fragment; doing the enumeration in Rust keeps Fulltext and Contains snippet
  behaviour consistent and reuses `fragment_around_text`.)

### Data shape: one `SearchResult` per snippet
- `SearchResult` (`backend/src/types.rs:135`) carries a single `snippet:
  String`. The simplest representation that fits the existing flat list is to
  **emit multiple `SearchResult` rows for the same record** — one per snippet —
  when `show_all_snippets` is on. Each row reuses the record's metadata
  (uid, title, sutta_ref, table_name) and differs only in `snippet`. This keeps
  the QML side (`results_model`, the delegate) essentially unchanged: it still
  consumes a flat list of `SearchResult`. Keep the single-`snippet` fast path
  (one row per record) unchanged when `show_all_snippets` is false.
- Pagination stays **record-based**: the page still selects up to 10 records;
  the per-record snippet rows for those 10 records are what gets returned for the
  page (so a page may contain more than 10 result rows).
- Two more per-row fields ride on `SearchResult`: an `is_snippet` marker (record
  vs. expanded-snippet row, for record-count grouping) and a `find_query` (the
  matched word + following word(s) derived from that row's snippet, for the
  click-to-open find-bar jump — Req 22). `find_query` is populated for both
  single-snippet and multi-snippet rows. `show_header` is **not** stored on
  `SearchResult`; it is computed QML-side in `update_page()` from consecutive
  uids, since it depends on adjacency within the rendered page.

### Page-scoped computation + existing cache (invalidation is free)
- Compute multi-snippets only for the records actually returned for the
  requested page (`results_page(page_num)`).
- **Reuse the existing `RESULTS_PAGE_CACHE`** in `bridges/src/sutta_bridge.rs`
  (`fetch_and_cache_page`), which already stores the **highlighted** results per
  page keyed by `cache_key`. The cache key is
  `format!("{}|{}|{}", query_text, search_area_text, params_json_text)`
  (`sutta_bridge.rs:1793`), where `params_json_text` is the serialized
  `SearchParams`. Because the two new fields (`show_all_snippets`,
  `snippet_exclude`) live on `SearchParams`, they are automatically part of
  `params_json_text` — so toggling Show All Snippets or editing the exclusion
  list **invalidates the cache automatically** with no extra key plumbing. The
  Combined-mode cache (`sutta_bridge.rs:1688`, `|combined` suffix) inherits the
  same property.

### UI delegate (near-unchanged)
- The list keeps its fixed `item_height` delegate
  (`FulltextResults.qml:244`). No variable-height or nested-repeater changes are
  needed: more snippets simply mean more flat `results_model` items appended in
  the existing per-page populate loop (`FulltextResults.qml:167`).
- The only delegate change is gating the metadata header `RowLayout`
  (`FulltextResults.qml:301`) on a new `show_header` role; `update_page()`
  computes it per row (uid != previous uid) when appending to `results_model`.

### Click-to-open find-bar jump
- The find-on-open trigger is `SuttaSearchWindow.qml:1019–1027`: when the user
  preference `open_find_in_sutta_results` is on and a sutta result is opened
  (content-replace), it sets `root.pending_find_query = root.last_query_text`.
  Change this to prefer the clicked result's `find_query` (threaded through
  `result_data` / `new_tab_data`), falling back to `last_query_text` when empty.
- `find_query` derivation (QML-side, in `update_page()`): locate the first
  `<span class='match'>` in the row's snippet, take the substring from there to
  the end, **strip all remaining HTML tags** (the highlight refactor removes the
  nested-span case, but strip defensively rather than assuming a single tag
  pair), drop the leading/trailing `…` ellipses and trailing punctuation, then
  take the matched word + the following 1–2 words (the example `pajahitvā ṭhito`
  is matched word + one). Empty string if there is no match span. The find bar's
  existing matching tolerates punctuation/normalization differences vs. the
  rendered HTML.

### Highlight pipeline refactor (prerequisite — do before multi-snippet work)
Current state (the bug): `results_page` (`query_task.rs:2311`) ends every search
with `page.into_iter().map(|r| self.highlight_row(r))`. `highlight_row` (`:2361`)
regex-wraps **all** literal-query occurrences via `highlight_query_in_content` →
`highlight_text` (`:147`). Meanwhile the Fulltext snippet was **already**
highlighted by tantivy in `render_snippet` (`searcher.rs:836`, `.to_html()` of a
`Snippet`). So Fulltext snippets get the literal query term wrapped **twice** →
nested `<span class='match'><span class='match'>…</span></span>` (unintended,
visually invisible, but breaks tag parsing and the focal-only rule). Contains
snippets are plain (`fragment_around_query`) and rely on the central pass —
correct, no nesting.

Refactor to a **range-based, producer-owned** highlighter (Req 12b–12e):
- Add a shared helper set (e.g. a small `highlight` module or functions on
  `QueryTask`): `merge_ranges(Vec<Range<usize>>) -> Vec<Range<usize>>` (sort +
  coalesce overlapping/adjacent) and `wrap_ranges(text, &[Range]) -> String`
  (emit exactly one `<span class='match'>` per merged range — **non-nested by
  construction**), plus `literal_ranges(text, term)` (all normalized,
  case-insensitive occurrences) and a focal single-range variant.
- **FulltextMatch** (`render_snippet`): stop using `.to_html()`. Take
  `Snippet::fragment()` + `Snippet::highlighted()` (tantivy's stemmed ranges,
  available in tantivy 0.25) **unioned with** `literal_ranges(fragment,
  normalized_query)`, `merge_ranges`, then `wrap_ranges`. Result: stemmed +
  literal occurrences, each wrapped once (Req 12d), no nesting.
- **ContainsMatch** (`db_sutta_to_result` / `db_book_spine_item_to_result`):
  highlight the plain fragment via `wrap_ranges(fragment, literal_ranges(...))`
  (all occurrences for single-snippet) so the producer owns it (Req 12c).
- **Central `highlight_row` becomes a fallback:** highlight only when the snippet
  has no `<span class='match'>` yet (i.e. `!snippet.contains("class='match'")`).
  This keeps the remaining plain-snippet modes (TitleMatch, UidMatch, non-DPD
  dict) working, can never double-wrap, and automatically leaves the focal
  all-snippets rows (already highlighted) untouched.
- All-snippets emitters (Fulltext + Contains) then apply a **single focal range**
  via `wrap_ranges` and set `is_snippet: true`; the fallback guard leaves them
  alone, satisfying Req 12a.
- `results_page` remains the single point where `db_query_hits_count` (record
  total) is set — still the natural home for the exclusion filter (Req 13–15),
  applied after the per-area handler returns the (possibly expanded) page.

### Normalization for exclusion
- Reuse the existing Pāli normalization (the same `normalize_plain_text` /
  iti-sandhi + niggahita handling referenced around `query_task.rs:101`) on both
  the snippet text and the exclusion strings before the substring test, to get
  diacritic-insensitive matching.
- Exclusion runs in `results_page` after `highlight_row`, so snippets may contain
  `<span class='match'>` markup; strip tags before the substring test. Excluding
  rows must **not** change `db_query_hits_count` (Resolved Decision 1).

## 8. Success Metrics

- A fulltext search for `pajahati` with "Show All Snippets" on expands
  `cnd8/pli/ms` into the two expected snippets (the acceptance test in
  Requirement 12), each correctly highlighting its own occurrence.
- With the exclusion input `pajahitvā, na upādiyati, akusalaṁ`, snippets
  containing any of those (diacritic-insensitively) are removed from the
  displayed results.
- Pagination still shows 10 records per page; total record count is unchanged by
  toggling "Show All Snippets".
- Navigating next → prev returns to a page without recomputing snippets
  (observable: no recompute work / instant render from cache).
- With "Show All Snippets" off, search results and performance are unchanged
  from current behaviour.
- When a record expands into several snippets, the metadata header (sutta_ref /
  title / uid) appears only on the first snippet item of that record.
- Clicking the snippet `… pajahati na upādiyati *pajahitvā* ṭhito …` opens the
  record and the find bar jumps to `pajahitvā ṭhito` (that occurrence), not to
  the first `pajahati` in the page.

## 9. Resolved Decisions

These were clarified during PRD review and are now fixed requirements:

1. **Exclusion vs. record count.** A record whose snippets are **all** excluded
   is **not displayed** on the page (results are seen to narrow down). This does
   **not** adjust the displayed total result count — the user knows they are
   filtering records out, so the count may exceed the number of records shown.
   (Requirement 15.)
2. **No expand/collapse control.** "Expansion" means more flat result items, not
   a per-record toggle. Each snippet is an ordinary result delegate item; the
   delegate and its fixed item height are unchanged, apart from the `show_header`
   gating that hides the repeated metadata header on a record's follow-on snippet
   items. (Requirements 2, 18, 19, 21.)
3. **Library content fields.** Snippet extraction for Library uses the same
   plain-text content field used today; per-snippet page-number metadata is
   preserved.
4. **Multi-term / AND queries.** "All snippets" enumerates occurrences of
   **every matched term**, consistent with how the record was matched.
5. **Cache invalidation.** Handled automatically: the new params live on
   `SearchParams`, which is serialized into the existing cache key
   (`params_json_text`), so any filter change invalidates the cached pages with
   no extra plumbing.

## 10. Open Questions

(None outstanding. Implementation details — e.g. exact tantivy re-tokenization
approach for enumerating match positions — are left to the developer per the
Technical Considerations.)
