# Tasks: make the localhost API routes more tolerant, guessable, and self-describing

Generated from `tasks/2026-06-21-150400-api-routes-tolerance-feature-request.md`.
Companion reference doc: `docs/localhost-api-search-endpoints.md`.

**Scope:** the Rocket webserver in `bridges/src/api.rs` plus the backend resolver
helpers it delegates to (`backend/src/app_data.rs`, `backend/src/db/dictionaries.rs`,
`backend/src/helpers.rs`). Every change must be **additive / backward-compatible**:
no currently-successful request may change its response shape (success cases
unchanged byte-for-byte unless an explicit new opt-in flag is set).

**Hard backward-compat rules (user-stated):**

1. **Route names/paths must not change.** New behaviour is added via *new* routes
   (P1's `/word.json?<uid>` etc., P5's `/health`); existing paths keep their names
   and their argument forms. (No renaming `/suttas/<uid>`, etc.)
2. **Adding an HTTP status must not remove the body that was returned before.**
   Where a route now returns a status (e.g. 404-on-miss), it must still return the
   **same body value it returned before** — notably the **empty set `[]`** on a
   word miss, and `{"hits":0,"results":[]}` for a 0-hit search. An already-installed
   **browser extension** (or any existing client) making the same queries must keep
   receiving the **same results as before**: successful queries byte-for-byte
   identical, and a body-reading client that ignores the status code sees no change.
   The status code is purely an *additional* signal layered on top of the unchanged
   body.

## Internal API usage map (behaviour to preserve)

These routes are consumed by three distinct kinds of caller. Knowing which is
which is what bounds the "preserve existing behaviour" requirement: the
**in-app QML search does NOT use the HTTP search routes at all** — it calls the
Rust `SuttaBridge` directly — so the POST search routes and `/words/<uid>.json`
have **no internal QML/TS caller** and only the GET render + GUI-navigation
routes are exercised in-app.

| Caller | Files | Routes hit | Uid / arg form passed |
|--------|-------|-----------|-----------------------|
| **QML WebEngine HTML views** | `assets/qml/SuttaHtmlView_{Desktop,Mobile}.qml`, `assets/qml/DictionaryHtmlView_{Desktop,Mobile}.qml` | `GET /get_sutta_html_by_uid/<window_id>/<uid..>?<anchor>`, `GET /get_word_html_by_uid/<window_id>/<uid..>` | **per-segment `encodeURIComponent`, joined with raw `/`, plus a trailing `/`** — `uid.split("/").map(encodeURIComponent).join("/")`. So a space arrives as `%20` but the `/` separators stay literal; `anchor` is `encodeURIComponent`'d. |
| **In-app sutta page JS** (compiled to `assets/js/`) | `src-ts/helpers.ts`, `assets/js/suttas.js` | `GET /open_sutta_window/<uid>`, `GET /open_sutta_tab/<window_id>/<uid>?<anchor>`, `POST /open_book_page_tab`, `POST /dppn_lookup`, `POST /copy_to_clipboard`, `POST /logger`, `POST /open_external_url`, `GET /lookup_window_query/<text>` | `/open_sutta_*` pass the uid **raw, unencoded** (`${uid}`, e.g. `sn47.8/en/thanissaro` from an `ssp://suttas/...` link); `lookup_window_query` uses `encodeURIComponent(selected_text)`. |
| **C++ / Rust (in-process)** | `cpp/gui.cpp`, `cpp/window_manager.cpp`, `backend/src/app_data.rs` | none over HTTP — `render_sutta_html_by_uid` / `render_word_html_by_uid` are called **directly** by both the GET routes and (the same fns) elsewhere; the api note at `app_data.rs:416` says the renderer must match `api.rs::get_word_html_by_uid` behaviour. | n/a (the shared P2 resolver must keep the direct-call renderers byte-identical). |
| **Browser extension / external `curl` / agents** | external (not in this repo tree) | all four POST search routes, `/words/<uid>.json`, `/sutta_and_dict_search_options`, the GET render routes | the friction the PRD targets; these are where tolerance is added. |

**Constraints this imposes on the work:**

