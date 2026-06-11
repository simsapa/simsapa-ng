# PRD: Pāli Stemmer ASCII Support for Fulltext Search

## 1. Introduction / Overview

Fulltext search (Tantivy, `SearchMode::FulltextMatch`) fails to return results when users type ASCII transliterations of Pāli words. For example, searching `anabhijanam` returns no results even though the corpus contains `anabhijānaṁ`, and `vinnanam` fails to match `viññāṇaṁ`.

The root cause is a mismatch in the tokenizer pipeline: the Pāli Snowball stemmer operates on diacritical characters (e.g., suffix `ānaṁ`), but ASCII queries contain plain equivalents (e.g., `anam`). The stemmer does not recognize these ASCII suffixes, so it produces different (or no) stem output compared to what was indexed from the diacritical source text.

A workaround was applied (changing `Occur::Must` to `Occur::Should` for the content field, plus snippet fallback to `content_exact`), but this is a band-aid that doesn't restore declension matching for ASCII queries. This PRD describes the proper fix.

## 2. Goals

1. ASCII queries produce the **same stemmed tokens** as their diacritical equivalents (e.g., `vinnanam` and `viññāṇaṁ` both stem to `vinnana`).
2. ASCII queries match **all declensions** of a Pāli word, not just the exact ASCII-folded form.
3. Snippet highlighting works for ASCII queries against Pāli text.
4. Revert the `Occur::Should` workaround and snippet fallback — the stemmed `content` field should be the primary match field again (`Occur::Must`).
5. Search ranking and exact-match boosting remain unchanged.

## 3. Problem Analysis

### 3.1 Current Tokenizer Pipeline

```
{lang}_stem:      SimpleTokenizer → RemoveLong → LowerCase → NiggahitaNormalize → PāliStemmer → AsciiFold
{lang}_normalize: SimpleTokenizer → RemoveLong → LowerCase → NiggahitaNormalize → AsciiFold
```

- `content` field uses `{lang}_stem` — stemmed, then ASCII-folded.
- `content_exact` field uses `{lang}_normalize` — ASCII-folded, no stemming.

### 3.2 The Mismatch

| Step | Indexing `viññāṇaṁ` | Querying `vinnanam` |
|---|---|---|
| LowerCase | `viññāṇaṁ` | `vinnanam` |
| NiggahitaNormalize | `viññāṇaṁ` | `vinnanam` (no-op) |
| **PāliStemmer** | `viññāṇa` (strips `ṁ` suffix) | `vinnanam` (no matching suffix rule) |
| AsciiFold | `vinnana` | `vinnanam` |

The stemmer strips `aṁ` from `viññāṇaṁ` → `viññāṇa`, but does **not** recognize `am` in `vinnanam`. Result: different indexed tokens, no match.

### 3.3 Example Queries and Expected Behavior

#### ASCII queries (no diacritics)

| ASCII Query | Diacritical Form | Current Result | Expected Result |
|---|---|---|---|
| `anabhijanam` | `anabhijānaṁ` | No results | Match `pli-tv-bu-pm/pli/ms`, `pli-tv-bu-vb-pj4/pli/ms` |
| `vinnanam` | `viññāṇaṁ` | No Pāli results (other languages match) | Match Pāli suttas containing `viññāṇaṁ` and declensions |
| `vinnana` | `viññāṇa` | Works (stemmer not involved, short enough) | Continue working |
| `sattanam` | `sattānaṁ` | No results via stemmed field | Match via stemming |
| `jaramaranam` | `jarāmaraṇaṁ` | No results via stemmed field | Match via stemming |
| `nandi dukkhassa mulam` | `nandī dukkhassa mūlaṁ` | Partial (depends on source) | Match all sources |

#### Diacritical queries (current behavior, should be preserved)

| Diacritical Query | Stemmed To | Result | Expected |
|---|---|---|---|
| `anabhijānaṁ` | `anabhijāna` → fold → `anabhijana` | Results from ms and cst | Continue working |
| `viññāṇaṁ` | `viññāṇa` → fold → `vinnana` | Results with declensions | Continue working |
| `bhikkhūnaṁ` | `bhikkhu` | Results with declensions | Continue working |
| `dhammo` | `dhamma` | Results with declensions | Continue working |

### 3.4 Declension Matching Gap

Even with the `Occur::Should` workaround, ASCII queries only match via `content_exact` (no stemming), so they find exact forms but **not** other declensions:

| Query | Matches via `content_exact` | Misses (no stemming) |
|---|---|---|
| `vinnanam` | `viññāṇaṁ` only | `viññāṇānaṁ`, `viññāṇena`, `viññāṇassa`, etc. |
| `anabhijanam` | `anabhijānaṁ` only | Other inflected forms |

The stemmer fix restores declension matching for ASCII queries.

### 3.5 Additional Issues from Transcript

