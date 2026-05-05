# Simsapa Dhamma Reader - Project Map

## Overview

Simsapa is a multi-platform Qt6 application for reading Buddhist suttas and Pali dictionaries. The architecture follows a layered approach:

```
Frontend (Qt6/QML) ← → C++ Layer ← → Rust Backend with CXX-Qt (Database + Logic)
```

## Directory Structure

```
├── AGENTS.md
├── CMakeLists.txt
├── Makefile
├── package.json
├── PROJECT_MAP.md
├── README.md
└── webpack.config.js
```

### Core Application Layers

#### `/android/` - Android Platform

**Primary Purpose:** Android-specific build configuration and resources

```
├── android
│   ├── AndroidManifest.xml
│   ├── build.gradle
│   ├── res
```

- `AndroidManifest.xml` - Android app manifest
- `build.gradle` - Android build configuration
- `res/` - Android resources (icons, configurations)

#### `/assets/css/`, `/assets/sass/` - Styling

**Primary Purpose:** Content styling and theming for HTML views

```
├── assets
│   ├── css
│   │   ├── dictionary.css
│   │   ├── ebook_extra.css
│   │   └── suttas.css
│   ├── sass
```

- `dictionary.css`, `suttas.css` - Main content styling
- `sass/` directory contains SASS source files that compile to CSS

#### `/assets/js/` - JavaScript Components

**Primary Purpose:** Client-side functionality for HTML content

```
├── assets
│   ├── js
│   │   ├── dictionary.js
│   │   ├── ebook_extra.js
│   │   ├── simsapa.min.js
│   │   └── suttas.js
```

- `simsapa.min.js` - Main JavaScript bundle (built from `/src-ts/`)
- `dictionary.js`, `suttas.js` - Content-specific JavaScript

#### `/assets/qml/` - QML User Interface Components

**Primary Purpose:** Declarative UI components for the application

```
├── assets
│   ├── qml
│   │   ├── com
│   │   │   └── profoundlabs
│   │   │       └── simsapa
│   │   ├── AboutDialog.qml
│   │   ├── ChapterListItem.qml
│   │   ├── CMenuItem.qml
│   │   ├── ColorThemeDialog.qml
│   │   ├── DictionaryHtmlView_Desktop.qml
│   │   ├── DictionaryHtmlView_Mobile.qml
│   │   ├── DictionaryHtmlView.qml
│   │   ├── DictionaryTab.qml
│   │   ├── DownloadAppdataWindow.qml
│   │   ├── DrawerEmptyItem.qml
│   │   ├── DrawerMenu.qml
│   │   ├── FulltextResults.qml
│   │   ├── GlossTab.qml
│   │   ├── ListBackground.qml
│   │   ├── PromptsTab.qml
│   │   ├── SearchBarInput.qml
│   │   ├── StorageDialog.qml
│   │   ├── SuttaHtmlView_Desktop.qml
│   │   ├── SuttaHtmlView_Mobile.qml
│   │   ├── SuttaHtmlView.qml
│   │   ├── SuttaSearchWindow.qml
│   │   ├── SuttaStackLayout.qml
│   │   ├── SuttaTabButton.qml
│   │   ├── tst_GlossTab.qml
│   │   └── WordSummary.qml
```

- **Main Components:**
  - `SuttaSearchWindow.qml` - Sutta search and reading interface
  - `LibraryWindow.qml` - Library management window with nested chapter list support
  - `ChapterListItem.qml` - Reusable component for rendering book chapters with expand/collapse for nested TOC items
  - `DictionaryTab.qml`, `GlossTab.qml`, `PromptsTab.qml` - Tab interfaces
  - `DictionaryHtmlView.qml`, `SuttaHtmlView.qml` - Content display views
  - `DrawerMenu.qml` - Navigation drawer menu
  - `SearchBarInput.qml`, - Search interface component
  - `AboutDialog.qml`, `StorageDialog.qml`, `ColorThemeDialog.qml` - Dialog windows

- `assets/qml/tst_*.qml` - QML component tests
- `assets/qml/profoundlabs/simsapa/` - type definition dummies for qmllint

#### `/assets/` - Static Resources

```
├── assets
│   ├── icons
│   ├── fonts
│   ├── dpd-res
│   ├── templates
│   │   ├── icons.html
│   │   ├── menu.html
│   │   └── page.html
│   ├── common-words.json
│   └── icons.qrc
```

