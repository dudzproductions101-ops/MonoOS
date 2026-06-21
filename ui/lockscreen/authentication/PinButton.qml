// PinButton.qml – Single PIN keypad button
import QtQuick 2.15

Rectangle {
    id: root
    property string label: ""
    signal tapped()
    width: 72; height: 72; radius: 36
    color: pressed ? "#60FFFFFF" : "#30FFFFFF"
    property bool pressed: false
    visible: label !== ""
    Behavior on color { ColorAnimation { duration: 60 } }

    Text {
        anchors.centerIn: parent
        text: root.label
        color: "white"
        font { pixelSize: label === "⌫" ? 22 : 28; weight: Font.Light }
    }

    TapHandler {
        onPressedChanged: root.pressed = pressed
        onTapped: root.tapped()
    }
}
