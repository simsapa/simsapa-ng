import QtQuick
import QtQuick.Layouts
import QtQuick.Controls

Frame {
    id: row_two_frame

    background: Rectangle {
        color: "transparent"
        border.color: "transparent"
        border.width: 0
    }

    RowLayout {

        ComboBox {
            id: search_mode_dropdown
            Layout.preferredHeight: 40
            model: [
                "Fulltext Match",
                "Contains Match",
                "Title Match",
                "RegEx Match",
            ]
        }

        Button {
            id: language_include_btn
            checkable: true
            icon.source: "icons/32x32/fa_plus-solid.png"
            Layout.preferredHeight: 40
            Layout.preferredWidth: 40
            ToolTip.visible: hovered
            ToolTip.text: "+ means 'must include', - means 'must exclude'"
        }

        ComboBox {
            id: language_filter_dropdown
            Layout.preferredHeight: 40
            model: [
                "Language",
                "en",
                "pli",
            ]
        }

        Button {
            id: source_include_btn
            checkable: true
            icon.source: "icons/32x32/fa_plus-solid.png"
            Layout.preferredHeight: 40
            Layout.preferredWidth: 40
            ToolTip.visible: hovered
            ToolTip.text: "+ means 'must include', - means 'must exclude'"
        }

        ComboBox {
            id: source_filter_dropdown
            Layout.preferredHeight: 40
            model: [
                "Sources",
                "ms",
                "cst4",
            ]
        }
    }
}
