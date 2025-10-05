# PRD: Anki CSV Export for Vocabulary Gloss

## Overview
Add "Anki CSV" as a new export format to the GlossTab.qml export and copy features, allowing users to export vocabulary glosses in a CSV format compatible with Anki flashcard import.

## Background
Currently, users can export/copy vocabulary gloss data in three formats: HTML, Markdown, and Org-Mode. Adding Anki CSV export enables users to directly import Pāli vocabulary into Anki for spaced repetition learning.

## User Story
As a user studying Pāli vocabulary, I want to export my vocabulary gloss data as an Anki-compatible CSV file, so that I can create flashcards for spaced repetition learning with the word stem on the front and dictionary definitions on the back.

## Feature Specification

### UI Changes

#### Export ComboBox (Global Export)
- **File:** `assets/qml/GlossTab.qml`
- **Line:** ~1472
- **Current model:** `["Export As...", "HTML", "Markdown", "Org-Mode"]`
- **Updated model:** `["Export As...", "HTML", "Markdown", "Org-Mode", "Anki CSV"]`

#### Copy ComboBox (Per-Paragraph Copy)
- **File:** `assets/qml/GlossTab.qml`
- **Line:** ~1764
- **Current model:** `["Copy As...", "HTML", "Markdown", "Org-Mode"]`
- **Updated model:** `["Copy As...", "HTML", "Markdown", "Org-Mode", "Anki CSV"]`

### Anki CSV Format Specification

#### CSV Structure
- **Two columns:** Front, Back
- **No header row** - First row contains data, not column names
- **Field separator:** Comma (`,`)
- **Text qualifier:** Double quotes (`"`)
- **Line separator:** `\n`

#### Front Field (Word Stem)
- Contains the word in its stem form
- Clean stem (disambiguating numbers removed)
- Example: `"dhamma"` (from `"dhamma 1.01"`)

#### Back Field (Meaning/Gloss)
- Contains the dictionary definition/summary **with HTML formatting preserved**
- HTML tags (like `<b>`, `<i>`) are kept for rich formatting in Anki
- Escape special CSV characters:
  - Double quotes → `""`
  - Preserve commas and newlines within quoted fields

#### CSV Output Example
```csv
"ariyasāvaka","<b>ariyasāvaka</b> <i>(masc)</i> noble disciple; follower of the noble ones"
"vossaggārammaṇa","<b>vossaggārammaṇa</b> <i>(nt)</i> object of letting go; support for relinquishment"
"karitvā","<i>(ind)</i> having done, having made"
"samādhi","<b>samādhi</b> <i>(masc)</i> concentration; unification of mind; mental focus"
"citta","<b>citta</b> <i>(nt)</i> mind, heart, consciousness"
"ekaggata","<b>ekaggata</b> <i>(fem)</i> one-pointedness; unification"
```

### Data Flow

#### Global Export Flow
```
export_btn.onCurrentIndexChanged
  → export_dialog_accepted()
  → gloss_as_anki_csv()
  → gloss_export_data() [processes all paragraphs]
  → format to CSV
  → SuttaBridge.save_file() with filename "gloss_export.csv"
```

#### Per-Paragraph Copy Flow
```
copy_combobox.onCurrentIndexChanged
  → paragraph_gloss_as_anki_csv(paragraph_index)
  → [processes single paragraph vocabulary data]
  → format to CSV
  → clip.copy_text() [copies to clipboard]
  → show "Copied!" message animation
  → reset ComboBox to index 0
```

### Code Architecture

#### New Functions (QML)

**Global Export:**
```qml
function gloss_as_anki_csv(): string
```
- Calls `gloss_export_data()` to get all vocabulary data
- Iterates through all paragraphs
- Formats each vocabulary word as CSV row
- Returns complete CSV string with header

**Per-Paragraph Export:**
```qml
function paragraph_gloss_as_anki_csv(paragraph_index: int): string
```
- Validates paragraph_index
- Gets gloss_export_data() for the specific paragraph
- Formats vocabulary words as CSV rows
- Returns CSV string with header

#### Helper Functions (Reuse Existing)
- `gloss_export_data()` - Extract vocabulary data from paragraph model
- `clean_stem(stem: string)` - Remove disambiguating numbers from stems

