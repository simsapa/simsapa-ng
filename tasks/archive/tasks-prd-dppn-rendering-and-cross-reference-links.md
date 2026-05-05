# Tasks: DPPN Page Rendering and Cross-Reference Links

PRD: [prd-dppn-rendering-and-cross-reference-links.md](./prd-dppn-rendering-and-cross-reference-links.md)

## Relevant Files

- `cli/src/bootstrap/dppn.rs` — Bootstrap import; gained `transform_dppn_definition_html()` which wraps fragments in `<div class="dppn">` and rewrites `t14` spans into `ssp://dppn_lookup/...` anchors. Includes unit tests for wrapper, multiple t14 spans, untouched other classes, diacritic encoding, and whitespace trimming.
- `cli/Cargo.toml` — Added `urlencoding = "2.1"` for percent-encoding the cross-reference query in the URL.
- `backend/src/html_content.rs` — Added `render_dppn_entry()` mirroring `render_bold_definition()`; routes through `sutta_html_page` with `DICTIONARY_CSS` and `WINDOW_ID` injection.
- `backend/src/app_data.rs` — `render_word_html_by_uid()`: added a `dict_label == "dppn"` branch that calls the new renderer instead of the full-document rewrite path.
- `backend/tests/test_dppn_render.rs` — New test verifying the bootstrap-style `<div class="dppn">` HTML renders inside the standard page chrome with `<style>`, suttas.js, and `WINDOW_ID` present, and not double-wrapped.
- `backend/tests/test_dppn_bootstrap.rs` — New test for the bootstrap rewrite helper: input fragment with multiple `t14` spans → wrapped, links present, diacritics percent-encoded, other span classes untouched.
- `assets/css/dictionary.css` — Add `.dppn`-scoped rules for `t14, t15, t17, t18, t19, t20, t21, t25, t26, t28, t29` plus dark-mode overrides and `a.dppn-ref` link styling.
- `assets/sass/` — If the `.dppn` rules should originate in Sass (check existing convention), add the source partial there and let `make sass` regenerate `dictionary.css`. Otherwise edit `dictionary.css` directly.
- `src-ts/helpers.ts` — Recognise `ssp://dppn_lookup/<query>` URLs and POST to `/dppn_lookup`; extend `handle_link_click` to dispatch them.
- `src-ts/simsapa.ts` — Tag `a.dppn-ref` anchors during `attach_link_handlers_to_element` (idempotent — class is already set at bootstrap).
- `bridges/src/api.rs` — New `POST /dppn_lookup` Rocket handler; new `callback_run_dppn_dictionary_query` FFI declaration in the `extern "C++"` block; mount in `routes![…]`.
- `cpp/gui.h`, `cpp/gui.cpp` — Declare and define `callback_run_dppn_dictionary_query()`; emit a new `WindowManager` signal.
- `cpp/window_manager.h`, `cpp/window_manager.cpp` — New signal + slot `run_dppn_dictionary_query(window_id, query)`; locate target window by `window_id`, invoke a new QML method.
- `assets/qml/SuttaSearchWindow.qml` — New QML function `run_dppn_dictionary_query(query)` that does NOT touch `search_bar_input`; reveals side panel (`show_sidebar_btn.checked = true`), activates Dictionary tab (`rightside_tabs.setCurrentIndex(1)`), runs a Fulltext Match query in the Dictionary area with `dict_source_uids = ["dppn"]`, and pushes results into the dictionary results panel.
- `PROJECT_MAP.md` — Note the DPPN render path under "Content Rendering" and the `/dppn_lookup` endpoint under "Search & Lookup".

### Notes

- After each top-level task the build should be green: `make build -B`. Run `cd backend && cargo test` only after all sub-tasks of a top-level task complete.
- Do not run `make qml-test` unless explicitly asked.
- This is a fresh-bootstrap feature — no migration / runtime fallback for installed databases.

## Tasks

- [x] 1.0 Implement bootstrap-time DPPN HTML transformation
  - [x] 1.1 In `cli/src/bootstrap/dppn.rs`, add a private helper `transform_dppn_definition_html(fragment: &str) -> String` that wraps the input in `<div class="dppn">…</div>` and rewrites every `<span class="t14">TEXT</span>` to `<a class="dppn-ref" href="ssp://dppn_lookup/{ENCODED}"><span class="t14">TEXT</span></a>`, where `ENCODED` is `TEXT.trim()` percent-encoded as UTF-8 (use `urlencoding` or `percent-encoding` crate — check `bridges`/`backend` Cargo.toml for an already-present dep before adding a new one).
  - [x] 1.2 Use a single targeted regex (case-insensitive on the class attribute is unnecessary — the source is uniform) plus `regex::Captures` callback for the rewrite. Verify spans of other classes (`t15`, `t17`, `t18`, …) pass through untouched.
  - [x] 1.3 Wire the helper into the per-row mapping in `dppn_bootstrap()` so the stored `definition_html` is the transformed string. `definition_plain` should continue to be derived via `compact_rich_text()` (input either pre- or post-transform yields the same plain text — pick one, document choice in a one-liner if non-obvious).
  - [x] 1.4 Add `backend/tests/test_dppn_bootstrap.rs` (or a unit test inside `dppn.rs` if the helper is `pub(crate)`-accessible from tests) covering: (a) wrapper present once; (b) two adjacent `t14` spans both rewritten; (c) a `t18` span left untouched; (d) diacritics in query (`Vaṅgīsa`) percent-encoded; (e) leading/trailing whitespace inside the span trimmed before encoding.
  - [x] 1.5 `make build -B` clean. Re-run the bootstrap against a scratch DB and spot-check one DPPN row to confirm the wrapped HTML and links appear in `dictionaries.sqlite3.dict_words`.

