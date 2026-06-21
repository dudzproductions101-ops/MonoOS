// QSTileCompact.qml – Compact (strip) version of a QS tile
import QtQuick 2.15
import MonoOS.Shell 1.0

Rectangle {
    id: root
    property var tileData: null
    width: 40; height: 40; radius: 20
    color: (tileData && tileData.active) ? "#4DA6FF" : "#30FFFFFF"
    Behavior on color { ColorAnimation { duration: 120 } }

    Text {
        anchors.centerIn: parent
        text: root.tileData ? root.tileData.icon : ""
        font.pixelSize: 18; color: "white"
    }

    MouseArea { anchors.fill: parent; onClicked: if (root.tileData) QuickSettingsController.toggle(root.tileData.id) }
}
