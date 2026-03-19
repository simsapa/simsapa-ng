# PRD: Fulltext Index Search with Tantivy and Custom Pali Stemmer

## 1. Introduction/Overview

Simsapa currently supports `ContainsMatch` search (SQL LIKE queries) but lacks fulltext search with linguistic stemming. This feature adds fulltext index search using the Tantivy search engine with language-appropriate stemmers, including a custom Pali stemmer built with the Snowball stemming framework.

The Pali stemmer handles citation-form stemming for inflected Pali words (e.g., "bhikkhūnaṁ" → "bhikkhu", "dhammo" → "dhamma"), enabling users to find suttas regardless of the grammatical form used. Translation texts are stemmed with the appropriate language stemmer (English, German, French, etc.).

Both sutta and dictionary databases will be indexed, with per-language indexes to ensure optimal stemming quality.

## 2. Goals

1. Implement `SearchMode::FulltextMatch` in the Rust backend using Tantivy.
2. Integrate the custom Pali Snowball stemmer alongside standard language stemmers from the Snowball project.
3. Build per-language fulltext indexes for both suttas and dictionary entries.
4. Support filtering search results by `language`, `source_uid`, `nikaya`, and `sutta_ref`.
5. Provide CLI commands for pre-building indexes and a UI action for user-triggered re-indexing.
6. Design data types that are extensible for a future composable search pipeline (fulltext → contains → etc.).

## 3. User Stories

