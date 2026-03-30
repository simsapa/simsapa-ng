# PRD: TipitakaXmlImporter Refactor

## Introduction/Overview

The `TipitakaXmlImporter` in `cli/src/bootstrap/tipitaka_xml.rs` needs to be rewritten. The upstream `tipitaka-xml-parser` library was updated and the current code references types and functions that no longer exist (`FragmentAdjustments`, `load_fragment_adjustments()`). The old approach re-parsed XML files from scratch. The new approach uses two import paths:

1. **Fragment-based import** — For a whitelist of checked XML files, read pre-parsed sutta fragments directly from `fragments.sqlite3` and insert them as individual sutta records with proper SC-code UIDs.
2. **Direct XML import** — For all other XML files in `romn/`, import each file as a single large sutta record with a filename-based UID.

## Goals

1. Make `TipitakaXmlImporter` compatible with the updated `tipitaka-xml-parser` library.
2. Import whitelisted files from `fragments.sqlite3` as individual sutta records (one per fragment row).
3. Import all remaining XML files from `romn/` as single sutta records.
4. Preserve correct UID format for commentaries and subcommentaries.
5. Remove dead code and simplify the module (no dry-run, no backward compatibility).

## User Stories

- As a developer running bootstrap, I want the tipitaka XML import to succeed without compilation errors against the updated `tipitaka-xml-parser`.
- As a user, I want all Pali texts from the CST tipitaka XML to be available in the app — checked files with fine-grained sutta records, and unchecked files as whole-file records.

## Functional Requirements

### 1. `TipitakaXmlImporter` struct

```rust
pub struct TipitakaXmlImporter {
    fragments_db_path: PathBuf,
    romn_dir: PathBuf,
}
```

Constructor takes two arguments:
- `fragments_db_path: PathBuf` — path to `fragments.sqlite3`
- `romn_dir: PathBuf` — path to `tipitaka-xml/romn/` directory

### 2. `import(&mut self, conn: &mut SqliteConnection) -> Result<()>`

Main entry point. Performs two phases in order:

**Phase 1: Fragment-based import (whitelisted files)**

- Maintain a hardcoded whitelist of checked filenames (same `let files = [...]` pattern as current code, updated as needed).
- Open a read-only connection to `fragments.sqlite3`.
- For each whitelisted filename, query all rows from `xml_fragments` where `cst_file = <filename>` and `frag_type = "Sutta"`.
- For each fragment row, build a `SuttaRecord`:
  - **UID**: Use the `sc_code` field from the fragment row. Determine commentary type from the `cst_file` name:
    - `.att.xml` → `<sc_code>.att/pli/cst`
    - `.tik.xml` → `<sc_code>.tik/pli/cst`
    - `.mul.xml` → `<sc_code>/pli/cst`
  - **Title**: Use `cst_sutta` field from the fragment row (apply `consistent_niggahita` normalization).
  - **Content HTML**: Convert `content_xml` to HTML using `tipitaka_xml_parser::sutta_builder::xml_to_html()`. Wrap in the same `<div class="cst">` header structure as current code.
  - **Content plain**: Derive from HTML using `sutta_html_to_plain_text()`.
  - **Group path**: Deserialize the `group_levels` JSON field. Build path from levels excluding Nikaya and Sutta types, joined by ` / `.
  - **Nikaya**: Use the `nikaya` field from the fragment row.
  - **source_uid**: `"cst"`
- Skip fragments that have no `sc_code` (log a warning).
- Skip duplicate UIDs (log a warning).
- Insert all built sutta records into the appdata `suttas` table via Diesel.

**Phase 2: Direct XML import (remaining files)**

