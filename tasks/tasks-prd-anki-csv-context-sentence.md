# Tasks: Enhanced Anki CSV Export with Context Sentences

## Relevant Files

- `backend/src/helpers.rs:357` - GlossWordContext struct definition
- `backend/src/helpers.rs:361` - extract_words_with_context() function
- `assets/qml/GlossTab.qml:1303` - gloss_as_anki_csv() function
- `assets/qml/GlossTab.qml` - paragraph_gloss_as_anki_csv() function
- `backend/tests/` - Unit tests for word extraction and context

### Notes

- Use `cd backend && cargo test` to run all backend tests
- Use `cd backend && cargo test test_name` to run specific test function
- Use `make build -B` to verify compilation after changes
- Unit tests should be placed in `backend/tests/` directory
- Backend uses anyhow::Result for error handling

## Tasks

- [ ] 1.0 Extend GlossWordContext struct with new fields
  - [ ] 1.1 Add `original_word: String` field to GlossWordContext in backend/src/helpers.rs:357
  - [ ] 1.2 Add `context_snippet: String` field to GlossWordContext in backend/src/helpers.rs:357
  - [ ] 1.3 Update all references to GlossWordContext constructor calls throughout codebase
  - [ ] 1.4 Verify existing code compiles with new struct definition

- [ ] 2.0 Implement sentence boundary detection helper function
  - [ ] 2.1 Create `find_sentence_start()` helper function in backend/src/helpers.rs
  - [ ] 2.2 Create `find_sentence_end()` helper function in backend/src/helpers.rs
  - [ ] 2.3 Implement detection for period (`.`), question mark (`?`), exclamation mark (`!`)
  - [ ] 2.4 Handle edge cases: start of text, end of text, no boundaries found
  - [ ] 2.5 Add unit tests for sentence boundary detection

- [ ] 3.0 Rewrite extract_words_with_context() algorithm
  - [ ] 3.1 Store original text before regex transformations in backend/src/helpers.rs:361
  - [ ] 3.2 Create word position tracking using split_whitespace().enumerate()
  - [ ] 3.3 For each word, extract 50-char context window before and after from original text
  - [ ] 3.4 Apply sentence boundary detection to trim context snippets
  - [ ] 3.5 Store original_word form from untransformed text
  - [ ] 3.6 Apply regex operations to get clean_word
  - [ ] 3.7 Implement <b> tag wrapping around first occurrence of original word in context
  - [ ] 3.8 Handle UTF-8 multi-byte characters correctly (Pāli diacritics)
  - [ ] 3.9 Handle edge cases: word near start/end of text, word appears multiple times

- [ ] 4.0 Add comprehensive unit tests for extract_words_with_context()
  - [ ] 4.1 Create test file backend/tests/test_extract_words_with_context.rs
  - [ ] 4.2 Test mid-sentence word extraction (e.g., "Katamañca bhikkhave samādhindriyaṁ?")
  - [ ] 4.3 Test word at sentence boundary (e.g., "Word1 word2. Word3 word4.")
  - [ ] 4.4 Test first word in text (e.g., "Bhikkhave, listen carefully.")
  - [ ] 4.5 Test last word in text
  - [ ] 4.6 Test word near start of text (< 50 chars before)
  - [ ] 4.7 Test word near end of text (< 50 chars after)
  - [ ] 4.8 Test multiple occurrences of same word in snippet (verify only first is wrapped)
  - [ ] 4.9 Test Pāli diacritical marks (ā, ī, ū, ṃ, ṁ, ṅ, ñ, ṭ, ḍ, ṇ, ḷ)
  - [ ] 4.10 Test sandhi transformations (e.g., dhārayāmī'ti → dhārayāmi ti)
  - [ ] 4.11 Verify all existing word extraction tests still pass

- [ ] 5.0 Update gloss_as_anki_csv() in GlossTab.qml
  - [ ] 5.1 Locate gloss_as_anki_csv() function in assets/qml/GlossTab.qml:1303
  - [ ] 5.2 Update Front field format to include two <p> paragraphs
  - [ ] 5.3 First <p> tag contains clean_word stem
  - [ ] 5.4 Second <p> tag contains context_snippet with <b> wrapped original word
  - [ ] 5.5 Ensure existing escape_csv_field() function handles new format correctly
  - [ ] 5.6 Test CSV generation with sample vocabulary data

- [ ] 6.0 Update paragraph_gloss_as_anki_csv() in GlossTab.qml
  - [ ] 6.1 Locate paragraph_gloss_as_anki_csv() function in assets/qml/GlossTab.qml
  - [ ] 6.2 Apply same two-paragraph Front field format as gloss_as_anki_csv()
  - [ ] 6.3 Ensure context_snippet is available in paragraph vocabulary data
  - [ ] 6.4 Test CSV generation for paragraph-specific exports

- [ ] 7.0 Integration testing with real Pāli content
  - [ ] 7.1 Test Anki CSV export with actual sutta passage (e.g., DN22 excerpt)
  - [ ] 7.2 Verify context sentences appear for 100% of vocabulary words
  - [ ] 7.3 Verify sentence boundaries are correctly detected and trimmed
  - [ ] 7.4 Verify words are properly highlighted with <b> tags in context
  - [ ] 7.5 Test CSV escaping with special characters (commas, quotes, newlines in context)
  - [ ] 7.6 Import generated CSV into Anki to verify format compatibility
  - [ ] 7.7 Verify HTML rendering in Anki cards (both paragraphs display correctly)

- [ ] 8.0 Final verification and cleanup
  - [ ] 8.1 Run full backend test suite: cd backend && cargo test
  - [ ] 8.2 Verify make build -B compiles without errors or warnings
  - [ ] 8.3 Check for any TODO or FIXME comments added during implementation
  - [ ] 8.4 Verify backward compatibility with existing gloss display (non-CSV exports unchanged)
  - [ ] 8.5 Update PROJECT_MAP.md if significant new functions or modules were added
