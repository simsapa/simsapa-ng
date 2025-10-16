pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import QtQuick.Controls
import QtQuick.Window

import com.profoundlabs.simsapa

ApplicationWindow {
    id: root

    title: "Download Application Assets"
    width: is_mobile ? Screen.desktopAvailableWidth : 500
    height: is_mobile ? Screen.desktopAvailableHeight : 700
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

    property bool include_appdata_downloads: true

    AssetManager { id: manager }

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
            download_status.text = message;
        }

        function onDownloadsCompleted (value: bool) {
            if (value) {
                views_stack.currentIndex = 3;
            }
        }
    }

    function validate_and_run_download() {
        // TODO Check that all entered language codes are available.
        root.run_download()
    }

    function run_download() {
        // TODO _run_download_pre_hook

        const github_repo = "simsapa/simsapa-ng-assets";
        let version = "v0.1.0-alpha.1";

        let urls = [];

        if (root.include_appdata_downloads) {
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

            /* logger.log("Show progress bar"); */
            progress_bar.visible = true;

            manager.download_urls_and_extract(urls);
        }

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
                spacing: 10
                anchors.fill: parent

                Text {
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
<p>The application database<br>was not found on this system.</p>
<p>Checking for available sources to download...<p>
<p></p>
<p>If you need to remove the database, such as after a failed or partial download,<br>read the instructions at:</p>
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

                Button {
                    Layout.alignment: Qt.AlignCenter
                    text: "Quit"
                    onClicked: Qt.quit()
                }
            }
        }

        // Idx 1: Download bundle selection
        Frame {
            Layout.fillWidth: true
            Layout.fillHeight: true

            ColumnLayout {
                spacing: 10
                anchors.fill: parent

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
                    onLinkActivated: function(link) { Qt.openUrlExternally(link); }
                    text: `
<style>p { text-align: center; }</style>
<p>The application database<br>was not found on this system.</p>
<p>Please select the sources to download.<p>
<p></p>
<p>If you need to remove the database, such as after a failed or partial download,<br>read the instructions at:</p>
<p><a href="https://simsapa.github.io/install/uninstall/">https://simsapa.github.io/install/uninstall/</a></p>
`

                    MouseArea {
                        anchors.fill: parent
                        acceptedButtons: Qt.NoButton
                        cursorShape: parent.hoveredLink ? Qt.PointingHandCursor : Qt.ArrowCursor
                    }
                }

                ColumnLayout {
                    Layout.margins: 20

                    RadioButton {
                        text: "General bundle"
                        font.pointSize: root.pointSize
                        checked: true
                        enabled: false
                        onClicked: {} // _toggled_general_bundle
                    }

                    Label {
                        text: "Pāli and English + pre-generated search index"
                        font.pointSize: root.pointSize
                    }
                    Label { text: ""; font.pointSize: root.pointSize }

                    Label {
                        text: "(Note: Choices for sutta translations are coming later.)"
                        font.pointSize: root.pointSize
                    }

                    // RadioButton {
                    //     text: "Include additional texts"
                    //     checked: false
                    //     enabled: false
                    // }
                }

                Item { Layout.fillHeight: true }

                RowLayout {
                    id: horizontal_buttons
                    visible: root.is_desktop
                    Layout.margins: 20

                    Item { Layout.fillWidth: true }

                    Button {
                        text: "Quit"
                        font.pointSize: root.is_mobile ? root.largePointSize : root.pointSize
                        onClicked: Qt.quit()
                    }

                    Item { Layout.fillWidth: true }

                    Button {
                        text: "Select Storage"
                        visible: root.is_mobile
                        font.pointSize: root.is_mobile ? root.largePointSize : root.pointSize
                        onClicked: storage_dialog.open()
                    }

                    Item { Layout.fillWidth: true }

                    Button {
                        text: "Download"
                        font.pointSize: root.is_mobile ? root.largePointSize : root.pointSize
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
                    Layout.margins: 20

                    Button {
                        text: "Quit"
                        font.pointSize: root.is_mobile ? root.largePointSize : root.pointSize
                        onClicked: Qt.quit()
                    }

                    Button {
                        text: "Select Storage"
                        visible: root.is_mobile
                        font.pointSize: root.is_mobile ? root.largePointSize : root.pointSize
                        onClicked: storage_dialog.open()
                    }

                    Button {
                        text: "Download"
                        font.pointSize: root.is_mobile ? root.largePointSize : root.pointSize
                        onClicked: {
                            views_stack.currentIndex = 2;
                            root.validate_and_run_download();
                        }
                    }
                }

                Item {
                    visible: root.is_mobile
                    Layout.fillHeight: true
                }
            }
        }

        // Idx 2: Download progress
        Frame {
            Layout.fillWidth: true
            Layout.fillHeight: true

            ColumnLayout {
                spacing: 10
                anchors.fill: parent

                Item { Layout.fillHeight: true }

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
                }

                ProgressBar {
                    id: progress_bar
                    Layout.alignment: Qt.AlignCenter
                    Layout.preferredWidth: parent.width * 0.9
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

                RowLayout {
                    Layout.margins: 20

                    Item { Layout.fillWidth: true }

                    Button {
                        id: download_quit_button
                        text: "Quit"
                        font.pointSize: root.is_mobile ? root.largePointSize : root.pointSize
                        Layout.alignment: Qt.AlignCenter
                        onClicked: Qt.quit()
                    }

                    Item { Layout.fillWidth: true }
                }

                Item { Layout.fillHeight: true }
            }
        }

        // Idx 3: Completed
        Frame {
            Layout.fillWidth: true
            Layout.fillHeight: true

            ColumnLayout {
                spacing: 10
                anchors.fill: parent

                Image {
                    source: "icons/appicons/simsapa.png"
                    Layout.preferredWidth: 100
                    Layout.preferredHeight: 100
                    Layout.alignment: Qt.AlignCenter
                }

                ColumnLayout {
                    Layout.margins: 20

                    Item { Layout.fillHeight: true }

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

                    Item { Layout.fillHeight: true }

                    Button {
                        id: completed_quit_button
                        text: "Quit"
                        font.pointSize: root.is_mobile ? root.largePointSize : root.pointSize
                        Layout.alignment: Qt.AlignCenter
                        onClicked: Qt.quit()
                    }
                }

                Item {
                    visible: root.is_mobile
                    Layout.fillHeight: true
                }
            }
        }

    }
}
