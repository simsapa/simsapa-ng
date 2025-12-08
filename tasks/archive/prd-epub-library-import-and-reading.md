# PRD: Document Library Import and Reading (Epub, PDF, HTML)

## Introduction/Overview

This feature adds the ability to import documents in multiple formats (Epub, PDF, HTML) into Simsapa's database and read them within the application. Users will be able to import documents during the bootstrap process or manually through the UI, perform fulltext searches across document content, and navigate through chapters similar to reading suttas. This extends Simsapa's capabilities beyond Buddhist texts to include any document content, making it a more versatile reading and research platform.

The implementation leverages multiple Rust crates for parsing different formats:
- `epub` crate (danigm/epub-rs) for parsing epub files
- `pdf-extract` crate for extracting plain text from PDFs
- `lopdf` crate for extracting PDF metadata
- Built-in HTML parsing for HTML documents

All formats integrate seamlessly with the existing sutta reading infrastructure, including fulltext search, HTML rendering, and resource management.

## Goals

1. Enable users to import and read documents in Epub, PDF, and HTML formats within the Simsapa application
2. Provide fulltext search capabilities across all imported document content
3. Integrate document reading experience with existing sutta reading UI
4. Support both bootstrap-time and user-initiated document imports
5. Maintain consistent data architecture following existing patterns (suttas, spine items, resources)
6. Enable resource loading (images, CSS, fonts) for proper document rendering
7. Support PDF viewing with embedded PDF viewer (embedpdf.js)
8. Allow flexible HTML document splitting based on user-specified HTML tags

## User Stories

1. As a user, I want to import epub, PDF, and HTML documents during initial setup so that my library is ready when I start using the application
2. As a user, I want to import additional documents through the UI so that I can expand my library after initial setup
3. As a user, I want to search for content within my imported documents so that I can quickly find specific passages or topics
4. As a user, I want to browse my document library and see all available items so that I can navigate to specific sections
5. As a user, I want to read epub and HTML documents with proper formatting and images so that I have a quality reading experience
6. As a user, I want to view PDF documents with an embedded PDF viewer so that I can read them natively
7. As a user, I want to split HTML documents into chapters based on specific HTML tags so that I can organize long documents
8. As a user, I want documents to open in the same reading interface as suttas so that I have a consistent experience

## Functional Requirements

### Database Schema

1. The system must create a `books` table in appdata_schema with the following fields:
   - `id` (Integer, primary key)
   - `uid` (Text, unique identifier, e.g., "ess")
   - `document_type` (Text, one of: "epub", "pdf", "html")
   - `title` (Text, nullable)
   - `author` (Text, nullable)
   - `file_path` (Text, path to original document file)
   - `metadata_json` (Text, JSON-serialized metadata from respective parser)

2. The system must create a `book_spine_items` table with the following fields:
   - `id` (Integer, primary key)
   - `book_id` (Integer, foreign key to books.id)
   - `book_uid` (Text, denormalized for quick lookup)
   - `spine_item_uid` (Text, unique identifier in format "<book_uid>.<spine_index>", e.g., "ess.2" for spine index 2)
   - `spine_index` (Integer, 0-based position in the epub spine)
   - `title` (Text, nullable, chapter title if available)
   - `content_html` (Text, nullable, HTML content of the chapter)
   - `content_plain` (Text, nullable, plain text extracted from HTML for FTS5 indexing)

3. The system must create a `book_resources` table with the following fields:
   - `id` (Integer, primary key)
   - `book_id` (Integer, foreign key to books.id)
   - `book_uid` (Text, denormalized for quick lookup)
   - `resource_path` (Text, path/identifier within the document, or "document.pdf" for PDF files)
   - `mime_type` (Text, nullable, resource MIME type, e.g., "application/pdf")
   - `content_data` (Blob, binary content of the resource)

4. The system must create an FTS5 virtual table `book_spine_items_fts` that indexes the `content_plain` field from `book_spine_items`, following the same pattern as the `suttas` FTS5 index. This should be created via the `scripts/appdata-fts5-indexes.sql` script, not in the migration up.sql file

5. The system must define proper foreign key relationships with cascade delete behavior:
   - `book_spine_items.book_id` → `books.id`
   - `book_resources.book_id` → `books.id`

### Epub Parsing and Import

6. The system must use the `epub` Rust crate (danigm/epub-rs) to parse epub files via `EpubDoc`

7. The system must extract and store the following data during epub import:
   - Complete book metadata (title, author, and full Vec<MetadataItem>)
   - All spine items (chapters) with their HTML content
   - Chapter titles from the epub TOC (table of contents) when available, matched to corresponding spine items (see epub crate API examples)
   - All resources (images, CSS, fonts, etc.) as binary data without compression

