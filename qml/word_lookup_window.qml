import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

import com.profound_labs.simsapa 1.0

Item {
    anchors.fill: parent
    /* title: "Word Definition" */
    id: aw
    visible: true
    /* width: 1300 */
    /* height: 900 */
    /* color: palette.window */

    property string word
    property string definition_plain

    ColumnLayout {
        anchors.centerIn: parent
        spacing: 20

        Text {
            text: word
            font.pixelSize: 16
        }

        Text {
            id: definitionText
            text: definition_plain
            wrapMode: Text.WordWrap
            width: parent.width
        }
    }
}
