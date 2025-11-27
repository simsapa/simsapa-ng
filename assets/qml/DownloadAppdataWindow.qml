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

    Logger { id: logger }

    Component.onCompleted: {
        if (root.is_mobile) {
            manager.acquire_wake_lock_rust();
        }

        // Initialize language selection
        init_add_languages = manager.get_init_languages();
        available_languages = manager.get_available_languages();

        // Parse init languages and set selected_languages
        if (init_add_languages !== "") {
            add_languages_input.text = init_add_languages;
            sync_selection_from_input();
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

    AssetManager { id: manager }

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
        add_languages_input.text = root.selected_languages.join(", ");
    }

    function parse_language_input() {
        const text = add_languages_input.text.toLowerCase().trim();
        if (text === "" || text === "*") {
            return [];
        }
        return text.replace(/,/g, ' ').replace(/  +/g, ' ').split(' ');
    }

    function sync_selection_from_input() {
        root.selected_languages = parse_language_input();
    }

    StorageDialog { id: storage_dialog }

    Connections {
        target: manager

        function onDownloadProgressChanged(op_msg: string, downloaded_bytes: int, total_bytes: int) {
            let downloaded_bytes_mb_str = (downloaded_bytes / 1024 / 1024).toFixed(2);
            let total_bytes_mb_str = (total_bytes / 1024 / 1024).toFixed(2);
            var frac = total_bytes > 0 ? downloaded_bytes / total_bytes : 0;
                                         progress_bar.value = frac;
            if (downloaded_bytes == total_bytes) {
                download_status.text = op_msg;
            } else {
                download_status.text = `${op_msg}: ${downloaded_bytes_mb_str} / ${total_bytes_mb_str} MB`;
            }
        }

        function onDownloadShowMsg (message) {
            logger.log("onDownloadShowMsg(): " + message);
            download_status.text = message;
        }

        function onDownloadsCompleted (value: bool) {
            if (value) {
                views_stack.currentIndex = 3;
            }
        }
    }

    function validate_and_run_download() {
        // Check that all entered language codes are available.
        const lang_input = add_languages_input.text.toLowerCase().trim();

        if (lang_input !== "" && lang_input !== "*") {
            const selected_langs = lang_input.replace(/,/g, ' ').replace(/  +/g, ' ').split(' ');

            // Build available languages map
            const available_map = {};
            for (let i = 0; i < root.available_languages.length; i++) {
                const parts = root.available_languages[i].split('|');
                if (parts.length === 2) {
                    available_map[parts[0]] = parts[1];
                }
            }

            for (let i = 0; i < selected_langs.length; i++) {
                const lang = selected_langs[i];
                // Skip base languages
                if (lang === 'en' || lang === 'pli' || lang === 'san') {
                    continue;
                }
                if (!available_map[lang]) {
                    download_status.text = `Language not available: ${lang}`;
                    return;
                }
            }
        }

        root.run_download()
    }

    function run_download() {
        // TODO _run_download_pre_hook

        const github_repo = "simsapa/simsapa-ng-assets";
        let version = "v0.1.5";

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
        const lang_input = add_languages_input.text.toLowerCase().trim();
        let selected_langs = [];

        if (lang_input !== "" && lang_input !== "*") {
            const langs = lang_input.replace(/,/g, ' ').replace(/  +/g, ' ').split(' ');
            selected_langs = langs.filter(lang => !['en', 'pli', 'san'].includes(lang));
        } else if (lang_input === "*") {
            // Get all available language codes
            for (let i = 0; i < root.available_languages.length; i++) {
                const parts = root.available_languages[i].split('|');
                if (parts.length === 2) {
                    selected_langs.push(parts[0]);
                }
            }
        }

        // Add URLs for selected languages
        for (let i = 0; i < selected_langs.length; i++) {
            const lang = selected_langs[i];
            const lang_url = `https://github.com/${github_repo}/releases/download/${version}/suttas_lang_${lang}.tar.bz2`;
            urls.push(lang_url);
        }

        /* logger.log("Show progress bar"); */
        progress_bar.visible = true;

        manager.download_urls_and_extract(urls, root.is_initial_setup);
    }

    StackLayout {
        id: views_stack
        anchors.fill: parent
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
<p>If you need to remove the database, such as after a failed or partial download, read the instructions at:</p>
<p><a href="https://simsapa.github.io/install/uninstall/">https://simsapa.github.io/install/uninstall/</a></p>
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
<p>If you need to remove the database, such as after a failed or partial download, read the instructions at:</p>
<p><a href="https://simsapa.github.io/install/uninstall/">https://simsapa.github.io/install/uninstall/</a></p>
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
                        ColumnLayout {
                            id: language_list_selector
                            Layout.margins: 10
                            spacing: 10

                            Label {
                                text: "Include Languages"
                                font.pointSize: root.pointSize
                                font.bold: true
                            }

                            Label {
                                text: "Type language codes below, or click languages to select/unselect them. Type * to download all."
                                font.pointSize: root.pointSize
                                wrapMode: Text.WordWrap
                                Layout.fillWidth: true
                            }

                            TextField {
                                id: add_languages_input
                                placeholderText: "E.g.: it, fr, pt, th"
                                font.pointSize: root.pointSize
                                text: root.init_add_languages
                                Layout.fillWidth: true
                                onTextChanged: {
                                    // Update selection when user manually edits the input
                                    root.sync_selection_from_input();
                                }
                            }

                            Label {
                                text: "Available languages (click to select):"
                                font.pointSize: root.pointSize
                            }

                            ScrollView {
                                Layout.fillWidth: true
                                Layout.preferredHeight: 150
                                clip: true

                                ListView {
                                    id: languages_listview
                                    model: root.available_languages
                                    spacing: 0

                                    delegate: Rectangle {
                                        id: delegate_item
                                        required property string modelData
                                        required property int index

                                        width: languages_listview.width
                                        height: 30

                                        property string lang_code: {
                                            if (!modelData) return "";
                                            const parts = modelData.split('|');
                                            return parts.length === 2 ? parts[0] : "";
                                        }

                                        property string lang_name: {
                                            if (!modelData) return "";
                                            const parts = modelData.split('|');
                                            return parts.length === 2 ? parts[1] : "";
                                        }

                                        property bool is_selected: root.selected_languages.indexOf(lang_code) > -1

                                        color: is_selected ? palette.highlight : (index % 2 === 0 ? palette.alternateBase : palette.base)

                                        MouseArea {
                                            anchors.fill: parent
                                            cursorShape: Qt.PointingHandCursor
                                            onClicked: {
                                                root.toggle_language_selection(delegate_item.lang_code);
                                            }
                                        }

                                        Row {
                                            spacing: 10
                                            anchors.verticalCenter: parent.verticalCenter
                                            anchors.left: parent.left
                                            anchors.leftMargin: 10

                                            Text {
                                                text: delegate_item.lang_code
                                                font.pointSize: root.pointSize
                                                font.bold: delegate_item.is_selected
                                                color: delegate_item.is_selected ? palette.highlightedText : palette.text
                                                width: 50
                                            }

                                            Text {
                                                text: delegate_item.lang_name
                                                font.pointSize: root.pointSize
                                                font.bold: delegate_item.is_selected
                                                color: delegate_item.is_selected ? palette.highlightedText : palette.text
                                            }
                                        }
                                    }
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
                            views_stack.currentIndex = 2;
                            root.validate_and_run_download();
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
                        text: "Quit"
                        font.pointSize: root.pointSize
                        Layout.fillWidth: true
                        onClicked: Qt.quit()
                    }

                    Button {
                        text: "Select Storage"
                        font.pointSize: root.pointSize
                        Layout.fillWidth: true
                        onClicked: storage_dialog.open()
                    }

                    Button {
                        text: "Download"
                        font.pointSize: root.pointSize
                        Layout.fillWidth: true
                        onClicked: {
                            views_stack.currentIndex = 2;
                            root.validate_and_run_download();
                        }
                    }
                }
            }
        }

        // Idx 2: Download progress
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

                        AnimatedImage {
                            id: simsapa_loading_gif
                            source: "icons/gif/simsapa-loading.gif"
                            playing: true
                            Layout.alignment: Qt.AlignCenter
                        }

                        Label {
                            id: download_status
                            Layout.alignment: Qt.AlignCenter
                            text: "Downloading ..."
                            font.pointSize: root.pointSize
                            wrapMode: Text.WordWrap
                            horizontalAlignment: Text.AlignHCenter
                            Layout.fillWidth: true
                        }

                        ProgressBar {
                            id: progress_bar
                            Layout.alignment: Qt.AlignCenter
                            Layout.fillWidth: true
                            visible: true
                            from: 0
                            to: 1
                            value: 0
                            font.pointSize: root.pointSize
                        }

                        // Text {
                        //     id: download_msg
                        //     text: ""
                        //     textFormat: Text.RichText
                        //     font.pointSize: root.pointSize
                        //     Layout.alignment: Qt.AlignCenter
                        // }
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
                        id: download_quit_button
                        text: "Quit"
                        font.pointSize: root.pointSize
                        onClicked: Qt.quit()
                    }

                    Item { Layout.fillWidth: true }
                }
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
