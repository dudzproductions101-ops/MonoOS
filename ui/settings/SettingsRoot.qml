// SettingsRoot.qml – Top-level settings navigation shell
import QtQuick 2.15
import QtQuick.Controls 2.15
import OneOS.Shell 1.0

ApplicationWindow {
    id: root
    visible: true
    width: Screen.width; height: Screen.height
    title: "Settings"
    color: ThemeEngine.background

    StackView {
        id: stack
        anchors.fill: parent
        initialItem: SettingsMainPage {}
    }
}
