# PRD: Anki Export Template Customization

## Overview

Allow users to customize the format of Anki flashcard exports from the Gloss Tab by providing template strings for Front and Back fields, stored in AppSettings and editable through an enhanced AnkiExportDialog.

## Problem Statement

Currently, the Anki CSV export in GlossTab.qml uses a hardcoded string template for Front and Back fields (see `format_paragraph_anki_csv()` around line 1334-1379). Users cannot customize how their Anki cards are formatted, limiting flexibility for different study approaches and learning styles.

## Goals

1. Enable user customization of Anki card Front and Back templates
2. Provide multiple export format options (Simple, Templated, Data CSV)
3. Allow users to choose whether to include cloze format exports
4. Store templates persistently in app settings
5. Provide an intuitive editing interface similar to SystemPromptsDialog.qml
6. Allow previewing rendered templates with sample data before export

## Non-Goals

- Conditional logic in templates (e.g., if/else statements)
- Template syntax validation before saving
- Supporting custom export formats beyond the three specified
- Batch template management or import/export of templates
- Multiple template presets library (only one preset in initial version)

## User Stories

### User Story 1: Template Customization
**As a** Pāli student using Anki for vocabulary study,
**I want to** customize how my flashcard Front and Back are formatted,
**So that** I can optimize my cards for my specific learning style and Anki deck structure.

**Acceptance Criteria:**
- User can access Anki Export Settings dialog from global menu
- User can edit Front and Back template strings in separate text areas
- Templates support variable substitution (e.g., `${word_stem}`, `${vocab.summary}`, `${dpd.pos}`)
- Templates are saved to AppSettings and persist across sessions
- Templates are used when "Templated" format is selected during export
- One preset template is available (current format + DPD fields)

### User Story 2: Export Format Selection
**As a** user exporting vocabulary to Anki,
**I want to** choose between different export formats,
**So that** I can select the most appropriate format for my current needs.

**Acceptance Criteria:**
- ComboBox in AnkiExportDialog offers three options: Simple, Templated, Data CSV
- "Simple" uses the current hardcoded format (backward compatible)
- "Templated" uses user-defined templates from AppSettings
- "Data CSV" exports raw GlossWordContext and DpdHeadword data combined in one row
- Selected format is applied during export

### User Story 3: Cloze Format Toggle
**As a** user creating Anki cards,
**I want to** optionally include cloze-format cards in my export,
**So that** I can choose whether to study using cloze deletion or basic card format.

**Acceptance Criteria:**
- Checkbox in AnkiExportDialog labeled "Include cloze format CSV"
- When checked, both basic and cloze format CSVs are exported
- When unchecked, only the selected format (Simple/Templated/Data) is exported
- Checkbox state is remembered in AppSettings

### User Story 4: Template Preview
**As a** user customizing Anki templates,
**I want to** preview how my templates will render with sample data,
**So that** I can verify the formatting and layout before exporting hundreds of cards.

**Acceptance Criteria:**
- Preview panel in AnkiExportDialog shows rendered template output
- Preview uses hardcoded sample word "abhivādetvā" in context sentence "upasaṅkamitvā bhagavantaṁ abhivādetvā ekamantaṁ nisīdi."
- Preview shows both Front and Back rendered side-by-side
- Preview updates automatically as user types in template editor
- Preview renders HTML with actual formatting (not plain text)
- Preview handles template errors gracefully and shows error message
- Preview is read-only (not editable)

## Technical Design

### Data Model Changes

#### AppSettings (backend/src/app_settings.rs)

Add new fields to `AppSettings` struct:

```rust
pub struct AppSettings {
    // ... existing fields ...
    pub anki_template_front: String,
    pub anki_template_back: String,
    pub anki_export_format: AnkiExportFormat,
    pub anki_include_cloze: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AnkiExportFormat {
    Simple,
    Templated,
    DataCsv,
}
```

Default values:
- `anki_template_front`: `"<div><p>${word_stem}</p><p>${context_snippet}</p></div>"`
- `anki_template_back`: `"<div><b>${dpd.pos}</b> ${vocab.summary}</div>"`
- `anki_export_format`: `AnkiExportFormat::Simple`
- `anki_include_cloze`: `true`

Note: The default template is a preset that includes the current simple format plus DPD part-of-speech field.

### UI Changes

#### AnkiExportDialog.qml

Transform from minimal dialog to full-featured settings dialog similar to SystemPromptsDialog.qml:

