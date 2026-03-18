# Tasks: Fulltext Index Search with Tantivy and Custom Pali Stemmer

## Relevant Files

- `backend/Cargo.toml` - Add tantivy dependency
- `backend/src/lib.rs` - Extend `AppGlobalPaths` with index directory paths
- `backend/src/types.rs` - Contains `SearchMode::FulltextMatch` (already defined), `SearchParams`, `SearchResult`; add `SearchStep`, `SearchFilters`, `SearchPipeline` types
- `backend/src/query_task.rs` - Wire `FulltextMatch` handling into `results_page()` (currently falls to unimplemented catch-all)
- `backend/src/snowball/mod.rs` - NEW: Snowball module entry point with `Algorithm` enum, `Stemmer` struct
- `backend/src/snowball/among.rs` - NEW: Copied from Snowball runtime
- `backend/src/snowball/snowball_env.rs` - NEW: Copied from Snowball runtime
- `backend/src/snowball/algorithms/mod.rs` - NEW: pub mod declarations for all generated stemmers
- `backend/src/snowball/algorithms/*.rs` - NEW: Generated Rust stemmer files (one per language)
- `backend/src/search/mod.rs` - NEW: Search module entry point, re-exports
- `backend/src/search/tokenizer.rs` - NEW: NiggahitaNormalizer, StemmerFilter, `register_tokenizers()`
- `backend/src/search/schema.rs` - NEW: `build_sutta_schema()`, `build_dict_schema()`
- `backend/src/search/indexer.rs` - NEW: `open_or_create_index()`, `build_sutta_index()`, `build_dict_index()`
- `backend/src/search/searcher.rs` - NEW: `FulltextSearcher`, dual-field querying, snippet generation
- `backend/src/search/types.rs` - NEW: `SearchStep`, `SearchFilters`, `SearchPipeline` (or placed in `backend/src/types.rs`)
- `backend/tests/test_snowball.rs` - NEW: Unit tests for stemmer module
- `backend/tests/test_search.rs` - NEW: Integration tests for indexing and searching
- `cli/src/bootstrap/mod.rs` - Add index building stages to bootstrap pipeline
- `cli/Cargo.toml` - May need tantivy or backend search dependency updates
- `bridges/src/asset_manager.rs` - Merge index directories during download extraction
- `assets/qml/DownloadAppdataWindow.qml` - Add `index.tar.bz2` to initial setup download URLs
- `Makefile` - Add `compile-stemmers` target
- `pali-stemmer-in-snowball/assets/snowball/algorithms/*.sbl` - Source Snowball algorithm files (read-only reference)
- `pali-stemmer-in-snowball/algorithms/pali.sbl` - Custom Pali stemmer source (read-only reference)
- `pali-stemmer-in-snowball/assets/rust-stemmers-simsapa/src/lib.rs` - Reference for `Algorithm` enum and `Stemmer` struct patterns (read-only reference)
- `pali-stemmer-in-snowball/pali-search/src/tokenizer.rs` - Reference for NiggahitaNormalizer, PaliStemmerFilter (read-only reference)
- `pali-stemmer-in-snowball/pali-search/src/searcher.rs` - Reference for dual-field querying pattern (read-only reference)
- `pali-stemmer-in-snowball/pali-search/src/indexer.rs` - Reference for Tantivy schema and index building (read-only reference)

### Notes

- Generated Snowball `.rs` stemmer files are committed to the repo. The Snowball compiler binary is a dev-time tool only, not a build dependency.
- Use `cd backend && cargo test` to run backend tests. Use `cd backend && cargo test test_name` for a specific test.
- Use `make build -B` to verify full compilation (CMake + Qt6 + Rust).
- Avoid running the GUI app for testing — use compilation checks and unit/integration tests only.
- Use `try_exists()` instead of `.exists()` for all file/directory checks (Android compatibility).

## Tasks

