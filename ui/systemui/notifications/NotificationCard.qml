// NotificationCard.qml – A single dismissible notification card
import QtQuick 2.15
import QtQuick.Layouts 1.15
import MonoOS.Shell 1.0

Rectangle {
    id: root
    property var notification: null
    signal dismissed()
    height: 72; radius: 14
    color: "#2AFFFFFF"
    clip: true

    // Swipe to dismiss
    transform: Translate { x: swipe.translation.x }
    property alias swipeX: swipe.translation.x

    DragHandler {
        id: swipe
        xAxis.minimum: -root.width; xAxis.maximum: root.width
        onActiveChanged: {
            if (!active && Math.abs(translation.x) > root.width * 0.4)
                root.dismissed()
            else if (!active)
                xAxis.resetPersistantValue()
        }
    }

    RowLayout {
        anchors { fill: parent; margins: 14 }
        spacing: 12

        // App icon
        Image {
            width: 36; height: 36
            source: root.notification ? root.notification.smallIcon : ""
            fillMode: Image.PreserveAspectFit
        }

        // Text
        Column {
            Layout.fillWidth: true
            spacing: 3
            Text {
                text: root.notification ? root.notification.title : ""
                color: "white"; font.pixelSize: 14; font.weight: Font.Medium
                elide: Text.ElideRight; width: parent.width
            }
            Text {
                text: root.notification ? root.notification.body : ""
                color: "#CCFFFFFF"; font.pixelSize: 12
                elide: Text.ElideRight; width: parent.width
            }
        }

        // Timestamp
        Text {
            text: root.notification ? root.notification.timeAgo : ""
            color: "#80FFFFFF"; font.pixelSize: 11
        }
    }

    MouseArea {
        anchors.fill: parent
        onClicked: {
            NotificationController.open(root.notification)
            ShadeController.close()
        }
    }
}
