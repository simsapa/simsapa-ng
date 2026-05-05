# PRD: DPPN Page Rendering and Cross-Reference Links

## 1. Introduction / Overview

The Dictionary of Pāli Proper Names (DPPN, by Ānandajoti Bhikkhu) is imported
during the bootstrap procedure from
`bootstrap-assets-resources/dppn-anandajoti/dppn.sqlite3`, with its source
content originally drawn from the EPUB at
`bootstrap-assets-resources/dppn-anandajoti/DPPN-Complete/`.

Two problems exist with how DPPN entries currently render in the app:

1. **No styling.** The `definition_html` field stored in
   `dictionaries.sqlite3.dict_words` is an HTML *fragment* (not a full
   document). The dictionary rendering path in
   `backend/src/app_data.rs::render_word_uid_to_html` is built around full
   HTML documents — it expects `<html>`, `<head>`, `<body>` tags it can
   rewrite to inject the standard page chrome and `dictionary.css`. For DPPN
   fragments, those rewrites are no-ops, so DPPN entries render unstyled,
   without the standard page CSS/JS that suttas and DPD bold-definitions get.

2. **No cross-reference navigation.** DPPN definitions are full of
   cross-references to other DPPN entries, marked up in the source as
   `<span class="t14">Name</span>` (bold navy in the EPUB style). These spans
   are inert text in the app — clicking them does nothing. Users cannot
   navigate from one DPPN entry to a referenced entry without copy-pasting
   into the dictionary search box.

This feature wraps DPPN entries in the standard page chrome (with
`dictionary.css`) and converts `t14` spans into clickable links that, when
clicked, run a background DPPN-only Fulltext Match query in the dictionary
tab. The user then picks the most relevant match from the result list. We
use Fulltext (rather than exact UID lookup) because DPPN cross-references
are written for human readers and don't always match a single canonical
`uid` value.

## 2. Goals

1. DPPN entries render with the standard sutta/dictionary page chrome
   (`page.html` template, `dictionary.css`, `simsapa.min.js`), the same way
   DPD bold-definition entries do today.
2. EPUB style classes actually used in `definition_html` (`t14`, `t15`,
   `t17`, `t18`, `t19`, `t20`, `t21`, `t25`, `t26`, `t28`, `t29`) render
   with adapted styling under a `.dppn` scope in `dictionary.css`.
3. `<span class="t14">…</span>` cross-references become clickable
   `<a>` links using the `ssp://dppn_lookup/<query>` scheme. Clicking one
   triggers a DPPN-only Fulltext Match query in the dictionary tab without
   disturbing the user's current search-mode / filter UI state.
4. The transformation happens at **bootstrap time** (the wrapping `<div
   class="dppn">…</div>` and `t14 → <a>` rewrite are baked into the stored
   `definition_html`), so no runtime regex work is needed per render.
5. The runtime API port is injected at render time (via the existing
   `API_URL` JS global from `page.html`) — the bootstrap-stored HTML never
   hardcodes a port.

## 3. User Stories

- **As a Dhamma reader**, when I open the DPPN entry for `Hatthaka Āḷavaka`,
  I want it to display in the same legible style as other dictionary entries
  (proper fonts, dark-mode aware, page chrome) instead of as raw unstyled
  text.
- **As a Dhamma reader**, when I see a name like `Ānanda` inside a DPPN
  entry, I want to click it and have the dictionary look up matching DPPN
  entries so I can jump to the relevant one.
- **As a Dhamma reader**, when I click a DPPN cross-reference, I want my
  current dictionary search settings (mode, dict filters, search box
  contents) to remain untouched — only the visible result list should
  update.

## 4. Functional Requirements

### 4.1 Bootstrap import (`cli/src/bootstrap/dppn.rs`)

1. The bootstrap import MUST transform each source `definition_html`
   fragment before storing it in `dict_words.definition_html`:
   1. Wrap the entire fragment in `<div class="dppn">…</div>`.
   2. Replace every `<span class="t14">TEXT</span>` with
      `<a class="dppn-ref" href="ssp://dppn_lookup/{TEXT_URL_ENCODED}"><span class="t14">TEXT</span></a>`,
      where `TEXT_URL_ENCODED` is the inner text **trimmed** of leading/
      trailing whitespace and URL-encoded (preserving diacritics via
      percent-encoded UTF-8). The inner `<span class="t14">` is preserved
      so the cross-reference text retains its visual styling.
2. The transformation MUST handle nested or adjacent `t14` spans correctly
   (each span becomes its own link).