- `icons/` - Application icons in various formats (SVG, PNG)
- `fonts/` - Custom fonts (Abhaya Libre, Crimson Pro, Source Sans)
- `templates/` - HTML templates for content rendering
- `dpd-res/` - Digital Pali Dictionary specific resources

#### `/backend/` - Rust Backend Core

**Primary Purpose:** Database operations, business logic, content processing

```
├── backend
│   ├── src
│   │   ├── db
│   │   │   ├── appdata_models.rs
│   │   │   ├── appdata.rs
│   │   │   ├── appdata_schema.rs
│   │   │   ├── dictionaries_models.rs
│   │   │   ├── dictionaries.rs
│   │   │   ├── dictionaries_schema.rs
│   │   │   ├── dpd_models.rs
│   │   │   ├── dpd.rs
│   │   │   ├── dpd_schema.rs
│   │   │   └── mod.rs
│   │   ├── app_data.rs
│   │   ├── app_settings.rs
│   │   ├── dir_list.rs
│   │   ├── helpers.rs
│   │   ├── html_content.rs
│   │   ├── lib.rs
│   │   ├── logger.rs
│   │   ├── lookup.rs
│   │   ├── pali_sort.rs
│   │   ├── pali_stemmer.rs
│   │   ├── query_task.rs
│   │   ├── search
│   │   │   ├── indexer.rs
│   │   │   ├── mod.rs
│   │   │   ├── schema.rs
│   │   │   ├── searcher.rs
│   │   │   ├── tokenizer.rs
│   │   │   └── types.rs
│   │   ├── stardict_parse.rs
│   │   ├── theme_colors_dark.json
│   │   ├── theme_colors_light.json
│   │   ├── theme_colors.rs
│   │   └── types.rs
│   ├── tests
│   │   ├── helpers
│   │   │   └── mod.rs
│   │   ├── test_dpd_deconstructor_list.rs
│   │   ├── test_dpd_lookup.rs
│   │   ├── test_query_task.rs
│   │   └── test_render_sutta_content.rs
│   ├── Cargo.toml
```

- **Entry Point:** `src/lib.rs:37` - `init_app_globals()`, `src/lib.rs:54` - `init_app_data()`
- **Key Modules:**
  - `src/db/` - Database models, connections, and queries (Diesel ORM + SQLite)
  - `src/app_data.rs` - Central data management and caching
  - `src/lookup.rs` - Dictionary and word lookup functionality
  - `src/query_task.rs` - Search query processing and filtering; `results_page` dispatch, FTS5 helpers with uid prefix/suffix push-down + parallel `SELECT COUNT(*)`, and the boundary-aware `split_page_across_streams` orchestrator for regular ⊕ bold pagination
  - `src/search/` - Tantivy schema, indexer, searcher, and tokenizer for the unified dict (incl. bold-definitions), sutta, and library indexes
  - `src/html_content.rs` - HTML template rendering for content display
  - `src/pali_stemmer.rs` - Pali language stemming for better search
  - `src/stardict_parse.rs` - StarDict dictionary format parser
  - `src/theme_colors.rs` - Theme color management for dark/light modes
  - `src/app_settings.rs` - Application settings and configuration
  - `src/helpers.rs` - Utility functions including Linux desktop launcher creation
- `backend/tests/` - Rust backend unit tests

#### `/bridges/` - Rust-C++ Bridge Layer

**Primary Purpose:** CXX-Qt bindings connecting Rust backend to C++ frontend

```
├── bridges
│   ├── src
│   │   ├── api.rs
│   │   ├── asset_manager.rs
│   │   ├── lib.rs
│   │   ├── prompt_manager.rs
│   │   ├── storage_manager.rs
│   │   └── sutta_bridge.rs
│   ├── build.rs
│   └── Cargo.toml
```

- **Entry Point:** `src/lib.rs` - Bridge module declarations
- **Key Modules:**
  - `src/api.rs` - HTTP API bridge for web-based interactions
  - `src/sutta_bridge.rs` - Sutta (Buddhist text) related bridge functions
  - `src/asset_manager.rs` - Asset and resource management bridge
  - `src/storage_manager.rs` - Storage path and file management bridge
  - `src/prompt_manager.rs` - AI prompt management bridge

#### `/cli/` - Command Line Interface

**Primary Purpose:** CLI tool for backend functionality

```
├── cli
│   ├── src
│   │   └── main.rs
│   └── Cargo.toml
```

- `src/main.rs` - CLI entry point using the backend library

#### `/cpp/` - C++ Layer

**Primary Purpose:** Qt6 application framework and window management

