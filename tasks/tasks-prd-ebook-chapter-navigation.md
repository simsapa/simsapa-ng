# Tasks: Ebook Chapter Navigation Buttons

Generated from: `prd-ebook-chapter-navigation.md`

## Relevant Files

- `backend/src/db/appdata.rs` - ✅ Added database query functions get_prev_book_spine_item() and get_next_book_spine_item()
- `backend/src/db/appdata_models.rs` - BookSpineItem model (already exists, used for queries)
- `backend/src/db/appdata_schema.rs` - Database schema (reference for book_spine_items table with spine_index)
- `bridges/src/api.rs` - ✅ Added API endpoints prev_chapter() and next_chapter(), registered in routes
- `backend/src/html_content.rs` - ✅ Added PREV_NEXT_CHAPTER_HTML template, prev_next_chapter_html field to TmplContext, created sutta_html_page_with_nav()
- `backend/src/app_data.rs` - ✅ Updated render_book_spine_item_html() to query navigation state and populate template
- `assets/templates/prev_next_chapter.html` - ✅ Created new template file with prev/next buttons
- `assets/templates/page.html` - ✅ Added {prev_next_chapter_html} placeholder after {reading_mode_html}
- `assets/css/suttas.css` - ✅ Added CSS positioning (#prevChapterButton at left: 45px, #nextChapterButton at left: 80px) and styling
- `assets/js/suttas.js` - ✅ Added ChapterNavigationController class, initialized in DOMContentLoaded, handles click events with fetch API
- `bridges/build.rs` - Register new template in build configuration (if needed)

### Notes

- The implementation reuses the existing navigation flow: API endpoint → C++ callback → QML signal
- No new bridge functions needed - reuses `callback_show_chapter_in_sutta_window`
- Buttons should only appear for book_spine_items, not for sutta pages
- Testing should verify: button visibility, disabled states, navigation functionality
- Use `make build -B` to rebuild the application
- Use `make test` to run all tests after implementation

## Tasks

- [x] 1.0 Implement Backend Database Functions for Chapter Navigation
  - [x] 1.1 Open `backend/src/db/appdata.rs` and locate existing `get_book_spine_item` functions to understand the query patterns
  - [x] 1.2 Implement `get_prev_book_spine_item(&self, spine_item_uid: &str) -> Result<Option<BookSpineItem>>` function:
    - Get current spine item to obtain book_uid and spine_index
    - Query for spine item with same book_uid and spine_index - 1
    - Return None if not found (at first chapter boundary)
  - [x] 1.3 Implement `get_next_book_spine_item(&self, spine_item_uid: &str) -> Result<Option<BookSpineItem>>` function:
    - Get current spine item to obtain book_uid and spine_index
    - Query for spine item with same book_uid and spine_index + 1
    - Return None if not found (at last chapter boundary)
  - [x] 1.4 Add proper error handling using `anyhow::Result`
  - [x] 1.5 Run `cd backend && cargo test` to ensure no compilation errors

- [x] 2.0 Create API Endpoints for Prev/Next Chapter Navigation
  - [x] 2.1 Open `bridges/src/api.rs` and locate the existing `toggle_reading_mode` endpoint (line 198) as a reference pattern
  - [x] 2.2 Implement `prev_chapter` endpoint function:
    ```rust
    #[get("/prev_chapter/<window_id>/<current_spine_item_uid..>")]
    fn prev_chapter(window_id: &str, current_spine_item_uid: PathBuf, dbm: &State<Arc<DbManager>>) -> Status
    ```
    - **IMPORTANT**: window_id parameter identifies which SuttaSearchWindow should receive the navigation signal
    - Convert PathBuf to string using `pathbuf_to_forward_slash_string`
    - Call `dbm.appdata.get_prev_book_spine_item(&uid_str)`
    - If found, compose result_data JSON with fields: item_uid, table_name, sutta_title, sutta_ref, anchor
    - **Pass window_id to callback**: Call `ffi::callback_show_chapter_in_sutta_window(ffi::QString::from(window_id), json_string)`
    - Return Status::Ok or Status::NotFound
  - [x] 2.3 Implement `next_chapter` endpoint function following the same pattern as prev_chapter but calling `get_next_book_spine_item`:
    - **Must include window_id parameter** in the endpoint signature: `#[get("/next_chapter/<window_id>/<current_spine_item_uid..>")]`
    - Pass window_id to `ffi::callback_show_chapter_in_sutta_window(ffi::QString::from(window_id), json_string)`
  - [x] 2.4 Register both endpoints in the `routes![]` macro (around line 1004):
    ```rust
    prev_chapter,
    next_chapter,
    ```
  - [x] 2.5 Add logging calls using `info()` to log navigation requests
  - [x] 2.6 Run `make build -B` to verify compilation succeeds

- [x] 3.0 Create Navigation Button Template and Integrate into Template System
  - [x] 3.1 Create new file `assets/templates/prev_next_chapter.html` with two button elements:
    - Use class `find-search-button chapter-nav-button`
    - Add data attributes: `data-spine-item-uid="{current_spine_item_uid}"`, `data-book-uid="{current_book_uid}"`, `data-is-first="{is_first_chapter}"`, `data-is-last="{is_last_chapter}"`
    - Use chevron icons: `fa_chevron-left-solid.png` and `fa_chevron-right-solid.png`
    - Add aria-label attributes for accessibility
    - Set IDs: `prevChapterButton` and `nextChapterButton`
  - [x] 3.2 Open `backend/src/html_content.rs` and add static template string (after line 9):
    ```rust
    static PREV_NEXT_CHAPTER_HTML: &'static str = include_str!("../../assets/templates/prev_next_chapter.html");
    ```
  - [x] 3.3 Add `prev_next_chapter_html: String` field to `TmplContext` struct (after line 26)
  - [x] 3.4 Update `TmplContext::default()` implementation to initialize the new field (around line 41):
    ```rust
    prev_next_chapter_html: "".to_string(),  // Default to empty for suttas
    ```
  - [x] 3.5 Open `assets/templates/page.html` and add `{prev_next_chapter_html}` placeholder after `{reading_mode_html}` (after line 21)
  - [x] 3.6 Open `backend/src/app_data.rs` and locate `render_book_spine_item_html` function
  - [x] 3.7 In `render_book_spine_item_html`, query prev/next spine items to determine `is_first_chapter` and `is_last_chapter` boolean flags
  - [x] 3.8 Populate the prev_next_chapter template with placeholders: `{current_spine_item_uid}`, `{current_book_uid}`, `{is_first_chapter}`, `{is_last_chapter}`, `{api_url}`
    - **Note**: The template already includes WINDOW_ID as a global JavaScript variable (set in js_extra parameter)
  - [x] 3.9 For sutta rendering (in `render_sutta_content`), ensure `prev_next_chapter_html` remains empty string
  - [x] 3.10 Run `make build -B` to verify template integration compiles

- [x] 4.0 Implement Frontend JavaScript Controller and Event Handlers
  - [x] 4.1 Open `assets/js/suttas.js` and locate the `ReadingModeController` class (line 254) as a reference pattern
  - [x] 4.2 Create new `ChapterNavigationController` class after `ReadingModeController`:
    - Add constructor that gets prevButton and nextButton by ID
    - Call `this.init()` from constructor
  - [x] 4.3 Implement `init()` method:
    - Check if buttons exist (return early if not)
    - Read `data-is-first` and `data-is-last` attributes
    - Set `disabled` property on buttons based on these flags
    - Add click event listeners calling `navigatePrev()` and `navigateNext()`
  - [x] 4.4 Implement `async navigatePrev()` method:
    - Read `data-spine-item-uid` from button
    - **IMPORTANT**: Use fetch API with window_id: `${API_URL}/prev_chapter/${WINDOW_ID}/${spineItemUid}`
    - WINDOW_ID is a global variable set in the page template (identifies which SuttaSearchWindow to update)
    - Catch errors and log with `log_error()`
  - [x] 4.5 Implement `async navigateNext()` method following same pattern as navigatePrev:
    - **Must include WINDOW_ID**: `${API_URL}/next_chapter/${WINDOW_ID}/${spineItemUid}`
  - [x] 4.6 Add initialization at end of file: `new ChapterNavigationController();`
  - [x] 4.7 Run `make build -B` to rebuild with updated JavaScript

- [x] 5.0 Add CSS Styling for Navigation Buttons
  - [x] 5.1 Open `assets/css/suttas.css` and locate the `#readingModeButton` styles (around line 1360)
  - [x] 5.2 Add `#prevChapterButton` positioning rule:
    ```css
    #prevChapterButton {
        position: fixed;
        top: 10px;
        left: 45px;
        z-index: 1000;
        transform: translateZ(0);
    }
    ```
  - [x] 5.3 Add `#nextChapterButton` positioning rule:
    ```css
    #nextChapterButton {
        position: fixed;
        top: 10px;
        left: 80px;
        z-index: 1000;
        transform: translateZ(0);
    }
    ```
  - [x] 5.4 Add icon styling:
    ```css
    .chapter-nav-button .chapter-nav-icon {
        width: 15px;
        height: 15px;
        opacity: 0.8;
    }
    ```
  - [x] 5.5 Add disabled state styling:
    ```css
    .chapter-nav-button:disabled {
        opacity: 0.4;
        cursor: not-allowed;
    }
    ```
  - [x] 5.6 Run `make sass` to compile Sass if needed, or verify CSS changes are picked up
  - [x] 5.7 Run `make build -B` to rebuild with updated CSS

- [ ] 6.0 Test Navigation Flow and Edge Cases
  - [ ] 6.1 Run `make build -B` to ensure clean build
  - [ ] 6.2 Start the application with `make run`
  - [ ] 6.3 Open a book in the Library and navigate to a chapter in the middle of the book
  - [ ] 6.4 Verify both prev and next buttons appear and are enabled
  - [ ] 6.5 Click "Next Chapter" and verify it navigates to the next chapter
  - [ ] 6.6 Click "Previous Chapter" and verify it navigates to the previous chapter
  - [ ] 6.7 Navigate to the first chapter of a book
  - [ ] 6.8 Verify "Previous Chapter" button is visible but disabled (grayed out)
  - [ ] 6.9 Verify "Next Chapter" button is enabled
  - [ ] 6.10 Navigate to the last chapter of a book
  - [ ] 6.11 Verify "Next Chapter" button is visible but disabled
  - [ ] 6.12 Verify "Previous Chapter" button is enabled
  - [ ] 6.13 Open a sutta (not a book chapter) from search results
  - [ ] 6.14 Verify navigation buttons do NOT appear for suttas
  - [ ] 6.15 Check browser console for any JavaScript errors during navigation
  - [ ] 6.16 Verify buttons are positioned correctly (35px spacing between reading mode, prev, and next)
  - [ ] 6.17 If any issues found, fix and repeat relevant test steps
