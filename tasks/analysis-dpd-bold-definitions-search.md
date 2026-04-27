# Analysis: DPD Bold Definitions in Dictionary Search — branch review

> **§7 superseded (2026-04).** The §7 target design — Rust-side
> `apply_uid_filters` post-pass over a SAFETY-capped full fetch, with
> `cached_full_fetch` to amortise pagination — has been **replaced** by
> push-down filtering at the storage layer plus native pagination. See
> [`tasks/query-pipeline-filtering-strategy-refactor.md`](./query-pipeline-filtering-strategy-refactor.md)
> for the current pipeline (uniform `raw` uid fields, `uid_rev` for
> suffix push-down, unified dict tantivy index that absorbs
> `bold_definitions`, `Occur::MustNot { is_bold_definition }` to gate
> bold rows, no `cached_full_fetch`, no `SAFETY_LIMIT_TANTIVY`). The
> earlier §1–§6 findings (DB index coverage, full-fetch costs,
> simple_fold over-match) remain accurate as historical context for *why*
> the refactor was needed.

Source branch: `bold-definitions-search` (commits `61e29b3`, `b656e94`, `97e9c8b`).
Reviewer scope: db indexes for uid prefix/suffix filtering across search areas, and
a close read of `backend/src/query_task.rs` for conceptual issues in the refactor
of `results_page()` and in the four Dictionary-mode bold-definition integrations.

## 1. DB indexes for uid prefix / suffix filtering

| Table              | DB                   | `uid` btree                                                      | Prefix `LIKE 'x%'` | Suffix `LIKE '%x'`     |
|--------------------|----------------------|------------------------------------------------------------------|--------------------|------------------------|
| `suttas`           | appdata.sqlite3      | `UNIQUE(uid)` (implicit) + `idx_suttas_language_uid`             | ✅ index-usable    | ❌ (SQLite limitation) |
| `dict_words`       | dictionaries.sqlite3 | `UNIQUE(uid)` (implicit)                                         | ✅ index-usable    | ❌                     |
| `book_spine_items` | appdata.sqlite3      | `UNIQUE(spine_item_uid)` + `idx_book_spine_items_spine_item_uid` | ✅ index-usable    | ❌                     |
| `bold_definitions` | dpd.sqlite3          | `UNIQUE idx_bold_definitions_uid` (new)                          | ✅ index-usable    | ❌                     |
| `dpd_roots`        | dpd.sqlite3          | `UNIQUE(uid)`                                                    | ✅                 | ❌                     |
| `dpd_headwords`    | dpd.sqlite3          | no `uid` column (uid is synthesized)                             | n/a                | n/a                    |

**Conclusion on indexes:** every uid-bearing table has the btree coverage needed
for efficient prefix filtering. Suffix (`LIKE '%x'`) cannot use a btree in
SQLite by design; this is irrelevant for bold-definitions because `bd.uid
LIKE '%x'` runs *after* an FTS5 MATCH has already reduced the row set. For
`suttas`/`dict_words`/`book_spine_items`, suffix filtering happens in Rust
over a result page (see §2.6 below), so the missing btree doesn't hurt.

**Minor note:** `dict_words.uid` and `book_spine_items.spine_item_uid` have
only the implicit unique index — no composite index that combines `uid` with
`language` or `dict_label`. That's fine given current access patterns; no
action needed unless a future mode does `WHERE dict_label = ? AND uid LIKE
'x%'` in tight loops.

## 2. `query_task.rs` — conceptual issues

### 2.1 Off-spec pagination in three of the four Dictionary modes

The PRD (§4.3.12) says bold-definition results should be **appended** to the
regular dictionary results. "Appended" implies one logical stream: dict
results first, then bold results, paginated as a unified list. Three sites
don't implement it that way — they paginate the dict slice and the bold
slice separately, so each page returns up to `2 × page_len` results glued
together:

- `dict_words_contains_match_fts5` (lines 1245–1259): dict rows are SQL-LIMITed
  to `page_len` per page, then `query_bold_definitions_commentary_fts5` is
  called (no SQL LIMIT) and sliced `[start..start+page_len]` using the same
  `page_num`. The page returned therefore has `min(page_len, dict_page_size)
  + min(page_len, bold_page_size)` items.
- `lemma_1_dpd_headword_match_fts5` (lines 1936–1944): same pattern.
- `dpd_lookup` (lines 1542–1568) — **correct**: merges into `all_results`
  first, then paginates once. This is the model the others should follow.

**Why this matters:**

1. The UI's "results N of total" counter becomes incoherent. The total is
   `dict_total + bold_total` but pages actually deliver page_len from each
   source, not from the unified total.
2. Later pages can be double-empty or double-full inconsistently (if dict
   has 100 hits and bold has 3, pages 1–9 get 10 dict + 0 bold; page 0
   gets 10 dict + 3 bold = 13 rows, breaking the page_len invariant).
3. Consumers that rely on `.len() == page_len` as "more pages exist" will
   mispaginate.