8. The system must convert each epub spine item's HTML content to plain text (using the same logic as sutta imports during bootstrap) and store it in `content_plain` for FTS5 indexing

9. The system must store the spine_index (0-based) from the epub spine for each spine item

10. The system must generate epub spine item UIDs in the format "<book_uid>.<spine_index>" using a dot separator (e.g., "ess.2" for book "ess" at spine index 2)

11. The system must rewrite resource links in epub chapter HTML to use the localhost API endpoint format: `/book_resources/<book_uid>/<resource_path>`

### PDF Parsing and Import

12. The system must use the `pdf-extract` crate to extract plain text content from PDF files

13. The system must use the `lopdf` crate to extract basic metadata from PDF files (title, author) following the example at https://github.com/Autoparallel/learner/blob/main/crates/learner/src/pdf.rs

14. The system must represent a PDF as a book with a single `book_spine_item` entry where:
    - `spine_index` is 0
    - `spine_item_uid` is "<book_uid>.0"
    - `content_plain` contains extracted text from `pdf-extract` for FTS5 indexing
    - `content_html` contains HTML that embeds the PDF using embedpdf.js (see https://www.embedpdf.com/docs/snippet/introduction)

15. The system must store the complete PDF file as a blob in the `book_resources` table with:
    - `resource_path` set to "document.pdf"
    - `mime_type` set to "application/pdf"
    - `content_data` containing the full PDF binary data

16. The system must generate HTML content for PDF display that references the PDF resource via `/book_resources/<book_uid>/document.pdf` and uses embedpdf.js for rendering

### HTML Document Parsing and Import

17. The system must accept HTML documents and provide an option in the import UI to either:
    - Import as a single spine item (no splitting)
    - Split into multiple spine items based on a specified HTML tag

18. When splitting HTML documents, the system must display UI options for:
    - A dropdown with common tags (h1, h2, h3, h4, h5, h6)
    - A text field to enter a custom HTML tag as a string

19. The system must split HTML documents by the specified tag, creating one `book_spine_item` for each section

20. The system must use the text content of the splitting tag as the chapter title for each spine item

21. The system must generate HTML spine item UIDs in the format "<book_uid>.<spine_index>" where spine_index is 0-based sequential position

22. The system must convert HTML content to plain text for each spine item and store in `content_plain` for FTS5 indexing

23. The system must extract and store embedded images or other resources from HTML as separate entries in `book_resources` table

24. The system must rewrite resource links in HTML content to use the localhost API endpoint format: `/book_resources/<book_uid>/<resource_path>`

### Backend Helper Functions

12. The system must implement `appdata.get_book_spine_item(spine_item_uid: &str) -> Option<BookSpineItem>` similar to `appdata.get_sutta()`

13. The system must implement `appdata.get_book_resource(book_uid: &str, resource_path: &str) -> Option<BookResource>` to retrieve resources by book and path

14. The system must add wrapper functions to `SuttaBridge`:
    - `get_book_spine_html(window_id: &QString, spine_item_uid: &QString) -> QString` (similar to `get_sutta_html()`)

15. The system must add an API endpoint in `api.rs`: `GET /book_resources/<book_uid>/<resource_path>` that returns the resource with appropriate MIME type headers

### Search Integration

16. The system must add "Library" as an option to `search_area_dropdown` in `SearchBarInput.qml`

17. The system must extend `query_task.rs` to handle Library search queries by querying the `book_spine_items_fts` table

18. The system must return book search results in a format compatible with `FulltextResults.qml`, allowing them to display alongside sutta results

19. The system must ensure clicking on a book search result opens the corresponding chapter in the sutta HTML view (SuttaSearchWindow)

### Library UI

20. The system must add a "Library" menu item under the "Windows" menu in `SuttaSearchWindow.qml`

21. The system must create a new `LibraryWindow.qml` (ApplicationWindow type) similar to `SuttaLanguagesWindow.qml`

22. The system must display all imported books in `LibraryWindow` with their basic metadata (title, author)

23. The system must display books with a collapsible list of chapters beneath each book

24. The system must allow clicking on a chapter to open it in the sutta reading view in `SuttaSearchWindow`

25. The system must add an "Import Document..." button at the top of `LibraryWindow` (supporting Epub, PDF, and HTML formats)

26. The system must add a "Remove" button at the top of `LibraryWindow` to remove the selected book from the database

### User Import Flow

27. The system must display a file picker dialog when "Import Document..." is selected, with file filters for .epub, .pdf, and .html/.htm files

28. The system must detect the document type based on file extension

29. The system must extract basic metadata (title, author) from the selected document:
    - For epub: extract from EpubDoc metadata
    - For PDF: extract using lopdf crate
    - For HTML: attempt to extract from `<title>` tag and meta tags

30. For HTML imports only, the system must display additional UI controls in the import dialog:
    - Checkbox: "Split into chapters"
    - Dropdown (enabled when checkbox is checked): Common HTML tags (h1, h2, h3, h4, h5, h6)
    - Text field (enabled when checkbox is checked): Custom HTML tag input

31. The system must display an import dialog with editable fields:
    - Title (pre-filled from document metadata)
    - Author (pre-filled from document metadata)
    - UID (pre-filled with the document filename without extension)
    - HTML splitting options (only visible for HTML files)

32. The system must provide "Import" and "Cancel" buttons in the import dialog

33. The system must validate that the UID is not empty and does not conflict with existing book UIDs

34. If a UID conflict is detected, the system must prompt the user to either:
    - Provide a different UID
    - Overwrite the existing book (remove and re-import)

35. The system must perform the appropriate import operation based on document type when the user clicks "Import" with valid data

36. The system must refresh the library window display after successful import

### Document Removal

37. The system must enable the "Remove" button only when a document is selected in the library list

38. The system must display a confirmation dialog when the user clicks "Remove" showing the document title and asking for confirmation

39. The system must delete the selected document and all associated spine items and resources from the database (using cascade delete)

40. The system must refresh the library window display after successful removal

### Bootstrap Import

41. The system must support importing documents during the bootstrap process by accepting:
    - File path to the document
    - Intended book UID
    - Document type (epub, pdf, or html)
    - For HTML: optional split tag specification

42. The system must create test cases using `backend/tests/data/its-essential-meaning.epub` with book UID "ess"

43. The system must create test cases for PDF import using:
    - `backend/tests/data/pali-lessons.pdf` with book UID "pali-lessons"

44. The system must create test cases for HTML import with both single-spine and split-by-tag scenarios

45. The system must validate that test spine item UIDs follow the format "<uid>.0", "<uid>.1", "<uid>.2", etc. (using dot separator and 0-based spine index)

### Error Handling

46. If a document import fails (corrupted file, missing metadata, parse errors), the system must:
    - Display an error dialog with a descriptive error message specific to the document type
    - Skip the file and not create partial database entries
    - Log the error for debugging purposes

47. The system must handle missing resources gracefully by logging warnings but continuing with content display

48. The system must validate document file format before attempting full import

49. For HTML splitting, if the specified tag is not found in the document, the system must:
    - Display a warning to the user
    - Offer to import as a single spine item instead

## Non-Goals (Out of Scope)

1. Bookmarking and annotation features within documents (future enhancement)
2. Reading progress tracking and resume functionality (future enhancement)
3. Document export functionality (future enhancement)
4. Support for formats other than Epub, PDF, and HTML (e.g., MOBI, DOCX, TXT)
5. Editing or modifying imported document content
6. Cloud sync of library across devices
7. Advanced library organization features (tags, collections, ratings)
8. OCR for scanned PDFs (use pre-processed PDFs with text layer)
9. PDF annotation or form filling capabilities

## Design Considerations

### QML Components

- Create `LibraryWindow.qml` following the pattern of `SuttaLanguagesWindow.qml`
- Create `DocumentImportDialog.qml` for the user import flow with metadata fields and HTML splitting options
- Extend `SearchBarInput.qml` to include "Library" option
- Reuse existing `SuttaHtmlView` components for rendering document content (epub, HTML, and PDF with embedpdf.js)
- Reuse existing `FulltextResults.qml` for displaying search results

### UI Layout

- Library window should have "Import Document..." and "Remove" buttons at the top
- Library window should use a simple list view with collapsible/expandable document sections
- Each document item shows title, author, and document type icon/badge (epub/PDF/HTML)
- Nested item list shows spine index and titles (for multi-item documents) or single entry (for PDFs)
- "Remove" button should be enabled only when a document is selected
- Import dialog should be modal with clear field labels and validation feedback
- Import dialog should show HTML splitting options only when importing HTML files
- UID field in import dialog should be pre-filled with the document filename (without extension)

### Resource Management

- Store all resources as uncompressed binary data in the database for simplicity
- Use API endpoint pattern `/book_resources/<book_uid>/<resource_path>` for resource loading
- Ensure proper MIME type headers for images, CSS, fonts, and PDF files
- For PDFs, use resource_path "document.pdf" and mime_type "application/pdf"

## Technical Considerations

### Dependencies

- Add `epub = "2.0"` (or latest version) to `backend/Cargo.toml`
  - Reference docs at https://docs.rs/epub/latest/epub/
- Add `pdf-extract` (latest version) to `backend/Cargo.toml` for PDF text extraction
- Add `lopdf` (latest version) to `backend/Cargo.toml` for PDF metadata extraction
  - Reference example at https://github.com/Autoparallel/learner/blob/main/crates/learner/src/pdf.rs
- Include embedpdf.js library for PDF rendering in HTML views
  - Reference docs at https://www.embedpdf.com/docs/snippet/introduction

### Database Migration

- Create new migration file in `backend/migrations/appdata/`
- Follow Diesel ORM conventions for table definitions
- Do NOT include FTS5 virtual table creation in the migration up.sql file
- Create FTS5 index for `book_spine_items_fts` in `scripts/appdata-fts5-indexes.sql` following the same pattern as `suttas_fts`
- Add proper indexes on foreign keys and frequently queried fields in the migration

### Bridge Integration

- Add new Rust file `bridges/src/library_bridge.rs` if additional bridge functions are needed beyond `SuttaBridge`
- Register any new QML modules in `bridges/build.rs`
- Create corresponding QML type definitions for qmllint

### HTML Content Processing

- Reuse existing plain text extraction logic from sutta bootstrap imports for all document types
- Apply same HTML sanitization and processing as suttas
- Parse and rewrite resource URLs in content HTML before storage
- Handle relative and absolute paths in resources correctly
- For epub: Extract chapter titles from TOC (table of contents) and match them to corresponding spine items (refer to epub crate API examples)
- For PDF: Generate HTML wrapper that embeds PDF using embedpdf.js, pointing to `/book_resources/<book_uid>/document.pdf`
- For HTML: Parse and split by specified tag if requested, using tag content as chapter titles

### Testing

- Create unit tests in `backend/tests/test_document_import.rs` covering all three formats
- Use `backend/tests/data/its-essential-meaning.epub` as epub test data
- Create or obtain sample PDF and HTML files for test data
- Test parsing, import, retrieval, and search functionality for all formats
- Verify spine item UID format: "<uid>.0", "<uid>.1", "<uid>.2", etc. (dot separator, 0-based index)
- Test resource loading through API endpoint for all resource types (images, CSS, PDFs)
- Test PDF viewing with embedpdf.js integration
- Test HTML splitting by various tags (h1, h2, custom tags)
- Test HTML import without splitting (single spine item)
- Test search integration with query_task for all document types
- Test document removal with cascade delete of spine items and resources

### Query Integration

- Extend `SearchArea` enum in `backend/src/types.rs` to include `Library`
- Extend `SearchQueryTask` in `backend/src/query_task.rs` to query `book_spine_items_fts`
- Ensure search results include sufficient context for display
- Format results consistently with sutta search results

## Success Metrics

1. Users can successfully import documents in all three formats (Epub, PDF, HTML) during bootstrap with 100% success rate for valid files
2. Users can import documents through the UI with clear feedback on success/failure
3. Fulltext search returns relevant results from document content within 1 second
4. Epub and HTML content renders correctly with images and formatting
5. PDFs display correctly using embedpdf.js viewer
6. Navigation between chapters/sections is smooth and intuitive
7. Zero crashes or data corruption during import process
8. Resource loading (images, CSS, PDFs) works correctly for all document types
9. HTML splitting by tags works accurately for common tags (h1-h6) and custom tags



## Test Data

### Epub Test
- Primary test file: `backend/tests/data/its-essential-meaning.epub`
- Test book UID: "ess"
- Expected spine item UIDs: "ess.0", "ess.1", "ess.2", etc. (dot separator, 0-based spine index)

### PDF Test
- Test PDF files: 
  - `backend/tests/data/pali-lessons.pdf`
- Test book UIDs: "word-of-buddha", "pali-lessons"
- Expected single spine item UIDs: "word-of-buddha.0", "pali-lessons.0"
- Verify PDF stored in book_resources with resource_path "document.pdf" and mime_type "application/pdf"

### HTML Test
- Test HTML file: Create sample HTML with multiple h2 sections
- Test book UID: "sample-html"
- Test with splitting by h2 tag
- Expected spine item UIDs: "sample-html.0", "sample-html.1", etc. based on number of h2 sections
- Test without splitting: single spine item "sample-html.0"

### All Tests Should Verify
- Successful import with valid metadata
- Spine item UIDs follow naming convention (dot separator, 0-based)
- Resources are stored and retrievable
- FTS5 search finds content within spine items
- Content rendering includes working resource links
- Document removal deletes all associated spine items and resources
- PDF rendering with embedpdf.js works correctly
- HTML splitting creates correct number of spine items with appropriate titles
