// AboutSettings.qml – About MonoOS device information page
import QtQuick 2.15
import QtQuick.Controls 2.15
import QtQuick.Layouts 1.15
import MonoOS.Shell 1.0

Page {
    background: Rectangle { color: ThemeEngine.background }

    ScrollView {
        anchors.fill: parent
        Column {
            width: parent ? parent.width : Screen.width
            spacing: 0

            // MonoOS logo / version hero
            Rectangle {
                width: parent.width; height: 220
                color: ThemeEngine.surface
                Column {
                    anchors.centerIn: parent; spacing: 12
                    Text { text: "MonoOS"; font { pixelSize: 42; weight: Font.Thin }; color: ThemeEngine.labelPrimary; anchors.horizontalCenter: parent.horizontalCenter }
                    Text { text: "by DudasCorp"; font: ThemeEngine.subheadline; color: ThemeEngine.labelSecondary; anchors.horizontalCenter: parent.horizontalCenter }
                    Rectangle {
                        width: 120; height: 28; radius: 14; color: ThemeEngine.accent
                        anchors.horizontalCenter: parent.horizontalCenter
                        Text { anchors.centerIn: parent; text: AboutController.versionString; color: "white"; font: ThemeEngine.footnote }
                    }
                }
            }

            Rectangle {
                width: parent.width - 32; x: 16; height: childrenRect.height
                radius: 14; color: ThemeEngine.surface; clip: true
                Column {
                    width: parent.width
                    AboutRow { label: "Build number";     value: AboutController.buildNumber }
                    AboutRow { label: "Android security"; value: AboutController.securityPatch; divider: true }
                    AboutRow { label: "Kernel version";   value: AboutController.kernelVersion }
                    AboutRow { label: "Device model";     value: AboutController.deviceModel }
                    AboutRow { label: "Serial number";    value: AboutController.serialNumber; divider: false }
                }
            }

            // Tappable update row
            Rectangle {
                width: parent.width - 32; x: 16; height: 52; radius: 14; color: ThemeEngine.surface
                Row {
                    anchors { fill: parent; leftMargin: 16; rightMargin: 16 }
                    Text { text: "Software updates"; font: ThemeEngine.body; color: ThemeEngine.labelPrimary; anchors.verticalCenter: parent.verticalCenter; Layout.fillWidth: true }
                }
                MouseArea { anchors.fill: parent; onClicked: AboutController.checkUpdates() }
            }
        }
    }
}