**Suggested fix:** mirror `dpd_lookup`'s pattern everywhere: pull both
result sets into one `Vec<SearchResult>`, then apply a single
`offset..offset+page_len` slice. For `dict_words_contains_match_fts5` this
means dropping the SQL-side LIMIT/OFFSET and paginating in Rust like
`dpd_lookup` does — which is already the behavior the `results_page`
post-filter path depends on anyway.

### 2.2 Fulltext Match: per-index pagination merged — correctness hazard

`fulltext_dict_words` (lines 2227–2293):

```
let (_, dict_results) = searcher.search_dict_words_with_count(..., page_num);
let (_, bold_scored)  = query_bold_definitions_fulltext(..., page_num);
merged.sort_by_score_desc();
take(page_len);
```

Each sub-searcher returns page_num's slice from **its own** index. The
merge-then-sort-then-take(page_len) is only locally correct: a
bold-definition whose score would place it on page 0 of the *merged* list
but which sits at rank 15 in the bold index will never appear on page 0 of
the merged results, because only bold[0..10] was fetched.

The PRD §4.3.12 allows "ranking bias" because BM25 scores across indexes
aren't comparable. That note forgives *ordering*; it doesn't forgive
*dropping* high-scoring items. For users with bold-heavy queries, relevant
bold results will silently disappear from early pages.

**Suggested fix:** over-fetch. Pull the first `(page_num + 1) * page_len`
hits from each index (cap at some ceiling), merge, sort by score, then
slice `[page_num*page_len..(page_num+1)*page_len]`. This is cheap since
each tantivy `search` already sorts by score internally. Also report
`dict_total + bold_total` as before.

### 2.3 `results_page` post-filter path — capped at 10,000 pre-filter rows

`results_page()` now branches on `needs_post_filter()`:

- If `uid_suffix` is set, or `uid_prefix` is set for non-Suttas areas,
  force `self.page_len = 10_000`, call `run_mode_for_area(0)`, filter in
  Rust, paginate the filtered list.
- Else use the mode's native pagination.

Observations and issues:

1. **Magic constant, no surfacing.** If the filtered result set would
   exceed 10k *before* the suffix filter, the user sees a truncated list
   without notice. For Dictionary Fulltext Match, this is actually more
   likely than you'd think because the Pāli tokenizer can generate broad
   matches. Consider: (a) raising the cap, or (b) emitting a warn log
   when hit, or (c) applying the suffix filter inside each mode's SQL
   (see §2.5).
2. **Mutates `self.page_len` temporarily.** If `run_mode_for_area` throws
   between the mutation and the restore, `self.page_len` stays at 10_000
   for the lifetime of this `SearchQueryTask`. Current code catches via
   `inner_result?` *after* restoring — the order is safe, but it's fragile
   and a future refactor could break it. Prefer scoped state via a guard
   struct (`Drop`) or pass the fetch-size as a parameter to
   `run_mode_for_area`.
3. **Interaction with bold-definition "append" branches.** When
   `dict_words_contains_match_fts5` is called with `page_len = 10_000` and
   `page_num = 0`, it fetches 10_000 dict rows *and* up to another 10_000
   bold rows, so `results_page` can get up to 20_000 items to filter. Fine
   functionally but multiplies the cost of a uid-filtered query by ~2.
4. **Double-filtering of uid_prefix for bold helpers.** The three
   `query_bold_definitions_*_fts5` helpers already push `uid_prefix`
   *down* to SQL (`bd.uid LIKE ?`). When the post-filter path also filters
   by prefix in Rust, rows pass both checks — correct, but redundant.
5. **`needs_post_filter` decision is search-area-coarse.** It says
   "Suttas applies prefix in SQL, others don't". But `dict_words_contains_match_fts5`
   in the SQL path does *not* push `uid_prefix` down (only the
   bold-definition helpers do). So for Dictionary, even regular dict rows
   need the Rust post-filter — that's what the code does, correctly.
   Library (`book_spine_items_contains_match_fts5`) also doesn't push
   prefix down. The mechanism works, it's just opaque that the gating is
   by area rather than by what each branch actually does.

### 2.4 Highlighting mismatch for bold-definition snippets

`results_page` runs `highlight_query_in_content(self.query_text,
result.snippet)` on every non-DPD result (lines 2163–2173). Bold-definition
results have `table_name = "bold_definitions"` (not in the DPD skip list),
so they *do* get re-highlighted. But:

- The snippet is built from `commentary_plain`, which is **normalized** Pāli.
- `self.query_text` is the user's **unnormalized** input.

For a query like `bhikkhū` (with the long ū) the stored plain text has
been normalized to `bhikkhu`; the highlighter will fail to mark any span.
The Contains Match path normalizes the query for searching
(`normalize_plain_text(&self.query_text)`) but that normalized form is
dropped before highlighting.

**Suggested fix:** either highlight using the same normalized query used
for searching bold-definition rows, or add `bold_definitions` to the skip
list so the raw snippet (still plain text) is shown without highlighting —
then optionally produce a pre-highlighted snippet inside the helper.

