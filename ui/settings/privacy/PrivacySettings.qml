// PrivacySettings.qml – Privacy & permissions settings page
import QtQuick 2.15
import QtQuick.Controls 2.15
import QtQuick.Layouts 1.15
import OneOS.Shell 1.0

Page {
    background: Rectangle { color: ThemeEngine.background }

    header: ToolBar {
        background: Rectangle { color: ThemeEngine.surface }
        RowLayout {
            anchors.fill: parent; leftMargin: 8
            ToolButton { text: "‹"; onClicked: stackView.pop() }
            Label { text: "Privacy"; font: ThemeEngine.title3; color: ThemeEngine.labelPrimary }
        }
    }

    ListView {
        anchors.fill: parent
        model: PrivacyController.sections
        spacing: 20
        delegate: PrivacySection { sectionData: modelData; width: ListView.view.width }
    }
}
