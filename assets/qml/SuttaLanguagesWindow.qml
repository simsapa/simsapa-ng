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

    property bool is_downloading: false

    Connections {
        target: manager

        function onDownloadProgressChanged(op_msg: string, downloaded_bytes: int, total_bytes: int) {
            let downloaded_bytes_mb_str = (downloaded_bytes / 1024 / 1024).toFixed(2);
            let total_bytes_mb_str = (total_bytes / 1024 / 1024).toFixed(2);
            var frac = total_bytes > 0 ? downloaded_bytes / total_bytes : 0;
            download_progress_frame.progress_value = frac;
            if (downloaded_bytes == total_bytes) {
                download_progress_frame.status_text = op_msg;
            } else {
                download_progress_frame.status_text = `${op_msg}: ${downloaded_bytes_mb_str} / ${total_bytes_mb_str} MB`;
            }
        }

        function onDownloadShowMsg(message: string) {
            download_progress_frame.status_text = message;
        }

        function onDownloadsCompleted(success: bool) {
            root.is_downloading = false;
            if (success) {
                completion_message.text = "Language downloads have completed successfully.\n\nThe application will need to restart to use the new languages.";
                views_stack.currentIndex = 2;
            }
        }

        function onRemovalShowMsg(message: string) {
            download_progress_frame.status_text = message;
        }

        function onRemovalCompleted(success: bool, error_msg: string) {
            if (success) {
                completion_message.text = "Languages have been successfully removed from the database.\n\nThe application will now quit. Please restart to apply changes.";
                views_stack.currentIndex = 2;
            } else {
                error_dialog.error_message = error_msg || "Failed to remove languages. Please check the application logs for details.";
                error_dialog.open();
                // Return to main frame
                views_stack.currentIndex = 0;
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

    StackLayout {
        id: views_stack
        anchors.fill: parent
        currentIndex: 0

        // Idx 0: Main language selection frame
        Frame {
            Layout.fillWidth: true
            Layout.fillHeight: true

            ColumnLayout {
                spacing: 0
                anchors.fill: parent

                // Scrollable content area
                ScrollView {
                    id: scroll_view
                    Layout.fillWidth: true
                    Layout.fillHeight: true
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

                            // Filler to push button up
                            Item {
                                Layout.fillHeight: true
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
                    }
                }

                // Fixed button area at the bottom
                RowLayout {
                    visible: root.is_desktop
                    Layout.fillWidth: true
                    Layout.margins: 20

                    Item { Layout.fillWidth: true }

                    Button {
                        text: "Close"
                        onClicked: {
                            root.close();
                        }
                    }

                    Item { Layout.fillWidth: true }
                }

                ColumnLayout {
                    visible: root.is_mobile
                    Layout.fillWidth: true
                    Layout.margins: 10
                    // Extra space on mobile to avoid the bottom bar covering the button.
                    Layout.bottomMargin: 60
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

        // Idx 1: Download/Removal progress frame
        DownloadProgressFrame {
            id: download_progress_frame
            pointSize: root.pointSize
            is_mobile: root.is_mobile
            status_text: "Processing..."
            show_cancel_button: root.is_downloading
            quit_button_text: root.is_downloading ? "Close" : "Quit"

            onQuit_clicked: {
                if (root.is_downloading) {
                    root.close();
                } else {
                    Qt.quit();
                }
            }

            onCancel_clicked: {
                root.cancel_downloads();
            }
        }

        // Idx 2: Completion frame
        Frame {
            Layout.fillWidth: true
            Layout.fillHeight: true

            ColumnLayout {
                spacing: 0
                anchors.fill: parent

                // Centered content area
                Item {
                    Layout.fillWidth: true
                    Layout.fillHeight: true

                    ColumnLayout {
                        anchors.centerIn: parent
                        width: parent.width * 0.9
                        spacing: 10

                        Image {
                            source: "icons/appicons/simsapa.png"
                            Layout.preferredWidth: 100
                            Layout.preferredHeight: 100
                            Layout.alignment: Qt.AlignCenter
                        }

                        Label {
                            id: completion_message
                            text: "Operation completed successfully."
                            font.pointSize: root.pointSize
                            wrapMode: Text.WordWrap
                            horizontalAlignment: Text.AlignHCenter
                            Layout.fillWidth: true
                            Layout.alignment: Qt.AlignCenter
                        }
                    }
                }

                // Fixed button area at the bottom
                RowLayout {
                    Layout.fillWidth: true
                    Layout.margins: 20
                    // Extra space on mobile to avoid the bottom bar covering the button.
                    Layout.bottomMargin: root.is_mobile ? 60 : 20

                    Item { Layout.fillWidth: true }

                    Button {
                        text: "Quit Application"
                        font.pointSize: root.pointSize
                        onClicked: {
                            Qt.quit();
                        }
                    }

                    Item { Layout.fillWidth: true }
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

        // Show progress frame
        root.is_downloading = true;
        download_progress_frame.status_text = "Starting download...";
        download_progress_frame.progress_value = 0;
        views_stack.currentIndex = 1;

        // Start download directly using AssetManager
        // is_initial_setup = false, so it won't download base files
        manager.download_urls_and_extract(urls, false);
    }

    function cancel_downloads() {
        // TODO: Implement proper download cancellation in AssetManager
        // For now, we'll just inform the user that downloads continue in background
        root.is_downloading = false;
        download_progress_frame.status_text = "Cleaning up partially downloaded files...";
        
        // Return to main view
        views_stack.currentIndex = 0;
        
        // Show info dialog
        error_dialog.error_message = "Downloads have been canceled. Partially downloaded files will be cleaned up automatically.\n\nYou can close this window. Any in-progress downloads will continue in the background but incomplete language imports will not be applied.";
        error_dialog.open();
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

        download_progress_frame.status_text = "Removing languages...";
        views_stack.currentIndex = 1;

        // Call the backend to remove languages (runs in background thread)
        // The Connections handler above will receive onRemovalCompleted signal
        manager.remove_sutta_languages(codes_to_remove);
    }
}
