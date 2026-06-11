# PRD: App Settings Window

## Introduction/Overview

This feature introduces a centralized Settings window (`AppSettingsWindow.qml`) to consolidate various application preferences currently scattered across menu items and dialogs. The Settings window will provide a tabbed interface for organizing settings into logical groups, improving discoverability and usability, especially on mobile devices where menu navigation is more cumbersome.

The window will be accessible from "File > Settings" menu and will contain settings previously found in the View menu, checkable items from the Find menu, and some items from the Help menu.

## Goals

1. Centralize application settings into a single, discoverable location
2. Simplify the menu structure by removing less-frequently used options
3. Improve mobile user experience where menus are harder to navigate
4. Eliminate duplication of settings controls between menus and dialogs
5. Provide immediate visual feedback when settings are changed (live preview)

## User Stories

1. As a user, I want to access all app settings from one place so that I can configure the app without searching through multiple menus.
2. As a mobile user, I want a dedicated settings window so that I can change preferences without navigating complex menu structures.
3. As a user, I want to see my changes applied immediately so that I can preview the effect of my settings.
4. As a user, I want settings organized into logical tabs so that I can quickly find the option I'm looking for.

## Functional Requirements

### FR1: Settings Window Structure

1.1. Create a new `AppSettingsWindow.qml` component that opens as a non-modal window (users can interact with both windows simultaneously).

1.2. The window must use a `Frame` as its toplevel container with adequate margins for mobile devices (using the existing `top_bar_margin` pattern).

1.3. The window must have:
   - A scrollable content area for settings (using `ScrollView` or `Flickable`)
   - A fixed bottom area containing a "Close" button

1.4. Implement a `TabBar` with `TabButton` items at the top, controlling a `StackLayout` that shows one settings group at a time (following the pattern of `rightside_tabs` and `tab_stack` in `SuttaSearchWindow.qml` lines 1704-1832).

### FR2: Tab Organization

2.1. Create three tabs in this order:
   - **General** - Application-wide settings
   - **View** - Display and appearance settings
   - **Find** - Search behavior settings

### FR3: General Tab Settings

3.1. **Notify About Simsapa Updates** - Checkable setting (previously `action_notify_about_updates` in `help_menu`)
   - Uses `SuttaBridge.get_notify_about_simsapa_updates()` to load initial state
   - Uses `SuttaBridge.set_notify_about_simsapa_updates(checked)` on change
   - Apply changes immediately

3.2. **Database Validation** - Button that opens `DatabaseValidationDialog`
   - Previously `action_database_validation` in `help_menu`
   - The dialog itself remains as a separate component
   - Button text: "Run Database Validation..."

### FR4: View Tab Settings

4.1. **Color Theme** - Radio button group (previously `ColorThemeDialog`)
   - Options: "System", "Light", "Dark"
   - Uses `SuttaBridge.get_theme_name()` to load initial state
   - Uses `SuttaBridge.set_theme_name(theme_name)` on change
   - Emit signal to trigger `apply_theme()` on `SuttaSearchWindow`
   - Apply changes immediately (live preview)

4.2. **Mobile Top Margin** - Checkbox and SpinBox (previously `MobileTopMarginDialog`)
   - Only visible/enabled when `is_mobile` is true
   - Checkbox: "Use system value (X dp)" where X is from `SuttaBridge.get_status_bar_height()`
   - SpinBox: Custom value (0-100 dp), enabled when checkbox is unchecked
   - Uses `SuttaBridge.is_mobile_top_bar_margin_system()` and `SuttaBridge.get_mobile_top_bar_margin_custom_value()` to load initial state
   - Uses `SuttaBridge.set_mobile_top_bar_margin_system()` or `SuttaBridge.set_mobile_top_bar_margin_custom(value)` on change
   - Emit signal to trigger `update_top_bar_margin()` on `SuttaSearchWindow`
   - Apply changes immediately

4.3. **Show Footnotes Bar** - Checkable setting (previously `action_show_bottom_footnotes` in `view_menu`)
   - Uses `SuttaBridge.get_show_bottom_footnotes()` to load initial state
   - Uses `SuttaBridge.set_show_bottom_footnotes(checked)` on change
   - Apply changes immediately

### FR5: Find Tab Settings

5.1. **Search As You Type** - Checkable setting (previously `search_as_you_type` in `find_menu`)
   - Uses `SuttaBridge.get_search_as_you_type()` to load initial state
   - Uses `SuttaBridge.set_search_as_you_type(checked)` on change
   - Apply changes immediately

5.2. **Open Find in Sutta Results** - Checkable setting (previously `action_open_find_in_sutta_results` in `find_menu`)
   - Uses `SuttaBridge.get_open_find_in_sutta_results()` to load initial state
   - Uses `SuttaBridge.set_open_find_in_sutta_results(checked)` on change
   - Apply changes immediately

### FR6: Menu Modifications

6.1. Add "Settings..." menu item to `file_menu` that opens `AppSettingsWindow`
   - Position: After existing items, before "Close Window"
   - Keyboard shortcut: `Ctrl+,` (standard settings shortcut)

6.2. Remove `view_menu` entirely from the MenuBar

6.3. Remove from `find_menu`:
   - `search_as_you_type` checkable action
   - `action_open_find_in_sutta_results` checkable action

6.4. Remove from `help_menu`:
   - `action_notify_about_updates` checkable action
   - `action_database_validation` action

