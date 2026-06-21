// main.qml – MonoOS Developer Tools App
import QtQuick 2.15
import QtQuick.Controls 2.15
import QtQuick.Layouts 1.15
import MonoOS.Apps 1.0

ApplicationWindow {
    visible: true; width: Screen.width; height: Screen.height
    title: "Developer Tools"; color: ThemeEngine.background

    TabBar {
        id: tabBar; width: parent.width; anchors.top: parent.top

        TabButton { text: "System" }
        TabButton { text: "Network" }
        TabButton { text: "Logcat" }
        TabButton { text: "Performance" }
    }

    StackLayout {
        anchors { top: tabBar.bottom; bottom: parent.bottom; left: parent.left; right: parent.right }
        currentIndex: tabBar.currentIndex

        // System info
        ScrollView {
            ListView {
                model: DevToolsController.systemInfo
                delegate: Rectangle {
                    width: ListView.view.width; height: 52
                    color: ThemeEngine.surface
                    Row {
                        anchors { fill: parent; leftMargin: 16; rightMargin: 16 }
                        Text { text: modelData.key;   width: 180; font.pixelSize: 13; color: ThemeEngine.labelSecondary; anchors.verticalCenter: parent.verticalCenter }
                        Text { text: modelData.value; font.pixelSize: 13; color: ThemeEngine.labelPrimary; anchors.verticalCenter: parent.verticalCenter; elide: Text.ElideRight }
                    }
                    Rectangle { anchors { bottom: parent.bottom; left: parent.left; right: parent.right; leftMargin: 16 }; height: 0.5; color: ThemeEngine.border }
                }
            }
        }

        // Network monitor
        Column {
            spacing: 12; padding: 16
            Rectangle { width: parent.width - 32; height: 80; radius: 12; color: ThemeEngine.surface
                Column { anchors.centerIn: parent; spacing: 4
                    Text { text: DevToolsController.netTxRate; font.pixelSize: 22; color: ThemeEngine.accent; anchors.horizontalCenter: parent.horizontalCenter }
                    Text { text: "Upload"; font.pixelSize: 12; color: ThemeEngine.labelSecondary; anchors.horizontalCenter: parent.horizontalCenter }
                }
            }
            Rectangle { width: parent.width - 32; height: 80; radius: 12; color: ThemeEngine.surface
                Column { anchors.centerIn: parent; spacing: 4
                    Text { text: DevToolsController.netRxRate; font.pixelSize: 22; color: ThemeEngine.accentSecondary; anchors.horizontalCenter: parent.horizontalCenter }
                    Text { text: "Download"; font.pixelSize: 12; color: ThemeEngine.labelSecondary; anchors.horizontalCenter: parent.horizontalCenter }
                }
            }
        }

        // Logcat
        Rectangle {
            color: "#0D0D0D"
            Column {
                anchors.fill: parent; spacing: 0

                Row {
                    width: parent.width; height: 44; spacing: 8; padding: 8
                    TextField { width: parent.width - 120; placeholderText: "Filter…"; onTextChanged: DevToolsController.setLogFilter(text)
                                background: Rectangle { radius: 8; color: "#333" }; color: "white" }
                    Rectangle { width: 60; height: 36; radius: 8; color: "#333"
                        Text { anchors.centerIn: parent; text: DevToolsController.logPauseLabel; color: "white"; font.pixelSize: 13 }
                        MouseArea { anchors.fill: parent; onClicked: DevToolsController.toggleLogPause() }
                    }
                }

                ListView {
                    id: logView; width: parent.width; height: parent.height - 44
                    model: DevToolsController.logLines; clip: true

                    delegate: Text {
                        width: logView.width; leftPadding: 8
                        text: modelData.text
                        color: modelData.level === "E" ? "#FF5555" : modelData.level === "W" ? "#FFAA00" : "#AAFFAA"
                        font { family: "Courier New"; pixelSize: 11 }
                        wrapMode: Text.WrapAnywhere
                    }

                    onCountChanged: if (!DevToolsController.logPaused) positionViewAtEnd()
                }
            }
        }

        // Performance
        Column {
            spacing: 16; padding: 16
            PerfGraph { title: "CPU"; value: DevToolsController.cpuPercent; color: ThemeEngine.accent }
            PerfGraph { title: "RAM"; value: DevToolsController.ramPercent; color: ThemeEngine.accentSecondary }
            PerfGraph { title: "GPU"; value: DevToolsController.gpuPercent; color: "#FF9F0A" }
            PerfGraph { title: "Storage I/O"; value: DevToolsController.ioPercent; color: "#BF5AF2" }
        }
    }
}
