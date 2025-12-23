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
│   │   ├── WordLookupWindow.qml
│   │   └── WordSummary.qml
```

- **Main Components:**
  - `WordLookupWindow.qml` - Dictionary lookup window UI
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
  - `src/query_task.rs` - Search query processing and filtering
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
│   ├── window_manager.h
│   ├── word_lookup_window.cpp
│   └── word_lookup_window.h
```

- **Entry Point:** `main.cpp:6` - `start()` function called from `main()`
- **Key Components:**
  - `gui.cpp/.h` - Main GUI initialization and callbacks
  - `window_manager.cpp/.h` - Multiple window management system
  - `sutta_search_window.cpp/.h` - Sutta search interface
  - `word_lookup_window.cpp/.h` - Dictionary lookup interface
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

## Essential Function Locations

### Application Lifecycle
- **App Initialization:** `cpp/main.cpp:6` → `cpp/gui.cpp` → `backend/src/lib.rs:37`
- **Global State:** `backend/src/lib.rs:44` - `get_app_globals()`
- **App Data:** `backend/src/lib.rs:63` - `get_app_data()`

### Database Operations
- **Database Models:** `backend/src/db/schema.rs` (Diesel models)
- **Connection Management:** `backend/src/db/` modules
- **Query Processing:** `backend/src/query_task.rs`

### Search & Lookup
- **Word Lookup:** `backend/src/lookup.rs`
- **Pali Stemming:** `backend/src/pali_stemmer.rs`
- **Dictionary Parsing:** `backend/src/stardict_parse.rs`

### Content Rendering  
- **HTML Generation:** `backend/src/html_content.rs`
- **Template Processing:** Uses `tinytemplate` crate for HTML templates
- **Content Display:** QML views in `assets/qml/`

### UI Components
- **Main Windows:** `cpp/window_manager.cpp`, QML window components
- **Search Interface:** `cpp/sutta_search_window.cpp`, `assets/qml/SuttaSearchWindow.qml`
- **Dictionary Interface:** `cpp/word_lookup_window.cpp`, `assets/qml/WordLookupWindow.qml`
- **Download Interface:** `cpp/download_appdata_window.cpp`, `assets/qml/DownloadAppdataWindow.qml`
  - **Language Selection:** User can enter comma-separated language codes (e.g., "hu, pt, it") or "*" for all
  - **Language Validation:** Validates entered codes against available languages from LANG_CODE_TO_NAME
  - **Language Downloads:** Downloads suttas_lang_{lang}.tar.bz2 files and imports into userdata.sqlite3
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
  - **Language Import:** `import_suttas_lang_to_userdata()` - Imports suttas from language databases into userdata
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
- **App Settings:** `backend/src/app_settings.rs`
- **Theme Colors:** `backend/src/theme_colors.rs`
- **Directory Paths:** `backend/src/lib.rs:131` - `AppGlobalPaths`

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
- **Build Installer:** `make windows` or `powershell -ExecutionPolicy Bypass -File build-windows.ps1`
- **Clean rebuild:** `make windows-rebuild`
- **Clean only:** `make windows-clean`
- **Requirements:**
  - Qt 6.9.3 installed at `C:\Qt\6.9.3\msvc2022_64`
  - CMake and Ninja (from Qt installation or system PATH)
  - Rust toolchain: `x86_64-pc-windows-msvc`
  - Inno Setup 6 for installer creation
- **Output:**
  - `dist\simsapadhammareader.exe` (with Qt dependencies)
  - `Simsapa-Setup-{version}.exe` (installer)

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
