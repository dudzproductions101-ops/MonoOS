// AppearanceSettings.qml – Display, theme, and font settings
import QtQuick 2.15
import QtQuick.Controls 2.15
import QtQuick.Layouts 1.15
import OneOS.Shell 1.0

Page {
    background: Rectangle { color: ThemeEngine.background }
    header: ToolBar {
        background: Rectangle { color: ThemeEngine.surface }
        Label { text: "Appearance"; font: ThemeEngine.title3; color: ThemeEngine.labelPrimary; anchors.centerIn: parent }
    }

    ScrollView {
        anchors.fill: parent
        Column {
            width: parent ? parent.width : Screen.width
            padding: 16
            spacing: 20

            // Theme picker
            Rectangle {
                width: parent.width - 32; height: childrenRect.height + 24; radius: 14; color: ThemeEngine.surface

                Column {
                    anchors { left: parent.left; right: parent.right; top: parent.top; margins: 16 }
                    spacing: 12

                    Text { text: "Theme"; font: ThemeEngine.headline; color: ThemeEngine.labelPrimary }

                    Row {
                        spacing: 12

                        Repeater {
                            model: [
                                { id: "light",  label: "Light",  bg: "#F2F2F7", fg: "#000" },
                                { id: "dark",   label: "Dark",   bg: "#111111", fg: "#FFF" },
                                { id: "amoled", label: "AMOLED", bg: "#000000", fg: "#FFF" },
                            ]
                            delegate: Column {
                                spacing: 6
                                Rectangle {
                                    width: 80; height: 52; radius: 12
                                    color: modelData.bg
                                    border.color: ThemeEngine.currentTheme === modelData.id ? ThemeEngine.accent : "transparent"
                                    border.width: 2
                                    Text { anchors.centerIn: parent; text: "Aa"; color: modelData.fg; font.pixelSize: 18 }
                                    MouseArea { anchors.fill: parent; onClicked: ThemeEngine.setTheme(modelData.id) }
                                }
                                Text {
                                    text: modelData.label; font: ThemeEngine.caption1
                                    color: ThemeEngine.labelSecondary; anchors.horizontalCenter: parent.horizontalCenter
                                }
                            }
                        }
                    }
                }
            }

            // Text size slider
            Rectangle {
                width: parent.width - 32; height: 90; radius: 14; color: ThemeEngine.surface
                Column {
                    anchors { fill: parent; margins: 16 }
                    spacing: 8
                    Text { text: "Text Size"; font: ThemeEngine.headline; color: ThemeEngine.labelPrimary }
                    Slider {
                        width: parent.width
                        from: 0.8; to: 1.4; stepSize: 0.1
                        value: AppearanceController.textScale
                        onValueChanged: AppearanceController.setTextScale(value)
                    }
                }
            }

            // Wallpaper row
            SettingsRow { rowData: AppearanceController.wallpaperRow; width: parent.width - 32 }
        }
    }
}
