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

- [x] 1.0 Generalise `summary_selection()` to accept an explicit word string
  - [x] 1.1 Change the signature at `assets/js/suttas.js:48` to `function summary_selection(explicit_text)` with `explicit_text` defaulting to `null`/`undefined`.
  - [x] 1.2 Use `explicit_text` when it is a non-empty string; otherwise fall back to `selection_text()` as today. Trim the explicit value before the empty-check.
  - [x] 1.3 Keep the existing `fetch(\`${API_URL}/summary_query/${WINDOW_ID}/${encodeURIComponent(text)}\`)` call; pass the resolved text through `encodeURIComponent`.
  - [x] 1.4 Confirm the desktop `dblclick` call site at line 424 still works with no argument (no change needed there).
  - [x] 1.5 Sanity check: grep the repo for other callers of `summary_selection(` to make sure none pass unexpected arguments — update them if they do.

- [x] 2.0 Replace the mobile `selectionchange` handler with a single-tap caret-at-point word-lookup handler
  - [x] 2.1 Inside the `DOMContentLoaded` callback at `assets/js/suttas.js:353`, locate the `if (IS_MOBILE) { ... } else { ... }` block (lines 358–426). Remove the entire mobile branch body — `previous_selection_text`, `word_summary_was_closed`, `window.word_summary_closed`, and the `selectionchange` listener (lines 361–414). Leave the desktop branch untouched.
  - [x] 2.2 In the new mobile branch, add `document.body.classList.add('mobile')` so SCSS can target mobile-only rules (used in Task 3).
  - [x] 2.3 Add a helper `function caret_node_offset_from_point(x, y)` that calls `document.caretPositionFromPoint(x, y)` when available and falls back to `document.caretRangeFromPoint(x, y)`. Return `{ node, offset }` or `null` if the resolved node is not a text node or the result is null/undefined.
  - [x] 2.4 Add a helper `function word_at_offset(text_node, offset)` that returns the contiguous Pāli/English word containing `offset` in `text_node.nodeValue`. Use the Unicode-aware regex `/[\p{L}\p{M}'’]+/gu` applied to the node value; pick the match whose start ≤ offset ≤ end. Return `""` if the tap lands on whitespace/punctuation.
  - [x] 2.5 Register a `click` listener on `document` inside the mobile branch. Guard with `if (event.target.closest('#findContainer, a, button, #menuButton, .menu-dropdown, .variant-wrap, [contenteditable], input, textarea')) return;` — this preserves existing tap handlers (variant toggle at line 428, hamburger menu, find bar).
  - [x] 2.6 In the listener: call `caret_node_offset_from_point(event.clientX, event.clientY)`, then `word_at_offset(...)`. If the resulting word is non-empty after trimming, call `summary_selection(word)` (the generalised version from Task 1).
  - [x] 2.7 Add a `function highlight_tapped_word(text_node, start, end)` that wraps the matched substring in a `<span class="tapped-word">` using `Range.surroundContents()`, then schedules an unwrap after 250 ms (replace the span with its text content and call `parent.normalize()`). Invoke it from the listener on successful word extraction.
  - [x] 2.8 Wrap the listener body in a `try/catch` that logs via the existing `log_error()` helper (line 23) so a WebView quirk cannot silently break lookups.
  - [x] 2.9 Re-read the whole `DOMContentLoaded` block once at the end and confirm no dangling references to the removed `previous_selection_text` / `word_summary_was_closed` / `window.word_summary_closed` remain anywhere in the file.

- [ ] 3.0 Suppress the native selection/callout UI and add the tapped-word highlight via mobile-scoped SCSS
  - [x] 3.1 In `assets/sass/suttas.sass`, add a rule block scoped to `body.mobile #ssp_content` setting: `user-select: none`, `-webkit-user-select: none`, `-webkit-touch-callout: none`, `touch-action: manipulation`.
  - [x] 3.2 Within the same scope, add an override for `#findContainer, #findContainer input` restoring `user-select: text` and `-webkit-user-select: text` so the find bar input still accepts typing and caret positioning.
  - [x] 3.3 Add a `.tapped-word` style: subtle yellow background (`background-color: rgba(255, 230, 130, 0.5)`), no border, no layout shift. Add a `body.dark .tapped-word` variant with a darker, readable background (e.g. `rgba(255, 210, 90, 0.35)`).
  - [x] 3.4 Verify no global `user-select` rule in `_base.sass` or `suttas.sass` conflicts with the mobile overrides; if one exists, scope the conflicting rule to `body:not(.mobile)` or adjust specificity rather than duplicating.
  - [x] 3.5 Run `make sass` and confirm `assets/css/suttas.css` regenerates without warnings.

