// Main.qml – Basic App template UI
import QtQuick 2.15
import QtQuick.Controls 2.15
import QtQuick.Layouts 1.15

ApplicationWindow {
    id: root
    visible: true
    width: Screen.width
    height: Screen.height
    title: "Basic App"

    // ── Background ─────────────────────────────────────────────────────────
    Rectangle {
        anchors.fill: parent
        color: "#1C1C1E"
    }

    // ── Content ────────────────────────────────────────────────────────────
    ColumnLayout {
        anchors.centerIn: parent
        spacing: 24

        Text {
            Layout.alignment: Qt.AlignHCenter
            text: "Hello, MonoOS!"
            color: "white"
            font { pixelSize: 32; weight: Font.Light }
        }

        Text {
            Layout.alignment: Qt.AlignHCenter
            text: "Replace this template with your app."
            color: "#8E8E93"
            font.pixelSize: 16
        }

        Button {
            Layout.alignment: Qt.AlignHCenter
            text: "Tap me"
            onClicked: messageText.visible = !messageText.visible
        }

        Text {
            id: messageText
            Layout.alignment: Qt.AlignHCenter
            visible: false
            text: "🎉 It works!"
            color: "#30D158"
            font.pixelSize: 20
        }
    }
}
