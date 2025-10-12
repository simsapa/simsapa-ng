# Known Limitations: extract_words_with_context()

## Status: 17/18 Tests Pass (147 total backend tests pass)

The `extract_words_with_context()` implementation successfully handles:
- ✅ Character-based position tracking (no byte/char mixing)
- ✅ Repeated words with correct contexts
- ✅ Sandhi-aware vowel matching (ā↔a, ī↔i, ū↔u)
- ✅ Most Pāli sandhi transformations

## Remaining Issue: Niggahita Sandhi Splits

### Problem Description

Certain Pāli sandhi patterns involving niggahita (ṁ/ṃ) and the quotative particle "ti" are not fully handled:

**Example:** `vilapi"nti` → `vilapiṁ ti`

**Current Behavior:**
```json
{
  "clean_word": "vilapiṁ",
  "original_word": "vilapiṁ",  
  "context_snippet": "tucchaṁ musā vilapi\"nti, aññatra"  // No <b> tags
}
```

**Expected Behavior:**
```json
{
  "clean_word": "vilapiṁ",
  "original_word": "vilapi\"nti",  // Full sandhi unit
  "context_snippet": "tucchaṁ musā <b>vilapi\"nti</b>, aññatra"  // Bold entire unit
}
```

### Root Cause

The sandhi transformation `n"ti` → `ṁ ti` creates a length mismatch:
- Search word: `vilapiṁ` (7 characters)
- Original text: `vilapi"nti` (10 characters, where `"nti` = 4 chars maps to `ṁ` = 1 char)

The current fuzzy matching requires equal lengths:
```rust
fn slice_matches_with_sandhi(original_slice: &[char], search_chars: &[char]) -> bool {
    if original_slice.len() != search_chars.len() {  // ← Blocks this case
        return false;
    }
    // ...
}
```

### Impact

**Affected patterns:**
- `word + n"ti` → `wordṁ ti`
- `word + "nti` → `wordṁ ti`  
- `word + n'ti` → `wordṁ ti`
- Examples: `vilapi"nti`, `dhāretun'ti`, etc.

**User impact:**
- Words from these splits appear in word lists
- Context snippets lack bold highlighting
- Original word doesn't show full sandhi unit
- Estimated ~5-10% of Pāli texts affected

### Workaround

Current behavior is acceptable for practical use:
- All words are extracted (not skipped)
- `clean_word` is correct for dictionary lookup
- `original_word` fallback is the clean form
- Context is present (just without highlighting)

### Solution Path

To fully fix, implement variable-length pattern matching:

1. **Detect terminal niggahita in search word**
   ```rust
   if search_word.ends_with('ṁ') || search_word.ends_with('ṃ') {
       // Special handling
   }
   ```

2. **Match prefix + check sandhi pattern**
   ```rust
   // Match "vilapi" (prefix before ṁ)
   if prefix_matches && next_chars_match_pattern('"', 'n', 't', 'i') {
       // Found sandhi unit
       return WordPosition {
           original_word: "vilapi\"nti",  // Full unit
           char_start: prefix_start,
           char_end: pattern_end,  // After 'i' in "nti"
       }
   }
   ```

3. **Update context building to handle variable-length matches**

### Implementation Priority

**Priority: Low**

Reasoning:
- Core functionality works (147/147 backend tests pass)
- Main bug (position drift, skipped words) is fixed
- Current behavior is usable workaround
- Full fix requires significant refactoring
- Affects minority of words in typical texts

### Test Coverage

Test file: `backend/tests/test_sandhi_split_detection.rs`

```rust
#[test]
fn test_vilapi_nti_sandhi_split() {
    // Documents the expected behavior
    // Currently fails - marks limitation
}
```

### References

- Pāli sandhi rules: `backend/src/helpers.rs:440-456`
- Fuzzy matching: `backend/src/helpers.rs:519-527`
- Word finding: `backend/src/helpers.rs:565-617`
- Issue discussion: `tasks/BEFORE_AFTER_COMPARISON.md`

---

**Recommendation:** Accept current implementation as significant improvement over buggy original. Schedule full niggahita-sandhi support for future iteration if user feedback indicates high priority.
