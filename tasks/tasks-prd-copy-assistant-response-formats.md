# Tasks: Copy Assistant Response in Multiple Formats

## Phase 1: Test Creation

### Task 1.1: Create QML Test File
- [x] Create test file for PromptsTab export functions
- [x] Set up test infrastructure with mock data
- [x] Create realistic chat conversation test data with:
  - System message
  - Multiple user messages
  - Multiple assistant messages with multi-model responses
  - Mix of selected and unselected responses

### Task 1.2: Write Tests for chat_as_html()
- [x] Create test with expected complete HTML output
- [x] Verify HTML structure: doctype, html, head, body tags
- [x] Verify message role headings (h2 tags)
- [x] Verify blockquote formatting
- [x] Verify model name headings (h3 tags) for assistant responses
- [x] Verify "(selected)" indicator appears correctly
- [x] Verify markdown-to-html conversion for assistant content
- [x] Test edge cases: empty messages, missing content

### Task 1.3: Write Tests for chat_as_markdown()
- [x] Create test with expected complete Markdown output
- [x] Verify heading levels (# for title, ## for roles, ### for models)
- [x] Verify blockquote syntax (> prefix)
- [x] Verify line breaks in blockquotes
- [x] Verify "(selected)" indicator
- [x] Test edge cases

### Task 1.4: Write Tests for chat_as_orgmode()
- [x] Create test with expected complete Org-Mode output
- [x] Verify heading levels (* for title, ** for roles, *** for models)
- [x] Verify #+begin_quote/#+end_quote blocks
- [x] Verify #+begin_src markdown/#+end_src blocks for responses
- [x] Verify asterisk-to-dash conversion for lists
- [x] Verify "(selected)" indicator
- [x] Test edge cases

### Task 1.5: Run Initial Tests
- [x] Execute tests with current implementation
- [x] Verify all tests pass
- [x] Document exact expected output for regression testing
- [x] Commit test file

## Phase 2: Refactoring Export Functions

### Task 2.1: Create message_as_html() Helper
- [ ] Extract HTML formatting logic for single message
- [ ] Handle system role messages
- [ ] Handle user role messages
- [ ] Handle assistant role messages with responses
- [ ] Include model names and "(selected)" indicator
- [ ] Apply markdown-to-html conversion for assistant responses
- [ ] Return formatted HTML string

### Task 2.2: Create message_as_markdown() Helper
- [ ] Extract Markdown formatting logic for single message
- [ ] Handle system role messages
- [ ] Handle user role messages
- [ ] Handle assistant role messages with responses
- [ ] Include model names and "(selected)" indicator
- [ ] Apply proper blockquote formatting
- [ ] Return formatted Markdown string

### Task 2.3: Create message_as_orgmode() Helper
- [ ] Extract Org-Mode formatting logic for single message
- [ ] Handle system role messages
- [ ] Handle user role messages
- [ ] Handle assistant role messages with responses
- [ ] Include model names and "(selected)" indicator
- [ ] Apply proper quote block and src block formatting
- [ ] Convert asterisk lists to dash lists
- [ ] Return formatted Org-Mode string

### Task 2.4: Refactor chat_as_html()
- [ ] Update to use message_as_html() helper
- [ ] Maintain exact same output as before
- [ ] Remove duplicated formatting code
- [ ] Test against baseline tests

### Task 2.5: Refactor chat_as_markdown()
- [ ] Update to use message_as_markdown() helper
- [ ] Maintain exact same output as before
- [ ] Remove duplicated formatting code
- [ ] Test against baseline tests

### Task 2.6: Refactor chat_as_orgmode()
- [ ] Update to use message_as_orgmode() helper
- [ ] Maintain exact same output as before
- [ ] Remove duplicated formatting code
- [ ] Test against baseline tests

### Task 2.7: Run Regression Tests
- [ ] Run all QML tests
- [ ] Verify output matches exactly
- [ ] Fix any discrepancies
- [ ] Commit refactored code

## Phase 3: Add Copy Functionality to UI

### Task 3.1: Add Invisible Clipboard Helper
- [ ] Add TextEdit element with visible: false
- [ ] Add copy_text() function similar to SuttaSearchWindow.qml
- [ ] Place in messages_component delegate
- [ ] Give it appropriate id (e.g., message_clip)

### Task 3.2: Add Success Message Element
- [ ] Add Text element for "Copied!" message
- [ ] Set initial visibility to false
- [ ] Set initial opacity to 0
- [ ] Use green color (#4CAF50)
- [ ] Position before ComboBox in RowLayout

### Task 3.3: Add Success Message Animation
- [ ] Create SequentialAnimation
- [ ] Add PropertyAction to set visible: true
- [ ] Add NumberAnimation for fade in (opacity 0 → 1.0, 200ms)
- [ ] Add PauseAnimation (1500ms)
- [ ] Add NumberAnimation for fade out (opacity 1.0 → 0, 300ms)
- [ ] Add PropertyAction to set visible: false

### Task 3.4: Add Copy ComboBox
- [ ] Add ComboBox to RowLayout after msg_role Label
- [ ] Set model: ["Copy As...", "HTML", "Markdown", "Org-Mode"]
- [ ] Set currentIndex: 0
- [ ] Set visibility: message_item.role === "assistant"
- [ ] Position on right side of RowLayout

### Task 3.5: Implement Copy Logic
- [ ] Add onCurrentIndexChanged handler
- [ ] Return early if currentIndex === 0
- [ ] Get current message from messages_model using message_item.index
- [ ] Get selected_ai_tab index
- [ ] Parse responses_json
- [ ] Build message data structure with selected response marked
- [ ] Call appropriate helper function based on currentIndex
- [ ] Copy result using message_clip.copy_text()
- [ ] Start success message animation
- [ ] Reset ComboBox to index 0

### Task 3.6: Handle Edge Cases
- [ ] Handle missing responses_json
- [ ] Handle empty responses array
- [ ] Handle invalid selected_ai_tab index
- [ ] Handle incomplete/error responses
- [ ] Show error message if copy fails

## Phase 4: Testing and Polish

### Task 4.1: Manual UI Testing
- [ ] Test copy functionality with single model response
- [ ] Test copy functionality with multiple model responses
- [ ] Test all three formats (HTML, Markdown, Org-Mode)
- [ ] Test with different selected tabs
- [ ] Verify clipboard content is correct
- [ ] Verify success message animation
- [ ] Test rapid clicking (shouldn't break)

### Task 4.2: Verify Existing Functionality
- [ ] Test "Export As..." button still works
- [ ] Test folder dialog appears correctly
- [ ] Test file save with all formats
- [ ] Test overwrite confirmation
- [ ] Verify full chat export is identical

### Task 4.3: Cross-Platform Testing
- [ ] Test on Linux desktop
- [ ] Test on Android (if applicable)
- [ ] Verify clipboard works on all platforms
- [ ] Verify animations are smooth

### Task 4.4: Code Review and Cleanup
- [ ] Review code style (snake_case, spacing, etc.)
- [ ] Add appropriate comments
- [ ] Remove any debug logging
- [ ] Check for memory leaks
- [ ] Verify no binding loops

### Task 4.5: Documentation
- [ ] Update AGENTS.md if needed
- [ ] Update any relevant user documentation
- [ ] Add inline code comments for complex logic
- [ ] Document the new feature in commit message

## Phase 5: Finalization

### Task 5.1: Final Testing
- [ ] Run all QML tests
- [ ] Run manual test suite
- [ ] Test all user scenarios
- [ ] Verify no regressions

### Task 5.2: Commit and Push
- [ ] Commit test file
- [ ] Commit refactored functions
- [ ] Commit UI changes
- [ ] Write comprehensive commit message
- [ ] Push to repository

### Task 5.3: Mark PRD Complete
- [ ] Update PRD success criteria checkboxes
- [ ] Move PRD and tasks to archive
- [ ] Update PROJECT_MAP.md if significant changes

## Notes

- Keep changes focused on the PRD scope
- Don't add unrelated features
- Maintain backward compatibility
- Follow existing code patterns
- Test thoroughly before committing
