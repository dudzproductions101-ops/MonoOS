// SettingsGroup.qml – Labelled section of settings rows
import QtQuick 2.15
import QtQuick.Layouts 1.15
import OneOS.Shell 1.0

Column {
    id: root
    property var groupData: null
    width: parent ? parent.width : 0

    Text {
        text: root.groupData ? root.groupData.label.toUpperCase() : ""
        font: ThemeEngine.caption1
        color: ThemeEngine.labelSecondary
        leftPadding: 32; topPadding: 8; bottomPadding: 4
    }

    Rectangle {
        width: parent.width - 32; x: 16
        height: childrenRect.height; radius: ThemeEngine.radiusLg
        color: ThemeEngine.surface
        clip: true

        Column {
            width: parent.width
            Repeater {
                model: root.groupData ? root.groupData.items : []
                delegate: SettingsRow {
                    width: parent.width
                    rowData: modelData
                    showDivider: index < (root.groupData.items.length - 1)
                }
            }
        }
    }
}