**Layout Structure:**
```
┌─────────────────────────────────────────────────────────────────┐
│ [Icon] Anki Export Settings                                     │
├─────────────────────────────────────────────────────────────────┤
│ ┌─────────────┬─────────────────────────┬──────────────────┐   │
│ │ Front       │ Template Editor:         │ Preview:         │   │
│ │ Back        │ Front                    │                  │   │
│ │             │ ┌───────────────────────┐│ ┌──────────────┐│   │
│ │             │ │ ${word_stem}          ││ │ Front:       ││   │
│ │             │ │ <div>${context...     ││ │ bhikkhu      ││   │
│ │             │ │                        ││ │              ││   │
│ │             │ └───────────────────────┘│ │ Back:        ││   │
│ │             │                           │ │ monk, ...    ││   │
│ │             │         [Preview]         │ └──────────────┘│   │
│ └─────────────┴─────────────────────────┴──────────────────┘   │
├─────────────────────────────────────────────────────────────────┤
│ Export Format: [Simple ▼]                                       │
│ ☑ Include cloze format CSV                                      │
├─────────────────────────────────────────────────────────────────┤
│                                                    [OK]          │
└─────────────────────────────────────────────────────────────────┘
```

**Components:**
- SplitView with three panels:
  - Left: List of templates (Front/Back)
  - Middle: TextArea for template editing with Preview button
  - Right: Preview panel showing rendered output
- ComboBox for export format selection
- CheckBox for cloze format inclusion
- Auto-save on text changes (like SystemPromptsDialog)
- Preview panel updates automatically as user types
- Preview panel renders HTML to show actual card appearance
- Preview panel shows Front and Back sections

**Bridge Methods:**
Add to SuttaBridge (or create new bridge if needed):
- `get_anki_template_front() -> String`
- `get_anki_template_back() -> String`
- `set_anki_template_front(template: String)`
- `set_anki_template_back(template: String)`
- `get_anki_export_format() -> String`
- `set_anki_export_format(format: String)`
- `get_anki_include_cloze() -> bool`
- `set_anki_include_cloze(include: bool)`
- `get_sample_vocabulary_data_json() -> String` (for preview)

#### GlossTab.qml

**Changes to export flow:**

1. Update `export_dialog_accepted()` (line 233):
   - Read export format setting from AppSettings
   - Apply appropriate format based on selection
   - Conditionally export cloze format based on checkbox

2. Update `gloss_as_anki_csv()` (line 1381):
   - Add format parameter to switch between Simple/Templated/DataCsv
   - When format is "Templated", call new template rendering function

3. Add new function `format_paragraph_anki_csv_templated()`:
   - Retrieve DpdHeadword data using vocab.uid
   - Build template variable context with:
     - `${word_stem}` - cleaned stem from vocab
     - `${context_snippet}` - example sentence from words_data
     - `${vocab.summary}` - DPD summary
     - `${vocab.word}` - DPD word/lemma
     - `${vocab.uid}` - DPD unique identifier
     - `${dpd.pos}` - part of speech
     - `${dpd.grammar}` - grammar info
     - `${dpd.meaning_1}` - primary meaning
     - `${dpd.construction}` - word construction
     - `${dpd.derivative}` - derivative info
     - (and other relevant DpdHeadword fields)
   - Render templates using JavaScript template string evaluation
   - Return formatted CSV rows

4. Add new function `format_paragraph_anki_csv_data()`:
   - Export raw data fields as CSV columns with header row
   - Include ALL DpdHeadword fields for maximum flexibility
   - Columns: word_stem, context_snippet, original_word, uid, lemma_1, lemma_2, pos, grammar, derived_from, meaning_1, construction, derivative, example_1, synonym, antonym, summary, and all other DpdHeadword fields
   - Useful for importing into spreadsheets or custom Anki note types

**Template Rendering:**

Use JavaScript template string evaluation in QML:

```javascript
function render_template(template, context) {
    // Create a function that evaluates the template with context variables
    let func_body = 'return `' + template + '`;';
    let func = new Function(...Object.keys(context), func_body);
    return func(...Object.values(context));
}
```

### Backend Changes

#### SuttaBridge (bridges/src/sutta_bridge.rs)

Add methods for Anki template settings:

```rust
#[qinvokable]
pub fn get_anki_template_front(&self) -> QString {
    // Read from AppSettings
}

#[qinvokable]
pub fn set_anki_template_front(&self, template: QString) {
    // Save to AppSettings
}

// Similar for back, format, include_cloze
```

Add method to retrieve full DpdHeadword by UID:

```rust
#[qinvokable]
pub fn get_dpd_headword_by_uid(&self, uid: QString) -> QString {
    // Query database for DpdHeadword
    // Serialize relevant fields to JSON
    // Return as QString
}
```