- **Snippet highlighting missing:** When a match comes only through `content_exact`, the snippet generator (which uses the `content` field) produces no highlights. A fallback to `content_exact` snippets was added as a workaround.
- **Query race condition:** Typing `anabhijānaṁ` progressively, an older query's results sometimes override a newer query's results. (Separate issue, not addressed in this PRD.)
- **Iti sandhi normalization:** `mūlan'ti` and `mūla'nti` should be normalized to `mūlaṁ ti`. The commit `747a3de` 'refactor plain text normalization' should have resolved it.

## 4. Solution: Move AsciiFold Before Stemmer

### 4.1 Revised Pipeline

Since `content` and `content_exact` are independent fields with independent tokenizer pipelines, moving AsciiFold before the stemmer in `{lang}_stem` does **not** affect `content_exact` or its boost scoring.

```
{lang}_stem (NEW): SimpleTokenizer → RemoveLong → LowerCase → NiggahitaNormalize → AsciiFold → PāliStemmer
{lang}_normalize:  SimpleTokenizer → RemoveLong → LowerCase → NiggahitaNormalize → AsciiFold  (unchanged)
```

Now both indexing and querying pass through AsciiFold **before** the stemmer. The stemmer always receives ASCII-only input.

| Step | Indexing `viññāṇaṁ` | Querying `vinnanam` |
|---|---|---|
| LowerCase | `viññāṇaṁ` | `vinnanam` |
| NiggahitaNormalize | `viññāṇaṁ` | `vinnanam` |
| **AsciiFold** | `vinnanam` | `vinnanam` |
| **PāliStemmer** | `vinnana` (strips `am`) | `vinnana` (strips `am`) |

Both produce `vinnana`. Match.

### 4.2 Stemmer Modification

The Pāli stemmer (Snowball `.sbl` file at `pali-stemmer-in-snowball/algorithms/pali.sbl`) must be rewritten to use **ASCII-only suffix rules** instead of diacritical ones.

Changes required:

1. **Remove all `stringdef` declarations** for diacritical characters (`{aa}`, `{ii}`, `{uu}`, `{.m}`, `{~n}`, `{.t}`, `{.d}`, `{.n}`, `{.l}`, etc.) — the stemmer will never see these characters.

2. **Replace all diacritical suffix patterns** with their ASCII equivalents:
   - `{aa}` → `a` (long ā → a)
   - `{ii}` → `i` (long ī → i)
   - `{uu}` → `u` (long ū → u)
   - `{.m}` → `m` (ṁ → m)
   - `{~n}` → `n` (ñ → n)
   - `{.t}` → `t` (ṭ → t)
   - `{.d}` → `d` (ḍ → d)
   - `{.n}` → `n` (ṇ → n)
   - `{.l}` → `l` (ḷ → l)
   - `{oo}` → `o` (ō → o)

3. **Update the exception list** — all diacritical forms become ASCII (e.g., `nibb{aa}na{.m}` → `nibbanam`, replacement `nibb{aa}na` → `nibbana`).

4. **Update the vowel grouping** — `define v 'aeiou{aa}{ii}{uu}'` becomes `define v 'aeiou'` (since folded text only has ASCII vowels).

5. **Remove the `prelude` routine** — NiggahitaNormalize and root-marker stripping happen before the stemmer now, and ṃ→ṁ normalization is irrelevant after ASCII folding.

6. **Review suffix overlaps** — some suffix rules that were distinct in diacritical form may collide in ASCII. For example:
   - `{aa}na{.m}` (ānaṁ, gen pl) and `ana{.m}` (anaṁ) both fold to `anam` — need to check if this causes incorrect stemming.
   - `{ii}` and `i` both fold to `i` — noun suffix rules for ī-stem and i-stem merge.

   These overlaps need careful review. In Snowball's `among` blocks, **longest match wins**, so most cases should resolve correctly. Document any ambiguous cases.

### 4.3 Compile and Integrate

1. Edit `pali-stemmer-in-snowball/algorithms/pali.sbl`
2. Run `make compile-stemmer` in `pali-stemmer-in-snowball/` to generate Rust code
3. Copy the generated Rust file to the backend's snowball module
4. Update `backend/src/search/tokenizer.rs`: move `AsciiFoldingFilter` before `StemmerFilter` in `{lang}_stem`

### 4.4 Revert Workaround

In `backend/src/search/searcher.rs`:

1. **Revert query logic** (lines ~356-368): Change back to `Occur::Must` for `content_query`, `Occur::Should` for `boosted_exact`:
   ```rust
   let mut subqueries = vec![
       (Occur::Must, Box::new(content_query)),
       (Occur::Should, Box::new(boosted_exact)),
   ];
   ```

2. **Remove `generate_snippet_with_fallback`** and revert to using `generate_snippet` (now `generate_snippet_for_field`) with only the `content_field`. Remove the extra `content_exact_field` parameter from `sutta_doc_to_result` and `dict_doc_to_result`.

### 4.5 Re-index Required

After changing the tokenizer pipeline order, **all existing indexes must be rebuilt** (bootstrap). The indexed tokens will differ from the old pipeline.

We will run the index rebuild manually.

## 5. User Stories

