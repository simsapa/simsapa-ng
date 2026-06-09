# PRD: Fixing the slow Headword Match query

## Status

**Resolved.** Root cause: the on-disk SQLite databases shipped without
`sqlite_stat1` / `sqlite_stat4` tables, so SQLite's query planner had no
statistics and fell back to heuristics that produced a catastrophic plan for
the Headword Match query (FTS5 trigram + `dict_label IN (...)`). Running
`ANALYZE` once populates the stat tables and brings the query from ~170 s to
~120 ms end-to-end (~17 ms for the SQL itself).

## Resolution

Two changes:

1. **One-time runtime `ANALYZE`** — `backend/src/db/mod.rs::ensure_sqlite_stats`
   runs at `DbManager::new()` for each of `appdata`, `dictionaries`, `dpd`.
   It checks for the `sqlite_stat1` table; if missing, it runs `ANALYZE` once
   (idempotent, <1 s per DB at our scale). Existing installs self-heal on
   their next launch.
2. **Bundled SQLite version bump** (kept though it wasn't the root cause):
   `diesel` 2.2.10 → 2.3.9, `diesel_migrations` 2.2.0 → 2.3.0,
   `libsqlite3-sys` `"*"` → `">=0.32"` (Cargo resolves to 0.36.0, bundles
   SQLite ~3.50.x). Transitively bumps `rusqlite` 0.31 → 0.38 and
   `stardict` 0.2.2 → 0.2.3.

A small unrelated improvement also landed during this investigation:
**snippet generation in Headword Match is now deferred until after
pagination** (`backend/src/query_task.rs::lemma_1_dpd_headword_match_fts5` and
`headword_match_with_bold`). Previously, `_full()` built a `SearchResult` for
every matched row — for DPD rows that meant a per-row `get_dpd_meaning_snippet`
query (a fresh `dpd_headwords` lookup each time). Now only the ~20 rows on
the displayed page get snippeted. This wasn't the dominant cost in the
174 s wedge (the SQL itself was), but it would have shown up next.

The two-path retrieval design was also collapsed into one query against
`dict_words_fts.word`, since for `dict_label = "dpd"` the value of
`dict_words.word` mirrors `dpd_headwords.lemma_1`. `dpd_headwords_fts` is
retained for DPD Lookup mode, which still uses it.

## Bootstrap follow-up (recommended)

Bootstrap should `ANALYZE` each DB after building it, so shipped DBs arrive
with stats and the runtime self-heal is just a safety net. Touch points:

- `cli/src/bootstrap/` — wherever each of `appdata.sqlite3`,
  `dictionaries.sqlite3`, `dpd.sqlite3` is produced, append `ANALYZE;` after
  inserts and after FTS5 indexing.
- The FTS5 scripts in `scripts/` already include `INSERT INTO <fts>(<fts>) VALUES('optimize');`
  and `VACUUM;`; an `ANALYZE;` should land right after the optimize step.

Not done here — bootstrap rebuild is out of scope for this fix.

## What was tried and ruled out (chronological)

1. **N+1 across two databases** (original suspect). Path A did one
   `dict_words` lookup per matched DPD headword. Collapsed to one FTS5 query
   against `dict_words_fts`. Genuine improvement, but the wedge persisted.
2. **Per-row snippet generation.** Deferred from `_full()` to the page slice.
   Genuine improvement (would have mattered after the wedge was gone), but
   not the dominant cost.
3. **FTS5 trigram + bind-parameter LIKE optimization.** `EXPLAIN QUERY PLAN`
   in `sqlite3` CLI showed identical plans (`SCAN f VIRTUAL TABLE INDEX 0:L2`)
   for literal and bind variants, both fast. Not the trigger.
4. **Disk I/O / cold page cache.** NVMe SSD; cold `sqlite3` runs still 12 ms.
5. **Tantivy reconcile / dictionary import holding a writer lock.** User
   confirmed neither was running during the slow queries. Reproducibly slow,
   not bursty. WAL-mode change prepared, reverted.
6. **Connection-pool starvation / wedged pool connection.** Controls A and B
   on the same pool connection ran in 17 ms; only the full-WHERE query (C, D)
   was slow. Connection itself was healthy.
7. **Diesel cached prepared statement with a bad plan.** Control C used a
   unique trailing comment so Diesel could not reuse a cached statement.
   Still 170 s. Not a cached-plan issue.
8. **Wide `SELECT dw.*` materialising big `definition_html` columns.**
   `SELECT COUNT(*)` (C) and `SELECT dw.id` (D) were equally slow as
   `SELECT dw.*`. Not row materialisation.
9. **Bundled vs system SQLite version mismatch.** Bumped diesel + libsqlite3-sys
   to bring bundled SQLite from 3.45.0 to ~3.50.x. Still 170 s. Not the cause,
   but the bump was kept for hygiene.
10. **Bind parameters on the IN clause / uid LIKE.** Control F inlined every
    value as a literal — still 170 s. Not a bind-parameter interaction.

## What it actually was

The on-disk `dictionaries.sqlite3` (and the other shipped DBs) had **no
`sqlite_stat*` tables**. Without them, the planner can't know that the FTS5
LIKE side returns ~96 rows out of a 192 k-row table, so on encountering the
combined predicate `f.word LIKE ? AND dw.dict_label IN (?, ?, ?, ?, ?, ?)` it
chose a plan that scanned `dict_words` (or the FTS5 content shadow table) in
a way that visited the full table per IN-list element rather than using the
FTS5 trigram driver. Once `ANALYZE` populated `sqlite_stat1`/`sqlite_stat4`,
the planner switched to the expected `SCAN f VIRTUAL TABLE INDEX 0:L2` →
`SEARCH dw USING INTEGER PRIMARY KEY (rowid=?)` plan and the same SQL ran in
~17 ms.

`EXPLAIN QUERY PLAN` in the `sqlite3` CLI had been showing the fast plan all
along — but the CLI was reading the same DB file without using stats either,
so why does it work without stats? The system SQLite is compiled with
`ENABLE_STAT4` plus other defaults that change the no-stats heuristics; the
bundled SQLite from `libsqlite3-sys` is built with different defaults and
made the bad choice. We didn't pin this down precisely (would require diffing
`PRAGMA compile_options` between the two builds), but the fix — ensure stats
are present — works for both code paths and is the right thing to do
regardless of compile flags.