1. **Path routes (`<uid..>`) must keep accepting the QML form** — per-segment
   `%20`-encoded with raw `/` separators **and a trailing `/`**
   (`pathbuf_to_forward_slash_string` already normalizes both). P1 only *adds*
   query-param variants; it must not alter the `<uid..>` path routes.
2. **`render_word_html_by_uid` / `render_sutta_html_by_uid` are dual-purpose** —
   called both by the GET routes and directly in-app. The P2 shared resolver
   must be factored so the existing direct-call output is unchanged (the
   `app_data.rs:416/376` "to ensure consistent behavior" notes are load-bearing).
3. **P3's 404-on-miss and P4's auto-detect fallback have no internal QML/TS
   caller** (in-app search is bridge-direct, not HTTP), so the regression surface
   is the browser extension + external clients only — but the success-case body
   must still stay byte-for-byte identical (verbose/404 only change the miss case).
4. **`/open_sutta_*` raw-uid callers** must keep working — do not start requiring
   encoding on the GUI-navigation routes.

## Relevant Files

- `bridges/src/api.rs` - The Rocket webserver: all route handlers, `routes![...]` mount list, the `build_search_params` / `run_search` / `run_suttas_search` helpers, and the `mod tests` block. Primary file for every task.
- `backend/src/app_data.rs` - `render_word_html_by_uid` (api.rs:687 delegate, the looser resolver to factor out for P2), `render_sutta_html_by_uid`, `get_dpd_headword_by_uid`, `get_dpd_root_by_root_key`. Home for the new shared `resolve_word_uid` helper.
- `backend/src/db/dictionaries.rs` - `get_word`, `get_distinct_sources` (P6 junk filter), `get_distinct_languages`.
- `backend/src/db/mod.rs` / `backend/src/db/appdata.rs` - `get_sutta_languages`; add row-count helpers for `/health` (P5).
- `backend/src/helpers.rs` - `query_text_to_uid_field_query` (the uid auto-detect whose 0-hit forms P4 must repair; also `word_uid_sanitize`).
- `backend/src/lib.rs` - `AppGlobals` / `AppGlobalPaths` (db paths + `api_port` for `/health`), `FULLTEXT_SEARCHER`, `init_fulltext_searcher`, `with_fulltext_searcher`; add `is_fulltext_searcher_ready()` (P5).
- `backend/src/update_checker.rs` - `get_app_version()` (`/health` version field, P5).
- `assets/qml/SuttaHtmlView_{Desktop,Mobile}.qml`, `assets/qml/DictionaryHtmlView_{Desktop,Mobile}.qml` - **Read-only reference**: confirm the `<uid..>` path routes keep working with the per-segment-encoded + raw-`/` + trailing-`/` form (do not modify unless a regression appears).
- `src-ts/helpers.ts` - **Read-only reference**: the raw-uid `/open_sutta_*` callers (P7 naming must not break them).
- `docs/localhost-api-search-endpoints.md` - Companion doc; rewrite for the new tolerant behaviour and trim the now-fixed gotchas (Task 7).
- `bridges/src/api.rs` `mod tests` - Unit tests for `parse_*`; extend with resolver/normalization/`dict_sources`-filter tests.
- `backend/src/db/dpd_models.rs` - **(Task 1)** added `serde::Serialize` to `BoldDefinition` so the resolver's JSON lane can return bold-definition structured rows.
- `backend/tests/test_resolve_word_uid.rs` - **(Task 1)** new integration test for `resolve_word_uid` two-lane invariant + HTML parity (live DB).
- `backend/src/lib.rs` - **(Task 1)** `init_app_globals` / `init_app_data` converted to race-free `get_or_init` (the prior `get().is_none()` + `set().expect()` pattern aborted under parallel integration-test init). Also home for P5's `is_fulltext_searcher_ready()`.

### Notes

