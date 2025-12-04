# Tasks: Document Library Import and Reading (Epub, PDF, HTML)

## Relevant Files

### Backend Database
- `backend/src/db/appdata_schema.rs` - Database schema definitions using Diesel ORM (UPDATED: added language fields)
- `backend/src/db/appdata_models.rs` - Rust model structs for database tables (UPDATED: added language fields to Book and BookSpineItem)
- `backend/src/db/appdata.rs` - Database connection and query functions
- `backend/migrations/appdata/2025-12-04-130316_create_books_tables/up.sql` - Migration to create books tables (UPDATED: added language columns and indexes)
- `backend/migrations/appdata/2025-12-04-130316_create_books_tables/down.sql` - Migration rollback
- `scripts/books-fts5-indexes.sql` - FTS5 fulltext search index creation for book_spine_items (UPDATED: language as UNINDEXED field for filtering)

### Backend Import Logic
- `backend/src/document_import.rs` - Core document import functionality for all formats (future)
- `backend/src/epub_import.rs` - Epub-specific parsing and import logic (COMPLETED - extracts language from metadata, stores in book and spine items)
- `backend/src/pdf_import.rs` - PDF-specific parsing and import logic (COMPLETED - extracts metadata with lopdf, text with pdf-extract, embeds PDF)
- `backend/src/html_import.rs` - HTML-specific parsing and import logic (future)
- `backend/Cargo.toml` - Added dependencies: epub = "=2.1.5", pdf-extract = "0.7", lopdf = "0.34"
- `backend/src/lib.rs` - Added epub_import and pdf_import module exports

### Backend Queries and Helpers
- `backend/src/app_data.rs` - Add book-related query methods (get_book_spine_item, get_book_resource)
- `backend/src/query_task.rs` - Extend search to support Library area
- `backend/src/types.rs` - Add Library variant to SearchArea enum

### Bridges
- `bridges/src/sutta_bridge.rs` - Add get_book_spine_html function
- `bridges/src/api.rs` - Add /book_resources/<book_uid>/<resource_path> endpoint
- `bridges/src/library_bridge.rs` - New bridge for library-specific functions (optional)
- `bridges/build.rs` - Register new QML files and bridge modules

### QML Components
- `assets/qml/LibraryWindow.qml` - Main library browsing window
- `assets/qml/DocumentImportDialog.qml` - Import dialog with metadata fields
- `assets/qml/SearchBarInput.qml` - Add "Library" option to search_area_dropdown
- `assets/qml/com/profoundlabs/simsapa/LibraryBridge.qml` - Type definition for qmllint (if LibraryBridge created)
- `assets/qml/com/profoundlabs/simsapa/qmldir` - Update with new QML types

### C++ Layer
- `cpp/library_window.cpp` - C++ window management for library (if needed)
- `cpp/library_window.h` - Header file
- `cpp/window_manager.cpp` - Add library window to window manager

### CLI Module
- `cli/src/main.rs` - Added ImportEpub subcommand with database migration (COMPLETED)
- `cli/src/bootstrap/appdata.rs` - Bootstrap appdata database (UPDATED: runs books-fts5-indexes.sql after appdata-fts5-indexes.sql)
- `cli/src/bootstrap/import_documents.rs` - Document import logic for CLI (future)

### Tests
- `backend/tests/test_document_import.rs` - Comprehensive tests for all import formats
- `backend/tests/test_epub_import.rs` - Epub-specific import tests
- `backend/tests/test_pdf_import.rs` - PDF-specific import tests
- `backend/tests/test_html_import.rs` - HTML-specific import tests
- `backend/tests/data/its-essential-meaning.epub` - Epub test data (existing)
- `backend/tests/data/pali-lessons.pdf` - PDF test data (existing)
- `backend/tests/data/sample.html` - HTML test data (to be created)

### Assets
- `assets/js/vendor/embedpdf.js` - Embedded PDF viewer library for rendering PDFs

### Notes

