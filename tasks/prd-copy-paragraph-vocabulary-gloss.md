# PRD: Copy Paragraph Vocabulary Gloss to Clipboard

## Overview
Add a dropdown "Copy As..." feature to the vocabulary gloss section in GlossTab.qml that allows users to copy a single paragraph's dictionary definitions to the clipboard in different formats (HTML, Markdown, Org-Mode).

## Background
Currently, users can export the entire gloss document with all paragraphs to files using the export_btn ComboBox. This feature extends that functionality by allowing users to quickly copy just one paragraph's vocabulary data to the clipboard for immediate use.

## User Story
As a user studying Pāli texts, I want to copy the vocabulary definitions for a single paragraph to my clipboard in different formats, so that I can paste them into my notes or other documents without having to export the entire gloss.

## Feature Specification

### UI Changes

#### Location
- File: `assets/qml/GlossTab.qml`
- Line: ~1644 (in the paragraph_gloss_component)
- Element: Text with text "Dictionary definitions from DPD:"

#### Implementation
1. **Wrap the existing Text element in a RowLayout**
   - The Text element "Dictionary definitions from DPD:" should remain on the left
   - A "Copied!" success message Text should appear in the middle (initially invisible)
   - The new ComboBox should appear on the right side

2. **Add "Copied!" success message with fade animation**
   - Text element with text "Copied!"
   - Initially: `visible: false` and `opacity: 0`
   - Animation behavior:
     - After successful clipboard copy, set `visible: true`
     - Fade in: opacity 0 → 1.0 over 200ms
     - Hold: stay at opacity 1.0 for 1500ms
     - Fade out: opacity 1.0 → 0 over 300ms
     - After fade out: set `visible: false`
   - Styling:
     - Green color to indicate success (e.g., `color: "#4CAF50"`)
     - Same font size as the header text
     - Positioned between header Text and ComboBox using `Layout.leftMargin` spacing

3. **Add ComboBox with format options**
   - Model: `["Copy As...", "HTML", "Markdown", "Org-Mode"]`
   - The "Copy As..." option (index 0) should do nothing (default state)
   - When other options are selected:
     - Copy the paragraph's vocabulary gloss data to clipboard in the selected format
     - Trigger the "Copied!" fade animation
     - Reset ComboBox to index 0 after copying

4. **Add invisible clipboard helper**
   - Similar to `TextEdit { id: clip }` in SuttaSearchWindow.qml (line 1176-1184)
   - This will handle the actual clipboard copy operation

### Data Flow

#### Current Export Flow
The existing export functions follow this pattern:
```
export_btn.onCurrentIndexChanged 
  → export_dialog_accepted()
  → gloss_as_html() / gloss_as_markdown() / gloss_as_orgmode()
  → gloss_export_data() [processes all paragraphs]
  → SuttaBridge.save_file()
```

#### New Copy Flow
The new copy feature should follow this pattern:
```
copy_combobox.onCurrentIndexChanged
  → paragraph_gloss_as_html() / paragraph_gloss_as_markdown() / paragraph_gloss_as_orgmode()
  → [processes single paragraph vocabulary data]
  → clip.copy_text() [copies to clipboard]
  → show_copied_message() [triggers fade animation]
  → reset ComboBox to index 0
```

### Code Architecture

#### Function Refactoring

**Current Functions (Process ALL paragraphs):**
- `gloss_as_html()` (line 1143)
- `gloss_as_markdown()` (line 1209)
- `gloss_as_orgmode()` (line 1269)

**New Functions (Process SINGLE paragraph):**
- `paragraph_gloss_as_html(paragraph_index: int): string`
- `paragraph_gloss_as_markdown(paragraph_index: int): string`
- `paragraph_gloss_as_orgmode(paragraph_index: int): string`

#### Implementation Strategy

1. **Extract shared formatting logic**
   - Identify the code sections in existing functions that format vocabulary data
   - Create new functions that accept a paragraph object and return formatted vocabulary text
   - These functions should NOT include:
     - Document headers (e.g., "# Gloss Export", `<html>`, etc.)
     - Main text blockquotes
     - Multiple paragraph loops
   - These functions SHOULD include:
     - Paragraph text blockquote
     - AI translations section (if exists)
     - Vocabulary section header
     - Formatted vocabulary table/list

2. **Reuse existing helper functions**
   - `summary_strip_html()` (line 996)
   - `summary_html_to_md()` (line 1005)
   - `summary_html_to_orgmode()` (line 1015)
   - `gloss_export_data()` pattern for extracting paragraph data

3. **Implement fade animation**
   - Use `SequentialAnimation` with multiple `NumberAnimation` steps for opacity changes
   - Use `onStopped` handler to set `visible: false` after fade out completes
   - Trigger animation using `animation.start()` after successful clipboard copy

### Testing Requirements

#### QML Tests
Create tests in `assets/qml/tst_GlossTab.qml`:

1. **Test existing functionality preservation**
   - Verify `gloss_as_html()`, `gloss_as_markdown()`, `gloss_as_orgmode()` still work correctly
   - Use existing mock data from test file