#### CSV Formatting Logic

**Escape CSV field:**
```qml
function escape_csv_field(field: string): string {
    // Replace double quotes with two double quotes
    field = field.replace(/"/g, '""');
    // Wrap in quotes if contains comma, newline, or quote
    if (field.includes(',') || field.includes('\n') || field.includes('"')) {
        field = '"' + field + '"';
    }
    return field;
}
```

**Format CSV row:**
```qml
function format_csv_row(front: string, back: string): string {
    return escape_csv_field(front) + "," + escape_csv_field(back);
}
```

### Implementation Details

#### gloss_as_anki_csv() Implementation
```qml
function gloss_as_anki_csv(): string {
    let gloss_data = root.gloss_export_data();
    
    let csv_lines = [];
    
    for (var i = 0; i < gloss_data.paragraphs.length; i++) {
        var paragraph = gloss_data.paragraphs[i];
        
        for (var j = 0; j < paragraph.vocabulary.length; j++) {
            var vocab = paragraph.vocabulary[j];
            var front = root.clean_stem(vocab.word);
            var back = vocab.summary;
            
            csv_lines.push(root.format_csv_row(front, back));
        }
    }
    
    return csv_lines.join("\n");
}
```

#### paragraph_gloss_as_anki_csv() Implementation
```qml
function paragraph_gloss_as_anki_csv(paragraph_index: int): string {
    if (paragraph_index < 0 || paragraph_index >= paragraph_model.count) {
        logger.error("Invalid paragraph index:", paragraph_index);
        return "";
    }
    
    let gloss_data = root.gloss_export_data();
    if (paragraph_index >= gloss_data.paragraphs.length) {
        logger.error("Paragraph index out of range:", paragraph_index);
        return "";
    }
    
    var paragraph = gloss_data.paragraphs[paragraph_index];
    let csv_lines = [];
    
    for (var j = 0; j < paragraph.vocabulary.length; j++) {
        var vocab = paragraph.vocabulary[j];
        var front = root.clean_stem(vocab.word);
        var back = vocab.summary;
        
        csv_lines.push(root.format_csv_row(front, back));
    }
    
    return csv_lines.join("\n");
}
```

#### Export Dialog Handler Update
In `export_dialog_accepted()` (line ~232), add new condition:

```qml
function export_dialog_accepted() {
    if (export_btn.currentIndex === 0) return;
    let save_file_name = null
    let save_content = null;

    if (export_btn.currentValue === "HTML") {
        save_file_name = "gloss_export.html";
        save_content = root.gloss_as_html();
    } else if (export_btn.currentValue === "Markdown") {
        save_file_name = "gloss_export.md";
        save_content = root.gloss_as_markdown();
    } else if (export_btn.currentValue === "Org-Mode") {
        save_file_name = "gloss_export.org";
        save_content = root.gloss_as_orgmode();
    } else if (export_btn.currentValue === "Anki CSV") {
        save_file_name = "gloss_export.csv";
        save_content = root.gloss_as_anki_csv();
    }

    // ... rest of function
}
```

#### Copy ComboBox Handler Update
In copy_combobox.onCurrentIndexChanged (line ~1768), add new condition:

```qml
onCurrentIndexChanged: {
    if (currentIndex === 0) {
        return;
    }

    var content = "";
    if (currentIndex === 1) {
        content = root.paragraph_gloss_as_html(paragraph_item.index);
    } else if (currentIndex === 2) {
        content = root.paragraph_gloss_as_markdown(paragraph_item.index);
    } else if (currentIndex === 3) {
        content = root.paragraph_gloss_as_orgmode(paragraph_item.index);
    } else if (currentIndex === 4) {
        content = root.paragraph_gloss_as_anki_csv(paragraph_item.index);
    }

    if (content.length > 0) {
        paragraph_clip.copy_text(content);
        copied_message_animation.start();
    }

    copy_combobox.currentIndex = 0;
}
```

### Testing Requirements

#### QML Tests
Create tests in `assets/qml/tst_GlossTab.qml`:

