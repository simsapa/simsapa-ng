# PRD: Mobile Single-Tap Dictionary Word Lookup

## Introduction/Overview

On desktop, a sutta page triggers dictionary lookup via `dblclick` — the browser selects the word under the cursor and `summary_selection()` reads `document.getSelection()` and posts it to `/summary_query/...`.

On mobile (QtWebView — Android WebView on Android, WKWebView on iOS) there is no `dblclick`. The current mobile flow listens to `selectionchange`, requiring the user to long-press to create a native selection before lookup fires (`assets/js/suttas.js:377–414`). The OS selection UI (selection handles, copy/paste/share bar) then appears over the in-app `WordSummary` panel, obscuring or crowding it, and selection drag re-triggers lookup repeatedly — mitigated today by a boundary-drag detection workaround (`suttas.js:398–407`).

We want mobile lookup to feel like tapping a link: **a single tap on a Pāli word opens `WordSummary` with that word as the query**, with no native selection UI and no long-press required. The desktop `dblclick` path stays unchanged.

## Goals

- Single tap on a word in the sutta content triggers dictionary lookup on mobile.
- The native OS text-selection UI (selection handles, callout menu) does not appear on the sutta content area.
- Works for both rendered bilara-text HTML (JSON + template path) and suttas stored directly as HTML in the database.
- Works identically in the English translation pane (tap any word) — not the primary use case, but should not regress.
- Desktop `dblclick` behaviour is unchanged.

## User Stories

1. **As a mobile user reading a Pāli sutta**, I want to tap a word once and see its dictionary entry, so I can look up unfamiliar words without fighting the OS selection UI.
2. **As a mobile user**, I want no copy/paste/share bar to appear when I tap words, so the `WordSummary` panel is not obscured.
3. **As a mobile user**, I want taps on interactive elements (variant toggles, links, find bar, menu) to behave as they always did, so only body text triggers lookup.

## Functional Requirements

### Event handling (mobile only, `IS_MOBILE === true`)

1. Remove the `selectionchange` handler and associated boundary-drag state (`suttas.js:361–414`) on mobile.
2. Register a `click` listener on the sutta content root. The listener receives `event.clientX` / `event.clientY`.
3. Convert the tap coordinates to a text-node + offset:
   - Prefer `document.caretPositionFromPoint(x, y)` (standard; Baseline 2025, supported on current Android WebView and WKWebView).
   - Fall back to `document.caretRangeFromPoint(x, y)` (non-standard WebKit/Chromium API; long-standing support on both backends) if the standard method is unavailable.
   - If neither returns a usable text node, do nothing.
