// AppIcon.qml – Reusable app icon with label and badge
import QtQuick 2.15
import QtQuick.Controls 2.15

Item {
    id: root
    property var appInfo: null
    property real iconSize: 52
    property bool showLabel: false
    signal tapped()
    signal longPressed()

    Column {
        anchors.centerIn: parent
        spacing: 4

        // Icon
        Rectangle {
            id: iconBg
            width: root.iconSize; height: root.iconSize
            radius: width * 0.22
            color: "#1AFFFFFF"
            anchors.horizontalCenter: parent.horizontalCenter

            Image {
                anchors.fill: parent
                anchors.margins: 4
                source: root.appInfo ? root.appInfo.iconPath : ""
                fillMode: Image.PreserveAspectFit
                asynchronous: true
            }

            // Unread badge
            Rectangle {
                visible: root.appInfo && root.appInfo.badgeCount > 0
                anchors { top: parent.top; right: parent.right; margins: -3 }
                width: 18; height: 18; radius: 9
                color: "#FF3B30"
                Text {
                    anchors.centerIn: parent
                    text: root.appInfo ? Math.min(root.appInfo.badgeCount, 99).toString() : ""
                    font.pixelSize: 10; font.bold: true
                    color: "white"
                }
            }
        }

        // Label
        Text {
            visible: root.showLabel
            text: root.appInfo ? root.appInfo.label : ""
            color: "white"
            font.pixelSize: 11
            horizontalAlignment: Text.AlignHCenter
            width: root.iconSize + 12
            anchors.horizontalCenter: parent.horizontalCenter
            elide: Text.ElideRight
            style: Text.Raised
            styleColor: "#60000000"
        }
    }

    scale: tapHandler.pressed ? 0.92 : 1.0
    Behavior on scale { NumberAnimation { duration: 80 } }

    TapHandler {
        id: tapHandler
        onTapped: root.tapped()
        onLongPressed: root.longPressed()
    }
}