- As a user, I want to search `vinnanam` and find all suttas containing `viññāṇaṁ` and its declensions, so that I don't need to type diacritics.
- As a user, I want to search `anabhijanam` and find the Vinaya passages containing `anabhijānaṁ`, so that ASCII-only input works.
- As a user, I want highlighted snippets for ASCII searches against Pāli text.
- As a user, I want diacritical queries to continue working as before.

## 6. Functional Requirements

1. The `{lang}_stem` tokenizer pipeline must apply `AsciiFoldingFilter` **before** `PāliStemmerFilter`.
2. The Pāli Snowball stemmer must be rewritten to operate on ASCII-only input, producing ASCII-only stems.
3. Both `viññāṇaṁ` (indexed) and `vinnanam` (queried) must produce the identical stemmed token `vinnana`.
4. The `content` field query must use `Occur::Must` (revert workaround).
5. Snippet generation must use only the `content` field (revert fallback).
6. All existing stemmer rules (noun, verb, participle, residual suffixes, exception list) must be preserved in ASCII form.
7. The `{lang}_normalize` pipeline must remain unchanged.

## 7. Non-Goals (Out of Scope)

- Query race condition fix (older results overriding newer ones).
- Supporting non-Pāli stemmers for ASCII queries (other language stemmers are unaffected).
- Modifying `content_exact` field behavior.

## 8. Technical Considerations

- **Snowball compilation:** The `.sbl` file compiles to Rust via `make compile-stemmer`. The generated file needs `use crate::snowball::` path adjustment (handled by the Makefile's `sed` command).
- **Suffix collision risk:** When diacritical suffixes fold to the same ASCII string, the longest-match-first rule in Snowball `among` blocks should handle most cases. Any remaining ambiguities should be documented and tested.
- **Index rebuild:** Changing the pipeline order means all tantivy indexes must be manually rebuilt. This is part of the normal bootstrap process.
- **Pipeline for non-Pāli languages:** Only the `pli` stemmer needs ASCII-ification. Other languages (English, etc.) already have ASCII-compatible Snowball stemmers. The `AsciiFold` move applies to all `{lang}_stem` pipelines, but non-Pāli stemmers already handle ASCII fine.

## 9. Test Plan

### 9.1 Stemmer Unit Tests (pali-stemmer-in-snowball project)

Verify ASCII input produces correct stems:

| Input (ASCII) | Expected Stem | Rule |
|---|---|---|
| `vinnanam` | `vinnana` | a-stem: strips `m` (was `aṁ`) |
| `anabhijanam` | `anabhijana` | a-stem: strips `m` |
| `bhikkhunam` | `bhikkhu` | u-stem: strips `nam` (was `ūnaṁ`) |
| `dhammo` | `dhamma` | a-stem: `o` → `a` |
| `dhammam` | `dhamma` | a-stem: strips `m` |
| `sattanam` | `satta` | a-stem: strips `nam` (was `ānaṁ`) |
| `nibbanam` | `nibbana` | exception list |
| `bhagavantam` | `bhagavant` | exception list |
| `arahato` | `arahant` | exception list |

### 9.2 Backend Integration Tests (backend/src/search/)

Verify end-to-end search behavior:

| Test | Query | Expected |
|---|---|---|
| ASCII matches Pāli | `sattanam` | Matches documents containing `sattānaṁ` |
| ASCII matches declensions | `vinnanam` | Matches `viññāṇaṁ`, `viññāṇānaṁ`, `viññāṇena`, etc. |
| Diacritical still works | `viññāṇaṁ` | Same results as before |
| Snippet highlights | `vinnanam` | Snippet contains `<span class='match'>` |
| Multi-word ASCII | `nandi dukkhassa mula` | Matches `nandī dukkhassa mūla` |
| Short words unchanged | `ca`, `na`, `hi` | Pass through, no stemming |

### 9.3 Regression Tests

- All existing stemmer tests in `backend/src/search/tokenizer.rs` must pass (with expected values updated for the new pipeline order).
- All existing searcher tests in `backend/src/search/searcher.rs` must pass.

## 10. Implementation Steps

1. **Rewrite `pali.sbl`** — convert all suffix rules and exception list to ASCII-only.
2. **Compile** — run `make compile-stemmer`, copy output to backend.
3. **Update tokenizer pipeline** — move `AsciiFoldingFilter` before `StemmerFilter` in `register_tokenizers()`.
4. **Revert workaround** — restore `Occur::Must` for content, remove snippet fallback, remove extra `content_exact_field` parameter.
5. **Update tests** — adjust expected values, add ASCII-specific test cases.
6. **Rebuild indexes** — run bootstrap to re-index all content.
7. **Manual verification** — test the queries from the table above in the GUI.

## 11. Open Questions

1. **Suffix collisions:** Which diacritical suffixes collapse to the same ASCII string? Do any cause incorrect stemming? Examine the cases for ambiguity and test.
2. **Exception list completeness:** Are there exception words where the ASCII form is ambiguous (different Pāli words folding to the same ASCII)? Examine the cases for ambiguity and test.