**Test 1: CSV escape function**
```qml
function test_escape_csv_field() {
    // Test normal text (no escaping needed)
    compare(gloss_tab.escape_csv_field("simple"), "simple");
    
    // Test text with comma (should be quoted)
    compare(gloss_tab.escape_csv_field("hello, world"), '"hello, world"');
    
    // Test text with quotes (should escape and quote)
    compare(gloss_tab.escape_csv_field('say "hello"'), '"say ""hello"""');
    
    // Test text with newline (should be quoted)
    compare(gloss_tab.escape_csv_field("line1\nline2"), '"line1\nline2"');
    
    // Test text with all special chars
    compare(gloss_tab.escape_csv_field('test, "quoted"\ntext'), '"test, ""quoted""\ntext"');
}
```

**Test 2: CSV row formatting**
```qml
function test_format_csv_row() {
    compare(gloss_tab.format_csv_row("front", "back"), "front,back");
    compare(gloss_tab.format_csv_row("dhamma", "teaching, doctrine"), '"dhamma","teaching, doctrine"');
    compare(gloss_tab.format_csv_row("word", 'meaning with "quotes"'), 'word,"meaning with ""quotes"""');
}
```

**Test 3: Global Anki CSV export**
```qml
function test_gloss_as_anki_csv() {
    var paragraph = "Idha, bhikkhave, ariyasāvako vossaggārammaṇaṁ karitvā labhati samādhiṁ, labhati cittassa ekaggataṁ.";
    processTextBackground(paragraph);
    
    var csv = gloss_tab.gloss_as_anki_csv();
    
    // Verify header
    verify(csv.startsWith("Front,Back\n"));
    
    // Verify contains vocabulary words
    verify(csv.includes("ariyasāvaka"));
    verify(csv.includes("karitvā"));
    
    // Verify no HTML tags in output
    verify(!csv.includes("<b>"));
    verify(!csv.includes("</b>"));
    verify(!csv.includes("<i>"));
    verify(!csv.includes("</i>"));
    
    // Verify clean stems (no disambiguating numbers)
    var lines = csv.split("\n");
    for (var i = 1; i < lines.length; i++) {
        if (lines[i].trim() === "") continue;
        var front = lines[i].split(",")[0].replace(/"/g, "");
        // Front field should not contain space + number pattern
        verify(!front.match(/\s+\d+(\.\d+)?$/));
    }
}
```

**Test 4: Per-paragraph Anki CSV export**
```qml
function test_paragraph_gloss_as_anki_csv() {
    var paragraph1 = "Idha, bhikkhave, ariyasāvako vossaggārammaṇaṁ karitvā.";
    var paragraph2 = "Labhati samādhiṁ, labhati cittassa ekaggataṁ.";
    processTextBackground(paragraph1 + "\n\n" + paragraph2);
    
    // Test first paragraph export
    var csv = gloss_tab.paragraph_gloss_as_anki_csv(0);
    
    verify(csv.startsWith("Front,Back\n"));
    verify(csv.includes("ariyasāvaka"));
    verify(csv.includes("karitvā"));
    
    // Should NOT include words from second paragraph
    verify(!csv.includes("samādhi") || csv.includes("samādhi") === false);
    
    // Test invalid index
    csv = gloss_tab.paragraph_gloss_as_anki_csv(-1);
    compare(csv, "");
    
    csv = gloss_tab.paragraph_gloss_as_anki_csv(999);
    compare(csv, "");
}
```

**Test 5: Verify CSV format integrity**
```qml
function test_anki_csv_format_integrity() {
    var paragraph = "Idha, bhikkhave, ariyasāvako karitvā.";
    processTextBackground(paragraph);
    
    var csv = gloss_tab.gloss_as_anki_csv();
    var lines = csv.split("\n");
    
    // Verify header is present
    compare(lines[0], "Front,Back");
    
    // Verify each data line has exactly 2 fields
    for (var i = 1; i < lines.length; i++) {
        if (lines[i].trim() === "") continue;
        
        // Parse CSV line (simple parser for testing)
        var fields = [];
        var current_field = "";
        var in_quotes = false;
        
        for (var j = 0; j < lines[i].length; j++) {
            var char = lines[i][j];
            
            if (char === '"' && (j === 0 || lines[i][j-1] !== '"')) {
                in_quotes = !in_quotes;
            } else if (char === ',' && !in_quotes) {
                fields.push(current_field);
                current_field = "";
            } else {
                current_field += char;
            }
        }
        fields.push(current_field);
        
        // Should have exactly 2 fields (Front and Back)
        compare(fields.length, 2);
    }
}
```

