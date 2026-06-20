// HomePageDelegate.qml – A single page of app icons on the home screen
import QtQuick 2.15
import OneOS.Shell 1.0

Item {
    id: root
    property int pageIndex: 0
    width: parent ? parent.width : Screen.width
    height: parent ? parent.height : Screen.height

    GridView {
        id: grid
        anchors {
            top: parent.top; topMargin: 24
            left: parent.left; leftMargin: 12
            right: parent.right; rightMargin: 12
            bottom: parent.bottom; bottomMargin: 100
        }
        cellWidth: (width - spacing) / ShellController.gridColumns
        cellHeight: cellWidth + 24
        model: ShellController.appsForPage(pageIndex)
        interactive: false
        clip: true

        delegate: AppIcon {
            width: grid.cellWidth
            height: grid.cellHeight
            appInfo: modelData
            iconSize: 52
            showLabel: true
            onTapped: ShellController.launchApp(modelData.packageName)
            onLongPressed: ShellController.beginDrag(modelData, index, pageIndex)
        }
    }
}