- [ ] 1.0 Set up Snowball stemmers module in backend
  - [ ] 1.1 Create `backend/src/snowball/` directory structure: `mod.rs`, `among.rs`, `snowball_env.rs`, `algorithms/mod.rs`
  - [ ] 1.2 Copy the Snowball Rust runtime files (`among.rs`, `snowball_env.rs`) from `pali-stemmer-in-snowball/assets/snowball/rust/src/snowball/` into `backend/src/snowball/`. Fix `use` paths to use `crate::snowball::` instead of `snowball::`.
  - [ ] 1.3 Add a `compile-stemmers` Makefile target that iterates over all `.sbl` files in `pali-stemmer-in-snowball/assets/snowball/algorithms/` (excluding `dutch_porter.sbl`, `lovins.sbl`, `porter.sbl`) plus `pali-stemmer-in-snowball/algorithms/pali.sbl`, compiles each to Rust with `snowball {file}.sbl -rust -o backend/src/snowball/algorithms/{lang}`, and applies `sed -i 's/use snowball::/use crate::snowball::/g'` to each generated file.
  - [ ] 1.4 Run `make compile-stemmers` to generate all 32 stemmer `.rs` files into `backend/src/snowball/algorithms/`.
  - [ ] 1.5 Create `backend/src/snowball/algorithms/mod.rs` with `pub mod` declarations for all generated stemmer modules.
  - [ ] 1.6 Create `backend/src/snowball/mod.rs` with: `Algorithm` enum (all 32 languages + fallback), `Stemmer` struct with `new(algo) -> Self` and `stem(word) -> Cow<str>` methods, and `lang_to_algorithm(lang_code: &str) -> Algorithm` mapping function (per PRD section 4.1 table). Reference `pali-stemmer-in-snowball/assets/rust-stemmers-simsapa/src/lib.rs` for the pattern.
  - [ ] 1.7 Register the `snowball` module in `backend/src/lib.rs` (or `backend/src/main.rs` / module root).
  - [ ] 1.8 Write unit tests in `backend/tests/test_snowball.rs`:
    - Verify Pali stemmer citation forms: basic a-stems ("dhammo" → "dhamma", "bhikkhūnaṁ" → "bhikkhu"), exception list words ("nibbānaṁ" → "nibbāna"), consonantal stems ("bhagavantaṁ" → "bhagavant"), verb forms ("passati" → expected root form), niggahīta normalization (ṃ → ṁ), root marker stripping ("√kar" → "kar").
    - Verify English stemmer: "suffering" → "suffer", "running" → "run".
    - Verify `lang_to_algorithm()` returns correct mappings: `pli` → Pali, `san` → Pali, `en` → English, `de` → German, unknown code → English fallback.
  - [ ] 1.9 Run `cd backend && cargo test test_snowball` and verify all tests pass. Run `make build -B` to verify full compilation.

- [ ] 2.0 Add Tantivy dependency and create search module with tokenizers and schemas
  - [ ] 2.1 Add `tantivy = "0.25"` to `backend/Cargo.toml` dependencies.
  - [ ] 2.2 Create `backend/src/search/` directory with `mod.rs`.
  - [ ] 2.3 Implement `backend/src/search/tokenizer.rs`:
    - `NiggahitaNormalizer` as a Tantivy `TokenFilter` (normalizes ṃ U+1E43 → ṁ U+1E41, strips √ root markers). Reference `pali-stemmer-in-snowball/pali-search/src/tokenizer.rs`.
    - `StemmerFilter` as a Tantivy `TokenFilter` that uses the `snowball::Stemmer` for the index's language.
    - `register_tokenizers(index: &Index, lang: &str)` function that registers three analyzers on the index:
      - `{lang}_stem`: `SimpleTokenizer → RemoveLong(50) → LowerCaser → NiggahitaNormalizer → StemmerFilter → AsciiFoldingFilter`
      - `simple_fold`: `SimpleTokenizer → LowerCaser → AsciiFoldingFilter`
      - `{lang}_normalize`: `SimpleTokenizer → RemoveLong(50) → LowerCaser → NiggahitaNormalizer → AsciiFoldingFilter`
  - [ ] 2.4 Implement `backend/src/search/schema.rs`:
    - `build_sutta_schema(lang: &str) -> Schema` — creates the sutta index schema with fields: `uid` (simple_fold, stored), `title` (simple_fold, stored), `language` (raw, stored), `source_uid` (raw, stored), `sutta_ref` (simple_fold, stored), `nikaya` (raw, stored), `content` ({lang}_stem, stored), `content_exact` ({lang}_normalize, stored).
    - `build_dict_schema(lang: &str) -> Schema` — creates the dictionary index schema with fields: `uid` (simple_fold, stored), `word` (simple_fold, stored), `synonyms` (simple_fold, stored), `language` (raw, stored), `source_uid` (raw, stored), `content` ({lang}_stem, stored), `content_exact` ({lang}_normalize, stored).
  - [ ] 2.5 Register the `search` module in `backend/src/lib.rs`.
  - [ ] 2.6 Run `cd backend && cargo test` and `make build -B` to verify compilation with the new tantivy dependency.