3. The transformation MUST NOT modify spans of any other class.
4. `definition_plain` (used for full-text indexing) MUST continue to be
   produced from the original (or transformed — both yield the same plain
   text via `compact_rich_text`) HTML so search results are unchanged.

### 4.2 Rendering (`backend/src/app_data.rs` /
`backend/src/html_content.rs`)

5. The dictionary rendering path MUST detect DPPN entries (by
   `dict_label == "dppn"` on the matched `DictWord`) and route them through
   a full-page wrapper analogous to `render_bold_definition` — that is,
   call `sutta_html_page` with the `definition_html` as the content and
   `DICTIONARY_CSS` as the extra CSS.
6. The wrapper MUST inject `WINDOW_ID` and the standard JS context
   (`API_URL`, `IS_MOBILE`, etc.) the same way `render_bold_definition`
   does, so `simsapa.min.js` link-click handlers attach correctly.
7. Existing dictionary entries with `dict_label != "dppn"` (e.g. DPD,
   StarDict imports) MUST continue to render via the existing full-document
   rewrite path. No regression.

### 4.3 Styling (`assets/css/dictionary.css`)

8. New CSS rules MUST be added under the `.dppn` scope, adapting only the
   classes actually present in DPPN `definition_html`:
   - `.dppn .t14` — bold navy (cross-reference; also styled via
     `.dppn a.dppn-ref` for hover/cursor)
   - `.dppn .t15` — bold blue
   - `.dppn .t17`, `.dppn .t27` — purple
   - `.dppn .t18`, `.dppn .t23`, `.dppn .t26`, `.dppn .t33` — italic
     variants
   - `.dppn .t19` — bold purple
   - `.dppn .t20`, `.dppn .t25`, `.dppn .t30` — maroon variants
   - `.dppn .t21` — Times New Roman fallback
   - `.dppn .t28` — bold italic navy
   - `.dppn .t29` — blue
9. Each rule MUST have a matching `.dark .dppn .tNN` override that swaps
   the navy/blue/maroon/purple foreground for a readable dark-theme color
   (consistent with the existing `.dark .bold-definition-*` palette).
10. `.dppn a.dppn-ref` MUST display as a standard underlined link
    (underlined by default, `cursor: pointer`) while inheriting the `t14`
    color (bold navy in light mode, adapted color in dark mode).
11. The DPPN CSS MUST NOT leak into other dictionary entries — every new
    rule MUST be prefixed with `.dppn `.

### 4.4 Link click handling (`src-ts/helpers.ts`,
`src-ts/simsapa.ts`)

12. `extract_sutta_uid_from_link` (or a new sibling routine) MUST recognize
    `ssp://dppn_lookup/<query>` URLs.
13. `handle_link_click` MUST, on encountering a DPPN lookup link:
    1. `event.preventDefault()`.
    2. URL-decode `<query>`.
    3. POST to a new backend endpoint (see 4.5) with the decoded query and
       the current `WINDOW_ID`.
14. The TypeScript MUST follow the existing pattern (use the `API_URL`
    global pulled from `page.html`; do not hardcode a port).
15. `attach_link_handlers_to_element` MUST also tag DPPN-ref anchors with
    a `dppn-ref` class for any future styling hooks (already present from
    bootstrap, but the JS scan SHOULD NOT remove or duplicate it).

### 4.5 Backend API endpoint (`bridges/src/api.rs`)

16. A new endpoint MUST be added (POST with JSON body, mirroring
    `suttas_fulltext_search` in `bridges/src/api.rs:845`):
    ```
    POST /dppn_lookup
    body: { window_id: String, query: String }
    ```
17. The handler MUST invoke a new FFI callback (added to the `extern
    "C++"` block alongside `callback_run_lookup_query` etc.):
    ```
    fn callback_run_dppn_dictionary_query(window_id: QString, query: QString);
    ```
18. The C++ side MUST wire this callback to a slot on the dictionary
    tab/window that:
    1. Runs a Fulltext Match query in the dictionary area, restricted to
       `dict_source_uids = ["dppn"]`.
    2. Reveals the side panel and activates the results tab — the same
       UI flow used by other callbacks that run lookups
       (`callback_run_lookup_query`, `callback_open_in_lookup_window`).
    3. Pushes the resulting page into the dictionary results panel.
    4. Does NOT modify persisted `search_last_mode["Dictionary"]`,
       persisted dict filters, or the search input field that the user is
       currently editing.
19. The endpoint MUST be registered in the Rocket `routes![…]` block in
    `start_webserver`.

### 4.6 Documentation

