pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import QtQuick.Controls
import QtQuick.Window
import QtWebEngine

import components as C

import com.profoundlabs.simsapa

ApplicationWindow {
    id: root
    title: "Sutta Search - Simsapa"
    width: 1300
    height: 900
    visible: true
    color: palette.window

    property var all_results: []
    property bool is_loading: false

    SuttaBridge {
        id: sb
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

    // function show_sutta(query) {
    //     if (query.length < 4) {
    //         return;
    //     }
    //     var html = sb.get_sutta_html(query);
    //     web.loadHtml(html);
    // }

    function load_url(url) {
        webEngineView.url = url;
    }

    function set_query(text) {
        search_bar_input.search_input.text = text;
    }

    menuBar: MenuBar {
        Menu {
            title: "&File"

            C.MenuItem {
                text: "&Close Window"
                onTriggered: root.close()
            }

            C.MenuItem {
                action: Action {
                    text: "&Quit Simsapa"
                    icon.source: "icons/32x32/fa_times-circle.png"
                    id: action_quit
                    shortcut: Shortcut {
                        /* FIXME sequences: [StandardKey.Quit] */
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

            C.MenuItem {
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
            title: "&Find"

            C.MenuItem {
                action: Action {
                    id: incremental_search
                    text: "Search As You Type"
                    checkable: true
                    checked: true
                }
            }

            C.MenuItem {
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

            C.MenuItem {
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

            C.MenuItem {
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

            C.MenuItem {
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

            C.MenuItem {
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

            C.MenuItem {
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

            C.SearchBarInput {
                id: search_bar_input
                web: webEngineView
                run_search_fn: root.run_search
                debounce_timer: debounce_timer
                incremental_search: incremental_search
            }

            C.SearchBarOptions {
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

                    // Left side tabs area
                    ColumnLayout {
                        SplitView.preferredWidth: parent.width * 0.5

                        ColumnLayout {
                            /* Layout.alignment: Qt.AlignTop */

                            WebEngineView {
                                id: webEngineView
                                /* focus: true */
                                Layout.fillWidth: true
                                Layout.fillHeight: true
                                url: "http://localhost:4848/"
                            }
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

                            background: Rectangle {
                                color: palette.window
                            }

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

                            C.FulltextResults {
                                Layout.preferredWidth: root.width * 0.5
                                id: fulltext_results
                                all_results: root.all_results
                                is_loading: root.is_loading
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