### 2.5 Regular dict_words and book_spine_items could push uid_prefix to SQL

`suttas_*_fts5` branches push `uid_prefix` down to SQL (`query_task.rs:743,
839, 1648`). `dict_words_contains_match_fts5` and
`book_spine_items_contains_match_fts5` do not — they rely on the 10k-row
post-filter path. For narrowly-scoped prefix queries (e.g. `uid_prefix =
"bodhi"`) this means fetching up to 10k rows that will mostly be discarded
in Rust. Pushing the prefix down would:

- eliminate the 10k cap as a practical concern for prefix-only queries;
- avoid the temporary-page_len mutation in those cases
  (`needs_post_filter()` can remain true only when suffix is set);
- align Dictionary/Library with Suttas for no extra cost.

This is an optimization, not a correctness fix — current behavior is fine
as long as the filtered universe fits in 10k.

### 2.6 Bold-definition filters silently ignore the regular-dict filters

`query_bold_definitions_fulltext` hard-codes:

```
SearchFilters {
    lang: None, source_uid: None, nikaya_prefix: None, uid_prefix: None,
    include_cst_mula: true, include_cst_commentary: true, include_ms_mula: true, …
}
```

Consequence: if the user sets a dict `source` filter (e.g. "exclude PTS"),
or language, or nikaya, the regular dict-fulltext path honors it but the
bold-definition fulltext path does not — so excluded-source bold hits still
show up. The scripted FTS5 helpers (bold and commentary) propagate
`uid_prefix`/`uid_suffix` but ignore `lang`, `source`, `include_ms_mula`,
etc.

For most of these the mismatch is semantically defensible (bold
definitions are always Pāli, don't have `dict_label`, aren't part of MS
Mūla). **But the `uid_prefix`/`uid_suffix` are applied inconsistently:
the tantivy bold path passes `uid_prefix: None` and so falls back to the
Rust post-filter, whereas the FTS5 bold path pushes it down to SQL.** Both
end up correct because the Rust post-filter catches the tantivy case, but
it does mean the tantivy bold search can't short-circuit on a narrow
prefix — it always fetches the top-`page_len` overall.

### 2.7 Per-row id lookup pattern in bold helpers — acceptable, but…

Both `query_bold_definitions_bold_fts5` and
`query_bold_definitions_commentary_fts5` do the FTS5 MATCH → ids, then a
second Diesel query to load `BoldDefinition` rows by `id.eq_any(&ids)`.
Two round-trips per query. Either:

- Change the SQL to `SELECT bd.* FROM <fts> f JOIN bold_definitions bd ON
  bd.id = f.bold_definitions_id WHERE ...` with a `#[derive(QueryableByName)]`
  on a mirror of `BoldDefinition`, collapsing to one round-trip; or
- Keep two queries but push the limit/offset into the inner FTS5 query so
  we never materialize more ids than we need.

Minor perf, not a correctness issue.

### 2.8 Redundant `ORDER BY f.bold_definitions_id` in FTS5 queries

Both bold helpers do `ORDER BY f.bold_definitions_id` inside the FTS5 SELECT
and then feed the ids into a second query also ordered by id. The inner
ordering is discarded by `eq_any`. Drop the inner ORDER BY unless the
caller depends on deterministic order before the secondary load — in which
case you're paying for the sort twice. Either way, one of the two is dead.

### 2.9 `bold_ascii` silently added outside the PRD

The branch introduces a `bold_ascii` column + FTS5 coverage
(`scripts/dpd-bold-definitions-fts5-indexes.sql:68-91`,
`backend/src/db/dpd.rs:620-680`,
`query_task.rs:1404` uses `bold LIKE ? OR bold_ascii LIKE ?`). This is a
reasonable improvement (ASCII queries match diacritic entries, mirrors
`word_ascii`), but it is not in `tasks/prd-dpd-bold-definitions-search.md`.
Either:

- update the PRD (§4.1 / §4.2) to include `bold_ascii` so the intent is
  recorded; or
- drop `bold_ascii` if you didn't actually want it.

Also the idempotency check is now `uid OR bold_ascii empty`, which
correctly forces a re-population when someone upgrades from an older
migrated DB that has `uid` but no `bold_ascii`.

### 2.10 PRD drift: at-startup SQL index execution

