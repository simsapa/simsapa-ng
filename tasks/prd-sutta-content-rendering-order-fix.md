# PRD: Sutta Content Rendering Order Fix

## Introduction/Overview

The `render_sutta_content()` function currently produces HTML for suttas with paragraph segments rendered in incorrect order. This blocks core functionality as users cannot read suttas properly - the content appears jumbled with text segments appearing out of sequence compared to the source JSON data. 

The issue stems from the segment combination process in `bilara_text_to_segments()` where JSON lists are deserialized into BTreeMap structures, but the final HTML output doesn't preserve the correct sequential order of the segments.

**Goal:** Fix the sutta content rendering to display segments in the correct order and update the HTML output format to match SuttaCentral's structure exactly.

## Goals

1. **Primary Goal:** Ensure sutta content segments render in the correct sequential order as defined in the source JSON files
2. **Secondary Goal:** Update HTML output format to match SuttaCentral's structure exactly (`<span class="segment" id="...">` format)
3. **Supporting Goal:** Implement comprehensive tests for each stage of the rendering pipeline to prevent regressions

## User Stories

- **As a Simsapa user**, I want to read suttas with text appearing in the correct sequence, so that I can understand the dhamma teachings properly without confusion
- **As a developer**, I want clear test coverage for each rendering stage, so that I can confidently modify the rendering pipeline without breaking functionality
- **As a content reviewer**, I want sutta HTML output to match the established SuttaCentral format, so that styling and downstream processing work consistently

## Functional Requirements

### Core Rendering Fix
1. The system must render sutta segments in the exact order specified in the source JSON files (e.g., `sn56.11:1.2` must immediately follow `sn56.11:1.1`)
2. The `bilara_text_to_segments()` function must preserve segment order when combining BTreeMap values
3. The fix must apply to all suttas in the database, not just the test example

### HTML Output Format
4. The system must generate HTML segments using the SuttaCentral format:
   ```html
   <span class="segment" id="segment_id">
     <span class="root" lang="pli" translate="no">
       <span class="text" lang="la">segment_content</span>
     </span>
   </span>
   ```
5. The system must replace the current `<span data-tmpl-key="...">` format with the required SuttaCentral format
6. Language attributes must be set correctly (`lang="pli"` for Pali content, `lang="la"` for text spans)

### Code Structure & Testing
7. Large functions in the rendering pipeline must be extracted to focused helper functions with clear responsibilities
8. Function signatures may be refactored as needed for better design (backward compatibility not required)
9. Each stage of the rendering pipeline must have corresponding unit tests that verify intermediate outputs
10. Tests must validate the progression from JSON input → BTreeMap combination → HTML generation → final output

## Non-Goals (Out of Scope)

- Performance optimizations for the rendering pipeline
- Error handling improvements in the rendering process  
- Support for content formats other than the existing JSON structure
- UI/UX changes to how rendered content is displayed
- Migration of existing rendered content in the database

## Design Considerations

The current rendering pipeline follows this flow:
1. `sutta_to_segments_json()` retrieves SuttaVariant, SuttaComment, SuttaGloss data
2. `bilara_text_to_segments()` combines JSON lists into BTreeMap structures
3. `render_sutta_content()` generates final HTML using templates
4. `sutta_html_page()` receives the content for final page assembly

The HTML format change requires updating the template generation logic to produce the nested span structure with appropriate CSS classes and attributes.

## Technical Considerations

- The issue likely occurs in `bilara_text_to_segments()` where BTreeMap iteration may not preserve insertion order
- Consider using `IndexMap` or similar ordered map structure if order preservation is the root cause
- Test data location: `backend/tests/data/` contains reference files for validation
- The rendering pipeline integrates with the Rust backend using Diesel ORM for database access

## Success Metrics

- The specific test case `sn56.11` renders with correct segment order (text "Dveme, bhikkhave, antā pabbajitena na sevitabbā." appears immediately after "Tatra kho bhagavā pañcavaggiye bhikkhū āmantesi:")
- Generated HTML output matches the reference `sn56.11_pli_ms.suttacentral.main.html` structure exactly
- All unit tests pass for each rendering pipeline stage
- No regressions in existing sutta rendering functionality