- **No standalone HTTP route test harness exists** (Rocket is launched via FFI). New automated tests are **unit tests** on the extracted helpers (`resolve_word_uid`, the `dict_sources` filter, the uid-normalization fn) in `mod tests` / backend `cargo test`, plus **manual `curl` verification** against a running app per `docs/localhost-api-search-endpoints.md §12`.
- Tests that need real data use the live appdata DB at the path in `CLAUDE.md` (do **not** gate behind `#[ignore]`; see `feedback_local_integration_tests`).
- **Build with `make build -B`** (not direct cmake). **Run tests only after all sub-tasks of a top-level task are done**, and `make qml-test` only if explicitly asked (see memory).
- After each top-level task the app must compile cleanly and existing tests pass; every change is additive, success-case responses unchanged byte-for-byte.
- `FulltextSearcher` readiness (P5) is currently only observable via `with_fulltext_searcher` returning `Some`/`None`; expose a small `is_fulltext_searcher_ready()` rather than running a throwaway query.

## Phase 3 review — consistency findings & guard-rails

Verified against the code before finalizing. Findings, most → least important:

1. **The JSON and HTML word routes are two views (structured vs rendered-HTML) of
   the SAME word, joined by uid (refines PRD P2; per user + memory
   `project-dpd-records-correlate-dict-words`).** `dpd_headwords` / `dpd_roots`
   hold the **structured** data; the **rendered HTML** for those same words lives
   in the correlated `dict_words` record (joined by uid). That is why the two
   routes currently *look* like they reach different sets:
   - `get_word_json` (`api.rs:1303`) returns the **structured** row: `dpd_headwords` → `dpd_roots` → `dict_words`.
   - `render_word_html_by_uid` (`app_data.rs:434–450`) returns **HTML**: `bold_definitions` → `dict_words` (+ `word_uid_sanitize` retry) — and the `dict_words` row it finds **is** the rendered HTML of the correlated dpd record.

   **Implication for the unification (user decision):** the shared work is **uid
   resolution / correlation + normalization** — mapping every human/uid form
   (`dhamma 1.01`, `dhamma 1.01/dpd`, `dhamma-1-01/dpd`, numeric `34626/dpd`,
   `√kar/dpd`) onto the one canonical word — **not** building a new DPD HTML
   renderer (the HTML already exists in `dict_words`). So:
   - **Do NOT change `get_word_json`'s structured success shape** (PRD non-goal):
     it keeps returning the structured dpd/dict_words row.
   - **Extend each route to reach the correlated record via the shared resolver:**
     `get_word_json` gains the forms it currently misses (e.g. the human/`%20`
     forms, bold-definition uids) by resolving to the canonical uid first; the
     HTML route gains the numeric `34626/dpd` / `√kar/dpd` forms by correlating
     them to their `dict_words` HTML row.
   - The resolver still returns a **kind discriminant** so the JSON route can pick
     the structured row and the HTML route the rendered `dict_words` row, but the
     kinds are *correlated*, not disjoint. **Adjustment applied:** Tasks 1.1–1.5
     rewritten around uid correlation; no separate DPD HTML renderer is needed.

2. **`run_search` consumes `query_text: String` + `params` by value and returns
   `Json<ApiSearchResult>` (P4 mechanics).** The 0-hit fallback must (a) **clone**
   `query_text_orig` and rebuild `params` *before* the first `run_search` call so
   the original survives for the re-run, and (b) inspect the returned
   `result.0.hits == 0` to decide. This is a clean reuse (no signature change) —
   **Adjustment applied:** Tasks 4.1–4.3 note the clone-before-call requirement.

3. **P3.3 sutta 404 has no resolver from Task 1 (word-only).** Detecting a missing
   sutta for `get_sutta_html_by_uid` / the new `/sutta_html` needs an existence
   check — **reuse `lookup_sutta_with_fallback` (`api.rs:162`)** (already used by
   `open_sutta_by_uid`) together with `convert_verse_ref_to_sutta_uid`, rather than
   inventing one. **Adjustment applied:** Tasks 2.3 and 3.3 name these helpers.

4. **Verbose envelope changes the response *type*, not just the body (P3.2).**
   `get_word_json` currently returns `Json<Vec<Value>>`; 404 needs
   `(Status, Json<Vec<Value>>)` and `?verbose=1` needs a *different JSON object*
   shape. Returning two different bodies from one handler means an
   **untyped `Json<serde_json::Value>`** (or an enum `Responder`) — pick one and
   keep the **non-verbose array** byte-identical. **Adjustment applied:** noted in
   Task 3.2.

