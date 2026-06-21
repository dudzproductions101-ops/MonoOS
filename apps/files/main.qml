// main.qml – MonoOS Files App
import QtQuick 2.15
import QtQuick.Controls 2.15
import QtQuick.Layouts 1.15
import MonoOS.Apps 1.0

ApplicationWindow {
    visible: true; width: Screen.width; height: Screen.height
    title: "Files"; color: ThemeEngine.background

    ColumnLayout {
        anchors.fill: parent; spacing: 0

        // Breadcrumb bar
        Rectangle {
            Layout.fillWidth: true; height: 52; color: ThemeEngine.surface
            Row {
                anchors { verticalCenter: parent.verticalCenter; left: parent.left; leftMargin: 16 }
                spacing: 4
                Repeater {
                    model: FilesController.breadcrumbs
                    Row {
                        Text { text: index > 0 ? " › " : ""; color: ThemeEngine.labelTertiary; font.pixelSize: 14 }
                        Text {
                            text: modelData
                            color: index === FilesController.breadcrumbs.length - 1
                                   ? ThemeEngine.labelPrimary : ThemeEngine.accent
                            font.pixelSize: 14
                            MouseArea { anchors.fill: parent; onClicked: FilesController.navigateToCrumb(index) }
                        }
                    }
                }
            }
        }

        // File list
        ListView {
            Layout.fillWidth: true; Layout.fillHeight: true
            model: FilesController.entries
            clip: true

            delegate: FileRow {
                width: ListView.view.width
                entry: modelData
                onTapped: FilesController.open(modelData)
                onLongPressed: FilesController.showContextMenu(modelData)
            }
        }
    }

    // FAB – new folder
    RoundButton {
        anchors { right: parent.right; bottom: parent.bottom; margins: 24 }
        text: "+"; font.pixelSize: 28
        Material.background: ThemeEngine.accent
        onClicked: FilesController.createFolder()
    }
}