2. **Test new paragraph-level functions**
   - Create mock paragraph data with:
     - Paragraph text
     - Vocabulary words with selected definitions
     - AI translations (optional)
   - Verify each format function produces correct output:
     - HTML: proper tags, table structure
     - Markdown: proper formatting, table syntax
     - Org-Mode: proper syntax, table structure

3. **Test clipboard functionality**
   - Mock the clipboard copy operation
   - Verify ComboBox resets to index 0 after copy
   - Verify correct format is copied based on selection

4. **Test "Copied!" message animation**
   - Verify message is initially invisible
   - Verify message becomes visible after copy action
   - Verify opacity animation sequence (fade in → hold → fade out)
   - Verify message becomes invisible again after animation completes
   - Note: Animation timing verification may be skipped in unit tests

## Implementation Notes

### QML Animation Pattern

**SequentialAnimation for Fade Effect:**
```qml
SequentialAnimation {
    id: copied_message_animation
    
    PropertyAction {
        target: copied_message
        property: "visible"
        value: true
    }
    
    NumberAnimation {
        target: copied_message
        property: "opacity"
        from: 0
        to: 1.0
        duration: 200
    }
    
    PauseAnimation {
        duration: 1500
    }
    
    NumberAnimation {
        target: copied_message
        property: "opacity"
        from: 1.0
        to: 0
        duration: 300
    }
    
    PropertyAction {
        target: copied_message
        property: "visible"
        value: false
    }
}
```

**Trigger the animation:**
```qml
clip.copy_text(content);
copied_message_animation.start();
copy_combobox.currentIndex = 0;
```

### Data Structure Reference

From `gloss_export_data()` (line 1025-1141):

**Paragraph Model Structure:**
```javascript
{
    text: "paragraph text",
    words_data_json: "[...]",  // Array of word objects
    translations_json: "[...]", // Array of translation objects
    selected_ai_tab: 0
}
```

**Words Data Structure:**
```javascript
{
    original_word: "word",
    results: [
        {
            uid: "...",
            word: "stem",
            summary: "definition with <i>html</i>"
        }
    ],
    selected_index: 0,
    stem: "stem",
    example_sentence: ""
}
```

**Translation Structure:**
```javascript
{
    model_name: "...",
    response: "...",
    status: "completed",
    is_selected: true/false
}
```

### HTML Format Output Example
```html
<blockquote>
[paragraph text]
</blockquote>

<h3>AI Translations</h3>
<h4>model_name (selected)</h4>
<blockquote>[response]</blockquote>

<h3>Vocabulary</h3>
<p><b>Dictionary definitions from DPD:</b></p>
<table><tbody>
<tr><td> <b>word</b> </td><td> summary </td></tr>
</tbody></table>
```

### Markdown Format Output Example
```markdown
> paragraph text

### AI Translations
#### model_name (selected)
> response

### Vocabulary
**Dictionary definitions from DPD:**
|    |    |
|----|----|
| **word** | summary |
```

### Org-Mode Format Output Example
```org
#+begin_quote
paragraph text
#+end_quote

*** AI Translations
**** model_name (selected)
#+begin_src markdown
response
#+end_src

*** Vocabulary
*Dictionary definitions from DPD:*
| *word* | summary |
```

## Success Criteria

1. ✅ User can click "Copy As..." dropdown and select a format
2. ✅ Paragraph vocabulary gloss data is copied to clipboard in the selected format
3. ✅ "Copied!" success message appears and fades in/out smoothly
4. ✅ ComboBox resets to default "Copy As..." after copying
5. ✅ All three formats (HTML, Markdown, Org-Mode) work correctly
6. ✅ Existing export functionality remains unchanged
7. ✅ QML tests pass for both existing and new functionality
8. ✅ Code follows existing patterns and conventions (lowercase snake_case for new functions)
9. ✅ Success message is visible only during the animation (not before or after)

## Out of Scope

- Exporting/copying multiple paragraphs at once
- Adding new export formats beyond HTML, Markdown, and Org-Mode
- Changing the existing export_btn behavior
- Customizable animation timing or colors
- Sound effects or haptic feedback
- Alternative success indicators (e.g., checkmark icon)

## Dependencies

- Existing functions: `gloss_export_data()`, `summary_html_to_md()`, `summary_html_to_orgmode()`, `summary_strip_html()`
- Qt/QML clipboard functionality via TextEdit component
- No new Rust bridge functions required
- No new external dependencies

## File Changes Summary

**Modified Files:**
- `assets/qml/GlossTab.qml`
  - Wrap Text element (line ~1644) in RowLayout
  - Add "Copied!" Text element with fade animation
  - Add SequentialAnimation for fade effect
  - Add ComboBox with copy functionality
  - Add invisible TextEdit clipboard helper
  - Add three new functions: `paragraph_gloss_as_html()`, `paragraph_gloss_as_markdown()`, `paragraph_gloss_as_orgmode()`

**Test Files:**
- `assets/qml/tst_GlossTab.qml`
  - Add tests for new paragraph-level functions
  - Verify existing functions still work
  - Test clipboard copy functionality
