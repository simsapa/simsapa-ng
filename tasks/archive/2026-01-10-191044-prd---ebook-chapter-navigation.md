# PRD: Ebook Chapter Navigation Buttons

## Introduction/Overview

When users are reading ebook chapters (book_spine_items) in the Simsapa app, they currently need to navigate back to the chapter list to move between chapters. This PRD describes a feature to add "Previous Chapter" and "Next Chapter" navigation buttons directly in the reading view, allowing users to move seamlessly between chapters without leaving the reading interface.

**Problem:** Users reading ebooks lack quick navigation controls to move between consecutive chapters, requiring them to return to the chapter list for navigation.

**Goal:** Provide intuitive, always-visible navigation controls for ebook chapters that integrate smoothly with the existing reading interface.

## Goals

1. Enable one-click navigation between consecutive ebook chapters without leaving the reading view
2. Maintain consistent visual design with existing UI controls (matching {text_resize_html} button style)
3. Clearly distinguish between ebook chapter view (where buttons are visible) and sutta page view (where buttons are hidden)
4. Provide clear visual feedback about navigation availability (disabled state at book boundaries)

## User Stories

1. **As an ebook reader**, I want to click "Next Chapter" to advance to the following chapter, so that I can continue reading without interrupting my flow.

2. **As an ebook reader**, I want to click "Previous Chapter" to go back to the prior chapter, so that I can review content I just read.

3. **As an ebook reader**, I want to see when I've reached the first or last chapter of a book (via disabled button state), so that I know the book boundaries.

4. **As a sutta reader**, I want the chapter navigation buttons to be hidden when viewing suttas, so that the interface remains clean and relevant to the content type.

## Functional Requirements

1. **Button Creation and Templating**
   - Create a new template placeholder `{prev_next_chapter}` that renders two buttons: "Previous Chapter" and "Next Chapter"
   - These buttons should be inserted in page.html template immediately after `{reading_mode_html}`

2. **Visual Design and Layout**
   - Position all three icons (`{reading_mode_html}`, Previous Chapter, Next Chapter) in a horizontal row on the left side of the page
   - Apply the same spacing between these three buttons as exists between `{text_resize_html}` and `{menu_html}` on the right side
   - Style the prev/next buttons to match the general appearance of `{text_resize_html}` buttons (consistent with existing UI design)

3. **Visibility Control**
   - Show prev/next buttons **only** when displaying a book_spine_item (ebook chapter)
   - Hide prev/next buttons when displaying sutta pages
   - The buttons should be part of the rendered template, with visibility controlled by the content type

4. **Button State Management**
   - When on the first chapter of a book, the "Previous Chapter" button should be **visible but disabled** (grayed out)
   - When on the last chapter of a book, the "Next Chapter" button should be **visible but disabled** (grayed out)
   - When chapters exist in both directions, both buttons should be active/enabled

5. **API Integration**
   - Clicking "Previous Chapter" or "Next Chapter" should send an HTTP request to the localhost API (see api.rs)
   - The API should determine the appropriate prev/next book_spine_item based on the current chapter
   - The API response should trigger appropriate signals/events for navigation

6. **Window Integration**
   - After the API processes the request, a signal should be sent to SuttaSearchWindow
   - SuttaSearchWindow should execute a QML function to load the requested book_spine_item in the HTML sutta view
   - The loading behavior should match the existing navigation flow when users click prev/next chapter titles in ChapterListItem.qml

7. **No Loading State Indicator**
   - No special loading spinner or visual feedback is required during chapter load
   - Users will wait for the new chapter to appear (consistent with current navigation behavior)

## Non-Goals (Out of Scope)

1. **Keyboard shortcuts** for chapter navigation (may be added in future)
2. **Touch gestures** (swipe left/right) for chapter navigation
3. **Chapter progress indicators** showing position within the book
4. **Quick jump** to arbitrary chapters (e.g., chapter selector dropdown)
5. **Cross-book navigation** (jumping to chapters in different books)
6. **Animation or transitions** during chapter load
7. **Preloading** of adjacent chapters for faster navigation
8. **Reading position preservation** within a chapter (scrolling to same relative position)

## Design Considerations

### UI/UX Requirements

1. **Icon Selection**: Choose appropriate icons for Previous/Next that clearly indicate direction (e.g., chevron-left, chevron-right, or arrow icons)

