// GestureNavigator.qml – System-wide gesture navigation bar
import QtQuick 2.15
import OneOS.Shell 1.0

Item {
    id: root
    width: Screen.width
    height: 32 + safeAreaBottom
    property real safeAreaBottom: 8

    // Central home indicator pill
    Rectangle {
        id: homePill
        width: 134; height: 5; radius: 2.5
        color: "white"; opacity: 0.7
        anchors { horizontalCenter: parent.horizontalCenter; bottom: parent.bottom; bottomMargin: safeAreaBottom + 6 }

        DragHandler {
            id: pillDrag
            yAxis.minimum: -200; yAxis.maximum: 0
            onActiveChanged: {
                if (!active) {
                    if (pillDrag.translation.y < -60) ShellController.showRecents()
                    else ShellController.goHome()
                }
            }
        }

        TapHandler { onTapped: ShellController.goHome() }
    }

    // Left swipe → back
    Item {
        anchors { left: parent.left; top: parent.top; bottom: parent.bottom }
        width: 20

        DragHandler {
            xAxis.minimum: 0; xAxis.maximum: 160
            onActiveChanged: if (!active && translation.x > 60) ShellController.goBack()
        }
    }

    // Right swipe → back
    Item {
        anchors { right: parent.right; top: parent.top; bottom: parent.bottom }
        width: 20

        DragHandler {
            xAxis.minimum: -160; xAxis.maximum: 0
            onActiveChanged: if (!active && translation.x < -60) ShellController.goBack()
        }
    }
}
