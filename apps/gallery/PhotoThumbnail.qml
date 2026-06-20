// PhotoThumbnail.qml – Grid photo thumbnail
import QtQuick 2.15

Item {
    id: root; property var photo: null; signal tapped()

    Image {
        anchors.fill: parent; anchors.margins: 1
        source: root.photo ? root.photo.thumbUri : ""
        fillMode: Image.PreserveAspectCrop; asynchronous: true
        Rectangle { anchors.fill: parent; color: "#111"; visible: parent.status !== Image.Ready }
    }

    // Video badge
    Rectangle {
        visible: root.photo && root.photo.isVideo
        width: 26; height: 26; radius: 13; color: "#80000000"
        anchors { right: parent.right; bottom: parent.bottom; margins: 4 }
        Text { anchors.centerIn: parent; text: "▶"; color: "white"; font.pixelSize: 11 }
    }

    MouseArea { anchors.fill: parent; onClicked: root.tapped() }
}
