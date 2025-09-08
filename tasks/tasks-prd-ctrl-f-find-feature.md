# Tasks: Ctrl+F Find Feature for Simsapa

## Relevant Files

- `package.json` - Added dom-find-and-replace npm dependency
- `src-ts/tsconfig.json` - Updated to include find.ts and module resolution settings
- `src-ts/find.ts` - New TypeScript module implementing find functionality (core implementation complete, uses global Map for state persistence)
- `src-ts/find.test.ts` - Unit tests for find module (12 tests covering search, accent folding, navigation)
- `src-ts/test-setup.ts` - Jest test setup file
- `jest.config.js` - Jest configuration for TypeScript testing
- `package.json` - Added test dependencies and npm test script
- `src-ts/simsapa.ts` - Updated to import and expose find module in document.SSP.find
- `assets/js/simsapa.min.js` - Compiled JavaScript including dom-find-and-replace library
- `assets/templates/find.html` - HTML template for find bar UI (complete structure created)
- `assets/templates/page.html` - Updated to include {find_html} template before menu
- `assets/sass/_find.scss` - SCSS styling for find bar components (Qt Fusion style created)
- `assets/sass/suttas.sass` - Updated to load find styles
- `assets/css/suttas.css` - Generated CSS including find bar styles
- `assets/icons/32x32/fa_magnifying-glass-solid.png` - Search icon (already exists)

### Notes

- Unit tests should be placed alongside the code files they are testing
- Use `npx jest src-ts/find.test.ts` to run find-specific tests
- Use `npx webpack` to compile TypeScript changes to assets/js/simsapa.min.js
- Use `make sass` to compile SCSS changes

## Tasks

- [x] 1.0 Setup Dependencies and Project Structure
  - [x] 1.1 Install dom-find-and-replace npm package (`npm install dom-find-and-replace`)
  - [x] 1.2 Verify TypeScript types are available (dom-find-and-replace has built-in TypeScript declarations)
  - [x] 1.3 Test webpack compilation works with new dependency
  - [x] 1.4 Create empty `src-ts/find.ts` file with basic module structure

- [x] 2.0 Create Find Bar HTML Template and Basic Styling
  - [x] 2.1 Create `assets/templates/find.html` with complete find bar structure (search icon, input, controls, checkboxes)
  - [x] 2.2 Update `assets/templates/page.html` to include find template before `{menu_html}`
  - [x] 2.3 Create `assets/sass/_find.scss` with Qt Fusion styling matching existing menu design
  - [x] 2.4 Update `assets/sass/suttas.sass` to load find styles using `@include meta.load-css("find")`
  - [x] 2.5 Test template rendering and basic styling (find bar should be hidden by default)

- [x] 3.0 Implement Core TypeScript Find Module
  - [x] 3.1 Create basic FindManager class structure in `src-ts/find.ts`
  - [x] 3.2 Add dom-find-and-replace import and basic wrapper functions
  - [x] 3.3 Implement debounced search with 400ms delay
  - [x] 3.4 Add show/hide find bar functionality
  - [x] 3.5 Update `src-ts/simsapa.ts` to import and expose find module in `document.SSP.find`

- [x] 4.0 Add Search Functionality and DOM Integration
  - [x] 4.1 Implement search within `#ssp_content` element only
  - [x] 4.2 Add match highlighting with background colors (#ffff00 for matches, #ff9632 for current)
  - [x] 4.3 Implement match counter display ("current/total" format)
  - [x] 4.4 Add scroll-to-match functionality for current match
  - [x] 4.5 Handle "No matches found" state and display appropriate message

- [x] 5.0 Implement Navigation and Match Management
  - [x] 5.1 Add next match navigation with wrap-around behavior
  - [x] 5.2 Add previous match navigation with wrap-around behavior
  - [x] 5.3 Update current match highlighting when navigating
  - [x] 5.4 Ensure current match is scrolled into view on navigation
  - [x] 5.5 Handle edge cases (single match, no matches, navigation at boundaries)

- [x] 6.0 Add Keyboard Shortcuts and Event Handling
  - [x] 6.1 Implement Ctrl+F to open find bar and focus search input
  - [x] 6.2 Implement Escape to close find bar and clear highlights
  - [x] 6.3 Implement Enter for next match navigation
  - [x] 6.4 Implement Shift+Enter for previous match navigation
  - [x] 6.5 Add click handlers for search icon, previous/next buttons

- [ ] 7.0 Implement Pali/Sanskrit Accent Folding
  - [ ] 7.1 Create accent folding utility function using provided character mappings
  - [ ] 7.2 Integrate accent folding with search when checkbox is enabled
  - [ ] 7.3 Add case-sensitive search option when checkbox is enabled
  - [ ] 7.4 Ensure both options work correctly together and separately
  - [ ] 7.5 Test with Pali text examples (ā, ī, ū, ṃ, ṁ, ṅ, ñ, ṭ, ḍ, ṇ, ḷ, ṛ, ṣ, ś)

- [ ] 8.0 Add State Persistence and Error Handling
  - [ ] 8.1 Implement localStorage for search term persistence
  - [ ] 8.2 Implement localStorage for accent folding and case-sensitive preferences
  - [ ] 8.3 Add error handling for invalid regex patterns with user-friendly messages
  - [ ] 8.4 Add warning when result count exceeds 1000 matches (hard limit)
  - [ ] 8.5 Implement proper cleanup when find bar is closed (remove highlights, event listeners)

- [ ] 9.0 Final Integration and Testing
  - [ ] 9.1 Create comprehensive unit tests in `src-ts/find.test.ts` covering all functionality
  - [ ] 9.2 Test mobile responsiveness and ensure identical functionality to desktop
  - [ ] 9.3 Performance testing with large documents (up to 100KB)
  - [ ] 9.4 Integration testing with existing menu and page functionality
  - [ ] 9.5 Final code review, cleanup, and documentation
