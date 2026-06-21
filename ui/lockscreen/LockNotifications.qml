// LockNotifications.qml – Compact notification list on the lock screen
import QtQuick 2.15
import QtQuick.Controls 2.15
import MonoOS.Shell 1.0

ListView {
    id: root
    height: Math.min(contentHeight, 240)
    clip: true
    model: NotificationController.lockScreenNotifications

    delegate: Rectangle {
        width: root.width - 32; x: 16
        height: 60; radius: 12
        color: "#30FFFFFF"

        Row {
            anchors { fill: parent; margins: 12 }
            spacing: 10

            Image {
                width: 36; height: 36; anchors.verticalCenter: parent.verticalCenter
                source: modelData.smallIcon; fillMode: Image.PreserveAspectFit
            }

            Column {
                anchors.verticalCenter: parent.verticalCenter
                Text { text: modelData.title; color: "white"; font.pixelSize: 13; font.bold: true }
                Text { text: modelData.body;  color: "#CCFFFFFF"; font.pixelSize: 12; elide: Text.ElideRight; width: root.width - 100 }
            }
        }
    }

    ScrollIndicator.vertical: ScrollIndicator {}
}
