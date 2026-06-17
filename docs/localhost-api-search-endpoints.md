# Localhost API search endpoints

**Status:** implemented (see `tasks/2026-06-17-170209-prd---search-api-endpoints.md`
and the matching `...-tasks-...` file).

The app runs a local HTTP server (Rocket, `bridges/src/api.rs`, bound to
`127.0.0.1:<api_port>`) used by the browser extension and other local clients.
This document describes the four search endpoints it exposes, their request /
response JSON, and the shared helpers behind them.

All search routes reuse the in-app search path
(`SearchQueryTask::new` + `results_page(page_num)` + `total_hits()`), so the
returned results carry exactly the same producer-owned, **non-nested**
`<span class='match'>` highlighting, `is_snippet` markers, per-occurrence
snippet expansion, and snippet-exclusion behaviour as the in-app results. See
[search-snippet-highlight-pipeline.md](./search-snippet-highlight-pipeline.md)
for how the snippet/highlight stages work; this doc only covers the API plumbing.

---

## 1. The four endpoints

| Route | Default mode (non-reference) | Area | Deconstructor |
|-------|------------------------------|------|---------------|
| `POST /search` | area-specific (see §4) | any (request-driven) | Dictionary only |
| `POST /suttas_fulltext_search` | `FulltextMatch` (tantivy) | Suttas | `None` |
| `POST /suttas_contains_search` | `ContainsMatch` (literal) | Suttas | `None` |
| `POST /dict_combined_search` | `DpdLookup` | Dictionary | yes |

The three Suttas/general routes plus `/dict_combined_search` are mounted in the
`routes![...]` list in `start_webserver()`.

> **Note — `/suttas_fulltext_search` changed.** It previously ran `ContainsMatch`
> (despite the name). It now runs real `FulltextMatch` (tantivy). No legacy alias
> is kept; clients wanting literal substring matching use
> `/suttas_contains_search`.

---

## 2. Request shape (`ApiSearchRequest`)

All fields except `query_text` are optional; serde deserializes a missing
`Option` field to `None`, so existing clients that send only a subset keep
working unchanged.

```jsonc
{
  "query_text": "pajahati",        // required
  "page_num": 0,                    // default 0
  "page_len": 20,                   // default 20

  // General /search only (the named routes hardcode mode + area):
  "mode": "Fulltext Match",         // exact SearchMode serde label (see §3)
  "search_area": "Suttas",          // exact SearchArea serde label (see §3)

  // Suttas/Library areas:
  "suttas_lang": "en",
  "suttas_lang_include": true,
  "show_all_snippets": true,        // default false; per-occurrence expansion
  "snippet_exclude": ["upādiyati"], // JSON array (NOT a CSV string)

  // Dictionary area:
  "dict_lang": "en",
  "dict_lang_include": true,
  "dict_dict": "PTS",
  "dict_dict_include": true
}
```

- `snippet_exclude` is an **already-split array**; CSV-splitting is a QML/UI
  concern, not done API-side.
- The language/source filters treat the placeholder values `"Languages"` /
  `"Language"` (and `"Dictionaries"` / `"Dictionary"` for the source) — and the
  empty string — as **no filter**.

## 3. Exact mode / area serde names

Request strings must match the `SearchMode` / `SearchArea` serde labels exactly
(`backend/src/types.rs`); an unrecognized value on `/search` returns **HTTP 400**.

`mode`:
`"Combined"`, `"Fulltext Match"`, `"Contains Match"`, `"Headword Match"`,
`"Title Match"`, `"DPD ID Match"`, `"DPD Lookup"`, `"Uid Match"`,
`"RegEx Match"`.

`search_area`: `"Suttas"`, `"Library"`, `"Dictionary"`.

## 4. `POST /search` mode / area resolution

- `search_area` defaults to `"Suttas"` when omitted; an unknown value → 400.
- `mode` defaults are **area-specific** (matching the `SearchBarInput.qml`
  dropdown index 0): Suttas/Library → `"Fulltext Match"`, Dictionary →
  `"Combined"`. An explicitly-sent unknown `mode` → 400.
- `/search` honors the requested mode **strictly** — there is no
  reference → `UidMatch` override (that lives only on the named convenience
  routes, see §5).

**Dictionary `Combined` is special.** `SearchQueryTask` rejects
`Combined + Dictionary` (it is bridge-orchestrated and would error → empty
results). So when `/search` resolves to `Dictionary` + `Combined` (the default,
or an explicit request), it maps it to the **`/dict_combined_search` behaviour**:
a UID-pattern query → `UidMatch`, otherwise `DpdLookup`, plus the `deconstructor`.
For Suttas/Library, `Combined` is fine — `results_page` maps it to
`FulltextMatch` internally.

For the Dictionary area, `/search` applies the dict language/source filters and
returns the `deconstructor` (computed from the original query via
`dpd_deconstructor_list`), so `/search` is a strict superset of
`/dict_combined_search`. For Suttas/Library it applies the suttas language
filter and `deconstructor` is `None`.

## 5. Named-route UID auto-detect