## Diagnostic timings (final)

After `ANALYZE` on the dictionaries DB, with the original full query
("gacchati", 6 dict_source_uids):

```
ctrl A (pool, fts only):              18.7 ms
ctrl B (pool, join, no filters):      19.0 ms
ctrl C (pool, full WHERE, COUNT only): 18.9 ms
ctrl D (pool, +IN only):              18.6 ms
ctrl E (pool, +uid LIKE only):        18.9 ms
ctrl F (pool, full WHERE, ALL INLINED): 18.6 ms
headword_match SQL load:              23.0 ms  (rows=96)
Query took:                          136.8 ms
```

vs the pre-fix figures:

```
ctrl D (pool, +IN only):           163.7 s
ctrl F (pool, full WHERE, INLINED): 164.9 s
headword_match SQL load:            172.2 s
Query took:                         515.6 s   (includes the 6 controls × ~170 s each)
```

## Files touched

- `backend/src/db/mod.rs` — added `ensure_sqlite_stats`, called from
  `DbManager::new()` for each of the three DBs.
- `backend/src/query_task.rs` — collapsed N+1 paths, deferred snippet
  generation to page-slice callers (`lemma_1_dpd_headword_match_fts5`,
  `headword_match_with_bold`), removed the diagnostic controls.
- `backend/Cargo.toml`, `cli/Cargo.toml` — diesel 2.3.9, diesel_migrations
  2.3.0, libsqlite3-sys `>=0.32` (kept after investigation; SQLite 3.50.x is
  preferable to 3.45.0 even though the version bump didn't fix the bug).
- `backend/Cargo.lock` (regenerated).

## Deferred / future work

- **ASCII folding** (`sankh → saṅkhāra`). Out of scope here. Requires a
  `word_ascii` column on `dict_words_fts` plus a bootstrap re-build of
  `dictionaries.sqlite3`.
- **Bootstrap-time `ANALYZE`** (see "Bootstrap follow-up" above) — so we can
  remove the runtime self-heal in a future release once all shipped DBs are
  known to have stats.
- **Buffer-cache for `_full()`** — originally asked for, no longer urgent now
  that `_full()` is ~17 ms. The bridge's `RESULTS_PAGE_CACHE` already covers
  revisited pages.