2. **Spacing**: Maintain visual balance between left-side controls (reading mode + navigation) and right-side controls (text resize + menu)

3. **Disabled State Styling**: Ensure disabled buttons are clearly distinguishable from active buttons (typically lower opacity or gray color)

4. **Responsive Behavior**: Consider how these buttons should behave on different screen sizes (mobile, tablet, desktop)

### Existing Components to Reference

- **{reading_mode_html}**: Study how this button interacts with the UI, its styling, and event handling
- **{text_resize_html}**: Match the button style, size, and visual appearance
- **ChapterListItem.qml**: Understand the existing click behavior for chapter navigation to replicate the same loading flow

## Technical Considerations

### Existing Navigation Flow (Investigated)

**ChapterListItem.qml → SuttaSearchWindow Navigation:**

1. User clicks chapter in ChapterListItem.qml
2. ChapterListItem emits signal: `chapter_clicked(window_id, spine_item_uid, title, anchor)`
3. BooksList.qml handles signal via `onChapter_clicked`:
   ```qml
   onChapter_clicked: (window_id, spine_item_uid, title, anchor) => {
       const result_data = {
           item_uid: spine_item_uid,
           table_name: "book_spine_items",
           sutta_title: title,
           sutta_ref: "",
           anchor: anchor
       };
       SuttaBridge.emit_show_chapter_from_library(window_id, JSON.stringify(result_data));
   }
   ```
4. SuttaBridge (sutta_bridge.rs:572) calls C++ callback:
   ```rust
   pub fn emit_show_chapter_from_library(self: Pin<&mut Self>, window_id: QString, result_data_json: QString) {
       ffi::callback_show_chapter_in_sutta_window(window_id, result_data_json);
   }
   ```
5. SuttaSearchWindow.qml receives signal via `onShowChapterFromLibrary`:
   ```qml
   function onShowChapterFromLibrary(window_id: string, result_data_json: string) {
       if (window_id === "" || window_id === root.window_id) {
           root.show_result_in_html_view_with_json(result_data_json);
       }
   }
   ```

**Reading Mode Button HTTP Flow (for reference):**

1. JavaScript (suttas.js:256-295) handles button click
2. Sends fetch request: `${API_URL}/toggle_reading_mode/${WINDOW_ID}/true`
3. api.rs:198-204 handles endpoint:
   ```rust
   #[get("/toggle_reading_mode/<window_id>/<is_active>")]
   fn toggle_reading_mode(window_id: &str, is_active: &str) -> Status {
       let active = is_active == "true";
       ffi::callback_toggle_reading_mode(ffi::QString::from(window_id), active);
       Status::Ok
   }
   ```
4. Callback triggers QML to hide/show search UI

### Database Schema

**book_spine_items table** (appdata_schema.rs:104-118):
```rust
book_spine_items (id) {
    id -> Integer,
    book_id -> Integer,
    book_uid -> Text,
    spine_item_uid -> Text,
    spine_index -> Integer,      // ← Used for ordering chapters
    resource_path -> Text,
    title -> Nullable<Text>,
    language -> Nullable<Text>,
    content_html -> Nullable<Text>,
    content_plain -> Nullable<Text>,
}
```

**Existing database functions** (appdata.rs):
- `get_book_spine_item(&self, spine_item_uid_param: &str)` - Get by UID
- `get_book_spine_item_by_path(&self, book_uid_param: &str, resource_path_param: &str)` - Get by path
- **No existing prev/next functions** - will need to be created

### Template System Architecture

**Template Rendering** (html_content.rs:1-116):

1. Templates loaded as static strings:
   ```rust
   static PAGE_HTML: &'static str = include_str!("../../assets/templates/page.html");
   static READING_MODE_HTML: &'static str = include_str!("../../assets/templates/reading_mode.html");
   static TEXT_RESIZE_HTML: &'static str = include_str!("../../assets/templates/text_resize.html");
   ```

2. TmplContext struct defines all placeholders:
   ```rust
   struct TmplContext {
       reading_mode_html: String,
       find_html: String,
       text_resize_html: String,
       menu_html: String,
       // ... need to add prev_next_chapter_html
   }
   ```

3. Templates populated in `Default::default()`:
   ```rust
   reading_mode_html: READING_MODE_HTML.replace("{api_url}", &g.api_url).to_string(),
   ```