- [ ] 3.0 Implement index building (indexer) and index storage paths
  - [ ] 3.1 Extend `AppGlobalPaths` in `backend/src/lib.rs` with three new fields: `index_dir: PathBuf` (`{app_assets_dir}/index/`), `suttas_index_dir: PathBuf` (`{index_dir}/suttas/`), `dict_words_index_dir: PathBuf` (`{index_dir}/dict_words/`). Initialize them in the constructor.
  - [ ] 3.2 Implement `backend/src/search/indexer.rs`:
    - `open_or_create_index(dir: &Path, schema: Schema, lang: &str) -> Result<Index>` — creates the directory (using `try_exists()`), opens or creates a Tantivy `MmapDirectory` index, calls `register_tokenizers()`.
    - `build_sutta_index(db_url: &str, index_dir: &Path, lang: &str) -> Result<()>` — queries suttas for the given language from the database, builds content by prepending `sutta_ref`, `title`, `title_pali` to `content_plain`, skips entries with no content, writes documents to the index using a 50MB writer. Ensures `IndexWriter` is committed and dropped before returning.
    - `build_dict_index(db_url: &str, index_dir: &Path, lang: &str) -> Result<()>` — queries dict_words for the given language, builds content by prepending `word` and `synonyms` to `definition_plain`, skips entries with no definition, writes documents.
    - `build_all_indexes(db_url: &str, paths: &AppGlobalPaths) -> Result<()>` — queries distinct languages from both suttas and dict_words tables, calls `build_sutta_index` and `build_dict_index` for each.
    - `get_sutta_languages(db_url: &str) -> Result<Vec<String>>` and `get_dict_word_languages(db_url: &str) -> Result<Vec<String>>` — `SELECT DISTINCT language` queries.
  - [ ] 3.3 Create an `INDEX_VERSION` constant (e.g., `"1.0"`) and a function `write_version_file(index_dir: &Path)` that writes a `VERSION` file containing the index schema version and stemmer version. Add a corresponding `read_version_file()` and `is_index_current()` check.
  - [ ] 3.4 Write integration tests in `backend/tests/test_search.rs`: create a temporary directory, build a small sutta index with test data, verify the index directory and Tantivy files are created, verify the VERSION file is written. Similarly test dict_word indexing.
  - [ ] 3.5 Run `cd backend && cargo test test_search` and verify tests pass.

- [ ] 4.0 Implement fulltext search (searcher) and integrate with query_task
  - [ ] 4.1 Implement `backend/src/search/searcher.rs`:
    - `FulltextSearcher` struct holding a map of language → `(Index, IndexReader)` pairs for both suttas and dict_words.
    - `FulltextSearcher::open(paths: &AppGlobalPaths) -> Result<Self>` — scans `suttas_index_dir` and `dict_words_index_dir` for per-language subdirectories, opens each index and creates a reader, registers tokenizers.
    - `search_suttas(query: &str, filters: &SearchFilters, page_len: usize) -> Result<Vec<SearchResult>>` — implements dual-field querying: `content` field with `Occur::Must`, `content_exact` field with `Occur::Should` and 2.0x boost. Adds `TermQuery` filters for `language`, `source_uid`, `nikaya`, `sutta_ref` as applicable. If no language filter, searches all language indexes and merges results interleaved by score (do NOT group by language). Do NOT search the `title` field — title matches produce misleading results (title text is already prepended to `content` per requirement 20).
    - `search_dict_words(query: &str, filters: &SearchFilters, page_len: usize) -> Result<Vec<SearchResult>>` — same dual-field pattern for dictionary indexes. Do NOT search the `word` field directly (word text is prepended to `content` per requirement 21).
    - Snippet generation using Tantivy's `SnippetGenerator` with max 200 chars, post-processing `<b>`/`</b>` → `<span class='match'>`/`</span>`.
  - [ ] 4.2 Wire `FulltextSearcher` into the application lifecycle: initialize it (lazily or at startup) and make it accessible from `SearchQueryTask`. Consider storing it in `AppGlobals` or a similar shared state.
  - [ ] 4.3 In `backend/src/query_task.rs`, implement the `SearchMode::FulltextMatch` branch in `results_page()`:
    - Call `FulltextSearcher::search_suttas()` or `search_dict_words()` based on `SearchArea`.
    - Verify the existing `SearchResult` struct in `types.rs` has all required fields: `uid`, `title`, `source_uid`, `snippet`, `score`, `language`, `sutta_ref`, `nikaya`, `word`. Add any missing fields.
    - Convert results into `SearchResult` structs.
    - Populate `highlighted_result_pages` and `db_query_hits_count` for pagination.
  - [ ] 4.4 Add search tests: index test data, search with stemmed terms (e.g., search "bhikkhu" matches "bhikkhūnaṁ"), verify dual-field boost ranking, verify filter narrowing, verify snippet HTML uses `<span class='match'>`.
  - [ ] 4.5 Run `cd backend && cargo test` and `make build -B` to verify everything compiles and tests pass.

