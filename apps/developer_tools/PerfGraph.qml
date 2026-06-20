// PerfGraph.qml – Horizontal percentage bar for Dev Tools
import QtQuick 2.15
import OneOS.Shell 1.0

Item {
    property string title: ""
    property real value: 0       // 0.0–100.0
    property color color: ThemeEngine.accent
    height: 56; width: parent ? parent.width - 32 : 300

    Column {
        anchors.fill: parent; spacing: 6
        Row {
            Text { text: title; font: ThemeEngine.subheadline; color: ThemeEngine.labelPrimary; Layout.fillWidth: true }
            Text { text: value.toFixed(1) + "%"; font: ThemeEngine.subheadline; color: ThemeEngine.labelSecondary }
        }
        Rectangle {
            width: parent.width; height: 10; radius: 5; color: ThemeEngine.surfaceRaised
            Rectangle {
                width: parent.width * value / 100; height: parent.height; radius: 5; color: root.color
                Behavior on width { NumberAnimation { duration: 300 } }
            }
        }
    }
}