4. page.html structure (page.html:14-28):
   ```html
   {reading_mode_html}
   <!-- NEW: {prev_next_chapter_html} goes here -->
   {find_html}
   {text_resize_html}
   {menu_html}
   ```

### CSS Styling Reference

**Button Base Styles** (suttas.css:1105-1140):
- Class: `.find-search-button`
- Size: 30px × 30px
- Position: `position: fixed; top: 10px;`
- Background: Gradient with hover/active states
- Border: 1px solid with rounded corners
- z-index: 1000

**Button Positions**:
- readingModeButton: `left: 10px`
- textSizeDecreaseButton: `right: 75px`
- textSizeIncreaseButton: `right: 45px`
- Menu button: `right: 10px`

**Spacing Pattern**: 30px buttons + ~5px gap = ~35px between buttons

**New buttons should use**:
- prevChapterButton: `left: 45px` (35px after reading mode)
- nextChapterButton: `left: 80px` (35px after prev button)

**Disabled state** (existing pattern from .active state):
```css
.chapter-nav-button:disabled {
    opacity: 0.4;
    cursor: not-allowed;
    /* Keep same visual style but muted */
}
```

### Implementation Plan

**1. Backend (Rust)**:

a. Add database functions in `backend/src/db/appdata.rs`:
   ```rust
   pub fn get_prev_book_spine_item(&self, spine_item_uid: &str) -> Result<Option<BookSpineItem>>
   pub fn get_next_book_spine_item(&self, spine_item_uid: &str) -> Result<Option<BookSpineItem>>
   ```
   - Query by spine_index - 1 or + 1 within same book_uid
   - Return None if at boundaries

b. Add API endpoints in `bridges/src/api.rs`:
   ```rust
   #[get("/prev_chapter/<window_id>/<current_spine_item_uid..>")]
   fn prev_chapter(window_id: &str, current_spine_item_uid: PathBuf, dbm: &State<Arc<DbManager>>) -> Status

   #[get("/next_chapter/<window_id>/<current_spine_item_uid..>")]
   fn next_chapter(window_id: &str, current_spine_item_uid: PathBuf, dbm: &State<Arc<DbManager>>) -> Status
   ```
   - Get prev/next spine item from database
   - Compose result_data JSON (same format as ChapterListItem)
   - Call `ffi::callback_show_chapter_in_sutta_window(window_id, json)`
   - Return Status::Ok or Status::NotFound

c. Update `html_content.rs`:
   - Add `PREV_NEXT_CHAPTER_HTML` static string
   - Add `prev_next_chapter_html` field to TmplContext
   - Conditionally populate based on content type (empty for suttas)

**2. Frontend (HTML/CSS/JS)**:

a. Create `assets/templates/prev_next_chapter.html`:
   ```html
   <button class="find-search-button chapter-nav-button" id="prevChapterButton"
           data-spine-item-uid="{current_spine_item_uid}"
           data-book-uid="{current_book_uid}"
           data-is-first="{is_first_chapter}"
           title="Previous chapter">
       <img src="{api_url}/assets/icons/32x32/fa_chevron-left-solid.png"
            alt="Previous" class="chapter-nav-icon">
   </button>
   <button class="find-search-button chapter-nav-button" id="nextChapterButton"
           data-spine-item-uid="{current_spine_item_uid}"
           data-book-uid="{current_book_uid}"
           data-is-last="{is_last_chapter}"
           title="Next chapter">
       <img src="{api_url}/assets/icons/32x32/fa_chevron-right-solid.png"
            alt="Next" class="chapter-nav-icon">
   </button>
   ```

b. Add CSS in `assets/css/suttas.css`:
   ```css
   #prevChapterButton {
       position: fixed;
       top: 10px;
       left: 45px;
       z-index: 1000;
   }

   #nextChapterButton {
       position: fixed;
       top: 10px;
       left: 80px;
       z-index: 1000;
   }

   .chapter-nav-button .chapter-nav-icon {
       width: 15px;
       height: 15px;
       opacity: 0.8;
   }

   .chapter-nav-button:disabled {
       opacity: 0.4;
       cursor: not-allowed;
   }
   ```

