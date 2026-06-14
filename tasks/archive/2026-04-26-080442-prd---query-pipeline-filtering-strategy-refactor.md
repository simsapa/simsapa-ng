# Query Pipeline — Filtering-Strategy Refactor

Supersedes the §7 target design in
`tasks/analysis-dpd-bold-definitions-search.md`. That design moved every
filter into a Rust post-pass over a SAFETY-capped full fetch; with the
schemas now uniformly using `raw` for uid fields, we can push filters down
to the storage layer and restore native pagination.

## 1. Updated assumptions (what changed since the previous plan)

1. **All tantivy uid fields are `raw`.**
   - `build_sutta_schema` — `uid` already `raw_opts`
     (`backend/src/search/schema.rs:39`).
   - `build_library_schema` — `spine_item_uid` now `raw_opts` (line 89).
   - `build_dict_schema` — `uid` now `raw_opts` (line 137); also gains
     `is_bold_definition: bool` (line 148) and a `nikaya_group_path` raw
     field used by bold-definition rows.
   - `simple_fold` is no longer used for any uid field, so a `RegexQuery`
     against any uid field hits the term dictionary's btree directly and
     returns *exactly* the rows we want (no over-match, no Rust filter
     "source of truth" caveat).

2. **Dict and bold-definitions are one consolidated tantivy index.**
   `build_bold_definitions_schema` is gone; both kinds of docs use
   `build_dict_schema`, share one index directory, and are
   distinguished by an `is_bold_definition: bool` field (false for
   dict_words, true for bold definitions). The index has been
   re-bootstrapped against the new schema. BM25 scores are now
   internally consistent — no cross-index merge needed for Fulltext
   Dictionary searches.

3. **Existing add_dict_filters / add_library_filters still carry the
   "simple_fold over-match" caveat in their comments and still rely on
   the Rust-side `apply_uid_filters` as the source of truth**
   (`searcher.rs:623, 644`). With the schema change those caveats are
   stale — push-down is now exact.

4. **The current pipeline (analysis §7 / commit b18b4c0) does
   full-fetch + Rust filter for every uid-filtered query.** It compensates
   with a 300-row `SAFETY_LIMIT_TANTIVY` (down from 100k) and a
   `cached_full_fetch` to amortise pagination. Both are workarounds for
   the wrong filter layer, and the small cap silently truncates results
   on broad filtered queries.

5. **`SearchResult.score: Option<f32>`** is set by every tantivy path and
   `None` for SQL paths. That stays.

## 2. Findings

### 2.1 Why full fetch was chosen, and why it's no longer needed

The post-filter approach was chosen because uid filtering couldn't be
expressed accurately at the storage layer:
- tantivy uid fields were tokenized (`simple_fold` for dict/library), so
  a regex against them over-matched;
