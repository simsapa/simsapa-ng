import QtQuick
import QtQuick.Window

ApplicationWindow {
    id: root

    required property int top_bar_margin

    property string shortcut: ""
    property string conflicting_action_name: ""

    signal confirmed()
    signal cancelled()
}
