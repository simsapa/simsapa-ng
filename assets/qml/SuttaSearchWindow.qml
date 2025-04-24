pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import QtQuick.Controls
import QtQuick.Window

import com.profoundlabs.simsapa

ApplicationWindow {
    id: root
    title: "Sutta Search - Simsapa"
    width: 1300
    height: 900
    visible: true
    color: palette.window

    /* property SuttaHtmlView current_web_view: (suttas_tab_bar.currentIndex < suttas_tab_bar.count) ? sutta_html_view_layout.children[suttas_tab_bar.currentIndex] : null */

    // TODO: implment find_bar
    // onCurrent_web_viewChanged: {
    //     find_bar.reset();
    // }

    property bool is_mac: Qt.platform.os == "osx"

    property var all_results: []
    property bool is_loading: false

    SuttaBridge {
        id: sb
    }

    ListModel {
        id: tabs_pinned_model
        /* ListElement { title: "Pinned"; pinned: true } */
    }

    ListModel {
        id: tabs_results_model
        /* ListElement { title: "Sutta"; pinned: false } */
    }

    ListModel {
        id: tabs_translations_model
        /* ListElement { title: "Translations"; pinned: false } */
    }

    // Timer for incremental search debounce
    Timer {
        id: debounce_timer
        interval: 400 // milliseconds
        repeat: false
        onTriggered: {
            if (incremental_search.checked && search_bar_input.search_input.text.length >= 4) {
                root.run_search(search_bar_input.search_input.text)
            }
        }
    }

    function run_search(query) {
        root.is_loading = true
        Qt.callLater(function() {
            let json_res = sb.search(query)
            root.all_results = JSON.parse(json_res)
            fulltext_results.current_page = 1
            fulltext_results.update_page()
            root.is_loading = false
        })
    }

    function create_new_tab_and_webview(sutta_uid, focus_on_new_tab = false) {
        var data = { title: sutta_uid, pinned: false }
        tabs_results_model.append(data);

        // TODO: implement focusing/selecting the new tab

        var webview = sutta_tab_component.createObject(sutta_html_view_layout, {sutta_uid: sutta_uid});
        return webview;
    }

    Component.onCompleted: {
        if (tabs_results_model.count == 0) {
            tabs_results_model.append({title: "Sutta", pinned: false});
        }
        if (sutta_html_view_layout.count == 0) {
            sutta_tab_component.createObject(sutta_html_view_layout, {sutta_uid: "Sutta"});
        }
    }

    /* function run_search(query) { */
    /*     if (query.length < 4) { */
    /*         return; */
    /*     } */
    /*     var html = sb.get_sutta_html(query); */
    /*     webEngineView.loadHtml(html); */
    /* } */

    function set_query(text) {
        search_bar_input.search_input.text = text;
    }

    menuBar: MenuBar {
        Menu {
            title: "&File"

            CMenuItem {
                text: "&Close Window"
                onTriggered: root.close()
            }

            CMenuItem {
                action: Action {
                    text: "&Quit Simsapa"
                    icon.source: "icons/32x32/fa_times-circle.png"
                    id: action_quit
                    shortcut: Shortcut {
                        sequences: ["Ctrl+Q"]
                        context: Qt.WindowShortcut
                        onActivated: action_quit.trigger()
                    }
                    onTriggered: Qt.quit()
                }
            }
        }

        Menu {
            title: "&Edit"

            CMenuItem {
                action: Action {
                    id: action_focus_search
                    text: "Focus Search Input"
                    shortcut: Shortcut {
                        sequences: ["Ctrl+L"]
                        context: Qt.WindowShortcut
                        onActivated: action_focus_search.trigger()
                    }
                    onTriggered: {
                        search_bar_input.search_input.forceActiveFocus();
                        search_bar_input.search_input.selectAll();
                    }
                }
            }
        }

        Menu {
            title: "&View"

            CMenuItem {
                action: Action {
                    id: web_reload
                    text: "Reload Page"
                    // TODO: implement reload
                    // onTriggered: {
                    //     if (root.current_web_view)
                    //         root.current_web_view.reload();
                    // }
                }
            }
        }

        Menu {
            title: "&Find"

            CMenuItem {
                action: Action {
                    id: incremental_search
                    text: "Search As You Type"
                    checkable: true
                    checked: true
                }
            }

            CMenuItem {
                action: Action {
                    id: select_previous_result
                    text: "Previous Result"
                    shortcut: Shortcut {
                        sequences: ["Ctrl+Up", "Ctrl+K"]
                        context: Qt.WindowShortcut
                        onActivated: select_previous_result.trigger()
                    }
                    onTriggered: fulltext_results.select_previous_result()
                }
            }

            CMenuItem {
                action: Action {
                    id: select_next_result
                    text: "Next Result"
                    shortcut: Shortcut {
                        sequences: ["Ctrl+Down", "Ctrl+J"]
                        context: Qt.WindowShortcut
                        onActivated: select_next_result.trigger()
                    }
                    onTriggered: fulltext_results.select_next_result()
                }
            }
        }

        Menu {
            title: "&Windows"

            CMenuItem {
                action: Action {
                    id: action_sutta_search
                    text: "&Sutta Search"
                    icon.source: "icons/32x32/bxs_book_bookmark.png"
                    shortcut: Shortcut {
                        sequences: ["F5"]
                        context: Qt.WindowShortcut
                        onActivated: action_sutta_search.trigger()
                    }
                    /* onTriggered: TODO */
                }
            }

            CMenuItem {
                action: Action {
                    id: action_sutta_study
                    text: "&Sutta Study"
                    icon.source: "icons/32x32/bxs_book_bookmark.png"
                    shortcut: Shortcut {
                        sequences: ["Ctrl+F5"]
                        context: Qt.WindowShortcut
                        onActivated: action_sutta_study.trigger()
                    }
                    /* onTriggered: TODO */
                }
            }

            CMenuItem {
                action: Action {
                    id: action_dictionary_search
                    text: "&Dictionary Search"
                    icon.source: "icons/32x32/bxs_book_content.png"
                    shortcut: Shortcut {
                        sequences: ["F6"]
                        context: Qt.WindowShortcut
                        onActivated: action_dictionary_search.trigger()
                    }
                    /* onTriggered: TODO */
                }
            }

            CMenuItem {
                action: Action {
                    id: action_show_word_lookup
                    text: "Show Word Lookup"
                    checkable: true
                    checked: false
                    shortcut: Shortcut {
                        sequences: ["Ctrl+F6"]
                        context: Qt.WindowShortcut
                        onActivated: action_show_word_lookup.trigger()
                    }
                    /* onTriggered: TODO */
                }
            }
        }
    }

    ColumnLayout {
        anchors.fill: parent

        RowLayout {

            SearchBarInput {
                id: search_bar_input
                run_search_fn: root.run_search
                debounce_timer: debounce_timer
                incremental_search: incremental_search
            }

            SearchBarOptions {
                id: search_bar_options
            }

        }

        // Main horizontal layout
        RowLayout {
            Layout.fillWidth: true
            Layout.fillHeight: true

            // Left side layout
            ColumnLayout {
                Layout.fillWidth: true
                Layout.fillHeight: true

                SplitView {
                    id: panel_split
                    orientation: Qt.Horizontal
                    Layout.fillWidth: true
                    Layout.fillHeight: true

                    handle: Rectangle {
                        id: split_handle
                        implicitWidth: 2
                        implicitHeight: panel_split.height
                        color: SplitHandle.pressed ? panel_split.palette.dark : (SplitHandle.hovered ? panel_split.palette.midlight : panel_split.palette.mid)
                        containmentMask: Item {
                            x: (split_handle.width - width) / 2
                            width: 20
                            height: split_handle.height
                        }
                    }

                    Item {
                        id: suttas_tab_container
                        SplitView.preferredWidth: parent.width * 0.5
                        /* Layout.alignment: Qt.AlignTop */

                        TabBar {
                            id: suttas_tab_bar
                            anchors.top: parent.top
                            /* anchors.left: parent.left */
                            /* anchors.right: parent.right */

                            Component {
                                id: sutta_tab_component

                                SuttaHtmlView {
                                    Layout.fillWidth: true
                                    Layout.fillHeight: true
                                    // focus: true
                                }
                            }

                            contentItem: RowLayout {
                                spacing: 0

                                Repeater {
                                    id: tabs_pinned
                                    model: tabs_pinned_model
                                    delegate: SuttaTabButton {
                                        id: pinned_tab_btn
                                        onPinToggled: function (pinned) {
                                            if (pinned) return;
                                            // Unpin and move back to results group
                                            var data = { title: pinned_tab_btn.title, pinned: false }
                                            tabs_results_model.append(data)
                                            tabs_pinned_model.remove(pinned_tab_btn.index)
                                        }
                                        onCloseClicked: tabs_pinned_model.remove(index)
                                    }
                                }

                                Item { Layout.preferredWidth: 5 }

                                Repeater {
                                    id: tabs_results
                                    model: tabs_results_model
                                    delegate: SuttaTabButton {
                                        id: results_tab_btn
                                        onPinToggled: function (pinned) {
                                            if (!pinned) return;
                                            // Pin and move to pinned group
                                            var data = { title: results_tab_btn.title, pinned: true };
                                            tabs_pinned_model.append(data);
                                            tabs_results_model.remove(results_tab_btn.index);
                                        }
                                        onCloseClicked: tabs_results_model.remove(index);
                                    }
                                }

                                Item { Layout.preferredWidth: 5 }

                                Repeater {
                                    id: tabs_translations
                                    model: tabs_translations_model
                                    delegate: SuttaTabButton {
                                        id: translations_tab_btn
                                        onPinToggled: function (pinned) {
                                            if (!pinned) return;
                                            // Pin and move to pinned group
                                            var data = { title: translations_tab_btn.title, pinned: true };
                                            tabs_pinned_model.append(data);
                                            tabs_translations_model.remove(translations_tab_btn.index);
                                        }
                                        onCloseClicked: tabs_translations_model.remove(index);
                                    }
                                }

                            }

                        }

                        StackLayout {
                            id: sutta_html_view_layout
                            currentIndex: suttas_tab_bar.currentIndex

                            anchors.top: suttas_tab_bar.bottom
                            anchors.bottom: suttas_tab_container.bottom
                            anchors.left: suttas_tab_container.left
                            anchors.right: suttas_tab_container.right
                            // anchors.topMargin: 5
                        }
                    }

                    Item {
                        SplitView.preferredWidth: parent.width * 0.5

                        // Right side tabs
                        TabBar {
                            id: rightside_tabs
                            anchors.top: parent.top
                            anchors.left: parent.left
                            anchors.right: parent.right

                            TabButton {
                                text: "Results"
                                id: results_tab
                                icon.source: "icons/32x32/bx_search_alt_2.png"
                                padding: 5
                            }
                            TabButton {
                                text: "History"
                                id: history_tab
                                icon.source: "icons/32x32/fa_clock-rotate-left-solid.png"
                                padding: 5
                            }
                        }

                        // Tab content areas
                        StackLayout {
                            currentIndex: rightside_tabs.currentIndex
                            anchors.top: rightside_tabs.bottom
                            anchors.topMargin: 5
                            width: parent.width

                            FulltextResults {
                                id: fulltext_results
                                all_results: root.all_results
                                is_loading: root.is_loading

                                function update_item() {
                                    var all_results_count = fulltext_results.all_results.length;
                                    /* console.log("update_item() count: " + all_results_count); */
                                    /* if (all_results_count == 0) return; */
                                    tabs_results_model.clear();
                                    var uid = fulltext_results.current_uid();
                                    var data = { title: uid, pinned: false };
                                    tabs_results_model.append(data);

                                    // In the StackLayout, the children for tabs_results_model are preceded by the
                                    // items related to tabs_pinned_model.
                                    /* var n_pinned = tabs_pinned_model.count; */

                                    // The count is already index + 1.

                                    if (sutta_html_view_layout.count == 0) {
                                        sutta_tab_component.createObject(sutta_html_view_layout, {sutta_uid: uid});
                                    } else {
                                        sutta_html_view_layout.children[0].sutta_uid = uid;
                                    }
                                }

                                onCurrentIndexChanged: fulltext_results.update_item()
                            }

                            // History Tab
                            ColumnLayout {
                                id: recent_tab
                                ListView { id: recent_list }
                            }
                        }

                    }

                }
            }
        }
    }
}
