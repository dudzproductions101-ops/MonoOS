// WidgetGrid.qml – Resizable widget host on the home screen
import QtQuick 2.15
import OneOS.Shell 1.0

Item {
    id: root
    anchors.fill: parent

    // 4-column, 6-row grid; each cell is (screen.width / 4) wide
    property real cellW: width / 4
    property real cellH: 96

    Repeater {
        model: WidgetManager.placedWidgets
        delegate: WidgetFrame {
            x: modelData.col * root.cellW
            y: modelData.row * root.cellH
            width: modelData.colSpan * root.cellW
            height: modelData.rowSpan * root.cellH
            widgetInfo: modelData
        }
    }
}
