// QuickSettingsStrip.qml – Compact single-row quick-toggle strip
import QtQuick 2.15
import MonoOS.Shell 1.0

Row {
    id: root
    spacing: 12
    height: 48

    Repeater {
        model: QuickSettingsController.pinnedTiles
        delegate: QSTileCompact { tileData: modelData }
    }

    // Expand arrow
    Rectangle {
        width: 36; height: 36; radius: 18
        color: "#30FFFFFF"
        anchors.verticalCenter: parent.verticalCenter
        Text { anchors.centerIn: parent; text: "⌄"; color: "white"; font.pixelSize: 18 }
        MouseArea { anchors.fill: parent; onClicked: ShadeController.expandQuickSettings() }
    }
}