## Success Criteria

1. ✅ "Anki CSV" option appears in export_btn ComboBox
2. ✅ "Anki CSV" option appears in copy_combobox ComboBox
3. ✅ Global export creates valid CSV file with all vocabulary words
4. ✅ Per-paragraph copy creates valid CSV with single paragraph vocabulary
5. ✅ CSV has no header row (first row contains data)
6. ✅ Front field contains clean stem (no disambiguating numbers)
7. ✅ Back field contains summary with HTML formatting preserved
8. ✅ Special CSV characters are properly escaped
9. ✅ Multi-paragraph export includes all vocabulary without duplicates
10. ✅ CSV can be successfully imported into Anki
11. ✅ All QML tests pass
12. ✅ Existing export/copy formats remain unchanged
13. ✅ Code follows snake_case naming conventions

## Out of Scope

- Custom CSV column configuration (only Front/Back)
- Including AI translations in CSV export
- Including paragraph text in CSV
- Tags or deck metadata in CSV
- Alternative CSV delimiters (only comma)
- Automatic Anki import (only file export)
- Duplicate detection across multiple exports
- Including pronunciation or etymology data
- Media/audio attachments
- Custom Anki note types

## Dependencies

- Existing functions: `gloss_export_data()`, `clean_stem()`
- Qt/QML clipboard functionality via TextEdit component
- No new Rust bridge functions required
- No new external dependencies

## File Changes Summary

**Modified Files:**
- `assets/qml/GlossTab.qml`
  - Update export_btn model to include "Anki CSV" (line ~1472)
  - Update copy_combobox model to include "Anki CSV" (line ~1764)
  - Update export_dialog_accepted() to handle Anki CSV export (line ~232)
  - Update copy_combobox.onCurrentIndexChanged to handle Anki CSV copy (line ~1768)
  - Add new function: `escape_csv_field(field: string): string`
  - Add new function: `format_csv_row(front: string, back: string): string`
  - Add new function: `gloss_as_anki_csv(): string`
  - Add new function: `paragraph_gloss_as_anki_csv(paragraph_index: int): string`

**Test Files:**
- `assets/qml/tst_GlossTab.qml`
  - Add test: `test_escape_csv_field()`
  - Add test: `test_format_csv_row()`
  - Add test: `test_gloss_as_anki_csv()`
  - Add test: `test_paragraph_gloss_as_anki_csv()`
  - Add test: `test_anki_csv_format_integrity()`

## Implementation Order

1. Implement CSV helper functions (`escape_csv_field`, `format_csv_row`)
2. Implement `paragraph_gloss_as_anki_csv()` (single paragraph first for easier testing)
3. Implement `gloss_as_anki_csv()` (reuse paragraph function logic)
4. Update export_btn ComboBox model
5. Update export_dialog_accepted() handler
6. Update copy_combobox ComboBox model
7. Update copy_combobox.onCurrentIndexChanged handler
8. Write QML tests
9. Manual testing with Anki import

## Anki Import Verification

After implementation, verify CSV can be imported into Anki:

1. Export vocabulary gloss as "gloss_export.csv"
2. Open Anki
3. File → Import → Select CSV file
4. Configure import:
   - Field 1 → Front
   - Field 2 → Back
   - Field separator: Comma
5. Verify flashcards created correctly:
   - Front shows clean stem word
   - Back shows dictionary definition
   - No HTML artifacts
   - Special characters display correctly

## Notes

- CSV format follows RFC 4180 standard
- Anki supports UTF-8 encoding (Pāli diacritics will work)
- Each vocabulary word becomes one flashcard
- Duplicate stems across paragraphs will create duplicate cards (user can handle in Anki)
- HTML tags must be stripped to avoid display issues in Anki
- Clean stem ensures consistency (e.g., "dhamma" not "dhamma 1.01")
