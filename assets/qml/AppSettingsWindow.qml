pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import QtQuick.Controls
import QtQuick.Window

import com.profoundlabs.simsapa

ApplicationWindow {
    id: root

    title: "Settings - Simsapa"
    width: is_mobile ? Screen.desktopAvailableWidth : 600
    height: is_mobile ? Screen.desktopAvailableHeight : Math.min(800, Screen.desktopAvailableHeight)
    visible: false
    color: palette.window
    flags: Qt.Dialog

    readonly property bool is_mobile: Qt.platform.os === "android" || Qt.platform.os === "ios"
    readonly property bool is_desktop: !root.is_mobile
    readonly property int pointSize: is_mobile ? 16 : 12

    property int top_bar_margin: 0
    property var database_validation_dialog: null

    signal themeChanged(string theme_name)
    signal marginChanged()
    signal keybindingsChanged()

    // State properties for mobile margin settings
    property bool use_system_margin: true
    property int custom_margin_value: 24

    // Wake lock state
    property bool wake_lock_acquired: false

    // Keybindings data
    property var keybindings_data: ({})
    property var default_keybindings: ({})
    property var action_names: ({})
    property var action_descriptions: ({})
    property var action_ids_list: []

    // Expose settings as properties for external access (avoid repeated database calls)
    property alias search_as_you_type: search_as_you_type_checkbox.checked
    property alias open_find_in_sutta_results: open_find_in_results_checkbox.checked

    ThemeHelper {
        id: theme_helper
        target_window: root
    }

    // State for capture dialog
    property string capture_action_id: ""
    property int capture_shortcut_index: -1
    property bool capture_is_new: false

    // State for conflict dialog
    property string pending_shortcut: ""
    property string conflicting_action_id: ""

    // Load keybindings from backend
    function load_keybindings() {
        root.keybindings_data = JSON.parse(SuttaBridge.get_keybindings_json());
        root.default_keybindings = JSON.parse(SuttaBridge.get_default_keybindings_json());
        root.action_names = JSON.parse(SuttaBridge.get_action_names_json());
        root.action_descriptions = JSON.parse(SuttaBridge.get_action_descriptions_json());
        root.action_ids_list = Object.keys(root.action_names);
    }

    // Open capture dialog for editing existing shortcut
    function open_capture_dialog(action_id: string, shortcut_index: int, current_shortcut: string) {
        root.capture_action_id = action_id;
        root.capture_shortcut_index = shortcut_index;
        root.capture_is_new = false;

        keybinding_capture_dialog.action_name = root.action_names[action_id] || action_id;
        keybinding_capture_dialog.current_shortcut = current_shortcut;
        keybinding_capture_dialog.is_new_shortcut = false;
        keybinding_capture_dialog.show();
    }

    // Open capture dialog for adding new shortcut
    function open_capture_dialog_for_new(action_id: string) {
        root.capture_action_id = action_id;
        root.capture_shortcut_index = -1;
        root.capture_is_new = true;

        keybinding_capture_dialog.action_name = root.action_names[action_id] || action_id;
        keybinding_capture_dialog.current_shortcut = "";
        keybinding_capture_dialog.is_new_shortcut = true;
        keybinding_capture_dialog.show();
    }

    // Find if shortcut conflicts with another action, returns action_id or empty string
    function find_conflict(shortcut: string, exclude_action_id: string): string {
        for (let action_id in root.keybindings_data) {
            if (action_id === exclude_action_id) continue;
            let shortcuts = root.keybindings_data[action_id];
            if (shortcuts && shortcuts.indexOf(shortcut) >= 0) {
                return action_id;
            }
        }
        return "";
    }

    // Save shortcut at specific index
    function save_shortcut(action_id: string, shortcut_index: int, new_shortcut: string) {
        let shortcuts = root.keybindings_data[action_id] || [];
        shortcuts = shortcuts.slice(); // copy array
        if (shortcut_index >= 0 && shortcut_index < shortcuts.length) {
            shortcuts[shortcut_index] = new_shortcut;
        }
        SuttaBridge.set_keybinding(action_id, JSON.stringify(shortcuts));
        root.load_keybindings();
        root.keybindingsChanged();
    }

    // Add new shortcut to action
    function add_shortcut(action_id: string, new_shortcut: string) {
        let shortcuts = root.keybindings_data[action_id] || [];
        shortcuts = shortcuts.slice(); // copy array
        shortcuts.push(new_shortcut);
        SuttaBridge.set_keybinding(action_id, JSON.stringify(shortcuts));
        root.load_keybindings();
        root.keybindingsChanged();
    }

    // Remove shortcut at index from action
    function remove_shortcut(action_id: string, shortcut_index: int) {
        let shortcuts = root.keybindings_data[action_id] || [];
        shortcuts = shortcuts.slice(); // copy array
        if (shortcut_index >= 0 && shortcut_index < shortcuts.length) {
            shortcuts.splice(shortcut_index, 1);
        }
        SuttaBridge.set_keybinding(action_id, JSON.stringify(shortcuts));
        root.load_keybindings();
        root.keybindingsChanged();
    }

    // Remove shortcut from conflicting action
    function remove_conflict_shortcut(action_id: string, shortcut: string) {
        let shortcuts = root.keybindings_data[action_id] || [];
        shortcuts = shortcuts.slice(); // copy array
        let idx = shortcuts.indexOf(shortcut);
        if (idx >= 0) {
            shortcuts.splice(idx, 1);
        }
        SuttaBridge.set_keybinding(action_id, JSON.stringify(shortcuts));
    }

    // Handle accepted shortcut with conflict check
    function handle_shortcut_accepted(shortcut: string) {
        let conflict_action = find_conflict(shortcut, root.capture_action_id);

        if (conflict_action !== "") {
            // Store pending state and show conflict dialog
            root.pending_shortcut = shortcut;
            root.conflicting_action_id = conflict_action;
            shortcut_conflict_dialog.shortcut = shortcut;
            shortcut_conflict_dialog.conflicting_action_name = root.action_names[conflict_action] || conflict_action;
            shortcut_conflict_dialog.open();
        } else {
            // No conflict, apply directly
            apply_shortcut(shortcut);
        }
    }

    // Apply the shortcut (after conflict resolution or no conflict)
    function apply_shortcut(shortcut: string) {
        if (root.capture_is_new) {
            add_shortcut(root.capture_action_id, shortcut);
        } else {
            save_shortcut(root.capture_action_id, root.capture_shortcut_index, shortcut);
        }
    }

    // Keybinding capture dialog
    KeybindingCaptureDialog {
        id: keybinding_capture_dialog
        top_bar_margin: root.top_bar_margin

        onShortcutAccepted: function(shortcut) {
            root.handle_shortcut_accepted(shortcut);
        }

        onShortcutRemoved: {
            root.remove_shortcut(root.capture_action_id, root.capture_shortcut_index);
        }
    }

    // Shortcut conflict dialog
    ShortcutConflictDialog {
        id: shortcut_conflict_dialog
        parent: Overlay.overlay
        anchors.centerIn: parent

        onConfirmed: {
            // Remove from conflicting action and apply
            root.remove_conflict_shortcut(root.conflicting_action_id, root.pending_shortcut);
            root.apply_shortcut(root.pending_shortcut);
        }

        onCancelled: {
            // Do nothing, user cancelled
        }
    }

    Frame {
        anchors.fill: parent

        ColumnLayout {
            spacing: 0
            anchors.fill: parent
            anchors.topMargin: root.top_bar_margin
            anchors.margins: 10

            TabBar {
                id: settings_tabs
                Layout.fillWidth: true

                TabButton {
                    text: "General"
                    padding: 5
                }

                TabButton {
                    text: "View"
                    padding: 5
                }

                TabButton {
                    text: "Find"
                    padding: 5
                }

                TabButton {
                    text: "Keybindings"
                    padding: 5
                    visible: root.is_desktop
                }
            }

            StackLayout {
                id: settings_stack
                currentIndex: settings_tabs.currentIndex
                Layout.fillWidth: true
                Layout.fillHeight: true

                // General Tab
                ScrollView {
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    contentWidth: availableWidth
                    clip: true

                    ColumnLayout {
                        width: parent.width
                        spacing: 15

                        Label {
                            text: "General Settings"
                            font.pointSize: root.pointSize + 2
                            font.bold: true
                            Layout.topMargin: 10
                        }

                        // Updates section
                        Label {
                            text: "Updates"
                            font.pointSize: root.pointSize + 1
                            font.bold: true
                            Layout.topMargin: 10
                        }

                        CheckBox {
                            id: notify_updates_checkbox
                            text: "Notify About Simsapa Updates"
                            font.pointSize: root.pointSize
                            onCheckedChanged: {
                                SuttaBridge.set_notify_about_simsapa_updates(checked);
                            }
                        }

                        Button {
                            text: "Check for Simsapa Updates..."
                            font.pointSize: root.pointSize
                            onClicked: {
                                SuttaBridge.check_for_updates(true, Screen.desktopAvailableWidth + " x " + Screen.desktopAvailableHeight, "determine");
                            }
                        }

                        // Database section
                        Label {
                            text: "Database"
                            font.pointSize: root.pointSize + 1
                            font.bold: true
                            Layout.topMargin: 10
                        }

                        Button {
                            text: "Run Database Validation..."
                            font.pointSize: root.pointSize
                            onClicked: {
                                if (root.database_validation_dialog) {
                                    root.database_validation_dialog.show_from_menu();
                                }
                            }
                        }

                        // Wake Lock section (mobile only)
                        Label {
                            visible: root.is_mobile
                            text: "Wake Lock"
                            font.pointSize: root.pointSize + 1
                            font.bold: true
                            Layout.topMargin: 10
                        }

                        Label {
                            visible: root.is_mobile
                            text: "The wake lock for example allows the Obsidian Simsapa plugin to communicate with the Simsapa app while it is in the background."
                            font.pointSize: root.pointSize - 2
                            wrapMode: Text.WordWrap
                            Layout.fillWidth: true
                        }

                        Button {
                            visible: root.is_mobile
                            text: root.wake_lock_acquired ? "Release Wake Lock" : "Acquire Wake Lock"
                            font.pointSize: root.pointSize
                            onClicked: {
                                if (root.wake_lock_acquired) {
                                    SuttaBridge.release_wake_lock_rust();
                                } else {
                                    SuttaBridge.acquire_wake_lock_rust();
                                }
                                root.wake_lock_acquired = SuttaBridge.is_wake_lock_acquired_rust();
                            }
                        }

                        Button {
                            visible: root.is_mobile
                            text: "Refresh Status"
                            font.pointSize: root.pointSize
                            onClicked: {
                                root.wake_lock_acquired = SuttaBridge.is_wake_lock_acquired_rust();
                            }
                        }

                        Label {
                            visible: root.is_mobile
                            text: "Wake Lock Status: " + (root.wake_lock_acquired ? "Acquired" : "Not Acquired")
                            font.pointSize: root.pointSize
                        }

                        Item { Layout.fillHeight: true }
                    }
                }

                // View Tab
                ScrollView {
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    contentWidth: availableWidth
                    clip: true

                    ColumnLayout {
                        width: parent.width
                        spacing: 15

                        Label {
                            text: "View Settings"
                            font.pointSize: root.pointSize + 2
                            font.bold: true
                            Layout.topMargin: 10
                        }

                        // Color Theme section
                        Label {
                            text: "Color Theme"
                            font.pointSize: root.pointSize + 1
                            font.bold: true
                            Layout.topMargin: 10
                        }

                        ButtonGroup {
                            id: theme_group
                        }

                        RadioButton {
                            id: system_theme_radio
                            text: "System"
                            font.pointSize: root.pointSize
                            ButtonGroup.group: theme_group
                            onClicked: {
                                SuttaBridge.set_theme_name("system");
                                theme_helper.apply();
                                root.themeChanged("system");
                            }
                        }

                        RadioButton {
                            id: light_theme_radio
                            text: "Light"
                            font.pointSize: root.pointSize
                            ButtonGroup.group: theme_group
                            onClicked: {
                                SuttaBridge.set_theme_name("light");
                                theme_helper.apply();
                                root.themeChanged("light");
                            }
                        }

                        RadioButton {
                            id: dark_theme_radio
                            text: "Dark"
                            font.pointSize: root.pointSize
                            ButtonGroup.group: theme_group
                            onClicked: {
                                SuttaBridge.set_theme_name("dark");
                                theme_helper.apply();
                                root.themeChanged("dark");
                            }
                        }

                        // Mobile Top Margin section (only visible on mobile)
                        Label {
                            visible: root.is_mobile
                            text: "Mobile Top Margin"
                            font.pointSize: root.pointSize + 1
                            font.bold: true
                            Layout.topMargin: 10
                        }

                        Label {
                            visible: root.is_mobile
                            text: "The spacing between the mobile's UI status bar and the app's top elements."
                            font.pointSize: root.pointSize - 2
                            wrapMode: Text.WordWrap
                            Layout.fillWidth: true
                        }

                        CheckBox {
                            visible: root.is_mobile
                            id: use_system_margin_checkbox
                            text: "Use system value (" + SuttaBridge.get_status_bar_height() + " dp)"
                            font.pointSize: root.pointSize
                            checked: root.use_system_margin
                            onCheckedChanged: {
                                root.use_system_margin = checked;
                                if (checked) {
                                    SuttaBridge.set_mobile_top_bar_margin_system();
                                } else {
                                    SuttaBridge.set_mobile_top_bar_margin_custom(root.custom_margin_value);
                                }
                                root.marginChanged();
                            }
                        }

                        RowLayout {
                            visible: root.is_mobile
                            Layout.fillWidth: true
                            spacing: 10
                            enabled: !root.use_system_margin

                            Label {
                                text: "Custom value (dp):"
                                font.pointSize: root.pointSize
                                opacity: root.use_system_margin ? 0.5 : 1.0
                            }

                            SpinBox {
                                id: custom_margin_spinbox
                                from: 0
                                to: 100
                                value: root.custom_margin_value
                                editable: true
                                font.pointSize: root.pointSize
                                opacity: root.use_system_margin ? 0.5 : 1.0
                                onValueModified: {
                                    root.custom_margin_value = value;
                                    if (!root.use_system_margin) {
                                        SuttaBridge.set_mobile_top_bar_margin_custom(value);
                                        root.marginChanged();
                                    }
                                }
                            }
                        }

                        // Display section
                        Label {
                            text: "Display"
                            font.pointSize: root.pointSize + 1
                            font.bold: true
                            Layout.topMargin: 10
                        }

                        CheckBox {
                            id: show_footnotes_checkbox
                            text: "Show Footnotes Bar"
                            font.pointSize: root.pointSize
                            onCheckedChanged: {
                                SuttaBridge.set_show_bottom_footnotes(checked);
                            }
                        }

                        Label {
                            text: "While scrolling on a page, show the definitions of visible footnotes at the bottom of the page."
                            font.pointSize: root.pointSize - 2
                            wrapMode: Text.WordWrap
                            Layout.fillWidth: true
                        }

                        Item { Layout.fillHeight: true }
                    }
                }

                // Find Tab
                ScrollView {
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    contentWidth: availableWidth
                    clip: true

                    ColumnLayout {
                        width: parent.width
                        spacing: 15

                        Label {
                            text: "Find Settings"
                            font.pointSize: root.pointSize + 2
                            font.bold: true
                            Layout.topMargin: 10
                        }

                        // Search Behavior section
                        Label {
                            text: "Search Behavior"
                            font.pointSize: root.pointSize + 1
                            font.bold: true
                            Layout.topMargin: 10
                        }

                        CheckBox {
                            id: search_as_you_type_checkbox
                            text: "Search As You Type"
                            font.pointSize: root.pointSize
                            onCheckedChanged: {
                                SuttaBridge.set_search_as_you_type(checked);
                            }
                        }

                        Label {
                            text: "The search query is immediately started while typing to provide incremental results."
                            font.pointSize: root.pointSize - 2
                            wrapMode: Text.WordWrap
                            Layout.fillWidth: true
                        }

                        CheckBox {
                            id: open_find_in_results_checkbox
                            text: "Open Find in Sutta Results"
                            font.pointSize: root.pointSize
                            onCheckedChanged: {
                                SuttaBridge.set_open_find_in_sutta_results(checked);
                            }
                        }

                        Label {
                            text: "When selecting a search result, the page also opens the Find Bar with the current search query to jump to the first occurrence of the search term."
                            font.pointSize: root.pointSize - 2
                            wrapMode: Text.WordWrap
                            Layout.fillWidth: true
                        }

                        Item { Layout.fillHeight: true }
                    }
                }

                // Keybindings Tab
                ScrollView {
                    id: keybindings_scrollview
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    contentWidth: availableWidth
                    clip: true
                    ScrollBar.vertical.policy: ScrollBar.AlwaysOn

                    ColumnLayout {
                        width: keybindings_scrollview.availableWidth - 20
                        spacing: 10

                        Label {
                            text: "Keybindings"
                            font.pointSize: root.pointSize + 2
                            font.bold: true
                            Layout.topMargin: 10
                            Layout.fillWidth: true
                        }

                        Label {
                            text: "Click a keyboard shortcut to edit, or use [+] to add additional shortcuts."
                            font.pointSize: root.pointSize - 2
                            wrapMode: Text.WordWrap
                            Layout.fillWidth: true
                        }

                        // Reset All button at top
                        Button {
                            text: "Reset All to Defaults"
                            font.pointSize: root.pointSize
                            Layout.bottomMargin: 10
                            onClicked: {
                                SuttaBridge.reset_all_keybindings();
                                root.load_keybindings();
                                root.keybindingsChanged();
                            }
                        }

                        // Keybindings list
                        Repeater {
                            id: keybindings_repeater
                            model: root.action_ids_list

                            delegate: ColumnLayout {
                                id: keybinding_item
                                Layout.fillWidth: true
                                spacing: 2
                                required property string modelData
                                required property int index

                                property string action_id: modelData
                                property var shortcuts: root.keybindings_data[action_id] || []
                                property var default_shortcuts: root.default_keybindings[action_id] || []

                                RowLayout {
                                    id: keybinding_row
                                    Layout.fillWidth: true
                                    spacing: 8

                                    // Action name
                                    Label {
                                        id: label_action_name
                                        text: root.action_names[keybinding_item.action_id] || keybinding_item.action_id
                                        font.pointSize: root.pointSize
                                        Layout.minimumWidth: 180
                                    }

                                    // Shortcut buttons
                                    Flow {
                                        Layout.fillWidth: true
                                        spacing: 5

                                        Repeater {
                                            model: keybinding_item.shortcuts

                                            delegate: Button {
                                                id: shortcut_button
                                                required property string modelData
                                                required property int index

                                                text: modelData
                                                font.pointSize: root.pointSize - 1
                                                padding: 5

                                                // Highlight if different from default
                                                property bool is_default: {
                                                    let defaults = keybinding_item.default_shortcuts;
                                                    return defaults.indexOf(modelData) >= 0;
                                                }

                                                background: Rectangle {
                                                    color: shortcut_button.is_default ?
                                                        (shortcut_button.down ? palette.mid : palette.button) :
                                                        (shortcut_button.down ? "#5a9bd4" : "#7ab8e8")
                                                    border.color: shortcut_button.is_default ? palette.mid : "#4a8bc4"
                                                    border.width: 1
                                                    radius: 4
                                                }

                                                onClicked: {
                                                    root.open_capture_dialog(keybinding_item.action_id, shortcut_button.index, modelData);
                                                }
                                            }
                                        }

                                        // Add [+] button
                                        Button {
                                            text: "+"
                                            font.pointSize: root.pointSize - 1
                                            padding: 5
                                            implicitWidth: 30

                                            onClicked: {
                                                root.open_capture_dialog_for_new(keybinding_item.action_id);
                                            }
                                        }

                                        // Spacer to push reset button to the right
                                        Item { Layout.fillWidth: true }
                                    }

                                    // Reset button
                                    Button {
                                        text: "Reset"
                                        font.pointSize: root.pointSize - 2
                                        padding: 4
                                        visible: {
                                            let current = JSON.stringify(keybinding_item.shortcuts);
                                            let defaults = JSON.stringify(keybinding_item.default_shortcuts);
                                            return current !== defaults;
                                        }

                                        onClicked: {
                                            SuttaBridge.reset_keybinding(keybinding_item.action_id);
                                            root.load_keybindings();
                                            root.keybindingsChanged();
                                        }
                                    }
                                }

                                // Description label
                                Label {
                                    text: root.action_descriptions[keybinding_item.action_id] || ""
                                    font.pointSize: root.pointSize - 2
                                    color: palette.placeholderText
                                    wrapMode: Text.WordWrap
                                    Layout.fillWidth: true
                                    Layout.bottomMargin: 8
                                }
                            }
                        }

                        Item { Layout.fillHeight: true }
                    }
                }
            }

            // Fixed bottom area with Close button
            Item {
                Layout.fillWidth: true
                Layout.preferredHeight: 50
                Layout.topMargin: 10

                Button {
                    text: "Close"
                    anchors.right: parent.right
                    anchors.verticalCenter: parent.verticalCenter
                    font.pointSize: root.pointSize
                    onClicked: root.close()
                }
            }
        }
    }

    Component.onCompleted: {
        theme_helper.apply();

        // Load initial state for General tab settings
        notify_updates_checkbox.checked = SuttaBridge.get_notify_about_simsapa_updates();

        // Load initial state for View tab settings
        let theme_name = SuttaBridge.get_theme_name();
        if (theme_name === "system") {
            system_theme_radio.checked = true;
        } else if (theme_name === "light") {
            light_theme_radio.checked = true;
        } else if (theme_name === "dark") {
            dark_theme_radio.checked = true;
        }

        // Load mobile margin settings into root properties
        if (root.is_mobile) {
            root.use_system_margin = SuttaBridge.is_mobile_top_bar_margin_system();
            if (!root.use_system_margin) {
                root.custom_margin_value = SuttaBridge.get_mobile_top_bar_margin_custom_value();
            }
        }

        // Load footnotes setting
        show_footnotes_checkbox.checked = SuttaBridge.get_show_bottom_footnotes();

        // Load initial state for Find tab settings
        search_as_you_type_checkbox.checked = SuttaBridge.get_search_as_you_type();
        open_find_in_results_checkbox.checked = SuttaBridge.get_open_find_in_sutta_results();

        // Load wake lock state (mobile only)
        if (root.is_mobile) {
            root.wake_lock_acquired = SuttaBridge.is_wake_lock_acquired_rust();
        }

        // Load keybindings
        root.load_keybindings();
    }
}
