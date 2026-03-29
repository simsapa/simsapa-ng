# PRD: CST4 & Commentary Filter Settings

## Introduction/Overview

Users need control over whether CST4 (Chaṭṭha Saṅgāyana Tipiṭaka 4) Pāli texts and commentary texts (Aṭṭhakathā, Ṭīkā) appear in search results and translation tabs. Currently, all related texts are loaded unconditionally when viewing translations, and search result filtering for CST4/commentary sources is not exposed as a user setting.

This feature adds four new toggles to the "Find" section of AppSettingsWindow, controlling:
- Whether commentary texts appear in translation tabs
- Whether CST4 mūla (root) texts appear in search results
- Whether commentary texts appear in search results
- Whether CST4 mūla texts appear in translation tabs

## Goals

1. Give users fine-grained control over CST4 and commentary visibility in search results and translation tabs.
2. Reduce noise from duplicate Pāli results (CST4 vs MS) while preserving access when needed.
3. Follow the existing settings persistence pattern (AppSettings struct → JSON blob in database → RwLock cache).
4. Settings apply to new queries and newly opened suttas (no live refresh of already-open tabs required).

## User Stories

- As a Pāli student, I want to include CST4 texts in my search results so that I can compare variant spellings between MS and CST4 editions.
- As a reader, I want to hide CST4 duplicates from search results (default) so that my results are less cluttered.
- As a scholar, I want to see Aṭṭhakathā and Ṭīkā commentary alongside the root text translations so that I can study the commentarial tradition.
- As a casual reader, I want to hide commentary from translations so that I only see the root texts and their translations.
- As a researcher, I want commentary records to appear in search results so that I can search within the commentaries themselves.

## Functional Requirements

### New Settings

Four new boolean fields in `AppSettings`:

| # | Setting | Field name | Default | Location affected |
|---|---------|-----------|---------|-------------------|
| 1 | Include commentary in translations | `include_commentary_in_translations` | `false` | Translation tabs |
| 2 | Include cst4 mūla in search results | `include_cst4_mula_in_search_results` | `false` | Search results |
| 3 | Include commentary in search results | `include_commentary_in_search_results` | `true` | Search results |
| 4 | Include cst4 mūla in translations | `include_cst4_mula_in_translations` | `false` | Translation tabs |

### FR-1: Settings Persistence (Rust backend)

1. Add the four new boolean fields to the `AppSettings` struct in `backend/src/app_settings.rs` with the defaults specified above.
2. Add getter/setter pairs in `backend/src/app_data.rs` following the existing pattern: read from `app_settings_cache` (RwLock), write to cache + serialize full struct to database.
3. Expose getter/setter functions on the SuttaBridge in `bridges/src/sutta_bridge.rs` so QML can call them.
4. Add corresponding function signatures in the qmllint type definition `assets/qml/com/profoundlabs/simsapa/SuttaBridge.qml`.

### FR-2: QML Settings UI

5. Add four checkboxes to the "Find" section of `assets/qml/AppSettingsWindow.qml`, following the existing pattern (property alias, `onCheckedChanged` → `SuttaBridge.set_*()`, load in `Component.onCompleted`).
6. Each checkbox must have a label and a description line beneath it, matching the style of existing settings in the Find section.

The settings should appear in this order within the Find section:
- Include commentary in translations
- Include cst4 mūla in search results
- Include commentary in search results
- Include cst4 mūla in translations

### FR-3: Search Result Filtering — CST4 Mūla

7. When `include_cst4_mula_in_search_results` is `false` (default), exclude sutta records where the UID ends with `/cst4` AND the reference part (before the first `/`) does NOT contain `.att`, `.tik`, or `.xml`.
   - Excluded: `mn1/pli/cst4`, `dn22/pli/cst4`
   - NOT excluded: `mn1.att/pli/cst4` (commentary), `mn1.tik/pli/cst4` (sub-commentary), `s0502.att.xml/pli/cst4` (XML-based record)
8. When `include_cst4_mula_in_search_results` is `true`, no CST4 filtering is applied.
9. This filtering must be applied in all SearchMode types where source filtering is already implemented: `ContainsMatch`, `RegExMatch`, `FulltextMatch` (FTS5 queries), and any other modes that already apply source filters in `query_task.rs`.

### FR-4: Search Result Filtering — Commentary

10. When `include_commentary_in_search_results` is `false`, exclude sutta records where the reference part (before the first `/`) contains `.att` or `.tik` (but NOT `.xml` suffixed records like `s0502.att.xml`).
    - Excluded: `mn1.att/pli/cst4`, `mn1.tik/pli/cst4`
    - NOT excluded: `s0502.att.xml/pli/cst4`
11. When `include_commentary_in_search_results` is `true` (default), no commentary filtering is applied to search results.
12. Applied in the same SearchMode types as FR-3.

### FR-5: Translation Tab Filtering — Commentary

13. In `get_translations_data_json_for_sutta_uid()` (in `backend/src/db/appdata.rs`), when `include_commentary_in_translations` is `false` (default), do NOT include the `.att/%` and `.tik/%` LIKE patterns in the query. Only fetch `uid_ref/%` matches.
14. When `include_commentary_in_translations` is `true`, include `.att` and `.tik` patterns as currently implemented.

### FR-6: Translation Tab Filtering — CST4 Mūla

15. In `get_translations_data_json_for_sutta_uid()`, when `include_cst4_mula_in_translations` is `false` (default), exclude records where UID ends with `/cst4` AND the reference part does NOT contain `.att`, `.tik`, or `.xml`.
    - This uses the same logic as FR-3 but applied to the translations query.
