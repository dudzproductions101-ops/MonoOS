// main.qml – MonoOS Terminal Emulator
import QtQuick 2.15
import QtQuick.Controls 2.15
import MonoOS.Apps 1.0

ApplicationWindow {
    visible: true; width: Screen.width; height: Screen.height
    title: "Terminal"; color: "#0D0D0D"

    ColumnLayout {
        anchors.fill: parent; spacing: 0

        // Tab bar
        Rectangle {
            Layout.fillWidth: true; height: 40
            color: "#1A1A1A"

            Row {
                id: tabRow; height: parent.height

                Repeater {
                    model: TerminalController.tabs
                    delegate: Rectangle {
                        width: 120; height: parent.height
                        color: modelData.active ? "#0D0D0D" : "transparent"

                        Text {
                            anchors.centerIn: parent
                            text: "sh " + (index + 1)
                            color: modelData.active ? "#30D158" : "#666"
                            font.pixelSize: 13
                        }

                        MouseArea { anchors.fill: parent; onClicked: TerminalController.switchTab(index) }
                    }
                }

                Rectangle {
                    width: 40; height: parent.height; color: "transparent"
                    Text { anchors.centerIn: parent; text: "+"; color: "#666"; font.pixelSize: 20 }
                    MouseArea { anchors.fill: parent; onClicked: TerminalController.newTab() }
                }
            }
        }

        // Output area
        Flickable {
            id: flick
            Layout.fillWidth: true; Layout.fillHeight: true
            contentHeight: outputText.contentHeight + 12
            clip: true

            Text {
                id: outputText
                x: 8; y: 6
                width: parent.width - 16
                text: TerminalController.output
                color: "#E0E0E0"
                font { family: "Courier New, monospace"; pixelSize: 13 }
                wrapMode: Text.WrapAnywhere
                textFormat: Text.RichText
            }

            onContentHeightChanged: contentY = Math.max(0, contentHeight - height)
        }

        // Input row
        Rectangle {
            Layout.fillWidth: true; height: 44
            color: "#1A1A1A"

            Row {
                anchors { fill: parent; leftMargin: 8; rightMargin: 8 }
                spacing: 6

                Text {
                    text: TerminalController.prompt
                    color: "#30D158"; font { family: "Courier New"; pixelSize: 13 }
                    anchors.verticalCenter: parent.verticalCenter
                }

                TextInput {
                    id: cmdInput
                    Layout.fillWidth: true; width: parent.width - 80
                    anchors.verticalCenter: parent.verticalCenter
                    color: "#E0E0E0"
                    font { family: "Courier New"; pixelSize: 13 }
                    focus: true

                    Keys.onReturnPressed: {
                        TerminalController.execute(text)
                        text = ""
                    }
                    Keys.onUpPressed: text = TerminalController.historyBack()
                    Keys.onDownPressed: text = TerminalController.historyForward()
                    Keys.onTabPressed: text = TerminalController.tabComplete(text)
                }
            }
        }
    }
}
