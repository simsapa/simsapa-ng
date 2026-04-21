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