- [x] 2.0 Route DPPN entries through the standard page-chrome renderer
  - [x] 2.1 In `backend/src/html_content.rs`, add `pub fn render_dppn_entry(word: &DictWord, window_id: &str, body_class: Option<String>) -> String` that builds `js_extra` with `WINDOW_ID` (mirroring `render_bold_definition`) and calls `sutta_html_page(definition_html, None, Some(DICTIONARY_CSS.to_string()), Some(js_extra), body_class)`. The DPPN HTML is already wrapped in `<div class="dppn">…</div>` from bootstrap — do not double-wrap.
  - [x] 2.2 In `backend/src/app_data.rs::render_word_uid_to_html` (around line 432), add a branch *before* the existing full-document path: if `word.dict_label == "dppn"` and `word.definition_html.is_some()`, call `render_dppn_entry(&word, window_id, Some(body_class.clone()))` and return.
  - [x] 2.3 Confirm the existing non-DPPN branch (DPD `/dpd`, StarDict, etc.) is untouched — its regex-based `<html>/<head>/<body>` rewriting still runs for any `dict_label != "dppn"`.
  - [x] 2.4 Add `backend/tests/test_dppn_render.rs` that constructs a synthetic `DictWord` with `dict_label = "dppn"` and a small `<div class="dppn">…</div>` definition, calls `render_word_uid_to_html`, and asserts the output contains: `<style>` with `.dppn` rules from `DICTIONARY_CSS`, the `simsapa.min.js`-equivalent inline JS, and the `WINDOW_ID` const.
  - [x] 2.5 `cd backend && cargo test` passes (after all sub-tasks of 2.0 done).

- [ ] 3.0 Add `.dppn`-scoped styles to `dictionary.css`
  - [x] 3.1 Decide whether to edit `assets/css/dictionary.css` directly or add a Sass partial under `assets/sass/` (check if `dictionary.css` is generated from a Sass source by looking at `make sass` output mapping). Use whichever is the canonical source.
  - [x] 3.2 Add `.dppn` light-mode rules adapted from `bootstrap-assets-resources/dppn-anandajoti/DPPN-Complete/Ops/style.css` for the classes actually used: `.dppn .t14`, `.t15`, `.t17`, `.t18`, `.t19`, `.t20`, `.t21`, `.t25`, `.t26`, `.t28`, `.t29` (port colour/weight/style only — drop EPUB-specific font-family `ITM_TMS_UNI`, paragraph margins/indents).
  - [x] 3.3 Add matching `.dark .dppn .tNN` overrides using a palette consistent with the existing `.bold-definition-*` dark rules (avoid raw `navy` / `maroon` on the dark background).
  - [x] 3.4 Add `.dppn a.dppn-ref { text-decoration: underline; cursor: pointer; color: inherit; }` plus a `.dark .dppn a.dppn-ref` colour override if needed. The inner `<span class="t14">` continues to colour the text.
  - [x] 3.5 Verify every new selector starts with `.dppn ` — no leakage into other dictionary entries.
  - [x] 3.6 Run `make sass` if the source was Sass; `make build -B` clean.

- [ ] 4.0 Wire `ssp://dppn_lookup/<query>` link handling in TypeScript
  - [x] 4.1 In `src-ts/helpers.ts`, add a small helper (or extend the case dispatch in `handle_link_click`) that detects `href.startsWith('ssp://dppn_lookup/')`, extracts and `decodeURIComponent`s the query, and POSTs to `${API_URL}/dppn_lookup` with body `{ window_id: WINDOW_ID, query }`. Follow the existing `open_book_page_in_tab` pattern for fetch + error handling.
  - [x] 4.2 Insert the new case in `handle_link_click` *before* the generic `ssp://` sutta-extraction case so DPPN refs don't get misrouted as sutta UIDs.
  - [x] 4.3 In `src-ts/simsapa.ts::attach_link_handlers_to_element`, ensure `a.dppn-ref` anchors get the click handler (they will via the existing `links.forEach` — just confirm the `dppn-ref` class set at bootstrap is preserved; do not add it again). No other change needed.
  - [x] 4.4 Build TypeScript with `npx webpack` and confirm `assets/js/simsapa.min.js` updates without errors.
  - [x] 4.5 `make build -B` clean.