c. Add JavaScript in `assets/js/suttas.js`:
   ```javascript
   class ChapterNavigationController {
       constructor() {
           this.prevButton = document.getElementById('prevChapterButton');
           this.nextButton = document.getElementById('nextChapterButton');
           this.init();
       }

       init() {
           if (!this.prevButton || !this.nextButton) return;

           // Set initial disabled state
           if (this.prevButton.dataset.isFirst === 'true') {
               this.prevButton.disabled = true;
           }
           if (this.nextButton.dataset.isLast === 'true') {
               this.nextButton.disabled = true;
           }

           this.prevButton.addEventListener('click', () => this.navigatePrev());
           this.nextButton.addEventListener('click', () => this.navigateNext());
       }

       async navigatePrev() {
           const spineItemUid = this.prevButton.dataset.spineItemUid;
           try {
               await fetch(`${API_URL}/prev_chapter/${WINDOW_ID}/${spineItemUid}`);
           } catch (error) {
               log_error('Failed to navigate to previous chapter: ' + error);
           }
       }

       async navigateNext() {
           const spineItemUid = this.nextButton.dataset.spineItemUid;
           try {
               await fetch(`${API_URL}/next_chapter/${WINDOW_ID}/${spineItemUid}`);
           } catch (error) {
               log_error('Failed to navigate to next chapter: ' + error);
           }
       }
   }

   // Initialize on page load
   new ChapterNavigationController();
   ```

**3. Template Population Logic**:

The `render_book_spine_item_html()` function needs to:
1. Get current spine_item
2. Query prev/next items to determine is_first/is_last
3. Populate template with current_spine_item_uid, current_book_uid, is_first_chapter, is_last_chapter
4. For sutta pages, set `prev_next_chapter_html` to empty string

### Technical Constraints

1. **Template System**: Must add new placeholder to TmplContext struct and populate in html_content.rs
2. **Bridge Communication**: Reuses existing `callback_show_chapter_in_sutta_window` - no new bridge functions needed
3. **State Tracking**: Backend queries database to determine prev/next availability using spine_index
4. **Content Type Detection**: Template rendering distinguishes book_spine_items vs suttas by checking table_name

### Files to Modify

**Rust Backend**:
- `backend/src/db/appdata.rs` - Add get_prev/next_book_spine_item functions
- `backend/src/html_content.rs` - Add prev_next_chapter_html to TmplContext
- `backend/src/app_data.rs` - Update render_book_spine_item_html to populate navigation state
- `bridges/src/api.rs` - Add /prev_chapter and /next_chapter endpoints

**Frontend Assets**:
- `assets/templates/prev_next_chapter.html` - New template file
- `assets/templates/page.html` - Add {prev_next_chapter_html} placeholder
- `assets/css/suttas.css` - Add button positioning and styling
- `assets/js/suttas.js` - Add ChapterNavigationController class

**Build Configuration**:
- `bridges/build.rs` - Add new template to qml_files list (if using QML type definitions)

## Success Metrics

1. **Functionality**: Users can successfully navigate to previous/next chapters using the buttons without errors
2. **Consistency**: The navigation behavior exactly matches clicking chapter titles in ChapterListItem.qml
3. **Visibility**: Buttons appear only for ebook chapters and are correctly hidden for sutta pages
4. **State Management**: Buttons are correctly disabled at book boundaries (first/last chapters)
5. **User Adoption**: Track usage of the new buttons vs. manual chapter list navigation (if analytics available)

## Open Questions

1. **~~Chapter Order Logic~~**: ✓ **RESOLVED** - Chapters are ordered by `spine_index` (Integer) field in book_spine_items table

2. **~~Multi-Book Collections~~**: ✓ **RESOLVED** - Not a concern. Navigation stops at book boundaries (when book_uid changes).

3. **~~Error Handling~~**: ✓ **RESOLVED** - Log errors to console using existing log_error function. No user-facing error messages.

4. **~~Icons~~**: ✓ **RESOLVED** - Use existing Font Awesome icons:
   - Previous: `fa_chevron-left-solid.png`
   - Next: `fa_chevron-right-solid.png`
   - Located in `assets/icons/32x32/`

5. **~~Accessibility~~**: ✓ **RESOLVED** - Add aria-label attributes to buttons for screen readers.

6. **Mobile Considerations**: Are there any special considerations for mobile devices (touch targets, spacing)?
   - **Recommendation**: Keep 30px button size (adequate touch target). On very small screens, may need media query adjustments, but test first.

7. **~~Performance~~**: ✓ **RESOLVED** - Not a concern. API is localhost, database queries are fast. No optimistic UI updates needed.
