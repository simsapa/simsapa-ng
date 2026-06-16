# Search snippet & highlight pipeline (ContainsMatch / FulltextMatch)

**Status:** target design after the "Show All Snippets" refactor
(see `tasks/2026-06-16-085912-prd---show-all-snippets.md` and the matching
`...-tasks-...` file). This document describes how the **snippet** and
**highlight** stages of the Suttas / Library search pipeline are *intended* to
work once the highlight-pipeline refactor (PRD §Goal 9, Reqs 12b–12e) lands. It
complements [text-processing-for-contains-match-and-fulltext-match-search.md](./text-processing-for-contains-match-and-fulltext-match-search.md),
which covers how `content_plain` is normalized at bootstrap; here we cover what
happens at **query time**.

Two search modes produce result snippets for Suttas and Library:

- **ContainsMatch** — SQLite FTS5 (`trigram`) literal substring search. No Pāli
  stemming; matches the query string itself.
- **FulltextMatch** — Tantivy index with a Pāli stemmer; `pajahati` also matches
  inflected forms like `pajahitvā`.

The same two modes feed one results list (`FulltextResults.qml`), so their
snippet/highlight output must be **consistent in shape** (non-nested
`<span class='match'>` markup) even though they find matches very differently.

---

## 1. Why this refactor exists

Historically, highlighting happened in **two** places and could double-wrap:

```
FulltextMatch:  Tantivy renders snippet WITH <span class='match'>      (pass 1)
                          │
                          ▼
                results_page() → highlight_row() regex-wraps the literal   (pass 2)
                query term AGAIN  →  <span class='match'><span class='match'>…</span></span>
                                                       ^^^ nested, unintended
```

ContainsMatch produced a *plain* snippet and relied on pass 2, which was
correct (no nesting). The nested spans on FulltextMatch were visually invisible
(same CSS background) but break:

- **per-snippet tag parsing** (the find-bar "jump to this snippet" feature reads
  the first `<span class='match'>`), and
- the **focal-only** highlight rule needed by "Show All Snippets" (each expanded
  snippet must highlight only its own occurrence).

The fix: **highlighting becomes the snippet producer's responsibility**, built
on a single range-based helper that is *non-nested by construction*, and the old
central pass is demoted to a **fallback** for modes that still emit plain
snippets.

---

## 2. High-level pipeline

```
 QML (SuttaSearchWindow.qml)
   get_search_params()  ── JSON ──►  SuttaBridge.results_page(query, page, area, params_json)
                                                   │
                                                   ▼
                              bridges/src/sutta_bridge.rs
                              fetch_and_cache_page(cache_key, …)
                                 │   (cache_key = query | area | params_json)
                                 │   RESULTS_PAGE_CACHE: page_num → Vec<SearchResult>
                                 ▼
                       backend QueryTask::results_page(page_num)
                                 │
            ┌────────────────────┴─────────────────────┐
            ▼                                           ▼
   SearchMode::ContainsMatch                   SearchMode::FulltextMatch
   suttas_contains_match_fts5 /                fulltext_suttas / fulltext_library
   book_spine_items_contains_match_fts5          → search/searcher.rs
            │                                           │
            ▼                                           ▼
   db_*_to_result  (producer-owned highlight)   render_snippet (producer-owned highlight)
            │                                           │
            └────────────────────┬─────────────────────┘
                                 ▼
                  results_page() tail:
                    • highlight_row()  ── FALLBACK only (plain snippets)
                    • snippet_exclude  ── drop excluded snippets / empty records
                    • db_query_hits_count = record total
                                 │
                                 ▼
                   SearchResultPage { total_hits, page_len, page_num, results }
                                 │  (serialized, cached, emitted)
                                 ▼
            QML results_page_ready → FulltextResults.update_page()
                    • flatten Vec<SearchResult> into results_model
                    • show_header  (uid != previous uid)
                    • find_query   (parsed from snippet HTML)
```

Key invariants carried end-to-end:

- `total_hits` is always the **record** count, never the snippet count.
- `page_len` records per page; in "Show All Snippets" mode a page's
  `Vec<SearchResult>` may hold *more* rows than `page_len` (one per occurrence),
  but `total_pages = ceil(total_hits / page_len)` stays record-based.

---

## 3. The highlight model (producer-owned, range-based)

A single shared module (`backend/src/highlight.rs`) provides the only code that
writes `<span class='match'>`:

```
literal_ranges(text, term)      ─┐
Tantivy Snippet::highlighted()  ─┤
focal range (one occurrence)    ─┤
                                 ▼
                          merge_ranges()         ← coalesce overlapping/adjacent
                                 │
                                 ▼
                          wrap_ranges(text, ranges)
                                 │
                                 ▼
                  "<span class='match'>…</span>"   ← exactly one span per merged
                                                     range ⇒ NEVER nested
```

Because every span is emitted from a *merged, disjoint* range set, nesting is
impossible by construction. Producers choose **which ranges** to pass:

