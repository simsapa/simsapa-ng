## Relevant Files

- `backend/src/app_settings.rs` - Add new Anki template settings fields to AppSettings struct
- `backend/src/db/dpd_models.rs` - DpdHeadword model used for template variable data
- `backend/src/db/dpd.rs` - Database queries for retrieving DpdHeadword data
- `bridges/src/sutta_bridge.rs` - Add bridge methods for Anki template settings and DPD data retrieval
- `bridges/build.rs` - Register new QML files in qml_files list
- `assets/qml/com/profoundlabs/simsapa/SuttaBridge.qml` - Add qmllint type definitions for new bridge methods
- `assets/qml/AnkiExportDialog.qml` - Transform into full-featured settings dialog with template editor and preview
- `assets/qml/GlossTab.qml` - Update export flow to support multiple formats and template rendering

### Notes

- Follow SystemPromptsDialog.qml architecture for the AnkiExportDialog layout (SplitView with list, editor, preview panels)
- Use JavaScript template string evaluation for rendering templates in QML
- Store sample vocabulary data as hardcoded/cached JSON to avoid repeated database queries
- All new QML files must be added to bridges/build.rs qml_files list
- Corresponding qmllint type definitions must be created for new Rust bridge methods
- Test backend functions with `cd backend && cargo test`
- Build with `make build -B` to verify compilation
- Run sass compilation with `make sass` if CSS changes are needed

## Tasks

### Phase 1: Backend Settings and Data Model

- [ ] 1.0 Backend Settings and Data Model
  - [ ] 1.1 Add `AnkiExportFormat` enum to `backend/src/app_settings.rs`
    - Add enum with variants: Simple, Templated, DataCsv
    - Derive Debug, Clone, PartialEq, Eq, Serialize, Deserialize
  - [ ] 1.2 Add new fields to `AppSettings` struct in `backend/src/app_settings.rs`
    - Add `anki_template_front: String`
    - Add `anki_template_back: String`
    - Add `anki_export_format: AnkiExportFormat`
    - Add `anki_include_cloze: bool`
  - [ ] 1.3 Update `Default` impl for `AppSettings`
    - Set `anki_template_front` default: `"<div><p>${word_stem}</p><p>${context_snippet}</p></div>"`
    - Set `anki_template_back` default: `"<div><b>${dpd.pos}</b> ${vocab.summary}</div>"`
    - Set `anki_export_format` default: `AnkiExportFormat::Simple`
    - Set `anki_include_cloze` default: `true`
  - [ ] 1.4 Test backend compilation with `cd backend && cargo build`

### Phase 2: Bridge Methods for Settings and Sample Data

- [ ] 2.0 Bridge Methods for Settings and DPD Data
  - [ ] 2.1 Add getter methods to `SuttaBridge` in `bridges/src/sutta_bridge.rs`
    - Add `get_anki_template_front() -> QString`
    - Add `get_anki_template_back() -> QString`
    - Add `get_anki_export_format() -> QString` (returns "Simple"/"Templated"/"DataCsv")
    - Add `get_anki_include_cloze() -> bool`
  - [ ] 2.2 Add setter methods to `SuttaBridge`
    - Add `set_anki_template_front(template: QString)`
    - Add `set_anki_template_back(template: QString)`
    - Add `set_anki_export_format(format: QString)`
    - Add `set_anki_include_cloze(include: bool)`
  - [ ] 2.3 Create sample vocabulary data for preview
    - Create `backend/src/anki_sample_data.rs` module
    - Generate JSON for word "abhivādetvā" with full DpdHeadword data
    - Include context: "upasaṅkamitvā bhagavantaṁ <b>abhivādetvā</b> ekamantaṁ nisīdi."
    - Include all template variables: word_stem, context_snippet, vocab.*, dpd.*
  - [ ] 2.4 Add `get_sample_vocabulary_data_json() -> QString` to SuttaBridge
    - Return hardcoded sample data from anki_sample_data module
  - [ ] 2.5 Add `get_dpd_headword_by_uid(uid: QString) -> QString` to SuttaBridge
    - Query DPD database for headword by UID
    - Serialize all relevant DpdHeadword fields to JSON
    - Return as QString
  - [ ] 2.6 Update `assets/qml/com/profoundlabs/simsapa/SuttaBridge.qml` with qmllint type definitions
    - Add function signatures for all new bridge methods
    - Use correct return types and parameter types
  - [ ] 2.7 Test bridge compilation with `make build -B`

