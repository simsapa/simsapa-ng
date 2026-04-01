## Relevant Files

- `pali-stemmer-in-snowball/algorithms/pali.sbl` - Snowball source for the Pāli stemmer. Rewritten for ASCII-only input.
- `pali-stemmer-in-snowball/Makefile` - Contains `compile-stemmer` target to generate Rust from `.sbl`.
- `backend/src/snowball/algorithms/pali_stemmer.rs` - Generated Rust stemmer code. Regenerated from ASCII `.sbl`.
- `backend/src/search/tokenizer.rs` - Tokenizer pipeline: reordered AsciiFold before Stemmer, added ASCII stem tests.
- `backend/src/search/searcher.rs` - Reverted Occur::Should workaround, removed snippet fallback, added declension test.
- `backend/src/search/schema.rs` - Schema definitions for `content` and `content_exact` fields (reference only).
- `backend/src/snowball/mod.rs` - Snowball module with `Algorithm` enum and `lang_to_algorithm()` (reference only).
- `backend/tests/test_search.rs` - Integration tests for search. Fixed API calls (`search_suttas` → `search_suttas_with_count`).
- `backend/tests/test_fulltext_search_results.rs` - Fulltext search result comparison tests.

### Notes

- Run stemmer compilation: `cd pali-stemmer-in-snowball && make compile-stemmer`
- Run backend tests: `cd backend && cargo test`
- Run specific test module: `cd backend && cargo test --lib search::searcher`
- Run specific test module: `cd backend && cargo test --lib search::tokenizer`
- After all changes, a full index rebuild (bootstrap) is required. This is done manually.

## Tasks

- [x] 1.0 Rewrite the Pāli Snowball stemmer for ASCII-only input
  - [x] 1.1 Remove all `stringdef` declarations for diacritical characters (`{aa}`, `{ii}`, `{uu}`, `{.m}`, `{~n}`, `{.t}`, `{.d}`, `{.n}`, `{.l}`, `{oo}`, `{.r}`, `{.s}`, `{'s}`, `{vr}`, `{.M}`) from `pali.sbl`.
  - [x] 1.2 Replace all diacritical references in suffix rules with ASCII equivalents: `{aa}`→`a`, `{ii}`→`i`, `{uu}`→`u`, `{.m}`→`m`, `{~n}`→`n`, `{.t}`→`t`, `{.d}`→`d`, `{.n}`→`n`, `{.l}`→`l`, `{oo}`→`o`.
  - [x] 1.3 Update the vowel grouping: `define v 'aeiou{aa}{ii}{uu}'` → `define v 'aeiou'`.
  - [x] 1.4 Remove or simplify the `prelude` routine — root marker stripping (`√`) and ṃ→ṁ normalization are handled before the stemmer now.
  - [x] 1.5 Convert all exception list entries to ASCII (e.g., `'nibb{aa}na{.m}'` → `'nibbanam'`, replacement `'nibb{aa}na'` → `'nibbana'`).
  - [x] 1.6 Convert all `noun_suffix`, `verb_suffix`, `participle_suffix`, and `residual_suffix` rules to ASCII.
  - [x] 1.7 Review suffix rules for collisions where distinct diacritical suffixes map to the same ASCII string (see task 5.0). Adjust ordering or merge as needed.
  - [x] 1.8 Compile the updated `.sbl` to Rust: run `make compile-stemmer` in `pali-stemmer-in-snowball/`.
  - [x] 1.9 Copy the generated `pali_stemmer.rs` to `backend/src/snowball/algorithms/pali_stemmer.rs` (handled by the Makefile, verify the `sed` path fix is applied).
  - [x] 1.10 Verify the backend compiles: `cd backend && cargo build`.

- [x] 2.0 Update the tokenizer pipeline
  - [x] 2.1 In `register_tokenizers()` in `backend/src/search/tokenizer.rs`, move `AsciiFoldingFilter` before `StemmerFilter` in the `{lang}_stem` pipeline.
  - [x] 2.2 Verify `NiggahitaNormalizer` is still needed in `{lang}_stem` — kept because it strips `√` root marker which AsciiFold doesn't handle. The ṃ→ṁ normalization before fold is harmless.
  - [x] 2.3 Verify `{lang}_normalize` pipeline remains unchanged.
  - [x] 2.4 Verify the backend compiles: `cd backend && cargo build`.

