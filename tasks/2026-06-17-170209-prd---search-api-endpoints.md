# PRD: Fulltext & Contains Search over the Localhost API

**Date:** 2026-06-17
**Feature area:** Localhost HTTP API (`bridges/src/api.rs`) — search endpoints
**Status:** Draft

## 1. Introduction / Overview

The app runs a local HTTP server (Rocket, `bridges/src/api.rs`, bound to
`127.0.0.1:<api_port>`) that the browser extension and other local clients use.
Today it exposes only two search endpoints:

- `POST /suttas_fulltext_search` — **misleadingly named**: it actually runs
  `ContainsMatch` (or `UidMatch` for reference-like queries), never the
  Tantivy-backed `FulltextMatch`. It hardcodes `page_len: 20`,
  `show_all_snippets: false`, `snippet_exclude: None`, and has no Library
  support.
- `POST /dict_combined_search` — dictionary search (`DpdLookup` / `UidMatch`).

The recently completed **"Show All Snippets & Snippet Exclusion Filter"** work
(see [archived PRD](./archive/2026-06-16-085912-prd---show-all-snippets.md))
deliberately kept all data-shaping — per-occurrence snippet expansion,
producer-owned non-nested highlighting, and the exclusion filter — on the shared
backend path (`SearchQueryTask::results_page`). Because the API endpoints already
call `results_page`, the highlight, multi-snippet, and exclusion behaviour are
**already produced backend-side and serialized into the `SearchResult` JSON for
free**; the only outstanding work to expose them over the API is request
plumbing.

This feature makes Fulltext Match and Contains Match search **fully accessible
over the localhost API via curl**, returning JSON with paginated results,
highlights, and both single-snippet and all-snippets modes — matching what the
in-app `SuttaSearchWindow` already produces. It does this through:

1. A general `POST /search` route that accepts the full parameter surface
   (mode, search area, pagination, snippet options, filters).
2. A `POST /suttas_fulltext_search` route repurposed to run **real
   FulltextMatch** (tantivy), exposing all the parameters the UI uses.
3. A new `POST /suttas_contains_search` route for ContainsMatch.
4. Shared helper functions in `api.rs` so the three routes do not duplicate the
   request → `SearchParams` → `SearchQueryTask` → JSON logic.

## 2. Goals

1. Add a general `POST /search` endpoint that can run any search **mode** in any
   **search area** (Suttas, Library, Dictionary) with the full parameter set,
   returning the same paginated, highlighted `SearchResult` JSON the UI uses.
2. Repurpose `POST /suttas_fulltext_search` to run **FulltextMatch** (tantivy)
   over Suttas, exposing pagination, highlights, single-/all-snippets mode, and
   the snippet-exclusion filter.
3. Add `POST /suttas_contains_search` for ContainsMatch over Suttas, with the
   same parameter surface.
4. Extract shared helpers in `api.rs` so the three search routes share one
   request-parsing / param-building / execution / response path with no
   duplication.
5. Expose `show_all_snippets` and `snippet_exclude` over the API so a curl
   client gets per-occurrence expanded snippets and exclusion filtering exactly
   as the in-app results do.
6. Make `page_num` and `page_len` client-controllable for true pagination.
7. Keep `POST /dict_combined_search` working (it may be reimplemented in terms
   of the shared helpers but its existing request/response contract must not
   change).

## 3. User Stories

- **As a developer / power user**, I want to run
  `curl -X POST localhost:<port>/search -d '{"query_text":"pajahati","mode":"Fulltext Match","search_area":"Suttas","show_all_snippets":true}'`
  and get JSON with one highlighted snippet per occurrence, so I can script
  corpus analysis outside the GUI.
- **As a script author**, I want to page through a large result set by sending
  `page_num` and `page_len`, so I can fetch all results in batches.
- **As an API client**, I want a dedicated `/suttas_fulltext_search` that does
  real fulltext (stemmed) matching and a `/suttas_contains_search` for literal
  substring matching, so I can pick the matching semantics explicitly without
  building the full `/search` body.
- **As a researcher**, I want to pass `snippet_exclude` over the API to suppress
  unwanted forms/phrases, narrowing the JSON results the same way the in-app
  exclusion filter does.

