import QtQuick
import QtQuick.Layouts
import QtQuick.Controls
import QtQuick.Window
import QtWebView

import QtQuick.Controls.Basic

// import com.profoundlabs.simsapa 1.0

ApplicationWindow {
    id: aw
    title: "Sutta Search - Simsapa"
    width: 1300
    height: 900
    visible: true
    color: palette.window

    // SuttaBridge {
    //     id: sb
    // }

    function load_url(url) {
        webEngineView.url = url;
    }

    function set_query(text) {
        search_input.text = text;
    }

    function show_sutta(query) {
        if (query.length < 4) {
            return;
        }
        var html = sb.get_sutta_html(query);
        webEngineView.loadHtml(html);
    }

    Action {
        id: action_focus_search
        shortcut: "Ctrl+L"
        onTriggered: {
            search_input.forceActiveFocus();
            search_input.selectAll();
        }
    }

    Action {
        id: action_quit
        shortcut: StandardKey.Quit
        onTriggered: Qt.quit()
    }

    Action {
        id: action_sutta_search
        shortcut: "F5"
        /* onTriggered: aw.close() */
    }

    Action {
        id: action_Sutta_Study
        shortcut: "Ctrl+F5"
        /* onTriggered: aw.close() */
    }

    Action {
        id: action_Dictionary_Search
        shortcut: "F6"
        /* onTriggered: aw.close() */
    }

    menuBar: MenuBar {
        Menu {
            title: "&File"
            MenuItem {
                text: "&Close Window"
                onTriggered: aw.close()
            }
            MenuItem {
                text: "&Quit Simsapa"
                icon.source: "icons/32x32/fa_times-circle.png"
                action: action_quit
            }
        }

        Menu {
            title: "&Windows"
            MenuItem {
                text: "&Sutta Search"
                icon.source: "icons/32x32/bxs_book_bookmark.png"
                action: action_sutta_search
            }

            /* MenuItem { */
            /*     text: "Sutta Study" */
            /*     // book icon */
            /*     // Ctrl+F5 */
            /*     onTriggered: action_Sutta_Study() */
            /* } */
            /* MenuItem { */
            /*     text: "&Dictionary Search" */
            /*     // dict icon */
            /*     // F6 */
            /*     onTriggered: action_Dictionary_Search() */
            /* } */
        }
    }

    ColumnLayout {
        anchors.fill: parent

        // SearchBar {
        //     id: searchbar_layout
        // }

        Frame {
            id: search_bar
            Layout.fillWidth: true
            Layout.minimumHeight: 40

            RowLayout {
                id: searchbar_layout
                Layout.fillWidth: true

                TextField {
                    id: search_input
                    Layout.fillWidth: true
                    Layout.preferredWidth: 160

                    focus: true
                    font.pointSize: 12
                    placeholderText: qsTr("Search for suttas...")
                    /* Binding on text { */
                    /*     when: webEngineView */
                    /*     value: webEngineView.url */
                    /* } */
                    /* onAccepted: webEngineView.url = Utils.fromUserInput(text) */
                    selectByMouse: true
                }

                Button {
                    id: search_btn
                    icon.source: "icons/32x32/bx_search_alt_2.png"
                    onClicked: aw.show_sutta(search_input.text)
                    // activeFocusOnTab: !aw.platformIsMac
                }
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
                    orientation: Qt.Horizontal
                    Layout.fillWidth: true
                    Layout.fillHeight: true

                    // Left side tabs area
                    ColumnLayout {
                        SplitView.preferredWidth: parent.width * 0.5

                        ColumnLayout {
                            /* Layout.alignment: Qt.AlignTop */

                            WebView {
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

                            TabButton {
                                text: "Results"
                                icon.source: "icons/32x32/bx_search_alt_2.png"
                            }
                            TabButton {
                                text: "History"
                                icon.source: "icons/32x32/fa_clock-rotate-left-solid.png"
                            }
                        }

                        // Tab content areas
                        StackLayout {
                            currentIndex: rightside_tabs.currentIndex
                            anchors.top: rightside_tabs.bottom
                            /* Layout.topMargin: 5 */

                            // Results Tab
                            ColumnLayout {
                                id: fulltext_tab
                                /* anchors.fill: parent */

                                RowLayout {
                                    Layout.fillWidth: true

                                    SpinBox {
                                        id: fulltext_page_input; from: 1; to: 999;
                                        // Layout.alignment: Qt.AlignVCenter
                                    }

                                    Button {
                                        id: fulltext_prev_btn
                                        icon.source: "icons/32x32/fa_angle-left-solid.png"
                                        /* tooltip: qsTr("Previous page of results") */
                                    }
                                    Button {
                                        id: fulltext_next_btn
                                        icon.source: "icons/32x32/fa_angle-right-solid.png"
                                        /* tooltip: qsTr("Next page of results") */
                                    }
                                    Label { id: fulltext_label; text: "Showing a-b out of x" }

                                    // Spacer
                                   Item {
                                        Layout.fillWidth: true
                                    }

                                    Button {
                                        id: fulltext_first_page_btn
                                        icon.source: "icons/32x32/fa_angles-left-solid.png"
                                        /* tooltip: qsTr("First page of results") */
                                    }
                                    Button {
                                        id: fulltext_last_page_btn
                                        icon.source: "icons/32x32/fa_angles-right-solid.png"
                                        /* tooltip: qsTr("Last page of results") */
                                    }
                                }

                                // Rectangle {
                                //     id: fulltext_loading_bar
                                //     Layout.preferredHeight: 5
                                //     color: "transparent"
                                //     border.color: "black"
                                //     Layout.fillWidth: true
                                // }

                                ListView {
                                    id: fulltext_list
                                    Layout.fillWidth: true
                                    Layout.fillHeight: true
                                    /* frameVisible: false */
                                    /* borderWidth: 1 */
                                }
                            }

                            // History Tab
                            ColumnLayout {
                                id: recent_tab
                                /* anchors.fill: parent */

                                ListView { id: recent_list }
                            }
                        }

                    }

                }
            }
        }
    }
}
