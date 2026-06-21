// BatteryIcon.qml – Battery level indicator in the status bar
import QtQuick 2.15

Row {
    id: root
    property int level: 100
    property bool charging: false
    spacing: 3

    // Charging bolt
    Text {
        visible: root.charging
        text: "⚡"; font.pixelSize: 11; color: "#FFD60A"
        anchors.verticalCenter: parent.verticalCenter
    }

    // Battery outline
    Rectangle {
        width: 22; height: 12; radius: 2
        color: "transparent"
        border.color: "white"; border.width: 1.5
        anchors.verticalCenter: parent.verticalCenter

        // Nub
        Rectangle {
            width: 2; height: 5; radius: 1
            color: "white"
            anchors { left: parent.right; verticalCenter: parent.verticalCenter; leftMargin: 1 }
        }

        // Fill
        Rectangle {
            x: 2; y: 2
            width: Math.max(0, (parent.width - 4) * root.level / 100)
            height: parent.height - 4
            radius: 1
            color: root.level <= 20 ? "#FF3B30" : root.charging ? "#30D158" : "white"
            Behavior on width { NumberAnimation { duration: 300 } }
        }
    }

    Text {
        text: root.level + "%"
        color: "white"; font.pixelSize: 11
        anchors.verticalCenter: parent.verticalCenter
    }
}
