# PRD: Smart Link Handling in HTML Content

## Introduction/Overview

The Simsapa app displays HTML content that includes various types of links: sutta references, external websites, and in-page anchors. Currently, all external links open in a browser, which disrupts the user experience when the link is to a sutta that could be opened within the app. This feature will intelligently detect and handle sutta links by opening them internally in the app, while still requiring user confirmation for non-sutta external links.

**Problem:** When users click on sutta reference links in imported HTML content, they are taken to external websites instead of viewing the sutta within the app, which provides a poor user experience and defeats the purpose of having a local sutta reader.

**Goal:** Implement smart link detection and routing so that sutta links open within the app, anchor links continue to work as expected, and external links require user confirmation.

## Goals

1. Automatically detect and open sutta links within the app instead of in an external browser
2. Support multiple sutta link patterns from different sources (ssp:// protocol, thebuddhaswords.net URLs, and text-based references)
3. Maintain current behavior for anchor links (jumping to sections on the same page)
4. Protect users by requiring confirmation before opening external non-sutta links
5. Provide helpful error messages when a sutta reference cannot be found in the database
6. Implement intelligent fallback to Pāli text (`/pli/ms`) when a specific translation is not available, ensuring users can always access suttas in their canonical form

## User Stories

1. As a user, when I click on a sutta link in HTML content, I want it to open in a new window within the app so I can seamlessly navigate between related suttas.

2. As a user, when I click on an anchor link within a page, I want it to jump to that section of the current page without any interruption.

3. As a user, when I click on an external website link, I want to be asked for confirmation before opening it in my browser so I'm aware I'm leaving the app.

4. As a user, if a sutta reference doesn't exist in my database, I want to see a clear error message with the option to open the URL externally so I can still access the content if needed.

## Functional Requirements

### 1. TypeScript Link Handler Implementation

1.1. In `simsapa.ts`, add a `DOMContentLoaded` event listener that scans the DOM for all `<a>` links and attaches click event listeners to each one:

```typescript
document.addEventListener("DOMContentLoaded", function(_event) {
    const links = document.querySelectorAll('a');
    links.forEach(link => {
        link.addEventListener('click', handleLinkClick);
    });
});
```

1.2. Port the `RE_ALL_BOOK_SUTTA_REF` regex pattern from `backend/src/helpers.rs` (lines 25-27) to TypeScript:
```typescript
const RE_ALL_BOOK_SUTTA_REF = /\b(DN|MN|SN|AN|Pv|Vv|Vism|iti|kp|khp|snp|th|thag|thig|ud|uda|dhp)[ \.]*(\d[\d\.:]*)\b/i;
```

1.3. Implement the `handleLinkClick` function with link classification logic using the following priority order:
   - **First priority:** Anchor links (href starting with `#`) → Allow default behavior (don't prevent default)
   - **Second priority:** `ssp://` protocol links → Extract UID and open internally
   - **Third priority:** thebuddhaswords.net links → Extract sutta code from URL
   - **Fourth priority:** Link text matching `RE_ALL_BOOK_SUTTA_REF` → Extract sutta code from text
   - **Fifth priority:** All other links → Show external link confirmation dialog

1.4. Use `event.preventDefault()` to prevent default browser link behavior for all links except anchor links

### 2. Sutta Link Pattern Handling

2.1. **ssp:// Protocol Links**
   - Pattern: `ssp://suttas/{uid}` where uid is in format like `sn47.8/en/thanissaro`
   - Extract the UID portion after `ssp://suttas/`
   - Open sutta by making GET request to: `${API_URL}/open_sutta/${uid}`
   - Example: `GET http://localhost:5001/open_sutta/sn47.8/en/thanissaro`
   - Backend will attempt to find this specific translation, and fallback to `/pli/ms` if not found

2.2. **thebuddhaswords.net Links**
   - Pattern: `https://thebuddhaswords.net/{path}/{code}.html` where code is like `an4.41`
   - Extract the sutta code from the filename (before `.html`)
   - Construct UID in format `{code}/pli/ms` (e.g., `an4.41/pli/ms`)
   - Open sutta by making GET request to: `${API_URL}/open_sutta/${uid}`
   - Example: `GET http://localhost:5001/open_sutta/an4.41/pli/ms`
   - Since this is already `/pli/ms`, no fallback is needed

2.3. **Text-based Sutta References**
   - Match link text against `RE_ALL_BOOK_SUTTA_REF` pattern
   - Extract the nikaya abbreviation and number (e.g., "SN 56.11" → "sn56.11")
   - Construct UID in format `{code}/pli/ms` (e.g., `sn56.11/pli/ms`)
   - Open sutta by making GET request to: `${API_URL}/open_sutta/${uid}`
   - Example: `GET http://localhost:5001/open_sutta/sn56.11/pli/ms`
   - Since this is already `/pli/ms`, no fallback is needed

2.4. **TypeScript API Request Implementation**
   - Use `fetch()` API to make the GET request
   - Handle errors gracefully (network errors, 404 responses, etc.)
   - Log API calls using the existing logger endpoint for debugging
   - Example implementation:
     ```typescript
     async function openSuttaByUid(uid: string) {
         try {
             const response = await fetch(`${API_URL}/open_sutta/${uid}`);
             if (!response.ok) {
                 // Handle error - show "sutta not found" dialog
             }
         } catch (error) {
             console.error('Failed to open sutta:', error);
         }
     }
     ```

### 3. Backend API Endpoint and Callback Implementation

**Implementation follows the pattern of `callback_run_summary_query()` as the reference example.**

3.1. **Modify C++ callback in `cpp/gui.h`:**
   - Change signature from: `void callback_open_sutta_search_window();`
   - To: `void callback_open_sutta_search_window(QString sutta_query = "");`
   - This matches the pattern of `callback_run_summary_query(QString window_id, QString query_text)`

3.2. **Modify C++ callback implementation in `cpp/gui.cpp`:**
   - Update `callback_open_sutta_search_window()` to accept the `sutta_query` parameter
   - Pass it to the WindowManager's window creation or emit it via a signal
   - Follow the pattern of lines 53-55 where `callback_run_summary_query()` emits `signal_run_summary_query`

3.3. **Update WindowManager in `cpp/window_manager.h`:**
   - Modify `create_sutta_search_window()` to accept an optional `QString sutta_query` parameter
   - Or add a new signal/slot pair to handle the query after window creation
   - Follow the pattern of `signal_run_summary_query` (line 40) and `run_summary_query` (lines 45-46)

3.4. **Update WindowManager implementation in `cpp/window_manager.cpp`:**
   - In `create_sutta_search_window()` (line 52), after creating the window and setting the window_id
   - If `sutta_query` is provided, invoke `handle_query` on the window's QML root object
   - Use `QMetaObject::invokeMethod` pattern similar to line 101:
     ```cpp
     QMetaObject::invokeMethod(w->m_root, "handle_query", Q_ARG(QString, sutta_query));
     ```

3.5. **Update Rust bridge in `bridges/src/api.rs`:**
   - Modify the CXX bridge declaration (line 51) to match the new signature:
     ```rust
     fn callback_open_sutta_search_window(sutta_query: QString);
     ```
   - Create a new GET endpoint with fallback logic:
     ```rust
     #[get("/open_sutta/<uid..>")]
     fn open_sutta(uid: PathBuf, dbm: &State<Arc<DbManager>>) -> Status {
         let uid_str = uid.to_string_lossy().to_string();
         
         // Check if the sutta exists in the database
         let sutta_exists = dbm.appdata.get_sutta(&uid_str).is_some();
         
         let final_uid = if sutta_exists {
             // Sutta found with the requested UID
             uid_str
         } else {
             // Sutta not found - check if this is a non-pli/ms request
             if uid_str.ends_with("/pli/ms") {
                 // Already pli/ms, sutta truly doesn't exist
                 uid_str
             } else {
                 // Try to extract the code and fallback to pli/ms
                 let parts: Vec<&str> = uid_str.split('/').collect();
                 if parts.len() >= 3 {
                     let code = parts[0];
                     let fallback_uid = format!("{}/pli/ms", code);
                     
                     // Check if pli/ms version exists
                     if dbm.appdata.get_sutta(&fallback_uid).is_some() {
                         info(&format!("Sutta {} not found, using fallback: {}", uid_str, fallback_uid));
                         fallback_uid
                     } else {
                         // Fallback also doesn't exist
                         uid_str
                     }
                 } else {
                     // Invalid UID format
                     uid_str
                 }
             }
         };
         
         let sutta_query = format!("uid:{}", final_uid);
         ffi::callback_open_sutta_search_window(ffi::QString::from(sutta_query));
         Status::Ok
     }
     ```
   - Add the new route to the `mount` call in `start_webserver()` (around line 214)

3.6. **Update SuttaBridge Rust binding in `bridges/src/sutta_bridge.rs`:**
   - Update the `open_sutta_search_window()` method (line 921) to call the updated callback:
     ```rust
     pub fn open_sutta_search_window(&self) {
         use crate::api::ffi;
         ffi::callback_open_sutta_search_window(QString::from(""));
     }
     ```

3.7. **SuttaSearchWindow.qml already has the `handle_query` function** (line 149)
   - It accepts a `query_text_orig` parameter
   - When the query format is `uid:{sutta_uid}`, the function will:
     - Convert it using `SuttaBridge.query_text_to_uid_field_query()` (line 157)
     - Set the search mode to 'Uid Match' (line 160)
     - Execute the search (line 184)
   - No changes needed to the QML file

### 4. External Link Confirmation Dialog

4.1. Show a confirmation dialog with:
   - The full URL being opened
   - Warning message: "Open this link in your web browser?"
   - Two buttons: "Open" and "Cancel"

4.2. Only open the external link if user clicks "Open"

### 5. Error Handling and Language Fallback

5.1. **Language Fallback Logic (implemented in backend `open_sutta` endpoint):**
   - If the requested UID ends with `/pli/ms`, consider it the canonical Pāli text
   - If `/pli/ms` is not found, the sutta does not exist in the database
   - If the requested UID uses a different language/author (e.g., `sn56.11/en/thanissaro`):
     - First, try to find the exact UID requested
     - If not found, extract the sutta code (e.g., `sn56.11`)
     - Construct fallback UID as `{code}/pli/ms` (e.g., `sn56.11/pli/ms`)
     - Check if the fallback exists in the database
     - If fallback exists, use it; otherwise, proceed with original UID
   - This ensures users can always access the Pāli text even if a specific translation is unavailable

5.2. **Sutta Not Found Error Dialog:**
   - When a sutta UID is not found in the database (after fallback attempt):
     - Display a user-friendly error dialog
     - Message: "Sutta not found in database: {uid}"
     - Show the original URL that was clicked
     - Offer two options: "Open External Link" and "Cancel"
     - If user selects "Open External Link", open the original URL in a browser
   - This gives users a way to still access content if it's not in their local database

### 6. Script Loading for Dictionary Pages

6.1. Modify `sutta_bridge.rs::get_word_html()` (around line 525-589) to include `simsapa.min.js` script before the closing `</body>` tag, similar to how it's loaded in `page.html` (line 32)

6.2. Ensure the script has access to the `API_URL` constant that is already being injected into the page

## Non-Goals (Out of Scope)

1. Handling sutta links in non-HTML content (plain text, markdown, etc.)
2. Prefetching or preloading suttas for faster navigation
3. Back/forward navigation history management
4. Keyboard shortcuts for link navigation
5. Link preview tooltips showing sutta titles
6. Customizable link handling preferences in settings
7. Handling links to dictionary words or other content types
8. Supporting regex patterns for all PTS volume references (e.g., "M. III. 203")

## Design Considerations

### UI/UX

- Error dialogs should be modal and clearly visible
- Confirmation dialogs should be non-intrusive but noticeable
- No visual changes to existing link styling or appearance
- Loading states are handled by the existing sutta window opening mechanism

### TypeScript Implementation

- Use modern ES6+ syntax for consistency with existing TypeScript code
- Add comprehensive error handling with try-catch blocks
- Log all link handling actions to the backend logger for debugging
- Use TypeScript interfaces for type safety where appropriate

## Technical Considerations

### Dependencies

- No new dependencies required
- Uses existing Rocket framework for API endpoints
- Uses existing CXX-Qt bridge for C++/Rust communication
- Uses existing dialog mechanisms in QML

### Integration Points

The implementation follows the existing pattern used by `callback_run_summary_query()`:

1. **TypeScript → Backend API:** 
   - HTTP GET request to `/open_sutta/<uid>`
   - Example: `fetch('http://localhost:5001/open_sutta/sn56.11/pli/ms')`

2. **Backend API (Rust) → C++ callback:**
   - `bridges/src/api.rs::open_sutta()` extracts UID from URL path
   - Constructs query string: `uid:{sutta_uid}` (e.g., `uid:sn56.11/pli/ms`)
   - Calls `callback_open_sutta_search_window(sutta_query)`

3. **C++ callback → WindowManager:**
   - `cpp/gui.cpp::callback_open_sutta_search_window()` receives the query
   - Creates new SuttaSearchWindow via WindowManager
   - WindowManager invokes `handle_query()` on the QML window root

4. **QML → Backend:**
   - `SuttaSearchWindow.qml::handle_query()` receives `uid:sn56.11/pli/ms`
   - Recognizes UID format, sets search mode to 'Uid Match'
   - Calls `SuttaBridge.results_page()` to fetch sutta
   - Displays results in the window

### Browser Compatibility

- The WebEngine component used in both desktop and mobile versions should support the required JavaScript features
- All regex patterns should be tested to ensure they work in the WebEngine JavaScript environment

### Performance

- Link click handlers should execute synchronously to provide immediate feedback
- API calls for opening suttas should be asynchronous to avoid blocking the UI
- Regex matching should be efficient as it runs on every link click

## Success Metrics

1. **Functional Success:**
   - All three sutta link patterns (ssp://, thebuddhaswords.net, text references) successfully open suttas internally
   - External links show confirmation dialog before opening
   - Anchor links continue to work without interruption
   - Error dialogs appear when suttas are not found

2. **User Experience Success:**
   - Users can navigate between related suttas without leaving the app
   - No broken links or unexpected behaviors
   - Clear feedback for all link types

3. **Code Quality:**
   - TypeScript code passes linting (`qmllint`)
   - Rust code compiles without warnings
   - All link handling edge cases are covered with appropriate error handling

## Open Questions

1. Should we cache the regex pattern or recompile it on each link click?
   - **Decision:** Create pattern once as a module-level constant (best practice for performance)

2. Should the external link dialog include a "Don't ask again" checkbox?
   - **Decision:** No, keep it simple for MVP; can be added later if users request it

3. Should we handle relative URLs in imported HTML differently?
   - **Decision:** Relative URLs without a recognizable sutta pattern should be treated as external links

4. What should happen if multiple sutta patterns match in a single link?
   - **Answer:** Use the priority order specified in requirement 1.3

5. Should we support opening suttas in the same window versus always opening a new window?
   - **Decision:** Always open in a new window (matches current behavior of search results)

6. How should we handle the case where a sutta exists but the specific translation doesn't?
   - **Decision:** Implement automatic fallback to `/pli/ms` in the backend API endpoint
   - If requested UID with specific translation (e.g., `en/thanissaro`) is not found, fallback to `{code}/pli/ms`
   - If `/pli/ms` also doesn't exist, then show "sutta not found" error
   - Exception: If the original request was already for `/pli/ms`, no fallback is attempted

7. Should anchor links that also match sutta patterns be treated as anchors or suttas?
   - **Decision:** Anchor links (starting with `#`) take highest priority and are allowed default behavior