Add method to get sample vocabulary data for preview:

```rust
#[qinvokable]
pub fn get_sample_vocabulary_data_json(&self) -> QString {
    // Get hardcoded sample word "abhivādetvā" from DPD database
    // Context sentence: "upasaṅkamitvā bhagavantaṁ abhivādetvā ekamantaṁ nisīdi."
    // Build complete context object with:
    //   - word_stem: "abhivādeti" (cleaned)
    //   - context_snippet: "upasaṅkamitvā bhagavantaṁ <b>abhivādetvā</b> ekamantaṁ nisīdi."
    //   - original_word: "abhivādetvā"
    //   - vocab object (uid, word, summary)
    //   - dpd object (all DpdHeadword fields)
    // Serialize to JSON and store as static/cached data
    // Return as QString
}
```

Note: This data can be generated once and stored as a JSON file in assets or hardcoded, avoiding repeated database queries.

### Variable Substitution

**Available Variables:**

From GlossWordContext:
- `${clean_word}` - cleaned word
- `${original_word}` - original word from text
- `${context_snippet}` - sentence with word highlighted

From vocabulary gloss (LookupResult):
- `${vocab.uid}` - unique identifier
- `${vocab.word}` - stem/lemma
- `${vocab.summary}` - dictionary summary

From DpdHeadword (retrieved via uid):
- `${dpd.pos}` - part of speech
- `${dpd.grammar}` - grammar
- `${dpd.meaning_1}` - primary meaning
- `${dpd.meaning_lit}` - literal meaning
- `${dpd.construction}` - construction
- `${dpd.derivative}` - derivative
- `${dpd.root_key}` - root key
- `${dpd.compound_type}` - compound type
- `${dpd.example_1}` - example usage
- `${dpd.synonym}` - synonyms
- `${dpd.antonym}` - antonyms

**Simplified stem access:**
- `${word_stem}` - shorthand for cleaned vocab.word

### Export Format Specifications

#### Simple Format (current behavior)
- Front: word stem OR word + context snippet in div
- Back: vocab summary OR vocab summary in div (cloze has context with {{c1::word}})

#### Templated Format
- Front: rendered from `anki_template_front` setting
- Back: rendered from `anki_template_back` setting
- Variables substituted from vocab + DPD data

#### Data CSV Format
- Header row with all column names
- Each row contains all available data fields from DpdHeadword
- Columns include: word_stem, context_snippet, original_word, uid, lemma_1, lemma_2, pos, grammar, derived_from, neg, verb, trans, plus_case, meaning_1, meaning_lit, meaning_2, non_ia, sanskrit, root_key, root_sign, root_base, construction, derivative, suffix, phonetic, compound_type, compound_construction, source_1, sutta_1, example_1, antonym, synonym, variant, commentary, notes, stem, pattern, summary, etc.
- All DpdHeadword fields are included for maximum flexibility
- Useful for advanced users who want to process data externally or create custom Anki note types

## Implementation Plan

### Phase 1: Backend Settings
1. Add fields to AppSettings struct
2. Implement default values
3. Add getter/setter methods in SuttaBridge
4. Add DpdHeadword retrieval method

### Phase 2: Dialog UI
1. Create enhanced AnkiExportDialog.qml layout
2. Implement SplitView with template list, editor, and preview panels
3. Add ComboBox for format selection
4. Add CheckBox for cloze inclusion
5. Wire up bridge methods to load/save settings
6. Add auto-save on text changes
7. Implement auto-updating preview panel (triggers on text change)
8. Add HTML rendering to preview panel (TextArea with textFormat: Text.RichText)
9. Create hardcoded/cached sample data for "abhivādetvā"

### Phase 3: Template Rendering & Preview
1. Implement template variable context builder
2. Add DpdHeadword data retrieval in export flow
3. Implement JavaScript template string rendering (shared function for preview and export)
4. Add error handling for invalid templates (try-catch with error display)
5. Create/load hardcoded sample data for "abhivādetvā" with full context
6. Add preview rendering function in AnkiExportDialog (reuses export rendering logic)
7. Add error display in preview panel for invalid templates
8. Add debouncing (200-500ms) to preview updates to avoid performance issues

### Phase 4: Export Integration
1. Update export_dialog_accepted() to use format setting
2. Implement format_paragraph_anki_csv_templated()
3. Implement format_paragraph_anki_csv_data()
4. Update gloss_as_anki_csv() to handle all formats
5. Conditionally export cloze based on checkbox

### Phase 5: Testing & Polish
1. Test template rendering with various variable combinations
2. Test all three export formats
3. Test cloze checkbox functionality
4. Verify settings persistence
5. Test edge cases (missing data, invalid templates)

