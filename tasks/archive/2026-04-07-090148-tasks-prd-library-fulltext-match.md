# Tasks: Library FulltextMatch Search

## Relevant Files

- `backend/src/search/schema.rs` — Add `build_library_schema()` for library Tantivy schema
- `backend/src/search/indexer.rs` — Add `build_library_index()`, `get_library_languages()`, update `build_all_indexes()`
- `backend/src/search/searcher.rs` — Add `library_indexes` to `FulltextSearcher`, `search_library_with_count()`, `library_doc_to_result()`
- `backend/src/lib.rs` — Add `library_index_dir` to `AppGlobalPaths`
- `backend/src/query_task.rs` — Wire `fulltext_library()` into `SearchMode::FulltextMatch` / `SearchArea::Library`
- `cli/src/main.rs` — Add `Library` to `IndexArea` enum, handle in `index build`/`rebuild`
- `cli/src/bootstrap/mod.rs` — Build library indexes during bootstrap before `create_index_archive()`
- `bridges/src/sutta_bridge.rs` — Add `get_library_language_labels()`, update `update_book_metadata()` and `import_document()` signatures, add `get_book_metadata_json()` language field
- `backend/src/db/appdata.rs` — Update `update_book_metadata()` to accept and persist language
- `assets/qml/SearchBarInput.qml` — Swap language dropdown model on search area change
- `assets/qml/DocumentMetadataEditDialog.qml` — Add language input field
- `assets/qml/DocumentImportDialog.qml` — Add language input field
- `assets/qml/com/profoundlabs/simsapa/SuttaBridge.qml` — Add qmllint stubs for new/updated bridge functions

### Notes

- Run `cd backend && cargo test` for Rust unit tests.
- Run `make build -B` to verify compilation.
- Don't run `make qml-test` unless explicitly asked.
- The import functions (`import_epub_to_db`, `import_pdf_to_db`, `import_html_to_db`) already accept `custom_language: Option<&str>` — the bridge just needs to pass it through instead of `None`.
- The `create_index_archive()` tars the entire `index/` directory, so `index/library/{lang}/` will be included automatically once built.

## Tasks

- [x] 1.0 Tantivy schema and index builder for library book chapters
  - [x] 1.1 Add `library_index_dir: PathBuf` to `AppGlobalPaths` struct in `backend/src/lib.rs` (line ~255), initialize it as `index_dir.join("library")` in the constructor (line ~398), and add it to the struct literal (line ~429).
  - [x] 1.2 Create `build_library_schema(lang: &str) -> Schema` in `backend/src/search/schema.rs`. Fields: `spine_item_uid` (simple_fold, stored), `book_uid` (raw, stored), `book_title` (simple_fold, stored), `author` (simple_fold, stored), `title` (simple_fold, stored), `language` (raw, stored), `content` (lang_stem, stored), `content_exact` (lang_normalize, stored).
  - [x] 1.3 Create `build_library_index(appdata_db: &DatabaseHandle, index_dir: &Path, lang: &str) -> Result<()>` in `backend/src/search/indexer.rs`. Query `book_spine_items` joined with `books` to get `author` and `book.title`. Determine effective language per spine item: `spine_item.language` if non-empty, else `book.language` if non-empty, else `"en"`. Only index items whose effective language matches `lang`. Prepend book_title, chapter title, and author to content. Skip items with empty `content_plain`.
  - [x] 1.4 Create `get_library_languages(appdata_db: &DatabaseHandle) -> Result<Vec<String>>` in `backend/src/search/indexer.rs`. Return distinct effective languages across all book spine items using the fallback chain (spine item language > book language > "en").
  - [x] 1.5 Update `build_all_indexes()` in `backend/src/search/indexer.rs` to also call `build_library_index()` for each language returned by `get_library_languages()`, using `paths.library_index_dir`.
  - [x] 1.6 Verify with `cd backend && cargo test` that existing tests still pass and the new code compiles.

- [x] 2.0 Searcher integration for library fulltext search
  - [x] 2.1 Add `library_indexes: HashMap<String, (Index, IndexReader)>` field to `FulltextSearcher` struct in `backend/src/search/searcher.rs` (line ~23).
  - [x] 2.2 Update `FulltextSearcher::open()` to also open library indexes from `paths.library_index_dir`. Use a new schema selector — refactor `open_single_index()` to handle sutta/dict/library schema selection (e.g., use an enum or a third `bool`/variant). Register tokenizers for library indexes.
  - [x] 2.3 Update `FulltextSearcher::open_from_dirs()` to accept an optional library index dir parameter (or add a separate constructor method).
  - [x] 2.4 Add `has_library_indexes(&self) -> bool` method.
  - [x] 2.5 Add `search_library_with_count()` method mirroring `search_suttas_with_count()`, calling `search_indexes()` with `library_indexes`. The `search_indexes` method uses `is_sutta: bool` to dispatch — refactor to support a third index type (e.g., add an enum `IndexType { Sutta, Dict, Library }` to replace the `is_sutta` bool).
  - [x] 2.6 Add `library_doc_to_result()` method mapping Tantivy doc fields to `SearchResult`: `uid` = `spine_item_uid`, `table_name` = `"book_spine_items"`, `schema_name` = `"appdata"`, `title` = chapter title, `author` = book author, `lang` = language, `snippet` = generated snippet.
  - [x] 2.7 Add `fulltext_library()` method to `QueryTask` in `backend/src/query_task.rs`, following the same pattern as `fulltext_suttas()` (line ~1781): build `SearchFilters`, call `with_fulltext_searcher(|s| s.search_library_with_count(...))`.
  - [x] 2.8 Wire `fulltext_library()` into the `SearchMode::FulltextMatch` / `SearchArea::Library` match arm in `query_task.rs` (line ~1736), replacing the empty result fallback.
  - [x] 2.9 Verify with `cd backend && cargo test` and `make build -B`.

