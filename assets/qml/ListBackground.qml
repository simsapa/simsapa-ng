pragma ComponentBehavior: Bound

import QtQuick

Rectangle {
    id: row_rect
    anchors.fill: parent
    radius: 5 // slight rounding for a button feel
    border.width: 1
    border.color: Qt.darker(base_color, 1.15)

    required property ListView results_list
    required property int result_item_index

    property color even_color: "#E6E6E6" // 90%
    property color odd_color: "#FFFFFF"
    property color selected_color: "#A0C4FF"
    property color base_color: (result_item_index % 2 === 0 ? even_color : odd_color)

    color: results_list.currentIndex === result_item_index ? row_rect.selected_color : row_rect.base_color

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