- Unit tests should be placed in `backend/tests/` directory
- Use `cd backend && cargo test` to run all backend tests
- Use `cd backend && cargo test test_epub_import` to run specific test
- QML tests can be run with `make qml-test`
- Build with `make build -B` to verify compilation

## Tasks

- [x] 1.0 Create database schema and migrations for books, spine items, and resources
  - [x] 1.1 Create new migration directory: `backend/migrations/appdata/2025-12-04-130316_create_books_tables/`
  - [x] 1.2 Write `up.sql` to create `books` table with fields: id, uid, document_type, title, author, file_path, metadata_json
  - [x] 1.3 Write `up.sql` to create `book_spine_items` table with fields: id, book_id, book_uid, spine_item_uid, spine_index, title, content_html, content_plain
  - [x] 1.4 Write `up.sql` to create `book_resources` table with fields: id, book_id, book_uid, resource_path, mime_type, content_data (Blob)
  - [x] 1.5 Add foreign key constraints with cascade delete: book_spine_items.book_id → books.id, book_resources.book_id → books.id
  - [x] 1.6 Add indexes on foreign keys and frequently queried fields (book_uid, spine_item_uid)
  - [x] 1.7 Write `down.sql` to drop all three tables in reverse order
  - [x] 1.8 Add FTS5 virtual table creation for `book_spine_items_fts` in `scripts/books-fts5-indexes.sql` following pattern from `suttas_fts`
  - [x] 1.9 Add triggers for INSERT, UPDATE, DELETE to keep FTS5 table synchronized with book_spine_items
  - [x] 1.10 Define Diesel models in `backend/src/db/appdata_models.rs`: Book, BookSpineItem, BookResource structs
  - [x] 1.11 Update `backend/src/db/appdata_schema.rs` with table definitions and joinable relationships
  - [x] 1.12 Run migration and verify tables are created: migrations run automatically on connection, backend compiles successfully

- [x] 2.0 Implement Epub parsing and import functionality
  - [x] 2.1 Add `epub = "=2.1.5"` dependency to `backend/Cargo.toml`
  - [x] 2.2 Create `backend/src/epub_import.rs` module with full implementation
  - [x] 2.3 Fixed epub crate API usage to match version 2.1.5 (metadata is Vec<MetadataItem>, resources use ResourceItem struct)
  - [x] 2.4 Extract metadata (title, author) from epub using MetadataItem.value
  - [x] 2.5 Extract TOC (table of contents) and match chapter titles to spine items using correct API
  - [x] 2.6 Iterate through spine items and extract HTML content for each chapter using ResourceItem.path
  - [x] 2.7 Convert HTML content to plain text using existing plain text extraction logic (reuse from sutta imports)
  - [x] 2.8 Generate spine_item_uid in format "<book_uid>.<spine_index>" (e.g., "ess.0", "ess.1", "ess.2")
  - [x] 2.9 Extract all resources (images, CSS, fonts) from epub as binary data using ResourceItem struct
  - [x] 2.10 Rewrite resource links in chapter HTML to use API endpoint format: `/book_resources/<book_uid>/<resource_path>`
  - [x] 2.11 Insert book record into `books` table
  - [x] 2.12 Insert all spine items into `book_spine_items` table with book_id, spine_index, titles, content_html, content_plain
  - [x] 2.13 Insert all resources into `book_resources` table with book_id, resource_path, mime_type, binary content_data
  - [x] 2.14 Add error handling for corrupted files, missing metadata, and parse errors
  - [x] 2.15 Add function `import_epub_to_db(db_conn: &mut SqliteConnection, epub_path: &Path, book_uid: &str) -> Result<()>`