5. **`/health` `counts` cost & honesty (P5).** `COUNT(*)` on `dpd_headwords` /
   `dict_words` is cheap, but make the handler resilient — a count error should
   return `null`/omit, not fail the whole `/health`. The PRD example shows
   `"counts": {...: 0}`; ensure a real count, and document that `0` means "DB not
   loaded / not installed", consistent with `fulltext_searcher_ready:false`.

6. **P6 is deprioritized — `dict_sources` junk is test-suite residue, not a user
   issue (user clarification).** The `ssp_test_abort_partial_*` labels only appear
   on the dev machine after running the test suite; real user installs don't have
   them. Task 6 is now **optional**; if done at all, a defensive prefix filter (a
   constant, not a hardcoded list) at `get_distinct_sources` is the only acceptable
   scope — do **not** build import-abort/startup cleanup. `/health` and
   `/sutta_and_dict_search_options` return `get_distinct_sources` as-is.

7. **Doc-trim coupling (P7).** `docs/localhost-api-search-endpoints.md` §13.3 and
   §12.2 currently *document the gotchas as permanent*; Task 7.2 must replace (not
   append to) those caveats, and the existing `app_data.rs:376/416` "to ensure
   consistent behavior" comments should be updated to point at the new shared
   resolver. The `find.ts` / snippet-pipeline docs are unaffected (no behaviour
   change there).

