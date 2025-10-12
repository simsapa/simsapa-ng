# PRD: Fix extract_words_with_context() for Repeated Words

## Problem Statement

The `extract_words_with_context()` function in `backend/src/helpers.rs` has a critical bug when processing texts with repeated words, especially in Pāli text containing Unicode diacritics (ā, ī, ū, ṁ, ṅ, ñ, ṭ, ḍ, ṇ, ḷ).

### Observed Behavior

When processing the following Pārājika passage:
```
Yo pana bhikkhu anabhijānaṁ uttarimanussadhammaṁ attupanāyikaṁ alamariyañāṇadassanaṁ 
samudācareyya "iti jānāmi, iti passāmī"ti, tato aparena samayena samanuggāhīyamāno vā 
asamanuggāhīyamāno vā āpanno visuddhāpekkho evaṁ vadeyya "ajānamevaṁ āvuso avacaṁ 
jānāmi, apassaṁ passāmi, tucchaṁ musā vilapi"nti, aññatra adhimānā, ayampi pārājiko 
hoti asaṁvāso.
```

**Expected:** Extract all 40 words correctly with proper context snippets  
**Actual:** 
- Words 0-12 extract correctly: "Yo pana bhikkhu ... iti jānāmi iti passāmi ti"
- Words 13-39 have **empty** `original_word` and `context_snippet` fields
- Second occurrence of "iti" (idx 10) incorrectly references the **same position** as the first "iti" (idx 8)

### Impact

1. **GlossTab.qml**: When generating glosses, words with empty `original_word` are skipped
2. **Anki CSV Export**: Incomplete vocabulary lists - many words missing entirely
3. **User Experience**: Unreliable word lookup and vocabulary building

## Root Cause Analysis

### Location
`backend/src/helpers.rs:472-477`

### The Bug
```rust
let (word_start_char, word_end_char, original_word) = if let Some(rel_pos) = original_lower[current_search_pos..].find(&search_word) {
    let byte_pos = current_search_pos + rel_pos;
    let start = original_normalized[..byte_pos].chars().count();
    let end = start + search_word.chars().count();
    let orig = chars[start..end.min(text_len)].iter().collect();
    current_search_pos = byte_pos + search_word.len();  // ⚠️ BUG HERE
    (start, end, orig)
```

**Problem:** Line 477 mixes byte positions with character lengths:
- `byte_pos` is a **byte index** into the string
- `search_word.len()` returns the **byte length** (not character count)
- For Unicode text, bytes ≠ characters
- Example: "iti jānāmi" - the 'ā' in "jānāmi" is 2 bytes, causing drift

**Result:** After finding "jānāmi" with diacritics, `current_search_pos` advances too far (past the actual position in bytes), causing subsequent searches to fail or find wrong occurrences.

### Why Second "iti" Fails
1. First "iti" at byte position ~95, found correctly
2. "jānāmi" at byte position ~100 (contains 2-byte 'ā')
3. `current_search_pos` updated to ~100 + 7 bytes = ~107
4. But the string slice `original_lower[107..]` might now be **invalid** (slicing mid-UTF8 character) or positioned **incorrectly**
5. When searching for second "iti", it either panics, returns None, or finds the first "iti" again due to incorrect slicing

## Algorithm Design: Character-Based Sequential Matching

### Core Principle
**Never mix byte positions and character positions.** Use character indices throughout.

### New Algorithm

```
INPUTS:
  - original_text: &str          // Original text with proper case and diacritics
  - clean_words: Vec<String>     // List of cleaned words to find

OUTPUTS:
  - Vec<GlossWordContext>        // Word + context for each word

ALGORITHM:

1. PREPROCESSING
   - original_normalized = original_text with newlines replaced by spaces
   - chars = original_normalized.chars().collect::<Vec<char>>()
   - text_len = chars.len()
   - original_lower = original_normalized.to_lowercase()
   - original_lower_chars = original_lower.chars().collect::<Vec<char>>()
   
2. INITIALIZE
   - results = Vec::new()
   - current_search_char_pos = 0  // Character position, NOT byte position
   
3. FOR EACH clean_word IN clean_words:
   
   a. FIND WORD IN TEXT (character-by-character matching)
      - search_word = clean_word.to_lowercase()
      - search_word_chars = search_word.chars().collect::<Vec<char>>()
      - search_len = search_word_chars.len()
      
      - found = false
      - FOR char_pos FROM current_search_char_pos TO (text_len - search_len):
          - IF original_lower_chars[char_pos..char_pos+search_len] == search_word_chars:
              - word_start_char = char_pos
              - word_end_char = char_pos + search_len
              - found = true
              - BREAK
      
      - IF NOT found:
          - Push GlossWordContext with empty original_word and context
          - CONTINUE to next word
   
   b. EXTRACT ORIGINAL WORD
      - original_word = chars[word_start_char..word_end_char].iter().collect()
   
   c. DETERMINE CONTEXT BOUNDARIES
      - sentence_start = find_sentence_start(original_normalized, word_start_char)
      - sentence_end = find_sentence_end(original_normalized, word_end_char)
      - context_start = max(sentence_start, word_start_char - 50)
      - context_end = min(sentence_end, word_end_char + 50)
   
   d. EXTRACT CONTEXT WITH BOLD TAGS
      - context_slice = chars[context_start..context_end].iter().collect()
      - relative_word_start = word_start_char - context_start
      - relative_word_end = word_end_char - context_start
      
      - IF relative_word_start >= 0 AND relative_word_end <= context_slice.len():
          - before = &context_slice[..relative_word_start]
          - word = &context_slice[relative_word_start..relative_word_end]
          - after = &context_slice[relative_word_end..]
          - context_snippet = format!("{}<b>{}</b>{}", before, word, after)
      - ELSE:
          - context_snippet = context_slice
   
   e. UPDATE SEARCH POSITION
      - current_search_char_pos = word_end_char  // ✅ Character position only
   
   f. STORE RESULT
      - Push GlossWordContext { clean_word, original_word, context_snippet }

4. RETURN results
```

