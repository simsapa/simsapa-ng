# Tasks: Fulltext & Contains Search over the Localhost API

PRD: [2026-06-17-170209-prd---search-api-endpoints.md](./2026-06-17-170209-prd---search-api-endpoints.md)

## Relevant Files

- `bridges/src/api.rs` - All work lives here. Extend `ApiSearchRequest`; add private parsing helpers (`parse_search_mode`, `parse_search_area`) and the shared `build_search_params` / `run_search` helpers; repurpose `suttas_fulltext_search`; add `suttas_contains_search` and the general `search` routes; refactor `dict_combined_search` onto the helpers; register the two new routes in the `routes![...]` mount in `start_webserver()`.
- `backend/src/types.rs` - Reference only: `SearchMode` / `SearchArea` serde-renamed enums (`:56`–`:82`), `SearchParams` (`:86`, incl. `show_all_snippets` / `snippet_exclude` already present), `SearchResult` (`is_snippet` already present). No change expected unless an enum needs `Serialize` for round-tripping a parse (parse via a small match, so likely no change).
- `backend/src/lib.rs` - Reference only: `init_fulltext_searcher()` (`:308`, idempotent), `reinit_fulltext_searcher()` (`:321`), `with_fulltext_searcher()` (`:338`), `FULLTEXT_SEARCHER` global (`:158`). The shared helper calls `init_fulltext_searcher()` lazily for fulltext-needing modes.
- `backend/src/query_task.rs` - Reference only: `SearchQueryTask::new(dbm, query_text, params, area)`, `results_page(page_num)`, `total_hits()`. The API path already calls these; no change.
- `backend/src/helpers.rs` - Reference only: `query_text_to_uid_field_query` (used for the sutta-reference → UidMatch auto-detect on the named routes).
- `cpp/gui.cpp` - Reference only: `start_webserver` runs on `std::thread` (`:483`) in the same process; reconcile/Tantivy writes happen after (`:495+`). Explains why fulltext init is lazy + mode-gated, not eager. No change.
- `docs/localhost-api-search-endpoints.md` - **new** doc: the three search routes + `/dict_combined_search`, request/response JSON shapes, exact mode/area serde names, pagination, `show_all_snippets` / `snippet_exclude`, the UID auto-detect on named routes, area-specific default modes, and the lazy mode-gated searcher init rationale. Cross-link to `docs/search-snippet-highlight-pipeline.md`.
- `PROJECT_MAP.md` / `AGENTS.md` (real target of the `CLAUDE.md` symlink) - add a pointer to the new doc.

### Notes

- This feature is **Rust-only** in `bridges/src/api.rs` — no QML, no new QML files to register in `bridges/build.rs`, no new Rust bridge to register.
- Build with `make build -B` (not direct cmake). Per project guidance, only run tests after all sub-tasks of a top-level task are done; skip `make qml-test` unless explicitly asked.
- There is no existing unit-test harness for `api.rs` routes (Rocket app is launched via FFI). Verification is by `make build -B` + manual `curl` against a running app (the user runs the GUI); the agent verifies via compile. Do not add `#[ignore]` DB-backed tests; if a pure helper (e.g. `parse_search_mode`) is unit-testable without a running server, a small `#[cfg(test)]` test in `api.rs` is welcome but optional.
- `ApiSearchRequest` is API-local — keep it in `api.rs`, do not move it to `backend`.
- Editing `CLAUDE.md` is refused (it is a symlink); edit `AGENTS.md` (the real target).

## Instructions for Completing Tasks

As you complete each sub-task, change `- [ ]` to `- [x]` in this file, updating after each sub-task (not just per parent task).

## Tasks

### Specs & dependencies for 1.0

- **Request fields (all optional, `serde` defaults so existing clients keep working):**
  `mode: Option<String>`, `search_area: Option<String>`, `page_len: Option<i32>`,
  `show_all_snippets: Option<bool>`, `snippet_exclude: Option<Vec<String>>`,
  plus existing `query_text`, `page_num`, `suttas_lang`, `suttas_lang_include`,
  `dict_lang`, `dict_lang_include`, `dict_dict`, `dict_dict_include`.
- **Parsing:** map request strings to enums via the **exact serde names**
  (`SearchMode`: `"Fulltext Match"`, `"Contains Match"`, `"Combined"`,
  `"DPD Lookup"`, `"Headword Match"`, `"Title Match"`, `"Uid Match"`,
  `"DPD ID Match"`, `"RegEx Match"`; `SearchArea`: `"Suttas"`, `"Library"`,
  `"Dictionary"`). Unknown value → caller can return HTTP 400 (Req 2).
- **No behaviour change yet:** the new fields are added but not consumed by the
  existing routes in this task (they keep their hardcoded params); 2.0 wires them.

