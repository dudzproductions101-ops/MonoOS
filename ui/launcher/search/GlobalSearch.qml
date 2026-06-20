// GlobalSearch.qml – System-wide search overlay
import QtQuick 2.15
import QtQuick.Controls 2.15
import QtQuick.Layouts 1.15
import OneOS.Shell 1.0

Item {
    id: root
    width: Screen.width
    height: Screen.height

    Rectangle { anchors.fill: parent; color: "#CC000000" }

    ColumnLayout {
        anchors { top: parent.top; topMargin: 60; left: parent.left; right: parent.right }
        anchors.margins: 16
        spacing: 12

        // Search input
        Rectangle {
            Layout.fillWidth: true
            height: 48; radius: 24
            color: "#FFFFFF"

            RowLayout {
                anchors { fill: parent; leftMargin: 16; rightMargin: 16 }
                Text { text: "⌕"; font.pixelSize: 18; color: "#666" }
                TextInput {
                    id: input
                    Layout.fillWidth: true
                    focus: true
                    font.pixelSize: 16
                    color: "#111"
                    onTextChanged: SearchController.query(text)
                }
            }
        }

        // Result sections
        Repeater {
            model: SearchController.sections
            delegate: SearchSection { section: modelData }
        }
    }

    Keys.onEscapePressed: ShellController.closeSearch()
}
