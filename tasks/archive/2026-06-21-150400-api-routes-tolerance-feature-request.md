# Feature request: make the localhost API routes more tolerant, guessable, and self-describing

**Scope:** `bridges/src/api.rs` (the Rocket webserver: route handlers + helpers).
**Goal:** reduce the number of *verification* round-trips an external client (browser
extension, scripts, an LLM agent) must make to use the API correctly, by making the
handlers tolerant of the uid forms callers naturally have, honest about "not found",
and self-describing — **without changing any currently-successful request's
behaviour**.

This is written from real friction hit while driving the API headlessly with `curl`.
Each proposal lists the observed symptom, the relevant current code, a concrete
change, and a backward-compatibility note. Reproductions were run against a live
instance on `127.0.0.1:4848`; `PORT=4848` below.

---

## Motivation — the gotchas that cost round-trips today

| # | Gotcha (observed) | Where | Cost to caller |
|---|-------------------|-------|----------------|
| G1 | A percent-encoded `/` (`%2F`) in a uid path → **HTTP 422**, raw `/` works | every `<uid..>` route, e.g. `get_word_json`, `get_word_html_by_uid` | Caller must know to *not* encode `/`; the natural "URL-encode the path param" reflex fails. |
| G2 | `GET /words/<uid>.json` returns **`200 []`** for a wrong-but-plausible uid **and** for a genuinely-missing word — indistinguishable | `get_word_json` (api.rs:1288) | Caller can't tell "I built the uid wrong" from "no such word"; leads to guess-and-retry. |
| G3 | The JSON route resolves *fewer* uid forms than the HTML route for the same word | `get_word_json` vs `get_word_html_by_uid` / `render_word_html_by_uid` | `dhamma 1.01/dpd` and `dhamma/ncped` render as HTML but return `[]` as JSON — surprising inconsistency. |
| G4 | The display `title` ("dhamma 1.01") is **not** a usable uid; the uid is `34626/dpd` or the hyphenated `dhamma-1-01/dpd` | search results vs `get_word_json` | Caller must learn the hyphenation rule by trial. |
| G5 | Uid auto-detect produces a query that finds nothing | `dict_combined_search` (api.rs:1166), `query_text_to_uid_field_query` | `{"query_text":"dhamma 1.01"}` and `"dhamma 1.01/dpd"` → **0 hits**, though the code comment lists them as supported patterns. |
| G6 | `dict_sources` is polluted with aborted-import junk | `get_search_options` → `get_distinct_sources()` | `["dpd","dppn","ssp_test_abort_partial_1781…", …]` — caller can't trust the source list. |
| G7 | No single call returns version / live port / installed dictionaries / readiness | (none) | Caller makes several probe calls (`/`, `/sutta_and_dict_search_options`, a test search) to learn the environment. |

The **code comments are also stale** in two spots and actively mislead a reader:

- `dict_combined_search` (api.rs:1162): *"Check if query is a UID pattern (e.g.,
  `"dhamma 1.01"`, `"dhamma 1.01/dpd"`, `"123/dpd"`)"* — the first two return 0 hits.
- `get_word_json` (api.rs:1319): *"This handles UIDs like `"dhamma 1.01/dpd"` which
  are stored in dict_words"* — that exact form resolves to nothing; the stored uid is
  `dhamma-1-01/dpd`.

---

## Proposals

Ordered by value/effort. P1–P3 remove the most friction.

### P1 — Accept a percent-encoded uid (fix the `%2F` → 422 trap)

**Symptom (G1).**
```sh
curl -s -o /dev/null -w '%{http_code}\n' "localhost:$PORT/words/34626%2Fdpd.json"  # 422
curl -s -o /dev/null -w '%{http_code}\n' "localhost:$PORT/words/34626/dpd.json"    # 200
```
**Cause.** The word/sutta routes capture the uid as a multi-segment trailing param
(`#[get("/words/<uid_with_ext..>")]`, `uid: PathBuf`). Rocket does **not** decode
`%2F` into a path separator (a path-traversal safeguard), so a request that encodes
the `/` fails to match and 422s. Every `<uid..>` route inherits this.

**Proposal.** Add an **encoding-agnostic query-parameter variant** alongside (not
replacing) the existing path routes, so a caller can pass the uid exactly as it
appears in a `SearchResult.uid`, with or without encoding:

```rust
// New, tolerant of %2F / %20 (Rocket decodes query strings fully):
#[get("/word.json?<uid>")]
fn get_word_json_q(uid: &str, dbm: &State<Arc<DbManager>>) -> (Status, Json<Vec<Value>>) { … }

#[get("/word_html?<window_id>&<uid>")]
fn get_word_html_q(window_id: &str, uid: &str, …) -> RawHtml<String> { … }
```
Both delegate to the same resolver as the path routes (see P2). The existing
`/words/<uid..>.json` and `/get_word_html_by_uid/<window_id>/<uid..>` stay exactly
as they are.

