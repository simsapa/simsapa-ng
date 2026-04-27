# PRD: DPD Bold Definitions in Dictionary Search

> **Status note (2026-04, post-refactor).** Sections marked here as the
> originally-shipped design were superseded by the refactor described in
> `tasks/query-pipeline-filtering-strategy-refactor.md`. The user-facing
> behaviour in this PRD still holds; the storage and search-pipeline shape
> changed:
>
> - The separate `bold_definitions` tantivy index has been **consolidated
>   into the dict index**. One schema, one index directory per language;
>   `dict_words` and `bold_definitions` rows coexist in
>   `dict_words_index_dir`, distinguished by an `is_bold_definition: bool`
>   field. BM25 is internally consistent — no cross-index merge.
> - The schema gains an `is_bold_definition` field, a `nikaya_group_path`
>   raw field used by bold rows, and a `uid_rev` raw field (lowercased uid
>   reversed character-by-character) used to push uid-suffix filtering down
>   as an anchored regex query.
> - `include_comm_bold_definitions = false` now adds an
>   `Occur::MustNot { is_bold_definition = true }` clause to the dict
>   tantivy query, instead of gating an append step. Default is `true`, so
>   no visible change.
> - DPD Lookup / Headword Match / Contains Match remain SQL-driven and
>   still append bold-definition rows after regular dict rows, but every
>   FTS5 helper now pushes `uid LIKE ?%` and `uid LIKE ?%y` down with
>   parallel `SELECT COUNT(*)` for paginated totals.
>
> See §4.2 (item 9), §4.3 (item 12), and §7 below for the corresponding
> updates inline.

## 1. Introduction / Overview

The DPD (Digital Pāli Dictionary) database contains a `bold_definitions` table,
populated from bold-highlighted terms extracted from Pāli commentaries. Each
row pairs a bold term (the term being defined) with its surrounding commentary
passage, plus hierarchy metadata (nikaya, book, ref_code, title, subhead,
file_name).

Currently these entries are not searchable from Simsapa's Dictionary search
area. This feature integrates `bold_definitions` into all four dictionary
search modes (DPD Lookup, Fulltext Match, Contains Match, Headword Match),
renders them as dictionary-entry-style results, and extends bootstrap
migration + advanced filters accordingly.

**Goal:** Make commentary bold-term definitions first-class results in the
Dictionary search area, with the same look-and-feel as other dictionary
entries.

## 2. Goals

1. Extend the DPD bootstrap migration to produce a searchable
   `bold_definitions` table in `dpd.sqlite3`, including a unique `uid` and a
   derived `commentary_plain` field.
2. Build a tantivy fulltext index and an FTS5 trigram index over
   `bold_definitions.commentary_plain`.
3. Integrate bold definitions into all four Dictionary search modes in
   `query_task.rs`.
4. Render bold-definition results in the dictionary results view with a
   hierarchy header and file_name footer.
5. Add a new "UID suffix" advanced filter option.
6. Add an advanced-search checkbox to toggle inclusion of bold definitions in
   Dictionary search results (default: on).

## 3. User Stories

- As a Pāli reader, I search a term in Dictionary mode and, in addition to
  dictionary headwords, see relevant commentary bold-definition passages so
  that I can understand how the term is glossed in commentaries.
- As a researcher, I want to do a Contains Match on a phrase that only appears
  in commentaries so that I can find commentary glosses without reading each
  source text.
- As an advanced user, I want to filter dictionary results to only those
  coming from a specific commentary source (e.g. `/vvt`) via a "UID suffix"
  filter.
- As a user who prefers a clean headword-only list, I want to turn off
  commentary definitions via an advanced-search checkbox.

## 4. Functional Requirements

### 4.1 Bootstrap migration — schema changes to `bold_definitions`

1. The DPD migration (producing `dpd.sqlite3` from `dpd.db`) **must** add two
   columns to `bold_definitions`:
   - `uid TEXT NOT NULL` — unique per row, always lowercased.
   - `commentary_plain TEXT NOT NULL` — normalized plain-text derived from the
     `commentary` field, built the same way as sutta `content_plain`.
2. `uid` **must** be generated as `lowercase({bold})/lowercase({ref_code})`.
   All uid values must be stored fully lowercased, consistent with uid format
   elsewhere in the app (e.g. `sujjhituṁ/dpd`).