- [ ] 4.0 Verify end-to-end and update `PROJECT_MAP.md`
  - [ ] 4.1 `make build -B`, run the desktop app, open any Pāli sutta (e.g. SN 56.11). Confirm: `dblclick` on a word still opens `WordSummary`; drag-selection still works on desktop (the `body.mobile` class is not set, so the `user-select: none` rule does not apply).
  - [ ] 4.2 Build the Android APK via Qt Creator and install on a device. Open SN 56.11 Pāli. Confirm each acceptance item from the PRD's Verification section 2: single-tap lookup on `Ekaṁ`, `bhagavā`, `bārāṇasiyaṁ`, `pañcavaggiye`; no selection handles on long-press; variant mark (⧫) toggle still works; find bar input still accepts text; hamburger menu still opens.
  - [ ] 4.3 On the same Android build: open a DB-stored HTML sutta (non-bilara path) and an English translation (e.g. DN 22 Sujato). Confirm tap-to-lookup works for both.
  - [ ] 4.4 If an iOS device is available, repeat 4.2 and specifically confirm the long-press callout (copy/look up/share) does not appear on sutta content.
  - [ ] 4.5 `cd backend && cargo test` — confirm no regressions (should be untouched by this feature).
  - [ ] 4.6 Update `PROJECT_MAP.md`: in the section describing `assets/js/suttas.js` and mobile interactions, replace the `selectionchange` description with the new single-tap caret-at-point flow. Note the `body.mobile` class convention and the mobile-scoped `user-select: none` rule in `assets/sass/suttas.sass`.
  - [ ] 4.7 If issues from PRD Open Question 2 appear (Android ActionMode toolbar still showing on some device/WebView versions), document the limitation in `PROJECT_MAP.md` or the relevant `docs/` file and leave a short TODO comment near the CSS rule in `suttas.sass`. Do not add a native Qt/Java workaround in this iteration.

- [ ] 5.0 Post-implementation review fixes (see PRD "Post-Implementation Review" section)
  - [x] 5.1 Restore pinch-zoom on the reading area: in `assets/sass/suttas.sass`, remove `touch-action: manipulation` from the `body.mobile #ssp_content` block. Keep `-webkit-touch-callout: none` (still suppresses iOS long-press callout). Add a comment explaining that pinch-zoom is intentionally preserved for accessibility — the in-app text-resize controller is step-based and does not replace pinch-zoom. Run `make sass`.
  - [x] 5.2 Restore `selectionchange`-driven lookup alongside the single-tap handler. In the `IS_MOBILE` branch of `assets/js/suttas.js`:
    - Re-add a `document.addEventListener("selectionchange", ...)` handler that calls `summary_selection()` (no argument — reads from `selection_text()`) whenever the current selection is non-empty and not inside `#findContainer` / inputs.
    - Do **not** reintroduce the `previous_selection_text` / `word_summary_was_closed` / boundary-drag state — with single-tap covering the zero-selection case, the simple "selection changed and is non-empty → lookup" rule is sufficient. Drag-to-extend a selection naturally triggers repeated lookups with updated text.
    - Add a comment above the handler explaining the split of responsibilities: single-tap handles the caret-at-point case; `selectionchange` handles the long-press + drag-to-extend case (needed because the single-tap regex excludes `-`, so hyphenated compounds can only be looked up via selection).
  - [x] 5.3 Audit `assets/js/` for text-node reference caches inside `#ssp_content`. Grep for `TextNode`, `nodeValue`, `createTreeWalker`, `Range`, and uses of `document.getSelection()`-derived ranges held across events. Files to check: `suttas.js`, plus anything loaded by it (find bar logic, footnote/comment toggles, chapter navigation). If any cache such references, remove `parent.normalize()` from `highlight_tapped_word()` — the unwrap is already correct without it (leaves adjacent text nodes unmerged, which is harmless). Add a comment at the unwrap site explaining whichever choice was made and why.
  - [x] 5.4 Debounce rapid taps in the `click` handler. Add a module-scoped (inside the `IS_MOBILE` branch) `let last_tap_ms = 0;`. At the top of the handler, read `const now = performance.now();` and `if (now - last_tap_ms < 250) return;`, then set `last_tap_ms = now;` after the guards pass. Add a brief comment that this prevents nested `.tapped-word` spans when the user taps faster than the 700 ms highlight timeout.
  - [x] 5.5 Guard synthetic clicks: at the top of the click handler, add `if (event.detail === 0) return;` with a comment explaining that synthetic clicks (keyboard activation, accessibility tooling) have `detail === 0` and `clientX/Y === 0`, which would otherwise resolve to the top-left of the viewport and trigger a spurious lookup.
  - [ ] 5.6 `make sass && make build -B`. On Android: confirm pinch-zoom now works on `#ssp_content`; long-press on a hyphenated compound creates a selection and `WordSummary` opens with the selected text; dragging the selection handles updates the lookup live; single-tap still works for simple words; rapid taps no longer nest highlights.

