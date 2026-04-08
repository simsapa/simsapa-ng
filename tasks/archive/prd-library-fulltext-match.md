# PRD: FulltextMatch Search for Library Book Chapters

## 1. Introduction/Overview

Library books (EPUBs, PDFs, HTML documents) currently support **Contains Match** (FTS5) and **Title Match** search modes but lack **Fulltext Match** (Tantivy) support. Fulltext Match provides superior search quality through language-aware stemming, relevance scoring, and snippet generation — features already available for Suttas and Dictionary searches.

This feature adds Tantivy-based fulltext indexing and searching for library book chapters (`book_spine_items`), bringing Library search to feature parity with the other search areas.

## 2. Goals

- Enable Fulltext Match search mode for library book chapters using Tantivy indexes.
- Index book chapters with the correct language-specific stemmer (spine item language > book language > English default).
- Allow users to change a book's language via the Edit Metadata dialog, triggering re-indexing.
- Automatically index newly imported books during the import process.
- Extend the CLI `index` command to support `--area library`.
- Support language filtering in library fulltext search results.

## 3. User Stories

- **As a user**, I want to search library books using Fulltext Match so that I get relevance-ranked results with stemming support (e.g., searching "meditation" also matches "meditations", "meditative").
- **As a user**, I want to set a book's language in Edit Metadata so that the correct stemmer is used for indexing.
- **As a user**, I want newly imported books to be immediately searchable via Fulltext Match without manual steps.
- **As a user**, I want to filter library fulltext results by language using the language dropdown.
- **As a CLI user**, I want to rebuild library indexes with `simsapa index build --area library`.

## 4. Functional Requirements

### 4.1 Tantivy Schema for Library

1. Create `build_library_schema(lang)` in `backend/src/search/schema.rs` with the following fields:
   - `spine_item_uid` — unique ID of the chapter (simple_fold tokenizer, stored)
   - `book_uid` — parent book UID (raw tokenizer, stored)
   - `book_title` — title of the parent book (simple_fold tokenizer, stored)
   - `author` — author from the parent book (simple_fold tokenizer, stored)
   - `title` — chapter title (simple_fold tokenizer, stored)
   - `language` — language code (raw tokenizer, stored)
   - `content` — chapter plain text (lang_stem tokenizer, stored) — for stemmed matching
   - `content_exact` — chapter plain text (lang_normalize tokenizer, stored) — for exact matching with boost

### 4.2 Index Building

2. Create `build_library_index(appdata_db, index_dir, lang)` in `backend/src/search/indexer.rs` following the same pattern as `build_sutta_index()`:
   - Query `book_spine_items` joined with `books` to get author and book title.
   - Determine the effective language for each spine item: use `book_spine_items.language` if present and non-empty, else `books.language`, else `"en"`.
   - Only index spine items whose effective language matches the `lang` parameter.
   - Prepend book_title, chapter title, and author to the content field for better matching.
   - Skip spine items with empty `content_plain`.

3. Create `get_library_languages(appdata_db)` in `backend/src/search/indexer.rs` that returns distinct effective languages across all book spine items (using the fallback chain: spine item language > book language > "en").

4. Add `library_index_dir: PathBuf` to `AppGlobalPaths` (path: `index/library/{lang}/`).

5. Update `build_all_indexes()` to also build library indexes for all library languages.

### 4.3 Searcher Integration

6. Add `library_indexes: HashMap<String, (Index, IndexReader)>` to `FulltextSearcher`.

7. Add `open_library_indexes()` call in `FulltextSearcher::open()`.

8. Add `search_library_with_count()` method mirroring `search_suttas_with_count()`.

9. Add `library_doc_to_result()` method that maps Tantivy document fields to `SearchResult`:
   - `uid` = `spine_item_uid`
   - `table_name` = `"book_spine_items"`
   - `schema_name` = `"appdata"`
   - `title` = chapter title
   - `author` = book author
   - `snippet` = generated snippet from content field

### 4.4 Query Routing

10. In `query_task.rs`, replace the `SearchArea::Library` arm under `SearchMode::FulltextMatch` (currently returns empty) with a call to the fulltext searcher's `search_library_with_count()`.

11. Apply language filtering from the language filter dropdown, same as for Suttas.

### 4.5 Edit Metadata Dialog — Language Field

12. Add a language input field to `DocumentMetadataEditDialog.qml` between the Author and Embedded CSS fields. This should be a `TextField` for the language code (e.g., "en", "pli", "de").

13. Load the current book language in `load_metadata()` from the metadata JSON.

14. Pass the language value to `SuttaBridge.update_book_metadata()` — update the bridge function signature to accept a `language` parameter.

15. Update `appdata.rs::update_book_metadata()` to also update the `books.language` column.

