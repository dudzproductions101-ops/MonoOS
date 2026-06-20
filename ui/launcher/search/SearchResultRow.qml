// SearchResultRow.qml – Single search result entry
import QtQuick 2.15
import QtQuick.Layouts 1.15
import OneOS.Shell 1.0

Rectangle {
    id: root
    property var result: null
    width: parent ? parent.width : 0
    height: 52; radius: 10
    color: hovered ? "#30FFFFFF" : "transparent"
    property bool hovered: false

    RowLayout {
        anchors { fill: parent; leftMargin: 12; rightMargin: 12 }
        spacing: 12

        Image {
            width: 32; height: 32
            source: root.result ? root.result.iconPath : ""
            fillMode: Image.PreserveAspectFit
        }

        Column {
            Layout.fillWidth: true
            Text { text: root.result ? root.result.title : ""; color: "white"; font.pixelSize: 14 }
            Text { text: root.result ? root.result.subtitle : ""; color: "#80FFFFFF"; font.pixelSize: 11 }
        }
    }

    MouseArea {
        anchors.fill: parent
        hoverEnabled: true
        onEntered: root.hovered = true
        onExited:  root.hovered = false
        onClicked: if (root.result) SearchController.activate(root.result)
    }
}