8. **VERIFIED against the live DB — the dpd↔dict_words correlation is via
   `lemma_1` sanitize, NOT uid equality; and the four uid forms do NOT collapse to
   one record (corrects Task 1.6's earlier over-claim).** Data confirms:
   `dpd_headwords` row `id=34626, uid=34626/dpd, lemma_1="dhamma 1.01"`; the
   `dict_words` counterpart is `uid=dhamma-1-01/dpd` (=`word_uid_sanitize(lemma_1)`
   + `/dpd`). **There is no `dict_words` row with uid `34626/dpd`.** Implications:
   - **Reaching the HTML row from a numeric `<id>/dpd`** requires a *lemma lookup +
     sanitize* (headword by id → `lemma_1` → `word_uid_sanitize` → `get_word`), not
     a string-equal uid join. `render_word_html_by_uid` today does **not** do this,
     so it currently returns a blank page for `34626/dpd` — Task 1.5's "gain the
     numeric form" is a genuine new capability, implemented via this lemma step.
   - **The JSON route must keep its existing per-uid record choice (back-compat):**
     `34626/dpd` → the **dpd_headwords** structured row; `dhamma-1-01/dpd` → the
     **dict_words** structured row. These are *different records of the same word*
     and both must stay as today. So the unification is **two preserved lanes**, not
     a collapse: the only *new* JSON resolutions are the previously-`[]` **human
     forms** (`dhamma 1.01`, `dhamma 1.01/dpd`), which normalize into the
     **dict_words lane** (canonical `dhamma-1-01/dpd`) — they do **not** start
     returning the headword-id record.
   - **Roots follow the same pattern (verified):** `dpd_roots.uid` is the root-key
     form (`√akkh/dpd`); the `dict_words` row is the sanitized root *word*
     (`√path-1/dpd`, `√rudh-etc/dpd`), identical to the root-key form only when
     undisambiguated. JSON lane uses `get_dpd_root_by_root_key`; HTML lane reaches
     `dict_words` via the same `word_uid_sanitize` retry.
   - **Adjustment applied:** Task 1.2 reworded to the lemma/root sanitize
     correlation; Task 1.6 assertion corrected to the two-lane invariant
     (human/lemma forms → `dhamma-1-01/dpd` dict_words record; `34626/dpd` →
     headword record unchanged; HTML route renders the *same entry* for all four).

**No blocking inconsistencies** — the task ordering is sound (1 → {2,3,4} → 5,6 →
7; Task 1 is correctly the shared dependency). The above are refinements folded
into the sub-tasks below.

## Evaluation — should bootstrap add a uid-mapping field? (DECIDED: code-only)

**Decision (user):** **code-only correlation for this feature — no schema change,
no re-bootstrap, no DB version bump.** Task 1.2 implements the `lemma_1` + sanitize
mapping in `resolve_word_uid`. The explicit mapping-field idea below is kept only as
documented background / a possible future improvement; it is **out of scope** here
and is **not** a task in this list.

Context (verified): the cross-table link is currently **implicit** — `dict_words.uid`
= `word_uid_sanitize(lemma_1) + "/dpd"`, with the StarDict import keyed by `lemma_1`
guaranteeing `dict_words.word ≡ dpd_headwords.lemma_1` 1:1. No explicit FK/id column
exists (`dict_words` has no `dpd_headword_id`; `dpd_headwords` has no `dict_word_uid`).
The numeric `<id>/dpd` → HTML mapping therefore needs a runtime
`lemma_1`-lookup + `word_uid_sanitize` round-trip across the two DBs.

**Recommendation — split into two decisions:**

- **For THIS feature (Task 1): implement the mapping in code, no schema change.**
  The lemma-sanitize correlation is correct, cheap (one indexed `uid` lookup per
  DB), needs **no re-bootstrap and no DB version bump**, and works on already-installed
  user DBs. This is what Task 1.2 specifies. Do not block the API work on a schema
  change.

- **Separately (optional future bootstrap improvement): add an explicit mapping
  column.** Worthwhile for ergonomics + robustness, but it is a **distinct,
  higher-cost change** and should be its own task/PRD, not bundled here. Trade-offs:
  - **Benefit:** removes the runtime dependency on `word_uid_sanitize` staying in
    lockstep with the bootstrap-time sanitize (a silent-break risk if either
    changes); turns the mapping into a direct `WHERE dpd_headword_id = ?` indexed
    lookup; clearer code.
  - **Preferred shape:** nullable indexed `dpd_headword_id` / `dpd_root_id` INTEGER
    columns on **`dict_words`** (in `dictionaries.sqlite3`), populated at bootstrap
    via the `lemma_1` join — rather than a `dict_word_uid` column on the dpd tables.
    Keeping the link in `dictionaries.sqlite3` avoids modifying the upstream-derived
    `dpd.sqlite3` (copied/migrated from Bodhirasa's export), serves the needed
    headword→dict_word direction *and* the reverse, and dovetails with the existing
    `lemma_1_dpd_headword_match` coupling.
  - **Costs / risks:** requires a **re-bootstrap of shipped DBs + a DB
    schema/version bump** (existing installs won't have the column until they
    re-download); adds invariant surface to the bootstrap pipeline; the **headword
    join is clean (`dict_words.word = dpd_headwords.lemma_1`) but the root join needs
    care** — `dict_words.word` (`√path 1`) carries disambiguation that
    `dpd_roots.root` (`√path`) does not, so root population must join on the
    sanitized/disambiguated form, not the bare root.
  - **Population point:** must run where both tables are available — the StarDict
    `dict_words` build (line ~23 of `cli/src/bootstrap/dpd.rs`) precedes
    `dpd_migrate` (line ~40), so populating the id needs a **post-migrate pass**
    joining `dict_words` ↔ `dpd_headwords`/`dpd_roots`.

  **→ Deferred (not in this feature).** If pursued later it would be its own PRD:
  add columns to the dictionaries schema + Diesel migration, populate in the
  bootstrap post-pass, index them, re-bootstrap, bump DB version, then simplify
  `resolve_word_uid` (Task 1.2) to read the id directly.

## Tasks

- [x] 1.0 Add a shared, tolerant word-uid resolver used by both the JSON and HTML word routes (P2, P4 dependency)
  - **Spec / deps:** build one backend resolver that maps any uid form onto the
    canonical word, exploiting the dpd↔dict_words uid correlation (Finding 1 /
    memory `project-dpd-records-correlate-dict-words`). The JSON route reads the
    **structured** row; the HTML route reads the correlated **`dict_words`** HTML
    row. **Every uid that resolves today must still resolve to the same record**
    (both the GET routes and the in-app direct renderer callers depend on it —
    `app_data.rs:376/416` notes).
  - [x] 1.1 Define `AppData::resolve_word_uid(&self, input_uid: &str) -> Option<ResolvedWord>` in `backend/src/app_data.rs` (a method — `AppData` already owns `self.dbm`, so no separate `dbm`/`app_data` params; Finding 8). `ResolvedWord` carries the **canonical uid**, a **kind discriminant** (`BoldDefinition` / `DpdHeadword` / `DpdRoot` / `DictWord`; note DPPN is a `DictWord` whose renderer branches on `dict_label`, not a separate kind), the **structured value** (for JSON), and the **correlated `dict_words` row when present** (for HTML). Expose `as_json()` (structured row) and `html_dict_word()` (correlated row) accessors.
  - [x] 1.2 Implement resolution + correlation (Finding 8 — make it a method on `AppData`, which already owns `self.dbm`, rather than passing `dbm` + `app_data` separately): normalize the input (1.3), then locate the word across `bold_definitions` (`dbm.dpd.get_bold_definition_by_uid`), `dpd_headwords` (numeric `<id>/dpd`), `dpd_roots` (`√…/dpd`), and `dict_words.get_word` (+ `word_uid_sanitize` retry). **The dpd→dict_words correlation is via `lemma_1` sanitize, not uid equality:** for a numeric `<id>/dpd`, fetch the headword by id → `word_uid_sanitize(lemma_1)` + `/dpd` → `get_word` to reach the HTML row (there is no `dict_words` row with the numeric uid). Preserve the two record lanes: `34626/dpd` keeps resolving to the dpd_headword structured row; the human/lemma forms (`dhamma 1.01`, `dhamma 1.01/dpd`, `dhamma-1-01/dpd`) resolve to the `dhamma-1-01/dpd` dict_words row.
  - [x] 1.3 Add a small pure normalization fn (e.g. `normalize_human_word_uid`) covering the documented human forms (numbered display `dhamma 1.01` → `dhamma-1-01/dpd`; numeric `<id>/dpd`; trailing `.json` trim) and unit-test it in isolation.
  - [x] 1.4 Rewrite `get_word_json` (`api.rs:1288`) to delegate to `resolve_word_uid` via `as_json()`, **preserving the structured one-element-array success shape byte-for-byte** (PRD non-goal: do not switch JSON to the dict_words HTML row) and the `[]`/miss shape (status change is Task 3).
  - [x] 1.5 Route `render_word_html_by_uid` through the resolver: dispatch by kind to the existing renderers, rendering the **correlated `dict_words` row** for the `DpdHeadword`/`DpdRoot` kinds (no new DPD HTML renderer needed — the HTML already lives in `dict_words`). Verify the in-app direct-call output is unchanged for forms it already handled, and that the newly-reachable numeric/`√…` forms now render the same entry the JSON route resolves.
  - [x] 1.6 Add `mod tests` for the **two-lane invariant** (Finding 8), NOT a single-record collapse: assert (a) the human/lemma forms `dhamma 1.01`, `dhamma 1.01/dpd`, `dhamma-1-01/dpd` resolve to the **same `dhamma-1-01/dpd` dict_words record**; (b) `34626/dpd` resolves to the **dpd_headword** structured row (unchanged); (c) the HTML route renders the **same entry** for all four (numeric form via the lemma-sanitize correlation). Build with `make build -B`.
- [ ] 2.0 Add encoding-agnostic query-parameter word/sutta retrieval routes (P1, the `%2F` → 422 trap)
  - **Spec / deps:** Rocket decodes query strings fully (incl. `%2F`), unlike `<uid..>` path segments. Add **new** query-param routes that delegate to the same resolver (Task 1) / render fns; **leave the existing `<uid..>` path routes untouched** so the QML per-segment-encoded + raw-`/` + trailing-`/` callers keep working. Register every new route in `routes![...]` (`api.rs:1378`).
  - [ ] 2.1 Add `#[get("/word.json?<uid>")]` delegating to `resolve_word_uid`, returning the same body as `/words/<uid..>` (honouring Task 3 status semantics).
  - [ ] 2.2 Add `#[get("/word_html?<window_id>&<uid>")]` delegating to `render_word_html_by_uid` (same resolver), returning `RawHtml<String>`.
  - [ ] 2.3 Add `#[get("/sutta_html?<window_id>&<uid>&<anchor>")]` mirroring `get_sutta_html_by_uid` (so a `%2F`/`%20` sutta uid has an encoding-agnostic variant). Sutta existence/normalization reuses `convert_verse_ref_to_sutta_uid` + `lookup_sutta_with_fallback` (`api.rs:138/162`), **not** the word resolver (Finding 3).
  - [ ] 2.4 Mount the new routes in `routes![...]` and confirm the existing path routes still parse the QML trailing-slash form (manual curl: `%2F` query-param variant resolves the same record as the raw-`/` path route).
- [ ] 3.0 Distinguish "not found" from "found but empty"; opt-in verbose envelope (P3)
  - **Spec / deps:** depends on Task 1 (resolver returns `Option`). **404-on-miss is the chosen primary mechanism** (user decision: it's beneficial and easily adopted in the existing tuple-returning routes); the `?verbose=1` envelope is a secondary opt-in nicety, not an either/or with 404. Success case stays `200` + one-element array byte-for-byte; only the **miss** branch and an explicit `?verbose=1` change. Adopt the honest-status pattern wherever a route currently returns `200` on a genuine miss (the word JSON/HTML routes and the sutta HTML route in 3.1/3.3; `open_sutta_by_uid` already returns 404).
  - [ ] 3.1 Change `get_word_json` (and the Task-2 `/word.json` variant) to return **HTTP 404 while still returning the existing `[]` body** when `resolve_word_uid` is `None` (Hard rule 2 — the empty set must still be in the body so a body-reading browser-extension client is unaffected); keep `200` + `[record]` on success. Use `(Status, Json<Vec<Value>>)` like the existing tuple-returning routes.
  - [ ] 3.2 Add an opt-in `?verbose=1` envelope (`{ "found": bool, "query_uid", "canonical_uid", "hint" }`) gated so the default (no `verbose`) response is **byte-identical** to today. Because verbose returns a *different JSON shape*, make the verbose handler return an untyped `Json<serde_json::Value>` (or an enum `Responder`) while the non-verbose path keeps the bare array (Finding 4); the hint lists the tried normalized forms and points at `/health` for "is the source dict installed?".
  - [ ] 3.3 Apply 404-on-miss to the HTML routes: for `get_word_html_by_uid` (+ `/word_html`) gate on `resolve_word_uid` being `Some` before rendering; for `get_sutta_html_by_uid` (+ `/sutta_html`) gate on `lookup_sutta_with_fallback` (Finding 3). Return `(Status::NotFound, RawHtml(..))` on miss, `200` otherwise — success HTML body unchanged.
  - [ ] 3.4 Add `mod tests` for the not-found status and the verbose envelope shape (unknown uid → 404 / `{found:false}`); build with `make build -B`.
- [ ] 4.0 Make the UID auto-detect self-correcting — no silent 0-hit (P4)
  - **Spec / deps:** `query_text_to_uid_field_query` (`helpers.rs:56`) flags `dhamma 1.01` / `dhamma 1.01/dpd` as uid-like and yields `uid:dhamma 1.01/dpd`, which `UidMatch` can't find (stored uid is `dhamma-1-01/dpd`). Only the **empty** branch changes; any query returning ≥1 hit today is untouched.
  - [ ] 4.1 In `dict_combined_search` (`api.rs:1158`): when the auto-detected `UidMatch` run returns 0 hits, transparently re-run the original query as `DpdLookup` before returning. Since `run_search` consumes `query_text`/`params` by value, **clone `query_text_orig` and rebuild `params` before the first call** and branch on the returned `result.0.hits == 0` (Finding 2); keep the deconstructor from the original query.
  - [ ] 4.2 In `search` (`api.rs:1099`) Dictionary+Combined path: apply the same 0-hit `UidMatch` → `DpdLookup` fallback. (`/search` stays strict for an *explicitly* requested mode — only the Combined-resolved-to-UidMatch auto path falls back.)
  - [ ] 4.3 For the named suttas routes (`run_suttas_search`, `api.rs:1056`): when reference auto-detect → `UidMatch` returns 0 hits, fall back to the route's `fallback_mode` (`FulltextMatch` / `ContainsMatch`), using the same clone-before-call pattern (Finding 2). Optionally normalize the detected uid via Task 1.3 first.
  - [ ] 4.4 Update the two stale comments (`api.rs:1162` and `api.rs:1319`) to describe the real, now-self-correcting behaviour.
  - [ ] 4.5 Add `mod tests` / manual-curl note asserting `{"query_text":"dhamma 1.01"}` to `/dict_combined_search` returns ≥1 hit; build with `make build -B`.
- [ ] 5.0 Add a `GET /health` environment/readiness endpoint (P5)
  - **Spec / deps:** a single read-once JSON document: `app_version` (`update_checker::get_app_version`), `api_port` + `db_paths` (`AppGlobalPaths`), `fulltext_searcher_ready`, `counts` (suttas / dict_words / dpd_headwords), `sutta_languages`, `dict_sources` (from `get_distinct_sources` as-is; Task 6 is optional/deprioritized). New route only; `GET /` stays the landing page.
  - [ ] 5.1 Add `is_fulltext_searcher_ready() -> bool` in `backend/src/lib.rs` (reads `FULLTEXT_SEARCHER` `RwLock`, returns `guard.is_some()` — no throwaway query).
  - [ ] 5.2 Add count helpers (`suttas`, `dict_words`, `dpd_headwords`) on the respective DB modules (`COUNT(*)`), or reuse existing ones if present.
  - [ ] 5.3 Define a `HealthInfo` serde struct and `#[get("/health")]` handler assembling version, port, db paths (via `pathbuf_to_forward_slash_string`), readiness, counts, languages, and `get_distinct_sources` as-is; register in `routes![...]`.
  - [ ] 5.4 Document `/health` in §14 of the doc (done fully in Task 7) and verify with manual curl; build with `make build -B`.
- [ ] 6.0 (OPTIONAL — deprioritized) `dict_sources` aborted-import labels (P6)
  - **Status (user clarification):** the `ssp_test_abort_partial_*` entries are
    **residue from running the test suite on the dev machine**, **not** an issue on
    real user installations. So P6 is **not required** for the feature's goal and
    can be **skipped** unless wanted as a defensive nicety. Do **not** invest in the
    "real fix" (cleanup at import-abort/startup) the PRD floated — there is no user
    impact. Left here for traceability against PRD G6/P6.
  - [ ] 6.1 (optional) If implemented at all, add a defensive prefix filter for
    `ssp_test_abort_partial_*` in `get_distinct_sources` (`dictionaries.rs`),
    keeping the existing empty-string filter + sort; otherwise close this task as
    "won't do — test-suite artifact only" and note it in the doc.
- [ ] 7.0 Consistency nits, stale-comment fixes, and documentation rewrite (P7 + doc)
  - **Spec / deps:** depends on Tasks 1–6 landing. No behaviour change beyond doc/comment clarity and optional naming.
  - [ ] 7.1 Clarify the `GET /suttas/<uid>` GUI-navigation route in the doc (it pokes the GUI, returns no content) — document it clearly under §14.2; do **not** rename the path (the raw-uid `src-ts/helpers.ts` callers depend on it). Note the intentional optional `.json` on `get_word_json`.
  - [ ] 7.2 Rewrite `docs/localhost-api-search-endpoints.md`: add the new `/word.json?<uid>` / `/word_html` / `/sutta_html` query-param routes and `/health`; replace the `%2F`-trap and silent-`[]` caveats (§12.2, §13.3) with the new tolerant behaviour and the 404/verbose semantics (noting the 404 still carries the prior `[]` body so body-reading clients are unaffected); document the self-correcting auto-detect (§5). Route names unchanged.
  - [ ] 7.3 Add an agent-facing "how to search suttas and the dictionary, and retrieve complete HTML pages" walkthrough (search → copy `uid` → fetch full HTML/JSON), making the happy path obvious in one read.
  - [ ] 7.4 Update the notable-docs entry for `localhost-api-search-endpoints.md` in `AGENTS.md` (the real file — `CLAUDE.md` is a symlink; writes through it are refused) to reflect the new tolerant routes; run `make test` (full suite) once, per the after-all-subtasks rule.
