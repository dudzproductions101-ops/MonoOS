// main.qml – OneOS Camera App
import QtQuick 2.15
import QtQuick.Controls 2.15
import QtQuick.Layouts 1.15
import QtMultimedia 5.15
import OneOS.Apps 1.0

ApplicationWindow {
    id: root; visible: true; width: Screen.width; height: Screen.height
    title: "Camera"; color: "black"

    // Camera viewfinder
    Camera {
        id: camera
        captureMode: CameraController.videoMode ? Camera.CaptureVideo : Camera.CaptureStillImage
        deviceId: CameraController.cameraId
        flash.mode: CameraController.flashMode
        focus.focusMode: Camera.FocusContinuous
    }

    VideoOutput {
        id: viewfinder; source: camera
        anchors.fill: parent; fillMode: VideoOutput.PreserveAspectCrop

        // Tap to focus
        TapHandler {
            onTapped: {
                camera.focus.customFocusPoint = Qt.point(
                    eventPoint.position.x / viewfinder.width,
                    eventPoint.position.y / viewfinder.height)
                camera.focus.focusMode = Camera.FocusAuto
                focusCircle.x = eventPoint.position.x - 32
                focusCircle.y = eventPoint.position.y - 32
                focusCircle.opacity = 1
                focusFade.restart()
            }
        }
    }

    // Focus ring
    Rectangle {
        id: focusCircle; width: 64; height: 64; radius: 32
        color: "transparent"; border.color: "white"; border.width: 2; opacity: 0
        NumberAnimation on opacity { id: focusFade; to: 0; duration: 1200; easing.type: Easing.InQuad }
    }

    // Bottom controls
    Rectangle {
        anchors { bottom: parent.bottom; left: parent.left; right: parent.right }
        height: 120 + safeAreaBottom; color: "#80000000"
        property real safeAreaBottom: 24

        Row {
            anchors.centerIn: parent; spacing: 48

            // Gallery thumbnail
            Rectangle {
                width: 56; height: 56; radius: 10; color: "#333"
                Image { anchors.fill: parent; source: CameraController.lastPhotoThumb; fillMode: Image.PreserveAspectCrop; asynchronous: true }
                MouseArea { anchors.fill: parent; onClicked: CameraController.openGallery() }
            }

            // Shutter
            Rectangle {
                width: 72; height: 72; radius: 36
                color: "white"; border.color: "#DDD"; border.width: 3

                Rectangle {
                    visible: CameraController.videoMode; anchors.centerIn: parent
                    width: 28; height: 28; radius: CameraController.recording ? 6 : 14
                    color: CameraController.recording ? "#FF3B30" : "#FF3B30"
                    Behavior on radius { NumberAnimation { duration: 150 } }
                }

                MouseArea {
                    anchors.fill: parent
                    onClicked: CameraController.videoMode ? CameraController.toggleRecord() : CameraController.capture()
                }
            }

            // Switch camera
            Rectangle {
                width: 56; height: 56; radius: 28; color: "#30FFFFFF"
                Text { anchors.centerIn: parent; text: "🔄"; font.pixelSize: 24 }
                MouseArea { anchors.fill: parent; onClicked: CameraController.flipCamera() }
            }
        }

        // Mode switcher
        Row {
            anchors { top: parent.top; topMargin: 8; horizontalCenter: parent.horizontalCenter }
            spacing: 24
            Repeater {
                model: ["Photo","Video","Portrait","Pro"]
                Text {
                    text: modelData
                    color: CameraController.modeLabel === modelData ? "white" : "#80FFFFFF"
                    font { pixelSize: 13; weight: CameraController.modeLabel === modelData ? Font.SemiBold : Font.Normal }
                    MouseArea { anchors.fill: parent; onClicked: CameraController.setMode(modelData) }
                }
            }
        }
    }

    // Flash, zoom controls (top bar)
    Row {
        anchors { top: parent.top; topMargin: 48; horizontalCenter: parent.horizontalCenter }
        spacing: 24
        Text { text: CameraController.flashIcon; font.pixelSize: 22; color: "white"
               MouseArea { anchors.fill: parent; onClicked: CameraController.cycleFlash() } }
        Text { text: CameraController.zoomLabel; font.pixelSize: 14; color: "white" }
        Text { text: "⚙"; font.pixelSize: 22; color: "white"
               MouseArea { anchors.fill: parent; onClicked: CameraController.openSettings() } }
    }
}
