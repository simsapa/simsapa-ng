# Tasks: Sutta Content Rendering Order Fix

Based on analysis of the PRD and existing codebase:

**Current State Assessment:**
- The rendering pipeline exists in `backend/src/app_data.rs::render_sutta_content()` 
- Core issue is in `backend/src/helpers.rs::bilara_text_to_segments()` using BTreeMap which may not preserve insertion order
- HTML generation uses `<span data-tmpl-key="...">` format but needs SuttaCentral `<span class="segment" id="...">` format
- Test files show the ordering problem: `sn56.11:1.2` should immediately follow `sn56.11:1.1` but appears much later
- Existing test structure in `backend/tests/test_render_sutta_content.rs` provides foundation for validation

## Relevant Files

- `backend/src/helpers.rs` - Contains `bilara_text_to_segments()` function that needs order preservation fix
- `backend/src/app_data.rs` - Contains `render_sutta_content()` and `sutta_to_segments_json()` functions  
- `backend/src/html_content.rs` - Contains `sutta_html_page()` for final page assembly
- `backend/tests/test_render_sutta_content.rs` - Updated tests to validate new SuttaCentral HTML format structure
- `backend/tests/data/sn56.11_pli_ms_sutta_content.json` - Reference JSON input data
- `backend/tests/data/sn56.11_pli_ms.suttacentral.main.html` - Target HTML format reference
- `backend/tests/data/sn56.11_pli_ms.main.html` - Current (incorrect) HTML output
- `backend/tests/helpers/mod.rs` - Test helper functions to extend for new test utilities

### Notes

- Tests use `cargo test` in `backend/` directory
- Current BTreeMap preserves keys alphabetically, not insertion order - likely root cause of ordering issue
- HTML format change requires updating template generation logic in `bilara_content_json_to_html()` function

## Tasks

- [ ] 1.0 Update HTML Output Format to Match SuttaCentral Structure
  - [x] 1.1 Update `bilara_content_json_to_html()` to generate SuttaCentral span structure instead of `data-tmpl-key` format
  - [ ] 1.2 Modify segment HTML generation to use nested spans: `<span class="segment" id="segment_id"><span class="root" lang="pli" translate="no"><span class="text" lang="la">content</span></span></span>`
  - [ ] 1.3 Add proper language attributes (`lang="pli"` for root spans, `lang="la"` for text spans)
  - [ ] 1.4 Update existing tests to validate new HTML format structure
  - [ ] 1.5 Create specific test for sn56.11 HTML format validation against reference file

- [ ] 2.0 Fix Segment Order Preservation in BTreeMap Processing  
  - [ ] 2.1 Investigate BTreeMap ordering behavior in `bilara_text_to_segments()` to confirm it's the root cause
  - [ ] 2.2 Replace BTreeMap with IndexMap or LinkedHashMap to preserve insertion order in JSON deserialization
  - [ ] 2.3 Update all BTreeMap type signatures in the rendering pipeline to use order-preserving maps
  - [ ] 2.4 Verify that segment iteration order matches JSON input order in test scenarios
  - [ ] 2.5 Create test that validates sn56.11 segment sequence (1.2 immediately follows 1.1, then 2.1, etc.)

- [ ] 3.0 Extract Large Functions into Focused Helper Functions
  - [ ] 3.1 Extract template application logic from `bilara_text_to_segments()` into separate `apply_content_template()` function
  - [ ] 3.2 Extract variant/comment/gloss processing into separate `merge_auxiliary_content()` function  
  - [ ] 3.3 Extract HTML span generation logic into separate `generate_segment_html()` function
  - [ ] 3.4 Refactor `render_sutta_content()` to use extracted helper functions with clearer separation of concerns
  - [ ] 3.5 Update function signatures as needed for better design (backward compatibility not required per PRD)

- [ ] 4.0 Create Comprehensive Pipeline Stage Testing
  - [ ] 4.1 Create test for JSON input → BTreeMap deserialization stage to verify order preservation
  - [ ] 4.2 Create test for BTreeMap combination stage (content + template + variants) to verify merge correctness  
  - [ ] 4.3 Create test for individual segment HTML generation to validate SuttaCentral format
  - [ ] 4.4 Create test for final HTML assembly to verify complete page structure
  - [ ] 4.5 Add regression test that verifies sn56.11 segment order matches expected sequence from reference files