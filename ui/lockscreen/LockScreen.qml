// LockScreen.qml – MonoOS Lock Screen root
import QtQuick 2.15
import QtQuick.Controls 2.15
import MonoOS.Shell 1.0

Item {
    id: root
    width: Screen.width
    height: Screen.height
    objectName: "lockScreen"

    Image {
        anchors.fill: parent
        source: WallpaperManager.currentWallpaper
        fillMode: Image.PreserveAspectCrop
    }

    // Ambient clock
    LockClock {
        id: clock
        anchors { horizontalCenter: parent.horizontalCenter; top: parent.top; topMargin: 120 }
        opacity: unlockArea.dragging ? 0.0 : 1.0
        Behavior on opacity { NumberAnimation { duration: 200 } }
    }

    // Notification peek
    LockNotifications {
        anchors { top: clock.bottom; topMargin: 32; left: parent.left; right: parent.right }
        visible: !LockController.authRequired
    }

    // Unlock area (swipe up or tap to reveal auth)
    UnlockArea {
        id: unlockArea
        anchors { bottom: parent.bottom; left: parent.left; right: parent.right }
        onUnlockTriggered: LockController.beginAuth()
    }

    // Authentication overlay
    Loader {
        id: authLoader
        anchors.fill: parent
        source: LockController.authRequired ? "authentication/AuthOverlay.qml" : ""
        opacity: LockController.authRequired ? 1 : 0
        Behavior on opacity { NumberAnimation { duration: 180 } }
    }
}
