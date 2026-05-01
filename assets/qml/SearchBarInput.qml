import QtQuick
import QtQuick.Layouts
import QtQuick.Controls

import com.profoundlabs.simsapa

Frame {
    id: root
    Layout.fillWidth: true
    Layout.minimumHeight: root.icon_size

    readonly property bool is_mobile: Qt.platform.os === "android" || Qt.platform.os === "ios"
    readonly property bool is_desktop: !root.is_mobile

    required property int window_width
    required property bool is_wide
    required property bool is_tall
    required property bool db_loaded
    required property var handle_query_fn
    required property Timer search_timer
    required property DrawerMenu mobile_menu
    required property bool search_as_you_type_checked
    required property bool is_loading
    required property bool has_query_error

    required property int icon_size

    property alias search_input: search_input
    property alias search_mode_dropdown: search_mode_dropdown
    property alias language_filter_dropdown: language_filter_dropdown
    property alias advanced_options_btn: advanced_options_btn

    // Search area state: "Suttas", "Dictionary", or "Library"
    property string search_area: "Suttas"
    readonly property var search_area_list: ["Suttas", "Dictionary", "Library"]

    function set_search_area(area: string) {
        if (search_area_list.indexOf(area) !== -1) {
            search_area = area;
        }
    }

    function cycle_search_area() {
        const current_index = search_area_list.indexOf(search_area);
        const next_index = (current_index + 1) % search_area_list.length;
        search_area = search_area_list[next_index];
    }

    background: Rectangle {
        color: "transparent"
        border.color: "transparent"
        border.width: 0
    }

    function load_language_labels_for_area(area: string) {
        let lang_labels;
        if (area === "Suttas") {
            lang_labels = SuttaBridge.get_sutta_language_labels();
        } else if (area === "Library") {
            lang_labels = SuttaBridge.get_library_language_labels();
        } else {
            // Dictionary: language filter not used
            lang_labels = [];
        }
        // Shorter first label for narrow screens.
        const first_label = root.is_wide ? "Language" : "Lang";
        language_filter_dropdown.model = [first_label].concat(lang_labels);
        language_filter_dropdown.currentIndex = 0;
    }

    Component.onCompleted: {
        load_language_labels_for_area(search_area);

        // Restore saved language filter key
        const saved_key = SuttaBridge.get_sutta_language_filter_key();
        if (saved_key) {
            const saved_index = language_filter_dropdown.model.indexOf(saved_key);
            if (saved_index !== -1) {
                language_filter_dropdown.currentIndex = saved_index;
            }
        }
    }

    onSearch_areaChanged: {
        load_language_labels_for_area(search_area);
    }

    onIs_wideChanged: {
        const saved_index = language_filter_dropdown.currentIndex;
        load_language_labels_for_area(search_area);
        language_filter_dropdown.currentIndex = saved_index;
    }

    function user_typed() {
        // TODO self._show_search_normal_icon()
        if (root.search_as_you_type_checked) root.search_timer.restart();
    }

    Flow {
        id: search_bar_layout
        width: parent.width
        spacing: 5

        RowLayout {
            id: search_input_layout
            /* On wide screens, constrain the search input to 600px width so the
             * options can sit beside it. On narrow screens, let it take full
             * width, which will push the options to wrap below. */
            width: root.is_wide ? 600 : parent.width

            // Open/close the drawer menu on mobile
            Button {
                id: show_menu
                visible: root.is_mobile
                icon.source: "icons/32x32/mdi--menu.png"
                Layout.preferredHeight: root.icon_size
                Layout.preferredWidth: root.icon_size
                ToolTip.visible: hovered
                ToolTip.text: "Show Menu"
                onClicked: mobile_menu.open()
            }

            // === Search Input ====
            TextField {
                id: search_input
                enabled: root.db_loaded
                Layout.fillWidth: true
                Layout.preferredWidth: root.is_wide ? 500 : 250
                Layout.preferredHeight: root.icon_size

                focus: true
                font.pointSize: root.is_mobile ? 14 : 12
                placeholderText: {
                    if (!root.db_loaded) return "Loading...";
                    if (root.search_area === "Dictionary") return "Search in dictionary";
                    if (root.search_area === "Library") return "Search in library";
                    return "Search in suttas";
                }

                onAccepted: search_btn.clicked()
                onTextChanged: root.user_typed()
                selectByMouse: true
            }

            Button {
                id: search_btn
                icon.source: root.has_query_error ? "icons/32x32/fa_triangle-exclamation-solid.png" : (root.is_loading ? "icons/32x32/fa_stopwatch-solid.png" : "icons/32x32/bx_search_alt_2.png")
                enabled: search_input.text.length > 0
                onClicked: root.handle_query_fn(search_input.text, 1) // qmllint disable use-proper-function
                Layout.preferredHeight: root.icon_size
                Layout.preferredWidth: root.icon_size
            }
        }

        RowLayout {
            id: search_options_layout

            Button {
                id: advanced_options_btn
                checkable: true
                icon.source: "icons/32x32/system-uicons--settings.png"
                Layout.preferredHeight: root.icon_size
                Layout.preferredWidth: root.icon_size
                ToolTip.visible: hovered
                ToolTip.text: "Advanced search options"
            }

            // Search area buttons (S = Suttas, D = Dictionary, L = Library)
            Row {
                id: search_area_buttons
                spacing: 0

                Button {
                    id: btn_suttas
                    text: "S"
                    checked: root.search_area === "Suttas"
                    checkable: true
                    autoExclusive: true
                    implicitWidth: root.icon_size
                    implicitHeight: root.icon_size
                    ToolTip.visible: hovered
                    ToolTip.text: "Suttas"
                    onClicked: root.search_area = "Suttas"
                }

                Button {
                    id: btn_dictionary
                    text: "D"
                    checked: root.search_area === "Dictionary"
                    checkable: true
                    autoExclusive: true
                    implicitWidth: root.icon_size
                    implicitHeight: root.icon_size
                    ToolTip.visible: hovered
                    ToolTip.text: "Dictionary"
                    onClicked: root.search_area = "Dictionary"
                }

                Button {
                    id: btn_library
                    text: "L"
                    checked: root.search_area === "Library"
                    checkable: true
                    autoExclusive: true
                    implicitWidth: root.icon_size
                    implicitHeight: root.icon_size
                    ToolTip.visible: hovered
                    ToolTip.text: "Library"
                    onClicked: root.search_area = "Library"
                }
            }

            ComboBox {
                id: search_mode_dropdown
                Layout.preferredHeight: root.icon_size
                Layout.preferredWidth: root.is_wide ? 120 : 80

                readonly property var search_mode_label_wide: {
                    "Suttas": [
                        "Fulltext Match",
                        "Contains Match",
                        "Title Match",
                    ],
                    "Library": [
                        "Fulltext Match",
                        "Contains Match",
                        "Title Match",
                    ],
                    "Dictionary": [
                        "DPD Lookup",
                        "Fulltext Match",
                        "Contains Match",
                        "Headword Match",
                    ],
                }

                // For narrow screen, show shorter label texts.
                // Value reading uses get_text(), which will return the longer label text,
                // which is used for the JSON search parameters.
                readonly property var search_mode_label_narrow: {
                    "Suttas": [
                        "Fulltext",
                        "Contains",
                        "Title",
                    ],
                    "Library": [
                        "Fulltext",
                        "Contains",
                        "Title",
                    ],
                    "Dictionary": [
                        "Lookup",
                        "Fulltext",
                        "Contains",
                        "Headword",
                    ],
                }

                model: {
                    if (root.is_wide) {
                        return search_mode_label_wide[root.search_area];
                    } else {
                        return search_mode_label_narrow[root.search_area];
                    }
                }

                onCurrentIndexChanged: root.handle_query_fn(search_input.text) // qmllint disable use-proper-function

                function get_text(): string {
                    // Return the value using the wide values which is expected for JSON search parameters.
                    return search_mode_label_wide[root.search_area][currentIndex];
                }
            }

            // Button {
            //     id: language_include_btn
            //     checkable: true
            //     icon.source: "icons/32x32/fa_plus-solid.png"
            //     Layout.preferredHeight: root.icon_size
            //     Layout.preferredWidth: root.icon_size
            //     ToolTip.visible: hovered
            //     ToolTip.text: "+ means 'must include', - means 'must exclude'"
            // }

            ComboBox {
                id: language_filter_dropdown
                Layout.preferredHeight: root.icon_size
                Layout.preferredWidth: root.is_wide ? 120 : 80
                // Shorter first label for narrow screens.
                model: root.is_wide ? ["Language",] : ["Lang",]
                enabled: root.search_area === "Suttas" || root.search_area === "Library"
                onCurrentIndexChanged: {
                    // Save the language filter selection
                    if (enabled) {
                        // currentIndex changed but currentText have not yet updated.
                        // Have to get the text manually from the model list.
                        const lang_key = language_filter_dropdown.model[currentIndex];
                        if (lang_key) {
                            SuttaBridge.set_sutta_language_filter_key(lang_key);
                        }
                        // Re-run search (handle_query will check text min length)
                        root.handle_query_fn(search_input.text); // qmllint disable use-proper-function
                    }
                }

                function get_text(): string {
                    // Always return "Language" for index 0, because it is a fixed keyword for
                    // "no language filter is selected".
                    if (currentIndex === 0) {
                        return "Language";
                    } else {
                        return model[currentIndex];
                    }
                }
            }

            // Button {
            //     id: source_include_btn
            //     checkable: true
            //     icon.source: "icons/32x32/fa_plus-solid.png"
            //     Layout.preferredHeight: root.icon_size
            //     Layout.preferredWidth: root.icon_size
            //     ToolTip.visible: hovered
            //     ToolTip.text: "+ means 'must include', - means 'must exclude'"
            // }

            // ComboBox {
            //     id: source_filter_dropdown
            //     Layout.preferredHeight: root.icon_size
            //     model: [
            //         "Sources",
            //         "ms",
            //         "cst",
            //     ]
            // }
        }
    }
}