- [x] 3.0 Add CLI command for migrating existing database and testing epub import
  - [x] 3.1 Add clap dependency to `cli/Cargo.toml` (already present)
  - [x] 3.2 Update `cli/src/main.rs` to add subcommand `import-epub` with arguments: `--db-path` and `--epub-path` and `--uid`
  - [x] 3.3 Implement command handler that connects to the specified appdata database
  - [x] 3.4 Run pending migrations ONLY if 'books' or 'book_spine_items_fts' table doesn't exist
  - [x] 3.5 Call `import_epub_to_db()` with the provided epub file path and book UID
  - [x] 3.6 Run `run_fts5_indexes_sql_script()` to create FTS5 indexes from books-fts5-indexes.sql after migrations
  - [x] 3.7 Print success message with imported book details or error message on failure
  - [x] 3.8 Test with command: successfully imported "Its Essential Meaning" epub - 24 spine items, 17 resources
  - [x] 3.9 Verified database contains: book record with metadata JSON, 24 spine items (ess.0-ess.23), 17 resources (images, fonts), FTS5 indexes created
  - [x] 3.10 Tested with existing database: migrations skipped correctly, second book imported successfully

- [x] 4.0 Implement PDF parsing and import functionality
  - [x] 4.1 Add `pdf-extract` and `lopdf` dependencies to `backend/Cargo.toml`
  - [x] 4.2 Create `backend/src/pdf_import.rs` module
  - [x] 4.3 Implement function to extract plain text from PDF using pdf-extract crate
  - [x] 4.4 Implement function to extract metadata (title, author, language) using lopdf crate
  - [x] 4.5 Read PDF file into memory as binary data
  - [x] 4.6 Generate HTML content that embeds PDF using embed tag, pointing to `/book_resources/<book_uid>/document.pdf`
  - [x] 4.7 Insert book record into `books` table with document_type="pdf"
  - [x] 4.8 Insert single spine item with spine_index=0, spine_item_uid="<book_uid>.0", content_html (embed HTML), content_plain (extracted text)
  - [x] 4.9 Insert PDF binary data into `book_resources` table with resource_path="document.pdf", mime_type="application/pdf"
  - [x] 4.10 Add error handling for corrupted PDFs and extraction failures
  - [x] 4.11 Add function `import_pdf_to_db(db_conn: &mut SqliteConnection, pdf_path: &Path, book_uid: &str) -> Result<()>`

- [ ] 5.0 Implement HTML parsing and import functionality
  - [ ] 5.1 Create `backend/src/html_import.rs` module
  - [ ] 5.2 Add function to parse HTML and extract metadata from <title> tag and meta tags (author, description)
  - [ ] 5.3 Implement function to split HTML by specified tag (h1, h2, h3, h4, h5, h6, or custom tag)
  - [ ] 5.4 For each split section, use tag content as chapter title
  - [ ] 5.5 Extract and parse embedded resources (images, CSS) from HTML
  - [ ] 5.6 Convert each section's HTML to plain text for FTS5 indexing
  - [ ] 5.7 Generate spine_item_uid for each section: "<book_uid>.<spine_index>"
  - [ ] 5.8 Rewrite resource links to use API endpoint format: `/book_resources/<book_uid>/<resource_path>`
  - [ ] 5.9 Insert book record into `books` table with document_type="html"
  - [ ] 5.10 Insert spine items (single item if no splitting, multiple if split by tag)
  - [ ] 5.11 Insert resources into `book_resources` table
  - [ ] 5.12 Handle case where split tag is not found: warn user and import as single spine item
  - [ ] 5.13 Add function `import_html_to_db(db_conn: &mut SqliteConnection, html_path: &Path, book_uid: &str, split_tag: Option<&str>) -> Result<()>`

- [ ] 6.0 Implement backend helper functions and database queries
  - [ ] 6.1 Add `get_book_by_uid(book_uid: &str) -> Result<Option<Book>>` to `backend/src/db/appdata.rs`
  - [ ] 6.2 Add `get_book_spine_item(spine_item_uid: &str) -> Result<Option<BookSpineItem>>` to `backend/src/db/appdata.rs`
  - [ ] 6.3 Add `get_book_resource(book_uid: &str, resource_path: &str) -> Result<Option<BookResource>>` to `backend/src/db/appdata.rs`
  - [ ] 6.4 Add `get_all_books() -> Result<Vec<Book>>` to `backend/src/db/appdata.rs`
  - [ ] 6.5 Add `get_spine_items_for_book(book_uid: &str) -> Result<Vec<BookSpineItem>>` to `backend/src/db/appdata.rs`
  - [ ] 6.6 Add `delete_book_by_uid(book_uid: &str) -> Result<()>` to `backend/src/db/appdata.rs` (relies on cascade delete for spine items and resources)
  - [ ] 6.7 Implement `get_book_spine_html()` in `backend/src/app_data.rs` similar to existing sutta rendering logic
  - [ ] 6.8 Reuse existing `sutta_html_page()` template function or create `book_spine_html_page()` if customization needed

