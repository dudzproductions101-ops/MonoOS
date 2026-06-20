// CalcButton.qml – Calculator key
import QtQuick 2.15

Rectangle {
    id: root
    property string label: ""
    property string style: "num"
    signal tapped()

    color: {
        if (style === "op") return pressed ? "#E08C00" : "#FF9F0A"
        if (style === "fn") return pressed ? "#909090" : "#636366"
        if (style === "eq") return pressed ? "#C07000" : "#FF9F0A"
        return pressed ? "#909090" : "#333335"
    }

    property bool pressed: false
    Behavior on color { ColorAnimation { duration: 60 } }

    Text {
        anchors.centerIn: parent
        text: root.label
        color: root.style === "fn" ? "black" : "white"
        font { pixelSize: root.label.length > 2 ? 20 : 32; weight: Font.Light }
    }

    TapHandler {
        onPressedChanged: root.pressed = pressed
        onTapped: root.tapped()
    }
}