### Phase 3: Core Template Rendering Function

- [ ] 3.0 Template Rendering Function in GlossTab
  - [ ] 3.1 Add `render_template(template, context)` function to `GlossTab.qml`
    - Create JavaScript function using template string evaluation
    - Accept template string and context object
    - Use `new Function()` to evaluate template with context variables
    - Wrap in try-catch for error handling
  - [ ] 3.2 Add `build_template_context(vocab, dpd_data, context_snippet)` function
    - Build context object with all available variables
    - Add word_stem (shorthand for vocab.word cleaned)
    - Add context_snippet, original_word, clean_word
    - Add vocab.uid, vocab.word, vocab.summary
    - Add all dpd.* fields from dpd_data
  - [ ] 3.3 Test template rendering with simple cases
    - Test with `${word_stem}` variable
    - Test with `${vocab.summary}` variable
    - Test with `${dpd.pos}` variable
    - Verify error handling with invalid template syntax

### Phase 4: Enhanced AnkiExportDialog UI

- [ ] 4.0 Enhanced AnkiExportDialog UI with Template Editor
  - [ ] 4.1 Create basic dialog structure in `assets/qml/AnkiExportDialog.qml`
    - Add ApplicationWindow with proper title and size
    - Add Logger component
    - Add properties for current templates and selected template
  - [ ] 4.2 Implement left panel template list
    - Add ListModel with two items: "Front", "Back"
    - Add ListView with ItemDelegate for each template
    - Implement selection highlighting
    - Wire up onClicked to switch between Front/Back templates
  - [ ] 4.3 Implement middle panel template editor
    - Add Label showing currently selected template name
    - Add GroupBox with ScrollView and TextArea
    - Set TextArea properties: wrapMode, selectByMouse, font.pointSize
    - Wire up TextArea.text to current template
    - Implement auto-save on textChanged (call setter bridge methods)
  - [ ] 4.4 Implement right panel preview
    - Add Label "Preview:"
    - Add GroupBox for preview container
    - Add ScrollView with TextArea for rendered output
    - Set textFormat: Text.RichText for HTML rendering
    - Make preview read-only
  - [ ] 4.5 Add SplitView layout
    - Create SplitView with Qt.Horizontal orientation
    - Add left panel (preferredWidth: 200, minimumWidth: 150)
    - Add middle panel (fillWidth: true)
    - Add right panel (preferredWidth: 300, minimumWidth: 200)
  - [ ] 4.6 Add bottom controls section
    - Add RowLayout with format selection controls
    - Add Label "Export Format:"
    - Add ComboBox with model: ["Simple", "Templated", "Data CSV"]
    - Wire up to get/set_anki_export_format bridge methods
  - [ ] 4.7 Add cloze checkbox
    - Add CheckBox with text "Include cloze format CSV"
    - Wire up to get/set_anki_include_cloze bridge methods
    - Load initial state from settings
  - [ ] 4.8 Add OK button
    - Add Button with text "OK"
    - Wire up onClicked to close dialog
  - [ ] 4.9 Implement load_templates() function
    - Call bridge methods to get current templates
    - Set front/back template properties
    - Load export format and cloze checkbox state
    - Select first template in list
  - [ ] 4.10 Wire up Component.onCompleted to load_templates()
  - [ ] 4.11 Register AnkiExportDialog.qml in `bridges/build.rs`
    - Add to qml_files list if not already present
  - [ ] 4.12 Test dialog opens and displays correctly with `make build -B && make run`

