import QtQuick
import QtQuick.Layouts
import QtQuick.Controls

Frame {
    id: root
    Layout.fillWidth: true
    Layout.minimumHeight: 40

    required property bool is_wide
    required property bool db_loaded
    required property var run_search_fn
    required property Timer debounce_timer
    required property Action incremental_search

    property alias search_input: search_input

    background: Rectangle {
        color: "transparent"
        border.color: "transparent"
        border.width: 0
    }

    RowLayout {
        id: searchbar_layout
        Layout.fillWidth: true

        // === Search Input ====
        TextField {
            id: search_input
            enabled: root.db_loaded
            Layout.fillWidth: true
            Layout.preferredWidth: root.is_wide ? 500 : 250
            Layout.preferredHeight: 40

            focus: true
            font.pointSize: 12
            placeholderText: root.db_loaded ? "Search in suttas" : "Loading..."

            onAccepted: search_btn.clicked()
            onTextChanged: {
                if (root.incremental_search.checked) root.debounce_timer.restart();
            }

            selectByMouse: true
        }

        Button {
            id: search_btn
            icon.source: "icons/32x32/bx_search_alt_2.png"
            enabled: search_input.text.length > 0
            onClicked: root.run_search_fn(search_input.text)
            Layout.preferredHeight: 40
            Layout.preferredWidth: 40
        }
    }
}
