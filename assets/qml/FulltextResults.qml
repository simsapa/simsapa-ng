pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import QtQuick.Controls

/* import com.profoundlabs.simsapa */
/* import data // for qml preview */

ColumnLayout {
    id: root

    required property bool is_dark
    readonly property bool is_mobile: Qt.platform.os === "android" || Qt.platform.os === "ios"
    readonly property bool is_desktop: !root.is_mobile
    readonly property string match_bg: root.is_dark ? "#007A31" : "#F6E600"

    Logger { id: logger }

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

    readonly property int font_point_size: root.is_mobile ? 14 : 11
    readonly property TextMetrics tm1: TextMetrics { text: "#"; font.pointSize: root.font_point_size }

    required property var new_results_page_fn

    property var current_results: []
    property int page_len: 10
    property int page_num: 0
    property int total_hits: 0
    property int total_pages: (total_hits > 0 ? Math.ceil(total_hits / page_len) : 1)
    property bool is_loading: false
    property alias currentIndex: fulltext_list.currentIndex
    property alias currentItem: fulltext_list.currentItem

    function set_search_result_page(search_result_page) {
        // SearchResultPage { total_hits, page_len, page_num, results }
        let d = search_result_page;
        root.total_hits = d.total_hits;
        root.page_len = d.page_len;
        root.page_num = d.page_num;
        root.current_results = d.results;
        root.update_page();
    }

    function current_result_data(): var {
        return results_model.get(fulltext_list.currentIndex);
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
            enabled: root.page_num > 0
            onClicked: {
                fulltext_list.positionViewAtBeginning();
                root.page_num--;
                root.new_results_page_fn(root.page_num); // qmllint disable use-proper-function
            }
        }
        Button {
            id: fulltext_next_btn
            Layout.preferredWidth: 40
            icon.source: "icons/32x32/fa_angle-right-solid.png"
            ToolTip.visible: hovered
            ToolTip.text: "Next page of results"
            enabled: root.page_num < root.total_pages
            onClicked: {
                fulltext_list.positionViewAtBeginning();
                root.page_num++;
                root.new_results_page_fn(root.page_num); // qmllint disable use-proper-function
            }
        }

        Label {
            id: fulltext_label
            // TODO: Use result count range: Showing a-b out of x
            text: "Page " + (root.page_num+1) + " of " + root.total_pages
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

    function update_page() {
        // Remove existing item selection.
        fulltext_list.currentIndex = -1;
        // Remove current list of items.
        results_model.clear()
        // Populate model with new items.
        root.total_pages = (root.total_hits > 0 ? Math.ceil(root.total_hits / root.page_len) : 1)
        for (var i = 0; i < root.current_results.length; i++) {
            var item = root.current_results[i];
            var result_data = {
                index: i,
                item_uid:    item.uid,
                table_name:  item.table_name,
                sutta_title: item.title,
                sutta_ref:   item.sutta_ref || "", // Can be 'None' from SearchResult::from_dict_word()
                snippet:     item.snippet,
                /* author:      item.author, */
            };
            results_model.append(result_data);
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

        Layout.fillHeight: true
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
            logger.log("key:" + event.key);
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
            required property string item_uid
            required property string table_name
            required property string sutta_title
            required property string sutta_ref
            required property string snippet
            /* required property string nikaya */
            property string author: ""
            /* required property int page_number */
            /* required property real score */

            Frame {
                id: item_frame
                anchors.fill: parent
                padding: fulltext_list.item_padding

                background: ListBackground {
                    is_dark: root.is_dark
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
                        Text { 
                            text: result_item.sutta_ref
                            visible: result_item.sutta_ref !== ""
                            font.pointSize: root.font_point_size
                            font.bold: true
                            color: root.palette.active.text
                        }
                        Text { text: result_item.sutta_title; font.pointSize: root.font_point_size; font.bold: true; color: root.palette.active.text }
                        Item { Layout.fillWidth: true }
                        Text { text: result_item.item_uid; font.pointSize: root.font_point_size; font.italic: true; color: root.palette.active.text }
                    }

                    // Snippet with highlighted HTML
                    Text {
                        id: item_snippet
                        color: root.palette.active.text
                        textFormat: Text.RichText
                        font.pointSize: root.font_point_size
                        text: "<style> span.match { background-color: %1; } </style>".arg(root.match_bg) + result_item.snippet
                        wrapMode: Text.WordWrap
                        Layout.fillWidth: true
                    }
                }
            }
        }
    }
}
