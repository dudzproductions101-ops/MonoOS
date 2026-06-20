// AppDrawer.qml – Full-screen alphabetical app drawer
import QtQuick 2.15
import QtQuick.Controls 2.15
import QtQuick.Layouts 1.15
import OneOS.Shell 1.0

Item {
    id: root
    width: Screen.width
    height: Screen.height

    // Frosted glass background
    Rectangle {
        anchors.fill: parent
        color: "#E8111111"

        layer.enabled: true
        layer.effect: FastBlur { radius: 32 }
    }

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 0
        spacing: 0

        // Search bar
        Rectangle {
            Layout.fillWidth: true
            height: 52
            color: "#30FFFFFF"
            radius: 12
            Layout.margins: 16
            Layout.topMargin: 24

            RowLayout {
                anchors { fill: parent; leftMargin: 12; rightMargin: 12 }
                spacing: 8
                Text { text: "⌕"; font.pixelSize: 18; color: "#80FFFFFF" }
                TextInput {
                    id: searchBox
                    Layout.fillWidth: true
                    placeholderText: "Search apps…"
                    color: "white"
                    font.pixelSize: 15
                    onTextChanged: DrawerController.setFilter(text)
                }
                Text {
                    visible: searchBox.text.length > 0
                    text: "✕"; font.pixelSize: 14; color: "#80FFFFFF"
                    MouseArea { anchors.fill: parent; onClicked: searchBox.clear() }
                }
            }
        }

        // App grid
        GridView {
            id: appGrid
            Layout.fillWidth: true
            Layout.fillHeight: true
            contentY: 0
            cellWidth: root.width / 4
            cellHeight: 90
            model: DrawerController.filteredApps
            clip: true

            ScrollBar.vertical: ScrollBar { policy: ScrollBar.AsNeeded }

            delegate: AppIcon {
                width: appGrid.cellWidth
                height: appGrid.cellHeight
                appInfo: modelData
                iconSize: 48
                showLabel: true
                onTapped: {
                    ShellController.launchApp(modelData.packageName)
                    ShellController.closeDrawer()
                }
                onLongPressed: ShellController.showAppOptions(modelData)
            }
        }
    }

    // Drag-to-close handle
    Rectangle {
        width: 36; height: 4; radius: 2
        color: "#60FFFFFF"
        anchors { top: parent.top; topMargin: 8; horizontalCenter: parent.horizontalCenter }
    }
}
