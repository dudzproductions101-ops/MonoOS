// StatusBar.qml – System status bar (top of screen)
import QtQuick 2.15
import QtQuick.Layouts 1.15
import MonoOS.Shell 1.0

Rectangle {
    id: root
    width: Screen.width
    height: 28 + safeAreaTop
    color: StatusBarController.transparent ? "transparent" : "#99000000"
    property real safeAreaTop: 12

    RowLayout {
        anchors {
            fill: parent
            topMargin: safeAreaTop
            leftMargin: 16; rightMargin: 16
        }

        // Clock
        Text {
            text: Qt.formatTime(new Date(), "HH:mm")
            color: "white"; font.pixelSize: 13; font.weight: Font.Medium
            Timer { interval: 30000; running: true; repeat: true; onTriggered: parent.text = Qt.formatTime(new Date(), "HH:mm") }
        }

        Item { Layout.fillWidth: true }

        // Signal strength
        StatusSignalIcon { strength: ModemController.signalBars; visible: ModemController.registered }

        // WiFi
        StatusWifiIcon { strength: WifiController.signalBars; visible: WifiController.connected }

        // Battery
        BatteryIcon { level: PowerController.batteryLevel; charging: PowerController.charging }

        // Notification dot
        Rectangle {
            visible: NotificationController.hasUnread
            width: 6; height: 6; radius: 3; color: "#4DA6FF"
        }
    }

    Behavior on color { ColorAnimation { duration: 200 } }
}