### Key Improvements

1. **Character-based indexing**: All positions are character indices, never bytes
2. **Sequential matching**: Guarantees we find words in order, never skipping or going backwards
3. **Explicit character array matching**: Compare `Vec<char>` slices directly
4. **No string slicing**: Avoid potential UTF-8 boundary errors
5. **Clear position updates**: Always advance `current_search_char_pos` to the end of the found word

### Edge Cases Handled

1. **Repeated words**: Sequential search ensures each occurrence is found in order
2. **Unicode diacritics**: Character-based matching handles multi-byte UTF-8 correctly
3. **Words not found**: Continue with empty fields (current behavior preserved)
4. **Boundary conditions**: Sentence/context boundaries calculated from character positions
5. **Case sensitivity**: Search is case-insensitive, but original case preserved

## Implementation Plan

### Phase 1: Core Algorithm Replacement
**File:** `backend/src/helpers.rs`

1. **Rewrite `extract_words_with_context()` function (lines 415-517)**
   - Replace byte-based search with character-based search
   - Implement character array matching loop
   - Update position tracking to use character indices only
   - Ensure all string slicing uses character positions via chars vector

2. **Update helper functions if needed**
   - Review `find_sentence_start()` (lines 366-391)
   - Review `find_sentence_end()` (lines 393-412)
   - Ensure they work correctly with character positions

### Phase 2: Testing

1. **Run existing test: `test_repeated_words_no_skipping`**
   ```bash
   cd backend && cargo test test_repeated_words_no_skipping -- --nocapture
   ```
   
   **Expected results:**
   - All 40 words have non-empty `original_word`
   - All 40 words have non-empty `context_snippet`
   - First "iti" (idx 8): context contains `<b>iti</b> jānāmi, iti passāmī`
   - Second "iti" (idx 10): context contains `iti jānāmi, <b>iti</b> passāmī`
   - Both occurrences of "jānāmi" found correctly
   - Both occurrences of "passāmi" found correctly
   - Both occurrences of "vā" found correctly

2. **Run all word extraction tests**
   ```bash
   cd backend && cargo test test_extract_words_with_context
   ```
   
   Ensure all existing tests still pass:
   - `test_mid_sentence_word`
   - `test_word_at_sentence_boundary`
   - `test_first_word_in_text`
   - `test_last_word_in_text`
   - `test_multiple_occurrences_only_first_bolded`
   - `test_pali_diacritics_preservation`
   - `test_sandhi_transformation_*` (all variants)
   - `test_semicolon_sentence_boundary`
   - `test_sentence_context_*` (all variants)

3. **Integration testing**
   - Test with GlossTab.qml in the running application
   - Generate gloss for the Pārājika passage
   - Verify all words appear in gloss output
   - Export to Anki CSV and verify no words are skipped

### Phase 3: Performance Validation

1. **Benchmark with long texts**
   - Test with full sutta texts (DN 1, MN 1, etc.)
   - Measure execution time vs. current implementation
   - Character-based matching may be slightly slower but correctness is priority

2. **Memory usage**
   - Verify no excessive allocations
   - Consider reusing character vectors if performance is an issue

### Phase 4: Documentation

1. **Update function documentation**
   - Add clear docstring explaining the character-based approach
   - Document why byte-based indexing was problematic
   - Add examples showing repeated word handling

2. **Update test documentation**
   - Ensure test comments explain what they're validating
   - Document the specific bug being prevented

## Success Criteria

- [ ] All 40 words in test passage have non-empty `original_word`
- [ ] All 40 words in test passage have non-empty `context_snippet`
- [ ] Second "iti" has different context than first "iti"
- [ ] Both "jānāmi" occurrences found with different contexts
- [ ] All existing tests pass
- [ ] GlossTab.qml shows all words in generated gloss
- [ ] Anki CSV export contains all words
- [ ] No performance regression > 2x on typical suttas

## Testing Command

```bash
cd backend && cargo test test_repeated_words_no_skipping -- --nocapture
```

## Files to Modify

1. `backend/src/helpers.rs` - Primary changes to `extract_words_with_context()`
2. `backend/tests/test_extract_words_with_context.rs` - Verify fix works

## Risks & Mitigations

**Risk:** Performance regression with character-based matching  
**Mitigation:** Profile and optimize hot paths; consider caching character vectors

**Risk:** Breaking existing functionality  
**Mitigation:** Comprehensive test suite already exists; run all tests

**Risk:** Unicode edge cases (combining characters, zero-width characters)  
**Mitigation:** Pāli text is well-defined; stick to standard Unicode normalization

## Alternative Approaches Considered

### 1. Fix byte position tracking
- Keep byte-based search but carefully track UTF-8 boundaries
- **Rejected:** Too error-prone, complex to maintain

### 2. Use regex with lookahead for repeated matches
- Use regex captures to find all occurrences
- **Rejected:** Regex doesn't guarantee sequential ordering we need

### 3. Convert to char indices after byte search
- Find in bytes, then convert positions to chars
- **Rejected:** Still mixing byte/char semantics, prone to same bugs

## References

- Test file: `backend/tests/test_extract_words_with_context.rs:208-345`
- Current implementation: `backend/src/helpers.rs:415-517`
- Unicode in Rust: https://doc.rust-lang.org/book/ch08-02-strings.html#bytes-and-scalar-values-and-grapheme-clusters