## 4. Functional Requirements

### Request shape

1. The system must extend the search request struct (currently
   `ApiSearchRequest`) with the additional fields needed to drive a full search,
   all **optional with sensible defaults** so existing clients keep working:
   - `mode: Option<String>` — deserialized using the **exact `SearchMode` serde
     names** (e.g. `"Fulltext Match"`, `"Contains Match"`, `"Combined"`,
     `"DPD Lookup"`, `"Uid Match"`).
   - `search_area: Option<String>` — exact `SearchArea` serde names
     (`"Suttas"`, `"Library"`, `"Dictionary"`).
   - `page_len: Option<i32>`.
   - `show_all_snippets: Option<bool>`.
   - `snippet_exclude: Option<Vec<String>>` (already-split array; the API client
     sends an array, not a CSV string — CSV parsing is a QML-side concern).
   - Existing fields retained: `query_text`, `page_num`, `suttas_lang`,
     `suttas_lang_include`, `dict_lang`, `dict_lang_include`, `dict_dict`,
     `dict_dict_include`.
2. Request bodies must use the exact `SearchMode` / `SearchArea` serde names so
   no string-mapping layer is required (the values match what QML sends and what
   `serde` already deserializes). An unrecognized `mode` / `search_area` must
   produce an HTTP 400 with a clear error message, not a silent fallback.

### General `POST /search` route

3. The system must add a `POST /search` route that accepts the extended request
   and can execute **any** search mode in **any** search area (Suttas, Library,
   Dictionary).
4. `POST /search` must default `search_area` to `"Suttas"` when omitted. The
   default `mode` is **area-specific**, matching the `SearchBarInput.qml`
   dropdown defaults (index 0): for Suttas/Library, default to `"Fulltext Match"`;
   for Dictionary, default to `"Combined"`. A minimal `{"query_text": "..."}`
   body therefore runs Suttas FulltextMatch, and
   `{"query_text": "...", "search_area": "Dictionary"}` runs Combined.
5. For the **Dictionary** area, `POST /search` must apply the dictionary
   language/source filters (`dict_lang`, `dict_dict`) and include the
   `deconstructor` field in the response (same logic as `/dict_combined_search`),
   so `/search` is a strict superset.
6. For the **Suttas / Library** areas, `POST /search` must apply the suttas
   language filter (`suttas_lang`) and leave `deconstructor` as `None`.
7. `POST /search` must honor `show_all_snippets` and `snippet_exclude` for the
   Suttas/Library areas exactly as the backend already does (these are produced
   in `results_page`). For Dictionary, they pass through with no effect (single
   definition per entry).

### `POST /suttas_fulltext_search` (repurposed)

8. The system must change `POST /suttas_fulltext_search` to run
   **`SearchMode::FulltextMatch`** (tantivy) over `SearchArea::Suttas`, replacing
   the current ContainsMatch placeholder. The browser extension's results from
   this endpoint will change accordingly; no legacy alias is kept.
9. It must expose the full parameter surface: `page_num`, `page_len` (client
   override; default per Req 14), `suttas_lang` / `suttas_lang_include`,
   `show_all_snippets`, and `snippet_exclude`.
10. It must keep the existing **sutta-reference auto-detection**: if
    `query_text_to_uid_field_query(query_text)` returns a `uid:`-prefixed query
    (e.g. for `"sn56.11"`, `"MN 44"`, `"dhp182"`), the route runs
    `SearchMode::UidMatch` instead; FulltextMatch is the fallback for ordinary
    queries.

### `POST /suttas_contains_search` (new)

11. The system must add a `POST /suttas_contains_search` route that runs
    **`SearchMode::ContainsMatch`** over `SearchArea::Suttas`, with the same
    parameter surface and the same sutta-reference → `UidMatch` auto-detection as
    Req 9–10.

### Shared helpers (no duplication)

