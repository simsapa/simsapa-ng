## Relevant Files

- `pali-stemmer-in-snowball/algorithms/pali.sbl` - Snowball source for the Pāli stemmer. Must be rewritten for ASCII-only input.
- `pali-stemmer-in-snowball/Makefile` - Contains `compile-stemmer` target to generate Rust from `.sbl`.
- `backend/src/snowball/algorithms/pali_stemmer.rs` - Generated Rust stemmer code. Will be regenerated after `.sbl` changes.
- `backend/src/search/tokenizer.rs` - Tokenizer pipeline registration. Must reorder filters and update tests.
- `backend/src/search/searcher.rs` - Search query construction and snippet generation. Must revert workaround.
- `backend/src/search/schema.rs` - Schema definitions for `content` and `content_exact` fields (reference only).
- `backend/src/snowball/mod.rs` - Snowball module with `Algorithm` enum and `lang_to_algorithm()` (reference only).
- `backend/tests/test_search.rs` - Integration tests for search (may need updates).
- `backend/tests/test_fulltext_search_results.rs` - Fulltext search result comparison tests.

### Notes

- Run stemmer compilation: `cd pali-stemmer-in-snowball && make compile-stemmer`
- Run backend tests: `cd backend && cargo test`
- Run specific test module: `cd backend && cargo test --lib search::searcher`
- Run specific test module: `cd backend && cargo test --lib search::tokenizer`
- After all changes, a full index rebuild (bootstrap) is required. This is done manually.

## Tasks

- [ ] 1.0 Rewrite the Pāli Snowball stemmer for ASCII-only input
  - [ ] 1.1 Remove all `stringdef` declarations for diacritical characters (`{aa}`, `{ii}`, `{uu}`, `{.m}`, `{~n}`, `{.t}`, `{.d}`, `{.n}`, `{.l}`, `{oo}`, `{.r}`, `{.s}`, `{'s}`, `{vr}`, `{.M}`) from `pali.sbl`.
  - [ ] 1.2 Replace all diacritical references in suffix rules with ASCII equivalents: `{aa}`→`a`, `{ii}`→`i`, `{uu}`→`u`, `{.m}`→`m`, `{~n}`→`n`, `{.t}`→`t`, `{.d}`→`d`, `{.n}`→`n`, `{.l}`→`l`, `{oo}`→`o`.
  - [ ] 1.3 Update the vowel grouping: `define v 'aeiou{aa}{ii}{uu}'` → `define v 'aeiou'`.
  - [ ] 1.4 Remove or simplify the `prelude` routine — root marker stripping (`√`) and ṃ→ṁ normalization are handled before the stemmer now.
  - [ ] 1.5 Convert all exception list entries to ASCII (e.g., `'nibb{aa}na{.m}'` → `'nibbanam'`, replacement `'nibb{aa}na'` → `'nibbana'`).
  - [ ] 1.6 Convert all `noun_suffix`, `verb_suffix`, `participle_suffix`, and `residual_suffix` rules to ASCII.
  - [ ] 1.7 Review suffix rules for collisions where distinct diacritical suffixes map to the same ASCII string (see task 5.0). Adjust ordering or merge as needed.
  - [ ] 1.8 Compile the updated `.sbl` to Rust: run `make compile-stemmer` in `pali-stemmer-in-snowball/`.
  - [ ] 1.9 Copy the generated `pali_stemmer.rs` to `backend/src/snowball/algorithms/pali_stemmer.rs` (handled by the Makefile, verify the `sed` path fix is applied).
  - [ ] 1.10 Verify the backend compiles: `cd backend && cargo build`.

- [ ] 2.0 Update the tokenizer pipeline
  - [ ] 2.1 In `register_tokenizers()` in `backend/src/search/tokenizer.rs`, move `AsciiFoldingFilter` before `StemmerFilter` in the `{lang}_stem` pipeline.
  - [ ] 2.2 Verify `NiggahitaNormalizer` is still needed in `{lang}_stem` — after AsciiFold, ṁ/ṃ are already folded to `m`. If redundant, remove it from the stem pipeline (keep it in `{lang}_normalize` which runs before AsciiFold).
  - [ ] 2.3 Verify `{lang}_normalize` pipeline remains unchanged.
  - [ ] 2.4 Verify the backend compiles: `cd backend && cargo build`.

