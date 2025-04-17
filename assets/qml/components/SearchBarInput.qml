import QtQuick
import QtQuick.Layouts
import QtQuick.Controls
import QtWebEngine

import com.profoundlabs.simsapa

Frame {
    id: search_bar_input
    // id: search_bar
    Layout.fillWidth: true
    Layout.minimumHeight: 40

    property WebEngineView web
    property alias search_input: search_input

    background: Rectangle {
        color: "transparent"
        border.color: "transparent"
        border.width: 0
    }

    function show_sutta(query) {
        if (query.length < 4) {
            return;
        }
        var html = sb.get_sutta_html(query);
        web.loadHtml(html);
    }

    RowLayout {
        id: searchbar_layout
        Layout.fillWidth: true

        SuttaBridge {
            id: sb
        }

        // === Search Input ====
        TextField {
            id: search_input
            Layout.fillWidth: true
            Layout.preferredWidth: 500
            Layout.preferredHeight: 40

            focus: true
            font.pointSize: 12
            placeholderText: qsTr("Search in suttas")

            /* Binding on text { */
            /* when: webEngineView */
            /* value: webEngineView.url */
            /* } */
            /* onAccepted: webEngineView.url = Utils.fromUserInput(text) */

            selectByMouse: true
        }

        Button {
            id: search_btn
            icon.source: "icons/32x32/bx_search_alt_2.png"
            onClicked: search_bar_input.show_sutta(search_input.text)
            // activeFocusOnTab: !aw.platformIsMac
            Layout.preferredHeight: 40
            Layout.preferredWidth: 40
        }
    }
}
