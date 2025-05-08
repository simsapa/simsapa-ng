pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import QtQuick.Controls
import QtQuick.Window

import com.profoundlabs.simsapa

ApplicationWindow {
    id: root
    title: "Download Application Assets"
    width: 500
    height: 700
    visible: true
    color: palette.window
    flags: Qt.Dialog

    Component.onCompleted: {
        // TODO: Implement checking releases info. See asset_management.py class ReleasesWorker(QRunnable).
        // Assuming there is a network connection, show the download selection screen.
        views_stack.currentIndex = 1;
    }

    property bool include_appdata_downloads: true

    AssetManager { id: manager }

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

            // Default: General bundle
            urls.push(appdata_tar_url);
            urls.push(dictionaries_tar_url);

            /* console.log("Show progress bar"); */
            progress_bar.visible = true;

            manager.download_urls(urls);

            // TODO start_animation()
            // TODO _run_download_post_hook()
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
                    font.pointSize: 11
                    wrapMode: Text.WordWrap
                    Layout.fillWidth: true
                    Layout.alignment: Qt.AlignCenter
                    onLinkActivated: function(link) {
                        /* console.log(link + " link activated"); */
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

                Text {
                    textFormat: Text.RichText
                    font.pointSize: 11
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
                    RadioButton {
                        text: "General bundle"
                        checked: true
                        onClicked: {} // _toggled_general_bundle
                    }

                    Label { text: "Pāli and English + pre-generated search index" }
                    Label { text: "" }

                    RadioButton {
                        text: "Include additional texts"
                        checked: false
                        enabled: false
                    }
                }

                Item { Layout.fillHeight: true }

                Button {
                    text: "Download"
                    Layout.alignment: Qt.AlignCenter
                    onClicked: {
                        views_stack.currentIndex = 2;
                        root.validate_and_run_download();
                    }
                }

                Button {
                    Layout.alignment: Qt.AlignCenter
                    text: "Quit"
                    onClicked: Qt.quit()
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

                Label {
                    id: download_status
                    Layout.alignment: Qt.AlignCenter
                    text: "Downloading ..."
                }

                ProgressBar {
                    id: progress_bar
                    Layout.alignment: Qt.AlignCenter
                    Layout.preferredWidth: parent.width * 0.9
                    visible: true
                    from: 0
                    to: 1
                    value: 0
                }

                Button {
                    id: progress_cancel_button
                    text: "Cancel"
                    Layout.alignment: Qt.AlignCenter
                    enabled: false
                    onClicked: {} // _handle_cancel_download
                }

                Button {
                    Layout.alignment: Qt.AlignCenter
                    text: "Quit"
                    onClicked: Qt.quit()
                }

                Item { Layout.fillHeight: true }

                Connections {
                    target: manager

                    function onDownloadProgressChanged(op_msg: string, downloaded_bytes: int, total_bytes: int) {
                        var frac = total_bytes > 0 ? downloaded_bytes / total_bytes : 0;
                        progress_bar.value = frac;
                        if (downloaded_bytes == total_bytes) {
                            download_status.text = op_msg;
                        } else {
                            download_status.text = `${op_msg}: ${downloaded_bytes} / ${total_bytes} bytes`;
                        }
                    }

                    function onDownloadFinished (message) {
                        download_status.text = message;
                        /* quitButton.enabled = true; */
                    }
                }
            }
        }
    }
}
