pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import QtQuick.Controls
import QtQuick.Window

import com.profoundlabs.simsapa

ApplicationWindow {
    id: root
    title: "Database Validation"
    width: is_mobile ? Screen.desktopAvailableWidth : 600
    height: is_mobile ? Screen.desktopAvailableHeight : 400
    visible: false
    color: palette.window
    flags: Qt.Dialog
    modality: Qt.ApplicationModal

    Logger { id: logger }

    readonly property bool is_mobile: Qt.platform.os === "android" || Qt.platform.os === "ios"
    readonly property bool is_desktop: !root.is_mobile
    readonly property int pointSize: is_mobile ? 14 : 12
    required property int top_bar_margin

    // Properties to track validation results for each database
    property var validation_results: ({})

    // Properties to track which databases failed
    property bool appdata_failed: false
    property bool dpd_failed: false
    property bool dictionaries_failed: false
    property bool userdata_failed: false

    // Computed properties
    readonly property bool has_downloadable_failures: appdata_failed || dpd_failed || dictionaries_failed
    readonly property bool has_userdata_failure: userdata_failed
    readonly property bool has_any_failure: has_downloadable_failures || has_userdata_failure
    readonly property bool has_both_types: has_downloadable_failures && has_userdata_failure

    // Track if dialog was opened from menu (manual) vs automatic validation failure
    property bool opened_from_menu: false

    function show_from_menu() {
        root.opened_from_menu = true;
        // Clear previous results and run validation
        root.validation_results = {};
        root.appdata_failed = false;
        root.dpd_failed = false;
        root.dictionaries_failed = false;
        root.userdata_failed = false;

        root.show();
        root.raise();
        root.requestActivate();

        // Run validation checks
        root.run_validation_checks();
    }

    function run_validation_checks() {
        logger.log("run_validation_checks()");
        // Clear previous results
        root.validation_results = {};
        root.appdata_failed = false;
        root.dpd_failed = false;
        root.dictionaries_failed = false;
        root.userdata_failed = false;

        // Run the first query checks which will emit validation signals
        SuttaBridge.appdata_first_query();
        SuttaBridge.dpd_first_query();
        SuttaBridge.dictionary_first_query();
        SuttaBridge.userdata_first_query();
    }

    function show_validation_failure(failed_databases) {
        // Parse the failed_databases string (comma-separated list)
        root.appdata_failed = failed_databases.includes("appdata");
        root.dpd_failed = failed_databases.includes("dpd");
        root.dictionaries_failed = failed_databases.includes("dictionaries");
        root.userdata_failed = failed_databases.includes("userdata");

        root.show();
        root.raise();
        root.requestActivate();
    }

    function set_validation_results(results) {
        root.validation_results = results;
        // Update failed flags based on results
        root.appdata_failed = results["appdata"] && !results["appdata"].is_valid;
        root.dpd_failed = results["dpd"] && !results["dpd"].is_valid;
        root.dictionaries_failed = results["dictionaries"] && !results["dictionaries"].is_valid;
        root.userdata_failed = results["userdata"] && !results["userdata"].is_valid;
    }

    function get_failed_downloadable_list() {
        let failed = [];
        if (root.appdata_failed) {
            let msg = root.validation_results["appdata"] ? root.validation_results["appdata"].message : "";
            failed.push({name: "Appdata Database", message: msg});
        }
        if (root.dpd_failed) {
            let msg = root.validation_results["dpd"] ? root.validation_results["dpd"].message : "";
            failed.push({name: "DPD Database", message: msg});
        }
        if (root.dictionaries_failed) {
            let msg = root.validation_results["dictionaries"] ? root.validation_results["dictionaries"].message : "";
            failed.push({name: "Dictionaries Database", message: msg});
        }
        return failed;
    }

    function get_userdata_message() {
        if (root.validation_results["userdata"]) {
            return root.validation_results["userdata"].message;
        }
        return "";
    }

    // Returns true if download was started, false if no databases to download
    function handle_redownload(): bool {
        logger.log("handle_redownload()");
        // Build list of failed downloadable databases
        let urls = [];
        const github_repo = SuttaBridge.get_compatible_asset_github_repo();
        let version = SuttaBridge.get_compatible_asset_version_tag();

        // ensure 'v' prefix
        if (version[0] !== "v") {
            version = "v" + version;
        }

        // Check which databases failed and add their URLs
        if (root.validation_results["appdata"] && !root.validation_results["appdata"].is_valid) {
            const appdata_tar_url = `https://github.com/${github_repo}/releases/download/${version}/appdata.tar.bz2`;
            urls.push(appdata_tar_url);
            logger.log("Adding appdata to re-download list");
        }

        if (root.validation_results["dpd"] && !root.validation_results["dpd"].is_valid) {
            const dpd_tar_url = `https://github.com/${github_repo}/releases/download/${version}/dpd.tar.bz2`;
            urls.push(dpd_tar_url);
            logger.log("Adding dpd to re-download list");
        }

        if (root.validation_results["dictionaries"] && !root.validation_results["dictionaries"].is_valid) {
            const dictionaries_tar_url = `https://github.com/${github_repo}/releases/download/${version}/dictionaries.tar.bz2`;
            urls.push(dictionaries_tar_url);
            logger.log("Adding dictionaries to re-download list");
        }

        if (urls.length > 0) {
            // Open DownloadAppdataWindow to handle the re-download
            // Note: We pass is_initial_setup = false
            logger.log(`Starting re-download of ${urls.length} database(s)`);
            download_window.start_redownload(urls);
            return true;
        } else {
            // No failed databases to re-download
            no_failed_databases_dialog.open();
            return false;
        }
    }

    function handle_reset_userdata() {
        logger.log("handle_reset_userdata()");

        const success = SuttaBridge.reset_userdata_database();

        if (success) {
            logger.log("Userdata database reset complete");
            reset_success_dialog.open();
        } else {
            logger.log("ERROR: Failed to reset userdata database");
            reset_error_dialog.open();
        }
    }

    function handle_remove_all_and_redownload() {
        logger.log("handle_remove_all_and_redownload()");
        SuttaBridge.prepare_for_database_upgrade();
        remove_all_success_dialog.open();
    }

    Dialog {
        id: reset_success_dialog
        title: "Userdata Reset Complete"
        anchors.centerIn: parent
        modal: true
        standardButtons: Dialog.Ok

        Label {
            text: "Userdata has been reset to defaults.\n\nQuit and restart the app."
            wrapMode: Text.WordWrap
            width: 400
        }

        onAccepted: {
            Qt.quit();
        }
    }

    Dialog {
        id: reset_error_dialog
        title: "Userdata Reset Failed"
        anchors.centerIn: parent
        modal: true
        standardButtons: Dialog.Ok

        Label {
            text: "Failed to reset userdata database."
            wrapMode: Text.WordWrap
            width: 400
        }
    }

    Dialog {
        id: no_failed_databases_dialog
        title: "No Failed Databases"
        anchors.centerIn: parent
        modal: true
        standardButtons: Dialog.Ok

        Label {
            text: "There are no failed databases to re-download."
            wrapMode: Text.WordWrap
            width: 400
        }
    }

    Dialog {
        id: remove_all_success_dialog
        title: "Ready for Re-download"
        anchors.centerIn: parent
        modal: true
        standardButtons: Dialog.Ok

        Label {
            text: "Start the app again to begin the database download."
            wrapMode: Text.WordWrap
            width: 400
        }

        onAccepted: {
            Qt.quit();
        }
    }

    DownloadAppdataWindow {
        id: download_window
        visible: false
        is_initial_setup: false
    }

    // Expected database names
    readonly property var expected_databases: ["appdata", "dpd", "dictionaries", "userdata"]

    // Function to check if all validations have completed
    function all_validations_completed() {
        for (let i = 0; i < root.expected_databases.length; i++) {
            if (!(root.expected_databases[i] in root.validation_results)) {
                return false;
            }
        }
        return true;
    }

    // Function to check if any validations failed
    function has_validation_failures() {
        for (let db_name in root.validation_results) {
            if (!root.validation_results[db_name].is_valid) {
                return true;
            }
        }
        return false;
    }

    Connections {
        target: SuttaBridge
        function onDatabaseValidationResult(database_name, is_valid, message) {
            // Store result in hashmap
            root.validation_results[database_name] = {
                is_valid: is_valid,
                message: message
            };

            // Update failed flags
            root.set_validation_results(root.validation_results);

            // Check if all validations are complete
            if (root.all_validations_completed()) {
                // All checks completed - cancel timeout
                validation_timeout_timer.stop();

                // If opened from menu, dialog is already visible - just update UI
                // If not from menu (automatic check), show dialog only if there were failures
                if (!root.opened_from_menu && root.has_validation_failures()) {
                    root.show_validation_dialog();
                }
            } else {
                // Not all checks completed yet - start/restart timeout
                validation_timeout_timer.restart();
            }
        }
    }

    function show_validation_dialog() {
        // Pass validation results to dialog
        root.set_validation_results(root.validation_results);

        // Build comma-separated list of failed databases for the dialog
        let failed_list = [];
        for (let db_name in root.validation_results) {
            if (!root.validation_results[db_name].is_valid) {
                failed_list.push(db_name);
            }
        }
        if (failed_list.length > 0) {
            root.show_validation_failure(failed_list.join(","));
        }
    }

    Timer {
        id: validation_timeout_timer
        interval: 5000 // 5 second timeout
        repeat: false
        onTriggered: {
            // Timeout reached - some checks haven't completed
            // Show dialog if we have any failures OR if some results are missing
            if (root.has_validation_failures() || !root.all_validations_completed()) {
                root.show_validation_dialog();
            }
        }
    }


    Item {
        x: 10
        y: 10 + root.top_bar_margin
        implicitWidth: root.width - 20
        implicitHeight: root.height - 20 - root.top_bar_margin

        ColumnLayout {
            spacing: 15
            anchors.fill: parent

            // Re-run Validation Checks button at the top
            Button {
                text: "Re-run Validation Checks"
                Layout.alignment: Qt.AlignLeft
                onClicked: {
                    root.run_validation_checks();
                }
            }

            // Status title
            Label {
                text: "Status:"
                font.bold: true
                font.pointSize: root.pointSize + 2
            }

            // Success message when no failures
            Label {
                text: "Database checks were successful."
                font.pointSize: root.pointSize
                visible: root.all_validations_completed() && !root.has_any_failure
            }

            // Failure message header
            Label {
                text: "Database checks failed for:"
                font.pointSize: root.pointSize
                visible: root.has_any_failure
            }

            // Downloadable databases section
            ColumnLayout {
                spacing: 10
                visible: root.has_downloadable_failures
                Layout.fillWidth: true

                Label {
                    text: "The following database(s) may be incomplete or corrupted and may need to be re-downloaded:"
                    font.pointSize: root.pointSize
                    wrapMode: Text.WordWrap
                    Layout.fillWidth: true
                }

                Repeater {
                    model: root.get_failed_downloadable_list()
                    delegate: ColumnLayout {
                        id: delegate_item
                        required property string name
                        required property string message
                        spacing: 2
                        Layout.fillWidth: true
                        Label {
                            text: "  - " + delegate_item.name
                            font.pointSize: root.pointSize
                            font.bold: true
                            Layout.fillWidth: true
                        }
                        Label {
                            text: "    " + delegate_item.message
                            font.pointSize: root.pointSize - 1
                            color: palette.mid
                            Layout.fillWidth: true
                            wrapMode: Text.WordWrap
                            visible: delegate_item.message !== ""
                        }
                    }
                }
            }

            // Userdata section
            ColumnLayout {
                spacing: 10
                visible: root.has_userdata_failure
                Layout.fillWidth: true

                Label {
                    text: "The userdata database may be corrupted. You can reset it to default settings."
                    font.pointSize: root.pointSize
                    wrapMode: Text.WordWrap
                    Layout.fillWidth: true
                    color: palette.text
                }

                Label {
                    text: "  - Userdata Database"
                    font.pointSize: root.pointSize
                    font.bold: true
                    Layout.fillWidth: true
                }

                Label {
                    text: "    " + root.get_userdata_message()
                    font.pointSize: root.pointSize - 1
                    color: palette.mid
                    Layout.fillWidth: true
                    wrapMode: Text.WordWrap
                    visible: root.get_userdata_message() !== ""
                }

                Label {
                    text: "WARNING: Resetting userdata will erase all your bookmarks, notes, and custom settings."
                    font.pointSize: root.pointSize
                    font.bold: true
                    wrapMode: Text.WordWrap
                    Layout.fillWidth: true
                    color: "#ff6b6b"
                }
            }

            Item { Layout.fillHeight: true }

            ColumnLayout {
                spacing: 10
                Layout.fillWidth: true
                // Extra space on mobile to avoid the bottom bar covering the buttons
                Layout.bottomMargin: root.is_mobile ? 60 : 5

                Button {
                    text: "Re-download Failed Databases"
                    font.pointSize: root.pointSize
                    Layout.fillWidth: true
                    onClicked: {
                        if (root.handle_redownload()) {
                            root.close();
                        }
                    }
                }

                Button {
                    text: "Reset Userdata"
                    font.pointSize: root.pointSize
                    Layout.fillWidth: true
                    onClicked: {
                        root.handle_reset_userdata();
                        root.close();
                    }
                }

                Button {
                    text: "Remove All and Re-download"
                    font.pointSize: root.pointSize
                    Layout.fillWidth: true
                    onClicked: {
                        root.handle_remove_all_and_redownload();
                    }
                }

                Button {
                    text: "Close"
                    font.pointSize: root.pointSize
                    Layout.fillWidth: true
                    onClicked: {
                        root.opened_from_menu = false;
                        root.close();
                    }
                }
            }
        }
    }
}
