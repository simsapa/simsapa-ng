pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import QtQuick.Controls

import com.profoundlabs.simsapa

Frame {
    id: root
    Layout.fillWidth: true
    Layout.minimumHeight: root.tm1.height*7
    Layout.maximumHeight: Math.min(root.window_height*0.5, main_col.height)
    visible: false

    required property int window_height
    readonly property TextMetrics tm1: TextMetrics { text: "#"; font.pointSize: 9; font.bold: true }

    SuttaBridge { id: sb }

    background: Rectangle {
        color: palette.window
        border.width: 1
        border.color: Qt.darker(palette.window, 1.15)
    }

    ListModel {
        id: deconstructor_model
        ListElement { words_joined: "olokita + saññāṇena + eva" }
        ListElement { words_joined: "olokita + saññāṇena + iva" }
    }

    ListModel {
        id: summaries_model
        ListElement { summary: "<b>olokita</b> pp. <b>looked at, inspected</b> [ava + √lok], pp of oloketi" }
        ListElement { summary: "<b>saññāṇa 1</b> nt. <b>marking; signing</b> [saṁ + √ñā + aṇa], nt, act, from sañjānāti" }
        ListElement { summary: "<b>saññāṇa 2</b> nt. <b>mental noting;</b> lit. marking [saṁ + √ñā + aṇa], nt, act, from sañjānāti" }
        ListElement { summary: "<b>eva 1</b> ind. <b>only; just; merely; exclusively</b>, ind, emph" }
        ListElement { summary: "<b>iva 1</b> ind. <b>like; as</b>, ind" }
    }

    function set_query(query: string) {
        if (query.length < 4) {
            return;
        }
        root.visible = true;
        lookup_input.text = query;
    }

    function run_lookup(query: string) {
        console.log("run_lookup(): " + query) // TODO
    }

    ColumnLayout {
        id: main_col
        anchors.fill: parent

        RowLayout {
            TextField {
                id: lookup_input
                Layout.fillWidth: true
                text: "olokitasaññāṇeneva"
            }
            Button {
                id: search_btn
                icon.source: "icons/32x32/bx_search_alt_2.png"
                onClicked: root.run_lookup(lookup_input.text)
                Layout.preferredHeight: lookup_input.height
                Layout.preferredWidth: lookup_input.height
                ToolTip.visible: hovered
                ToolTip.text: "Search"
            }
            Button {
                id: close_btn
                icon.source: "icons/32x32/mdi--close.png"
                Layout.preferredHeight: lookup_input.height
                Layout.preferredWidth: lookup_input.height
                ToolTip.visible: hovered
                ToolTip.text: "Close word summaries"
                onClicked: root.visible = false
            }
        }

        RowLayout {
            ComboBox {
                textRole: "words_joined"
                id: deconstructor
                model: deconstructor_model
                Layout.fillWidth: true
            }
            Button {
                id: copy_btn
                icon.source: "icons/32x32/lucide-lab--copy-text.png"
                Layout.preferredHeight: lookup_input.height
                Layout.preferredWidth: lookup_input.height
                ToolTip.visible: hovered
                ToolTip.text: "Copy listed summaries"
            }
            Button {
                id: open_lookup_window_btn
                icon.source: "icons/32x32/bxs_book_content.png"
                Layout.preferredHeight: lookup_input.height
                Layout.preferredWidth: lookup_input.height
                ToolTip.visible: hovered
                ToolTip.text: "Open query in Word Lookup Window"
            }
        }

        ListView {
            id: summaries_list
            orientation: ListView.Vertical
            clip: true
            spacing: 0

            readonly property int item_padding: 4
            readonly property int item_height: root.tm1.height*2 + item_padding*2

            Layout.preferredHeight: summaries_model.count * item_height
            Layout.fillWidth: true

            model: summaries_model
            delegate: summaries_delegate

            ScrollBar.vertical: ScrollBar {
                policy: ScrollBar.AlwaysOn // FIXME doesn't scroll
                padding: 5
            }
        }

        Component {
            id: summaries_delegate
            ItemDelegate {
                id: result_item
                width: parent ? parent.width : 0
                height: summaries_list.item_height

                required property int index
                required property string summary

                Frame {
                    id: item_frame
                    anchors.fill: parent
                    padding: summaries_list.item_padding

                    background: ListBackground {
                        results_list: summaries_list
                        result_item_index: result_item.index
                    }

                    MouseArea {
                        anchors.fill: parent
                        onClicked: {
                            summaries_list.currentIndex = result_item.index
                            // Ensure it's visible if scrolled out
                            summaries_list.positionViewAtIndex(result_item, ListView.Visible)
                        }
                    }

                    ColumnLayout {
                        anchors.fill: parent
                        spacing: 0
                        Text {
                            text: result_item.summary
                            textFormat: Text.RichText
                            font.pointSize: 9
                            wrapMode: Text.WordWrap
                            Layout.fillWidth: true
                        }
                    }
                }
            }
        }
    }
}
