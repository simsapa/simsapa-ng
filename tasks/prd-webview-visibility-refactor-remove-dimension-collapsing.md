# PRD: WebView Visibility Refactor - Remove Dimension Collapsing

## Introduction/Overview

The current WebView visibility control system uses multiple layers of defense to handle mobile platform quirks, including dimension collapsing (resizing non-visible webviews to 0x0). However, this dimension collapsing causes layout jitter during transitions when webviews are resized from 0 back to full size.

This PRD describes an incremental refactoring approach to remove dimension collapsing while maintaining reliable visibility control across all scenarios (wide/narrow screens, sidebar toggles, tab switching, drawer menus).

**Problem**: Resizing webviews to 0x0 size and then expanding them back causes visible layout jitter.

**Goal**: Refactor the visibility control logic to rely on `visible`, `should_be_visible`, and `enabled` properties without dimension manipulation, while ensuring all visibility scenarios continue to work correctly.

## Goals

1. **Eliminate layout jitter** caused by dimension collapsing (0x0 → full size transitions)
2. **Maintain visibility control** across all existing scenarios without regressions
3. **Simplify the codebase** by removing one layer of complexity (dimension bindings)
4. **Preserve performance** - no degradation in webview creation or switching speed
5. **Update documentation** to reflect the simplified approach

## User Stories

1. **As a user on mobile portrait mode**, when I toggle the sidebar button, I want smooth transitions without visible layout jumping or jittering.

2. **As a user on a wide screen**, when I switch between sutta tabs, I want seamless tab changes without the webview briefly flickering or resizing.

3. **As a user on narrow screen**, when I select a search result and the sidebar auto-hides, I want the webview to appear smoothly without jumping into place.

4. **As a developer**, I want simpler visibility control logic that's easier to understand and maintain, with fewer bindings to manage.

## Functional Requirements

### Current State Analysis

The current system uses these visibility control layers:

1. **Layer 1**: Item container wrapping (in SuttaHtmlView_Mobile.qml)
2. **Layer 2**: `should_be_visible` property with explicit visibility binding
3. **Layer 3**: **Dimension collapsing** - width/height set to 0 for non-visible items
4. **Layer 4**: Drawer/dialog detection via `webview_visible` property
5. **Layer 5**: Tab-specific visibility for sidebar tabs
6. **Deferred creation**: Loader `active: root.visible && root.width > 0 && root.height > 0`

**This refactoring targets Layer 3 (dimension collapsing) for removal.**

### Refactoring Requirements

#### FR-1: Incremental Step-by-Step Approach

The refactoring MUST follow this incremental approach to validate each change:

**Step 1: Remove dimension bindings from SuttaStackLayout**
- Remove `comp.width = Qt.binding(...)` (line 64 in SuttaStackLayout.qml)
- Remove `comp.height = Qt.binding(...)` (line 65 in SuttaStackLayout.qml)
- Keep all other visibility controls unchanged:
  - `comp.should_be_visible = is_current`
  - `comp.visible = Qt.binding(() => (root.current_key === key) && root.visible)`
- Manual testing in emulator for all scenarios
- If visibility breaks, try alternative approaches before reverting

**Step 2: Remove dimension collapsing from DictionaryTab**
- Only proceed if Step 1 succeeds
- Remove `Layout.preferredWidth: rightside_tabs.currentIndex === 1 ? parent.width : 0`
- Remove `Layout.preferredHeight: rightside_tabs.currentIndex === 1 ? parent.height : 0`
- Keep visibility binding: `visible: root.webview_visible && rightside_tabs.currentIndex === 1`
- May need to add `Layout.fillWidth/fillHeight: false` when not visible
- Manual testing in emulator

**Step 3: Review Loader active conditions**
- Consider later whether `root.width > 0 && root.height > 0` is still needed
- This controls creation timing, not resizing, so may be acceptable to keep
- Defer decision until Steps 1-2 are validated

**Step 4: Update documentation and inline comments**
- Remove "Layer 3: Dimension Collapsing" from documentation
- Update inline comments in affected files
- Update docs/mobile-webview-visibility-management.md
- Update docs/mobile-webview-visibility-fix-inline-comments.md

#### FR-2: Visibility Control Logic (Post-Refactoring)

After refactoring, webview visibility MUST be controlled by:

1. **should_be_visible** - Set by SuttaStackLayout based on `current_key === key`
2. **visible binding** - Explicit binding combining `should_be_visible && loader.visible && parent.visible`
3. **enabled property** - Set to `false` when not visible to stop native rendering
4. **webview_visible** - Top-level property checking no drawer/dialogs are open
5. **Tab selection** - For sidebar tabs, only visible when that tab is current

Dimension manipulation is NOT part of the visibility control.

#### FR-3: Test Scenarios (Manual Testing)

After each refactoring step, the following scenarios MUST be tested manually in an Android emulator:

**Wide Screen Scenarios (root.is_wide = true):**
- On startup: show_sidebar_btn checked, both panels visible, one blank webview visible
- Select search result: tabs added, clicking tab loads webview, only active tab's webview visible
- Switch between tabs: only current tab's webview visible, no jitter during transitions
- Toggle sidebar: smooth transitions without layout jumping