PRD §7 and task 2.2 say the new `dpd-bold-definitions-fts5-indexes.sql`
should also run in the at-startup path (`backend/src/db/dpd.rs` "~line
786"). In the current code the script is only executed inside
`create_dpd_indexes` (line 975), which is itself only called from
`import_migrate_dpd` — i.e. **bootstrap only**. The at-startup path does
not run it at all (there is no at-startup index-creation path in
`dpd.rs`). Decide which is correct:

- If the bootstrap-only path is intentional (the shipped `dpd.sqlite3`
  already has the indexes), update the PRD + task 2.2 to reflect it.
- If at-startup re-running is desired for users upgrading in place, wire
  it up where the app first opens `dpd.sqlite3`.

### 2.11 `populate_bold_definitions_derived_columns` lives in `backend`, not `cli`

PRD task 1.1 is emphatic: "This belongs in the CLI bootstrap path, not in
the at-startup `backend/src/db/dpd.rs` path." The function is defined at
`backend/src/db/dpd.rs:602` and called from `import_migrate_dpd` at the
same file (line 568). `cli/src/bootstrap/dpd.rs` then calls
`import_migrate_dpd`, so functionally this is still bootstrap-triggered.
But the *location* is in the `backend` crate — opposite of the PRD
directive. Either:

- move the function to `cli/src/bootstrap/dpd.rs` as the PRD requires, or
- amend the PRD to acknowledge it lives in `backend` because
  `import_migrate_dpd` already lives there and the migration is a
  backend-level concern.

No runtime risk — just PRD/code drift.

### 2.12 Hard-coded relative script path

`create_dpd_indexes` loads `dpd-bold-definitions-fts5-indexes.sql` as a
hardcoded relative path `../scripts/dpd-bold-definitions-fts5-indexes.sql`
(line 973). This works only when the process cwd is the `cli/` folder at
invocation — same issue the existing `dpd-fts5-indexes.sql` path has. Not
a regression, but a fragile pattern worth noting. The btree script is
loaded via `include_str!` (line 919) which is cwd-independent; the FTS5
ones can't be because `run_fts5_indexes_sql_script` shells out to
`sqlite3`.

## 3. Summary of improvements to prioritize

| # | Change                                                                                                                                    | Why                                                                  |
|---|-------------------------------------------------------------------------------------------------------------------------------------------|----------------------------------------------------------------------|
| 1 | Merge bold-definition results into one list *before* pagination in `dict_words_contains_match_fts5` and `lemma_1_dpd_headword_match_fts5` | §2.1 — current code breaks the page_len contract                     |
| 2 | Over-fetch both indexes in `fulltext_dict_words`, then merge + slice                                                                      | §2.2 — current code can drop high-scoring bold hits from early pages |
| 3 | Decide on highlighter behavior for bold-definition snippets (normalize query, or skip highlight)                                          | §2.4 — accented queries currently produce unhighlighted snippets     |
| 4 | Push `uid_prefix` down to SQL in Dictionary/Library contains paths                                                                        | §2.5 — removes 10k cap risk for prefix-only queries                  |
| 5 | Scope the `self.page_len = 10_000` mutation via a guard or parameter                                                                      | §2.3.2 — robustness                                                  |
| 6 | Align PRD with as-built `bold_ascii` column and at-startup/file-location choices                                                          | §2.9, §2.10, §2.11                                                   |
| 7 | Collapse two-query-per-helper pattern in bold helpers to a single JOIN, drop redundant inner `ORDER BY`                                   | §2.7, §2.8 — minor perf                                              |

## 4. Non-issues verified

- Every `SearchResult` path populates a non-empty `uid` field (suttas,
  dict_words, dpd_headwords, dpd_roots, book_spine_items, bold_definitions
  from both FTS5 and tantivy), so the UID suffix post-filter is
  well-defined everywhere (PRD §4.5.16).
- The new `idx_bold_definitions_uid` UNIQUE index is populated after
  migration as required; the btree script is run from `create_dpd_indexes`
  *after* `populate_bold_definitions_derived_columns`.
- `apply_uid_filters` correctly lowercases before `starts_with` /
  `ends_with`, matching the stored (lowercased) uid format.
- The 10k cap is the only `page_len` override in the file — no other
  hidden size limits.

## 5. Pipeline architecture — pre-branch vs post-branch

Now re-examining the refactor from first principles, by comparing the
pre-branch flow (`main`) with the post-branch flow.

### 5.1 Pre-branch flow

```
results_page(page_num)
  └─ match mode/area → mode_handler(page_num)
         (each handler SQL-paginates with LIMIT page_len OFFSET page_num*page_len
          or Rust-paginates over all_results; returns exactly one page)
  └─ highlight(each result)
  └─ return
```

Each mode handler owned its own pagination and its own `db_query_hits_count`
assignment. There was no cross-cutting filter stage.

### 5.2 Post-branch flow

```
results_page(page_num)
  ├─ if needs_post_filter:
  │     save orig page_len
  │     self.page_len = 10_000
  │     run_mode_for_area(0)
  │     restore page_len
  │     apply_uid_filters(results)
  │     Rust-paginate filtered[page_num*page_len .. ...]
  │     overwrite db_query_hits_count = filtered.len()
  └─ else:
        run_mode_for_area(page_num)  (mode handler's own pagination)
  └─ highlight(each result)
  └─ return
```

Plus: bold-definition append *inside* mode handlers (after the SQL page has
already been fetched in two of the three modes that now merge bold hits).

### 5.3 What the refactor got right

1. **Introducing a pipeline seam** between mode dispatch and pagination is
   the correct move. Without it, there's no place to insert a post-filter
   (or any cross-cutting concern: uid filters today, access-control or
   deduplication tomorrow).
2. **Choosing an abstraction boundary** at `run_mode_for_area()` matches
   the existing mode/area matrix — not a new concept the reader has to
   learn.
3. **Post-filter only runs when needed.** The non-filtered path still uses
   the fast native SQL pagination, preserving existing performance.

### 5.4 What the refactor got wrong (architecturally, beyond the local bugs in §2)

1. **Pagination is stuck inside mode handlers.** The seam was added
   *around* those handlers but not *into* them. To get "all results, not
   just page 0", the code has to lie by mutating `self.page_len = 10_000`
   and passing `page_num = 0`. This is the shape of a workaround, not a
   design. A cleaner contract:

   ```
   fn run_mode_for_area(&mut self, paging: Option<(usize, usize)>)
         -> Result<Vec<SearchResult>>
   ```

   where `None` means "return everything (or the implementation's
   conservative cap)". Then `results_page` calls it with `None` when a
   post-filter is needed and with `Some((page_len, page_num))` otherwise.
   No magic constants, no hidden state mutation.

