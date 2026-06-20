// UnlockArea.qml – "Swipe up to unlock" affordance
import QtQuick 2.15
import OneOS.Shell 1.0

Item {
    id: root
    height: 120
    signal unlockTriggered()
    property bool dragging: false
    property real dragY: 0

    Column {
        anchors.horizontalCenter: parent.horizontalCenter
        anchors.bottom: parent.bottom
        anchors.bottomMargin: 24
        spacing: 6

        Rectangle {
            width: 40; height: 4; radius: 2
            color: "#80FFFFFF"
            anchors.horizontalCenter: parent.horizontalCenter
        }

        Text {
            text: LockController.authMethod === "biometric" ? "Swipe up or use fingerprint"
                  : "Swipe up to unlock"
            color: "#CCFFFFFF"; font.pixelSize: 13
            anchors.horizontalCenter: parent.horizontalCenter
        }
    }

    DragHandler {
        id: drag
        yAxis.minimum: -root.height
        yAxis.maximum: 0
        onActiveChanged: {
            root.dragging = active
            if (!active && drag.centroid.position.y < -48)
                root.unlockTriggered()
        }
    }
}
