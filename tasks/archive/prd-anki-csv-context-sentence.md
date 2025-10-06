# PRD: Enhanced Anki CSV Export with Context Sentences

## Overview

Enhance the "Anki CSV" export format in GlossTab to include sentence context where each word was found. The Front field will display both the word and its original context sentence, while the Back field continues to show the gloss vocabulary meaning.

## Background

Currently, the Anki CSV export only includes the word stem and its dictionary definition. Users studying Pāli would benefit from seeing each word in its original context to better understand usage patterns and meaning nuances.

The current implementation:
- `GlossWordContext` (backend/src/helpers.rs:357) only stores `clean_word`
- `extract_words_with_context()` (backend/src/helpers.rs:361) performs regex operations on the entire text before splitting into words
- `gloss_as_anki_csv()` (GlossTab.qml:1303) generates simple two-column CSV: word stem, definition

## Goals

1. Extend `GlossWordContext` to capture original word form, cleaned word, and sentence context
2. Modify `extract_words_with_context()` to track word positions before regex transformations
3. Update Anki CSV export to include context sentences with highlighted words
4. Maintain backward compatibility with existing word extraction logic

## Non-Goals

- Modifying other export formats (HTML, Markdown, Org-Mode)
- Changing the UI for gloss display
- Altering how words are looked up in the DPD dictionary
- Adding new export formats

## User Stories

1. As a Pāli student, I want to see each vocabulary word in its original sentence context so I can understand how it's used
2. As an Anki user, I want the word highlighted in the context sentence so I can quickly identify it
3. As a language learner, I want both the inflected form and the cleaned stem so I can recognize word variations

## Detailed Design

### Data Structure Changes

#### GlossWordContext (backend/src/helpers.rs)

Extend the struct to include:

```rust
pub struct GlossWordContext {
    pub original_word: String,  // Word as it appears in original text
    pub clean_word: String,     // Word after regex cleaning
    pub context_snippet: String, // Sentence context with <b> tags around word
}
```

### Algorithm Changes

#### extract_words_with_context() (backend/src/helpers.rs:361)

**Current approach:**
1. Apply all regex transformations to entire text
2. Split by spaces
3. Return Vec<GlossWordContext> with only clean_word

**New approach:**
1. Before regex operations, create a list of (word, byte_position) tuples using `.split_whitespace().enumerate()`
2. For each word position:
   - Store original word form from untransformed text
   - Apply regex operations to get cleaned word
   - Extract context snippet (50 chars before, 50 chars after) from original text
   - Find sentence boundaries within snippet:
     - Sentence start: `. `, `? `, `! `, or beginning of text
     - Sentence end: `.`, `?`, `!`, or end of text
   - Trim snippet to sentence boundaries if present
   - Wrap the original word in snippet with `<b>` tags (first occurrence only)
3. Return Vec<GlossWordContext> with all three fields

**Sentence boundary detection:**
- Simple delimiters: period (`.`), question mark (`?`), exclamation mark (`!`)
- No special handling for abbreviations or Pāli-specific punctuation
- If sentence boundary found before word → trim to start there
- If sentence boundary found after word → trim to end there
- If neither found → use full 50-char window on each side

**Edge cases:**
- Word near start of text (< 50 chars before): use whatever text is available
- Word near end of text (< 50 chars after): use whatever text is available
- Word appears multiple times in snippet: wrap first occurrence only

### CSV Export Changes

#### gloss_as_anki_csv() (GlossTab.qml:1303)

**Current format:**
```csv
"word_stem","definition"
```

**New format:**
```csv
"<p>word_stem</p><p>context sentence with <b>word</b></p>","definition"
```

**Implementation details:**
1. Front field contains two HTML paragraphs:
   - First `<p>`: cleaned word stem (for recognition)
   - Second `<p>`: context snippet with original word in bold
2. Back field: unchanged, shows dictionary summary
3. HTML characters in context: no escaping needed (as clarified)
4. Use existing `escape_csv_field()` function for proper CSV formatting

### Integration Points

**Rust bridge (SuttaBridge):**
- No changes needed to `extract_words()` function
- `extract_words_with_context()` will be used internally by glossing logic
- QML will receive context_snippet in word data structures

**QML (GlossTab.qml):**
- Update `gloss_as_anki_csv()` to format Front field with two paragraphs
- Update `paragraph_gloss_as_anki_csv()` similarly
- Update `gloss_export_data()` to include context_snippet in vocabulary data if needed

## Technical Considerations

### Performance
- Not a concern per user feedback
- Processing typical sutta paragraphs (50-200 words) should be instantaneous
- Regex operations remain the same, just tracking additional metadata

### Character Position Tracking
- Use byte positions for string slicing (Rust requirement)
- Handle multi-byte UTF-8 characters (Pāli diacritics: ā, ī, ū, ṃ, ṁ, ṅ, ñ, ṭ, ḍ, ṇ, ḷ)
- Ensure word position tracking accounts for text transformations

### Testing Strategy

**Unit tests (backend/tests/):**
1. Test `extract_words_with_context()` with:
   - Simple sentences with punctuation
   - Words at text boundaries
   - Multiple occurrences of same word
   - Pāli diacritical marks
   - Sandhi transformations (e.g., dhārayāmī'ti → dhārayāmi ti)
2. Test sentence boundary detection
3. Test HTML bold tag insertion

**Integration tests:**
1. Verify Anki CSV export format from QML
2. Test with actual Pāli sutta passages
3. Verify CSV escaping with commas, quotes, newlines in context

**Example test cases:**
```rust
// Test case 1: Mid-sentence word
let text = "Katamañca bhikkhave samādhindriyaṁ?";
// Expected: context_snippet contains "Katamañca <b>bhikkhave</b> samādhindriyaṁ?"

// Test case 2: Word at sentence boundary
let text = "Word1 word2. Word3 word4.";
// Expected for "Word3": "<b>Word3</b> word4."

// Test case 3: First word in text
let text = "Bhikkhave, listen carefully.";
// Expected: "<b>Bhikkhave</b>, listen carefully."
```

## Success Metrics

1. All existing word extraction tests continue to pass
2. Anki CSV exports include context sentences for 100% of vocabulary words
3. Context snippets correctly trim to sentence boundaries
4. Words are properly highlighted with `<b>` tags
5. CSV format is valid and imports correctly into Anki

## Open Questions

None - all clarifications received from user.

## Implementation Plan

### Phase 1: Backend Changes
1. Extend `GlossWordContext` struct with new fields
2. Rewrite `extract_words_with_context()` algorithm
3. Add unit tests for new functionality
4. Ensure `extract_words()` still works (uses new implementation internally)

### Phase 2: QML Export Updates
1. Update `gloss_as_anki_csv()` to use two-paragraph Front field format
2. Update `paragraph_gloss_as_anki_csv()` similarly
3. Test CSV generation with sample data

### Phase 3: Integration Testing
1. Test with real Pāli sutta passages
2. Import generated CSV into Anki to verify format
3. Verify all edge cases (boundaries, special characters, etc.)

## Out of Scope

- Smarter sentence boundary detection (handling abbreviations)
- Configurable context window size (currently fixed at 50 chars)
- Option to include/exclude context in other export formats
- Highlighting multiple word forms in same sentence
