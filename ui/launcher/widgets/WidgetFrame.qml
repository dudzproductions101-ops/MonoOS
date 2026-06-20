// WidgetFrame.qml – Container that loads and displays a single widget
import QtQuick 2.15
import QtQuick.Controls 2.15
import OneOS.Shell 1.0

Rectangle {
    id: root
    property var widgetInfo: null
    radius: 16
    color: "#20000000"
    clip: true

    Loader {
        anchors { fill: parent; margins: 4 }
        source: root.widgetInfo ? root.widgetInfo.qmlPath : ""
        onLoaded: item.widgetData = root.widgetInfo
    }

    // Long-press to enter widget-edit mode
    TapHandler {
        longPressThreshold: 600
        onLongPressed: WidgetManager.beginEdit(root.widgetInfo)
    }

    // Remove button (edit mode only)
    Rectangle {
        visible: WidgetManager.editMode
        anchors { top: parent.top; right: parent.right; margins: 6 }
        width: 22; height: 22; radius: 11
        color: "#FF3B30"
        Text { anchors.centerIn: parent; text: "✕"; color: "white"; font.pixelSize: 12 }
        MouseArea { anchors.fill: parent; onClicked: WidgetManager.removeWidget(root.widgetInfo) }
    }
}