- [ ] 5.0 Add CLI commands for index management
  - [ ] 5.1 Add `index` subcommand group to the CLI (`cli/`) with the following subcommands:
    - `simsapa index build` — builds all sutta and dict_word indexes for all languages found in the database.
    - `simsapa index build --area suttas` — builds only sutta indexes.
    - `simsapa index build --area dict_words` — builds only dictionary indexes.
    - `simsapa index build --lang pli` — builds indexes only for the specified language.
    - `simsapa index rebuild` — deletes existing index directories and rebuilds from scratch.
  - [ ] 5.2 Implement the CLI handlers that call the backend `search::indexer` functions (`build_all_indexes`, `build_sutta_index`, `build_dict_index`). Add progress logging with `tracing`.
  - [ ] 5.3 Test CLI commands manually: `cargo run -p cli -- index build`, verify index directories are created under the configured `index_dir`.

- [ ] 6.0 Integrate index building into bootstrap pipeline
  - [ ] 6.1 In `cli/src/bootstrap/mod.rs`, after the appdata archive creation (step 3 in the current pipeline), add a call to build fulltext indexes for base languages (en, pli, san) and all dict_words languages.
  - [ ] 6.2 After building base indexes, create `index.tar.bz2` from the `index/` directory and move it to the release directory. Ensure the `IndexWriter` is dropped/committed before tar runs.
  - [ ] 6.3 In the per-language loop (step 5), after importing each language's suttas into its `.sqlite3`, call `build_sutta_index()` for that language. Modify `create_database_archive()` (or create a new function) to include both the `.sqlite3` file and `index/suttas/{lang}/` in the per-language tarball.
  - [ ] 6.4 Call `write_version_file()` before creating `index.tar.bz2` so `index/VERSION` is included. Also ensure the VERSION file is present in per-language archives (copy it into each per-language index directory or include it at the `index/` level in the tarball).
  - [ ] 6.5 Test the bootstrap pipeline end-to-end: run a bootstrap, verify `index.tar.bz2` is created with the expected directory structure, verify per-language archives include `index/suttas/{lang}/`.

- [ ] 7.0 Integrate index distribution with download flow
  - [ ] 7.1 In `assets/qml/DownloadAppdataWindow.qml`, add `index.tar.bz2` to the initial setup URL list in `run_download()`, alongside `appdata.tar.bz2`, `dictionaries.tar.bz2`, `dpd.tar.bz2`.
  - [ ] 7.2 Verify that `move_folder_contents()` in `bridges/src/asset_manager.rs` correctly merges the extracted `index/` directory into `app-assets/index/`. The existing mechanism should handle this — confirm with a manual test or code review.
  - [ ] 7.3 Verify per-language downloads: after downloading and extracting `suttas_lang_{code}.tar.bz2` (which now includes `index/suttas/{lang}/`), the index files should land at `app-assets/index/suttas/{lang}/`. No code changes expected — just confirm the existing extraction logic handles the nested directory structure.

- [ ] 8.0 Add extensible pipeline types and UI re-index action
  - [ ] 8.1 Define the extensible pipeline types in `backend/src/types.rs` (or `backend/src/search/types.rs`): `SearchStep` (mode, query_text, filters), `SearchFilters` (lang, source_uid, nikaya, sutta_ref with include booleans), `SearchPipeline` (steps, area, page_len). Only single-step `FulltextMatch` is implemented for now.
  - [ ] 8.2 Add a "Rebuild Search Index" action to the application menu or settings panel in QML. This should call a Rust bridge function that triggers `build_all_indexes()` in a background thread.
  - [ ] 8.3 Add a progress indicator for re-indexing (e.g., emit a Qt signal with progress percentage or per-language status updates).
  - [ ] 8.4 Add missing-index detection: on app startup (or when fulltext search is first attempted), check whether the expected index directories exist (using `try_exists()`). If indexes are missing, show a notification to the user suggesting they build or download indexes. Do NOT auto-build.
  - [ ] 8.5 Add index VERSION check: on startup, call `is_index_current()`. If the version file is missing or stale, notify the user that re-indexing is recommended.
  - [ ] 8.6 Run `make build -B` and `make qml-test` to verify full compilation and QML tests pass.