- SQL paths don't have a single uniform place to express
  `uid LIKE 'x%' AND uid LIKE '%y'` together — but this is actually
  trivial (it's just two extra WHERE clauses).

With schemas now `raw` everywhere, both blockers fall:
- **tantivy:** `RegexQuery::from_pattern("^bodhi.*$", uid_field)` and
  `RegexQuery::from_pattern("^.*mnt$", uid_field)` are exact and fast —
  raw fields store one term per document, so the pattern is matched
  against a small term dictionary, not against doc contents.
- **SQL:** prefix `uid LIKE 'x%'` uses the existing btree (analysis §1);
  suffix `uid LIKE '%y'` cannot use a btree but runs against the
  FTS5-narrowed candidate set, which is already small.

### 2.2 Cost of the current approach

- Every filtered fulltext query pays for snippet generation on
  `SAFETY_LIMIT_TANTIVY = 300` rows even when only `page_len = 10` are
  returned. SnippetGenerator was the dominant cost in the recent
  profiling (commit b18b4c0).
- 300 is small enough to silently truncate broad+filtered queries (e.g.
  `vinnana` + suffix `bodhi`). The user can't tell that they're seeing
  fewer than the true total.
- `cached_full_fetch` keeps the filtered result set in
  `SearchQueryTask` between paginated calls. This is correct for a
  single result page consumer but assumes no other call mutates the task
  in between — fragile.
- `apply_uid_filters` overrides `db_query_hits_count` with `filtered.len()`
  whenever any uid filter is set. With push-down, the hit count comes
  straight from the storage layer (tantivy `Count` collector / SQL
  `COUNT(*)`) and the override goes away.

### 2.3 What full fetch is genuinely required for

**Nothing, post-consolidation.** The previously hard case — dict + bold
score merge across two indexes — disappears because the two indexes
are one. A single tantivy query against the unified dict index returns
already-ranked, already-paginated results. No merge, no over-fetch.

### 2.4 SQL filter coverage today

The bold-definition FTS5 helpers (post Stage A of the previous refactor)
already push `uid LIKE ?%` down. The Suttas FTS5 paths push prefix down.
Dictionary `dict_words_contains_match_fts5` and Library
`book_spine_items_contains_match_fts5` do **not** push prefix down today —
they were left as a deliberate deferral (analysis §2.5) because the
post-filter caught them. With this refactor they need the push-down too,
plus the suffix push-down on every FTS5 helper.

## 3. Target pipeline

### 3.1 Filter-aware mode dispatch

```
results_page(page_num):
    let needs_filter = self.uid_prefix.is_some() || self.uid_suffix.is_some();

    let (page, total) = match self.search_mode_area() {
        FulltextMatch + Dictionary => self.fulltext_dict(page_num)?,   // bold-defs included via combined index
        FulltextMatch + Suttas     => self.fulltext_suttas(page_num)?,
        FulltextMatch + Library    => self.fulltext_library(page_num)?,
        ContainsMatch + Suttas     => self.suttas_contains_fts5(page_num)?,
        ContainsMatch + Dictionary => self.dict_words_contains_fts5(page_num)?,
        ContainsMatch + Library    => self.library_contains_fts5(page_num)?,
        DpdLookup                  => self.dpd_lookup(page_num)?,
        HeadwordMatch              => self.dpd_headword_match(page_num)?,
        TitleMatch + Suttas        => self.suttas_title_match(page_num)?,
        TitleMatch + Library       => self.library_title_match(page_num)?,
        UidMatch                   => self.uid_match(page_num)?,
    };

    self.db_query_hits_count = total as i64;
    Ok(page.into_iter().map(|r| self.highlight_row(r)).collect())
```

Each handler takes `page_num`, returns `(Vec<SearchResult>, total: usize)`,
and is responsible for pushing every filter — including uid prefix/suffix
— down to its storage layer. No Rust-side `apply_uid_filters` post-pass.
No `cached_full_fetch`. `SAFETY_LIMIT_*` survives only as a defensive
ceiling against pathological unbounded queries (raised back to ~50k for
SQL; tantivy's `TopDocs::with_limit(page_len)` doesn't need a separate
cap).

### 3.2 Tantivy push-down

Replace the existing `add_dict_filters` / `add_library_filters` body
(which only pushes prefix, with caveats) with exact push-down:

```rust
fn add_uid_filters(
    subqueries: &mut Vec<(Occur, Box<dyn Query>)>,
    filters: &SearchFilters,
    schema: &Schema,
    uid_field_name: &str,     // "uid" for sutta/dict, "spine_item_uid" for library
    uid_rev_field_name: &str, // "uid_rev" / "spine_item_uid_rev"
) -> Result<()> {
    if let Some(p) = filters.uid_prefix.as_deref().filter(|s| !s.is_empty()) {
        let field = schema.get_field(uid_field_name)?;
        let pattern = format!("{}.*", regex::escape(&p.to_lowercase()));
        subqueries.push((Occur::Must, Box::new(RegexQuery::from_pattern(&pattern, field)?)));
    }
    if let Some(s) = filters.uid_suffix.as_deref().filter(|s| !s.is_empty()) {
        // Match the suffix by querying the reversed-uid field as a prefix.
        let field = schema.get_field(uid_rev_field_name)?;
        let reversed: String = s.to_lowercase().chars().rev().collect();
        let pattern = format!("{}.*", regex::escape(&reversed));
        subqueries.push((Occur::Must, Box::new(RegexQuery::from_pattern(&pattern, field)?)));
    }
    Ok(())
}
```

Notes:
- Stored uids are always lowercase (invariant). `to_lowercase()` on the
  user input is therefore the only normalization needed.
- Tantivy `RegexQuery` is anchored end-to-end. Prefix-form (`x.*`) on
  either field walks only the term dictionary's btree — effectively
  O(log N) on number of unique terms. Both prefix and suffix queries
  reduce to prefix-on-some-field, so neither leans on a leading-`.*`
  scan of the entire term dict.
- The reversed-uid field (`uid_rev` for sutta/dict,
  `spine_item_uid_rev` for library) is added as a `raw` field
  containing the lowercased uid reversed character-by-character. Built
  at index time alongside the existing uid field; same storage cost
  (one extra term per doc).
- For `is_bold_definition`-aware searches see §3.4.

Result: tantivy returns the *correct* total via the `Count` collector
applied to the same `BooleanQuery`, and `TopDocs::with_limit(page_len)`
pulls exactly the page's documents. Snippet generation runs on
`page_len` rows, never on hundreds.

### 3.3 SQL push-down

For every FTS5 helper, in the same `WHERE`:

```sql
WHERE f MATCH ?
  [AND <table>.uid LIKE ?]    -- prefix, e.g. 'bodhi%'
  [AND <table>.uid LIKE ?]    -- suffix, e.g. '%mnt'
ORDER BY <table>.id
LIMIT ? OFFSET ?
```

And a parallel `SELECT COUNT(*)` over the same predicate for
`db_query_hits_count`. Both prefix and suffix get bound only when set;
otherwise the clause is omitted so the query plan stays as-is for
unfiltered queries.

Affected helpers:
- `suttas_contains_match_fts5` — already does prefix; add suffix.
- `dict_words_contains_match_fts5` — add both.
- `book_spine_items_contains_match_fts5` — add both.
- `query_bold_definitions_bold_fts5` — already does prefix; add suffix.
- `query_bold_definitions_commentary_fts5` — already does prefix; add
  suffix.
- `dpd_lookup`, `lemma_1_dpd_headword_match_fts5` — add both around the
  SQL paths; the bold-definition append they used to do is still there
  via the same helpers above (which now push down everything).
- Title-match SQL paths — add both, then `LIMIT page_len OFFSET …`.

### 3.4 Bold-definitions in Fulltext Match — combined dict index

The dict + bold-defs index consolidation is **already done** at the
schema/data layer (one schema, one bootstrap pass). What remains is
removing the now-dead two-index plumbing in `Searcher` and
`indexer.rs`:

- `fulltext_dict` becomes a single tantivy query against the unified
  dict index. BM25 is internally consistent — bold rows that
  genuinely outrank dict rows appear on page 0 with no merge step.
- A Dictionary-mode search with `include_comm_bold_definitions = false`
  adds `Occur::MustNot { is_bold_definition = true }` to filter bold
  rows out at the query stage. Default `true` means no extra
  constraint.
- `dict_doc_to_result` reads `is_bold_definition` per-doc and
  dispatches to `bold_definition_doc_to_result` for bold rows.
- `IndexType::BoldDefinitions`, `Searcher::bold_index*` fields, the
  `BoldDefinitions` arm of `open_single_index`, the
  `bold_definitions_index_dir` path, and `build_bold_definitions_index`
  are deleted.

### 3.5 Highlighting

Unchanged from analysis §7.4: highlight only the returned page; for
non-DPD rows pass `normalize_plain_text(&self.query_text)` to
`highlight_query_in_content`. Bold-definition rows naturally get correct
diacritic highlighting because their snippet text is normalized too.

### 3.6 What disappears

- `apply_uid_filters` (or it stays as a `debug_assert!`-only check).
- `cached_full_fetch` and the SearchQueryTask state it keeps.
- `SAFETY_LIMIT_TANTIVY = 300` constant; `TopDocs::with_limit(page_len)`
  is the new bound.
- `db_query_hits_count = filtered.len()` overrides; the count comes from
  the storage layer in one place per handler.
- The "uid is tokenized via simple_fold, so this push-down can
  over-match" comments on `add_dict_filters` / `add_library_filters`.
- The `SearchFilters { ..hard-coded bold defaults }` block in the bold
  fulltext path (it's gone with the combined index).

## 4. Stage-by-stage implementation plan

Each stage compiles + tests independently. Order matters: storage-layer
push-down lands before pipeline simplification, so we never have a
window where filters silently drop.

### Stage 1 — Tantivy push-down for uid prefix + suffix

- Schema (`backend/src/search/schema.rs`): add `uid_rev` (raw) to
  `build_sutta_schema` and `build_dict_schema`; add `spine_item_uid_rev`
  (raw) to `build_library_schema`.
- Indexer (`backend/src/search/indexer.rs`): when emitting each doc,
  write the lowercased uid into the original field and the
  character-reversed lowercased uid into the `*_rev` field.
- Re-bootstrap once after the indexer change.
- In `backend/src/search/searcher.rs`, replace `add_dict_filters` and
  `add_library_filters` bodies (and add equivalent push-down inside
  `add_sutta_filters`) with the `add_uid_filters` helper from §3.2,
  passing the right `(uid_field, uid_rev_field)` pair per index.
- Drop the stale "simple_fold over-match" comments.
- Add a unit test in `searcher.rs` (or a thin integration test) that
  runs a query with `uid_prefix = "an"` and `uid_suffix = "1.1"`
  against a small in-memory index and asserts the returned total is
  exactly the matching docs, not the full corpus.

### Stage 2 — SQL push-down for uid prefix + suffix

- `backend/src/query_task.rs` FTS5 helpers (Suttas/Dict/Library
  contains, bold helpers, DPD lookup/headword, title-match): extend
  each `WHERE` with conditional prefix/suffix `LIKE` clauses; add
  `SELECT COUNT(*)` siblings using the same predicate; bind both into
  a `(rows, total)` return.
- Helpers stop touching `self.db_query_hits_count` directly — they
  return `total` to the caller.
- Run `backend/tests/test_uid_suffix_and_bold_ascii.rs` and
  `backend/tests/test_search_filter_pagination.rs` — both must still
  pass; the pagination test will start exercising real per-page SQL
  rather than cache slicing.

### Stage 3 — Pipeline simplification

- Convert `results_page` to the §3.1 dispatch shape: each mode handler
  takes `page_num` and returns `(Vec<SearchResult>, usize)`.
- Delete `apply_uid_filters`, `cached_full_fetch`, `SAFETY_LIMIT_TANTIVY`,
  `merge_by_score_desc` (kept only if the fallback in §3.4 is taken),
  and the `_all` wrappers added in the previous Stage B.
- `db_query_hits_count` is written exactly once, at the end of
  `results_page`, from the handler's returned total.
- Highlight runs over the returned page only.
- `SAFETY_LIMIT_SQL` survives as a defensive ceiling on the COUNT and
  on broad unfiltered FTS5 (raise to e.g. 50k); document it as
  defense-in-depth.

### Stage 4 — Remove dead two-index plumbing

The data is already in one index. This stage just deletes the code that
still pretends there are two:

- Indexer: have dict_words rows and bold_definitions rows write into
  the unified `dict_words_index_dir`. Delete
  `build_bold_definitions_index` and the
  `bold_definitions_index_dir` path entry.
- Bootstrap (`cli/src/bootstrap`): drop the separate bold-definitions
  index step.
- Searcher: remove `IndexType::BoldDefinitions`, the
  `bold_index`/`bold_indexes` fields, the `BoldDefinitions` arm of
  `open_single_index`, and `query_bold_definitions_fulltext_all`.
- `fulltext_dict` becomes a single tantivy call against `dict_indexes`
  with `is_bold_definition` controlled by `filters.include_comm_bold_definitions`:
  - `true` (default): no constraint, both kinds participate.
  - `false`: `Occur::MustNot { is_bold_definition = true }`.
- `dict_doc_to_result` peeks at `is_bold_definition` and dispatches to
  `bold_definition_doc_to_result` per-doc.
- Update `tasks/prd-dpd-bold-definitions-search.md` §4.1 / §4.2 / §7
  (also covers analysis §2.9, §2.10, §2.11 — already pending in
  `tasks-analysis-dpd-bold-definitions-search.md` task 7.0).

### Stage 5 — Snippet-generator reuse + cleanup

The recent `search_single_index` change (commit b18b4c0) already builds
`SnippetGenerator` once per call. With the page-only fetch from Stage 3,
its cost is bounded to `page_len` snippets, so no further work is needed
here — but verify with the timing tests in
`test_search_filter_pagination.rs`. Drop the
`FILTERED_PAGINATION_BUDGET` slack (was sized for the cached full-fetch
worst case) once new numbers are in.

### Stage 6 — Tests

- Extend `test_search_filter_pagination.rs` to assert that for any
  filtered query, *the underlying SQL/tantivy execution* returns the
  page-sized chunk and the COUNT/Count-collector total — i.e. spy that
  the new push-down path is exercised, not the old full-fetch fallback
  (which won't exist anymore).
- Add `test_pagination_invariants.rs` (analysis §8.1): every page
  except the last has exactly `page_len`; pages are contiguous.
- Add `test_bold_definitions_highlighting.rs` (analysis §8.4):
  diacritic query yields highlight spans on bold-definition rows.
- Keep the timing assertions but tighten budgets after Stage 3 lands;
  e.g. `PER_PAGE_BUDGET = 2s`, `FILTERED_PAGINATION_BUDGET = 8s` for a
  10-page paginate.

## 5. Risks and open questions

- **`include_comm_bold_definitions` semantics flip.** Pre-refactor the
  flag gated whether to *append* bold rows; in the consolidated index
  it gates whether to *exclude* them via `Occur::MustNot`. Same
  user-facing behaviour, opposite default. UI default is `true`, so
  no visible change.
- **DPD lookup / Headword Match.** These remain SQL-driven and aren't
  affected by Stage 4. They benefit from Stage 2 (suffix push-down) and
  the elimination of the bold-append + Rust-paginate dance.
- **`uid_rev` re-bootstrap.** Adding the reversed-uid field to the
  schema requires one more re-bootstrap after Stage 1's indexer change.
  Land the schema + indexer change before flipping the searcher to use
  `uid_rev`, so old indexes don't get queried with a missing field.

## 6. Out-of-scope (deliberate)

- Deferred snippet rendering across pagination calls. With page-only
  fetches in Stage 3 it stops being a bottleneck; revisit only if the
  timing tests show otherwise.
- Pre-rendered HTML snippets stored in the index. A bigger architectural
  move; not justified by current numbers.
- Cross-mode result deduplication. Today's `(uid, table_name)` identity
  is good enough; deduplication is a separate concern.