3. Because `{bold}/{ref_code}` is **not unique** in the source DPD database
   (360,624 rows vs. 293,738 distinct combinations, case-sensitive — the
   lowercased rate will be at least as non-unique), the migration **must**
   disambiguate duplicates during generation:
   - The first occurrence of a lowercased `(bold, ref_code)` pair gets uid
     `{bold}/{ref_code}`.
   - Each subsequent occurrence gets uid `{bold} N/{ref_code}` where `N` is
     `2, 3, 4, …` — i.e. the number is appended to the `bold` portion with a
     single space separator (e.g. `tatthā 2/pyt`, `tatthā 3/pyt`).
   - Order of occurrence is deterministic (ORDER BY `id`).
4. `uid` **must** be `UNIQUE` after migration, enforced by an index.
5. `commentary_plain` is produced from `commentary` using the same
   normalization/strip pipeline used for suttas (HTML stripped, Pāli
   diacritics normalized, whitespace collapsed).

### 4.2 Bootstrap migration — indexes

6. A btree `UNIQUE` index **must** exist on `bold_definitions.uid`.
7. An FTS5 virtual table `bold_definitions_fts` (for commentary substring
   search) **must** be created using the **trigram tokenizer**, with
   `detail='none'`, indexing `commentary_plain` and storing
   `bold_definitions_id UNINDEXED`. Kept in sync via INSERT/UPDATE/DELETE
   triggers (same pattern as `dpd_headwords_fts` in
   `scripts/dpd-fts5-indexes.sql`).
8. A second FTS5 virtual table `bold_definitions_bold_fts` (for bold-term
   substring search used by DPD Lookup and Headword Match) **must** be
   created using the **trigram tokenizer**, with `detail='none'`, indexing
   the `bold` field. This mirrors the existing `dpd_headwords_fts` pattern
   and avoids slow leading-wildcard `LIKE` scans across 360k rows.
9. **Updated post-refactor.** Bold-definition rows are appended into the
   existing **unified dict tantivy index** (`dict_words_index_dir`),
   distinguished by an `is_bold_definition: bool` field. The dict schema
   (`build_dict_schema`) covers both `dict_words` and `bold_definitions`
   rows; bold-only fields (`bold_definitions_id`, `nikaya_group_path`,
   `ref_code`, `file_name`) live alongside the regular dict fields and
   are unset/empty for non-bold docs. The unified schema also carries a
   `uid_rev` raw field (lowercased uid reversed character-by-character)
   so a uid-suffix filter can be pushed down as
   `RegexQuery::from_pattern("{reversed}.*", uid_rev_field)`. The Pāli
   tokenizer (`lang = "pli"`) registers once on the unified index. There
   is no separate `bold_definitions_index_dir` and no separate
   `IndexType::BoldDefinitions`. (Originally specified as a stand-alone
   tantivy index built in parallel with the dict index — that two-index
   design has been deleted.)

### 4.3 Search integration — Dictionary search area (`query_task.rs`)

10. When `SearchArea::Dictionary` is active, the query pipeline **must**
    additionally search `bold_definitions` according to the selected mode:
    - **DPD Lookup:** case-insensitive substring match against the `bold`
      field via the `bold_definitions_bold_fts` trigram FTS5 index
      (equivalent to `LIKE '%query%'` but index-accelerated). The user's
      raw query string is used — **no** inflection-table expansion is
      applied (so e.g. `gacchati` is not required to find `gatvā`). This
      matches the existing convention for Headword Match on dict headwords
      (see `lemma_1_dpd_headword_match_fts5` in `query_task.rs`).
    - **Headword Match:** same as DPD Lookup for bold definitions —
      trigram FTS5 substring match on `bold`. This is consistent with the
      existing Headword Match semantics in the codebase, which also uses a
      trigram FTS5 substring against `dpd_headwords.lemma_1`.
    - **Fulltext Match:** query the tantivy index built on
      `commentary_plain`; results ranked by relevance (BM25).
    - **Contains Match:** query via `bold_definitions_fts` (FTS5 trigram)
      on `commentary_plain`, equivalent to `LIKE '%query%'`.
11. For Fulltext Match and Contains Match the query input **must** be
    normalized (same pipeline used for suttas) before being issued, because
    `commentary_plain` is normalized. For DPD Lookup and Headword Match the
    `bold` field is stored as-is and matched case-insensitively — the query
    input **must not** be normalized.