```
├── cpp
│   ├── download_appdata_window.cpp
│   ├── download_appdata_window.h
│   ├── errors.cpp
│   ├── errors.h
│   ├── gui.cpp
│   ├── gui.h
│   ├── main.cpp
│   ├── sutta_search_window.cpp
│   ├── sutta_search_window.h
│   ├── system_palette.cpp
│   ├── system_palette.h
│   ├── utils.cpp
│   ├── utils.h
│   ├── window_manager.cpp
│   └── window_manager.h
```

- **Entry Point:** `main.cpp:6` - `start()` function called from `main()`
- **Key Components:**
  - `gui.cpp/.h` - Main GUI initialization and callbacks
  - `window_manager.cpp/.h` - Multiple window management system
  - `sutta_search_window.cpp/.h` - Sutta search interface
  - `download_appdata_window.cpp/.h` - Data download interface
  - `system_palette.cpp/.h` - System theme integration
  - `errors.cpp/.h` - Custom exception handling

#### `/src-ts/` - TypeScript Source

**Primary Purpose:** TypeScript source that builds to `assets/js/simsapa.min.js`

```
├── src-ts
│   ├── helpers.ts
│   ├── index.d.ts
│   ├── simsapa.ts
│   └── tsconfig.json
```

- **Entry Point:** `simsapa.ts`
- **Build Process:** `npx webpack` → `assets/js/simsapa.min.js`
- `helpers.ts` - TypeScript utility functions
- `tsconfig.json` - TypeScript configuration

#### Root Configuration Files

```
├── AGENTS.md
├── CMakeLists.txt
├── Makefile
├── package.json
├── PROJECT_MAP.md
├── README.md
└── webpack.config.js
```

- `CMakeLists.txt` - Main CMake build configuration
- `Makefile` - Build shortcuts and common commands
- `package.json` & `webpack.config.js` - TypeScript/JavaScript build setup
- `build-appimage.sh` - Linux AppImage build script
- `build-macos.sh` - macOS .app bundle and DMG build script
- `build-windows.ps1` - Windows installer build script (PowerShell)
- `simsapa-installer.iss` - Inno Setup installer configuration for Windows
- `WINDOWS_QUICK_START.md` - Quick reference for Windows builds
- `WINDOWS_BUILD_GUIDE.md` - Complete Windows build documentation

## Essential Function Locations

### Application Lifecycle
- **App Initialization:** `cpp/main.cpp:6` → `cpp/gui.cpp` → `backend/src/lib.rs:52`
- **Global State:** `backend/src/lib.rs:59` - `get_app_globals()`
- **App Data:** `backend/src/lib.rs:78` - `get_app_data()`
- **Releases Info:** `backend/src/lib.rs:125` - `set_releases_info()`, `try_get_releases_info()` - Cached API response from update checks

### Database Operations
- **Database Models:** `backend/src/db/schema.rs` (Diesel models)
- **Connection Management:** `backend/src/db/` modules
- **Query Processing:** `backend/src/query_task.rs`