- [x] 1.0 Extend `ApiSearchRequest` and add mode/area parsing
  - [x] 1.1 Add `mode: Option<String>`, `search_area: Option<String>`, `page_len: Option<i32>`, `show_all_snippets: Option<bool>`, `snippet_exclude: Option<Vec<String>>` to `ApiSearchRequest` (`api.rs:43`). Existing clients omit them → all `None`, so deserialization stays backward-compatible (confirm `serde` treats missing `Option` fields as `None`; add `#[serde(default)]` on the struct or fields if needed).
  - [x] 1.2 Add `fn parse_search_mode(s: &str) -> Option<SearchMode>` mapping the exact serde label strings to `SearchMode` variants (a `match` on the rename strings; returns `None` for unknown).
  - [x] 1.3 Add `fn parse_search_area(s: &str) -> Option<SearchArea>` likewise for `"Suttas"`/`"Library"`/`"Dictionary"`.
  - [x] 1.4 (Optional) Add a small `#[cfg(test)] mod tests` in `api.rs` asserting `parse_search_mode("Fulltext Match")`, `parse_search_area("Dictionary")`, and unknown-string → `None`.
  - [x] 1.5 Build (`make build -B`); confirm clean compile, existing endpoints unchanged.

### Specs & dependencies for 2.0

- **Depends on 1.0** (request fields + parsers).
- **`build_search_params(request, mode, area) -> SearchParams`:** centralize the
  `SearchParams { .. }` literal currently duplicated at `api.rs:945` and `:1038`.
  Reads: `mode` (passed in, resolved by the caller), `page_len`
  (`request.page_len.unwrap_or(20)` → `Some(..)`; Req 14), language filter
  (suttas vs dict depending on area — reuse the existing `"Languages"`/`"Language"`
  placeholder gating), source filter (dict only), `show_all_snippets`
  (`request.show_all_snippets.unwrap_or(false)`), `snippet_exclude`
  (`request.snippet_exclude.clone()` — already a `Vec<String>`, no CSV parsing).
  All other `SearchParams` fields keep their current defaults.
- **`run_search(dbm, query_text, params, area, deconstructor) -> Json<ApiSearchResult>`:**
  centralize the `SearchQueryTask::new` + `results_page(page_num)` + `total_hits()`
  + error-handling block (currently duplicated). Takes `page_num` too (or reads it
  before). On `Err`, log + return empty results (current behaviour). The
  `deconstructor` value is passed in (computed by the dict caller; `None` for
  suttas/library).
- **Lazy mode-gated fulltext init (Resolved Decision 1):** inside `run_search`,
  **before** running the query, if `params.mode` needs the Tantivy index
  (`FulltextMatch`, or `Combined`), call `simsapa_backend::init_fulltext_searcher()`
  (idempotent; no-op if already loaded). Do **not** call it for Contains/DPD/etc.
  Do **not** init eagerly in `start_webserver()`.
- **`/suttas_fulltext_search` behaviour change (Req 8–10):** switch the default
  (non-reference) mode from `ContainsMatch` to **`FulltextMatch`**; keep the
  `query_text_to_uid_field_query` → `UidMatch` auto-detect for reference queries;
  expose `page_len` / `show_all_snippets` / `snippet_exclude` via the request.
- **`/dict_combined_search` (Req 7, Non-Goal):** refactor onto the helpers with
  **no observable contract change** — same `DpdLookup`/`UidMatch` auto-detect,
  same `deconstructor`, same default `page_len: 20`.

- [x] 2.0 Add shared `build_search_params` / `run_search` helpers and refactor the existing endpoints onto them
  - [x] 2.1 Implement `build_search_params(...)` producing the `SearchParams` (area-aware language/source filters, `page_len` default 20, `show_all_snippets`, `snippet_exclude`, existing defaults for the rest).
  - [x] 2.2 Implement `run_search(...)` wrapping `SearchQueryTask::new` + `results_page(page_num)` + `total_hits()` + the empty-on-error path, returning `Json<ApiSearchResult>`. Include the lazy mode-gated `init_fulltext_searcher()` call for fulltext-needing modes.
  - [x] 2.3 Repurpose `suttas_fulltext_search` (`api.rs:920`): keep the reference → `UidMatch` auto-detect; otherwise use `SearchMode::FulltextMatch`; build params via `build_search_params` (area `Suttas`, suttas lang filter, `show_all_snippets`/`snippet_exclude`/`page_len` from request); call `run_search` with `deconstructor: None`.
  - [x] 2.4 Refactor `dict_combined_search` (`api.rs:996`) onto `build_search_params` + `run_search`: keep the `DpdLookup`/`UidMatch` auto-detect and the `dpd_deconstructor_list` computation; pass that `deconstructor` into `run_search`. Verify the response JSON is unchanged.
  - [x] 2.5 Build (`make build -B`) and confirm clean compile; the two `SearchParams { .. }` literals are now gone, replaced by `build_search_params`.

### Specs & dependencies for 3.0

- **Depends on 2.0** (helpers).
- **`POST /suttas_contains_search` (Req 11):** identical shape to the repurposed
  `/suttas_fulltext_search` but the non-reference mode is `SearchMode::ContainsMatch`;
  keep the same sutta-reference → `UidMatch` auto-detect; same param surface;
  `deconstructor: None`; area `Suttas`.

