// PhotoViewer.qml – Full-screen photo/video viewer
import QtQuick 2.15
import QtQuick.Controls 2.15
import OneOS.Apps 1.0

Item {
    id: root; property var photo: null

    // Background
    Rectangle { anchors.fill: parent; color: "black" }

    // Photo with pinch-zoom
    PinchArea {
        id: pinch; anchors.fill: parent
        property real minScale: 1.0; property real maxScale: 4.0
        pinch.minimumScale: minScale; pinch.maximumScale: maxScale

        Image {
            id: img; anchors.fill: parent
            source: root.photo ? root.photo.fullUri : ""
            fillMode: Image.PreserveAspectFit; asynchronous: true
            scale: pinch.pinch.scale
        }
    }

    // Controls overlay (fades after 3 s)
    Rectangle {
        id: controls; anchors { top: parent.top; left: parent.left; right: parent.right }
        height: 56; color: "#80000000"
        opacity: uiVisible ? 1 : 0
        Behavior on opacity { NumberAnimation { duration: 250 } }

        Row {
            anchors { fill: parent; leftMargin: 8; rightMargin: 8 }
            spacing: 0
            ToolButton { text: "‹"; font.pixelSize: 26; palette.buttonText: "white"; onClicked: stack.pop() }
            Item { width: 1; Layout.fillWidth: true }
            ToolButton { text: "⋮"; font.pixelSize: 26; palette.buttonText: "white"; onClicked: GalleryController.showPhotoMenu(root.photo) }
        }
    }

    property bool uiVisible: true
    Timer { id: fadeTimer; interval: 3000; running: uiVisible; onTriggered: uiVisible = false }
    MouseArea { anchors.fill: parent; onClicked: { uiVisible = !uiVisible; if (uiVisible) fadeTimer.restart() } }
}