- [ ] 3.0 Revert the search workaround
  - [ ] 3.1 In `search_single_index()` in `searcher.rs` (~lines 356-368), revert the `BooleanQuery` to use `Occur::Must` for `content_query` and `Occur::Should` for `boosted_exact`:
    ```rust
    let mut subqueries = vec![
        (Occur::Must, Box::new(content_query)),
        (Occur::Should, Box::new(boosted_exact)),
    ];
    ```
    Remove the intermediate `match_query` wrapper and the associated comments.
  - [ ] 3.2 Remove the `generate_snippet_with_fallback()` method. Rename `generate_snippet_for_field()` back to `generate_snippet()`.
  - [ ] 3.3 Remove the `content_exact_field` parameter from `sutta_doc_to_result()` and `dict_doc_to_result()`. Update calls in `search_single_index()` accordingly.
  - [ ] 3.4 Update snippet calls in `sutta_doc_to_result()` and `dict_doc_to_result()` to use `generate_snippet()` with only `content_field`.
  - [ ] 3.5 Verify the backend compiles: `cd backend && cargo build`.

- [ ] 4.0 Update and add tests
  - [ ] 4.1 Update `test_pali_stem_basic` in `tokenizer.rs`: input `"viññāṇānaṁ"` should still produce `"vinnana"` (AsciiFold now runs before stemmer, so the stemmer receives `"vinnananam"` and stems to `"vinnana"`). Verify the expected value is correct.
  - [ ] 4.2 Update `test_pali_stem_multiple_words`: `"bhikkhūnaṁ dhammo"` should still produce `["bhikkhu", "dhamma"]`. Verify.
  - [ ] 4.3 Add test `test_ascii_input_matches_diacritical`: verify that `"vinnanam"` and `"viññāṇaṁ"` produce the same stemmed token through the `pali_stem_analyzer`.
  - [ ] 4.4 Add test `test_ascii_stem_anabhijanam`: verify `"anabhijanam"` stems to `"anabhijana"`.
  - [ ] 4.5 Add test `test_ascii_stem_bhikkhunam`: verify `"bhikkhunam"` stems to `"bhikkhu"`.
  - [ ] 4.6 Add test `test_ascii_stem_sattanam`: verify `"sattanam"` stems to `"satta"`.
  - [ ] 4.7 Add test `test_ascii_stem_nibbanam`: verify `"nibbanam"` stems to `"nibbana"` (exception list).
  - [ ] 4.8 Add test `test_ascii_stem_dhammo`: verify `"dhammo"` stems to `"dhamma"`.
  - [ ] 4.9 Update searcher tests: verify `test_ascii_query_matches_pali_text` and `test_ascii_query_jaramaranam` still pass after reverting the workaround.
  - [ ] 4.10 Add searcher test `test_ascii_query_with_declensions`: verify that `"vinnanam"` matches documents containing multiple declensions of `viññāṇa`.
  - [ ] 4.11 Run all tests: `cd backend && cargo test --lib search::tokenizer && cargo test --lib search::searcher`.

- [ ] 5.0 Analyze and document suffix collisions
  - [ ] 5.1 Enumerate all diacritical suffix pairs that collapse to the same ASCII string. Key cases to check:
    - `{aa}na{.m}` (ānaṁ) and `ana{.m}` (anaṁ) both → `anam`
    - `{ii}` and `i` both → `i`
    - `{uu}` and `u` both → `u`
    - `{aa}` and `a` both → `a`
    - `a{.m}` (aṁ) and `am` both → `am`
    - `{ii}na{.m}` (īnaṁ) and `ina{.m}` (inaṁ) both → `inam`
    - `{uu}na{.m}` (ūnaṁ) and `una{.m}` (unaṁ) both → `unam`
  - [ ] 5.2 For each collision, determine whether the Snowball `among` longest-match-first rule resolves it correctly. Document any cases where it doesn't.
  - [ ] 5.3 For exception list entries, verify ASCII forms are unambiguous (no two different Pāli words fold to the same ASCII exception).
  - [ ] 5.4 Add tests for any ambiguous collision cases found in 5.2-5.3.
  - [ ] 5.5 Document findings in a comment block at the top of `pali.sbl`.
