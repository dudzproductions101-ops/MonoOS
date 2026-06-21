// AboutRow.qml – Label / value row used in About settings
import QtQuick 2.15
import MonoOS.Shell 1.0

Rectangle {
    property string label: ""
    property string value: ""
    property bool divider: true
    height: 52; color: ThemeEngine.surface

    Row {
        anchors { fill: parent; leftMargin: 16; rightMargin: 16 }
        spacing: 8
        Text { text: label; font: ThemeEngine.body; color: ThemeEngine.labelPrimary; anchors.verticalCenter: parent.verticalCenter; width: 160 }
        Text { text: value; font: ThemeEngine.body; color: ThemeEngine.labelSecondary; anchors.verticalCenter: parent.verticalCenter; elide: Text.ElideRight }
    }

    Rectangle {
        visible: divider; anchors { bottom: parent.bottom; left: parent.left; right: parent.right; leftMargin: 16 }
        height: 0.5; color: ThemeEngine.border
    }
}