- **As a Pali student**, I want to search for "bhikkhu" and find all suttas containing any inflected form (bhikkhūnaṁ, bhikkhave, bhikkhuno, etc.) so I don't need to know the exact grammatical form.
- **As a translator**, I want to search English translations with English stemming so that searching "suffering" also matches "suffers" and "suffered."
- **As a researcher**, I want to filter fulltext results by nikaya (e.g., only DN) or source (e.g., only Bodhi's translations) to narrow results to a specific collection.
- **As a dictionary user**, I want to search dictionary definitions with fulltext to find entries that discuss a concept even when the headword doesn't match my query.
- **As a content publisher**, I want to pre-build fulltext indexes via the CLI so the distributed app bundle ships with ready-to-use indexes.
- **As an app user**, I want to be notified if a language index is missing and choose when to build it, rather than having it forced on startup.

## 4. Functional Requirements

### 4.1 Stemmer Integration

1. The project must include a `snowball-stemmers` module within the backend crate (not a separate dependency crate). This module contains:
   - The Snowball Rust runtime files (`snowball_env.rs`, `among.rs`) copied from the Snowball project's `rust/src/snowball/` directory.
   - Generated Rust stemmer files for all supported languages, compiled from `.sbl` algorithm files using the Snowball compiler (`snowball` binary from `pali-stemmer-in-snowball/assets/snowball/`).
   - A `stem(algorithm, word)` public API or per-algorithm `stem()` functions.
   - Reference `rust-stemmers-simsapa` (`pali-stemmer-in-snowball/assets/rust-stemmers-simsapa/`) for the `Algorithm` enum pattern, `Stemmer` struct, and `Cow<str>` return type.

2. A Makefile target (e.g., `make compile-stemmers`) must compile all `.sbl` files from the Snowball repo (`pali-stemmer-in-snowball/assets/snowball/algorithms/`) plus the custom Pali stemmer (`pali-stemmer-in-snowball/algorithms/pali.sbl`) to Rust. The workflow mirrors the existing `compile-stemmer` target in `pali-stemmer-in-snowball/Makefile`:
   ```
   snowball algorithms/{lang}.sbl -rust -o output_path/{lang}_stemmer
   sed -i 's/use snowball::/use crate::snowball::/g' output_path/{lang}_stemmer.rs
   ```
   This is a development-time step only. The generated `.rs` files are committed to the repo. The Snowball compiler is not a build or runtime dependency.

3. The Pali stemmer (`algorithms/pali.sbl`) handles:
   - Exception list for irregular forms (nibbānaṁ, hoti conjugations, consonantal stems like bhagavant, rājan, brahman, etc.)
   - Participle suffix stripping/replacement (gerundives, absolutives, infinitives, passives)
   - Noun suffix handling (a-stem, i-stem, u-stem, consonantal -ant stems)
   - Verb suffix handling (present, future, optative, imperative, aorist across a/e/o conjugations)
   - Niggahīta normalization (ṃ U+1E43 → ṁ U+1E41)
   - Root marker stripping (√)
4. All Snowball `.sbl` algorithms from `pali-stemmer-in-snowball/assets/snowball/algorithms/` are compiled to Rust, plus the custom Pali stemmer. The following algorithms are compiled (excluding variant/legacy stemmers `dutch_porter.sbl`, `lovins.sbl`, `porter.sbl` which are older alternatives to the primary stemmers):

   **Compiled algorithms (32):** arabic, armenian, basque, catalan, danish, dutch, english, esperanto, estonian, finnish, french, german, greek, hindi, hungarian, indonesian, irish, italian, lithuanian, nepali, norwegian, pali, polish, portuguese, romanian, russian, serbian, spanish, swedish, tamil, turkish, yiddish

   **Language code → stemmer mapping** (used when creating per-language indexes):

   | Language Code | Stemmer    | Notes |
   |---------------|------------|-------|
   | `pli`         | Pali       | Custom Snowball stemmer |
   | `san`         | Pali       | Use Pali stemmer for Sanskrit |
   | `ar`          | Arabic     | |
   | `hy`          | Armenian   | |
   | `eu`          | Basque     | |
   | `ca`          | Catalan    | |
   | `da`          | Danish     | |
   | `nl`          | Dutch      | |
   | `en`          | English    | |
   | `eo`          | Esperanto  | |
   | `et`          | Estonian   | |
   | `fi`          | Finnish    | |
   | `fr`          | French     | |
   | `de`          | German     | |
   | `el`          | Greek      | |
   | `hi`          | Hindi      | |
   | `hu`          | Hungarian  | |
   | `id`          | Indonesian | |
   | `ga`          | Irish      | |
   | `it`          | Italian    | |
   | `lt`          | Lithuanian | |
   | `ne`          | Nepali     | |
   | `no`          | Norwegian  | |
   | `pl`          | Polish     | |
   | `pt`          | Portuguese | |
   | `ro`          | Romanian   | |
   | `ru`          | Russian    | |
   | `sr`          | Serbian    | |
   | `es`          | Spanish    | |
   | `sv`          | Swedish    | |
   | `ta`          | Tamil      | |
   | `tr`          | Turkish    | |
   | `yi`          | Yiddish    | |
   | (other)       | English    | Fallback for unknown languages |

### 4.2 Tantivy Tokenizers

5. Register the following custom tokenizers with each Tantivy index:

   - **`{lang}_stem`** (e.g., `pli_stem`, `en_stem`): `SimpleTokenizer → RemoveLong(50) → LowerCaser → NiggahitaNormalizer → LanguageStemmerFilter → AsciiFoldingFilter`
     - `NiggahitaNormalizer` is applied for all languages (no-op for non-Pali text, normalizes ṃ→ṁ and strips √)
     - `LanguageStemmerFilter` uses the appropriate stemmer for the index's language
   - **`simple_fold`**: `SimpleTokenizer → LowerCaser → AsciiFoldingFilter` (for uid, title, sutta_ref fields)
   - **`{lang}_normalize`** (e.g., `pli_normalize`): `SimpleTokenizer → RemoveLong(50) → LowerCaser → NiggahitaNormalizer → AsciiFoldingFilter` (for content_exact field, no stemming)

### 4.3 Index Schemas

6. **Sutta index schema** (per-language):

   | Field           | Tokenizer       | Stored | Purpose |
   |-----------------|-----------------|--------|---------|
   | `uid`           | `simple_fold`   | Yes    | Sutta identifier |
   | `title`         | `simple_fold`   | Yes    | Display title |
   | `language`      | `raw`           | Yes    | Filter field |
   | `source_uid`    | `raw`           | Yes    | Filter field (ms, bodhi, etc.) |
   | `sutta_ref`     | `simple_fold`   | Yes    | Filter field (DN 1, MN 10, etc.) |
   | `nikaya`        | `raw`           | Yes    | Filter field (dn, mn, sn, an, kn) |
   | `content`       | `{lang}_stem`   | Yes    | Stemmed fulltext search |
   | `content_exact` | `{lang}_normalize` | Yes | Exact-form boost field |

7. **Dictionary index schema** (per-language, uses language-appropriate stemmer):

   | Field           | Tokenizer          | Stored | Purpose |
   |-----------------|--------------------|--------|---------|
   | `uid`           | `simple_fold`      | Yes    | Entry identifier |
   | `word`          | `simple_fold`      | Yes    | Headword for display and matching |
   | `synonyms`      | `simple_fold`      | Yes    | Synonym matching |
   | `language`      | `raw`              | Yes    | Filter field |
   | `source_uid`    | `raw`              | Yes    | Filter field (dpd, ncped, etc.) |
   | `content`       | `{lang}_stem`      | Yes    | Stemmed fulltext search |
   | `content_exact` | `{lang}_normalize` | Yes    | Exact-form boost field |

### 4.4 Index Storage and Management

8. Indexes are stored per-language in separate directories under the app assets folder, following the legacy Python project's structure (`ASSETS_DIR/index/suttas/{lang}/`, `ASSETS_DIR/index/dict_words/{lang}/`). In Simsapa-ng, the base path is `AppGlobalPaths.app_assets_dir` (i.e., `{simsapa_dir}/app-assets/`):

   ```
   {simsapa_dir}/app-assets/index/
   ├── suttas/
   │   ├── pli/          # Pali suttas index (pali stemmer)
   │   ├── en/           # English translation index (english stemmer)
   │   ├── de/           # German translation index (german stemmer)
   │   └── ...           # one directory per language found in the database
   └── dict_words/
       ├── en/           # English dict words index (english stemmer)
       ├── pli/          # Pali dict words index (pali stemmer)
       └── ...           # one directory per language found in the database
   ```

   Each per-language directory contains a complete Tantivy index. The schema for each index uses the language-appropriate stemmer on the `content` field (e.g., English stemmer for `en/`, Pali stemmer for `pli/`).

9. `AppGlobalPaths` must be extended with index directory paths:
   ```rust
   pub index_dir: PathBuf,          // {app_assets_dir}/index/
   pub suttas_index_dir: PathBuf,   // {index_dir}/suttas/
   pub dict_words_index_dir: PathBuf, // {index_dir}/dict_words/
   ```

10. The system must query the database for distinct languages (`SELECT DISTINCT language FROM suttas`, `SELECT DISTINCT language FROM dict_words`) to determine which per-language indexes to create, matching the legacy `get_sutta_languages()` / `get_dict_word_languages()` pattern.

11. **Pre-built index distribution via GitHub releases:**

    The fulltext indexes are pre-built during the bootstrap process (`cli/src/bootstrap/mod.rs`) and distributed as download archives alongside the database files. This follows the legacy pattern from `simsapa-assets` releases (confirmed by examining `suttas_lang_hu.tar.bz2` which contains both the `.sqlite3` file and the `index/suttas/hu/` directory with Tantivy index files).

    **Default bundle (base languages: en, pli, san):**
    - `appdata.tar.bz2` — the sutta database (separate due to file size)
    - `index.tar.bz2` — pre-built indexes for the base languages, containing:
      ```
      index/
      ├── suttas/
      │   ├── pli/    # Tantivy index files (.idx, .pos, .store, .fast, .term, .fieldnorm, .tantivy-meta.lock)
      │   └── en/
      └── dict_words/
          ├── en/
          └── pli/
      ```
    - `dictionaries.tar.bz2`, `dpd.tar.bz2` — dictionary databases

    **Per-language downloads (e.g., `suttas_lang_hu.tar.bz2`):**
    - Each language archive includes both the sutta data AND the pre-built fulltext index for that language (matching the confirmed legacy archive structure):
      ```
      suttas_lang_hu.tar.bz2 contents:
      ├── suttas_lang_hu.sqlite3   # sutta data to import
      └── index/
          └── suttas/
              └── hu/              # pre-built Tantivy index
      ```
    - After download and extraction, `move_folder_contents()` merges the `index/` directory into `app-assets/index/`, placing the language index alongside existing ones.

    **Integration with the bootstrap process (`cli/src/bootstrap/mod.rs`):**

    The current bootstrap stages are:
    1. Clean and create folders
    2. Create `appdata.sqlite3` (import suttas from SuttaCentral, Dhammatalks, etc.)
    3. Create `appdata.tar.bz2` → release dir
    4. DPD bootstrap → create `dictionaries.tar.bz2`, `dpd.tar.bz2` → release dir
    5. Import per-language suttas → for each language, create `suttas_lang_{code}.tar.bz2` → release dir
    6. Write release info

    The index building must be added as follows:
    - **After step 3** (appdata archive created): Build fulltext indexes for base languages (en, pli, san) and dict_words. Then create `index.tar.bz2` from the `index/` directory and move to release dir. This mirrors the legacy `bootstrap.py` which calls `reindex()` after appdata creation, then `tar cjf index.tar.bz2 index/`.
    - **In step 5** (per-language loop): After importing each language's suttas into its `.sqlite3` file, build the fulltext index for that language into `index/suttas/{lang}/`. Then modify `create_database_archive()` (or create a new function) to include both the `.sqlite3` file and the `index/suttas/{lang}/` directory in the tarball. This mirrors the legacy `import_index_move_lang()` which runs `tar cjf "suttas_lang_{lang}.tar.bz2" "suttas_lang_{lang}.sqlite3" index/suttas/{lang}/`.

    **Important:** The legacy bootstrap runs indexing in a subprocess to ensure Tantivy temp files are cleaned up before tar compresses (see legacy comment about `tar: .tmp...: File removed before we read it`). The Rust implementation should ensure the `IndexWriter` is dropped/committed before creating the tarball.

    **Integration with existing download flow (`asset_manager.rs`):**
    - The `download_urls_and_extract()` function already handles downloading, extracting to a temp folder, and calling `move_folder_contents()` to merge into `app-assets/`. The index directories inside the archives are automatically placed at the correct path (`app-assets/index/suttas/{lang}/`) by this existing mechanism.
    - The `DownloadAppdataWindow.qml` `run_download()` function constructs URLs from `github_repo` and `version`. It must add `index.tar.bz2` to the initial setup URL list (alongside `appdata.tar.bz2`, `dictionaries.tar.bz2`, `dpd.tar.bz2`).
    - No changes needed for per-language downloads — the existing `suttas_lang_{code}.tar.bz2` URL pattern works; the archives just need to include the index directory.

12. Indexes are **not** built automatically on startup. If pre-built indexes were downloaded, they are ready to use. The app must check for missing/empty indexes and notify the user, allowing them to choose when to build locally if indexes are missing.

13. The CLI module (`cli/`) must provide commands to:
    - Build all indexes: `simsapa index build`
    - Build indexes for a specific area: `simsapa index build --area suttas`
    - Build indexes for a specific language: `simsapa index build --lang pli`
    - Rebuild (clear + build): `simsapa index rebuild`
    - These commands are used both by users and by the bootstrap pipeline to pre-build indexes for distribution.

14. The Qt UI must provide a menu action or settings panel to trigger re-indexing with a progress indicator.

### 4.5 Search Query Handling

15. `SearchQueryTask` in `backend/src/query_task.rs` must handle `SearchMode::FulltextMatch` by:
    - Determining the target language index(es) based on `SearchParams.lang`
    - If no language filter: search across all available language indexes and merge results by score
    - If language filter set: search only the matching language index

16. Search uses dual-field querying (matching the pali-search prototype pattern):
    - Stemmed content field: `Occur::Must` — ensures recall via citation-form matching
    - Exact normalized content field: `Occur::Should` with 2.0x boost — ranks exact-form matches higher

17. Support tantivy `TermQuery` filters for:
    - `language` (raw exact match)
    - `source_uid` (raw exact match)
    - `nikaya` (raw exact match)
    - `sutta_ref` (simple_fold match)

18. Return results as `SearchResult` structs with:
    - `uid`, `title`, `source_uid`, `snippet` (HTML with highlighted matches), `score`, `language`
    - For suttas: `sutta_ref`, `nikaya`
    - For dict words: `word`

19. Snippet generation must use Tantivy's `SnippetGenerator` with max 200 characters. Post-process the output to replace `<b>`/`</b>` tags with `<span class='match'>`/`</span>` to match the existing highlight convention.

### 4.6 Indexing Content

20. When indexing suttas:
    - Use `content_plain` for the indexed content (this field is already converted from `content_html`)
    - Prepend `sutta_ref`, `title`, and `title_pali` to the content field so a single-field query matches title text
    - Skip entries with no content

21. When indexing dictionary entries:
    - Use `definition_plain` for the indexed content (this field is already converted from `definition_html`)
    - Prepend `word` and `synonyms` to content so single-field queries match headwords
    - Skip entries with no definition content

### 4.7 Extensibility for Composable Search Pipeline

22. Define the following types to support future pipeline composition (implement only `FulltextMatch` now):

    ```rust
    /// A single step in a composable search pipeline.
    #[derive(Debug, Clone, Deserialize)]
    pub struct SearchStep {
        pub mode: SearchMode,
        pub query_text: String,
        /// Filters applied at this step
        pub filters: SearchFilters,
    }

    #[derive(Debug, Clone, Default, Deserialize)]
    pub struct SearchFilters {
        pub lang: Option<String>,
        pub lang_include: bool,
        pub source_uid: Option<String>,
        pub source_include: bool,
        pub nikaya: Option<String>,
        pub sutta_ref: Option<String>,
    }

    /// A search pipeline is a list of steps, each narrowing the previous results.
    /// For now, only single-step pipelines are supported.
    #[derive(Debug, Clone, Deserialize)]
    pub struct SearchPipeline {
        pub steps: Vec<SearchStep>,
        pub area: SearchArea,
        pub page_len: Option<usize>,
    }
    ```

## 5. Non-Goals (Out of Scope)

- **Composable pipeline execution**: The pipeline types are defined but only single-step `FulltextMatch` is implemented. Multi-step pipeline execution (e.g., fulltext → contains narrowing) is a future feature.
- **Fuzzy search**: Tantivy fuzzy matching is not part of this PRD.
- **Regex search via Tantivy**: RegEx search continues to use the existing SQL-based implementation.
- **Auto-indexing on startup**: Indexes are pre-built via CLI or user-triggered. No automatic background indexing on app startup.
- **Real-time index updates**: When database content changes, the user must manually trigger a re-index.
- **DPD-specific indexing logic**: Special handling for DPD headwords/roots (like `pali_word_index_plaintext`) is deferred; use the standard `definition_plain`/`definition_html` fields.

## 6. Design Considerations

### Crate Structure

- Stemmers live inside the backend crate as a module, not as a separate crate. The module includes:
  - The Snowball Rust runtime (`among.rs`, `snowball_env.rs`) — copied from `pali-stemmer-in-snowball/assets/snowball/rust/src/snowball/`
  - Generated algorithm files (one `.rs` per language) — compiled from `.sbl` files using the Snowball compiler binary
  - A public API module (`mod.rs`) exposing an `Algorithm` enum and `Stemmer` struct, following the pattern in `rust-stemmers-simsapa`
- The Tantivy tokenizer and indexer code lives in the `backend` crate (e.g., `backend/src/search/` module).

### Module Layout (suggested)

```
backend/src/snowball/
  mod.rs              — pub use, Algorithm enum, Stemmer struct (pattern from rust-stemmers-simsapa)
  among.rs            — Among struct (copied from Snowball runtime)
  snowball_env.rs     — SnowballEnv (copied from Snowball runtime)
  algorithms/
    mod.rs            — pub mod for each language
    pali.rs           — generated from pali.sbl
    english.rs        — generated from english.sbl
    german.rs         — generated from german.sbl
    ...               — one file per supported language

backend/src/search/
  mod.rs           — public API, re-exports
  tokenizer.rs     — NiggahitaNormalizer, StemmerFilter, register_tokenizers()
  schema.rs        — build_sutta_schema(), build_dict_schema()
  indexer.rs       — build_sutta_index(), build_dict_index(), open_or_create_index()
  searcher.rs      — FulltextSearcher, search(), debug_query()
  types.rs         — SearchStep, SearchFilters, SearchPipeline
```

### UI Integration

- The `SearchMode::FulltextMatch` option is already defined in the `SearchMode` enum.
- The QML search bar already supports mode selection; no new QML components are needed for basic integration.
- A new "Rebuild Search Index" action should be added to the application menu or settings panel.

## 7. Technical Considerations

- **Tantivy version**: Use the same version as `pali-search` prototype (check `pali-stemmer-in-snowball/pali-search/Cargo.toml`).
- **Memory**: Index writer should use ~50MB heap (`index.writer(50_000_000)`).
- **Thread safety**: `IndexReader` is thread-safe and can be shared. Create one reader per language index at app startup and reuse.
- **Android**: Use `try_exists()` for all path checks per project convention. MmapDirectory should work on Android but test with the offscreen platform.
- **Index size**: Pali canon (~17,000 suttas) + translations produce indexes of ~50-100MB per language. Dictionary indexes are smaller.
- **Snowball compiler workflow**: The Snowball compiler binary is built from `pali-stemmer-in-snowball/assets/snowball/` (run `make` in that directory). A Makefile target (`make compile-stemmers`) iterates over all `.sbl` files in the Snowball `algorithms/` directory plus `pali-stemmer-in-snowball/algorithms/pali.sbl`, compiles each to Rust with `snowball {file}.sbl -rust -o {output}`, and applies the `sed` fixup to change `use snowball::` to `use crate::snowball::`. Generated `.rs` files are committed to the repo so the Snowball compiler is not needed at build time.

## 8. Success Metrics

- Searching "bhikkhu" in Pali returns results containing bhikkhūnaṁ, bhikkhave, bhikkhuno, bhikkhū, etc.
- Searching "suffering" in English returns results containing "suffers", "suffered", "suffering".
- Exact-form queries rank higher than stemmed matches (e.g., searching "dhammaṁ" ranks documents with literal "dhammaṁ" above those with only "dhammā").
- Fulltext search completes in < 100ms for typical single-word queries.
- Index build time for all Pali suttas < 60 seconds.
- Filter by nikaya/source_uid correctly narrows results.
- CLI `simsapa index build` successfully builds all language indexes.

## 9. Resolved Questions

1. **Title search**: Do not search the `title` field — title matches produce misleading results. Search only the `content` and `content_exact` fields (which already have title text prepended per requirement 20).

2. **Snippet highlighting tags**: Tantivy's `SnippetGenerator` produces `<b>` tags by default. Post-process snippet HTML to replace `<b>` / `</b>` with `<span class='match'>` / `</span>` to match the existing highlight convention used throughout the app (see `query_task.rs` and `FulltextResults.qml`).

3. **Cross-language search**: When no language filter is set, search all available language indexes and merge results by score (interleaved). Do not group by language.

4. **Index versioning**: Include a version marker file (e.g., `index/VERSION`) containing the index schema version and stemmer algorithm version. On startup, compare the marker against the expected version; if stale, notify the user that re-indexing is recommended. The version marker is also included in `index.tar.bz2` and per-language archives.