### 4.6 Re-indexing on Language Change

16. When `update_book_metadata()` detects a language change, re-index only the affected book's chapters:
    - Delete existing Tantivy documents for that `book_uid` from the old language index.
    - Add the chapters to the new language index with the correct stemmer.
    - If the old language index becomes empty after removal, it can remain (no cleanup needed).

17. Reload the `FulltextSearcher`'s library indexes after re-indexing so new results are immediately available.

### 4.7 Indexing on Import

18. After a book is successfully imported (EPUB/PDF/HTML), immediately index its chapters into the appropriate Tantivy language index as part of the same import operation.

19. If the imported book has no language metadata, default to `"en"`.

### 4.8 CLI Index Command

20. Add `Library` variant to `IndexArea` enum in `cli/src/main.rs`.

21. Handle `IndexArea::Library` in the `index build` and `index rebuild` subcommands, calling `build_library_index()` for the specified language or all library languages.

### 4.9 Language Filter Dropdown — Search Area Switching

23. When the user switches search area to **Library** (L button), repopulate the language filter dropdown with distinct languages from library book spine items (using the same fallback chain: spine item language > book language > "en").

24. When the user switches back to **Suttas** (S button), repopulate the language filter dropdown with sutta languages (current behavior).

25. Add a backend function `get_library_language_labels()` (on `SuttaBridge`) that returns distinct language labels from book spine items, analogous to `get_sutta_language_labels()`.

26. In `SearchBarInput.qml`, react to `search_area` changes to swap the language dropdown model between sutta and library language lists. Dictionary search area should keep language filter disabled (current behavior).

### 4.10 Language Field on Import Dialog

27. Add a `language` TextField to `DocumentImportDialog.qml` (e.g., between Author and UID fields) with placeholder "en" and a default value of "en".

28. Pass the language value to `SuttaBridge.import_document()` — update the bridge function signature to accept a `language` parameter.

29. Store the language on the `books.language` column during import, and propagate it to `book_spine_items.language` for chapters that don't have their own language metadata (e.g., from EPUB metadata).

### 4.11 Default Language Backfill

22. Existing books in the database with NULL or empty `language` should be treated as `"en"` during indexing (no schema migration needed — the fallback is applied at index-build time).

## 5. Non-Goals (Out of Scope)

- Per-chapter language editing (only book-level language is editable; the spine item language is set during import).
- Multilingual book support where different chapters have different stemmers (the fallback chain handles this naturally, but there is no UI to set per-chapter languages).
- RegEx Match for library.
- Full re-architecture of the FTS5 Contains Match search for library (it continues to work as-is).
- Adding new language stemmers beyond what Tantivy already supports.

## 6. Design Considerations

- The language field in Edit Metadata should be a simple text input with a label like "Language:" and placeholder "en". It sits between the Author field and the Embedded CSS checkbox.
- No changes to the SearchBarInput.qml search mode dropdown models — "Fulltext Match" is already listed for Library search area.
- The language filter dropdown dynamically swaps its model when the search area changes between Suttas and Library.

## 7. Technical Considerations

- **Index directory**: `{app_assets_dir}/index/library/{lang}/` — one subdirectory per language, same as suttas and dict_words.
- **Schema module**: Add `build_library_schema()` alongside existing `build_sutta_schema()` and `build_dict_schema()` in `backend/src/search/schema.rs`.
- **Searcher enum dispatch**: The `search_indexes()` method currently uses an `is_sutta: bool` flag to choose between sutta and dict schemas. This needs extending to handle library (e.g., an enum parameter or a third method).
- **Re-index granularity**: Deleting and re-adding individual book documents in Tantivy requires searching by `book_uid` term and deleting matching docs. Tantivy supports `IndexWriter::delete_term()` for this.
- **Import flow**: The import functions (`import_epub_to_db`, `import_pdf_to_db`, `import_html_to_db`) return after writing to the database. The indexing step should happen after the DB write succeeds, using the same thread/async context.
- **FulltextSearcher reload**: After re-indexing a single book, the `IndexReader` needs to be reloaded (or the searcher re-opened) to pick up changes. Tantivy's `IndexReader` auto-reloads on search, but if a new language subdirectory was created, the searcher must re-scan.

## 8. Success Metrics

- Fulltext Match search in Library returns relevant, ranked results with highlighted snippets.
- Searching with stemmed terms (e.g., "meditations" matching "meditation") works correctly for the book's language.
- Language filter dropdown filters library fulltext results.
- Changing a book's language in Edit Metadata triggers re-indexing and subsequent searches use the correct stemmer.
- Newly imported books are immediately searchable via Fulltext Match.
- CLI `simsapa index build --area library` builds library indexes successfully.

## 9. Open Questions

None — all resolved.
