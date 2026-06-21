// NetworkSettings.qml – Network & internet settings page
import QtQuick 2.15
import QtQuick.Controls 2.15
import MonoOS.Shell 1.0

Page {
    background: Rectangle { color: ThemeEngine.background }
    header: ToolBar {
        background: Rectangle { color: ThemeEngine.surface }
        Label { text: "Network & Internet"; font: ThemeEngine.title3; color: ThemeEngine.labelPrimary; anchors.centerIn: parent }
    }

    Column {
        anchors { top: parent.top; left: parent.left; right: parent.right; margins: 16 }
        spacing: 20

        // Wi-Fi section
        Rectangle {
            width: parent.width; height: childrenRect.height; radius: 14; color: ThemeEngine.surface; clip: true
            Column {
                width: parent.width
                SettingsRow { rowData: NetworkController.wifiRow; showDivider: false; width: parent.width }
            }
        }

        // Cellular section
        Rectangle {
            width: parent.width; height: childrenRect.height; radius: 14; color: ThemeEngine.surface; clip: true
            Column {
                width: parent.width
                SettingsRow { rowData: NetworkController.cellularRow; showDivider: true; width: parent.width }
                SettingsRow { rowData: NetworkController.apnRow;      showDivider: false; width: parent.width }
            }
        }

        // VPN / DNS
        Rectangle {
            width: parent.width; height: childrenRect.height; radius: 14; color: ThemeEngine.surface; clip: true
            Column {
                width: parent.width
                SettingsRow { rowData: NetworkController.vpnRow;       showDivider: true;  width: parent.width }
                SettingsRow { rowData: NetworkController.privateDnsRow; showDivider: false; width: parent.width }
            }
        }
    }
}
