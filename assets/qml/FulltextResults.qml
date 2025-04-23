pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import QtQuick.Controls

/* import data */

ColumnLayout {
    id: root

    function select_previous_result() {
        if (fulltext_list.currentIndex > 0)
            fulltext_list.currentIndex--
    }

    function select_next_result() {
        if (fulltext_list.currentIndex < fulltext_list.count - 1)
            fulltext_list.currentIndex++
    }

    property var all_results: []
    property int page_len: 10
    property int current_page: 1
    property int total_pages: (all_results.length > 0 ? Math.ceil(all_results.length / page_len) : 1)
    property bool is_loading: false

    RowLayout {
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
        readonly property int item_height: tm1.height + tm2.height*3 + item_padding*2

        readonly property TextMetrics tm1: TextMetrics { text: "#"; font.pointSize: 11; font.bold: true }
        readonly property TextMetrics tm2: TextMetrics { text: "#"; font.pointSize: 11 }

        Layout.preferredHeight: root.page_len * item_height
        Layout.minimumWidth: contentItem.childrenRect.width + item_padding*2
        Layout.fillWidth: true

        model: results_model
        clip: true
        spacing: 0
        visible: results_model.count > 0
        delegate: search_result_delegate

        ScrollBar.vertical: ScrollBar {
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
                width: fulltext_list.width
                height: parent.height

                background: Rectangle {
                    id: row_rect
                    width: parent.width
                    radius: 5 // slight rounding for a button feel
                    border.width: 1
                    border.color: Qt.darker(base_color, 1.15)

                    property color even_color: "#efefef"
                    property color odd_color: "#ffffff"
                    property color selected_color: "#a0c4ff"
                    property color base_color: (result_item.index % 2 === 0 ? even_color : odd_color)

                    color: fulltext_list.currentIndex === result_item.index ? row_rect.selected_color : row_rect.base_color

                    // 3D–button gradient: darker edges, flat center
                    gradient: Gradient {
                        // very top edge: slightly lighter
                        GradientStop { position: 0.0; color: Qt.lighter(row_rect.color, 1.15) }
                        // just below edge: back to base
                        GradientStop { position: 0.2; color: row_rect.color }
                        // just above bottom edge: base
                        GradientStop { position: 0.95; color: row_rect.color }
                        // very bottom edge: slightly darker
                        GradientStop { position: 1.0; color: Qt.darker(row_rect.color, 1.10) }
                    }

                }

                MouseArea {
                    anchors.fill: parent
                    onClicked: {
                        fulltext_list.currentIndex = result_item.index
                        // Ensure it's visible if scrolled out
                        fulltext_list.positionViewAtIndex(result_item, ListView.Visible)
                    }
                }

                ColumnLayout {
                    anchors.margins: 8
                    width: parent.width
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

    /* BojjhangaData { id: results_model } */
    ListModel { id: results_model }

}
