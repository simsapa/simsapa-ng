# Tasks: WebView Visibility Refactor - Remove Dimension Collapsing

## Relevant Files

- `assets/qml/SuttaStackLayout.qml` - Contains width/height dimension bindings that need removal (lines 64-65)
- `assets/qml/SuttaSearchWindow.qml` - Contains DictionaryTab dimension collapsing via Layout.preferredWidth/Height (around line 1317)
- `assets/qml/SuttaHtmlView_Mobile.qml` - WebView wrapper component with Loader conditions that may need review
- `docs/mobile-webview-visibility-management.md` - Documentation describing Layer 3 (dimension collapsing) that needs updating
- `docs/mobile-webview-visibility-fix-inline-comments.md` - Inline comment documentation that references dimension collapsing

### Notes

- Manual testing must be performed in Android emulator after each step
- Each step must validate all test scenarios (wide/narrow screens, sidebar toggles, drawer menus, tab switching)
- If a step fails, try alternative approaches (Layout.fillWidth/fillHeight: false, z-ordering, opacity, 1px instead of 0px) before reverting
- The refactoring is incremental - each step must be validated before proceeding to the next
- Build command: `make build -B`
- Run command: `make run` (for manual testing only - avoid automated GUI testing)

## Tasks

- [ ] 1.0 Remove dimension bindings from SuttaStackLayout (Step 1)
  - [ ] 1.1 Read and understand current implementation in `assets/qml/SuttaStackLayout.qml` (lines 60-65)
  - [ ] 1.2 Remove the width binding: `comp.width = Qt.binding(() => (root.current_key === key) ? comp.parent.width : 0);` (line 64)
  - [ ] 1.3 Remove the height binding: `comp.height = Qt.binding(() => (root.current_key === key) ? comp.parent.height : 0);` (line 65)
  - [ ] 1.4 Keep the visibility bindings unchanged: `comp.should_be_visible` and `comp.visible` bindings must remain
  - [ ] 1.5 Add inline comment explaining that width/height bindings were removed to prevent layout jitter
  - [ ] 1.6 Build the application using `make build -B` to verify compilation succeeds
  - [ ] 1.7 Perform manual testing in Android emulator - Wide screen startup scenario
  - [ ] 1.8 Perform manual testing in Android emulator - Wide screen tab switching scenario
  - [ ] 1.9 Perform manual testing in Android emulator - Narrow screen (portrait) startup scenario
  - [ ] 1.10 Perform manual testing in Android emulator - Narrow screen select result and tab switching
  - [ ] 1.11 Perform manual testing in Android emulator - Sidebar toggle on wide and narrow screens
  - [ ] 1.12 Perform manual testing in Android emulator - Drawer/dialog open and close scenarios
  - [ ] 1.13 Verify no layout jitter during transitions and no blank yellow webviews appear
  - [ ] 1.14 If visibility issues occur, try alternative approach: Add `Layout.fillWidth: false` and `Layout.fillHeight: false` when not visible
  - [ ] 1.15 If still failing, document the failure mode and consult FR-5 alternative approaches before reverting

- [ ] 2.0 Remove dimension collapsing from DictionaryTab (Step 2)
  - [ ] 2.1 Only proceed if Step 1.0 succeeded and all tests passed
  - [ ] 2.2 Locate the DictionaryTab in `assets/qml/SuttaSearchWindow.qml` (around line 1317)
  - [ ] 2.3 Identify current dimension collapsing implementation: `Layout.preferredWidth: rightside_tabs.currentIndex === 1 ? parent.width : 0`
  - [ ] 2.4 Identify current height collapsing: `Layout.preferredHeight: rightside_tabs.currentIndex === 1 ? parent.height : 0`
  - [ ] 2.5 Remove the `Layout.preferredWidth` conditional dimension binding
  - [ ] 2.6 Remove the `Layout.preferredHeight` conditional dimension binding
  - [ ] 2.7 Verify the visibility binding remains: `visible: root.webview_visible && rightside_tabs.currentIndex === 1`
  - [ ] 2.8 Consider adding `Layout.fillWidth: false` and `Layout.fillHeight: false` when tab is not current if needed
  - [ ] 2.9 Build the application using `make build -B`
  - [ ] 2.10 Perform manual testing - Switch between all sidebar tabs (Results/Dictionary/Gloss/Prompts)
  - [ ] 2.11 Verify only the current tab's webview is visible
  - [ ] 2.12 Verify dictionary webview appears/disappears correctly when switching to/from Dictionary tab
  - [ ] 2.13 Test with drawer menu open/close while on Dictionary tab
  - [ ] 2.14 Verify no blank webviews appear during tab switching
  - [ ] 2.15 If visibility issues occur, try alternative approach with `Layout.fillWidth/fillHeight: false` binding
  - [ ] 2.16 Document any issues and try alternative approaches before reverting

