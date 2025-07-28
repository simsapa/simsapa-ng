pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
/* import QtQuick.Controls */

/* import data // for qml preview */

ColumnLayout {
    id: root

    required property string window_id
    required property bool is_dark
    readonly property bool is_mobile: Qt.platform.os === "android" || Qt.platform.os === "ios"
    readonly property bool is_desktop: !root.is_mobile
    readonly property string match_bg: root.is_dark ? "#007A31" : "#F6E600"

    property string word_uid: "pāpuṇāti 1/dpd"

    DictionaryHtmlView {
        id: html_view
        window_id: root.window_id
        is_dark: root.is_dark
        word_uid: root.word_uid
        Layout.fillWidth: true
        Layout.fillHeight: true
    }
}