2. **Bold-definition appending is at the wrong layer.** It's currently
   inside each mode handler, *after* that handler has already paginated
   dict results. This is what causes the §2.1 pagination bug in two of
   the three modes — the third (`dpd_lookup`) coincidentally works
   because it paginates in Rust over the merged list. The correct layer
   for bold-definition appending is the *same* layer as uid filtering:
   inside `results_page`, between `run_mode_for_area` and pagination.

   Proposed structure:

   ```
   results_page:
     let regular = run_mode_for_area(None)?;       // unpaginated
     let bold    = run_bold_for_mode(None)?;       // unpaginated, gated on include_comm_bold_definitions
     let merged  = merge_by_mode(regular, bold);   // sort by score for Fulltext; concat else
     let filtered = apply_uid_filters(merged);
     paginate(filtered, page_num, page_len)
   ```

   This would fix §2.1 and §2.2 at once, eliminate the double-pagination
   footgun, and simplify the three mode handlers (they stop caring about
   bold definitions entirely).

3. **`db_query_hits_count` ownership is split.** Mode handlers set it;
   the bold-append blocks do `+=`; `results_page` overwrites it in the
   post-filter branch. Three different owners for one counter. Moving
   merge+filter+pagination into `results_page` consolidates this —
   `db_query_hits_count = merged.len()` (or `filtered.len()` when a
   filter ran) in exactly one place.

4. **Highlighting applied to already-merged list.** Because bold
   definitions are merged in *before* the highlighter, the
   normalized-snippet-vs-raw-query mismatch in §2.4 leaks into the
   user-visible output. If bold-definition merging moves into
   `results_page`, the highlighter can be taught to skip or specially
   handle `table_name == "bold_definitions"` at the same site — cleaner
   than per-helper handling.

### 5.5 Cost of the bigger refactor

- Mode handlers lose their SQL `LIMIT` as a natural memory cap. For
  `dict_words_contains_match_fts5` this could mean fetching millions of
  rows on broad queries. Mitigate with a hard SQL cap (`LIMIT 50_000` or
  similar) applied uniformly inside the helper when paging is `None`.
- `fulltext_suttas`/`fulltext_library` would need the same treatment to
  stay symmetrical, even though they don't currently participate in bold
  definitions — otherwise the codebase develops two different handler
  contracts.
- Needs a careful audit of what `run_mode_for_area` returning huge
  `Vec<SearchResult>` costs. For Fulltext Match this is already the
  effective behavior when the post-filter fires.

### 5.6 If the bigger refactor is out of scope

The minimum set of tactical fixes that preserve current architecture
but fix the identified bugs (§2.1, §2.2, §2.4):

- In `dict_words_contains_match_fts5` and
  `lemma_1_dpd_headword_match_fts5`: remove the SQL LIMIT/OFFSET, collect
  all dict rows, extend with bold, paginate the merged list once in Rust
  (mirror `dpd_lookup`). Keep `db_query_hits_count = all.len()`.
- In `fulltext_dict_words`: over-fetch `(page_num+1)*page_len` from each
  index, merge, sort, slice. Or just fetch a larger k and paginate that.
- In `results_page`'s highlighter: skip highlighting when
  `table_name == "bold_definitions"`, or normalize `self.query_text`
  before passing it in for those rows.

## 6. Revised priority list (supersedes §3)

In descending priority:

| # | Change | Category |
|---|---|---|
| 1 | Fix pagination in `dict_words_contains_match_fts5` + `lemma_1_dpd_headword_match_fts5` (merge-then-slice, like `dpd_lookup`) | correctness |
| 2 | Over-fetch + merge in `fulltext_dict_words` so top-scoring bold hits aren't dropped from early pages | correctness |
| 3 | Fix bold-definition snippet highlighting (skip or normalize the query) | correctness |
| 4 | Replace `self.page_len = 10_000` with an explicit paging parameter on `run_mode_for_area` (or a `Drop`-guarded scope) | robustness |
| 5 | (Optional bigger refactor) Move bold-definition merge + uid filter + pagination into `results_page`; mode handlers return unpaginated results | architecture |
| 6 | Push `uid_prefix` down to SQL in Dictionary/Library contains paths | perf |
| 7 | Collapse two-query bold helpers to single JOIN; drop redundant `ORDER BY` | perf |
| 8 | Sync PRD with as-built: `bold_ascii`, bootstrap-only index path, `populate_…` file location | docs |