| Mode / situation        | Ranges passed to `wrap_ranges`                          |
|-------------------------|---------------------------------------------------------|
| ContainsMatch, single   | `literal_ranges` of the query (all occurrences)         |
| ContainsMatch, all-snip | one **focal** range (the occurrence this snippet is for)|
| FulltextMatch, single   | Tantivy stemmed ranges ∪ `literal_ranges`, merged       |
| FulltextMatch, all-snip | one **focal** range (this occurrence)                   |

### The fallback

`results_page()` still ends with `highlight_row()`, but it is now a **fallback**:

```rust
fn highlight_row(&self, mut r: SearchResult) -> SearchResult {
    if is_dpd_result(&r) { return r; }                 // DPD: never highlighted here
    if r.snippet.contains("class='match'") { return r; } // producer already did it
    // else: plain snippet (TitleMatch, UidMatch, non-DPD dict) → highlight now
    r.snippet = self.highlight_query_in_content(&normalize_plain_text(&self.query_text), &r.snippet);
    r
}
```

This means:

- Fulltext + Contains snippets (already highlighted by their producers) pass
  through untouched — **no second pass, no nesting**.
- Focal "all-snippets" rows pass through untouched — focal-only is preserved.
- TitleMatch / UidMatch / non-DPD dict (still plain) get highlighted as before —
  **no behaviour change** for those modes.

---

## 4. ContainsMatch (FTS5) — detailed sequence

`suttas_contains_match_fts5()` / `book_spine_items_contains_match_fts5()`:

1. **Build the FTS5 query.** `content_plain LIKE '%query%'` plus lang / CST /
   MS-mūla / `uid_prefix` / `uid_suffix` filters pushed into SQL. *Purpose:*
   trigram FTS5 does the candidate matching; all filters are bound parameters so
   the `LIMIT/OFFSET` is spent on rows that survive.
2. **Count.** `SELECT COUNT(*)` for the true record total. *Purpose:* pagination
   is record-based; the count must not depend on snippet expansion.
3. **Page fetch.** `SELECT s.* … ORDER BY s.id LIMIT page_len OFFSET page_num*page_len`.
   *Purpose:* fetch exactly the page's records; the literal query is guaranteed
   to be a substring of each row's `content_plain` (so occurrences exist).
4. **Snippet production (producer-owned highlight):**
   - **Single-snippet** (`show_all_snippets` off): `db_sutta_to_result()` builds
     one window via `fragment_around_query()` and highlights **all** literal
     occurrences in that window via `wrap_ranges(literal_ranges(...))`.
     `is_snippet = false`.
   - **All-snippets** (on): map the row to **N** `SearchResult`s — one per
     literal occurrence of the normalized query in `content_plain`. Each window
     is built with `fragment_around_offset()` (around that specific occurrence),
     and only the **focal** occurrence is highlighted. `is_snippet = true`.
     *Zero-occurrence fallback:* emit one single snippet so the record still
     appears.
5. Return `(Vec<SearchResult>, record_total)` to `results_page()`.

Contains never highlights inflected forms: query `pajahati` highlights
`pajahati`, never `pajahitvā` — because FTS5 matched the literal substring and
the producer only wraps literal ranges.

---

## 5. FulltextMatch (Tantivy) — detailed sequence

`fulltext_suttas()` / `fulltext_library()` → `search/searcher.rs`:

1. **Build the dual-field query** in `search_single_index()`: `content` (stemmed,
   `Must`) + `content_exact` (`Should`, boosted) + filter term-queries.
   *Purpose:* the stemmer surfaces inflections (`pajahati` → `pajahitvā`) while
   the exact field boosts literal hits.
2. **Per-language gather + cross-language merge** in `search_indexes()`: fetch
   `limit = (page_num+1)*page_len` per language index, collect `(score, …)`,
   sort by score, then `skip(page_num*page_len).take(page_len)`.
   *Purpose:* a single ranked page across all language indexes.
3. **Record-level slice is mandatory.** The slice must operate on **one entry per
   record** so a page always holds `page_len` records. Snippet expansion happens
   **after** the slice.

   ```
   gather (per lang)      merge+sort         slice (records)     expand (page only)
   ┌─────────────┐        ┌──────────┐       ┌────────────┐      ┌──────────────────┐
   │ docs×limit  │  ───►  │ by score │  ───► │ page_len   │ ───► │ N snippets/record│
   │ (+DocAddr)  │        └──────────┘       │ records    │      │ (is_snippet=true)│
   └─────────────┘                           └────────────┘      └──────────────────┘
   ```

   To expand post-slice, each scored result carries a `(lang_key, DocAddress)`
   handle so it can be re-associated with its own `(index, reader)` to re-fetch
   the stored `content`. *Purpose:* correctness (record-based pages) **and** a
   bounded cost — only `page_len` records get the heavy per-occurrence work,
   regardless of page depth or number of language indexes.
