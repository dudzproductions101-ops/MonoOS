// PlayerUI.qml – Media player template
import QtQuick 2.15
import QtQuick.Controls 2.15
import QtQuick.Layouts 1.15
import OneOS.SDK 1.0

ApplicationWindow {
    visible: true; width: Screen.width; height: Screen.height
    title: "Player"; color: "#111"

    ColumnLayout {
        anchors.fill: parent; spacing: 0

        // Artwork
        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: parent.height * 0.5
            color: "#1C1C1E"
            Text { anchors.centerIn: parent; text: "🎵"; font.pixelSize: 96 }
        }

        // Metadata
        Column {
            Layout.fillWidth: true; Layout.margins: 24; spacing: 6
            Text { text: PlayerController.title;  color: "white"; font { pixelSize: 20; weight: Font.SemiBold } width: parent.width; elide: Text.ElideRight }
            Text { text: PlayerController.artist; color: "#8E8E93"; font.pixelSize: 16; width: parent.width; elide: Text.ElideRight }
        }

        // Progress
        Slider {
            Layout.fillWidth: true; Layout.leftMargin: 24; Layout.rightMargin: 24
            from: 0; to: PlayerController.duration
            value: PlayerController.position
            onMoved: PlayerController.seekTo(value)
        }

        // Controls
        Row {
            Layout.alignment: Qt.AlignHCenter; spacing: 32; Layout.bottomMargin: 40
            ToolButton { text: "⏮"; font.pixelSize: 28; palette.buttonText: "white"; onClicked: PlayerController.previous() }
            ToolButton { text: PlayerController.playing ? "⏸" : "▶"; font.pixelSize: 40; palette.buttonText: "white"; onClicked: PlayerController.togglePlay() }
            ToolButton { text: "⏭"; font.pixelSize: 28; palette.buttonText: "white"; onClicked: PlayerController.next() }
        }
    }
}