### Search & Lookup
- **Word Lookup:** `backend/src/lookup.rs`
- **Pali Stemming:** `backend/src/pali_stemmer.rs`
- **Dictionary Parsing:** `backend/src/stardict_parse.rs`
- **Query Pipeline:** `backend/src/query_task.rs` — `SearchQueryTask` and the unified `results_page(page_num)` dispatch over `(SearchMode, SearchArea)`. Each per-mode handler returns `(Vec<SearchResult>, total: usize)`; `db_query_hits_count` is written exactly once per call from the storage-layer total. Multi-phase modes (DPD Lookup, Headword Match, Contains+Dictionary) use `split_page_across_streams` for boundary-aware regular ⊕ bold pagination — true SQL `LIMIT/OFFSET` per stream, no Rust-side cover-fetch. `SearchMode::Combined + SearchArea::Dictionary` is rejected here (`Err`) — Combined is bridge-orchestrated; `Combined + (Suttas|Library)` falls through to `FulltextMatch`.
- **Dictionary Inclusion-Set Filtering:** `SearchParams.dict_source_uids: Option<Vec<String>>` carries the per-dict checkbox / lock selection assembled by `assets/qml/SuttaSearchWindow.qml::compute_dict_search_filter()`. ContainsMatch and HeadwordMatch push `dict_label IN (set)` down via JOIN to `dict_words` (rides `dict_words_dict_label_idx`); Fulltext pushes it into Tantivy via `add_dict_filters`; the dispatcher's `apply_dict_source_uids_filter` is a safety net that drops only `table_name == "dict_words"` rows (DPD-native `dpd_headwords` / `dpd_roots` rows pass through unchanged — the bridge's `dpd_enabled` gate is what protects Combined from leaks). DPD Lookup is structurally DPD-only and ignores user-dict membership by design.
- **`dict_words_fts` Schema:** `scripts/dictionaries-fts5-indexes.sql` declares two trigram-indexed columns: `word` and `definition_plain` (both serve `LIKE '%term%'` push-downs). `dict_label` is `UNINDEXED` in the FTS table, so `dict_label IN (set)` is filtered by JOIN to `dict_words`. Schema bumps require manual re-bootstrap of the dictionaries DB — there is no Diesel migration; the script recreates the FTS table and triggers.
- **Combined Mode (bridge-orchestrated):** `bridges/src/sutta_bridge.rs` defines `CombinedCache` + `static COMBINED_CACHE: Mutex<Option<CombinedCache>>` (isolated from `RESULTS_PAGE_CACHE`; cache key carries a `|combined` suffix to prevent cross-warming). `fetch_combined_page` runs DPD Lookup + Fulltext Match as two parallel `thread::spawn` sub-queries on page 0 (cold start), tops up side-aware on later pages, and serves the merged virtual stream `[DPD … , Fulltext …]` by slicing both buffers. The lock is never held across an SQLite or Tantivy call. `run_sub_query` is the unit run inside the parallel threads.
- **Tantivy Schema & Indexer:** `backend/src/search/schema.rs` (sutta / dict / library schemas), `backend/src/search/indexer.rs` (writers; `append_bold_definitions_to_dict_index` appends bold-definition rows into the unified Pāli `dict_words_index_dir`). Schemas store uid as a `raw` field plus a `uid_rev` raw field (lowercased uid reversed character-by-character) so a uid-suffix filter pushes down as `RegexQuery::from_pattern("{reversed}.*", uid_rev)`. Library uses `spine_item_uid` / `spine_item_uid_rev`. The dict schema also carries `is_bold_definition: bool` and `nikaya_group_path` for bold rows; there is no separate `bold_definitions_index_dir` and no `IndexType::BoldDefinitions`.
- **DPPN Cross-Reference Lookup:** `POST /dppn_lookup` in `bridges/src/api.rs` accepts `{ window_id, query }` (URL-decoded by the TS client in `src-ts/helpers.ts`) and invokes the `callback_run_dppn_dictionary_query` FFI callback. C++ side (`cpp/gui.cpp`, `cpp/window_manager.cpp`) routes via `WindowManager::run_dppn_dictionary_query` to the matching `SuttaSearchWindow` by `window_id` (no fallback window creation). The QML slot `SuttaSearchWindow.qml::run_dppn_dictionary_query` drives the visible search UI: reveals sidebar, switches search area to Dictionary, sets mode to Fulltext Match, solo-locks the DPPN dictionary via `dictionaries_panel.toggle_lock("dppn")`, populates the search input, and runs `handle_query` — so the user can edit the query or unlock the filter from the visible UI.
- **Tantivy Searcher:** `backend/src/search/searcher.rs` — `FulltextSearcher` opens per-language `dict_indexes` / `sutta_indexes` / `library_indexes`. `search_single_index` builds a single `BooleanQuery` (content + content_exact + filters), runs `TopDocs::with_limit(page_len)` paired with `Count`, and constructs `SnippetGenerator` once per call (snippet cost bounded to `page_len`). `add_uid_filters` is the one push-down helper used by sutta/dict/library; bold rows are gated via `Occur::MustNot { is_bold_definition = true }` when `include_comm_bold_definitions = false`. Per-doc dispatch in the dict arm peeks at `is_bold_definition` and routes bold rows to `bold_definition_doc_to_result`.

### Content Rendering  
- **HTML Generation:** `backend/src/html_content.rs`
- **Template Processing:** Uses `tinytemplate` crate for HTML templates
- **Content Display:** QML views in `assets/qml/`
- **DPPN Entries:** `backend/src/html_content.rs::render_dppn_entry` mirrors `render_bold_definition` — wraps the (already `<div class="dppn">`-prefixed) `definition_html` with the standard page chrome (`sutta_html_page` + `DICTIONARY_CSS` + `WINDOW_ID` JS). Dispatched from `backend/src/app_data.rs::render_word_uid_to_html` when `dict_label == "dppn"`, ahead of the generic full-document rewrite path. Bootstrap-time transform in `cli/src/bootstrap/dppn.rs::transform_dppn_definition_html` rewrites every `<span class="t14">TEXT</span>` to `<a class="dppn-ref" href="ssp://dppn_lookup/{ENCODED}">…</a>` with percent-encoded UTF-8 (preserves diacritics). Styling lives under `.dppn` scope in `assets/css/dictionary.css` (no leakage into other dict entries).

