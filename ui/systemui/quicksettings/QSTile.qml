// QSTile.qml – A single quick-settings tile (expanded grid)
import QtQuick 2.15
import OneOS.Shell 1.0

Rectangle {
    id: root
    property var tileData: null
    width: 78; height: 78; radius: 14
    color: (tileData && tileData.active) ? "#4DA6FF" : "#30FFFFFF"
    Behavior on color { ColorAnimation { duration: 150 } }

    Column {
        anchors.centerIn: parent; spacing: 6

        Text {
            anchors.horizontalCenter: parent.horizontalCenter
            text: root.tileData ? root.tileData.icon : ""
            font.pixelSize: 24; color: "white"
        }

        Text {
            anchors.horizontalCenter: parent.horizontalCenter
            text: root.tileData ? root.tileData.label : ""
            font.pixelSize: 10; color: "white"
        }
    }

    MouseArea { anchors.fill: parent; onClicked: if (root.tileData) QuickSettingsController.toggle(root.tileData.id) }
}