- [ ] 5.0 Add `POST /dppn_lookup` endpoint and FFI callback
  - [x] 5.1 In `bridges/src/api.rs`, in the `extern "C++"` block (around line 150-165), declare `fn callback_run_dppn_dictionary_query(window_id: QString, query: QString);`.
  - [x] 5.2 Add a `#[derive(Deserialize)] struct DppnLookupRequest { window_id: String, query: String }`.
  - [x] 5.3 Add a `#[post("/dppn_lookup", data = "<request>")] fn dppn_lookup(request: Json<DppnLookupRequest>) -> Status` handler that logs the call and invokes `ffi::callback_run_dppn_dictionary_query(QString::from(&request.window_id), QString::from(&request.query)); Status::Ok`. Mirror the shape of `sutta_menu_action` (line 286).
  - [x] 5.4 Register `dppn_lookup` in the `routes![…]` block in `start_webserver` (around line 1218).
  - [x] 5.5 `cargo build -p simsapa-bridges` (or `make build -B`) clean. The C++ side will not link until task 6.0 provides the callback definition — sequence so that 5.0 ends with the Rust-side change committed but expect 6.0 to be done in the same PR for a green link.

- [ ] 6.0 Wire C++ slot for non-disruptive DPPN-only Fulltext query and panel activation
  - [x] 6.1 In `cpp/gui.h` declare `void callback_run_dppn_dictionary_query(QString window_id, QString query);`. In `cpp/gui.cpp` define it to `emit AppGlobals::manager->signal_run_dppn_dictionary_query(window_id, query);` (mirror the pattern at gui.cpp:60 / gui.cpp:120).
  - [x] 6.2 In `cpp/window_manager.h` add `signal_run_dppn_dictionary_query(const QString&, const QString&)` and slot `void run_dppn_dictionary_query(const QString& window_id, const QString& query)`. Connect them in the `WindowManager` ctor (around line 31).
  - [x] 6.3 In `cpp/window_manager.cpp` implement the slot: locate the `SuttaSearchWindow` whose root has the matching `window_id` property (mirror `run_summary_query` at line 247), then `QMetaObject::invokeMethod(target_window->m_root, "run_dppn_dictionary_query", Q_ARG(QString, query))`. If no matching window is found, log and return — do NOT fall back to creating a new window (unlike `run_lookup_query`).
  - [x] 6.4 In `assets/qml/SuttaSearchWindow.qml`, add `function run_dppn_dictionary_query(query: string)` (place near the existing `run_lookup_query` at line 1140 for discoverability). It must:
    1. Reveal the side panel: `show_sidebar_btn.checked = true`.
    2. Activate the Dictionary tab: `rightside_tabs.setCurrentIndex(1)` (use the same index as `open_dict_tab` at line 1171).
    3. Build search params for a Fulltext Match query in the Dictionary area with `dict_source_uids: ["dppn"]` and `include_comm_bold_definitions: false`. Reuse whatever low-level helper feeds the dictionary results model — look for the call site that consumes `compute_dict_search_filter()` output (line 573-597) and call the same downstream function with the override params.
    4. **Must NOT** call `search_bar_input.set_search_area(...)` or assign `search_bar_input.search_input.text` (this is the key behavioural difference from `run_lookup_query`). The user's persisted `search_last_mode["Dictionary"]`, dict-filter checkboxes, and search-input contents are left intact.
  - [x] 6.5 If a new bridge function is added on the QML side, also update the `qmllint` stub in `assets/qml/com/profoundlabs/simsapa/SuttaBridge.qml` (only if the function is exposed via `SuttaBridge` — for an in-window QML function, no stub is required).
  - [x] 6.6 `make build -B` clean (Rust + C++ + QML registration all link).
  - [x] 6.7 Manual verification (user-driven, per project guidance): open a DPPN entry, click a `t14` link, confirm the side panel opens, the Dictionary results tab activates, and the results list shows DPPN-only Fulltext matches — while the search input field, search mode dropdown, and dict-filter checkboxes remain unchanged.

- [x] 7.0 Update documentation
  - [x] 7.1 In `PROJECT_MAP.md` under "Content Rendering", add a one-line entry for `render_dppn_entry` in `backend/src/html_content.rs` and the `dict_label == "dppn"` dispatch in `app_data.rs::render_word_uid_to_html`.
  - [x] 7.2 In `PROJECT_MAP.md` under "Search & Lookup", add an entry for `POST /dppn_lookup`. Note: the implemented behaviour is now **UI-driving** (sets search area to Dictionary, mode to Fulltext Match, populates the search input, solo-locks the DPPN dictionary via the panel's lock toggle) rather than the originally-specified non-disruptive semantics — the change was made to keep the query visible/editable.
  - [x] 7.3 No `docs/` page currently covers the dictionary rendering pipeline; PROJECT_MAP entries are sufficient.