**Narrow Screen Scenarios (mobile portrait):**
- On startup: show_sidebar_btn checked, only left panel (results) visible, no webview visible
- Select search result: show_sidebar_btn unchecks, tabs added to right panel, webview loads and displays
- Click tab: loads new webview, only active tab's webview visible
- Toggle show_sidebar_btn to checked: all webviews hidden, only results panel visible
- No layout jitter during any transitions

**Drawer/Dialog Scenarios:**
- Open mobile menu: all webviews hidden
- Close mobile menu: appropriate webviews reappear based on current state
- Open dialogs (color theme, storage, about, etc.): webviews hidden
- No blank yellow webviews appearing at any point

**Sidebar Tab Switching:**
- Switch between Results/Dictionary/Gloss/Prompts tabs
- Only the current tab's webview (if any) should be visible
- Dictionary webview only visible when Dictionary tab is current
- No webviews from other tabs appearing or flickering

#### FR-4: Success Criteria for Each Step

Each step is considered successful if:

1. **No layout jitter** - Transitions are smooth without visible resizing/jumping
2. **All visibility scenarios work** - Webviews appear/disappear correctly in all test cases
3. **No blank webviews** - No stray yellow webviews appearing inappropriately
4. **No regressions** - Existing functionality remains intact

If a step fails to meet these criteria, alternative approaches MUST be tried before reverting.

#### FR-5: Alternative Approaches (If Needed)

If removing dimension bindings causes visibility issues, try these alternatives in order:

1. **Use Layout.fillWidth/fillHeight: false** instead of dimension collapsing
2. **Investigate StackLayout's built-in behavior** - test if it works without manual manipulation
3. **Try collapsing to 1px** instead of 0px (minimal size that may avoid complete layout recalculation)
4. **Add explicit z-ordering** to ensure hidden webviews stay behind visible content
5. **Experiment with opacity** in combination with visibility (opacity: 0 when not visible)

Each alternative should be tested incrementally before moving to the next.

#### FR-6: Code Files to Modify

The following files will be modified during this refactoring:

**Step 1:**
- `assets/qml/SuttaStackLayout.qml` - Remove width/height bindings (lines 64-65)

**Step 2:**
- `assets/qml/SuttaSearchWindow.qml` - Remove Layout.preferredWidth/Height dimension collapsing from DictionaryTab

**Step 4:**
- `docs/mobile-webview-visibility-management.md` - Remove Layer 3 section, update visibility chain
- `docs/mobile-webview-visibility-fix-inline-comments.md` - Remove dimension collapsing references
- `assets/qml/SuttaStackLayout.qml` - Update inline comments

## Non-Goals (Out of Scope)

1. **Changing the deferred webview creation mechanism** - The Loader's `active: root.visible && root.width > 0 && root.height > 0` is deferred for Step 3 evaluation
2. **Refactoring other visibility layers** - Only dimension collapsing (Layer 3) is being removed
3. **Desktop webview changes** - This refactoring focuses on mobile behavior; desktop remains unchanged
4. **Adding automated tests** - Manual testing in emulator is sufficient for this refactoring
5. **Performance optimization** - The goal is to maintain current performance, not improve it
6. **Changing the multi-webview architecture** - The system continues to use multiple webview instances

## Design Considerations

### Current Implementation Details

**SuttaStackLayout.qml (lines 60-65):**
```qml
let is_current = Qt.binding(() => root.current_key === key);
comp.should_be_visible = is_current;
comp.visible = Qt.binding(() => (root.current_key === key) && root.visible);
comp.width = Qt.binding(() => (root.current_key === key) ? comp.parent.width : 0);  // REMOVE
comp.height = Qt.binding(() => (root.current_key === key) ? comp.parent.height : 0); // REMOVE
```

**After Step 1:**
```qml
let is_current = Qt.binding(() => root.current_key === key);
comp.should_be_visible = is_current;
comp.visible = Qt.binding(() => (root.current_key === key) && root.visible);
// Width/height bindings removed - let QML handle natural sizing
```

**SuttaSearchWindow.qml DictionaryTab (lines 1236-1237):**
```qml
SplitView.preferredWidth: show_sidebar_btn.checked ? (root.is_wide ? (parent.width * 0.5) : parent.width) : 0
visible: show_sidebar_btn.checked
// Also has Layout.preferredWidth/Height for tab-based dimension collapsing
```

**After Step 2:**
```qml
SplitView.preferredWidth: show_sidebar_btn.checked ? (root.is_wide ? (parent.width * 0.5) : parent.width) : 0
visible: show_sidebar_btn.checked && rightside_tabs.currentIndex === 1
// Remove Layout.preferredWidth/Height dimension collapsing, rely on visibility only
```

### Rationale for Incremental Approach

1. **Risk mitigation** - Each step is small and reversible
2. **Quick validation** - Problems are identified immediately after each change
3. **Clear causality** - If something breaks, we know exactly what caused it
4. **Alternative exploration** - If a step fails, we can try alternatives before giving up
5. **Developer confidence** - Manual testing after each step ensures nothing is silently broken