- [x] 3.0 Add the `POST /suttas_contains_search` route
  - [x] 3.1 Add the `suttas_contains_search` route fn mirroring `suttas_fulltext_search` but defaulting the non-reference mode to `ContainsMatch`; reuse `build_search_params` + `run_search`. (Consider extracting the shared "suttas search with reference auto-detect, given a fallback mode" body into one helper that both named routes call, parameterized by the fallback `SearchMode`.)
  - [x] 3.2 Build (`make build -B`); confirm clean compile (route registration happens in 5.0, but the fn must compile).

### Specs & dependencies for 4.0

- **Depends on 2.0** (helpers) and 1.0 (parsers).
- **`POST /search` (Req 3–7):** general route.
  - Resolve `search_area`: `parse_search_area(request.search_area)`; default
    `"Suttas"` when absent; unknown → **HTTP 400**.
  - Resolve `mode`: if `request.mode` present, `parse_search_mode`; unknown →
    **HTTP 400**. If absent, **area-specific default** (Req 4): Suttas/Library →
    `FulltextMatch`, Dictionary → `Combined`.
  - For Dictionary: apply dict lang/source filters and compute the
    `deconstructor` (same as `/dict_combined_search`); for Suttas/Library: suttas
    lang filter, `deconstructor: None`.
  - Honor `show_all_snippets` / `snippet_exclude` for Suttas/Library (backend
    already applies them); pass-through (no effect) for Dictionary.
- **HTTP 400 mechanics:** the other routes return `Json<ApiSearchResult>`. To
  return a 400, the handler's return type must allow it — e.g.
  `Result<Json<ApiSearchResult>, (Status, String)>` or a custom responder. Pick
  the simplest that Rocket supports and keep the success path returning the same
  JSON body.
- **Note on UID auto-detect:** `/search` honors the requested mode strictly (no
  reference → UidMatch override). The reference auto-detect lives only on the
  named convenience routes (matches the answered design: keep auto-detect on the
  named routes; `/search` is explicit).

- [x] 4.0 Add the general `POST /search` route
  - [x] 4.1 Add the `search` route fn with return type allowing a 400 (e.g. `Result<Json<ApiSearchResult>, (Status, String)>`).
  - [x] 4.2 Resolve area (default `Suttas`, unknown → 400) and mode (area-specific default when absent, unknown → 400) via the 1.0 parsers.
  - [x] 4.3 For Dictionary, compute the `deconstructor` (reuse `dpd_deconstructor_list` on the original query); build params with dict lang/source filters; for Suttas/Library use the suttas lang filter and `deconstructor: None`.
  - [x] 4.4 Build params via `build_search_params` and execute via `run_search` (which handles the lazy fulltext init for FulltextMatch/Combined).
  - [x] 4.5 Build (`make build -B`); confirm clean compile.

### Specs & dependencies for 5.0

- **Depends on 3.0 + 4.0** (the new route fns exist).
- **Mount (`api.rs:1278`):** add `search` and `suttas_contains_search` to the
  `routes![...]` list (the repurposed `suttas_fulltext_search` and refactored
  `dict_combined_search` are already mounted).
- **Verification (manual, user runs the GUI):** curl examples per PRD §8 —
  fulltext with `show_all_snippets`, contains literal-only, `/search` with each
  mode/area, pagination via `page_num`/`page_len`, `snippet_exclude`, a reference
  query on the named routes, an unknown mode → 400, and `/dict_combined_search`
  unchanged.

- [x] 5.0 Register routes, build, verify, and document
  - [x] 5.1 Add `search` and `suttas_contains_search` to the `mount("/", routes![...])` list in `start_webserver()`.
  - [x] 5.2 Build (`make build -B`); confirm clean compile and the full crate links.
  - [x] 5.3 Run `cd backend && cargo test` (ignore pre-existing unrelated failures per project guidance) to confirm nothing in `backend` regressed; run the optional `api.rs` parser tests if added. (216 lib + all functional tests pass; only pre-existing perf-timing tests `test_bold_definitions_highlighting` / `test_dpd_lookup` fail on duration thresholds, unrelated. The `api.rs` parser tests compile but can't run standalone — the `bridges` crate links against Qt/CXX-Qt and needs the CMake build env, per the task notes.)
  - [x] 5.4 Provide the curl verification command set (PRD §8) for the user to run against the live app; agent confirms compile + route registration. (Curl set in `docs/localhost-api-search-endpoints.md` §10.)
  - [x] 5.5 Write `docs/localhost-api-search-endpoints.md` documenting the four search endpoints, request/response JSON, exact mode/area serde names, pagination, `show_all_snippets`/`snippet_exclude`, named-route UID auto-detect, area-specific default modes, and the lazy mode-gated searcher-init rationale; cross-link `docs/search-snippet-highlight-pipeline.md`.
  - [x] 5.6 Add a pointer to the new doc in `PROJECT_MAP.md` and the notable-feature-docs list in `AGENTS.md` (the real target of the `CLAUDE.md` symlink).
