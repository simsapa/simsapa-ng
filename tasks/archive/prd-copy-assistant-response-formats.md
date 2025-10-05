# PRD: Copy Assistant Response in Multiple Formats

## Overview

Add a "Copy As..." ComboBox to assistant role messages in PromptsTab that allows users to copy individual assistant responses to the clipboard in HTML, Markdown, or Org-Mode format.

## Motivation

Users currently can export the entire chat conversation to different formats using the "Export As..." button, but there's no quick way to copy a single assistant response in a specific format. This feature provides a convenient way to extract and share individual assistant responses in the user's preferred markup format.

## Current State

- PromptsTab.qml has export functionality via `export_btn` ComboBox that exports the entire chat using `chat_as_html()`, `chat_as_markdown()`, and `chat_as_orgmode()` functions
- These functions process all messages in `messages_model` and format them for export
- GlossTab.qml has similar per-paragraph copy functionality with a success message animation
- SuttaSearchWindow.qml has an invisible `TextEdit` helper (id: `clip`) for clipboard operations

## Proposed Changes

### 1. Add Tests for Existing Export Functions

**File:** Create `assets/qml/test_PromptsTab.qml` (or similar test file)

**Requirements:**
- Create QML tests for `chat_as_html()`, `chat_as_markdown()`, `chat_as_orgmode()`
- Use mock data that represents a realistic chat conversation with:
  - System message
  - User messages
  - Assistant messages with multiple model responses
  - Selected and unselected responses
- Capture the **entire** generated output (HTML, Markdown, Org-Mode)
- Test against the complete expected output, not just string snippets
- This ensures functionality doesn't change during refactoring

**Test Coverage:**
- Verify HTML structure with proper tags, blockquotes, and model name headers
- Verify Markdown formatting with proper heading levels and blockquote syntax
- Verify Org-Mode formatting with proper heading levels and begin_quote/end_quote blocks
- Verify selected indicator "(selected)" appears correctly
- Verify handling of empty/missing responses

### 2. Refactor Export Functions

**File:** `assets/qml/PromptsTab.qml`

**Extract Helper Functions:**

Create three new helper functions that format a single message's response data:

```qml
function message_as_html(msg_data: var): string
function message_as_markdown(msg_data: var): string  
function message_as_orgmode(msg_data: var): string
```

**Input Format:**
These functions should accept the message data structure:
```javascript
{
    role: "system" | "user" | "assistant",
    content: "...",
    responses: [
        {
            model_name: "...",
            response: "...",
            is_selected: true/false
        }
    ]
}
```

**Output Format:**
- Return formatted string for a single message
- Include role heading (e.g., "## User", "## Assistant")
- For assistant messages, include all responses with model names
- Mark selected responses with "(selected)" indicator

**Update Existing Functions:**

Refactor `chat_as_html()`, `chat_as_markdown()`, `chat_as_orgmode()` to:
1. Call `chat_export_data()` to get structured data
2. Loop through messages and call corresponding `message_as_*()` helper
3. Combine results into final export string

**Reference Implementation:**

See GlossTab.qml for similar pattern:
- `gloss_export_data()` - exports structured data
- `format_paragraph_html()`, `format_paragraph_markdown()`, `format_paragraph_orgmode()` - format single items
- `gloss_as_html()`, `gloss_as_markdown()`, `gloss_as_orgmode()` - combine all items

### 3. Add Copy ComboBox to Assistant Messages

**File:** `assets/qml/PromptsTab.qml`

**Location:** In the `messages_component`, after the `Label` with `id: msg_role` (line 642)

**Requirements:**

Add a RowLayout containing:
1. Invisible TextEdit helper for clipboard (similar to SuttaSearchWindow.qml line 1176-1184)
2. Success message Text element with fade animation (similar to GlossTab.qml line 1716-1760)
3. ComboBox with model: `["Copy As...", "HTML", "Markdown", "Org-Mode"]`

**Visibility:**
- Show only when `message_item.role === "assistant"`
- Position on the right side of the Label

**Behavior:**

When user selects a format (index > 0):
1. Get the currently selected response for this assistant message using `selected_ai_tab` index
2. Build message data structure with the selected response marked as `is_selected: true`
3. Call appropriate helper function:
   - Index 1 (HTML): `message_as_html(msg_data)`
   - Index 2 (Markdown): `message_as_markdown(msg_data)`
   - Index 3 (Org-Mode): `message_as_orgmode(msg_data)`
4. Copy result to clipboard using invisible TextEdit helper
5. Show "Copied!" success message with fade in/out animation
6. Reset ComboBox to index 0 ("Copy As...")

**UI Layout:**

```
[Collapse Button] [Label: "assistant"]  [Copied!] [Copy As... ▼]
```

The success message should:
- Appear between the Label and ComboBox
- Initially invisible with opacity 0
- Fade in (200ms) when copy succeeds
- Stay visible for 1500ms
- Fade out (300ms)
- Use green color (#4CAF50) to indicate success

### 4. Testing

**Unit Tests:**
- Run QML tests to verify refactored functions produce identical output
- Verify all export formats still work correctly
- Test edge cases: empty messages, no responses, single response

**Manual Testing:**
1. Open PromptsTab and send a message to multiple AI models
2. Wait for responses to complete
3. Test copying assistant response in each format
4. Verify clipboard contains correctly formatted content
5. Verify success message appears and fades correctly
6. Test with single response vs multiple responses
7. Verify different tab selections copy the correct response

**Regression Testing:**
- Verify existing "Export As..." functionality still works
- Verify chat export includes all messages correctly
- Verify format selection and folder dialog work as before

## Technical Notes

### Code Style
- Use lowercase snake_case for function names (e.g., `message_as_html`)
- Follow existing patterns in PromptsTab.qml and GlossTab.qml
- Maintain consistent spacing and layout conventions

### Dependencies
- No new dependencies required
- Uses existing `SuttaBridge.markdown_to_html()` for HTML conversion
- Uses existing QML components: ComboBox, Text, TextEdit, SequentialAnimation

### Performance
- Formatting a single message should be very fast (< 10ms)
- Clipboard copy is synchronous and immediate
- Animation runs independently without blocking UI

## Success Criteria

1. ✅ QML tests created for existing export functions with full output validation
2. ✅ Helper functions extracted: `message_as_html()`, `message_as_markdown()`, `message_as_orgmode()`
3. ✅ Existing export functions refactored to use helpers
4. ✅ All existing tests pass with identical output
5. ✅ Copy ComboBox appears for assistant messages only
6. ✅ Copying works correctly for all three formats
7. ✅ Success message displays with proper animation
8. ✅ Clipboard receives correctly formatted content
9. ✅ Selected response is properly identified and copied
10. ✅ No regressions in existing export functionality

## Future Enhancements

- Add keyboard shortcut for copying (e.g., Ctrl+Shift+C)
- Allow copying multiple responses at once
- Add "Copy All Responses" option to include all model responses for one message
- Support copying with custom templates
- Add format preview before copying