- List all `*.xml` files in the `romn/` directory.
- Exclude files that are in the whitelist (already imported in Phase 1).
- For each remaining XML file:
  - Read the file content using `tipitaka_xml_parser::encoding::read_xml_file()`.
  - Convert full XML content to HTML using `xml_to_html()`.
  - **UID**: `<xml_filename>/pli/cst` (e.g., `s0401m.mul.xml/pli/cst`)
  - **Title**: Use the filename (without `.xml` extension, e.g., `s0401m.mul`).
  - **Content HTML**: Wrap converted HTML in `<div class="cst">` with a header using the filename as title.
  - **Content plain**: Derive from HTML.
  - **Nikaya**: Detect from filename prefix if possible (e.g., `s01` = digha, `s02` = majjhima), or use `"unknown"`.
  - **source_uid**: `"cst"`
- Insert into appdata `suttas` table.

### 3. `SuttaRecord` struct

Keep the same fields as the current struct — it maps to `NewSutta` for Diesel insertion:

```rust
pub struct SuttaRecord {
    pub uid: String,
    pub sutta_ref: String,
    pub nikaya: String,
    pub language: String,
    pub group_path: Option<String>,
    pub group_index: Option<i32>,
    pub order_index: Option<i32>,
    pub title: Option<String>,
    pub title_pali: Option<String>,
    pub content_plain: Option<String>,
    pub content_html: Option<String>,
    pub source_uid: Option<String>,
}
```

### 4. Database insertion

- Use a Diesel transaction for batch insertion (same pattern as current `insert_suttas_with_conn`).
- Check for existing UIDs before inserting, skip duplicates.
- Log counts: total processed, inserted, skipped.

### 5. Dependencies to use from `tipitaka-xml-parser`

- `tipitaka_xml_parser::encoding::read_xml_file` — for reading XML files in Phase 2.
- `tipitaka_xml_parser::detect_nikaya_structure` — for detecting nikaya from XML content in Phase 2.
- `tipitaka_xml_parser::sutta_builder::xml_to_html` — for XML-to-HTML conversion in both phases.
- `tipitaka_xml_parser::fragments_schema::xml_fragments` — for querying `fragments.sqlite3` (Diesel schema).
- `tipitaka_xml_parser::fragments_models::XmlFragmentRecord` — for deserializing query results.

### 6. Dependencies to remove

- Remove imports of `FragmentAdjustments`, `load_fragment_adjustments`, `parse_into_fragments`, `FileImportStats`, `NikayaStructure`, `XmlFragment`, `FragmentType`, `GroupType`.
- The `build_suttas()` standalone function is removed — its logic is inlined into Phase 1.

## Non-Goals (Out of Scope)

- Re-parsing XML files for whitelisted files (that's what fragments.sqlite3 is for).
- Dry-run mode.
- Verbose mode / configurable logging levels.
- Backward compatibility with the old `FragmentAdjustments` API.
- Updating the whitelist contents (the same files as current code are used).
- Changes to `cli/src/bootstrap/mod.rs` (it already passes both paths).

## Technical Considerations

- The `fragments.sqlite3` connection is separate from the appdata connection. Open it read-only.
- The `group_levels` column in `xml_fragments` is a JSON string containing an array of `GroupLevel` objects. Define a local deserialization struct or reuse `tipitaka_xml_parser::types::GroupLevel` if it's public.
- The `xml_to_html()` function from `tipitaka_xml_parser::sutta_builder` handles the XML-to-HTML conversion.
- The `frag_type` column stores `"Header"` or `"Sutta"` as strings — filter for `"Sutta"` when querying.

## Success Metrics

- `make build -B` compiles without errors.
- `cd backend && cargo test` passes.
- Running bootstrap imports all whitelisted files as individual sutta records and all remaining XML files as single records.

## Resolved Questions

1. **Nikaya detection for Phase 2**: Use `detect_nikaya_structure()` from `tipitaka-xml-parser` — read the XML content and detect the nikaya from the structure. This requires importing `detect_nikaya_structure` for Phase 2.
2. **Exclusions**: All XML files in `romn/` are sutta content — no files need to be excluded.
3. **Group metadata for Phase 2**: Leave `group_path`, `group_index`, and `order_index` as `None` for single-record imports.
