pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import QtQuick.Controls

/* import data // for qml preview */

ColumnLayout {
    id: root

    /* BojjhangaData { id: results_model } // for qml preview */
    ListModel { id: results_model }

    function select_previous_result() {
        if (fulltext_list.currentIndex > 0)
            fulltext_list.currentIndex--
    }

    function select_next_result() {
        if (fulltext_list.currentIndex < fulltext_list.count - 1)
            fulltext_list.currentIndex++
    }

    readonly property TextMetrics tm1: TextMetrics { text: "#"; font.pointSize: 11 }

    property var all_results: []
    property int page_len: 10
    property int current_page: 1
    property int total_pages: (all_results.length > 0 ? Math.ceil(all_results.length / page_len) : 1)
    property bool is_loading: false
    property alias currentIndex: fulltext_list.currentIndex
    property alias currentItem: fulltext_list.currentItem

    function current_uid() {
        return results_model.get(fulltext_list.currentIndex).uid;
    }

    RowLayout {
        id: controls_row
        Layout.fillWidth: true

        SpinBox {
            id: fulltext_page_input; from: 1; to: 999;
            editable: true
            Layout.preferredWidth: 50
        }

        Button {
            id: fulltext_prev_btn
            Layout.preferredWidth: 40
            icon.source: "icons/32x32/fa_angle-left-solid.png"
            ToolTip.visible: hovered
            ToolTip.text: "Previous page of results"
            enabled: root.current_page > 1
            onClicked: { root.current_page--; root.update_page(); }
        }
        Button {
            id: fulltext_next_btn
            Layout.preferredWidth: 40
            icon.source: "icons/32x32/fa_angle-right-solid.png"
            ToolTip.visible: hovered
            ToolTip.text: "Next page of results"
            enabled: root.current_page < root.total_pages
            onClicked: { root.current_page++; root.update_page(); }
        }

        Label {
            id: fulltext_label
            // TODO: Use result count range: Showing a-b out of x
            text: "Page " + root.current_page + " of " + root.total_pages
        }

        // Spacer
        Item {
            Layout.fillWidth: true
        }

        Button {
            id: fulltext_first_page_btn
            Layout.preferredWidth: 40
            icon.source: "icons/32x32/fa_angles-left-solid.png"
            ToolTip.visible: hovered
            ToolTip.text: "First page of results"
        }
        Button {
            id: fulltext_last_page_btn
            Layout.preferredWidth: 40
            icon.source: "icons/32x32/fa_angles-right-solid.png"
            ToolTip.visible: hovered
            ToolTip.text: "Last page of results"
            Layout.alignment: Qt.AlignRight
        }
    }

    Rectangle {
        id: fulltext_loading_bar
        color: "transparent"
        Layout.fillWidth: true
        Layout.preferredHeight: 5
        Layout.alignment: Qt.AlignCenter
        AnimatedImage {
            source: "icons/gif/loading-bar.gif"
            anchors.horizontalCenter: parent.horizontalCenter
            visible: root.is_loading
            playing: root.is_loading
            cache: true
        }
    }

    // Paginate results into the model
    function update_page() {
        results_model.clear()
        total_pages = (all_results.length > 0 ? Math.ceil(all_results.length / page_len) : 1)
        var start = (current_page - 1) * page_len
        var end = Math.min(start + page_len, all_results.length)
        for (var i = start; i < end; ++i) {
            var item = all_results[i]
            results_model.append({
                index: i,

                // uid:         item.uid,
                // schemaName:  item.schema_name,
                // tableName:   item.table_name,
                // sourceUid:   item.source_uid,
                // title:       item.title,
                // ref:         item.ref,
                // nikaya:      item.nikaya,
                // author:      item.author,
                // snippet:     item.snippet,
                // pageNumber:  item.page_number,
                // score:       item.score,
                // rank:        item.rank,

                uid:         item.uid,
                title:       item.title,
                snippet:     item.snippet,
                sutta_ref:   item.sutta_ref,
                /* author:      item.author, */
            })
        }
    }

    Text {
        id: empty_state
        text: "No results found."
        visible: !root.is_loading && results_model.count === 0
        horizontalAlignment: Text.AlignHCenter
        font.italic: true
        color: "grey"
        Layout.fillWidth: true
    }

    ListView {
        id: fulltext_list
        orientation: ListView.Vertical

        readonly property int item_padding: 10
        readonly property int item_height: root.tm1.height*4 + item_padding*2

        // FIXME: can't get this ListView to resize to fill the available height
        Layout.preferredHeight: 500
        Layout.fillWidth: true

        model: results_model
        clip: true
        spacing: 0
        visible: results_model.count > 0
        delegate: search_result_delegate

        ScrollBar.vertical: ScrollBar {
            // AlwaysOn b/c mobile can't hover to show the bar
            policy: ScrollBar.AlwaysOn
            padding: 5
        }

        Keys.onPressed: function(event) {
            console.log("key:" + event.key)
            if (event.key === Qt.Key_Up ||
                (event.key === Qt.Key_K && event.modifiers & Qt.ControlModifier)) {
                if (fulltext_list.currentIndex > 0)
                    fulltext_list.currentIndex--
                event.accepted = true
            }
            else if (event.key === Qt.Key_Down ||
                        (event.key === Qt.Key_J && event.modifiers & Qt.ControlModifier)) {
                if (fulltext_list.currentIndex < fulltext_list.count - 1)
                    fulltext_list.currentIndex++
                event.accepted = true
            }
        }
    }

    Component {
        id: search_result_delegate
        ItemDelegate {
            id: result_item
            // NOTE: parent.width occasionally causes: TypeError: Cannot read property 'width' of null
            width: parent ? parent.width : 0
            height: fulltext_list.item_height

            required property int index
            required property string uid
            required property string title
            required property string snippet
            /* required property string nikaya */
            required property string sutta_ref
            property string author: ""
            /* required property int page_number */
            /* required property real score */

            Frame {
                id: item_frame
                anchors.fill: parent
                padding: fulltext_list.item_padding

                background: ListBackground {
                    results_list: fulltext_list
                    result_item_index: result_item.index
                }

                MouseArea {
                    anchors.fill: parent
                    onClicked: fulltext_list.currentIndex = result_item.index
                }

                ColumnLayout {
                    anchors.fill: parent
                    spacing: 4

                    // property color text_color: fulltext_list.currentIndex === result_item.index ? "#000" : "#fff"

                    // Title and metadata
                    RowLayout {
                        spacing: 12
                        Text { text: result_item.sutta_ref; font.pointSize: 11; font.bold: true }
                        Text { text: result_item.title; font.pointSize: 11; font.bold: true }
                        Item { Layout.fillWidth: true }
                        Text { text: result_item.uid; font.pointSize: 11; font.italic: true }
                    }

                    // Snippet with highlighted HTML
                    Text {
                        id: item_snippet
                        textFormat: Text.RichText
                        font.pointSize: 11
                        text: "<style> span.match { background-color: yellow; } </style>" + result_item.snippet
                        wrapMode: Text.WordWrap
                        Layout.fillWidth: true
                    }
                }
            }
        }
    }
}