- [x] 3.0 Bootstrap and CLI — build library indexes and include in index.tar.bz2
  - [x] 3.1 Add `Library` variant to `IndexArea` enum in `cli/src/main.rs` (line ~1163).
  - [x] 3.2 Handle `IndexArea::Library` in the `index build` subcommand match arms (line ~805): for `(Some(IndexArea::Library), None)` build all library languages, for `(Some(IndexArea::Library), Some(lang))` build a single language.
  - [x] 3.3 Handle `IndexArea::Library` in the `index rebuild` subcommand similarly (delete library index dir contents, then rebuild).
  - [x] 3.4 Add library index building to the bootstrap process in `cli/src/bootstrap/mod.rs` (line ~333, after dict indexes and before `write_version_file`): call `get_library_languages()` and `build_library_index()` for each language. The resulting `index/library/{lang}/` directories will be included in `index.tar.bz2` automatically by `create_index_archive()`.
  - [x] 3.5 Verify with `make build -B`.

- [x] 4.0 Language filter dropdown — dynamic population by search area
  - [x] 4.1 Add `get_library_language_labels()` method to `SuttaBridge` in `bridges/src/sutta_bridge.rs` that returns a `QStringList` of distinct library languages (calling a new backend method or reusing `get_library_languages()` from the indexer).
  - [x] 4.2 Add the corresponding qmllint stub in `assets/qml/com/profoundlabs/simsapa/SuttaBridge.qml`: `function get_library_language_labels(): list<string> { return []; }`.
  - [x] 4.3 In `SearchBarInput.qml`, add an `onSearch_areaChanged` handler (or use a binding on `search_area`) that repopulates `language_filter_dropdown.model`: when `search_area === "Suttas"`, load sutta language labels; when `search_area === "Library"`, load library language labels; when `search_area === "Dictionary"`, keep current behavior (disabled).
  - [x] 4.4 Reset `language_filter_dropdown.currentIndex` to 0 when swapping the model to avoid stale index references.
  - [x] 4.5 Verify with `make build -B`.

- [x] 5.0 Edit Metadata dialog — language field and re-indexing
  - [x] 5.1 Add a `language` TextField to `DocumentMetadataEditDialog.qml` between the Author and Embedded CSS rows. Label: "Language:", placeholder: "en".
  - [x] 5.2 Update `load_metadata()` in the dialog to populate the language field from metadata JSON.
  - [x] 5.3 Update `get_book_metadata_json()` in `bridges/src/sutta_bridge.rs` (line ~2393) to include the `language` field in the returned JSON.
  - [x] 5.4 Update the Save button's `onClicked` handler to pass `language_field.text.trim()` to `SuttaBridge.update_book_metadata()`.
  - [x] 5.5 Update `update_book_metadata()` bridge function signature in `bridges/src/sutta_bridge.rs` (line ~2430) to accept a `language: &QString` parameter. Pass it through to the backend.
  - [x] 5.6 Update `appdata.rs::update_book_metadata()` (line ~747) to also update the `books.language` column.
  - [x] 5.7 After successful metadata update in the bridge, if language changed: delete the book's chapters from the old language's Tantivy index (using `IndexWriter::delete_term()` on the `book_uid` field), then re-index them into the new language index using `build_library_index` or a targeted per-book indexing helper. Call `reinit_fulltext_searcher()` to reload indexes.
  - [x] 5.8 Update the qmllint stub for `update_book_metadata()` in `SuttaBridge.qml` to match the new signature.
  - [x] 5.9 Verify with `make build -B`.

- [x] 6.0 Import Document — language field and auto-indexing
  - [x] 6.1 Add a `language` TextField to `DocumentImportDialog.qml` between the Author and UID fields. Label: "Language:", placeholder: "en", default text: "en".
  - [x] 6.2 Pass `language_field.text.trim()` to `SuttaBridge.import_document()` in the dialog's import call (line ~304).
  - [x] 6.3 Update `import_document()` bridge function signature in `bridges/src/sutta_bridge.rs` (line ~2201) to accept a `language: &QString` parameter. Convert to `Option<&str>` and pass as `custom_language` to `import_epub_to_db()` / `import_pdf_to_db()` / `import_html_to_db()` (replacing the current `None`).
  - [x] 6.4 After successful import in the bridge thread, build the Tantivy index for the imported book's chapters: determine the effective language, then call `build_library_index()` for that language (or a targeted single-book indexing helper). Call `reinit_fulltext_searcher()` to reload indexes.
  - [x] 6.5 Update the qmllint stub for `import_document()` in `SuttaBridge.qml` to match the new signature.
  - [x] 6.6 Verify with `make build -B`.
