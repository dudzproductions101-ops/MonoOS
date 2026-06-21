// PrivacySection.qml – Camera / Mic / Location section in privacy settings
import QtQuick 2.15
import QtQuick.Layouts 1.15
import MonoOS.Shell 1.0

Column {
    property var sectionData: null
    width: parent ? parent.width : 0
    spacing: 0

    Text {
        text: sectionData ? sectionData.resource.toUpperCase() : ""
        font: ThemeEngine.caption1; color: ThemeEngine.labelSecondary
        leftPadding: 32; bottomPadding: 4
    }

    // Active indicator
    Rectangle {
        width: parent.width - 32; x: 16; height: 44; radius: 12
        color: sectionData && sectionData.active ? "#33FF3B30" : ThemeEngine.surface

        RowLayout {
            anchors { fill: parent; leftMargin: 16; rightMargin: 16 }
            Text { text: sectionData ? sectionData.icon : ""; font.pixelSize: 22 }
            Text {
                Layout.fillWidth: true
                text: sectionData && sectionData.active
                      ? (sectionData.activeApp + " is using " + sectionData.resource)
                      : "No app using " + (sectionData ? sectionData.resource : "")
                font: ThemeEngine.body
                color: sectionData && sectionData.active ? "#FF3B30" : ThemeEngine.labelPrimary
            }
        }
    }

    // Per-app list
    Rectangle {
        width: parent.width - 32; x: 16; height: childrenRect.height
        radius: 12; color: ThemeEngine.surface; clip: true; topPadding: 0

        Column {
            width: parent.width
            Repeater {
                model: sectionData ? sectionData.apps : []
                delegate: PermissionAppRow { appData: modelData; width: parent.width }
            }
        }
    }
}
