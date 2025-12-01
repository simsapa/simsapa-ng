# Tasks: Smart Link Handling in HTML Content

## Relevant Files

- `src-ts/simsapa.ts` - Main TypeScript entry point with DOMContentLoaded event listener for link handling
- `src-ts/helpers.ts` - Helper functions including RE_ALL_BOOK_SUTTA_REF regex, extract_sutta_uid_from_link(), handle_link_click(), open_sutta_by_uid(), and show_external_link_confirmation()
- `assets/js/simsapa.min.js` - Compiled TypeScript output loaded in HTML pages
- `bridges/src/api.rs` - Backend API with new `/open_sutta/<uid>` endpoint and updated callback signature
- `cpp/gui.h` - Updated callback_open_sutta_search_window() signature with QString sutta_query parameter
- `cpp/gui.cpp` - Updated implementation that passes sutta_query to window and invokes handle_query()
- `cpp/sutta_search_window.h` - Header for SuttaSearchWindow with m_root member
- `cpp/window_manager.h` - Window manager header for sutta search window creation
- `backend/src/helpers.rs` - Contains RE_ALL_BOOK_SUTTA_REF regex pattern (reference only)
- `bridges/src/sutta_bridge.rs` - Updated get_word_html() to include simsapa.min.js script and open_sutta_search_window() with empty string parameter
- `assets/qml/SuttaSearchWindow.qml` - QML window with `handle_query()` function (already exists)
- `assets/qml/com/profoundlabs/simsapa/SuttaBridge.qml` - QML type definition for qmllint (already has open_sutta_search_window)
- `assets/templates/page.html` - HTML template that includes simsapa.min.js (reference only)
- `webpack.config.js` - TypeScript build configuration (unchanged)

### Notes

- The TypeScript code in `src-ts/` is built using `npx webpack` which outputs to `assets/js/simsapa.min.js`
- QML already has the `handle_query()` function at `assets/qml/SuttaSearchWindow.qml:149` which accepts `uid:` format queries
- The pattern follows the existing `callback_run_summary_query()` implementation in `bridges/src/api.rs:110`
- No unit tests required - functionality will be tested manually by clicking links in HTML content

## Tasks

- [x] 1.0 Implement TypeScript Link Handler in simsapa.ts
  - [x] 1.1 Port `RE_ALL_BOOK_SUTTA_REF` regex from `backend/src/helpers.rs:25-27` to TypeScript in `src-ts/helpers.ts` with pattern `/\b(DN|MN|SN|AN|Pv|Vv|Vism|iti|kp|khp|snp|th|thag|thig|ud|uda|dhp)[ \.]*(\d[\d\.:]*)\b/i`
  - [x] 1.2 Create helper function `extract_sutta_uid_from_link()` in `src-ts/helpers.ts` that takes an anchor element and returns sutta UID or null based on priority: ssp:// protocol → thebuddhaswords.net URL → text-based reference
  - [x] 1.3 Implement `handle_link_click()` event handler in `src-ts/helpers.ts` that classifies links (anchor links get default behavior, sutta links call API, external links show confirmation)
  - [x] 1.4 Create `open_sutta_by_uid()` async function in `src-ts/helpers.ts` that makes GET request to `${API_URL}/open_sutta/${uid}` with error handling
  - [x] 1.5 Create `show_external_link_confirmation()` function in `src-ts/helpers.ts` that uses browser confirm dialog with message "Open this link in your web browser?" and URL display
  - [x] 1.6 Add DOMContentLoaded event listener in `src-ts/simsapa.ts` that queries all `<a>` elements and attaches `handle_link_click` to each
  - [x] 1.7 Export the link handler functions from `src-ts/helpers.ts` for use in simsapa.ts
  - [x] 1.8 Run `npx webpack` to build TypeScript to `assets/js/simsapa.min.js` and verify no compilation errors

- [x] 2.0 Update C++ Callback Signature and Implementation
  - [x] 2.1 In `cpp/gui.h:12`, change callback signature from `void callback_open_sutta_search_window();` to `void callback_open_sutta_search_window(QString sutta_query = "");`
  - [x] 2.2 In `cpp/gui.cpp:61`, update `callback_open_sutta_search_window()` function signature to accept `QString sutta_query` parameter
  - [x] 2.3 In `cpp/gui.cpp:62`, modify implementation to call `AppGlobals::manager->create_sutta_search_window()` and store the returned `SuttaSearchWindow*` pointer
  - [x] 2.4 In `cpp/gui.cpp`, after window creation, add conditional check: if `sutta_query` is not empty, invoke `handle_query` on the window's `m_root` using `QMetaObject::invokeMethod(w->m_root, "handle_query", Q_ARG(QString, sutta_query));`
  - [x] 2.5 Run `make build -B` to verify C++ compilation succeeds with no errors