## Testing Considerations

### Unit Tests
- Template rendering with valid variables
- Template rendering with missing variables (should gracefully handle)
- CSV escaping in templated output
- Data CSV column order and formatting

### Integration Tests
- Settings persistence across app restarts
- Export format selection affects output
- Cloze checkbox controls file generation
- DpdHeadword retrieval by UID

### Manual Testing
- Open AnkiExportDialog from global menu and edit templates
- Edit template and verify preview updates automatically with HTML rendering
- Test preview with invalid template syntax (verify error message appears)
- Test preview with various template variables using "abhivādetvā" sample data
- Verify preview HTML rendering shows actual card appearance
- Export with Simple format (verify backward compatibility)
- Export with Templated format using custom templates
- Export with Data CSV format and verify all DpdHeadword fields are included
- Toggle cloze checkbox and verify file output
- Import generated CSV into Anki and verify cards render correctly
- Verify preview output matches actual exported card format exactly

## Design Decisions

All open questions have been resolved and integrated into the specification above:

1. **Conditional logic in templates:** Not included in initial version. Templates will use basic variable substitution only. This keeps the implementation simple and the learning curve low.

2. **Template validation:** No validation before saving. Templates will be validated at render time (preview and export), with clear error messages shown to the user.

3. **Data CSV fields:** ALL DpdHeadword fields will be included in Data CSV format for maximum flexibility. Advanced users can then process or filter the data as needed.

4. **Template presets:** One preset template will be provided in the initial version: the current simple format enhanced with DPD part-of-speech field. This serves as a good starting point for customization.

5. **Dialog access:** AnkiExportDialog will be accessible from the global menu only. No need for additional access points from GlossTab.

6. **Preview sample data:** Hardcoded word "abhivādetvā" in the sentence "upasaṅkamitvā bhagavantaṁ abhivādetvā ekamantaṁ nisīdi." will be used. The complete vocabulary context (DpdHeadword data, context snippet, etc.) will be generated once and stored as JSON to avoid repeated database queries.

7. **Preview update behavior:** Preview will update automatically as user types, with debouncing (200-500ms delay) to avoid performance issues while typing.

8. **Preview rendering:** HTML rendering will be enabled in the preview panel using QML TextArea with `textFormat: Text.RichText`, so users see the actual appearance of their cards including formatting, bold, italics, etc.

## Success Metrics

- Users can successfully customize and save Anki templates
- Preview accurately reflects what will be exported
- Exported Anki CSV files import correctly into Anki
- Template variables are correctly substituted in both preview and export
- Settings persist across sessions
- All three export formats produce valid CSV output
- Preview helps users catch template errors before export

## Risks & Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| Invalid templates crash export | High | Wrap template rendering in try-catch, show error message in preview and export |
| DpdHeadword lookup fails for some words | Medium | Handle missing data gracefully, use empty string or default value |
| Template syntax too complex for users | Medium | Provide preset template, clear documentation, and real-time preview |
| Performance impact from DPD queries during export | Low | Cache DpdHeadword results during export batch |
| Performance impact from real-time preview | Low | Debounce preview updates (200-500ms), use pre-generated sample data |
| Preview doesn't match actual export output | Medium | Use exact same rendering function for preview and export |
| Preview sample data not representative of user's vocabulary | Low | Sample word "abhivādetvā" has rich DPD data; future enhancement allows selecting different samples |

## Documentation

### User Documentation
- Add help text in AnkiExportDialog explaining template variables
- Document the preset template (current format + DPD pos field) as starting point
- Provide list of all available template variables with descriptions
- Document that preview uses hardcoded sample word "abhivādetvā"
- Add tooltip explaining that preview updates automatically as you type
- Note that all DpdHeadword fields are available in Data CSV export format

### Developer Documentation
- Document template rendering implementation
- Document AppSettings schema changes
- Update PROJECT_MAP.md with new dialog and functions

## Future Enhancements

- Template presets library (multiple built-in templates to choose from)
- Select different sample words for preview (dropdown or "use first word from gloss")
- Preview multiple sample cards at once (show 3-5 example cards)
- Conditional logic in templates (if/else, ternary operators)
- Support for custom JavaScript functions in templates
- Template validation and syntax highlighting in editor
- Import/export template collections
- Per-paragraph template overrides
- Side-by-side comparison of different template versions
- Preview in actual Anki card format (iframe with Anki CSS)
- Template variable browser/picker to help discover available fields
- "Copy from Simple/Templated" button to switch between formats while preserving structure
