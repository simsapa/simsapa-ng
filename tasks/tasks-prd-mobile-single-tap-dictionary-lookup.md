# Tasks: Mobile Single-Tap Dictionary Word Lookup

Derived from `tasks/prd-mobile-single-tap-dictionary-lookup.md`.

## Relevant Files

- `assets/js/suttas.js` — hosts the existing desktop `dblclick` and mobile `selectionchange` handlers (lines 358–426) and the `summary_selection()` / `selection_text()` helpers (lines 28–53). Main change target.
- `assets/sass/suttas.sass` — contains `#ssp_content` rules; add mobile-scoped `user-select` / `-webkit-touch-callout` / `touch-action` rules and the `.tapped-word` highlight style here.
- `assets/sass/_base.sass` — has base `#ssp_content` rules; confirm no global `user-select` rule conflicts with the mobile overrides.
- `assets/css/suttas.css` — compiled output of `make sass`; regenerated, not hand-edited.
- `PROJECT_MAP.md` — update the "sutta rendering / JS interaction" section to describe the new single-tap flow.
- `docs/` — add a short note describing the tap-to-lookup behaviour and the mobile CSS guard (optional; only if an adjacent doc already covers WebView behaviour, e.g. `docs/mobile-webview-visibility-management.md`).

### Notes

- There is no JS test harness in this project (nothing under `tests/` and no Jest config); verification is manual per the PRD's Verification section.
- Backend has no changes: `cd backend && cargo test` should still pass unchanged.
- Build commands: `make sass` (CSS), `make build -B` (full build). Do not run `make qml-test` unless the user asks.
- All DOM / word-extraction logic must be inside `assets/js/suttas.js` (plain JS, not TypeScript). Do not create a new TS source.

## Tasks

- [ ] 1.0 Generalise `summary_selection()` to accept an explicit word string
  - [ ] 1.1 Change the signature at `assets/js/suttas.js:48` to `function summary_selection(explicit_text)` with `explicit_text` defaulting to `null`/`undefined`.
  - [ ] 1.2 Use `explicit_text` when it is a non-empty string; otherwise fall back to `selection_text()` as today. Trim the explicit value before the empty-check.
  - [ ] 1.3 Keep the existing `fetch(\`${API_URL}/summary_query/${WINDOW_ID}/${encodeURIComponent(text)}\`)` call; pass the resolved text through `encodeURIComponent`.
  - [ ] 1.4 Confirm the desktop `dblclick` call site at line 424 still works with no argument (no change needed there).
  - [ ] 1.5 Sanity check: grep the repo for other callers of `summary_selection(` to make sure none pass unexpected arguments — update them if they do.

- [ ] 2.0 Replace the mobile `selectionchange` handler with a single-tap caret-at-point word-lookup handler
  - [ ] 2.1 Inside the `DOMContentLoaded` callback at `assets/js/suttas.js:353`, locate the `if (IS_MOBILE) { ... } else { ... }` block (lines 358–426). Remove the entire mobile branch body — `previous_selection_text`, `word_summary_was_closed`, `window.word_summary_closed`, and the `selectionchange` listener (lines 361–414). Leave the desktop branch untouched.
  - [ ] 2.2 In the new mobile branch, add `document.body.classList.add('mobile')` so SCSS can target mobile-only rules (used in Task 3).
  - [ ] 2.3 Add a helper `function caret_node_offset_from_point(x, y)` that calls `document.caretPositionFromPoint(x, y)` when available and falls back to `document.caretRangeFromPoint(x, y)`. Return `{ node, offset }` or `null` if the resolved node is not a text node or the result is null/undefined.
  - [ ] 2.4 Add a helper `function word_at_offset(text_node, offset)` that returns the contiguous Pāli/English word containing `offset` in `text_node.nodeValue`. Use the Unicode-aware regex `/[\p{L}\p{M}'’]+/gu` applied to the node value; pick the match whose start ≤ offset ≤ end. Return `""` if the tap lands on whitespace/punctuation.
  - [ ] 2.5 Register a `click` listener on `document` inside the mobile branch. Guard with `if (event.target.closest('#findContainer, a, button, #menuButton, .menu-dropdown, .variant-wrap, [contenteditable], input, textarea')) return;` — this preserves existing tap handlers (variant toggle at line 428, hamburger menu, find bar).
  - [ ] 2.6 In the listener: call `caret_node_offset_from_point(event.clientX, event.clientY)`, then `word_at_offset(...)`. If the resulting word is non-empty after trimming, call `summary_selection(word)` (the generalised version from Task 1).
  - [ ] 2.7 Add a `function highlight_tapped_word(text_node, start, end)` that wraps the matched substring in a `<span class="tapped-word">` using `Range.surroundContents()`, then schedules an unwrap after 250 ms (replace the span with its text content and call `parent.normalize()`). Invoke it from the listener on successful word extraction.
  - [ ] 2.8 Wrap the listener body in a `try/catch` that logs via the existing `log_error()` helper (line 23) so a WebView quirk cannot silently break lookups.
  - [ ] 2.9 Re-read the whole `DOMContentLoaded` block once at the end and confirm no dangling references to the removed `previous_selection_text` / `word_summary_was_closed` / `window.word_summary_closed` remain anywhere in the file.

