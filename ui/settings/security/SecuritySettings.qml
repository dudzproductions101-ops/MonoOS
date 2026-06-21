// SecuritySettings.qml – Security settings page
import QtQuick 2.15
import QtQuick.Controls 2.15
import MonoOS.Shell 1.0

Page {
    background: Rectangle { color: ThemeEngine.background }
    header: ToolBar {
        background: Rectangle { color: ThemeEngine.surface }
        Label { text: "Security"; font: ThemeEngine.title3; color: ThemeEngine.labelPrimary; anchors.centerIn: parent }
    }

    Column {
        anchors { top: parent.top; left: parent.left; right: parent.right; margins: 16 }
        spacing: 20

        Rectangle {
            width: parent.width; height: childrenRect.height; radius: 14; color: ThemeEngine.surface; clip: true
            Column {
                width: parent.width
                SettingsRow { rowData: SecurityController.screenLockRow;    showDivider: true; width: parent.width }
                SettingsRow { rowData: SecurityController.biometricRow;     showDivider: true; width: parent.width }
                SettingsRow { rowData: SecurityController.encryptionRow;    showDivider: true; width: parent.width }
                SettingsRow { rowData: SecurityController.secureBoot;       showDivider: false; width: parent.width }
            }
        }

        Rectangle {
            width: parent.width; height: childrenRect.height; radius: 14; color: ThemeEngine.surface; clip: true
            Column {
                width: parent.width
                SettingsRow { rowData: SecurityController.findMyDeviceRow;  showDivider: true; width: parent.width }
                SettingsRow { rowData: SecurityController.appPermissionsRow; showDivider: false; width: parent.width }
            }
        }
    }
}
