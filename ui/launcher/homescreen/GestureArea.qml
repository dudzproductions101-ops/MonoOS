// GestureArea.qml – Invisible overlay that captures directional swipes
import QtQuick 2.15

Item {
    id: root
    signal swipeUp()
    signal swipeDown()
    signal swipeLeft()
    signal swipeRight()

    property real swipeThreshold: 60
    property real startX: 0
    property real startY: 0

    MultiPointTouchArea {
        anchors.fill: parent
        touchPoints: [ TouchPoint { id: tp0 } ]

        onPressed: { root.startX = tp0.x; root.startY = tp0.y }

        onReleased: {
            var dx = tp0.x - root.startX
            var dy = tp0.y - root.startY
            if (Math.abs(dx) > Math.abs(dy)) {
                if (dx < -root.swipeThreshold) root.swipeLeft()
                else if (dx > root.swipeThreshold) root.swipeRight()
            } else {
                if (dy < -root.swipeThreshold) root.swipeUp()
                else if (dy > root.swipeThreshold) root.swipeDown()
            }
        }
    }
}
