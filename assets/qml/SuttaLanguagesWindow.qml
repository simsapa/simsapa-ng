pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import QtQuick.Controls
import QtQuick.Window

import com.profoundlabs.simsapa

ApplicationWindow {
    id: root

    title: "Sutta Languages"
    width: is_mobile ? Screen.desktopAvailableWidth : 600
    // Height must not be greater than the screen
    height: is_mobile ? Screen.desktopAvailableHeight : Math.min(900, Screen.desktopAvailableHeight)
    visible: true
    color: palette.window
    flags: Qt.Dialog

    readonly property bool is_mobile: Qt.platform.os === "android" || Qt.platform.os === "ios"
    readonly property bool is_desktop: !root.is_mobile

    readonly property int pointSize: is_mobile ? 16 : 12
    readonly property int largePointSize: pointSize + 5

    property var available_languages: []
    property var installed_languages_with_counts: []

    AssetManager { id: manager }

    Connections {
        target: manager

        function onRemovalShowMsg(message: string) {
            processing_label.text = message;
        }

        function onRemovalCompleted(success: bool, error_msg: string) {
            busy_indicator.running = false;

            if (success) {
                completion_dialog.open();
            } else {
                error_dialog.error_message = error_msg || "Failed to remove languages. Please check the application logs for details.";
                error_dialog.open();
            }
        }
    }

    Component.onCompleted: {
        // Populate language lists
        available_languages = manager.get_available_languages();
        installed_languages_with_counts = SuttaBridge.get_sutta_language_labels_with_counts();
    }

    // Confirmation dialog for language removal
    Dialog {
        id: confirm_removal_dialog
        title: "Confirm Language Removal"
        anchors.centerIn: parent
        modal: true
        standardButtons: Dialog.Cancel | Dialog.Ok

        property var languages_to_remove: []

        ColumnLayout {
            spacing: 10
            width: parent.width

            Label {
                text: "Are you sure you want to remove the following languages?"
                font.pointSize: root.pointSize
                wrapMode: Text.WordWrap
                Layout.fillWidth: true
            }

            Label {
                id: languages_list_label
                font.pointSize: root.pointSize
                wrapMode: Text.WordWrap
                Layout.fillWidth: true
                font.bold: true
            }

            Label {
                text: "This will delete all suttas for these languages. The application will need to restart."
                font.pointSize: root.pointSize
                wrapMode: Text.WordWrap
                Layout.fillWidth: true
                color: palette.text
            }
        }

        onAccepted: {
            root.perform_removal();
        }
    }

    // Completion dialog after successful removal
    Dialog {
        id: completion_dialog
        title: "Languages Removed"
        anchors.centerIn: parent
        modal: true
        standardButtons: Dialog.NoButton

        ColumnLayout {
            spacing: 10
            width: parent.width

            Label {
                text: "Languages have been successfully removed from the database."
                font.pointSize: root.pointSize
                wrapMode: Text.WordWrap
                Layout.fillWidth: true
            }

            Label {
                text: "The application will now quit. Please restart to apply changes."
                font.pointSize: root.pointSize
                wrapMode: Text.WordWrap
                Layout.fillWidth: true
                font.bold: true
            }

            Button {
                text: "Quit Application"
                Layout.alignment: Qt.AlignHCenter
                onClicked: {
                    Qt.quit();
                }
            }
        }
    }

    // Download started dialog
    Dialog {
        id: download_started_dialog
        title: "Download Started"
        anchors.centerIn: parent
        modal: true
        standardButtons: Dialog.Ok

        ColumnLayout {
            spacing: 10

            Label {
                text: "Language downloads have been started in the background."
                font.pointSize: root.pointSize
                wrapMode: Text.WordWrap
                Layout.fillWidth: true
            }

            Label {
                text: "You can close this window and continue using the application. Downloads will continue in the background."
                font.pointSize: root.pointSize
                wrapMode: Text.WordWrap
                Layout.fillWidth: true
            }

            Label {
                text: "The application will need to restart after downloads complete to use the new languages."
                font.pointSize: root.pointSize
                wrapMode: Text.WordWrap
                Layout.fillWidth: true
                color: palette.mid
            }
        }
    }

    // Error dialog
    Dialog {
        id: error_dialog
        title: "Error"
        anchors.centerIn: parent
        modal: true
        standardButtons: Dialog.Ok

        property string error_message: ""

        ColumnLayout {
            spacing: 10
            width: 400

            Label {
                text: error_dialog.error_message
                font.pointSize: root.pointSize
                wrapMode: Text.WordWrap
                Layout.fillWidth: true
            }
        }
    }

    // Processing indicator
    BusyIndicator {
        id: busy_indicator
        anchors.centerIn: parent
        running: false
        visible: running
        z: 100
    }

    Label {
        id: processing_label
        anchors.top: busy_indicator.bottom
        anchors.horizontalCenter: parent.horizontalCenter
        anchors.topMargin: 10
        text: "Removing languages..."
        font.pointSize: root.pointSize
        visible: busy_indicator.running
        z: 100
    }

    ScrollView {
        id: scroll_view
        anchors.fill: parent
        anchors.margins: 10
        contentWidth: availableWidth
        clip: true

        ColumnLayout {
            width: scroll_view.availableWidth
            spacing: 20

            // Download Languages Section
            ColumnLayout {
                Layout.fillWidth: true
                spacing: 10

                Label {
                    text: "Download Languages"
                    font.pointSize: root.largePointSize
                    font.bold: true
                }

                Label {
                    text: "Select additional language translations to download. You can download the same language again to update to the latest version."
                    font.pointSize: root.pointSize
                    wrapMode: Text.WordWrap
                    Layout.fillWidth: true
                }

                Label {
                    text: "Note: Each language database is approximately 10-50 MB compressed."
                    font.pointSize: root.pointSize
                    wrapMode: Text.WordWrap
                    Layout.fillWidth: true
                    color: palette.mid
                }

                LanguageListSelector {
                    id: download_selector
                    Layout.fillWidth: true
                    model: root.available_languages
                    section_title: "Select Languages to Download"
                    instruction_text: "Type language codes below, or click languages to select/unselect them."
                    placeholder_text: "E.g.: de, fr, es"
                    available_label: "Available languages (click to select):"
                    show_count_column: false
                    font_point_size: root.pointSize
                }

                Button {
                    text: "Download Selected Languages"
                    enabled: download_selector.get_selected_languages().length > 0
                    Layout.alignment: Qt.AlignRight
                    onClicked: {
                        root.start_download();
                    }
                }
            }

            // Separator
            /* Rectangle { */
            /*     Layout.fillWidth: true */
            /*     Layout.preferredHeight: 1 */
            /*     color: palette.mid */
            /* } */

            // Remove Languages Section
            ColumnLayout {
                Layout.fillWidth: true
                spacing: 10

                Label {
                    text: "Remove Languages"
                    font.pointSize: root.largePointSize
                    font.bold: true
                }

                Label {
                    text: "Remove language databases you no longer need to free disk space."
                    font.pointSize: root.pointSize
                    wrapMode: Text.WordWrap
                    Layout.fillWidth: true
                }

                Label {
                    text: "Note: English and Pāli cannot be removed as they are core languages required by the application."
                    font.pointSize: root.pointSize
                    wrapMode: Text.WordWrap
                    Layout.fillWidth: true
                    color: palette.mid
                }

                LanguageListSelector {
                    id: removal_selector
                    Layout.fillWidth: true
                    model: {
                        // Filter out "en" and "pli" from installed languages
                        return root.installed_languages_with_counts.filter(function(lang) {
                            return !lang.startsWith("en|") && !lang.startsWith("pli|");
                        });
                    }
                    section_title: "Select Languages to Remove"
                    instruction_text: "Type language codes below, or click languages to select/unselect them."
                    placeholder_text: "E.g.: de, fr, es"
                    available_label: "Installed languages (click to select):"
                    show_count_column: true
                    font_point_size: root.pointSize
                }

                Button {
                    text: "Remove Selected Languages"
                    enabled: removal_selector.get_selected_languages().length > 0
                    Layout.alignment: Qt.AlignRight
                    onClicked: {
                        root.show_removal_confirmation();
                    }
                }
            }

            // Bottom spacer
            Item {
                Layout.fillHeight: true
            }

            // Button layout
            RowLayout {
                visible: root.is_desktop
                Layout.fillWidth: true
                spacing: 10

                Button {
                    text: "Close"
                    onClicked: {
                        root.close();
                    }
                }

                Item {
                    Layout.fillWidth: true
                }
            }

            ColumnLayout {
                visible: root.is_mobile
                Layout.fillWidth: true
                spacing: 10

                Button {
                    text: "Close"
                    Layout.fillWidth: true
                    onClicked: {
                        root.close();
                    }
                }
            }
        }
    }

    function start_download() {
        const selected_codes = download_selector.get_selected_languages();

        if (selected_codes.length === 0) {
            return;
        }

        // Build URLs for selected languages
        const base_url = "https://github.com/simsapa/simsapa-ng-assets/releases/download/v0.1.5/suttas_lang_";
        let urls = [];

        for (let i = 0; i < selected_codes.length; i++) {
            urls.push(base_url + selected_codes[i] + ".tar.bz2");
        }

        console.log("Starting download for:", urls);

        // Start download directly using AssetManager
        // is_initial_setup = false, so it won't download base files
        manager.download_urls_and_extract(urls, false);

        // Show completion message
        download_started_dialog.open();
    }

    function show_removal_confirmation() {
        const selected_codes = removal_selector.get_selected_languages();

        if (selected_codes.length === 0) {
            return;
        }

        // Build language names list for confirmation dialog
        let language_names = [];
        for (let i = 0; i < selected_codes.length; i++) {
            const code = selected_codes[i];
            // Find the full label for this code
            for (let j = 0; j < root.installed_languages_with_counts.length; j++) {
                const label = root.installed_languages_with_counts[j];
                if (label.startsWith(code + "|")) {
                    const parts = label.split("|");
                    if (parts.length >= 2) {
                        language_names.push(parts[1] + " (" + code + ")");
                    }
                    break;
                }
            }
        }

        confirm_removal_dialog.languages_to_remove = selected_codes;
        languages_list_label.text = language_names.join(", ");
        confirm_removal_dialog.open();
    }

    function perform_removal() {
        const codes_to_remove = confirm_removal_dialog.languages_to_remove;

        if (codes_to_remove.length === 0) {
            return;
        }

        busy_indicator.running = true;
        processing_label.text = "Removing languages...";

        // Call the backend to remove languages (runs in background thread)
        // The Connections handler above will receive onRemovalCompleted signal
        manager.remove_sutta_languages(codes_to_remove);
    }
}