### UI Components
- **Main Windows:** `cpp/window_manager.cpp`, QML window components
- **Search Interface:** `cpp/sutta_search_window.cpp`, `assets/qml/SuttaSearchWindow.qml`
- **Download Interface:** `cpp/download_appdata_window.cpp`, `assets/qml/DownloadAppdataWindow.qml`
  - **Language Selection:** User can enter comma-separated language codes (e.g., "hu, pt, it") or "*" for all
  - **Language Validation:** Validates entered codes against available languages from LANG_CODE_TO_NAME
  - **Language Downloads:** Downloads suttas_lang_{lang}.tar.bz2 files and imports into appdata.sqlite3
  - **Auto-initialization:** Reads download_languages.txt from app_assets_dir if present
- **Gloss Tab:** `assets/qml/GlossTab.qml` - Pali text analysis with vocabulary and AI translations
  - **AI Translation Interface:** `assets/qml/AssistantResponses.qml` - Tabbed interface for multiple AI model responses
  - **Response Tab Buttons:** `assets/qml/ResponseTabButton.qml` - Individual tabs with status indicators
- **Prompts Tab:** `assets/qml/PromptsTab.qml` - AI conversation interface

### Platform Integration
- **Mobile Detection:** `backend/src/lib.rs:427` - `is_mobile()`
- **Storage Management:** `bridges/src/storage_manager.rs`
- **Asset Management:** `bridges/src/asset_manager.rs`
  - **Download & Extract:** `download_urls_and_extract()` - Downloads tar.bz2 files and extracts to app-assets
  - **Language Support:** `get_available_languages()` - Returns list of downloadable language codes from LANG_CODE_TO_NAME
  - **Language Initialization:** `get_init_languages()` - Reads download_languages.txt for pre-configured languages
  - **Language Import:** `import_suttas_lang_to_appdata()` - Imports suttas from language databases into appdata
- **Linux Desktop Launcher:** `backend/src/helpers.rs:910` - Automatic .desktop file creation for AppImage integration
  - **AppImage Detection:** `backend/src/helpers.rs:887` - `is_running_from_appimage()`
  - **Desktop File Creation:** `backend/src/helpers.rs:943` - `create_or_update_linux_desktop_icon_file()`
  - **Qt Integration:** `cpp/gui.cpp:93` - Calls desktop file creation during startup
  - **Desktop Filename Setting:** `cpp/gui.cpp:111` - Sets Qt desktop filename for proper integration

### AI Integration
- **Prompt Manager:** `bridges/src/prompt_manager.rs` - AI API communication and request handling
- **Translation Requests:** Multi-model support with automatic retry logic and error handling
- **Markdown Processing:** Built-in markdown to HTML conversion for AI responses
- **Export Integration:** AI translations included in HTML, Markdown, and Org-Mode exports

### Configuration & Settings
- **App Settings:** `backend/src/app_settings.rs` — includes `search_last_mode: IndexMap<String, String>` keyed by area name (`"Suttas"` / `"Dictionary"` / `"Library"`); per-area defaults applied at read time (`"Combined"` for Dictionary, `"Fulltext Match"` for Suttas/Library) via `AppData::get_last_search_mode(area)` / `set_last_search_mode(area, mode)`. Surfaced to QML as `SuttaBridge.get_last_search_mode` / `set_last_search_mode` (area-generic).
- **Theme Colors:** `backend/src/theme_colors.rs`
- **Directory Paths:** `backend/src/lib.rs:131` - `AppGlobalPaths`

### Database Upgrade Flow
The app uses a single `appdata.sqlite3` for both seeded content and user-generated data. User-generated rows are tagged with `is_user_added = true` (runtime default); bootstrap-seeded rows are inserted with `is_user_added = false`. Export/import filters on that column.

When the user triggers a database upgrade:

1. **Prepare for Upgrade:** `bridges/src/sutta_bridge.rs` - `prepare_for_database_upgrade()`
   - Exports user data via `export_user_data_to_assets()` in `backend/src/app_data.rs`
   - Creates marker files: `delete_files_for_upgrade.txt`, `auto_start_download.txt`, `download_languages.txt`

