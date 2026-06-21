// SettingsMainPage.qml – Settings home with grouped list
import QtQuick 2.15
import QtQuick.Controls 2.15
import QtQuick.Layouts 1.15
import MonoOS.Shell 1.0

Page {
    id: root
    background: Rectangle { color: ThemeEngine.background }

    header: ToolBar {
        background: Rectangle { color: ThemeEngine.surface }
        Label {
            text: "Settings"; font: ThemeEngine.title2
            color: ThemeEngine.labelPrimary
            anchors.centerIn: parent
        }
    }

    ListView {
        anchors.fill: parent
        model: SettingsController.groups
        clip: true
        spacing: 24

        delegate: SettingsGroup {
            width: ListView.view.width
            groupData: modelData
        }
    }
}
