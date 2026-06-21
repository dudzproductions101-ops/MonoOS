// SettingsRow.qml – Single settings row (label + value + accessory)
import QtQuick 2.15
import QtQuick.Layouts 1.15
import MonoOS.Shell 1.0

Rectangle {
    id: root
    property var rowData: null
    property bool showDivider: false
    height: 52
    color: pressed ? ThemeEngine.surfaceRaised : ThemeEngine.surface
    property bool pressed: false
    Behavior on color { ColorAnimation { duration: 80 } }

    RowLayout {
        anchors { fill: parent; leftMargin: 16; rightMargin: 16 }
        spacing: 12

        // Leading icon
        Rectangle {
            visible: root.rowData && root.rowData.iconColor !== undefined
            width: 30; height: 30; radius: 8
            color: root.rowData ? root.rowData.iconColor : "transparent"
            Text {
                anchors.centerIn: parent
                text: root.rowData ? root.rowData.icon : ""
                font.pixelSize: 16; color: "white"
            }
        }

        // Label
        Column {
            Layout.fillWidth: true
            Text {
                text: root.rowData ? root.rowData.title : ""
                font: ThemeEngine.body
                color: ThemeEngine.labelPrimary
            }
            Text {
                visible: root.rowData && root.rowData.subtitle !== undefined
                text: root.rowData ? (root.rowData.subtitle || "") : ""
                font: ThemeEngine.footnote
                color: ThemeEngine.labelSecondary
            }
        }

        // Value text
        Text {
            visible: root.rowData && root.rowData.value !== undefined
            text: root.rowData ? (root.rowData.value || "") : ""
            font: ThemeEngine.body; color: ThemeEngine.labelSecondary
        }

        // Toggle
        Switch {
            visible: root.rowData && root.rowData.type === "toggle"
            checked: root.rowData ? root.rowData.enabled : false
            onToggled: if (root.rowData) SettingsController.toggle(root.rowData.id)
        }

        // Chevron
        Text {
            visible: root.rowData && root.rowData.type !== "toggle"
            text: "›"; font.pixelSize: 22; color: ThemeEngine.labelTertiary
        }
    }

    // Divider
    Rectangle {
        visible: root.showDivider
        anchors { bottom: parent.bottom; left: parent.left; right: parent.right; leftMargin: 16 }
        height: 0.5; color: ThemeEngine.border
    }

    MouseArea {
        anchors.fill: parent
        onPressed: root.pressed = true
        onReleased: root.pressed = false
        onClicked: if (root.rowData) SettingsController.navigate(root.rowData.route)
    }
}
