import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

// import com.profoundlabs.simsapa 1.0

ApplicationWindow {
    id: aw
    title: qsTr("Simsapa Dhamma Reader - Word Lookup")
    width: 1300
    height: 900
    visible: true
    color: palette.window

    property string word
    property string definition_plain

    Action {
        id: action_quit
        shortcut: StandardKey.Quit
        onTriggered: Qt.quit()
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
    }

    ColumnLayout {
        anchors.centerIn: parent
        spacing: 20

        Text {
            text: aw.word
            font.pixelSize: 16
        }

        Text {
            id: definitionText
            text: aw.definition_plain
            wrapMode: Text.WordWrap
            implicitWidth: parent.width
        }
    }
}
