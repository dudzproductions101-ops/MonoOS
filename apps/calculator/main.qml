// main.qml – OneOS Calculator App
import QtQuick 2.15
import QtQuick.Controls 2.15
import QtQuick.Layouts 1.15
import OneOS.Shell 1.0

ApplicationWindow {
    visible: true; width: Screen.width; height: Screen.height
    title: "Calculator"; color: "#1C1C1E"

    Column {
        anchors.fill: parent

        // Display
        Rectangle {
            width: parent.width; height: parent.height * 0.38
            color: "#1C1C1E"

            Column {
                anchors { right: parent.right; rightMargin: 24; bottom: parent.bottom; bottomMargin: 16 }

                Text {
                    id: expressionText
                    text: CalcEngine.expression
                    color: "#8E8E93"; font.pixelSize: 24
                    anchors.right: parent.right
                    elide: Text.ElideLeft; width: parent ? parent.width - 48 : 300
                }

                Text {
                    id: resultText
                    text: CalcEngine.display
                    color: "white"
                    font { pixelSize: CalcEngine.display.length > 9 ? 52 : 72; weight: Font.Thin }
                    anchors.right: parent.right
                }
            }
        }

        // Button grid
        GridLayout {
            width: parent.width; height: parent.height * 0.62
            columns: 4; rowSpacing: 1; columnSpacing: 1

            Repeater {
                model: [
                    {l:"AC",  s:"fn",  v:"clear"},  {l:"±",  s:"fn",  v:"negate"}, {l:"%",  s:"fn", v:"percent"}, {l:"÷", s:"op", v:"/"},
                    {l:"7",   s:"num", v:"7"},       {l:"8",  s:"num", v:"8"},      {l:"9",  s:"num", v:"9"},      {l:"×", s:"op", v:"*"},
                    {l:"4",   s:"num", v:"4"},       {l:"5",  s:"num", v:"5"},      {l:"6",  s:"num", v:"6"},      {l:"−", s:"op", v:"-"},
                    {l:"1",   s:"num", v:"1"},       {l:"2",  s:"num", v:"2"},      {l:"3",  s:"num", v:"3"},      {l:"+", s:"op", v:"+"},
                    {l:"0",   s:"num", v:"0", wide:true},                           {l:".",  s:"num", v:"."},      {l:"=", s:"eq", v:"="},
                ]

                delegate: CalcButton {
                    Layout.fillWidth: true; Layout.fillHeight: true
                    Layout.columnSpan: modelData.wide ? 2 : 1
                    label: modelData.l
                    style: modelData.s
                    onTapped: CalcEngine.input(modelData.v)
                }
            }
        }
    }
}