12. The three search routes (`/search`, `/suttas_fulltext_search`,
    `/suttas_contains_search`) and the retained `/dict_combined_search` must
    share helper functions in `api.rs` that:
    - build a `SearchParams` from the request fields (mode, page_len, language /
      source filters, `show_all_snippets`, `snippet_exclude`, and the existing
      defaults), and
    - construct + run the `SearchQueryTask`, returning the `ApiSearchResult`
      (`hits`, `results`, `deconstructor`), with error handling that logs and
      returns an empty result set on failure (matching current behaviour).
    The mode-specific routes must be thin wrappers that set their fixed
    mode/area and the reference auto-detection, then call the shared helper.

### Pagination, highlights, snippets (shared backend behaviour)

13. All search routes must return paginated results keyed by `page_num`
    (record-based pagination, as the backend already implements). `hits` is the
    record total (`SearchQueryTask::total_hits()`), unchanged by snippet
    expansion or exclusion (consistent with the Show All Snippets PRD,
    Resolved Decision 1).
14. `page_len` must be client-controllable via the request; when omitted it
    defaults to **20** (the current browser-extension default) to preserve
    existing client behaviour.
15. Results must carry the existing producer-owned, **non-nested**
    `<span class='match'>` highlighting, the `is_snippet` marker on
    expanded-snippet rows, and per-occurrence snippets when
    `show_all_snippets` is true — all of which the backend already serializes
    into `SearchResult`. The API does not re-shape, re-highlight, or post-process
    results.

## 5. Non-Goals (Out of Scope)

- **`show_header` and `find_query`** — these two values are derived QML-side in
  `FulltextResults.update_page()` (uid adjacency; parsing the snippet HTML) and
  are **not** stored on `SearchResult`. The API will not compute them; a client
  that wants them can recompute trivially from the returned rows (uid adjacency;
  parse the snippet `<span class='match'>`). No backend change to move them onto
  `SearchResult`.
- **Changing the `SearchResult` / `ApiSearchResult` response JSON shape** beyond
  what the backend already serializes. No new response fields.
- **CSV parsing for `snippet_exclude`** on the API side — the API accepts a JSON
  array; CSV-splitting stays a QML/UI concern.
- **Persisting** any API search parameters; each request is stateless.
- **New highlight styling**, regex/boolean exclusion logic, or snippet-based
  pagination — unchanged from the in-app feature.
- **Authentication / remote exposure** — the server stays bound to
  `127.0.0.1`; no auth is added.
- **Changing `/dict_combined_search`'s request/response contract** — it may be
  refactored onto the shared helpers but its observable behaviour stays the same.

## 6. Design Considerations

- This is a backend/API-only change (Rust in `bridges/src/api.rs`, possibly small
  touches to `backend/src/types.rs` if the request struct lives there — but
  `ApiSearchRequest` is API-local, so keep it in `api.rs`). No QML changes.
- Register the two new routes (`search`, `suttas_contains_search`) in the
  `routes![...]` mount list in `start_webserver()`.
- Keep the helper functions private to `api.rs` (`fn build_search_params(...)`,
  `fn run_search(...)` or similar), mirroring the existing private helpers
  (`convert_verse_ref_to_sutta_uid`, `lookup_sutta_with_fallback`).
- Suggested `/search` request example (exact serde names):
  ```
  curl -X POST localhost:<port>/search \
    -H 'Content-Type: application/json' \
    -d '{"query_text":"pajahati","mode":"Fulltext Match","search_area":"Suttas","page_num":0,"page_len":10,"show_all_snippets":true,"snippet_exclude":["upādiyati"]}'
  ```

## 7. Technical Considerations

### Fulltext searcher initialization in the webserver (resolved design)
- **The webserver is a thread in the same process.** `cpp/gui.cpp:483` spawns
  `std::thread daemon_server_thread(start_webserver)`. The API thread and the
  QML/GUI threads therefore share the one **process-global** `FULLTEXT_SEARCHER`
  static (`backend/src/lib.rs:158`). The API needs **no separate searcher
  instance** — only that the global be initialized when a fulltext query runs.
- **The query path does not self-init.** `query_task.rs` reaches the searcher
  only via `with_fulltext_searcher(...)` (`:2169`/`:2221`/`:2266`), which returns
  `None` when uninitialized → logs *"Fulltext searcher not initialized"* and
  returns **empty results silently** (see `project_fulltext_searcher_init_separate`).
  So a fulltext route that runs before init returns zero hits with no error.
