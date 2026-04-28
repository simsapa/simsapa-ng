pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import QtQuick.Controls
import QtQuick.Window

import com.profoundlabs.simsapa

ApplicationWindow {
    id: root

    title: "Re-indexing Dictionaries"
    width: is_mobile ? Screen.desktopAvailableWidth : 520
    height: is_mobile ? Screen.desktopAvailableHeight : 220
    visible: true
    color: palette.window
    flags: Qt.Dialog
    modality: Qt.ApplicationModal

    readonly property bool is_mobile: Qt.platform.os === "android" || Qt.platform.os === "ios"
    readonly property int pointSize: is_mobile ? 16 : 12

    // Bridge wiring is filled in during task 4.0 (DictionaryManager bridge).
    // The pre-bridge fallback runs reconciliation synchronously from C++ before
    // SuttaSearchWindow is opened — this window is shown while that work is
    // delegated to the bridge worker thread and its progress signals.
    property string stage_text: "Re-indexing imported dictionaries — please wait."
    property real progress_value: 0.0
    property bool indeterminate: true

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 16
        spacing: 12

        Label {
            id: stage_label
            text: root.stage_text
            font.pointSize: root.pointSize
            wrapMode: Text.Wrap
            Layout.fillWidth: true
        }

        ProgressBar {
            id: progress_bar
            from: 0
            to: 1
            value: root.progress_value
            indeterminate: root.indeterminate
            Layout.fillWidth: true
        }

        Item { Layout.fillHeight: true }
    }
}