2. **Export User Data:** `backend/src/app_data.rs` - `export_user_data_to_assets()`
   - Creates `import-me/` folder in app_assets_dir
   - Exports `app_settings.json` - user's application settings
   - Exports `download_languages.txt` - selected language codes for re-download
   - Exports per-table SQLite files filtered by `is_user_added = true`: `appdata-books.sqlite3`, `appdata-bookmarks.sqlite3`, `appdata-chanting.sqlite3`

3. **User Restarts App**

4. **Startup Detection:** `cpp/gui.cpp` - `check_delete_files_for_upgrade()`
   - `backend/src/lib.rs` - Checks for marker file, deletes old databases

5. **Download New Databases:** `assets/qml/DownloadAppdataWindow.qml`
   - Auto-starts download if `auto_start_download.txt` marker exists
   - Pre-fills language selection from `download_languages.txt`

6. **User Restarts After Download**

7. **Import User Data:** `cpp/gui.cpp` - `import_user_data_after_upgrade()`
   - Called after `init_app_data()` on startup
   - `backend/src/app_data.rs` - `import_user_data_from_assets()`
     - Imports app settings from `import-me/app_settings.json`
     - Imports user books, bookmarks, and chanting data from the per-table files
     - Cleans up by removing the `import-me/` folder

### One-Shot Legacy Userdata Bridge (alpha testers)
Historically the app maintained a separate `userdata.sqlite3`. For alpha testers upgrading from that era, `export_user_data_to_assets()` detects any remaining `userdata.sqlite3` in `app_assets_dir`, runs `upgrade_appdata_schema` on a copy so Diesel models align, extracts `app_settings.json` from the legacy row, and aliases the migrated copy under the per-table filenames consumed by the standard importer. A safety copy is placed at `import-me/legacy-userdata.sqlite3`, and the standard importer runs a defensive tail pass to re-apply legacy `app_settings` if the JSON extraction failed silently. Once imported, `cleanup_stale_legacy_userdata()` in `backend/src/lib.rs` removes any leftover `userdata.sqlite3` on a subsequent startup.

## Build Commands Quick Reference

### Development Build
- **Full Build:** `make build -B`
- **Run Application:** `make run`
- **TypeScript Build:** `npx webpack`
- **Sass Build:** `make sass`
- **Backend Tests:** `cd backend && cargo test`
- **QML Tests:** `make qml-test`

### Distribution Packages

#### Linux AppImage
- **Build AppImage:** `make appimage -B`
- **Clean rebuild:** `make appimage-rebuild`
- **Clean only:** `make appimage-clean`

#### macOS Bundle & DMG
- **Build DMG:** `make macos -B`
- **App bundle only:** `make macos-app` (skips DMG creation)
- **Clean rebuild:** `make macos-rebuild`
- **Clean only:** `make macos-clean`

#### Windows Installer
- **Build Installer:** `powershell -ExecutionPolicy Bypass -File build-windows.ps1` or `make windows`
- **Clean rebuild:** `make windows-rebuild`
- **Clean only:** `make windows-clean`
- **Quick Start:** See [WINDOWS_QUICK_START.md](WINDOWS_QUICK_START.md)
- **Full Guide:** See [WINDOWS_BUILD_GUIDE.md](WINDOWS_BUILD_GUIDE.md)
- **Requirements:**
  - Qt 6.9.3 installed at `C:\Qt\6.9.3\msvc2022_64`
  - CMake and Ninja (from Qt installation or system PATH)
  - Rust toolchain: `x86_64-pc-windows-msvc`
  - Inno Setup 6 for installer creation
- **Output:**
  - `dist\simsapadhammareader.exe` (with Qt dependencies)
  - `Simsapa-Setup-{version}.exe` (installer)
- **Note:** Use `-ExecutionPolicy Bypass` to run PowerShell scripts if you get "scripts disabled" error

## Data Flow
1. **User Input** → QML Components → C++ Event Handlers
2. **C++ Bridge** → CXX-Qt Bindings → Rust Backend Functions  
3. **Rust Backend** → Database Queries → Content Processing
4. **Response Path** → Rust Results → C++ Bridge → QML Display

## Key External Dependencies
- **Qt6** - GUI framework and QML runtime
- **Diesel** - Rust ORM for SQLite database operations
- **CXX-Qt** - Rust-C++ interoperability layer
- **StarDict** - Dictionary format support
- **TinyTemplate** - HTML template engine