12. Merging of results into the Dictionary result list:
    - **Fulltext Match (post-refactor):** dict_words and bold_definitions
      docs share **one** tantivy index, so a single `BooleanQuery` returns
      already-ranked, already-paginated results; BM25 is internally
      consistent. No cross-index merge step. The
      `include_comm_bold_definitions` flag is realized as
      `Occur::MustNot { is_bold_definition = true }` when set to `false`;
      default `true` is a no-op (both kinds participate). Per-doc dispatch
      in `dict_doc_to_result` peeks at `is_bold_definition` and routes to
      `bold_definition_doc_to_result` for bold rows. (Originally specified
      as a merge of two tantivy indexes' BM25 scores — that two-index
      design has been deleted.)
    - **Contains Match:** bold-definition results **appended** after other
      dictionary results. Pagination uses the boundary-aware
      `split_page_across_streams` orchestrator (regular slice ⊕ bold slice
      with true `LIMIT/OFFSET` SQL, no Rust-side cover-fetch).
    - **DPD Lookup:** bold-definition results **appended** after regular DPD
      headword results, with the same boundary-aware orchestrator.
    - **Headword Match:** bold-definition results **appended** after regular
      dictionary headword results, with the same boundary-aware
      orchestrator.
13. Bold-definition results **must** be clickable and open an inline detail
    view in the dictionary results area, rendered as in §4.4 below (no
    navigation to the source sutta on click).

### 4.4 Rendering — HTML output for a bold definition

14. Each bold-definition result is rendered with three regions:
    - **Header:** the hierarchy breadcrumb
      `nikaya › book (ref_code) › title › subhead`.
    - **Body:** the `bold` term (styled as the headword) followed by the
      `commentary` definition (HTML preserved). The body uses the original
      `bold` string from the row — any disambiguation numbering lives only
      in the `uid`, never in the displayed term.
    - **Footer:** the `file_name` (small, muted). The `uid` is also shown on
      the result item itself (as for other dictionary results), so the
      header does not need to repeat it.
15. The header and footer **must not** be part of `commentary_plain`;
    `commentary_plain` is derived solely from the `commentary` field.

### 4.5 Advanced search options

16. A new advanced filter option **"UID suffix"** is added alongside the
    existing "UID prefix" option.
    - Behavior: plain string suffix match against the full (lowercased)
      `uid` (e.g. input `/bodhi` matches any result whose uid ends with
      `/bodhi`; `/vvt` matches uids ending `/vvt`). Input is lowercased
      before comparison to match the stored uid format.
    - Applies to all search modes and to both regular dictionary entries
      and bold-definition entries.
    - Applies to **all search areas** (Suttas, Dictionary, Library),
      consistent with UID prefix. The advanced-options panel's current
      visibility gate (restricted to Suttas in `SearchBarInput.qml:36`)
      **must** be widened so UID prefix and UID suffix are visible across
      search areas. Every `SearchResult` returned by the query pipeline
      **must** carry a `uid` value so the suffix filter can apply
      uniformly.
17. A new checkbox **`include_comm_bold_definitions_checkbox`** labeled
    **"Dictionary Commentary Definitions in Search"** is added to the
    advanced options panel for the Dictionary search area.
    - Default: **checked** (bold definitions included).
    - State **must** be persisted across sessions in user settings via a
      bridge getter/setter pair (same pattern as `include_ms_mula_checkbox`
      at `SearchBarInput.qml:403`).
    - `SearchParams` construction sites **must** read this value from the
      persisted setting at query time — not rely on a Rust struct default.
    - When unchecked: bold-definition results are excluded from all four
      search modes.
    - A sibling info **button** (flat, `fa_circle-info-solid.png` icon)
      next to the checkbox opens the app's `info_dialog` with title
      *"Dictionary Commentary Definitions in Search"* and message *"Also
      search bold-highlighted terms extracted from Pāli commentaries (DPD
      bold definitions). Turn off for headword-only results."* This
      mirrors the existing info-button pattern at
      `SearchBarInput.qml:413` for `include_ms_mula_checkbox`.

## 5. Non-Goals (Out of Scope)

- No edits to the upstream `dpd.db`; all schema changes happen in the Simsapa
  migration that produces `dpd.sqlite3`.
