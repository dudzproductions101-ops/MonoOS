// AuthOverlay.qml – PIN / biometric authentication overlay
import QtQuick 2.15
import QtQuick.Layouts 1.15
import OneOS.Shell 1.0

Rectangle {
    id: root
    color: "#CC000000"

    ColumnLayout {
        anchors.centerIn: parent
        spacing: 24
        width: Math.min(parent.width - 64, 320)

        // Lock icon
        Text {
            text: "🔒"; font.pixelSize: 48
            Layout.alignment: Qt.AlignHCenter
        }

        // Prompt text
        Text {
            text: LockController.authPrompt
            color: "white"; font.pixelSize: 16
            Layout.alignment: Qt.AlignHCenter
            horizontalAlignment: Text.AlignHCenter
            wrapMode: Text.WordWrap
            Layout.fillWidth: true
        }

        // PIN dots
        Row {
            spacing: 16; Layout.alignment: Qt.AlignHCenter
            Repeater {
                model: LockController.pinLength
                delegate: Rectangle {
                    width: 14; height: 14; radius: 7
                    color: index < LockController.pinEntered ? "white" : "#40FFFFFF"
                    Behavior on color { ColorAnimation { duration: 80 } }
                }
            }
        }

        // Number pad
        GridLayout {
            columns: 3; columnSpacing: 12; rowSpacing: 12
            Layout.alignment: Qt.AlignHCenter

            Repeater {
                model: ["1","2","3","4","5","6","7","8","9","","0","⌫"]
                delegate: PinButton {
                    label: modelData
                    onTapped: {
                        if (modelData === "⌫") LockController.backspace()
                        else if (modelData !== "") LockController.enterDigit(modelData)
                    }
                }
            }
        }

        // Biometric button
        Text {
            visible: LockController.biometricAvailable
            text: "Use Fingerprint / Face"
            color: "#4DA6FF"; font.pixelSize: 14
            Layout.alignment: Qt.AlignHCenter
            MouseArea { anchors.fill: parent; onClicked: LockController.triggerBiometric() }
        }

        // Error message
        Text {
            visible: LockController.errorMessage !== ""
            text: LockController.errorMessage
            color: "#FF5555"; font.pixelSize: 13
            Layout.alignment: Qt.AlignHCenter
        }
    }
}