- [x] 3.0 Implement Backend API Endpoint with Fallback Logic
  - [x] 3.1 In `bridges/src/api.rs:51`, update CXX bridge declaration to `fn callback_open_sutta_search_window(sutta_query: QString);` to match new C++ signature
  - [x] 3.2 In `bridges/src/api.rs`, create new GET endpoint function `open_sutta(uid: PathBuf, dbm: &State<Arc<DbManager>>) -> Status` with route annotation `#[get("/open_sutta/<uid..>")]`
  - [x] 3.3 In `open_sutta()` endpoint, convert PathBuf to string: `let uid_str = uid.to_string_lossy().to_string();`
  - [x] 3.4 Implement sutta existence check: `let sutta_exists = dbm.appdata.get_sutta(&uid_str).is_some();`
  - [x] 3.5 Implement fallback logic: if sutta not found and UID doesn't end with `/pli/ms`, extract code and try `{code}/pli/ms` format; if found, use fallback UID with info log, otherwise use original UID
  - [x] 3.6 Construct query string: `let sutta_query = format!("uid:{}", final_uid);` and call `ffi::callback_open_sutta_search_window(ffi::QString::from(sutta_query));`
  - [x] 3.7 Return `Status::Ok` from endpoint
  - [x] 3.8 In `bridges/src/api.rs:214`, add `open_sutta` route to the `.mount("/", routes![...])` call
  - [x] 3.9 In `bridges/src/sutta_bridge.rs:921-924`, update `open_sutta_search_window()` to call `ffi::callback_open_sutta_search_window(QString::from(""));` with empty string parameter
  - [x] 3.10 In `assets/qml/com/profoundlabs/simsapa/SuttaBridge.qml`, add function signature `function open_sutta_search_window() {}` for qmllint type checking
  - [x] 3.11 Run `make build -B` to verify Rust bridge compilation and C++ integration compiles successfully

- [x] 4.0 Add Script Loading to Dictionary Pages
  - [x] 4.1 In `bridges/src/sutta_bridge.rs`, locate `get_word_html()` function around line 525
  - [x] 4.2 Before the closing `</body>` tag insertion, add script tag: `<script src="{}/assets/js/simsapa.min.js"></script>` using the `api_url` variable from `app_data.api_url`
  - [x] 4.3 Ensure the script is inserted after the existing dictionary JavaScript in the `</head>` section but before `</body>` to match the pattern in `page.html:32`
  - [x] 4.4 Run `make build -B` to verify compilation succeeds

- [ ] 5.0 Build, Test, and Verify Link Handling
  - [ ] 5.1 Run `npx webpack` to ensure TypeScript builds cleanly to `assets/js/simsapa.min.js`
  - [ ] 5.2 Run `make build -B` to compile the full application (C++, Rust, and QML components)
  - [ ] 5.3 Run `make run` to launch the application
  - [ ] 5.4 Test anchor links: Navigate to a sutta with in-page anchor links, click one, verify it jumps to the section without opening external browser
  - [ ] 5.5 Test ssp:// protocol links: Find or create HTML content with `ssp://suttas/sn47.8/en/thanissaro` link, click it, verify new sutta window opens with correct sutta
  - [ ] 5.6 Test thebuddhaswords.net links: Find or create HTML with link like `https://thebuddhaswords.net/suttas/an4.41.html`, click it, verify sutta `an4.41/pli/ms` opens in new window
  - [ ] 5.7 Test text-based sutta references: Find or create HTML with link text like "SN 56.11" or "MN 10", click it, verify correct sutta opens with `/pli/ms` fallback
  - [ ] 5.8 Test external link confirmation: Click a regular external link (e.g., to wikipedia.org), verify confirmation dialog appears with URL and "Open"/"Cancel" buttons
  - [ ] 5.9 Test language fallback: Try to open a sutta with specific translation that doesn't exist (e.g., `sn47.8/de/translator`), verify it falls back to `/pli/ms` version if available
  - [ ] 5.10 Test sutta not found: Try to open a sutta UID that doesn't exist in database, verify appropriate handling (window opens with empty results or error message)
  - [ ] 5.11 Verify link handling works in dictionary pages: Open a dictionary word that contains links, verify link handler is active and working
  - [ ] 5.12 Check browser console logs using developer tools to ensure no JavaScript errors appear during link handling