## 7. Refactored pipeline — target design

Decisions locked in from the review (resolving §2.1–§2.12):

- **2.1** Unified pagination: merge *all* sources, then paginate once.
- **2.2** Over-fetch enough to preserve top-scoring items from both indexes.
- **2.3** Eliminate the 10,000 `self.page_len` mutation entirely.
- **2.4** Highlight with the normalized query (same as the one used for
  searching bold rows).
- **2.5** No temporary `page_len` mutation at all; drop the 10k cap.
- **2.6** `uid_prefix` and `uid_suffix` apply uniformly across areas and
  across regular + bold rows. Other filters (lang, source, MS Mūla, CST
  mūla/commentary) apply only where meaningful; they are out of scope for
  bold definitions.
- **2.7** Add `#[derive(QueryableByName)]` to `BoldDefinition` so a single
  `SELECT bd.* FROM bold_definitions_fts f JOIN bold_definitions bd …`
  replaces the current id-then-load pair. Avoids introducing a second
  mirror struct.
- **2.8** After collapsing to a single JOIN: keep one `ORDER BY bd.id` on
  the outer query, drop the inner FTS5 `ORDER BY`.
- **2.9** Amend the PRD to include `bold_ascii` in §4.1 / §4.2.
- **2.10** `dpd-bold-definitions-fts5-indexes.sql` and `create_dpd_indexes`
  are intentionally bootstrap-only. Update the PRD + task list to reflect
  this (remove the "at-startup also" wording).
- **2.11** `populate_bold_definitions_derived_columns` stays in `backend/`
  (siblings with `import_migrate_dpd`); amend the PRD wording.
- **2.12** Relative script paths are fine — bootstrap is run manually
  from `cli/`.

### 7.1 Target pipeline

```
pub fn results_page(page_num) -> Vec<SearchResult> {
    // --- Stage 1: fetch (unpaginated, each side returns everything it has,
    //     with a generous SQL safety cap inside each helper). ---
    let regular = self.fetch_regular_unpaginated()?;       // Vec<SearchResult>
    let bold    = if self.should_fetch_bold() {
                      self.fetch_bold_unpaginated()?       // Vec<SearchResult>
                  } else { Vec::new() };

    // --- Stage 2: merge (mode-specific). ---
    let merged = match self.search_mode {
        SearchMode::FulltextMatch => merge_by_score_desc(regular, bold),
        _                          => { let mut v = regular; v.extend(bold); v }
    };

    // --- Stage 3: unified uid filter (always, everywhere). ---
    let filtered = self.apply_uid_filters(merged);
    self.db_query_hits_count = filtered.len() as i64;

    // --- Stage 4: paginate once. ---
    let start = page_num * self.page_len;
    let end   = std::cmp::min(start + self.page_len, filtered.len());
    let page  = if start >= filtered.len() { Vec::new() }
                else { filtered[start..end].to_vec() };

    // --- Stage 5: highlight only the returned page. ---
    Ok(page.into_iter().map(|r| self.highlight_row(r)).collect())
}
```

Key invariants:

- Mode handlers no longer take `page_num` or call pagination. Their only
  job is to produce an already-filtered-by-mode vector of results, bounded
  by a `SAFETY_LIMIT` applied inside their SQL/tantivy call. The tantivy
  call sets `TopDocs::with_limit(SAFETY_LIMIT)`; the FTS5 calls add
  `LIMIT SAFETY_LIMIT`.
- `self.page_len` is read-only throughout the pipeline.
- `needs_post_filter`, the 10k mutation, and the save/restore dance are
  deleted.
- Highlighting runs on `page_len` rows, never on the full 10k+ intermediate.
- Bold-definition integration lives at one seam (`fetch_bold_unpaginated`),
  not scattered inside three mode handlers.

### 7.2 Fetch helpers

