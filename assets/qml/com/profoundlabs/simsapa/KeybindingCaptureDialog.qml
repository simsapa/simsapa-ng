import QtQuick
import QtQuick.Window

ApplicationWindow {
    id: root

    required property int top_bar_margin

    property string action_name: ""
    property string current_shortcut: ""
    property bool is_new_shortcut: true

    signal shortcutAccepted(string shortcut)
    signal shortcutRemoved()
}