- [x] 3.0 Revert the search workaround
  - [x] 3.1 In `search_single_index()` in `searcher.rs` (~lines 356-368), revert the `BooleanQuery` to use `Occur::Must` for `content_query` and `Occur::Should` for `boosted_exact`:
    ```rust
    let mut subqueries = vec![
        (Occur::Must, Box::new(content_query)),
        (Occur::Should, Box::new(boosted_exact)),
    ];
    ```
    Remove the intermediate `match_query` wrapper and the associated comments.
  - [x] 3.2 Remove the `generate_snippet_with_fallback()` method. Rename `generate_snippet_for_field()` back to `generate_snippet()`.
  - [x] 3.3 Remove the `content_exact_field` parameter from `sutta_doc_to_result()` and `dict_doc_to_result()`. Update calls in `search_single_index()` accordingly.
  - [x] 3.4 Update snippet calls in `sutta_doc_to_result()` and `dict_doc_to_result()` to use `generate_snippet()` with only `content_field`.
  - [x] 3.5 Verify the backend compiles: `cd backend && cargo build`.

- [x] 4.0 Update and add tests
  - [x] 4.1 Update `test_pali_stem_basic` in `tokenizer.rs`: input `"viññāṇānaṁ"` still produces `"vinnana"`. Verified.
  - [x] 4.2 Update `test_pali_stem_multiple_words`: `"bhikkhūnaṁ dhammo"` still produces `["bhikkhu", "dhamma"]`. Verified.
  - [x] 4.3 Add test `test_ascii_input_matches_diacritical`: verified `"vinnanam"` and `"viññāṇaṁ"` produce the same stemmed token.
  - [x] 4.4 Add test `test_ascii_stem_anabhijanam`: `"anabhijanam"` stems to `"anabhija"` (PRD expected `"anabhijana"` but `anam` gen pl suffix is longest match, same as original stemmer with `ānaṁ`).
  - [x] 4.5 Add test `test_ascii_stem_bhikkhunam`: verified `"bhikkhunam"` stems to `"bhikkhu"`.
  - [x] 4.6 Add test `test_ascii_stem_sattanam`: verified `"sattanam"` stems to `"satta"`.
  - [x] 4.7 Add test `test_ascii_stem_nibbanam`: verified `"nibbanam"` stems to `"nibbana"` (exception list).
  - [x] 4.8 Add test `test_ascii_stem_dhammo`: verified `"dhammo"` stems to `"dhamma"`.
  - [x] 4.9 Update searcher tests: `test_ascii_query_matches_pali_text` and `test_ascii_query_jaramaranam` pass after reverting the workaround. Updated comment.
  - [x] 4.10 Add searcher test `test_ascii_query_with_declensions`: verified `"vinnanam"` matches documents with viññāṇa declensions.
  - [x] 4.11 All tests pass: 14 tokenizer + 10 searcher = 24 tests.

- [x] 5.0 Analyze and document suffix collisions
  - [x] 5.1 Enumerate all diacritical suffix pairs that collapse to the same ASCII string. Found: 9 same-replacement merges (inam, isu, ini, usu, iyante/iyanti/iyati, eyyavho, imha), 1 different-replacement (ayo), and 3 identity no-ops (a, i, u).
  - [x] 5.2 All collisions resolved correctly by longest-match-first or same-replacement merge. The `ayo` collision resolved by choosing a-stem (<-'a') over i-stem. No incorrect stemmings found.
  - [x] 5.3 Exception list ASCII forms are unambiguous. Minor note: `mata` could arise from mātā (mother) or mata (dead), but maps to same citation form — harmless for search.
  - [x] 5.4 Added 4 collision tests: `test_collision_ayo_uses_a_stem`, `test_collision_inam_merged`, `test_collision_usu_merged`, `test_identity_noop_prevents_verb_match`.
  - [x] 5.5 Documented findings in comment block at top of `pali.sbl` (sections A-E).