- [ ] 7.0 Create API endpoint for book resource serving
  - [ ] 7.1 Add route handler in `bridges/src/api.rs` for `GET /book_resources/<book_uid>/<resource_path>`
  - [ ] 7.2 Parse book_uid and resource_path from URL parameters
  - [ ] 7.3 Query database using `get_book_resource(book_uid, resource_path)`
  - [ ] 7.4 Return resource binary data with appropriate MIME type headers (image/png, text/css, application/pdf, etc.)
  - [ ] 7.5 Handle missing resources gracefully with 404 response
  - [ ] 7.6 Test endpoint by manually requesting a resource URL in browser after importing a book

- [ ] 8.0 Create LibraryWindow QML component and UI
  - [ ] 8.1 Create `assets/qml/LibraryWindow.qml` as ApplicationWindow following pattern of `SuttaLanguagesWindow.qml`
  - [ ] 8.2 Add "Import Document..." button at top of window
  - [ ] 8.3 Add "Remove" button at top of window (disabled by default, enabled when book selected)
  - [ ] 8.4 Create list view to display all books with title, author, and document type badge (epub/PDF/HTML icon)
  - [ ] 8.5 Implement collapsible/expandable sections for each book showing spine items (chapters)
  - [ ] 8.6 Add click handler on spine items to open chapter in SuttaSearchWindow using `get_book_spine_html()`
  - [ ] 8.7 Add LibraryWindow.qml to qml_files list in `bridges/build.rs`
  - [ ] 8.8 Create type definition `assets/qml/com/profoundlabs/simsapa/LibraryWindow.qml` for qmllint if needed
  - [ ] 8.9 Add "Library" menu item under "Windows" menu in `SuttaSearchWindow.qml` to open LibraryWindow
  - [ ] 8.10 Test opening LibraryWindow and verify it displays empty state initially

- [ ] 9.0 Implement document import dialog and user flow
  - [ ] 9.1 Create `assets/qml/DocumentImportDialog.qml` as modal dialog
  - [ ] 9.2 Add file picker button that opens with filters for .epub, .pdf, .html, .htm files
  - [ ] 9.3 Detect document type based on file extension
  - [ ] 9.4 Extract basic metadata (title, author) from selected file using format-specific extraction
  - [ ] 9.5 Add editable text fields: Title (pre-filled), Author (pre-filled), UID (pre-filled with filename without extension)
  - [ ] 9.6 For HTML files only: add checkbox "Split into chapters" with dropdown (h1-h6) and custom tag text field
  - [ ] 9.7 Add "Import" button that validates UID is not empty and unique
  - [ ] 9.8 If UID conflict detected, show dialog: "UID already exists. Overwrite existing book or choose different UID?"
  - [ ] 9.9 Call appropriate import function from SuttaBridge (or LibraryBridge) based on document type
  - [ ] 9.10 Show progress indicator during import
  - [ ] 9.11 Show success/error message after import completion
  - [ ] 9.12 Refresh LibraryWindow display after successful import
  - [ ] 9.13 Add corresponding bridge functions in `bridges/src/sutta_bridge.rs` or create `bridges/src/library_bridge.rs`