### Phase 5: Template Preview with Auto-Update

- [ ] 5.0 Template Preview Functionality
  - [ ] 5.1 Add preview rendering function to `AnkiExportDialog.qml`
    - Add `render_preview()` function
    - Get sample data from SuttaBridge.get_sample_vocabulary_data_json()
    - Parse sample data JSON
    - Call GlossTab.render_template for Front template
    - Call GlossTab.render_template for Back template
    - Format output with "Front:" and "Back:" sections
    - Handle errors and display error messages in preview
  - [ ] 5.2 Add Timer for debouncing preview updates
    - Add Timer component with interval: 300ms
    - Set running: false, repeat: false
    - On triggered: call render_preview()
  - [ ] 5.3 Wire up template editor to trigger preview
    - In TextArea.onTextChanged, restart debounce timer
    - Preview updates 300ms after user stops typing
  - [ ] 5.4 Add initial preview rendering
    - Call render_preview() in load_templates() after loading
  - [ ] 5.5 Test preview updates automatically
    - Edit template and verify preview updates after 300ms
    - Test with valid templates (shows rendered output)
    - Test with invalid templates (shows error message)

### Phase 6: Export Integration - Simple Format

- [ ] 6.0 Maintain Simple Format Export (Backward Compatibility)
  - [ ] 6.1 Verify current `format_paragraph_anki_csv()` in `GlossTab.qml`
    - Review existing implementation (lines ~1334-1379)
    - Ensure it handles both "basic" and "cloze" format types
    - Confirm CSV escaping is correct
  - [ ] 6.2 No changes needed for Simple format (already implemented)

### Phase 7: Export Integration - Templated Format

- [ ] 7.0 Add Templated Format Export
  - [ ] 7.1 Add `format_paragraph_anki_csv_templated(paragraph, paragraph_index)` to `GlossTab.qml`
    - Get paragraph data from paragraph_model
    - Get words_data_json and parse
    - Get Front and Back templates from SuttaBridge
    - Loop through paragraph.vocabulary
    - For each vocab item:
      - Get DpdHeadword data using SuttaBridge.get_dpd_headword_by_uid(vocab.uid)
      - Parse DPD data JSON
      - Get context_snippet from words_data
      - Build template context with all variables
      - Render Front template with context
      - Render Back template with context
      - Format as CSV row with proper escaping
    - Return CSV lines joined with newline
  - [ ] 7.2 Update `gloss_as_anki_csv(format_type)` function
    - Check format_type parameter
    - If "templated", call format_paragraph_anki_csv_templated()
    - Otherwise use existing format_paragraph_anki_csv() logic
  - [ ] 7.3 Test templated export with sample templates
    - Export with default templates
    - Verify CSV output has correct Front/Back content
    - Verify template variables are substituted correctly

### Phase 8: Export Integration - Data CSV Format

- [ ] 8.0 Add Data CSV Format Export
  - [ ] 8.1 Add `format_paragraph_anki_csv_data(paragraph, paragraph_index)` to `GlossTab.qml`
    - Get paragraph data from paragraph_model
    - Get words_data_json and parse
    - Build CSV header row with all column names:
      - word_stem, context_snippet, original_word
      - uid, lemma_1, lemma_2, pos, grammar, derived_from, meaning_1, construction, derivative
      - example_1, synonym, antonym, summary
      - (include all relevant DpdHeadword fields)
    - Loop through paragraph.vocabulary
    - For each vocab item:
      - Get DpdHeadword data using SuttaBridge.get_dpd_headword_by_uid(vocab.uid)
      - Parse DPD data JSON
      - Get context_snippet from words_data
      - Build CSV row with all field values in same order as header
      - Properly escape CSV fields
    - Return header + data rows joined with newline
  - [ ] 8.2 Update `gloss_as_anki_csv(format_type)` to handle "data" format
    - Add else-if branch for format_type === "data"
    - Call format_paragraph_anki_csv_data()
  - [ ] 8.3 Test Data CSV export
    - Export with Data CSV format
    - Verify header row includes all columns
    - Verify data rows have values in correct order
    - Import into spreadsheet and verify all fields present

