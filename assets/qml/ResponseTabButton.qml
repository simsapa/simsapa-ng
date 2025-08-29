pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

TabButton {
    id: control
    
    property string model_name: ""
    property string status: "waiting"  // "waiting", "completed", "error"
    property int retry_count: 0
    
    property alias retry_btn: retry_btn
    
    signal retryRequested()
    
    contentItem: RowLayout {
        spacing: 5
        
        Image {
            id: status_icon
            Layout.preferredWidth: 16
            Layout.preferredHeight: 16
            Layout.alignment: Qt.AlignVCenter
            
            source: {
                if (control.status === "waiting") {
                    return "icons/32x32/fa_stopwatch-solid.png"
                } else if (control.status === "completed") {
                    return "icons/32x32/fa_square-check-solid.png"
                } else if (control.status === "error") {
                    return "icons/32x32/fa_triangle-exclamation-solid.png"
                } else {
                    return "icons/32x32/fa_stopwatch-solid.png"
                }
            }
        }
        
        Text {
            Layout.fillWidth: true
            text: control.model_name
            font: control.font
            elide: Text.ElideRight
            horizontalAlignment: Text.AlignLeft
            verticalAlignment: Text.AlignVCenter
        }
        
        Button {
            id: retry_btn
            Layout.preferredWidth: 20
            Layout.preferredHeight: 20
            Layout.alignment: Qt.AlignVCenter
            visible: control.status === "error"
            text: "↻"
            font.pixelSize: 12
            onClicked: {
                control.retryRequested()
            }
            
            ToolTip.visible: hovered
            ToolTip.text: `Retry request (attempted ${control.retry_count}x)`
        }
    }
}
