pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import QtQuick.Controls

import com.profoundlabs.simsapa

Frame {
    id: root
    height: Math.min(root.window_height*0.5, min_height)

    required property var handle_summary_close_fn

    readonly property int item_padding: 4
    property int min_height: summaries_model.count * (root.tm1.height*2 + item_padding*2) + 100
    required property int window_height
    readonly property TextMetrics tm1: TextMetrics { text: "#"; font.pointSize: 9; font.bold: true }

    required property bool incremental_search_checked

    SuttaBridge { id: sb }

    background: Rectangle {
        color: palette.window
        border.width: 1
        border.color: Qt.darker(palette.window, 1.15)
    }

    Timer {
        id: debounce_timer
        interval: 400 // milliseconds
        repeat: false
        onTriggered: {
            if (root.incremental_search_checked && lookup_input.text.length >= 4) {
                root.run_lookup(lookup_input.text);
            }
        }
    }

    ListModel { id: deconstructor_model }
    ListModel { id: summaries_model }

    // For qml preview
    /* ListModel { */
    /*     id: deconstructor_model */
    /*     ListElement { words_joined: "olokita + saññāṇena + eva" } */
    /*     ListElement { words_joined: "olokita + saññāṇena + iva" } */
    /* } */
    /* ListModel { */
    /*     id: summaries_model */
    /*     ListElement { summary: "<b>olokita</b> pp. <b>looked at, inspected</b> [ava + √lok], pp of oloketi" } */
    /*     ListElement { summary: "<b>saññāṇa 1</b> nt. <b>marking; signing</b> [saṁ + √ñā + aṇa], nt, act, from sañjānāti" } */
    /*     ListElement { summary: "<b>saññāṇa 2</b> nt. <b>mental noting;</b> lit. marking [saṁ + √ñā + aṇa], nt, act, from sañjānāti" } */
    /*     ListElement { summary: "<b>eva 1</b> ind. <b>only; just; merely; exclusively</b>, ind, emph" } */
    /*     ListElement { summary: "<b>iva 1</b> ind. <b>like; as</b>, ind" } */
    /* } */

    function set_query(query: string) {
        if (query.length < 4) {
            return;
        }
        lookup_input.text = query;
    }

    function run_lookup(query: string) {
        // root.is_loading = true; TODO
        Qt.callLater(function() {
            let res;
            deconstructor_model.clear();
            res = sb.dpd_deconstructor_list(query);
            for (let i=0; i < res.length; i++) {
                deconstructor_model.append({ words_joined: res[i] });
            }
            deconstructor.currentIndex = 0;

            summaries_model.clear();
            res = sb.dpd_lookup_list(query);
            for (let i=0; i < res.length; i++) {
                summaries_model.append({ summary: res[i] });
            }
            // clear the previous selection highlight
            summaries_list.currentIndex = -1;

            // root.is_loading = false; TODO
        });
    }

    ColumnLayout {
        id: main_col
        anchors.fill: parent

        RowLayout {
            id: row_one
            TextField {
                id: lookup_input
                Layout.fillWidth: true
                text: ""

                onAccepted: search_btn.clicked()
                onTextChanged: {
                    if (root.incremental_search_checked) debounce_timer.restart();
                }
                selectByMouse: true
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
                onClicked: root.handle_summary_close_fn() // qmllint disable use-proper-function
            }
        }

        RowLayout {
            id: row_two
            visible: deconstructor_model.count > 0
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

            readonly property int item_height: root.tm1.height*2 + root.item_padding*2

            // FIXME: can't get this ListView to resize to fill the available height
            Layout.preferredHeight: root.height - row_one.height - row_two.height - item_height
            Layout.fillWidth: true

            model: summaries_model
            delegate: summaries_delegate

            ScrollBar.vertical: ScrollBar {
                policy: ScrollBar.AlwaysOn
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
                    padding: root.item_padding

                    background: ListBackground {
                        results_list: summaries_list
                        result_item_index: result_item.index
                    }

                    MouseArea {
                        anchors.fill: parent
                        onClicked: summaries_list.currentIndex = result_item.index
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
