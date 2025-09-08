# PRD: Ctrl+F Find Feature for Simsapa

## Introduction/Overview

This feature implements a browser-style "Ctrl+F" find functionality for the Simsapa sutta reader application. It allows users to search for text within the currently displayed content, with visual highlighting of matches and navigation between results. The feature addresses the need for quick text location within potentially long sutta documents, improving user experience and accessibility.

**Goal:** Enable users to quickly locate specific text within sutta content using familiar find-and-highlight functionality with support for Pāli/Sanskrit text characteristics.

A list of accented Pāli/Sanskrit characters and their latin equivalent:

```
let accented = ["ā","ī","ū","ṃ","ṁ","ṅ","ñ","ṭ","ḍ","ṇ","ḷ","ṛ","ṣ","ś"];
let latin = ["a","i","u","m","m","n","n","t","d","n","l","r","s","s"];
```

## Goals

1. **Text Search & Highlighting:** Implement real-time text search with visual highlighting of all matches
2. **Navigation:** Provide next/previous navigation through search results with wrap-around behavior
3. **Pāli Language Support:** Support accent folding for Pāli/Sanskrit characters (ā → [aā]) and case-sensitive options
4. **Accessibility:** Ensure keyboard shortcuts work as expected (Ctrl+F, Escape, Enter, Shift+Enter)
5. **Performance:** Handle large sutta documents with maximum 1000 result limit and 400ms debounced search
6. **Mobile Compatibility:** Maintain same functionality across desktop and mobile devices

## User Stories

1. **As a sutta reader**, I want to press Ctrl+F to open a find bar so that I can quickly search for specific terms within the current document.

2. **As a Pāli student**, I want the search to fold accents by default (finding "saṁsāra" when I type "samsara" in the search input field) so that I don't have to worry about exact diacritical marks.

3. **As a researcher**, I want to navigate between search results using previous/next buttons or keyboard shortcuts so that I can review all instances of a term.

4. **As a user**, I want to see a match counter ("3/15") so that I know how many results exist and which one I'm currently viewing.

5. **As a mobile user**, I want the find feature to work on my phone or tablet with the same functionality as desktop.

## Functional Requirements

1. **Find Bar UI:**
   - Clicking the search icon (magnifying glass) shows/hides the find bar
   - Find bar slides out from the search icon to the left when opened
   - Search icon remains in pressed state until find bar is closed
   - Find bar is hidden by default

2. **Search Input:**
   - Text input field for search terms
   - Real-time search starts after 400ms delay from last keystroke
   - Or search starts immediately when the user presses Enter
   - Search is performed within the `<div id="ssp_content">` element only

3. **Search Results Display:**
   - All matches highlighted with background color
   - Current match highlighted with different background color
   - First match becomes current match and is scrolled into view
   - Match counter displays "current/total" format

4. **Navigation Controls:**
   - "Find previous" button with "<" icon - moves to previous match with wrap-around
   - "Find next" button with ">" icon - moves to next match with wrap-around
   - Current match scrolled into view when navigated to

5. **Search Options:**
   - Checkbox for accent folding labeled "ā" (enabled by default)
   - Checkbox for case-sensitive search labeled "Aa" (disabled by default)
   - Settings persist across find bar sessions

6. **Keyboard Shortcuts:**
   - Ctrl+F: Opens find bar and focuses search input
   - Escape: Closes find bar and clears highlights
   - Enter: Navigate to next match
   - Shift+Enter: Navigate to previous match

7. **Error Handling:**
   - Display error message in bottom area of find bar for invalid regex
   - Show "No matches found" when search yields no results
   - Display warning when result count exceeds 1000 matches

8. **State Management:**
   - Remember last search term when find bar is reopened
   - Persist accent folding and case-sensitive preferences
   - Clear highlights when find bar is closed

## Non-Goals (Out of Scope)

1. **Search across multiple documents or tabs** - only searches current content
2. **Replace functionality** - this is find-only, not find-and-replace
3. **Advanced regex features** - while regex is supported internally, UI doesn't expose regex options
4. **Search within navigation menus or UI elements** - only searches main content area
5. **Integration with existing search features** - this is a separate, standalone feature
6. **Bookmarking or saving search results** - results are session-only
7. **Search history dropdown** - only remembers last search term

## Design Considerations

1. **HTML Template:** Create `assets/templates/find.html` and include before `{menu_html}` in `page.html`
2. **Styling:** Add SCSS styling and load in `suttas.sass` using `@include meta.load-css("find")`
3. **Icons:** Use existing `icons/32x32/fa_magnifying-glass-solid.png` for search button
4. **Layout:** Find bar slides from search icon to the left, maintaining visual hierarchy
5. **Visual Hierarchy:** Current match visually distinct from other matches
6. **Mobile:** Should work the same as desktop

## Technical Considerations

1. **Dependencies:** Add `dom-find-and-replace` npm package (TypeScript compatible)
2. **TypeScript Integration:** Create new module in `src-ts/find.ts` and expose via `document.SSP.find`
3. **Build Process:** Update webpack configuration to include new module
4. **DOM Integration:** Search limited to `#ssp_content` element as specified
5. **Performance:** Use debounced search (400ms) and result limiting (1000 max) for large documents
6. **Regex Support:** Leverage dom-find-and-replace's built-in regex capabilities for accent folding
7. **Memory Management:** Properly clean up highlights and event listeners when find bar closes

## Success Metrics

1. **Functionality:** All keyboard shortcuts work as expected (Ctrl+F, Escape, Enter, Shift+Enter)
2. **Performance:** Search completes within 500ms for documents up to 100KB
3. **Accuracy:** Accent folding correctly matches Pāli characters (ā matches both a and ā)
4. **Usability:** Users can navigate between all search results without missing any matches
5. **Compatibility:** Feature works identically on desktop and mobile platforms

## Open Questions

1. **Styling specifics:** What exact colors should be used for match highlighting vs. current match highlighting?

General match background color: #ffff00

Current match background color: #ff9632

2. **Animation duration:** How fast should the find bar slide-out animation be? 1sec
3. **Mobile layout:** Should the find bar layout be modified for very small screens (< 320px)? No
4. **Performance tuning:** Should the 1000 result limit be configurable or fixed? Hard-coded
5. **Integration testing:** How should this feature interact with any existing text selection or highlighting features? Not a goal.
