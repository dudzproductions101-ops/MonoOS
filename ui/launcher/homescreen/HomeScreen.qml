// HomeScreen.qml – OneOS Launcher Home Screen
import QtQuick 2.15
import QtQuick.Layouts 1.15
import QtQuick.Controls 2.15
import OneOS.Shell 1.0

Item {
    id: root
    objectName: "homeScreen"
    width: Screen.width
    height: Screen.height

    // ── background wallpaper ─────────────────────────────────────────────────
    Image {
        id: wallpaper
        anchors.fill: parent
        source: WallpaperManager.currentWallpaper
        fillMode: Image.PreserveAspectCrop
        asynchronous: true
    }

    // ── page view (multiple home screens) ───────────────────────────────────
    SwipeView {
        id: pageView
        anchors.fill: parent
        currentIndex: ShellController.homePage

        Repeater {
            model: ShellController.pageCount
            delegate: HomePageDelegate { pageIndex: index }
        }
    }

    // ── page indicator dots ──────────────────────────────────────────────────
    PageIndicator {
        id: pageIndicator
        count: pageView.count
        currentIndex: pageView.currentIndex
        anchors {
            bottom: dockArea.top
            horizontalCenter: parent.horizontalCenter
            bottomMargin: 8
        }
        delegate: Rectangle {
            width: 6; height: 6; radius: 3
            color: index === pageIndicator.currentIndex
                   ? "#FFFFFF" : "#80FFFFFF"
            Behavior on color { ColorAnimation { duration: 150 } }
        }
    }

    // ── dock ─────────────────────────────────────────────────────────────────
    Rectangle {
        id: dockArea
        anchors { bottom: parent.bottom; left: parent.left; right: parent.right }
        height: 88 + safeAreaBottom
        color: "#20000000"

        Row {
            id: dockRow
            anchors {
                horizontalCenter: parent.horizontalCenter
                verticalCenter: parent.verticalCenter
                verticalCenterOffset: -safeAreaBottom / 2
            }
            spacing: 16

            Repeater {
                model: ShellController.dockApps
                delegate: AppIcon {
                    appInfo: modelData
                    iconSize: 56
                    onTapped: ShellController.launchApp(modelData.packageName)
                    onLongPressed: ShellController.enterEditMode()
                }
            }
        }
    }

    // ── gesture overlay (swipe up = recents, swipe down = notifications) ────
    GestureArea {
        anchors.fill: parent
        onSwipeUp: ShellController.showRecents()
        onSwipeDown: ShellController.showNotificationShade()
        onSwipeLeft: pageView.currentIndex = Math.min(pageView.currentIndex + 1, pageView.count - 1)
        onSwipeRight: pageView.currentIndex = Math.max(pageView.currentIndex - 1, 0)
    }

    property real safeAreaBottom: Qt.platform.os === "android" ? 34 : 0
}
