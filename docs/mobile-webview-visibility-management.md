# Mobile WebView Visibility Management

## Problem

On mobile platforms (Android/iOS), blank yellow webviews would sometimes cover the entire screen, obscuring the UI. This occurred in several scenarios:

1. **After opening and closing the DrawerMenu** - A blank webview would appear covering the screen
2. **When toggling between search results and suttas** - Background webviews would become visible
3. **During sidebar toggle operations** - Stray webviews would appear on top

## Root Cause

The issue stems from fundamental differences in how native WebViews behave on mobile platforms versus desktop:

### Native Platform View Rendering

`QtWebView` on Android and iOS uses **native platform views** (Android WebView, WKWebView) rather than rendering within Qt's scene graph. These native views have several characteristics that cause visibility issues:

1. **Always on top**: Native views render in a separate layer above Qt Quick/QML content
2. **Independent z-ordering**: They don't respect QML's `z` property or stacking order
3. **Visibility hierarchy issues**: Setting `visible: false` on parent QML items doesn't reliably hide the native view
4. **Async rendering**: Native views may continue rendering even when their QML wrapper is hidden

### Multiple WebView Instances

The application creates multiple WebView instances simultaneously:

- **SuttaStackLayout**: Creates one WebView per tab (dynamically created and destroyed)
- **DictionaryTab**: Has its own persistent WebView
- **Initial blank tab**: Created at startup with no content (shows yellow background)

When these WebViews are not properly hidden, they can appear on screen even when they shouldn't be visible.

### StackLayout Visibility Management

QML's `StackLayout` is designed to show only one child at a time by managing their `visible` properties. However, this automatic management doesn't work reliably for native WebViews because:

- StackLayout sets `visible: false` on non-current children
- Native WebViews may ignore this visibility setting
- The WebView continues rendering in the native layer

## Solution: Multi-Layer Visibility Control

The fix implements a **defense-in-depth** approach with multiple layers of visibility control:

### Layer 1: Item Container Wrapping

**Mechanism**: Wrap WebView in a QML `Item` container

```qml
Item {
    id: root
    anchors.fill: parent
    
    property bool is_dark
    property string data_json
    
    WebView {
        id: web
        anchors.fill: parent
        visible: root.visible
        enabled: root.visible
    }
}
```

**Why this works**:
- The outer `Item` provides a stable QML object that properly participates in the visibility hierarchy
- The WebView explicitly binds to the container's visibility
- Setting `enabled: false` tells the native view to stop processing input and rendering

### Layer 2: Explicit Visibility Binding

**Mechanism**: Add a `should_be_visible` property that controls whether a WebView should be shown

```qml
Loader {
    property bool should_be_visible: true
    
    onLoaded: {
        loader.item.visible = Qt.binding(() => loader.should_be_visible && loader.visible);
    }
}
```

**Why this works**:
- Separates the "should this be visible" logic from StackLayout's automatic management
- Creates an explicit binding that updates when conditions change
- Combines multiple visibility conditions (selected + parent visible)

### Layer 3: Dimension Collapsing

**Mechanism**: Set width and height to 0 for non-visible items

```javascript
comp.width = Qt.binding(() => (root.current_key === key) ? comp.parent.width : 0);
comp.height = Qt.binding(() => (root.current_key === key) ? comp.parent.height : 0);
```

**Why this works**:
- Even if the native view ignores `visible: false`, it has no dimensions to render into
- Prevents the WebView from occupying screen space
- Provides a physical constraint that the native view must respect

### Layer 4: Proper Drawer Detection

**Mechanism**: Check `visible` property instead of `activeFocus`

```qml
property bool webview_visible: root.is_desktop || (!mobile_menu.visible && ...)
```

**Why this works**:
- `Drawer` components don't automatically receive `activeFocus` when opened
- Checking `visible` property directly reflects the actual state
- Ensures webviews are hidden when the drawer is actually open

### Layer 5: Tab-Specific Visibility

**Mechanism**: Bind visibility to the current tab index for sidebar tabs

```qml
DictionaryTab {
    visible: root.webview_visible && rightside_tabs.currentIndex === 1
    Layout.preferredWidth: rightside_tabs.currentIndex === 1 ? parent.width : 0
    Layout.preferredHeight: rightside_tabs.currentIndex === 1 ? parent.height : 0
}
```

**Why this works**:
- Ensures only the currently selected sidebar tab's WebView is visible
- Prevents dictionary WebView from rendering when Results/Gloss/Prompts tabs are active
- Collapses dimensions when not current, preventing space allocation

## The Complete Visibility Chain

For a WebView to be visible, ALL of these conditions must be true:

1. **Current item selection**: `should_be_visible` (is this the current_key?)
2. **Loader visibility**: The Loader's `visible` property is true
3. **Parent container visibility**: The parent Item/Layout is visible
4. **No overlays**: No drawer menu or dialogs are open (`webview_visible`)
5. **Tab selection**: For sidebar tabs, the tab must be currently selected
6. **Non-zero dimensions**: Width and height must be greater than 0

If ANY condition is false, the WebView will be hidden through multiple mechanisms.

## Implementation Files

The following files implement this solution:

- `assets/qml/SuttaHtmlView_Mobile.qml` - WebView wrapped in Item with explicit visibility
- `assets/qml/DictionaryHtmlView_Mobile.qml` - Dictionary WebView wrapped with visibility control
- `assets/qml/SuttaHtmlView.qml` - Loader that propagates visibility through bindings
- `assets/qml/DictionaryHtmlView.qml` - Dictionary Loader with visibility propagation
- `assets/qml/SuttaStackLayout.qml` - Manages multiple webviews with should_be_visible and dimension control
- `assets/qml/SuttaSearchWindow.qml` - Top-level visibility control with proper drawer detection

## Key Principles

When working with mobile WebViews in Qt:

1. **Never trust implicit visibility**: Always set explicit visibility bindings
2. **Disable when hidden**: Set `enabled: false` in addition to `visible: false`
3. **Collapse dimensions**: Set width/height to 0 for hidden items
4. **Wrap in containers**: Use Item containers to provide proper QML hierarchy
5. **Multiple layers**: Use defense-in-depth with multiple visibility checks
6. **Test on actual devices**: Desktop behavior differs significantly from mobile

## Why Not Simpler Solutions?

### Why not just use StackLayout's built-in visibility?

StackLayout's automatic visibility management doesn't work for native platform views because they render in a separate layer.

### Why not destroy/recreate WebViews as needed?

While this would work, it has significant downsides:
- Performance cost of creating/destroying WebViews
- Loss of browsing state (scroll position, JavaScript state)
- Complexity in managing lifecycle
- Delays when switching between tabs

### Why not use a single WebView and swap content?

This would work but:
- Requires complete HTML reload when switching tabs
- Loses all state between switches
- Complicates back/forward navigation
- Doesn't solve the dictionary tab issue

## Testing

To verify the fix works correctly:

1. **Case 1**: Open app → click show_menu → click away → verify no blank webview
2. **Case 2**: Search → click result → open/close DrawerMenu → verify no blank webview
3. **Case 3**: Search → toggle sidebar → open/close DrawerMenu → toggle sidebar → verify no blank webview
4. **Case 4**: Switch between tabs (Results/Dictionary/Gloss/Prompts) → verify no stray webviews

All tests should show proper content with no blank yellow webviews covering the screen.