**Backward-compat.** Purely additive; no existing route or response changes.
**Alternative (cheaper, no new routes):** keep path routes only and just guarantee
the docs say "raw `/`, never `%2F`" (already done in
`simsapa-localhost-api-search-endpoints.md` §13.3) — but a tolerant route removes the trap
entirely rather than documenting around it.

---

### P2 — One shared, tolerant word-uid resolver for both the JSON and HTML routes

**Symptom (G3, G4).** Same uid, two answers:
```sh
curl -s "localhost:$PORT/words/dhamma%201.01/dpd.json"            # 200 []   (JSON: not found)
curl -s "localhost:$PORT/get_word_html_by_uid/web/dhamma 1.01/dpd" # 200 47kB (HTML: found)
```
`get_word_json` (api.rs:1288–1334) does exact lookups —
`get_dpd_headword_by_uid(uid)`, `get_dpd_root_by_root_key(root_key)`,
`dictionaries.get_word(uid)` — whereas `render_word_html_by_uid` (called by
`get_word_html_by_uid`, api.rs:687) clearly resolves looser forms.

**Proposal.** Factor the (more tolerant) resolution used by
`render_word_html_by_uid` into a shared backend helper, e.g.

```rust
// returns the canonical uid + the structured record, or None
fn resolve_word_uid(dbm: &DbManager, app_data: &AppData, input_uid: &str)
    -> Option<(String /*canonical_uid*/, serde_json::Value)>;
```
and have **both** `get_word_json` and `get_word_html_by_uid` call it, so the JSON
route inherits the same tolerance for free. Include light normalization of the
common human forms in the resolver:

- numbered headword display form → stored uid: `dhamma 1.01` / `dhamma 1.01/dpd`
  → try `dhamma-1-01/dpd` (space/dot → hyphen) and the numeric `<id>/dpd`.
- trim a stray trailing `.json` (already handled in `get_word_json`).

Returning the **canonical uid** alongside the record also lets callers learn the
real uid in one call (see P3 envelope).

**Backward-compat.** Resolver is a superset: every uid that resolves today still
resolves to the same record; only previously-`[]` inputs start succeeding. The JSON
array shape is unchanged.

---

### P3 — Distinguish "not found" from "found but empty"; optionally hint

**Symptom (G2).** `get_word_json` returns `Json(Vec::new())` (HTTP **200**) for a
missing word, identical to a (hypothetical) real empty record. A caller cannot tell
a wrong uid from a real miss without another search call.

**Proposal (pick one, both backward-compatible for the success case):**

1. **Status only:** return **404** with the existing `[]` body when nothing
   resolves. Successful lookups keep `200` + the one-element array. Clients that
   only check the body are unaffected; clients that check status get a clear signal.
2. **Opt-in envelope:** add `?verbose=1` that wraps the result:
   ```json
   { "found": false, "query_uid": "dhamma 1.01/dpd",
     "canonical_uid": null,
     "hint": "no word for this uid; tried dhamma-1-01/dpd, <id>/dpd. Is the source dict installed? See /health." }
   ```
   Default (no `verbose`) keeps the current bare-array response byte-for-byte.

Apply the same 404-on-miss to `get_sutta_html_by_uid` / `get_word_html_by_uid`,
which currently always return `200` even when rendering an error/empty page — a
caller cannot detect a bad uid from the status line.

**Backward-compat.** Option 1 changes only the *status* of the not-found case (body
unchanged); option 2 is fully opt-in.

---

### P4 — Make uid auto-detect self-correcting (no silent 0-hit)

**Symptom (G5).**
```sh
curl -s -X POST "localhost:$PORT/dict_combined_search" \
  -H 'Content-Type: application/json' -d '{"query_text":"dhamma 1.01"}'      # hits 0
curl -s -X POST "localhost:$PORT/dict_combined_search" \
  -H 'Content-Type: application/json' -d '{"query_text":"dhamma 1.01/dpd"}'  # hits 0
```
`dict_combined_search` (api.rs:1166) and `search` (api.rs:1125) route a query that
`query_text_to_uid_field_query` flags as uid-like to `UidMatch`. When the
constructed `uid:` query matches nothing (because the human form ≠ stored uid), the
result is a confident **0 hits** with no fallback.