- [ ] 3.0 Review and evaluate Loader active conditions (Step 3)
  - [ ] 3.1 Only proceed if Step 2.0 succeeded and all tests passed
  - [ ] 3.2 Open `assets/qml/SuttaHtmlView_Mobile.qml` and locate the Loader (around line 146)
  - [ ] 3.3 Review current condition: `active: root.visible && root.width > 0 && root.height > 0`
  - [ ] 3.4 Analyze the purpose: This controls webview creation timing, not visibility control
  - [ ] 3.5 Evaluate whether `root.width > 0 && root.height > 0` checks are still needed for deferred creation
  - [ ] 3.6 Consider that this is about creation timing (preventing premature instantiation), not resizing
  - [ ] 3.7 Test whether removing dimension checks from Loader active condition causes issues
  - [ ] 3.8 If no issues found, document that dimension checks in Loader are acceptable (different purpose than dimension collapsing)
  - [ ] 3.9 If issues found with dimension checks, evaluate whether they should be removed or kept
  - [ ] 3.10 Make a decision: keep as-is (acceptable for creation timing) or modify if causing problems
  - [ ] 3.11 Document the decision and rationale in inline comments

- [ ] 4.0 Update documentation and inline comments (Step 4)
  - [ ] 4.1 Open `docs/mobile-webview-visibility-management.md`
  - [ ] 4.2 Locate "Layer 3: Dimension Collapsing" section (lines 91-103)
  - [ ] 4.3 Remove the entire "Layer 3: Dimension Collapsing" section
  - [ ] 4.4 Update the section numbering for Layer 4 → Layer 3 and Layer 5 → Layer 4
  - [ ] 4.5 Update "The Complete Visibility Chain" section to remove dimension-related condition (line 143)
  - [ ] 4.6 Update the condition list to remove "Non-zero dimensions: Width and height must be greater than 0"
  - [ ] 4.7 Open `docs/mobile-webview-visibility-fix-inline-comments.md`
  - [ ] 4.8 Locate references to dimension collapsing in SuttaStackLayout section (lines 115-119)
  - [ ] 4.9 Remove "Layer 2: Dimension collapsing" comment block
  - [ ] 4.10 Update DictionaryTab section (lines 149-170) to reflect removal of dimension collapsing
  - [ ] 4.11 Update "The Complete Visibility Chain" in this file to remove dimension condition (line 181)
  - [ ] 4.12 Open `assets/qml/SuttaStackLayout.qml` and review inline comments around lines 60-65
  - [ ] 4.13 Update inline comments to explain that dimension collapsing was removed to prevent layout jitter
  - [ ] 4.14 Add comment explaining that visibility control now relies on visible/enabled properties only
  - [ ] 4.15 Review any other files with comments referencing "dimension collapsing" or "width/height to 0"
  - [ ] 4.16 Update all relevant inline comments to reflect the new approach

- [ ] 5.0 Final validation and acceptance testing
  - [ ] 5.1 Rebuild the entire application from clean state: `make build -B`
  - [ ] 5.2 Deploy to Android emulator for comprehensive testing
  - [ ] 5.3 Test wide screen startup: show_sidebar_btn checked, both panels visible, one blank webview visible
  - [ ] 5.4 Test wide screen: Select search result, tabs added, clicking tab loads webview, only active visible
  - [ ] 5.5 Test wide screen: Switch between multiple tabs, verify smooth transitions without jitter
  - [ ] 5.6 Test wide screen: Toggle sidebar button, verify smooth transitions without layout jumping
  - [ ] 5.7 Test narrow screen startup: show_sidebar_btn checked, only left panel visible, no webview visible
  - [ ] 5.8 Test narrow screen: Select result, sidebar unchecks, tabs added, webview loads and displays
  - [ ] 5.9 Test narrow screen: Click different tabs, verify only active tab's webview visible
  - [ ] 5.10 Test narrow screen: Toggle show_sidebar_btn, verify all webviews hidden and results panel visible
  - [ ] 5.11 Test drawer/dialog: Open mobile menu, verify all webviews hidden
  - [ ] 5.12 Test drawer/dialog: Close mobile menu, verify appropriate webviews reappear
  - [ ] 5.13 Test drawer/dialog: Open each dialog (color theme, storage, about), verify webviews hidden
  - [ ] 5.14 Test sidebar tabs: Switch between Results/Dictionary/Gloss/Prompts tabs
  - [ ] 5.15 Test sidebar tabs: Verify only current tab's webview visible, no flickering from other tabs
  - [ ] 5.16 Test sidebar tabs: Dictionary webview only visible when Dictionary tab is current
  - [ ] 5.17 Verify primary success metric: No layout jitter during any transitions
  - [ ] 5.18 Verify secondary success metric: No blank yellow webviews appearing inappropriately
  - [ ] 5.19 Verify secondary success metric: Webview creation and tab switching remain responsive
  - [ ] 5.20 Review all acceptance criteria from PRD (8 criteria total)
  - [ ] 5.21 Document any edge cases or issues discovered during final testing
  - [ ] 5.22 Mark refactoring as complete if all acceptance criteria are met