- No link-out from a bold-definition result to the source sutta/commentary
  file — click behavior is inline detail only.
- No normalized sibling column for `bold`; case-insensitive storage-as-is is
  sufficient.
- No inflection-table expansion for bold terms in DPD Lookup.
- No cross-search-area integration — bold definitions appear only in the
  Dictionary search area, not Suttas or Combined.
- No UI for browsing bold definitions by hierarchy (nikaya/book/title)
  outside of rendered result headers.

## 6. Design Considerations

- Hierarchy header styling should reuse existing dictionary-entry header CSS
  where possible (small, muted breadcrumb above the headword block).
- The bold term in the body should use the same visual weight as a
  dictionary headword so the result reads as "a definition of X".
- The advanced options panel already groups UID prefix; place "UID suffix"
  directly after it. Place the `include_comm_bold_definitions_checkbox` in
  the Dictionary-specific section of the advanced options.

## 7. Technical Considerations

- **Population is a bootstrap-only step.** The uid + commentary_plain
  population runs once as part of the CLI bootstrap that produces
  `dpd.sqlite3` (in `cli/src/bootstrap/dpd.rs`), not on every app startup.
  The at-startup path (`backend/src/db/dpd.rs`) already runs SQL index
  scripts idempotently and **must not** attempt to re-populate the
  columns. Migration code **must** be idempotent regardless (skip if uid
  already populated).
- Reference implementation for FTS5 trigram setup:
  `scripts/dpd-fts5-indexes.sql`. The two new scripts
  (`bold_definitions_fts` over `commentary_plain`, and
  `bold_definitions_bold_fts` over `bold`) should follow the same
  structure (drop existing, create virtual table, populate, add INSERT/
  UPDATE/DELETE triggers, `optimize`, `VACUUM`).
- Reference for `commentary_plain` normalization: the sutta `content_plain`
  pipeline — reuse the same normalization function, do not duplicate
  logic.
- **Tantivy index build (post-refactor).** Bold-definition rows are
  appended into the unified dict tantivy index by
  `append_bold_definitions_to_dict_index` (no `delete_all_documents`;
  opens the existing per-language dict subdir under
  `dict_words_index_dir`). The bootstrap step runs after the per-language
  dict build; there is no `bold_definitions_index_dir`. The Pāli
  tokenizer (`lang = "pli"`) is registered once on the unified index.
  (Originally specified as a separate `dpd_bold_definitions` index with
  its own builder — that path has been deleted.)
- `query_task.rs` changes should keep each search mode's branch readable;
  factor the bold-definitions query into helpers per mode to avoid
  bloating existing branches.
- The UNIQUE uid index should be created **after** population to avoid
  mid-migration conflicts while the disambiguation numbering is applied.
- HTML rendering of a bold-definition result is dispatched from the
  existing dict-HTML assembly site (`app_data.rs::render_word_html_by_uid`
  at `backend/src/app_data.rs:332`) — extend it to branch on uid/source
  and call the new `render_bold_definition` renderer.
- Keep use of `try_exists()` (not `.exists()`) for any filesystem checks
  in the migration/bootstrap code paths, per Android compatibility rules
  in `CLAUDE.md`.

## 8. Success Metrics

1. Migration produces `dpd.sqlite3` with `bold_definitions.uid` fully
   populated, lowercased, and 100% unique — matching the source row count
   (360,624 at current dataset version).
2. Searching a commentary-only term in Dictionary Fulltext Match returns at
   least one bold-definition hit.
3. All four search modes return bold-definition results when the
   `include_comm_bold_definitions_checkbox` is on, and zero when it is off.
4. "UID suffix" filter correctly restricts results (spot-checked against
   `/vvt`, `/pyt`, `/dpd`).
5. No regressions in existing dictionary search behavior (headword/DPD
   lookup latency and result counts for a benchmark set of queries remain
   within ~5% of baseline).

## 9. Open Questions

None at time of writing — prior open questions resolved:
- Tantivy bundle size: not a concern.
- Checkbox persistence: persisted in user settings, like other advanced-
  options checkboxes.
- DPD Lookup on bold: simple substring match only, no inflection lookup.
- Disambiguation numbering: lives only inside `uid`; the rendered header and
  body show the original `bold` value unchanged, and the uid is displayed on
  the result item itself.
