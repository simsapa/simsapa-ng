// Type stub for qmllint
import QtQuick
import QtQuick.Layouts
import QtQuick.Controls

ColumnLayout {
    // Configurable properties
    property var model: []
    property var selected_languages: []
    property string section_title: "Select Languages"
    property string instruction_text: "Type language codes below, or click languages to select/unselect them."
    property string placeholder_text: "E.g.: en, fr, es"
    property string available_label: "Available languages (click to select):"
    property bool show_count_column: false
    property int font_point_size: 12
    
    // Expose the text field for parent access
    property TextField language_input: TextField {}

    // Signal emitted when selection changes
    signal languageSelectionChanged(selected_codes: var)

    // Public function to get selected language codes
    function get_selected_languages(): var {
        return [];
    }
}