`/suttas_fulltext_search` and `/suttas_contains_search` keep a sutta-reference
auto-detect: if `query_text_to_uid_field_query(query_text)` returns a
`uid:`-prefixed query (e.g. for `"sn56.11"`, `"MN 44"`, `"dhp182"`), the route
runs `UidMatch` instead of its fallback mode. `/dict_combined_search` does the
same for dictionary UID patterns. `/search` does **not** do this (mode is
strict).

## 6. Pagination

Record-based pagination, driven by `page_num` + `page_len`. `hits` is the record
total (`SearchQueryTask::total_hits()`) and is **unchanged** by snippet expansion
or exclusion — it stays constant across pages. `page_len` defaults to 20.

## 7. Response shape (`ApiSearchResult`)

```jsonc
{
  "hits": 42,                       // record total (constant across pages)
  "results": [ /* SearchResult */ ],
  "deconstructor": ["a", "b"]       // Dictionary only; omitted when None
}
```

The API does **not** re-shape, re-highlight, or post-process results. It also
does not compute `show_header` / `find_query` (those are derived QML-side and are
not stored on `SearchResult`); a client can recompute them from the returned rows.

## 8. Lazy, mode-gated fulltext searcher init

The webserver runs on a thread in the **same process** as the GUI
(`cpp/gui.cpp` spawns `start_webserver`), so it shares the one process-global
`FULLTEXT_SEARCHER` (`backend/src/lib.rs`). The query path does **not** self-init
the searcher — `with_fulltext_searcher(...)` returns `None` when uninitialized
and the query returns **silent-empty** results
(see `project_fulltext_searcher_init_separate`).

The shared `run_search` helper therefore calls
`simsapa_backend::init_fulltext_searcher()` **only** when the resolved mode needs
the Tantivy index (`FulltextMatch` or `Combined`), immediately before running the
query. `init_fulltext_searcher()` is idempotent (no-op if already loaded), so in
steady state — any realistic curl / browser-extension request, long after the UI
finished starting — it does nothing. It does real work only in the edge case
where QML init never ran (the case that would otherwise return silent-empty).

Init is **not** eager at `start_webserver()`: right after the webserver thread is
spawned, `gui.cpp` runs `reconcile_dict_indexes_blocking_c()` which performs
Tantivy **writes** then `reinit_fulltext_searcher()`. Opening a reader eagerly on
the API thread at startup would contend with those writes and pay the cold
index-open cost even for clients that never query fulltext. Concurrency is safe:
searcher access is behind an `RwLock`; the API and QML threads are concurrent
readers.

## 9. Shared helpers (`bridges/src/api.rs`)

- `parse_search_mode` / `parse_search_area` — request string → enum (exact serde
  labels), `None` on unknown (→ 400 on `/search`).
- `build_search_params(request, mode, area)` — builds the `SearchParams`
  literal: area-aware language/source filters, `page_len` (default 20),
  `show_all_snippets` / `snippet_exclude` from the request, defaults for the rest.
- `run_search(dbm, query_text, params, area, page_num, deconstructor)` — lazy
  mode-gated searcher init, then `SearchQueryTask` + `results_page` +
  `total_hits`, returning `ApiSearchResult`; logs and returns empty on error.
- `run_suttas_search(request, dbm, fallback_mode)` — shared body for the two
  named Suttas routes: reference → `UidMatch` auto-detect, else `fallback_mode`.

## 10. Verification (curl)

The Rocket app is launched via FFI (no standalone route test harness); verify
with `make build -B` plus manual curl against a running app. Examples (replace
`<port>` with the running `api_port`):

```sh
# Fulltext, per-occurrence snippets
curl -s -X POST localhost:<port>/suttas_fulltext_search \
  -H 'Content-Type: application/json' \
  -d '{"query_text":"pajahati","show_all_snippets":true}'

# Contains (literal): pajahitvā NOT highlighted for query pajahati
curl -s -X POST localhost:<port>/suttas_contains_search \
  -H 'Content-Type: application/json' -d '{"query_text":"pajahati"}'

# General /search — explicit mode + area, pagination, exclusion
curl -s -X POST localhost:<port>/search \
  -H 'Content-Type: application/json' \
  -d '{"query_text":"pajahati","mode":"Fulltext Match","search_area":"Suttas","page_num":0,"page_len":10,"show_all_snippets":true,"snippet_exclude":["upādiyati"]}'

# Dictionary via /search (default mode Combined → DpdLookup + deconstructor)
curl -s -X POST localhost:<port>/search \
  -H 'Content-Type: application/json' \
  -d '{"query_text":"dhamma","search_area":"Dictionary"}'

# Reference query on a named route → UidMatch
curl -s -X POST localhost:<port>/suttas_fulltext_search \
  -H 'Content-Type: application/json' -d '{"query_text":"sn56.11"}'

# Unknown mode → HTTP 400
curl -s -o /dev/null -w '%{http_code}\n' -X POST localhost:<port>/search \
  -H 'Content-Type: application/json' \
  -d '{"query_text":"x","mode":"Nope"}'   # → 400
```