```
fn fetch_regular_unpaginated(&mut self) -> Result<Vec<SearchResult>> {
    match (self.search_mode, self.search_area) {
        (FulltextMatch, Suttas)     => self.fulltext_suttas_all(),
        (FulltextMatch, Dictionary) => self.fulltext_dict_words_all(),
        (FulltextMatch, Library)    => self.fulltext_library_all(),
        (ContainsMatch, Suttas)     => self.suttas_contains_all(),
        (ContainsMatch, Dictionary) => self.dict_words_contains_all(),
        (ContainsMatch, Library)    => self.book_spine_items_contains_all(),
        (DpdLookup,     Dictionary) => self.dpd_lookup_all(),
        (HeadwordMatch, Dictionary) => self.dpd_headword_match_all(),
        (TitleMatch,    Suttas)     => self.suttas_title_match_all(),
        (TitleMatch,    Library)    => self.library_title_match_all(),
        (UidMatch,      _)          => self.uid_match_all(),
        _                           => Ok(Vec::new()),
    }
}

fn should_fetch_bold(&self) -> bool {
    self.search_area == SearchArea::Dictionary
        && self.include_comm_bold_definitions
        && matches!(self.search_mode,
            SearchMode::DpdLookup | SearchMode::HeadwordMatch
          | SearchMode::ContainsMatch | SearchMode::FulltextMatch)
}

fn fetch_bold_unpaginated(&self) -> Result<Vec<SearchResult>> {
    match self.search_mode {
        SearchMode::DpdLookup | SearchMode::HeadwordMatch =>
            self.query_bold_definitions_bold_fts5(&self.query_text),
        SearchMode::ContainsMatch => {
            let q = normalize_plain_text(&self.query_text);
            self.query_bold_definitions_commentary_fts5(&q)
        }
        SearchMode::FulltextMatch => {
            let q = normalize_plain_text(&self.query_text);
            self.query_bold_definitions_fulltext_all(&q)
        }
        _ => Ok(Vec::new()),
    }
}
```

### 7.3 Score-aware merge for Fulltext

Each side is already internally sorted by descending score. A stable merge
suffices:

```
fn merge_by_score_desc(a: Vec<SearchResult>, b: Vec<SearchResult>)
    -> Vec<SearchResult>
{
    let mut out = Vec::with_capacity(a.len() + b.len());
    let (mut i, mut j) = (0, 0);
    while i < a.len() && j < b.len() {
        let sa = a[i].score.unwrap_or(0.0);
        let sb = b[j].score.unwrap_or(0.0);
        if sa >= sb { out.push(a[i].clone()); i += 1; }
        else        { out.push(b[j].clone()); j += 1; }
    }
    out.extend_from_slice(&a[i..]);
    out.extend_from_slice(&b[j..]);
    out
}
```

Inter-index BM25 scores aren't comparable, so the order can be biased —
PRD §4.3.12 accepts that — **but no item is dropped**, which fixes §2.2.

### 7.4 Highlighting with the normalized query

```
fn highlight_row(&self, mut r: SearchResult) -> SearchResult {
    let is_dpd = r.table_name == "dpd_headwords"
              || r.table_name == "dpd_roots"
              || (r.table_name == "dict_words"
                  && r.source_uid.as_ref()
                       .is_some_and(|s| s.to_lowercase().contains("dpd")));
    if !is_dpd {
        let q = normalize_plain_text(&self.query_text);
        r.snippet = self.highlight_query_in_content(&q, &r.snippet);
    }
    r
}
```

Both suttas (`content_plain`) and bold definitions (`commentary_plain`)
store normalized text, so normalizing the query is correct for both. The
DPD dict-word/headword/root branches already produce pre-highlighted
meaning snippets and are skipped as before.

### 7.5 Unified uid filter

```
fn apply_uid_filters(&self, results: Vec<SearchResult>) -> Vec<SearchResult> {
    let prefix = Self::normalized_filter(&self.uid_prefix);
    let suffix = Self::normalized_filter(&self.uid_suffix);
    if prefix.is_none() && suffix.is_none() { return results; }
    results.into_iter().filter(|r| {
        let u = r.uid.to_lowercase();
        prefix.as_ref().is_none_or(|p| u.starts_with(p))
          && suffix.as_ref().is_none_or(|s| u.ends_with(s))
    }).collect()
}
```

Applied uniformly — no more `prefix_handled_by_sql` branch. Any SQL-side
push-down of `uid_prefix` (as suttas paths already do) becomes a pure
optimization: the Rust filter is a no-op against rows that already
satisfy it.

### 7.6 Safety caps

Mode handlers that previously relied on SQL `LIMIT page_len OFFSET …` now
use `LIMIT SAFETY_LIMIT OFFSET 0`. Recommended starting value:
`SAFETY_LIMIT = 100_000`. Rationale:

- Fits comfortably in memory (a few hundred MB at most for rich snippets;
  usually far less).
- Large enough that practical broad queries don't get silently truncated.
- Much higher than the previous 10k cap, which was a *pre-filter* cap;
  here it's the *post-SQL* cap and can safely be higher because we're not
  materializing it into a page-sized working set elsewhere.

Tantivy: `TopDocs::with_limit(SAFETY_LIMIT)` — the collector already does
score-sorted top-k in `O(n log k)`, so this is cheap.

### 7.7 Stage-by-stage implementation plan

Each stage is independently compilable + testable.

**Stage A — Collapse bold helpers to single JOIN (§2.7, §2.8)**
- Add `#[derive(QueryableByName)]` on `BoldDefinition` in
  `backend/src/db/dpd_models.rs`; annotate every field with
  `#[diesel(sql_type = …)]`.
