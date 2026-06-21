// AlbumCard.qml – Album grid tile
import QtQuick 2.15
import MonoOS.Apps 1.0

Item {
    id: root; property var album: null; signal tapped()

    Image {
        anchors.fill: parent
        source: root.album ? root.album.coverUri : ""
        fillMode: Image.PreserveAspectCrop; asynchronous: true
    }

    Rectangle {
        anchors { left: parent.left; right: parent.right; bottom: parent.bottom }
        height: 48; color: "#80000000"

        Column {
            anchors { left: parent.left; bottom: parent.bottom; margins: 8 }
            Text { text: root.album ? root.album.name : ""; color: "white"; font.pixelSize: 14; font.weight: Font.Medium }
            Text { text: root.album ? root.album.count + " photos" : ""; color: "#CCFFFFFF"; font.pixelSize: 12 }
        }
    }

    MouseArea { anchors.fill: parent; onClicked: root.tapped() }
}