16. When `include_cst4_mula_in_translations` is `true`, no CST4 filtering is applied to translations.

### FR-7: Translation Tab Sort Order

17. When CST4 translations are included, `mn1/pli/cst4` should appear right after `mn1/pli/ms`, grouped with other Pāli records. Commentaries (when enabled) follow in the same Pāli group. The existing `sort_suttas()` function should handle this — verify and adjust if needed.

### Example: Translation Tabs for `mn1/en/sujato`

When the user clicks on `mn1/en/sujato` in search results, the translation tabs should show:

| UID | Condition |
|-----|-----------|
| `mn1/pli/ms` | Always shown |
| `mn1/pli/cst4` | Only if `include_cst4_mula_in_translations` is `true` |
| `mn1.att/pli/cst4` | Only if `include_commentary_in_translations` is `true` (NOT affected by cst4 mūla filter) |
| `mn1.tik/pli/cst4` | Only if `include_commentary_in_translations` is `true` (NOT affected by cst4 mūla filter) |
| `mn1/en/bodhi` | Always shown |
| `mn1/en/horner` | Always shown |
| `mn1/en/suddhaso` | Always shown |
| `mn1/en/thanissaro` | Always shown |

## Non-Goals (Out of Scope)

- Live-updating already-open sutta tabs when settings change. Settings apply to new queries and newly opened suttas only.
- Filtering XML-based records (e.g., `s0502.att.xml/pli/cst4`) — these are always included regardless of cst4/commentary filters.
- Adding source filtering to SearchMode types that don't already support it (e.g., UID exact match, Title match).
- Changing dictionary search filtering — these settings apply to sutta searches only.

## Technical Considerations

### Files to Modify

| File | Changes |
|------|---------|
| `backend/src/app_settings.rs` | Add 4 new fields to `AppSettings` struct and `Default` impl |
| `backend/src/app_data.rs` | Add 4 getter/setter pairs following existing pattern |
| `bridges/src/sutta_bridge.rs` | Expose 4 getter/setter bridge functions |
| `assets/qml/com/profoundlabs/simsapa/SuttaBridge.qml` | Add qmllint type definitions for new bridge functions |
| `assets/qml/AppSettingsWindow.qml` | Add 4 checkboxes in Find section |
| `backend/src/query_task.rs` | Add cst4 mūla and commentary filtering logic in search modes that already have source filtering |
| `backend/src/db/appdata.rs` | Modify `get_translations_data_json_for_sutta_uid()` to conditionally include commentary and cst4 |

### Filtering Logic

To distinguish mūla cst4 records from commentary/XML cst4 records, the filter should check:
```
UID ends with "/cst4" AND reference_part NOT LIKE "%.att%" AND reference_part NOT LIKE "%.tik%" AND reference_part NOT LIKE "%.xml%"
```

In SQL terms, for excluding cst4 mūla:
```sql
NOT (uid LIKE '%/cst4' AND uid NOT LIKE '%.att%/cst4' AND uid NOT LIKE '%.tik%/cst4' AND uid NOT LIKE '%.xml%/cst4')
```

For excluding commentary:
```sql
NOT (uid LIKE '%.att/%' AND uid NOT LIKE '%.xml%') AND NOT (uid LIKE '%.tik/%' AND uid NOT LIKE '%.xml%')
```

### Passing Settings to Query Functions

`get_translations_data_json_for_sutta_uid()` and the search query functions in `query_task.rs` need access to the `AppSettings` cache. The translation function is on `AppdataDb` — it will need the settings values passed as parameters. Search queries in `query_task.rs` should read from `app_settings_cache` (already accessible via `AppData`).

## Implementation Stages

### Stage 1: Settings Infrastructure
- Add fields to `AppSettings` struct with defaults
- Add getter/setter pairs in `app_data.rs`
- Add bridge functions in `sutta_bridge.rs`
- Add qmllint definitions
- Add QML checkboxes in AppSettingsWindow

### Stage 2: Search Result Filtering
- Implement cst4 mūla filtering in `query_task.rs` search modes
- Implement commentary filtering in `query_task.rs` search modes
- Read settings from `app_settings_cache` in query functions

### Stage 3: Translation Tab Filtering
- Modify `get_translations_data_json_for_sutta_uid()` to accept filter parameters
- Implement conditional commentary inclusion
- Implement conditional cst4 mūla inclusion
- Verify sort order with `sort_suttas()`

## Success Metrics

- All four toggles persist correctly across app restarts.
- Default behavior (all off except commentary in search) matches current implicit behavior.
- Enabling "include cst4 mūla in search results" adds CST4 root texts to results without affecting commentary or XML records.
- Enabling "include commentary in translations" shows `.att` and `.tik` tabs.
- Enabling "include cst4 mūla in translations" shows `*/pli/cst4` tab after `*/pli/ms`.
- Disabling "include commentary in search results" removes `.att`/`.tik` records from search results without affecting XML records.

## Open Questions

1. Should `get_translations_data_json_for_sutta_uid()` receive the settings as function parameters, or should it access the `AppSettings` cache directly? (Current architecture suggests passing parameters since `AppdataDb` doesn't hold a reference to the cache.)
2. Are there edge cases where a sutta exists in MS but not in CST4, or vice versa, that could affect the expected tab behavior?
3. Does the current `sort_suttas()` function already sort by source in a way that places `cst4` after `ms`, or does it need adjustment?
