// StatusSignalIcon.qml – Cellular signal bars
import QtQuick 2.15

Row {
    id: root
    property int strength: 0  // 0-4
    spacing: 2

    Repeater {
        model: 4
        delegate: Rectangle {
            width: 3
            height: 4 + index * 2
            radius: 1
            color: index < root.strength ? "white" : "#40FFFFFF"
            anchors.bottom: parent ? parent.bottom : undefined
        }
    }
}