**Proposal.** When the auto-detected `UidMatch` returns **0 hits**, transparently
re-run the original query under the normal mode (`DpdLookup` for dictionary;
`FulltextMatch` for suttas) before returning. Equivalently, run the P2 normalization
on the detected uid first. Net effect: `"dhamma 1.01"` yields the headword results a
human expects instead of an empty set.

**Backward-compat.** Only triggers on the *empty* branch, so every query that
returns ≥1 hit today is unchanged. Update the two stale comments (api.rs:1162, 1319)
to match real behaviour.

---

### P5 — A `/health` (a.k.a. `/api_info`) endpoint to collapse environment probes

**Symptom (G7).** Today a caller probes `/` (liveness), then
`/sutta_and_dict_search_options` (filters), then a throwaway search (to confirm the
fulltext searcher is warm) just to understand the instance.

**Proposal.** Add `GET /health` returning a small JSON document the client can read
once:
```json
{
  "app_version": "0.4.4",
  "api_port": 4848,
  "db_paths": { "appdata": "…", "dictionaries": "…", "dpd": "…" },
  "fulltext_searcher_ready": true,
  "counts": { "suttas": 0, "dict_words": 0, "dpd_headwords": 0 },
  "sutta_languages": ["pli","en"],
  "dict_sources": ["dpd","dppn"]
}
```
`fulltext_searcher_ready` is especially useful — it tells a headless caller whether a
`FulltextMatch`/`Combined` query will return real results yet (the lazy init added in
`run_search`, api.rs:1032, is otherwise invisible).

**Backward-compat.** New route; nothing else touched. (`GET /` stays the static
landing page.)

---

### P6 — Don't surface aborted-import sources in `dict_sources`

**Symptom (G6).**
```sh
curl -s "localhost:$PORT/sutta_and_dict_search_options" | jq .dict_sources
# ["dpd","dppn","ssp_test_abort_partial_1781454983693", … many more … ]
```
`get_search_options` (api.rs:949) returns `dictionaries.get_distinct_sources()`
verbatim, including orphan rows left by aborted/partial dictionary imports
(`ssp_test_abort_partial_*`).

**Proposal.** Either (a) filter these out at the query/handler level so only
usable, fully-imported sources are returned, or better (b) clean them up at import
abort / on startup so the underlying data is correct (this likely also affects the
GUI's source filter, not just the API). The API-side filter is the quick win; the
data cleanup is the real fix.

**Backward-compat.** Removes only entries that resolve to no usable dictionary.

---

### P7 — Smaller consistency nits (low priority)

- **`/suttas/<uid>` naming.** It is a *side-effecting GUI navigation* route returning
  `200` + "The Simsapa window should appear…" (api.rs:1190), not content. The name
  invites callers to expect the sutta. Consider documenting it as
  `/open_sutta_window_msg` or grouping all GUI-navigation routes under an
  `/ui/…` prefix so "returns content" vs "pokes the GUI" is guessable from the path.
- **`get_word_json` accepts a missing `.json`.** `<uid_with_ext..>` + `trim_end_matches(".json")`
  means `/words/34626/dpd` (no extension) also works. Harmless, but worth either
  documenting as intentional or normalizing.

---

## Non-goals / behaviours to preserve

- The existing `<uid..>` **path routes keep working with raw `/`** exactly as now —
  P1 adds an alternative, it does not replace them.
- **Response shapes** for successful calls are unchanged: `/words/<uid>.json` still
  returns a one-element (or empty) JSON array unless `?verbose=1` is explicitly set.
- **Search semantics** for any query that returns ≥1 hit today are untouched; P4
  only fills the empty branch.
- `POST /search`'s strict `mode`/`search_area` validation (HTTP 400 on unknown) and
  the named convenience routes' reference→`UidMatch` auto-detect stay as documented.

## Suggested tests (mirroring `api.rs`'s `mod tests`)

- `%2F`-encoded uid via the new query-param route resolves the same record as the
  raw-`/` path route.
- `resolve_word_uid` maps `dhamma 1.01`, `dhamma 1.01/dpd`, `dhamma-1-01/dpd`, and
  `34626/dpd` to the same canonical uid + record.
- `get_word_json` of an unknown uid returns 404 (or `{found:false}` under verbose).
- `dict_combined_search {"query_text":"dhamma 1.01"}` returns ≥1 hit after P4.
- `get_distinct_sources()` (or the handler) excludes `ssp_test_abort_partial_*`.

---

*Companion reference:* `docs/simsapa-localhost-api-search-endpoints.md` documents the current
(pre-improvement) behaviour and the raw-`/`/`%2F`, hyphenation, and silent-`[]`
gotchas in §12.2 and §13.3. As proposals here land, those caveats can be trimmed.