- Rewrite `query_bold_definitions_bold_fts5` and
  `query_bold_definitions_commentary_fts5` to a single `SELECT bd.* FROM
  <fts> f JOIN bold_definitions bd ON bd.id = f.bold_definitions_id WHERE
  … ORDER BY bd.id LIMIT SAFETY_LIMIT`.
- Drop the inner FTS5 `ORDER BY f.bold_definitions_id`.
- Remove the temporary `BdId` QueryableByName structs.
- Unit-test against the real `dpd.sqlite3`.

**Stage B — Convert mode handlers to unpaginated variants**
- For each mode handler currently called by `run_mode_for_area`, add an
  `_all` variant that returns `Vec<SearchResult>` without `page_num`, with
  SQL `LIMIT SAFETY_LIMIT` (no OFFSET).
- FTS5 handlers: strip `page_len`/`page_num` from SQL.
- Tantivy fulltext handlers (`fulltext_suttas`, `fulltext_dict_words`,
  `fulltext_library`): call searcher with `with_limit(SAFETY_LIMIT)`; keep
  per-row `score`.
- Each `_all` handler sets nothing on `self.db_query_hits_count` — that's
  owned by `results_page` now.
- Keep the old paginated handlers *temporarily* so existing call sites
  don't break until Stage D retires them.

**Stage C — Move bold-definition appending into `results_page`**
- Remove the inline bold-append blocks from `dpd_lookup`,
  `dict_words_contains_match_fts5`, `lemma_1_dpd_headword_match_fts5`, and
  `fulltext_dict_words`.
- Implement `fetch_bold_unpaginated` + `should_fetch_bold` as in §7.2.

**Stage D — Replace `results_page` body with the §7.1 pipeline**
- Swap the `needs_post_filter`/`run_mode_for_area` branch for the fetch →
  merge → filter → paginate → highlight flow.
- Delete `needs_post_filter`, the temporary `page_len = 10_000` mutation,
  and the old `run_mode_for_area` (it's now just
  `fetch_regular_unpaginated`).
- Delete the old paginated mode handlers kept in Stage B.

**Stage E — Highlight with normalized query (§2.4)**
- Update the highlight-row closure inside `results_page` to normalize
  `self.query_text` before passing to `highlight_query_in_content` for
  non-DPD rows.

**Stage F — Unified uid filter (§2.6)**
- Replace the current `apply_uid_filters` + `needs_post_filter` pair with
  the simpler version in §7.5.
- Decide whether to keep SQL-side `uid LIKE ?%` push-down in the suttas
  FTS5 paths; if kept, purely for performance, no semantic change.

**Stage G — PRD + task-list updates (§2.9, §2.10, §2.11)**
- Amend `prd-dpd-bold-definitions-search.md`:
  - §4.1: add `bold_ascii TEXT NOT NULL` (mirrors `word_ascii`).
  - §4.2: add `bold_ascii` to `bold_definitions_bold_fts` schema;
    amended `INSERT…SELECT` and triggers.
  - §7: clarify that `dpd-bold-definitions-fts5-indexes.sql` and
    `create_dpd_indexes` run only in bootstrap; the at-startup path
    unchanged. Remove the "~line 786" at-startup sentence.
  - §7: acknowledge `populate_bold_definitions_derived_columns` lives in
    `backend/src/db/dpd.rs` because `import_migrate_dpd` does.
- Amend `tasks-prd-dpd-bold-definitions-search.md` task 1.1 and 2.2 to
  match.

**Stage H — Tests**
- Add a page-size invariant test: for any `(mode, area)` with results >
  `page_len`, successive pages each have exactly `page_len` rows until
  the last.
- Add a Fulltext-merge "no-drop" test: when bold index has a clearly
  top-scoring commentary hit and dict has 20 low-scoring hits, the bold
  hit appears on page 0.
- Add a highlighting test: a diacritic query (e.g. `bhikkhū`) against a
  bold-definition snippet produces at least one highlight span.
- Keep the existing `test_uid_suffix_and_bold_ascii.rs` coverage; extend
  with a uid_prefix + uid_suffix combined case across all three search
  areas.

### 7.8 Risk register

- **Broad queries.** `ContainsMatch` on a single letter could return
  hundreds of thousands of rows. `SAFETY_LIMIT = 100_000` keeps the
  intermediate bounded; if users rely on that pathological case we can
  raise the cap or stream. The pre-branch code had the same issue behind
  its per-page LIMIT, just less visibly — moving it to a documented
  constant is a net improvement.
- **Memory during fulltext merge.** Two `Vec<SearchResult>` of up to
  `SAFETY_LIMIT` each get merged. Use iterator-based merge if this grows
  tight; cloning 100k `SearchResult`s is fine on desktop/mobile hardware
  today.
- **Regression in suttas path.** The Suttas FTS5 paths currently do SQL
  pagination with uid_prefix pushed down. Converting them to `_all`
  variants increases their CPU for unfiltered queries. Mitigate by
  keeping the SQL `uid LIKE ?%` push-down inside the `_all` variant — the
  fetched row set is already narrow.


