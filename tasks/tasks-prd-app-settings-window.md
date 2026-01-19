# Tasks: App Settings Window

## Relevant Files

- `assets/qml/AppSettingsWindow.qml` - New settings window component with tabbed interface
- `assets/qml/com/profoundlabs/simsapa/AppSettingsWindow.qml` - qmllint type definition for AppSettingsWindow
- `assets/qml/com/profoundlabs/simsapa/qmldir` - QML module directory, needs AppSettingsWindow registration
- `bridges/build.rs` - Build configuration, needs AppSettingsWindow.qml added to qml_files list
- `assets/qml/SuttaSearchWindow.qml` - Main window, needs menu modifications, dialog removals, and signal connections
- `assets/qml/ColorThemeDialog.qml` - To be removed (functionality moved to AppSettingsWindow)
- `assets/qml/MobileTopMarginDialog.qml` - To be removed (functionality moved to AppSettingsWindow)
- `assets/qml/DrawerMenu.qml` - Mobile drawer menu, needs update to reflect removed menu items

### Notes

- Use `make build -B` to verify compilation after changes
- Use `make qml-test` to run QML tests
- Follow the `rightside_tabs` + `tab_stack` pattern in `SuttaSearchWindow.qml:1704-1832` for TabBar/StackLayout implementation
- Settings are persisted via existing SuttaBridge functions; no new Rust code needed

## Tasks

- [ ] 1.0 Create AppSettingsWindow.qml with basic structure and register in build system
  - [ ] 1.1 Create `assets/qml/AppSettingsWindow.qml` with Window/Frame as toplevel, including `is_mobile`, `is_desktop`, `pointSize`, and `top_bar_margin` properties
  - [ ] 1.2 Add TabBar with three TabButtons: "General", "View", "Find"
  - [ ] 1.3 Add StackLayout controlled by TabBar's currentIndex (follow `rightside_tabs`/`tab_stack` pattern)
  - [ ] 1.4 Add ScrollView/Flickable for scrollable content area within each tab's ColumnLayout
  - [ ] 1.5 Add fixed bottom area with "Close" button that calls `root.close()`
  - [ ] 1.6 Define signals: `signal themeChanged(string theme_name)` and `signal marginChanged()`
  - [ ] 1.7 Add `"../assets/qml/AppSettingsWindow.qml"` to `qml_files` list in `bridges/build.rs`
  - [ ] 1.8 Create qmllint type definition at `assets/qml/com/profoundlabs/simsapa/AppSettingsWindow.qml` with stub signals
  - [ ] 1.9 Verify build compiles with `make build -B`

- [ ] 2.0 Implement General tab settings (Notify Updates, Database Validation)
  - [ ] 2.1 Add "Updates" section Label in General tab
  - [ ] 2.2 Add CheckBox for "Notify About Simsapa Updates" that loads state from `SuttaBridge.get_notify_about_simsapa_updates()` on Component.onCompleted
  - [ ] 2.3 Connect CheckBox onCheckedChanged to call `SuttaBridge.set_notify_about_simsapa_updates(checked)`
  - [ ] 2.4 Add "Database" section Label
  - [ ] 2.5 Add Button "Run Database Validation..." that requires `database_validation_dialog` property to be passed in
  - [ ] 2.6 Verify build compiles

- [ ] 3.0 Implement View tab settings (Color Theme, Mobile Top Margin, Show Footnotes)
  - [ ] 3.1 Add "Color Theme" section Label in View tab
  - [ ] 3.2 Add ButtonGroup and three RadioButtons: "System", "Light", "Dark"
  - [ ] 3.3 Load initial theme state from `SuttaBridge.get_theme_name()` on Component.onCompleted and set appropriate RadioButton checked
  - [ ] 3.4 On RadioButton click, call `SuttaBridge.set_theme_name(theme_name)` and emit `themeChanged(theme_name)` signal
  - [ ] 3.5 Add "Mobile Top Margin" section Label (only visible when `is_mobile`)
  - [ ] 3.6 Add CheckBox "Use system value (X dp)" showing `SuttaBridge.get_status_bar_height()` value
  - [ ] 3.7 Add SpinBox for custom value (0-100), enabled only when CheckBox is unchecked
  - [ ] 3.8 Load initial margin state from `SuttaBridge.is_mobile_top_bar_margin_system()` and `SuttaBridge.get_mobile_top_bar_margin_custom_value()`
  - [ ] 3.9 On margin setting change, call appropriate SuttaBridge setter and emit `marginChanged()` signal
  - [ ] 3.10 Add "Display" section Label
  - [ ] 3.11 Add CheckBox "Show Footnotes Bar" that loads from `SuttaBridge.get_show_bottom_footnotes()`
  - [ ] 3.12 Connect CheckBox to call `SuttaBridge.set_show_bottom_footnotes(checked)` on change
  - [ ] 3.13 Verify build compiles

