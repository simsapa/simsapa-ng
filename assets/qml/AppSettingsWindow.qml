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

    // State properties for mobile margin settings
    property bool use_system_margin: true
    property int custom_margin_value: 24

    // Expose settings as properties for external access (avoid repeated database calls)
    property alias search_as_you_type: search_as_you_type_checkbox.checked
    property alias open_find_in_sutta_results: open_find_in_results_checkbox.checked

    ThemeHelper {
        id: theme_helper
        target_window: root
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

                        Item { Layout.fillHeight: true }
                    }
                }

                // View Tab
                ScrollView {
                    Layout.fillWidth: true
                    Layout.fillHeight: true
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
    }
}