- [ ] 3.0 Suppress the native selection/callout UI and add the tapped-word highlight via mobile-scoped SCSS
  - [ ] 3.1 In `assets/sass/suttas.sass`, add a rule block scoped to `body.mobile #ssp_content` setting: `user-select: none`, `-webkit-user-select: none`, `-webkit-touch-callout: none`, `touch-action: manipulation`.
  - [ ] 3.2 Within the same scope, add an override for `#findContainer, #findContainer input` restoring `user-select: text` and `-webkit-user-select: text` so the find bar input still accepts typing and caret positioning.
  - [ ] 3.3 Add a `.tapped-word` style: subtle yellow background (`background-color: rgba(255, 230, 130, 0.5)`), no border, no layout shift. Add a `body.dark .tapped-word` variant with a darker, readable background (e.g. `rgba(255, 210, 90, 0.35)`).
  - [ ] 3.4 Verify no global `user-select` rule in `_base.sass` or `suttas.sass` conflicts with the mobile overrides; if one exists, scope the conflicting rule to `body:not(.mobile)` or adjust specificity rather than duplicating.
  - [ ] 3.5 Run `make sass` and confirm `assets/css/suttas.css` regenerates without warnings.

- [ ] 4.0 Verify end-to-end and update `PROJECT_MAP.md`
  - [ ] 4.1 `make build -B`, run the desktop app, open any Pāli sutta (e.g. SN 56.11). Confirm: `dblclick` on a word still opens `WordSummary`; drag-selection still works on desktop (the `body.mobile` class is not set, so the `user-select: none` rule does not apply).
  - [ ] 4.2 Build the Android APK via Qt Creator and install on a device. Open SN 56.11 Pāli. Confirm each acceptance item from the PRD's Verification section 2: single-tap lookup on `Ekaṁ`, `bhagavā`, `bārāṇasiyaṁ`, `pañcavaggiye`; no selection handles on long-press; variant mark (⧫) toggle still works; find bar input still accepts text; hamburger menu still opens.
  - [ ] 4.3 On the same Android build: open a DB-stored HTML sutta (non-bilara path) and an English translation (e.g. DN 22 Sujato). Confirm tap-to-lookup works for both.
  - [ ] 4.4 If an iOS device is available, repeat 4.2 and specifically confirm the long-press callout (copy/look up/share) does not appear on sutta content.
  - [ ] 4.5 `cd backend && cargo test` — confirm no regressions (should be untouched by this feature).
  - [ ] 4.6 Update `PROJECT_MAP.md`: in the section describing `assets/js/suttas.js` and mobile interactions, replace the `selectionchange` description with the new single-tap caret-at-point flow. Note the `body.mobile` class convention and the mobile-scoped `user-select: none` rule in `assets/sass/suttas.sass`.
  - [ ] 4.7 If issues from PRD Open Question 2 appear (Android ActionMode toolbar still showing on some device/WebView versions), document the limitation in `PROJECT_MAP.md` or the relevant `docs/` file and leave a short TODO comment near the CSS rule in `suttas.sass`. Do not add a native Qt/Java workaround in this iteration.