- [ ] 4.0 Implement Find tab settings (Search As You Type, Open Find in Sutta Results)
  - [ ] 4.1 Add "Search Behavior" section Label in Find tab
  - [ ] 4.2 Add CheckBox "Search As You Type" that loads from `SuttaBridge.get_search_as_you_type()`
  - [ ] 4.3 Connect CheckBox to call `SuttaBridge.set_search_as_you_type(checked)` on change
  - [ ] 4.4 Add CheckBox "Open Find in Sutta Results" that loads from `SuttaBridge.get_open_find_in_sutta_results()`
  - [ ] 4.5 Connect CheckBox to call `SuttaBridge.set_open_find_in_sutta_results(checked)` on change
  - [ ] 4.6 Verify build compiles

- [ ] 5.0 Integrate AppSettingsWindow into SuttaSearchWindow with menu and signals
  - [ ] 5.1 Instantiate `AppSettingsWindow` in SuttaSearchWindow with id `app_settings_window`
  - [ ] 5.2 Pass `top_bar_margin` and `database_validation_dialog` properties to AppSettingsWindow
  - [ ] 5.3 Connect `onThemeChanged` signal to call `SuttaBridge.set_theme_name(theme_name)` and `root.apply_theme()`
  - [ ] 5.4 Connect `onMarginChanged` signal to call `root.update_top_bar_margin()`
  - [ ] 5.5 Add `app_settings_window.visible` to `webview_visible` property condition (hide webview when settings open on mobile)
  - [ ] 5.6 Add "Settings..." menu item to `file_menu` before "Close Window" with `Ctrl+,` shortcut
  - [ ] 5.7 Create Action `action_settings` that calls `app_settings_window.show()`
  - [ ] 5.8 Verify build compiles and Settings window opens from menu

- [ ] 6.0 Remove migrated menu items and actions from SuttaSearchWindow
  - [ ] 6.1 Remove entire `view_menu` Menu from MenuBar
  - [ ] 6.2 Remove `search_as_you_type` CMenuItem and Action from `find_menu`
  - [ ] 6.3 Remove `action_open_find_in_sutta_results` CMenuItem and Action from `find_menu`
  - [ ] 6.4 Remove `action_notify_about_updates` CMenuItem and Action from `help_menu`
  - [ ] 6.5 Remove `action_database_validation` CMenuItem and Action from `help_menu`
  - [ ] 6.6 Update `mobile_menu` DrawerMenu `menu_list` to remove `view_menu` reference
  - [ ] 6.7 Update references to removed actions: change `search_as_you_type.checked` references to use `SuttaBridge.get_search_as_you_type()` directly
  - [ ] 6.8 Update `action_open_find_in_sutta_results.checked` references to use `SuttaBridge.get_open_find_in_sutta_results()` directly
  - [ ] 6.9 Remove `color_theme_dialog` and `mobile_top_margin_dialog` instantiations
  - [ ] 6.10 Update `webview_visible` property to remove `color_theme_dialog.visible` and `mobile_top_margin_dialog.visible` conditions
  - [ ] 6.11 Verify build compiles and all settings are accessible only from AppSettingsWindow

- [ ] 7.0 Cleanup: Remove ColorThemeDialog.qml and MobileTopMarginDialog.qml
  - [ ] 7.1 Delete `assets/qml/ColorThemeDialog.qml` file
  - [ ] 7.2 Delete `assets/qml/MobileTopMarginDialog.qml` file
  - [ ] 7.3 Remove `"../assets/qml/ColorThemeDialog.qml"` from `qml_files` list in `bridges/build.rs`
  - [ ] 7.4 Remove `"../assets/qml/MobileTopMarginDialog.qml"` from `qml_files` list in `bridges/build.rs`
  - [ ] 7.5 Verify final build compiles with `make build -B`
  - [ ] 7.6 Run `make qml-test` to verify QML tests pass
