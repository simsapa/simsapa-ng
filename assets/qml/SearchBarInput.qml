import QtQuick
import QtQuick.Layouts
import QtQuick.Controls

Frame {
    id: root
    Layout.fillWidth: true
    Layout.minimumHeight: 40

    readonly property bool is_mobile: Qt.platform.os === "android" || Qt.platform.os === "ios"
    readonly property bool is_desktop: !root.is_mobile

    required property bool is_wide
    required property bool db_loaded
    required property var handle_query_fn
    required property Timer search_timer
    required property Action search_as_you_type
    required property bool is_loading

    property alias search_input: search_input
    property alias search_area_dropdown: search_area_dropdown
    property alias search_mode_dropdown: search_mode_dropdown

    background: Rectangle {
        color: "transparent"
        border.color: "transparent"
        border.width: 0
    }

    function user_typed() {
        // TODO self._show_search_normal_icon()
        if (root.search_as_you_type.checked) root.search_timer.restart();
    }

    RowLayout {
        id: search_bar_layout
        Layout.fillWidth: true

        RowLayout {
            id: search_input_layout
            Layout.fillWidth: true

            // === Search Input ====
            TextField {
                id: search_input
                enabled: root.db_loaded
                Layout.fillWidth: true
                Layout.preferredWidth: root.is_wide ? 500 : 250
                Layout.preferredHeight: 40

                focus: true
                font.pointSize: root.is_mobile ? 14 : 12
                placeholderText: root.db_loaded ? (search_area_dropdown.currentText === "Dictionary" ? "Search in dictionary" : "Search in suttas") : "Loading..."

                onAccepted: search_btn.clicked()
                onTextChanged: root.user_typed()
                selectByMouse: true
            }

            Button {
                id: search_btn
                icon.source: root.is_loading ? "icons/32x32/fa_stopwatch-solid.png" : "icons/32x32/bx_search_alt_2.png"
                enabled: search_input.text.length > 0
                onClicked: root.handle_query_fn(search_input.text, 1) // qmllint disable use-proper-function
                Layout.preferredHeight: 40
                Layout.preferredWidth: 40
            }
        }

        RowLayout {
            id: search_options_layout
            Layout.fillWidth: true
            visible: (root.width - 550) > 550 // FIXME

            ComboBox {
                id: search_area_dropdown
                Layout.preferredHeight: 40
                Layout.preferredWidth: root.is_mobile ? 120 : 100
                currentIndex: 0 // Default to "Suttas"
                model: [
                    "Suttas",
                    "Dictionary",
                ]
            }

            ComboBox {
                id: search_mode_dropdown
                Layout.preferredHeight: 40
                // FIXME implement search types and pass it as SearchParams
                model: {
                    if (search_area_dropdown.currentText === "Suttas") {
                        return [
                                /* "Fulltext Match", */
                                "Contains Match",
                                /* "Title Match", */
                                /* "RegEx Match", */
                        ];
                    } else {
                        return [
                                "DPD Lookup",
                                "Contains Match",
                        ];
                    }
                }
            }

            // Button {
            //     id: language_include_btn
            //     checkable: true
            //     icon.source: "icons/32x32/fa_plus-solid.png"
            //     Layout.preferredHeight: 40
            //     Layout.preferredWidth: 40
            //     ToolTip.visible: hovered
            //     ToolTip.text: "+ means 'must include', - means 'must exclude'"
            // }

            // ComboBox {
            //     id: language_filter_dropdown
            //     Layout.preferredHeight: 40
            //     model: [
            //         "Language",
            //         "en",
            //         "pli",
            //     ]
            // }

            // Button {
            //     id: source_include_btn
            //     checkable: true
            //     icon.source: "icons/32x32/fa_plus-solid.png"
            //     Layout.preferredHeight: 40
            //     Layout.preferredWidth: 40
            //     ToolTip.visible: hovered
            //     ToolTip.text: "+ means 'must include', - means 'must exclude'"
            // }

            // ComboBox {
            //     id: source_filter_dropdown
            //     Layout.preferredHeight: 40
            //     model: [
            //         "Sources",
            //         "ms",
            //         "cst4",
            //     ]
            // }
        }
    }
}