- **Do NOT eagerly init at `start_webserver()`.** Right after the webserver
  thread is spawned, `gui.cpp:495+` runs `reconcile_dict_indexes_blocking_c()`,
  which performs **Tantivy writes** and then `reinit_fulltext_searcher()`. The
  searcher is deliberately not opened until QML `SuttaBridge::load_searcher()`
  runs (after the reconcile window closes) precisely so index writes never
  contend with a live reader. Eager init on the API thread at startup would
  reintroduce that contention and pay the cold-storage index-open cost even for
  clients that never query fulltext.
- **Resolved approach — lazy, idempotent, mode-gated.** The shared `run_search`
  helper must call `init_fulltext_searcher()` **only when the resolved mode needs
  the Tantivy index** (`FulltextMatch`, and `Combined` for Suttas/Library),
  immediately before running the query. `init_fulltext_searcher()` is idempotent
  (early-returns if already `Some`), so in steady state — any realistic external
  curl / browser-extension request, long after the UI finished starting — it is a
  no-op. It only does real work in the edge case where QML init never ran, which
  is exactly the case that would otherwise return silent-empty. Concurrency is
  safe: searcher access is behind an `RwLock`; the API and QML threads are
  concurrent readers.

### Existing param threading is done
- The `show_all_snippets` / `snippet_exclude` fields already exist on
  `SearchParams` (`backend/src/types.rs:114–119`) and the two `SearchParams { .. }`
  literals in `api.rs` already set them to `false` / `None`. The shared helper
  just needs to read them from the request instead of hardcoding.

### Mode / area parsing
- `SearchMode` and `SearchArea` already `#[derive(Deserialize)]` with the serde
  renames (`types.rs:56–82`). Parse the request strings via
  `serde_json::from_value`/`from_str` into those enums, or a small match, and
  return `Status::BadRequest` on an unknown value (Req 2).

### Reuse the existing search path
- All routes call `SearchQueryTask::new(...)` + `results_page(page_num)` +
  `total_hits()` exactly as the current endpoints do. No backend search logic
  changes; the highlight/expansion/exclusion all happen inside `results_page`.

### Dictionary deconstructor
- For the Dictionary area, reproduce the `dpd_deconstructor_list(&query_text)`
  call from `dict_combined_search` so `/search` returns the same `deconstructor`
  field; for Suttas/Library it is `None`.

## 8. Success Metrics

- `curl -X POST localhost:<port>/suttas_fulltext_search -d '{"query_text":"pajahati","show_all_snippets":true}'`
  returns JSON where a multi-occurrence sutta yields multiple `is_snippet: true`
  rows, each with a single non-nested `<span class='match'>` highlight.
- `curl ... /suttas_contains_search -d '{"query_text":"pajahati"}'` returns
  ContainsMatch (literal) results; `pajahitvā` is not highlighted for query
  `pajahati`.
- `POST /search` with `mode: "Fulltext Match"` / `"Contains Match"` /
  `"Combined"` and the matching `search_area` returns results equivalent to the
  in-app results for the same query/params.
- Sending `page_num` and `page_len` pages through results correctly; `hits`
  stays constant across pages and equals the record total.
- `snippet_exclude: ["..."]` drops matching snippets (diacritic-insensitive),
  with the record total unchanged.
- A reference query (`"sn56.11"`) to `/suttas_fulltext_search` or
  `/suttas_contains_search` returns the referenced sutta via UidMatch.
- An unrecognized `mode`/`search_area` returns HTTP 400.
- `/dict_combined_search` behaves exactly as before.

## 9. Resolved Decisions

1. **Searcher init.** Lazy, idempotent, mode-gated `init_fulltext_searcher()`
   inside the shared `run_search` helper (only for FulltextMatch / Combined),
   **not** eager at `start_webserver()`. See §7 for the full rationale (shared
   process-global searcher, silent-empty trap, reconcile-write contention,
   RwLock concurrency safety).
2. **`/search` default mode.** Area-specific, matching the `SearchBarInput.qml`
   dropdown defaults (index 0): Suttas/Library → `"Fulltext Match"`, Dictionary →
   `"Combined"`. See Req 4.

## 10. Open Questions

(None outstanding.)
