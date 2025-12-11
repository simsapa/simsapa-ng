pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import QtQuick.Controls
import QtQuick.Window

import com.profoundlabs.simsapa

ApplicationWindow {
    id: root

    title: "Download Application Assets"
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
    readonly property int top_bar_margin: is_mobile ? SuttaBridge.get_mobile_top_bar_margin() : 0

    Logger { id: logger }

    Component.onCompleted: {
        if (root.is_mobile) {
            root.wake_lock_acquired = manager.acquire_wake_lock_rust();
        }

        // Initialize language selection
        init_add_languages = manager.get_init_languages();
        available_languages = manager.get_available_languages();

        // Parse init languages and set selected_languages
        if (init_add_languages !== "") {
            language_list_selector.language_input.text = init_add_languages;
            root.sync_selection_from_input();
        }

        // TODO: Implement checking releases info. See asset_management.py class ReleasesWorker(QRunnable).
        // Assuming there is a network connection, show the download selection screen.
        views_stack.currentIndex = 1;

        if (root.is_mobile) {
            storage_dialog.open();
        }
    }

    Component.onDestruction: {
        if (root.is_mobile) {
            manager.release_wake_lock_rust();
        }
    }

    property bool is_initial_setup: true
    property string init_add_languages: ""
    property var available_languages: []
    property var selected_languages: []
    property bool wake_lock_acquired: false

    AssetManager { id: manager }

    function start_redownload(urls) {
        logger.log(`DownloadAppdataWindow.start_redownload() with ${urls.length} URL(s)`);

        // Show the window
        root.show();
        root.raise();
        root.requestActivate();

        // Set to download progress view (idx 2)
        views_stack.currentIndex = 2;

        // Start the download
        manager.download_urls_and_extract(urls, false);
    }

    function toggle_language_selection(lang_code) {
        let selected = root.selected_languages.slice();
        let index = selected.indexOf(lang_code);

        if (index > -1) {
            // Remove from selection
            selected.splice(index, 1);
        } else {
            // Add to selection
            selected.push(lang_code);
        }

        root.selected_languages = selected;
        update_language_input();
    }

    function update_language_input() {
        language_list_selector.language_input.text = root.selected_languages.join(", ");
    }

    function parse_language_input() {
        const text = language_list_selector.language_input.text.toLowerCase().trim();
        if (text === "") {
            return [];
        }
        return text.replace(/,/g, ' ').replace(/  +/g, ' ').split(' ');
    }

    function sync_selection_from_input() {
        root.selected_languages = parse_language_input();
    }

    StorageDialog { id: storage_dialog }

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

        function onDownloadShowMsg (message) {
            logger.log("onDownloadShowMsg(): " + message);
            download_progress_frame.status_text = message;
        }

        function onDownloadsCompleted (value: bool) {
            if (value) {
                views_stack.currentIndex = 3;
            }
        }
    }

    function validate_language_codes(selected_codes, available_list) {
        // Extract just the language codes from the available list
        let available_codes = [];
        for (let i = 0; i < available_list.length; i++) {
            const parts = available_list[i].split('|');
            if (parts.length >= 1) {
                available_codes.push(parts[0]);
            }
        }

        // Find invalid codes (excluding base languages)
        let invalid_codes = [];
        for (let i = 0; i < selected_codes.length; i++) {
            const code = selected_codes[i];
            // Skip base languages - they are always available
            if (code === 'en' || code === 'pli' || code === 'san') {
                continue;
            }
            if (available_codes.indexOf(code) === -1) {
                invalid_codes.push(code);
            }
        }

        return invalid_codes;
    }

    function validate_download() {
        // Check that all entered language codes are available.
        const lang_input = language_list_selector.language_input.text.toLowerCase().trim();

        if (lang_input !== "") {
            const selected_langs = lang_input.replace(/,/g, ' ').replace(/  +/g, ' ').split(' ');

            // Validate language codes
            const invalid_codes = validate_language_codes(selected_langs, root.available_languages);
            if (invalid_codes.length > 0) {
                error_dialog.error_message = "Not available for download:\n\n" + invalid_codes.join(", ");
                error_dialog.open();
                return false;
            }
        }

        return true;
    }

    function run_download() {
        // TODO _run_download_pre_hook

        const github_repo = "simsapa/simsapa-ng-assets";
        let version = "v0.1.7";

        let urls = [];

        if (root.is_initial_setup) {
            // Include appdata and other database downloads when the app is launched the first time.
            // ensure 'v' prefix
            if (version[0] !== "v") {
                version = "v" + version
            }

            const appdata_tar_url = `https://github.com/${github_repo}/releases/download/${version}/appdata.tar.bz2`;
            const dictionaries_tar_url = `https://github.com/${github_repo}/releases/download/${version}/dictionaries.tar.bz2`;
            const dpd_tar_url = `https://github.com/${github_repo}/releases/download/${version}/dpd.tar.bz2`;

            // Default: General bundle
            urls.push(appdata_tar_url);
            urls.push(dictionaries_tar_url);
            urls.push(dpd_tar_url);
        }

        // Add language databases
        const lang_input = language_list_selector.language_input.text.toLowerCase().trim();
        let selected_langs = [];

        if (lang_input !== "") {
            const langs = lang_input.replace(/,/g, ' ').replace(/  +/g, ' ').split(' ');
            selected_langs = langs.filter(lang => !['en', 'pli', 'san'].includes(lang));
        }

        // Add URLs for selected languages
        for (let i = 0; i < selected_langs.length; i++) {
            const lang = selected_langs[i];
            const lang_url = `https://github.com/${github_repo}/releases/download/${version}/suttas_lang_${lang}.tar.bz2`;
            urls.push(lang_url);
        }

        /* logger.log("Show progress bar"); */
        download_progress_frame.visible = true;

        manager.download_urls_and_extract(urls, root.is_initial_setup);
    }

    StackLayout {
        id: views_stack
        anchors.fill: parent
        anchors.topMargin: root.top_bar_margin
        currentIndex: 0

        // Idx 0: Checking sources
        Frame {
            Layout.fillWidth: true
            Layout.fillHeight: true

            ColumnLayout {
                spacing: 0
                anchors.fill: parent

                // Scrollable content area
                ScrollView {
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    contentWidth: availableWidth
                    clip: true

                    ColumnLayout {
                        width: parent.width
                        spacing: 5

                        Text {
                            textFormat: Text.RichText
                            font.pointSize: root.pointSize
                            color: palette.text
                            wrapMode: Text.WordWrap
                            Layout.fillWidth: true
                            Layout.alignment: Qt.AlignCenter
                            text: `
<style>p { text-align: center; }</style>
<p>The application database was not found on this system.</p>
<p>Checking for available sources to download...<p>
`
                        }

                        Text {
                            visible: root.is_desktop
                            textFormat: Text.RichText
                            font.pointSize: root.pointSize
                            color: palette.text
                            wrapMode: Text.WordWrap
                            Layout.fillWidth: true
                            Layout.alignment: Qt.AlignCenter
                            onLinkActivated: function(link) {
                                /* logger.log(link + " link activated"); */
                                Qt.openUrlExternally(link);
                            }
                            text: `
<style>p { text-align: center; }</style>
<p>See the feature demos for getting started:</p>
<p><a href="https://simsapa.github.io/">https://simsapa.github.io/</a></p>
`

                            // https://blog.shantanu.io/2015/02/15/creating-working-hyperlinks-in-qtquick-text/
                            MouseArea {
                                anchors.fill: parent
                                acceptedButtons: Qt.NoButton // we don't want to eat clicks on the Text
                                cursorShape: parent.hoveredLink ? Qt.PointingHandCursor : Qt.ArrowCursor
                            }
                        }

                        Item { Layout.fillHeight: true }
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
                        text: "Quit"
                        font.pointSize: root.pointSize
                        onClicked: Qt.quit()
                    }

                    Item { Layout.fillWidth: true }
                }
            }
        }

        // Idx 1: Download bundle selection
        Frame {
            Layout.fillWidth: true
            Layout.fillHeight: true

            ColumnLayout {
                spacing: 0
                anchors.fill: parent

                // Scrollable content area
                ScrollView {
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    contentWidth: availableWidth
                    clip: true

                    ColumnLayout {
                        spacing: 5
                        width: parent.width

                        Image {
                            source: "icons/appicons/simsapa.png"
                            Layout.preferredWidth: 100
                            Layout.preferredHeight: 100
                            Layout.alignment: Qt.AlignCenter
                        }

                        Text {
                            textFormat: Text.RichText
                            font.pointSize: root.pointSize
                            color: palette.text
                            wrapMode: Text.WordWrap
                            Layout.fillWidth: true
                            Layout.alignment: Qt.AlignCenter
                            text: `
<style>p { text-align: center; }</style>
<p>The application database was not found on this system.</p>
<p>Please select the sources to download.<p>
`
                        }

                        Text {
                            visible: root.is_desktop
                            textFormat: Text.RichText
                            font.pointSize: root.pointSize
                            color: palette.text
                            wrapMode: Text.WordWrap
                            Layout.fillWidth: true
                            Layout.alignment: Qt.AlignCenter
                            onLinkActivated: function(link) { Qt.openUrlExternally(link); }
                            text: `
<style>p { text-align: center; }</style>
<p>See the feature demos for getting started:</p>
<p><a href="https://simsapa.github.io/">https://simsapa.github.io/</a></p>
`

                            MouseArea {
                                anchors.fill: parent
                                acceptedButtons: Qt.NoButton
                                cursorShape: parent.hoveredLink ? Qt.PointingHandCursor : Qt.ArrowCursor
                            }
                        }

                        ColumnLayout {
                            Layout.margins: 10
                            Layout.fillWidth: true

                            RadioButton {
                                text: "General bundle (always included)"
                                font.pointSize: root.pointSize
                                checked: true
                                enabled: false
                                onClicked: {} // _toggled_general_bundle
                            }

                            Label {
                                text: "Pāli and English + pre-generated search index"
                                font.pointSize: root.pointSize
                            }

                            // RadioButton {
                            //     text: "Include additional texts"
                            //     checked: false
                            //     enabled: false
                            // }
                        }

                        // Language selection section
                        LanguageListSelector {
                            id: language_list_selector
                            Layout.margins: 10
                            model: root.available_languages
                            selected_languages: root.selected_languages
                            section_title: "Include Languages"
                            instruction_text: "Type language codes below, or click to select/unselect."
                            placeholder_text: "E.g.: it, fr, pt, th"
                            available_label: "Available languages (click to select):"
                            show_count_column: true
                            font_point_size: root.pointSize

                            onLanguageSelectionChanged: function(selected_codes) {
                                root.selected_languages = selected_codes;
                            }

                            Component.onCompleted: {
                                // Initialize with existing selection
                                if (root.init_add_languages !== "") {
                                    root.sync_selection_from_input();
                                }
                            }
                        }
                    }
                }

                // Fixed button area at the bottom
                RowLayout {
                    id: horizontal_buttons
                    visible: root.is_desktop
                    Layout.fillWidth: true
                    Layout.margins: 20

                    Item { Layout.fillWidth: true }

                    Button {
                        text: "Quit"
                        font.pointSize: root.pointSize
                        onClicked: Qt.quit()
                    }

                    Item { Layout.fillWidth: true }

                    Button {
                        text: "Download"
                        font.pointSize: root.pointSize
                        onClicked: {
                            // Validate first, only proceed if validation passes
                            if (root.validate_download()) {
                                views_stack.currentIndex = 2;
                                root.run_download();
                            }
                        }
                    }

                    Item { Layout.fillWidth: true }
                }

                ColumnLayout {
                    id: vertical_buttons
                    visible: root.is_mobile
                    Layout.fillWidth: true
                    Layout.margins: 10
                    // Extra space on mobile to avoid the bottom bar covering the button.
                    Layout.bottomMargin: 60
                    spacing: 10

                    Button {
                        text: "Download"
                        font.pointSize: root.pointSize
                        Layout.fillWidth: true
                        onClicked: {
                            // Validate first, only proceed if validation passes
                            if (root.validate_download()) {
                                views_stack.currentIndex = 2;
                                root.run_download();
                            }
                        }
                    }

                    Button {
                        text: "Select Storage"
                        font.pointSize: root.pointSize
                        Layout.fillWidth: true
                        onClicked: storage_dialog.open()
                    }

                    Button {
                        text: "Quit"
                        font.pointSize: root.pointSize
                        Layout.fillWidth: true
                        onClicked: Qt.quit()
                    }
                }
            }
        }

        // Idx 2: Download progress
        DownloadProgressFrame {
            id: download_progress_frame
            pointSize: root.pointSize
            is_mobile: root.is_mobile
            status_text: "Downloading ..."
            wake_lock_acquired: root.wake_lock_acquired

            onQuit_clicked: {
                Qt.quit();
            }
        }

        // Idx 3: Completed
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

                        Text {
                            text: `
<style>p { text-align: center; }</style>
<p>Completed.</p>
<p>Quit and start the application again.</p>`
                            textFormat: Text.RichText
                            font.pointSize: root.pointSize
                            color: palette.text
                            wrapMode: Text.WordWrap
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
                        id: completed_quit_button
                        text: "Quit"
                        font.pointSize: root.pointSize
                        onClicked: Qt.quit()
                    }

                    Item { Layout.fillWidth: true }
                }
            }
        }

    }
}
