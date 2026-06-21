// QuickSettings.qml – Expanded quick-settings panel (two swipes down)
import QtQuick 2.15
import QtQuick.Layouts 1.15
import MonoOS.Shell 1.0

Rectangle {
    id: root
    color: "#E8111111"
    width: Screen.width

    GridLayout {
        anchors { fill: parent; margins: 16 }
        columns: 4
        columnSpacing: 12; rowSpacing: 12

        Repeater {
            model: QuickSettingsController.tiles
            delegate: QSTile { tileData: modelData }
        }
    }
}
