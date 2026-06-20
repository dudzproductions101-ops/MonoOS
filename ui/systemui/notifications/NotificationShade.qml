// NotificationShade.qml – Pull-down notification panel
import QtQuick 2.15
import QtQuick.Controls 2.15
import QtQuick.Layouts 1.15
import OneOS.Shell 1.0

Item {
    id: root
    width: Screen.width
    height: Screen.height

    property real openY: 0
    property real closedY: -height
    y: ShadeController.open ? openY : closedY
    Behavior on y { NumberAnimation { duration: 280; easing.type: Easing.OutCubic } }

    Rectangle {
        anchors.fill: parent
        color: "#E8111111"
    }

    ColumnLayout {
        anchors { top: parent.top; topMargin: 48; left: parent.left; right: parent.right; margins: 0 }
        spacing: 0

        // Quick settings strip
        QuickSettingsStrip {
            Layout.fillWidth: true
            Layout.margins: 16
        }

        // Clear all button
        RowLayout {
            Layout.fillWidth: true
            Layout.leftMargin: 16; Layout.rightMargin: 16

            Text { text: "Notifications"; color: "white"; font.pixelSize: 16; font.weight: Font.Medium }
            Item { Layout.fillWidth: true }
            Text {
                text: "Clear all"; color: "#4DA6FF"; font.pixelSize: 14
                visible: NotificationController.hasNotifications
                MouseArea { anchors.fill: parent; onClicked: NotificationController.clearAll() }
            }
        }

        // Notification list
        ListView {
            id: notifList
            Layout.fillWidth: true
            Layout.fillHeight: true
            height: root.height - 180
            model: NotificationController.allNotifications
            clip: true
            spacing: 8
            leftMargin: 12; rightMargin: 12; topMargin: 8

            delegate: NotificationCard {
                width: notifList.width - 24
                notification: modelData
                onDismissed: NotificationController.dismiss(modelData.id)
            }

            ScrollIndicator.vertical: ScrollIndicator {}
        }
    }

    // Drag to close
    DragHandler {
        yAxis.maximum: 0
        yAxis.minimum: -root.height
        onActiveChanged: if (!active && root.y < -root.height * 0.3) ShadeController.close()
    }
}