### Phase 9: Export Dialog Integration

- [ ] 9.0 Update Export Flow in GlossTab
  - [ ] 9.1 Update `export_dialog_accepted()` in `GlossTab.qml`
    - Read export format from SuttaBridge.get_anki_export_format()
    - Read include_cloze from SuttaBridge.get_anki_include_cloze()
    - When "Anki CSV" selected:
      - If format is "Simple": use current logic (basic + optional cloze)
      - If format is "Templated": call gloss_as_anki_csv("templated")
      - If format is "DataCsv": call gloss_as_anki_csv("data")
      - Handle cloze checkbox for Simple/Templated (not applicable for Data CSV)
  - [ ] 9.2 Update file naming for different formats
    - Simple: gloss_export_anki_basic.csv, gloss_export_anki_cloze.csv
    - Templated: gloss_export_anki_templated.csv, gloss_export_anki_templated_cloze.csv
    - Data CSV: gloss_export_anki_data.csv (single file, no cloze variant)
  - [ ] 9.3 Add menu item or button to access AnkiExportDialog
    - Determine appropriate location in global menu
    - Add menu action to open AnkiExportDialog
    - Test dialog opens from menu

### Phase 10: Testing and Verification

- [ ] 10.0 Testing and Verification
  - [ ] 10.1 Unit test template rendering function
    - Test render_template with valid template and context
    - Test with missing variables (should handle gracefully)
    - Test with invalid syntax (should catch error)
    - Test with HTML in template
  - [ ] 10.2 Test Settings Persistence
    - Edit templates in dialog and close
    - Reopen dialog and verify templates saved
    - Change export format and verify saved
    - Toggle cloze checkbox and verify saved
    - Restart app and verify settings persist
  - [ ] 10.3 Test Preview Functionality
    - Open dialog and verify preview shows default template
    - Edit Front template and verify preview updates
    - Edit Back template and verify preview updates
    - Enter invalid template syntax and verify error shown
    - Test with various template variables
  - [ ] 10.4 Test Simple Format Export
    - Export with Simple format
    - Verify backward compatibility with current format
    - Import CSV into Anki and verify cards display correctly
  - [ ] 10.5 Test Templated Format Export
    - Edit templates with custom format
    - Export with Templated format
    - Verify CSV has correctly rendered Front/Back
    - Verify all template variables substituted
    - Import CSV into Anki and verify cards display correctly
  - [ ] 10.6 Test Data CSV Export
    - Export with Data CSV format
    - Verify header row has all columns
    - Verify data rows have all fields populated
    - Import into spreadsheet and verify data integrity
  - [ ] 10.7 Test Cloze Checkbox
    - Enable cloze checkbox with Simple format, verify both files exported
    - Disable cloze checkbox, verify only basic file exported
    - Test with Templated format
  - [ ] 10.8 Test Edge Cases
    - Export paragraph with no vocabulary words
    - Export with DpdHeadword lookup failures (missing UIDs)
    - Export with missing/empty context snippets
    - Export with special characters requiring CSV escaping
    - Test very long template strings
  - [ ] 10.9 Full Integration Test
    - Create new gloss session with multiple paragraphs
    - Customize templates in AnkiExportDialog
    - Preview templates
    - Export with all three formats
    - Verify all exports successful
    - Import all CSVs into Anki
    - Verify all cards display correctly
  - [ ] 10.10 Performance Testing
    - Test export with large vocabulary list (100+ words)
    - Verify preview updates don't lag with debouncing
    - Verify DpdHeadword queries cached during export
  - [ ] 10.11 Final build verification
    - Run `make build -B` and verify no errors
    - Run `cd backend && cargo test` and verify tests pass
    - Test on desktop platform
    - Document any known limitations