- [ ] 10.0 Integrate library search into existing search infrastructure
  - [ ] 10.1 Add `Library` variant to `SearchArea` enum in `backend/src/types.rs`
  - [ ] 10.2 Add "Library" option to search_area_dropdown in `assets/qml/SearchBarInput.qml`
  - [ ] 10.3 Extend `SearchQueryTask` in `backend/src/query_task.rs` to handle SearchArea::Library
  - [ ] 10.4 Implement FTS5 query against `book_spine_items_fts` table when search_area is Library
  - [ ] 10.5 Format search results to include book_uid, spine_item_uid, title, snippet with highlighted matches
  - [ ] 10.6 Return results in format compatible with `FulltextResults.qml` display
  - [ ] 10.7 Update result click handler to open book spine item using `get_book_spine_html()` when result is from Library
  - [ ] 10.8 Test search functionality: import a book, search for content, verify results display and clicking opens correct chapter

- [ ] 11.0 Implement document removal functionality
  - [ ] 11.1 Add selection tracking to LibraryWindow list view to track currently selected book
  - [ ] 11.2 Enable "Remove" button only when a book is selected
  - [ ] 11.3 On "Remove" button click, show confirmation dialog with book title: "Remove '<title>' from library?"
  - [ ] 11.4 Add bridge function `remove_book(book_uid: QString)` in SuttaBridge or LibraryBridge
  - [ ] 11.5 Call `delete_book_by_uid()` from backend which triggers cascade delete of spine items and resources
  - [ ] 11.6 Show success message after deletion
  - [ ] 11.7 Refresh LibraryWindow display to remove deleted book from list
  - [ ] 11.8 Test removal: import a book, select it, remove it, verify it's deleted from database and UI

- [ ] 12.0 Create comprehensive tests for all document formats
  - [ ] 12.1 Create `backend/tests/test_epub_import.rs`
  - [ ] 12.2 Test importing `backend/tests/data/its-essential-meaning.epub` with book_uid="ess"
  - [ ] 12.3 Verify book record is created with correct metadata
  - [ ] 12.4 Verify spine items are created with UIDs: "ess.0", "ess.1", "ess.2", etc.
  - [ ] 12.5 Verify spine items have correct titles from TOC
  - [ ] 12.6 Verify content_plain contains extractable text for FTS5
  - [ ] 12.7 Verify resources are stored with correct paths and MIME types
  - [ ] 12.8 Test resource retrieval through `get_book_resource()`
  - [ ] 12.9 Create `backend/tests/test_pdf_import.rs`
  - [ ] 12.10 Test importing `backend/tests/data/pali-lessons.pdf` with book_uid="pali-lessons"
  - [ ] 12.11 Verify single spine item created with spine_item_uid="word-of-buddha.0"
  - [ ] 12.12 Verify PDF stored in book_resources with resource_path="document.pdf" and mime_type="application/pdf"
  - [ ] 12.13 Verify content_html contains embedpdf.js reference
  - [ ] 12.14 Verify content_plain contains extracted text
  - [ ] 12.15 Create `backend/tests/test_html_import.rs`
  - [ ] 12.16 Create sample HTML file `backend/tests/data/sample.html` with multiple h2 sections
  - [ ] 12.17 Test importing HTML without splitting (single spine item)
  - [ ] 12.18 Test importing HTML with splitting by h2 tag (multiple spine items)
  - [ ] 12.19 Verify spine item titles match h2 tag contents
  - [ ] 12.20 Test FTS5 search finds content within imported documents
  - [ ] 12.21 Test cascade delete removes all spine items and resources when book is deleted
  - [ ] 12.22 Run all tests: `cd backend && cargo test`

- [ ] 13.0 Add bootstrap import support and integration
  - [ ] 13.1 Create or update `cli/src/bootstrap/import_documents.rs` module
  - [ ] 13.2 Add function to read document import configuration from file or command line args
  - [ ] 13.3 Support batch import of multiple documents during bootstrap
  - [ ] 13.4 Add documents to bootstrap process configuration with: file_path, book_uid, document_type, optional split_tag for HTML
  - [ ] 13.5 Call appropriate import functions (epub, PDF, HTML) during bootstrap
  - [ ] 13.6 Add error handling and logging for bootstrap import failures
  - [ ] 13.7 Test bootstrap import with test data files
  - [ ] 13.8 Document bootstrap import configuration format in README or docs
  - [ ] 13.9 Verify imported documents are searchable and readable after bootstrap completes