- [ ] 6.0 Second-review performance and robustness fixes (see PRD "Second Post-Implementation Review" section)
  - [x] 6.1 Defer `collect_range_text_parts` to the 3-s debounce timer. In `assets/js/suttas.js` mobile `selectionchange` handler:
    - Remove the `pending_range_parts = collect_range_text_parts(range);` call from the per-event path. The handler should only update `last_mobile_selection_text`, run the throttled live lookup, and (re)start the 3-s timer.
    - Drop the `pending_range_parts` module variable entirely — no longer needed.
    - In the timer callback: read `document.getSelection()` directly, if it has a non-empty range call `collect_range_text_parts(range)` once, then `removeAllRanges()`, then `highlight_tapped_parts(parts)`. If the live selection is already empty at fire time (user dismissed selection mid-timer), just bail.
    - Add a comment explaining that computing parts once per finished selection (instead of once per selectionchange) is the hot-path optimisation and that it also batches DOM writes into a single reflow.
  - [x] 6.2 Throttle the live `summary_selection()` call during drag. Add `let last_live_lookup_ms = 0;` in the mobile branch. In the `selectionchange` handler, before calling `summary_selection()`:
    - `const now = (typeof performance !== "undefined" && performance.now) ? performance.now() : Date.now();`
    - `if (now - last_live_lookup_ms < 30) return_from_lookup; else { last_live_lookup_ms = now; summary_selection(); }`
    - Note the structure: we still want to return from the whole handler after updating `last_mobile_selection_text` and restarting the timer — the throttle only skips the `summary_selection()` fetch, not the timer/cache bookkeeping. Implement with a local helper or an `if` that skips only the fetch line.
    - Comment: 30 ms keeps live-follow feeling instantaneous (≤1 frame at 30 fps) while capping the fetch cadence at ~33/s.
  - [x] 6.3 Filter whitespace-only text nodes in `collect_range_text_parts`. Before pushing a slice, check `(node.nodeValue || "").slice(start, end).trim() !== ""`. Skip if empty. Add a comment that this prevents narrow coloured gaps between block elements when a drag crosses paragraph boundaries.
  - [x] 6.4 Wrap the mobile `selectionchange` handler body in `try { ... } catch (e) { log_error("mobile selectionchange: " + e); }`, matching the click handler. Comment: defensive against `compareBoundaryPoints` / Range API throws on disconnected or mid-mutation ranges (e.g. when `#ssp_content` is replaced during sutta navigation).
  - [x] 6.5 In the timer callback (after deferral from 6.1), filter out parts whose `text_node.isConnected === false` before calling `highlight_tapped_parts`. If all parts are detached, skip the highlight step (still call `removeAllRanges()`). Comment: guards against stale text-node references held across sutta navigation (which replaces `#ssp_content` innerHTML and detaches the nodes we captured).
  - [ ] 6.6 `make build -B`. On Android: drag a selection across several paragraphs and confirm smoother feel (no visible lag on long suttas); confirm the final highlight paint is clean (no yellow gaps between paragraphs); confirm live-follow lookup in `WordSummary` still feels responsive; confirm no error console spam after fast sutta navigation while a selection was active.