4. From the text node's `nodeValue` and the returned offset, extract the word under the tap: walk left and right until a character outside the Pāli word class is hit. Word class: Unicode letters + combining marks (`\p{L}\p{M}`), plus `'` and Pāli-specific diacritic-carrying codepoints. Regex `/[\p{L}\p{M}'’]+/gu` applied around the offset (or `Intl.Segmenter` with `granularity: 'word'` as an implementation option — both are supported on current WebViews).
5. If a non-empty word is found, call a generalised `summary_selection(word)` which POSTs it to `/summary_query/<window_id>/<word>` (reuse the existing endpoint).

### Suppressing native selection UI

6. Apply CSS to the sutta content container (scoped, not the whole document) on mobile:
   ```css
   user-select: none;
   -webkit-user-select: none;
   -webkit-touch-callout: none;
   touch-action: manipulation;
   ```
   `-webkit-touch-callout: none` disables the iOS long-press callout. `user-select: none` suppresses selection on both backends. `touch-action: manipulation` removes the default long-press context menu on Android and the 300 ms click delay.
7. On Android specifically, verify with the QtWebView build on a device whether a residual `ActionMode` (selection toolbar) can still appear on a prolonged press. If so, the existing `user-select: none` on the container typically prevents it; if not, document the known limitation — do not add a native Qt/Java workaround in this iteration.
8. The find bar input (`#findContainer`) must keep `user-select: text` so the user can select the query text they are typing.

### Guards and target filtering

9. Skip lookup if `event.target.closest('#findContainer, a, button, .hamburger-menu, .variant-wrap, [contenteditable]')` matches — these already handle their own taps.
10. Skip lookup if the extracted word is empty or is a single punctuation/whitespace character.
11. Skip lookup if the tap lands inside `<input>` or `<textarea>`.

### Visual feedback

12. On successful word extraction, transiently highlight the tapped word for ~250 ms (e.g. wrap the matched range in a `<span class="tapped-word">`, then unwrap) to confirm the hit.
13. Style `.tapped-word` with a subtle background (e.g. `rgba(255, 230, 130, 0.5)` in light mode, a darker analogue in dark mode). Rules live in `assets/sass/` and get compiled.

### `summary_selection` generalisation

14. Extend `summary_selection(explicit_text = null)` (`suttas.js:48`) to accept an optional string. If provided, use it; otherwise fall back to today's `selection_text()` path. Desktop keeps calling it with no argument.

## Non-Goals (Out of Scope)

- **Multi-word lookup on mobile** via drag-selection. Single-tap only. If users want phrase lookup later, that's a follow-up PRD.
- **Per-word DOM wrapping at render time.** This PRD is JS-only; it does not modify the Rust template or the rendering pipeline.
- **Changing the desktop `dblclick` path.**
- **Changing the `/summary_query` or `/lookup_window_query` backend endpoints.**
- **Triple-tap or other gestures.**

## Design Considerations

### Why caret-at-point and not span-wrapping

Rendered sutta HTML (`backend/tests/data/sn56.11_pli_ms_rendered.html`) stores whole sentences as plain text inside `<span class="text" lang="la">`. Individual words are not wrapped. Per-word wrapping would require changing the Rust template, inflate DOM size for long suttas, and still need a separate path for DB-stored HTML. Caret-at-point is JS-only, works uniformly for both rendering paths, and adds zero markup.

### Platform verification done

- **`caretPositionFromPoint`**: Baseline 2025 (newly interoperable Dec 2025) — supported on Chrome, Safari, iOS Safari/WebView, Android WebView, Firefox. Source: MDN.
- **`caretRangeFromPoint`**: non-standard but long-supported on WebKit and Blink — safe fallback for older WebView builds.
- **`-webkit-touch-callout: none` + `user-select: none`**: standard technique on iOS WKWebView for disabling long-press callout; confirmed on community and Apple-developer sources.
- **QtWebView `runJavaScript`**: not needed for this feature; the JS code already fetches the API directly via `fetch(API_URL/...)`, same as today.
- **`Intl.Segmenter`**: widely available since 2023 on Android WebView and WKWebView — acceptable as an implementation detail inside step 4 if the regex proves insufficient.

### Edge cases

- Tap between two words (whitespace or punctuation): no word extracted → no lookup. Acceptable.
- Tap on a variant mark (`.variant-wrap .mark`): existing click handler toggles the variant (`suttas.js:428–430`); the new lookup handler must check `event.target.closest('.variant-wrap')` and bail.
- Tap on soft hyphens or zero-width joiners inside Pāli compounds: regex should treat these as part of the word (`\p{M}` covers combining marks).
- Very long compound words (e.g. `Dhammacakkappavattanasutta`): lookup is expected to work on the whole compound as a single word — backend decides how to segment it.

## Technical Considerations

### Files to modify

- `assets/js/suttas.js` — main change. Replace `IS_MOBILE` branch at lines 358–414 with tap handler. Generalise `summary_selection()` at line 48.
- `assets/sass/*.scss` — add mobile-scoped `user-select: none` / `-webkit-touch-callout: none` / `touch-action: manipulation` rules on the sutta content container, plus `.tapped-word` highlight style. Rebuild with `make sass`.
- No Rust, QML, template, or C++ changes.

### Existing functions to reuse

- `summary_selection()` — `assets/js/suttas.js:48` (generalise, don't duplicate).
- `selection_text()` — `assets/js/suttas.js:28` (kept as-is; used by the non-argument `summary_selection()` path on desktop).
- `API_URL`, `WINDOW_ID`, `IS_MOBILE` — injected by `backend/.../html_content.rs:121`, already available in scope.
- `document.SSP` object — used throughout `suttas.js` for find/scroll; not involved here but confirms JS is the right layer.

### QML side

No change. `assets/qml/SuttaHtmlView_Mobile.qml` uses `QtWebView` → native WebView; all behaviour is driven from inside the loaded page. `SuttaHtmlView_Desktop.qml` uses `QtWebEngine`; unchanged.

## Success Metrics

- Tapping any Pāli word in SN 56.11, DN 22, MN 1 on Android opens `WordSummary` with that exact word.
- No OS selection handles or context menu appear on sutta content during normal tap/hold interaction.
- Variant-mark toggle, find bar, hamburger menu, in-text links keep working on mobile.
- Desktop `dblclick` path behaves identically to current `main` branch.

## Open Questions

1. Should the tapped-word highlight persist while `WordSummary` is open (as a reading anchor), or clear after ~250 ms? Default proposed: clear after 250 ms; revisit if users want persistence.
2. On Android, if a residual ActionMode toolbar still appears on some device/WebView versions despite the CSS, is that acceptable as a known limitation, or should we defer release until a native-side workaround is added? Default proposed: ship with CSS-only suppression and document the edge case.

## Verification

1. `make sass && make build -B`, run on desktop — confirm:
   - `dblclick` still opens `WordSummary`.
   - Normal selection (drag) still works on desktop (`user-select` rules must be mobile-scoped).
2. Build and install the Android APK via Qt Creator. Load SN 56.11 Pāli:
   - Tap `Ekaṁ`, `bhagavā`, `bārāṇasiyaṁ`, `pañcavaggiye` — `WordSummary` opens with the exact word each time, including diacritics.
   - Long-press on body text: no selection handles, no copy/share bar.
   - Tap the variant mark (⧫) — toggles variant text, does not open `WordSummary`.
   - Type in the find bar — text input and selection inside the input still work.
   - Tap hamburger menu, menu items — still work.
3. Load a DB-stored HTML sutta (non-bilara path) on Android — confirm tap lookup works there too.
4. Load an English translation (e.g. DN 22 Sujato) — confirm tap on an English word opens `WordSummary`.
5. iOS device test: same checklist. Confirm `-webkit-touch-callout: none` suppresses the long-press callout on WKWebView.
6. `cd backend && cargo test` — no backend changes, tests should pass unchanged.

## Post-Implementation Review: Issues and Planned Fixes

After the initial implementation shipped and was confirmed working on device, a review surfaced the following issues. Fixes are planned below and tracked in the task list as Task 5.0.

### Issue 1 — `touch-action: manipulation` disables pinch-zoom on the reading area

The mobile-scoped rule `touch-action: manipulation` on `body.mobile #ssp_content` was added to remove the iOS 300 ms click delay. As a side effect it also disables pinch-zoom on the main reading area. Since the in-app text resize controller only steps through preset sizes, losing pinch-zoom is an accessibility regression.

**Fix:** Remove `touch-action: manipulation`. The iOS click delay is negligible on modern WebViews and we do not need to suppress other native gestures.

### Issue 2 — Hyphenated compounds and loss of long-press → selection-driven lookup

The word regex `/[\p{L}\p{M}'’]+/gu` excludes `-`, so a tap on a hyphenated Pāli compound (e.g. *anatta-lakkhaṇa*) looks up only one half. More importantly, the original PRD removed the `selectionchange`-based flow entirely, but some users rely on long-press to select a word, then drag the selection handles to extend the selection — the dictionary lookup should follow the selection live (as it did before this PRD).

**Fix:** Restore the `selectionchange` handler **alongside** the single-tap handler. Long-press creates a native selection and triggers `summary_selection()` from the selected text; dragging selection handles extends the selection and updates the lookup. The previous boundary-drag / `word_summary_was_closed` workaround was only needed because a stray selection event could re-open a closed `WordSummary`. With the single-tap path doing the common case, we can keep the `selectionchange` path simple: trigger `summary_selection()` whenever a non-empty selection exists inside the sutta content. Single-tap continues to handle the zero-selection case and the highlight feedback.

### Issue 3 — `parent.normalize()` in the highlight unwrap may invalidate cached text-node references

After the 700 ms highlight timeout, `parent.normalize()` merges adjacent text nodes inside `#ssp_content`. Any other code that caches text-node references (e.g. find-bar match ranges, footnote/comment toggle bindings) could be silently pointing at stale nodes after a tap.

**Fix:** Audit the other JS modules that touch `#ssp_content` (find bar, footnote toggle, comment toggle, chapter navigation) to confirm none cache text-node references across tap events. If any do, either drop the `normalize()` call (the unwrap still works without it — just leaves an extra adjacent text node, which is harmless) or reset the affected caches after normalize.

### Issue 4 — Rapid repeated taps can nest highlight spans

A second tap within the 700 ms highlight window creates a nested `<span class="tapped-word">` inside the first. No functional bug — the nested spans unwind in order — but it's a visible cosmetic artefact on fast repeated taps.

**Fix:** Track the timestamp of the last tap and ignore subsequent taps within a small debounce window (e.g. 250 ms). This is simpler than reference-counting nested spans.

### Issue 5 — Synthetic clicks with `(clientX, clientY) === (0, 0)` can trigger spurious lookups

Synthetic clicks from keyboard activation or accessibility tooling have `event.detail === 0` and `clientX/Y === 0`. They currently resolve to whatever text is at the viewport's top-left, triggering an unintended lookup.

**Fix:** Guard the click handler with `if (event.detail === 0) return;` — synthetic clicks always have `detail === 0`, real taps always have `detail >= 1`.

## Second Post-Implementation Review: Performance and Robustness

After the second batch of fixes shipped (pinch-zoom restored, `selectionchange` lookup re-added, selection auto-clear with `.tapped-word` fallback, persistent highlight, multi-element highlight painting), a further review surfaced performance and robustness issues. Fixes are planned below and tracked in the task list as Task 6.0.

### Issue 6 — `collect_range_text_parts` runs on every `selectionchange`

`selectionchange` fires tens of times per second while the user drags selection handles. Each fire walks all text nodes under the range's `commonAncestorContainer` (possibly the whole `#ssp_content` on a long drag), creating a throwaway `Range` per text node for `compareBoundaryPoints`. On a long sutta this is the dominant cost of a drag.

**Fix:** Defer part collection to the 3-second debounce timer. The `selectionchange` handler does only the O(1) work: capture `last_mobile_selection_text`, call the throttled live lookup, and restart the timer. When the timer fires, it re-reads `document.getSelection().getRangeAt(0)` and calls `collect_range_text_parts` exactly once, then clears the selection and paints the highlight. This also fixes Issue 12 (multi-paragraph paint jank) as a side effect — one computation and one batched DOM write per finished selection.

### Issue 7 — `summary_selection()` fires on every `selectionchange`

Each selectionchange produces a `fetch('/summary_query/...')`. During a long drag this can be dozens of fetches per second.

**Fix:** Throttle the live lookup to ~30 ms so the UI still feels live-follow to the user but the fetch cadence caps at ~33/s. A simple timestamp-based throttle is enough: record `last_live_lookup_ms`, skip the call if `(now - last_live_lookup_ms) < 30`. We keep 30 ms (not a larger value) because the user explicitly wants the `WordSummary` query to track the selection while dragging — a noticeable lag would defeat the purpose.

### Issue 8 — Whitespace-only text nodes between block elements are wrapped

A drag that crosses `<p>` boundaries traverses the whitespace text nodes the browser inserts between blocks. Each becomes a `<span class="tapped-word">` containing just `"\n  "`; with a background colour, that renders as a thin coloured gap.

**Fix:** In `collect_range_text_parts`, filter `(node.nodeValue || "").trim() === ""` before pushing. Lookup unaffected (the text was already empty after trim). Highlight becomes visually clean.

### Issue 9 — `selectionchange` handler has no `try/catch`

If `compareBoundaryPoints` or a Range API throws (for example on a disconnected or mid-mutation range — the page-load code replaces `#ssp_content` innerHTML when navigating suttas), the error propagates and breaks subsequent `selectionchange` handling until the user reloads.

**Fix:** Wrap the handler body in `try/catch` with `log_error(...)`, matching the click handler's defensive pattern.

### Issue 10 — Stale text-node references in `pending_range_parts` across sutta navigation

If the user long-presses, starts the 3-second timer, then navigates to a different sutta within that window, the stored `text_node` references point at nodes that were detached when `#ssp_content` was replaced. The timer still runs and calls `surroundContents` on detached nodes — currently a silent no-op (Range APIs don't throw on detached nodes), so no crash, but the wasted work and the dangling closure retaining the old subtree until the timer fires are untidy.

**Fix:** In the timer callback, check `text_node.isConnected` before including each part. Parts with detached nodes are skipped. Live selection is still cleared (`removeAllRanges()` is harmless on an empty selection). This also keeps the implementation robust against any other DOM-replacing path we add later without needing a navigation hook.

### Non-issues (reviewed, no change needed)

- **Single-tap active-selection guard swallowing taps during the 3-s window** — tested on device: tapping outside the selection clears the native selection, selectionchange fires with empty text, the tap on the new word highlights immediately. Works as intended.
- **`last_mobile_selection_text` persistence across navigation** — acceptable edge case.
- **`caretRangeFromPoint` edge-pixel miss** — negligible in practice.
- **Unwrap safety on subtree detachment** — current code silently skips, which is correct.
