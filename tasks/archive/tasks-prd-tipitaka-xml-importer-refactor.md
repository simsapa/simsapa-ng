# Tasks: TipitakaXmlImporter Refactor

## Relevant Files

- `cli/src/bootstrap/tipitaka_xml.rs` — Main file being rewritten. Contains `TipitakaXmlImporter` struct, `SuttaRecord`, and all import logic.
- `cli/src/bootstrap/mod.rs` — Calls `TipitakaXmlImporter::new()` and `.import()`. No changes expected but verify compatibility.
- `cli/src/bootstrap/helpers.rs` — Contains `uid_to_ref()` used for sutta_ref generation.
- `cli/Cargo.toml` — Dependency on `tipitaka_xml_parser` (local path). May need feature or re-export adjustments.

### External Dependencies (read-only reference)

- `tipitaka-xml-parser/src/lib.rs` — Public API exports from the parser library.
- `tipitaka-xml-parser/src/fragments_schema.rs` — Diesel schema for `xml_fragments` table.
- `tipitaka-xml-parser/src/fragments_models.rs` — `XmlFragmentRecord` queryable model.
- `tipitaka-xml-parser/src/sutta_builder.rs` — `xml_to_html()` function.
- `tipitaka-xml-parser/src/encoding.rs` — `read_xml_file()` function.
- `tipitaka-xml-parser/src/types.rs` — `GroupLevel`, `GroupType` types.

### Notes

- Run `make build -B` to verify compilation after each task.
- Run `cd backend && cargo test` to verify backend tests.
- The `fragments.sqlite3` database is at `bootstrap-assets/tipitaka-xml-data/fragments.sqlite3` relative to the bootstrap assets directory.
- The `romn/` directory is at `bootstrap-assets/tipitaka-org-vri-cst/tipitaka-xml/romn/`.

## Tasks

- [x] 1.0 Rewrite `TipitakaXmlImporter` struct, constructor, and imports
  - [x] 1.1 Remove all old imports (`FragmentAdjustments`, `load_fragment_adjustments`, `parse_into_fragments`, `FileImportStats`, `NikayaStructure`, `XmlFragment`, `FragmentType`, `GroupType`).
  - [x] 1.2 Add new imports: `tipitaka_xml_parser::encoding::read_xml_file`, `tipitaka_xml_parser::detect_nikaya_structure`, `tipitaka_xml_parser::sutta_builder::xml_to_html`, `tipitaka_xml_parser::fragments_schema::xml_fragments`, `tipitaka_xml_parser::fragments_models::XmlFragmentRecord`, `tipitaka_xml_parser::types::{GroupLevel, GroupType}`.
  - [x] 1.3 Rewrite `TipitakaXmlImporter` struct to hold `fragments_db_path: PathBuf` and `romn_dir: PathBuf`.
  - [x] 1.4 Rewrite `new(fragments_db_path: PathBuf, romn_dir: PathBuf) -> Self` constructor.
  - [x] 1.5 Keep `SuttaRecord` struct unchanged.
  - [x] 1.6 Remove `with_verbose()`, `process_file()`, and the standalone `build_suttas()` function.

- [x] 2.0 Implement Phase 1: Fragment-based import from `fragments.sqlite3` for whitelisted files
  - [x] 2.1 Define the hardcoded whitelist of checked filenames (same files as current `let files = [...]` array).
  - [x] 2.2 Open a read-only Diesel `SqliteConnection` to `fragments.sqlite3`. Use `try_exists()` to check the path first (Android safety).
  - [x] 2.3 For each whitelisted filename, query `xml_fragments` table where `cst_file = filename` and `frag_type = "Sutta"`, returning `XmlFragmentRecord` rows.
  - [x] 2.4 For each fragment row, build a `SuttaRecord`:
    - Skip if `sc_code` is `None` (log warning).
    - Determine commentary suffix from `cst_file`: `.att.xml` → `.att`, `.tik.xml` → `.tik`, else none.
    - Build UID: `<sc_code>[.att|.tik]/pli/cst4`.
    - Title: `cst_sutta` field, normalized with `consistent_niggahita()`.
    - Content HTML: Convert `content_xml` via `xml_to_html()`, wrap in `<div class="cst4">` with header.
    - Content plain: Derive via `sutta_html_to_plain_text()`.
    - Group path: Deserialize `group_levels` JSON into `Vec<GroupLevel>`, filter out `GroupType::Nikaya` and `GroupType::Sutta`, join titles with ` / `.
    - Nikaya: Use `nikaya` field from fragment row.
    - `sutta_ref`: Use `uid_to_ref()` on the sc_code.
    - `source_uid`: `"cst4"`.
    - `group_index` / `order_index`: Use enumeration index.
  - [x] 2.5 Track used UIDs in a `HashSet` and skip duplicates (log warning).

- [x] 3.0 Implement Phase 2: Direct XML import for remaining `romn/` files
  - [x] 3.1 List all `*.xml` files in the `romn/` directory. Use `try_exists()` to check directory first.
  - [x] 3.2 Filter out files that are in the whitelist (already imported in Phase 1).
  - [x] 3.3 For each remaining XML file:
    - Read content using `read_xml_file()`.
    - Detect nikaya using `detect_nikaya_structure()` on the XML content.
    - Convert full XML content to HTML using `xml_to_html()`.
    - Build UID: `<xml_filename>/pli/cst4` (e.g., `s0401m.mul.xml/pli/cst4`).
    - Title: filename without `.xml` extension (e.g., `s0401m.mul`).
    - Wrap HTML in `<div class="cst4">` with header using filename as title.
    - Content plain: Derive via `sutta_html_to_plain_text()`.
    - Nikaya: Use `nikaya_structure.nikaya` from `detect_nikaya_structure()`.
    - `sutta_ref`: Use `uid_to_ref()` on the filename stem.
    - `group_path`, `group_index`, `order_index`: `None`.
    - `source_uid`: `"cst4"`.
  - [x] 3.4 Skip duplicate UIDs (log warning), continue on errors per file (log error).

- [x] 4.0 Implement shared database insertion logic
  - [x] 4.1 Write `insert_suttas()` method on `TipitakaXmlImporter` that takes `Vec<SuttaRecord>` and `&mut SqliteConnection`.
  - [x] 4.2 Use Diesel transaction for batch insertion (same pattern as current `insert_suttas_with_conn`).
  - [x] 4.3 Check for existing UIDs in appdata before inserting, skip duplicates.
  - [x] 4.4 Log counts: total processed, inserted, skipped.

- [x] 5.0 Wire up `import()` entry point combining both phases
  - [x] 5.1 Implement `import(&mut self, conn: &mut SqliteConnection) -> Result<()>`.
  - [x] 5.2 Run Phase 1 (fragment-based import), collect `Vec<SuttaRecord>`, insert via `insert_suttas()`.
  - [x] 5.3 Run Phase 2 (direct XML import), collect `Vec<SuttaRecord>`, insert via `insert_suttas()`.
  - [x] 5.4 Log summary of total imports from both phases.

- [x] 6.0 Verify compilation and tests pass
  - [x] 6.1 Run `make build -B` and fix any compilation errors.
  - [x] 6.2 Run `cd backend && cargo test` and verify all tests pass (pre-existing failures in test_search.rs and test_database_comparison.rs are unrelated).
  - [x] 6.3 Verify `mod.rs` call site (`TipitakaXmlImporter::new(fragments_db_path, romn_dir)`) is compatible with the new constructor signature.