20. `PROJECT_MAP.md` MUST be updated with:
    - A note under "Content Rendering" pointing at the new DPPN render
      path.
    - A note under "Search & Lookup" describing the
      `/dppn_lookup` endpoint and its non-disruptive query semantics.

## 5. Non-Goals (Out of Scope)

- **Backward compatibility for installed databases.** This is a new app
  version; the local DB will be re-bootstrapped before testing. No
  migration / runtime fallback is required for un-wrapped DPPN HTML.
- **Linking other span classes.** Only `t14` becomes a link. `t18`
  (italic Pāli phrases), `t17` (purple highlighted terms), etc. remain
  display-only.
- **Disambiguation / "did you mean" UI.** The user picks from the
  Fulltext result list; no fuzzy-match scoring or auto-redirect on a
  single hit.
- **Linking to non-DPPN dictionaries** from a DPPN cross-reference. The
  lookup is scoped to DPPN only.
- **Re-importing the EPUB HTML.** We continue to use the existing
  `dppn.sqlite3` source — no EPUB re-parsing.
- **Persisting / restoring the user's pre-click search state** as an
  undoable action. The user's UI state is simply left alone; no
  "navigate back" history is added.
- **Fonts from the EPUB** (`ITM_TMS_UNI`). We use the app's existing
  font stack.

## 6. Design Considerations

- **Mirror `render_bold_definition`.** The cleanest implementation adds
  a `render_dppn_entry(word, window_id, body_class)` in
  `backend/src/html_content.rs` that wraps the (already-prefixed-with-
  `<div class="dppn">`) `definition_html` and routes through
  `sutta_html_page` with `DICTIONARY_CSS` extra and a `WINDOW_ID` JS
  injection. The dispatcher in `app_data.rs::render_word_uid_to_html`
  branches: if `dict_label == "dppn"`, call the new renderer; else
  current path.
- **CSS color palette.** Reuse the dark-theme overrides from existing
  `.bold-definition-*` rules where possible (greyed maroons, lighter
  navy) so DPPN sits visually within the existing dictionary look.
- **URL encoding.** The bootstrap MUST use percent-encoding of UTF-8
  bytes for `<query>` in `ssp://dppn_lookup/<query>` so diacritics
  (`Vaṅgīsa` → `Va%E1%B9%85g%C4%ABsa`) survive HTML attribute escaping
  and the JS `decodeURIComponent` round-trip on click.
- **Span-text extraction.** When the bootstrap rewrites a `t14` span,
  the `<query>` MUST be the inner text content of the span, not any
  surrounding punctuation.

## 7. Technical Considerations

- **Rust HTML rewriting.** Prefer a small targeted regex pass for the
  `t14 → <a>` rewrite over a full HTML parse (the source is uniform and
  well-formed; regex matches the existing project style for HTML
  post-processing in `app_data.rs`).
- **`compact_rich_text`** already strips tags for `definition_plain`;
  it will treat the new `<a>` wrappers transparently. No change needed
  there.
- **DPPN `dict_label`** is set to `"dppn"` during bootstrap — confirmed
  in `cli/src/bootstrap/dppn.rs:113`. The render-time branch can rely
  on this exact value.
- **No new tantivy index work.** Fulltext Match dictionary search
  already supports `dict_source_uids` push-down (see PROJECT_MAP §
  Search & Lookup). The new endpoint is a thin caller of the existing
  pipeline.
- **C++ slot wiring.** The new FFI callback follows the existing
  pattern used by `callback_run_lookup_query` and friends — declare in
  `api.rs`, implement in the matching C++ TU, and route to the
  appropriate window/tab via the `WINDOW_ID`.
- **QML `qmllint` dummies.** No new `SuttaBridge` method is added in
  Rust, so `assets/qml/com/profoundlabs/simsapa/SuttaBridge.qml` does
  not need an update for this feature. (If the C++ slot ends up exposed
  to QML, add a matching stub there.)

## 8. Success Metrics

- A DPPN entry (e.g. `hatthaka-āḷavaka/dppn`) renders with the standard
  page chrome and adapted DPPN styles (visually verified).
- Clicking a `t14` cross-reference (e.g. `Vaṅgīsa`) updates the
  dictionary results panel with DPPN-only Fulltext matches for that
  query, without changing the search input field, search mode, or dict
  filter checkboxes.
- The user's prior dictionary search results are *replaced*, but their
  search-input state and persisted settings are unchanged after page
  reload.
- No regression in rendering for non-DPPN dictionary entries (DPD,
  StarDict imports, bold-definitions).
- `cd backend && cargo test` passes.

## 9. Open Questions

(None remaining — all clarifying questions resolved.)
