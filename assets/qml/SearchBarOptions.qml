import QtQuick
import QtQuick.Layouts
import QtQuick.Controls

Frame {
    id: row_two_frame

    required property string search_area_text

    property alias search_mode_dropdown: search_mode_dropdown

    background: Rectangle {
        color: "transparent"
        border.color: "transparent"
        border.width: 0
    }

    RowLayout {

        ComboBox {
            id: search_mode_dropdown
            Layout.preferredHeight: 40
            // FIXME implement search types and pass it as SearchParams
            model: {
                if (row_two_frame.search_area_text === "Suttas") {
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