4. **Snippet production (producer-owned highlight) in `render_snippet`:**
   - **Single-snippet:** take Tantivy `Snippet::fragment()` +
     `Snippet::highlighted()` (stemmed ranges) **∪** `literal_ranges(fragment,
     query)`, `merge_ranges`, `wrap_ranges`. Highlights stemmed **and** literal
     occurrences in the one fragment, non-nested. `is_snippet = false`.
     (`render_snippet` is shared by the sutta/dict/library/bold doc-builders, so
     this also fixes Dictionary fulltext nesting.)
   - **All-snippets:** enumerate **all** occurrences across the doc's full
     `content` by re-tokenizing it with the index analyzer and matching tokens
     whose stem equals a query term's stem (every term, for AND queries). For
     each occurrence, window via `fragment_around_offset()` and highlight **only
     the focal** occurrence. One `SearchResult` per occurrence,
     `is_snippet = true`. *Zero-occurrence fallback:* emit the single best
     snippet.
5. Return `(Vec<SearchResult>, record_total)` to `results_page()`.

---

## 6. `results_page()` tail — shared finishing steps

After the per-mode handler returns the (possibly expanded) page:

1. **Dictionary inclusion-set post-filter** (Dictionary only — unrelated to this
   feature).
2. **`db_query_hits_count = record_total`** — the value QML divides by `page_len`
   for `total_pages`. *Never* the snippet count.
3. **Highlight fallback** (`highlight_row`, §3) over each row.
4. **Snippet exclusion** (`snippet_exclude`): drop any `SearchResult` whose
   snippet (tags stripped, `normalize_plain_text` applied to both sides) contains
   any CSV term. A record whose snippets are *all* excluded simply contributes no
   rows — it disappears from the page — but `db_query_hits_count` is left
   unchanged (the user knows they are filtering, so "shown < total" is expected).

---

## 7. QML render — `FulltextResults.update_page()`

The flat `Vec<SearchResult>` becomes `results_model` rows. Two values are derived
QML-side while appending:

- **`show_header`** = this row's `uid` differs from the previous appended row's
  `uid`. The delegate shows the metadata header (sutta_ref / title / uid) only
  when true, so a record's follow-on snippet rows read as one group. Item height
  is unchanged.
- **`find_query`** = parsed from the snippet HTML: the first
  `<span class='match'>` word plus the following 1–2 words (tags stripped,
  ellipses/punctuation dropped). On click, the find-bar jumps to *this* snippet's
  passage (`pajahitvā ṭhito`) instead of the original query.

`is_snippet` is carried for record-grouping/counting; `total_pages` stays derived
from the record-count `total_hits`.

---

## 8. Caching & invalidation

`RESULTS_PAGE_CACHE` (in `sutta_bridge.rs`) stores highlighted pages keyed by
`query | area | params_json`. Because `show_all_snippets` and `snippet_exclude`
live inside `SearchParams` (hence inside `params_json`), toggling either one
changes the key and **invalidates cached pages automatically** — no extra
plumbing. Prev/next navigation re-serves cached pages without recomputation.

---

## 8a. Reuse by the localhost API (deferred, but kept unblocked)

The Rocket endpoints in `bridges/src/api.rs` (e.g. `POST /suttas_fulltext_search`)
call the **same** `SearchQueryTask::results_page` the UI uses. As a result,
everything in §3–§6 is produced **backend-side** and is already serialized into
the `SearchResult` JSON those endpoints return: the producer-owned non-nested
highlight markup, the per-occurrence expansion (`is_snippet: true` rows), the
exclusion filter, and the record-based `total_hits`. Exposing Fulltext/Contains
search over curl with single-/all-snippets mode is therefore **request-plumbing
only** (add `mode` / `search_area` / `page_len` / `show_all_snippets` /
`snippet_exclude` to the request struct and set them on `SearchParams`) — it is
deferred to a later change.

The only two values an API client does **not** receive directly are
`show_header` and `find_query`, because they are derived in
`FulltextResults.update_page()` (§7), not stored on `SearchResult`. Both are
trivially recomputable by any client: `show_header` from `uid`/`is_snippet`
adjacency, and `find_query` from the (non-nested, hence parseable) snippet HTML.
**Invariant to preserve:** keep all data-shaping (expansion, highlight,
exclusion) in the backend on the `results_page` path — never move it into
`update_page()` — so the API and UI stay in parity.

---

## 9. Where to look in the code

| Concern                         | Location                                                        |
|---------------------------------|-----------------------------------------------------------------|
| Range highlighter               | `backend/src/highlight.rs` (`merge_ranges`/`wrap_ranges`/`literal_ranges`) |
| Windowing                       | `query_task.rs` `fragment_around_text` / `fragment_around_offset` |
| Highlight fallback              | `query_task.rs` `highlight_row`                                 |
| Contains handlers               | `query_task.rs` `suttas_contains_match_fts5`, `book_spine_items_contains_match_fts5` |
| Fulltext handlers / snippet     | `backend/src/search/searcher.rs` `search_indexes`, `render_snippet` |
| Occurrence enumerator (stemmed) | `searcher.rs` + `search/tokenizer.rs` analyzer                  |
| Page assembly / exclusion       | `query_task.rs` `results_page`                                  |
| Cache                           | `bridges/src/sutta_bridge.rs` `RESULTS_PAGE_CACHE`              |
| QML render                      | `assets/qml/FulltextResults.qml` `update_page`                  |
| Normalization (bootstrap)       | [text-processing doc](./text-processing-for-contains-match-and-fulltext-match-search.md) |