6.5. Update `DrawerMenu` (`mobile_menu`) to reflect the removed menu and items

### FR7: Signal Communication

7.1. Define signals on `AppSettingsWindow` for settings that require `SuttaSearchWindow` to react:
   - `signal themeChanged(string theme_name)` - for color theme changes
   - `signal marginChanged()` - for mobile top margin changes

7.2. Connect these signals in `SuttaSearchWindow` where `AppSettingsWindow` is instantiated:
   ```qml
   AppSettingsWindow {
       id: app_settings_window
       onThemeChanged: function(theme_name) {
           SuttaBridge.set_theme_name(theme_name);
           root.apply_theme();
       }
       onMarginChanged: {
           root.update_top_bar_margin();
       }
   }
   ```

### FR8: Cleanup

8.1. Remove `ColorThemeDialog.qml` component (functionality moved inline to AppSettingsWindow)

8.2. Remove `MobileTopMarginDialog.qml` component (functionality moved inline to AppSettingsWindow)

8.3. Remove the dialog instances from `SuttaSearchWindow.qml`:
   - `color_theme_dialog`
   - `mobile_top_margin_dialog`

8.4. Update `webview_visible` property in `SuttaSearchWindow` to remove references to removed dialogs

8.5. Remove the action definitions that are no longer used:
   - `action_show_bottom_footnotes` (define in AppSettingsWindow instead)
   - `search_as_you_type` (define in AppSettingsWindow instead)
   - `action_open_find_in_sutta_results` (define in AppSettingsWindow instead)
   - `action_notify_about_updates` (define in AppSettingsWindow instead)

## Non-Goals (Out of Scope)

1. Moving non-checkable menu items like "Focus Search Input", "Find in Page", etc. to Settings
2. Adding new settings beyond those already existing in the menus
3. Creating a separate mobile-specific settings UI
4. Persisting window position/size for the settings window
5. Adding undo/cancel functionality for settings changes (changes apply immediately)
6. Moving the following Help menu items: "Check for Simsapa Updates...", "Dhamma Text Sources", "About"

## Design Considerations

### Layout Structure

```
+--------------------------------------------------+
|  [General] [View] [Find]     <- TabBar           |
+--------------------------------------------------+
|                                                  |
|  +--------------------------------------------+  |
|  |                                            |  |
|  |  (Scrollable settings content area)        |  |
|  |                                            |  |
|  |  Section label                             |  |
|  |  [ ] Checkbox setting                      |  |
|  |  ( ) Radio option 1                        |  |
|  |  ( ) Radio option 2                        |  |
|  |  [Button]                                  |  |
|  |                                            |  |
|  +--------------------------------------------+  |
|                                                  |
+--------------------------------------------------+
|                               [Close]            |
+--------------------------------------------------+
```

### Styling

- Use consistent `font.pointSize` based on `is_mobile` (16 for mobile, 12 for desktop)
- Use `ColumnLayout` for settings within each tab
- Use `Label` for section headers with slightly larger/bold text
- Use standard Qt Quick Controls 2 components: `CheckBox`, `RadioButton`, `SpinBox`, `Button`
- Follow existing spacing patterns (10-15 pixel margins, 10-15 pixel spacing)

### Mobile Considerations

- Apply `top_bar_margin` to account for Android status bar
- Settings window should use appropriate sizing for mobile (near full screen)
- Use larger touch targets on mobile (already handled by `pointSize` differences)

## Technical Considerations

### File Locations

- New file: `assets/qml/AppSettingsWindow.qml`
- Register in `bridges/build.rs` `qml_files` list
- Create qmllint type definition: `assets/qml/com/profoundlabs/simsapa/AppSettingsWindow.qml`
- Update `assets/qml/com/profoundlabs/simsapa/qmldir`

### Integration Points

1. **SuttaBridge functions used:**
   - `get_theme_name()`, `set_theme_name(string)`
   - `get_status_bar_height()`
   - `is_mobile_top_bar_margin_system()`, `get_mobile_top_bar_margin_custom_value()`
   - `set_mobile_top_bar_margin_system()`, `set_mobile_top_bar_margin_custom(int)`
   - `get_show_bottom_footnotes()`, `set_show_bottom_footnotes(bool)`
   - `get_search_as_you_type()`, `set_search_as_you_type(bool)`
   - `get_open_find_in_sutta_results()`, `set_open_find_in_sutta_results(bool)`
   - `get_notify_about_simsapa_updates()`, `set_notify_about_simsapa_updates(bool)`

2. **Pattern Reference:** Follow `rightside_tabs` + `tab_stack` pattern in `SuttaSearchWindow.qml:1704-1832`

3. **Dialog Reference:** Use `DatabaseValidationDialog` from settings (remains a separate dialog opened via button)

### State Synchronization

Since settings are applied immediately via SuttaBridge calls:
- SuttaBridge already emits signals when settings change (e.g., `onShowBottomFootnotesChanged`)
- The existing `Connections` handlers in `SuttaSearchWindow` will continue to work
- No need for complex state synchronization between windows

## Success Metrics

1. All settings from removed menus are accessible from AppSettingsWindow
2. Settings changes apply immediately without requiring explicit save
3. No duplicate controls exist between menus and settings window
4. Mobile users can access all settings without using the drawer menu
5. Build and QML tests pass without errors

## Open Questions

1. Should the settings window remember its open/closed state across app restarts? (Currently: No, out of scope)