### Why Dimension Collapsing Was Originally Added

From the documentation (docs/mobile-webview-visibility-management.md Layer 3):

> Even if the native view ignores `visible: false`, it has no dimensions to render into. Prevents the WebView from occupying screen space. Provides a physical constraint that the native view must respect.

**Hypothesis**: With the other visibility layers properly implemented (Item wrapping, explicit bindings, enabled: false), dimension collapsing may no longer be necessary. This refactoring tests that hypothesis.

## Technical Considerations

### Mobile Native WebView Behavior

QtWebView on Android/iOS uses native platform views that:
- Render in a separate layer above Qt Quick content
- Don't reliably respect QML's `visible: false` on parent items
- May continue rendering even when QML wrapper is hidden

**Remaining controls after removing dimension collapsing:**
- Item container wrapping (provides QML hierarchy participation)
- Explicit `visible` binding (propagates visibility changes)
- `enabled: false` (tells native view to stop processing)
- Deferred creation via Loader (WebView doesn't exist until needed)

### Layout System Interaction

QML's layout system (StackLayout, ColumnLayout, RowLayout) may behave differently when:
- Items have `visible: false` but natural dimensions
- Items have `visible: false` and 0x0 dimensions

**Potential concern**: Does StackLayout allocate space for invisible items with non-zero dimensions?

**Testing will reveal**: Whether we need to add `Layout.fillWidth/fillHeight: false` as an alternative to dimension collapsing.

### Binding Performance

Removing two bindings per webview instance:
- Reduces binding recalculation overhead
- Simplifies the property dependency graph
- May improve transition performance (fewer properties changing simultaneously)

## Success Metrics

### Primary Metric
- **Layout jitter eliminated**: Visual inspection confirms smooth transitions without resizing jumps

### Secondary Metrics
- **Visibility correctness**: All test scenarios pass (wide/narrow screen, sidebar, tabs, drawer)
- **No visual artifacts**: No blank yellow webviews appearing inappropriately
- **Performance maintained**: Webview creation and tab switching remain responsive

### Validation Method
Manual testing in Android emulator for all scenarios listed in FR-3.

## Open Questions

1. **Loader dimension checks**: In Step 3, should we keep `root.width > 0 && root.height > 0` in the Loader's active condition? (Deferred until Steps 1-2 complete)

2. **Layout space allocation**: Will invisible webviews with natural dimensions cause StackLayout to allocate space for them? (Will be discovered in Step 1 testing)

3. **Alternative if Step 1 fails**: Which alternative approach from FR-5 should be tried first? (Will be determined based on failure mode)

4. **Desktop platform**: Do desktop webviews benefit from dimension collapsing removal, or should they remain unchanged? (Current scope is mobile-only)

5. **Documentation structure**: Should we create a new doc for the refactored approach, or update existing docs? (Decision after Step 4)

## Implementation Notes

### Files to Monitor During Testing

When testing in the emulator, pay special attention to:
- `SuttaHtmlView_Mobile.qml` - The webview wrapper
- `SuttaStackLayout.qml` - Multi-webview management
- `SuttaSearchWindow.qml` - Top-level visibility control

### Debugging Tips

If visibility issues occur during testing:
1. Check Qt's log output for webview creation/destruction messages
2. Use `Component.onCompleted` / `Component.onDestruction` logging to track lifecycle
3. Add temporary logging to visibility bindings to see when they trigger
4. Verify `enabled` property is correctly set (use qmlscene debugging)

### Rollback Strategy

Each step's changes are isolated and can be reverted independently:
- Step 1: Re-add width/height bindings to SuttaStackLayout.qml
- Step 2: Re-add Layout.preferredWidth/Height to DictionaryTab
- Documentation updates: Revert to previous version

## Acceptance Criteria

This refactoring is considered complete and successful when:

1. ✅ Width/height bindings removed from SuttaStackLayout.qml
2. ✅ Dimension collapsing removed from DictionaryTab (if Step 1 succeeds)
3. ✅ All test scenarios pass in Android emulator:
   - Wide screen startup and tab switching
   - Narrow screen startup and tab switching
   - Sidebar toggle behavior (wide and narrow)
   - Drawer/dialog opening and closing
   - Sidebar tab switching (Results/Dictionary/Gloss/Prompts)
4. ✅ No layout jitter observed during any transitions
5. ✅ No blank yellow webviews appearing inappropriately
6. ✅ Performance remains responsive (subjective assessment)
7. ✅ Documentation updated to reflect new approach
8. ✅ Inline comments updated in modified files

## References

- Current implementation: `assets/qml/SuttaStackLayout.qml` lines 60-65
- Current implementation: `assets/qml/SuttaSearchWindow.qml` lines 1236-1237
- Mobile visibility documentation: `docs/mobile-webview-visibility-management.md`
- Inline comments documentation: `docs/mobile-webview-visibility-fix-inline-comments.md`
- Deferred webview creation: `assets/qml/SuttaHtmlView_Mobile.qml` lines 8-34
