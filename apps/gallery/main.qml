// main.qml – MonoOS Gallery App
import QtQuick 2.15
import QtQuick.Controls 2.15
import QtQuick.Layouts 1.15
import MonoOS.Apps 1.0

ApplicationWindow {
    id: root; visible: true; width: Screen.width; height: Screen.height
    title: "Gallery"; color: "black"

    StackView {
        id: stack; anchors.fill: parent
        initialItem: albumsPage
    }

    Component {
        id: albumsPage
        Rectangle {
            color: "black"
            GridView {
                anchors.fill: parent
                cellWidth: root.width / 2; cellHeight: root.width / 2
                model: GalleryController.albums
                delegate: AlbumCard {
                    width: root.width / 2; height: root.width / 2
                    album: modelData
                    onTapped: stack.push(photosPage, { "albumId": modelData.id })
                }
            }
        }
    }

    Component {
        id: photosPage
        Rectangle {
            property string albumId: ""
            color: "black"
            GridView {
                anchors.fill: parent
                cellWidth: root.width / 3; cellHeight: root.width / 3
                model: GalleryController.photosForAlbum(albumId)
                delegate: PhotoThumbnail {
                    width: root.width / 3; height: root.width / 3
                    photo: modelData
                    onTapped: stack.push(viewerPage, { "photo": modelData })
                }
            }
        }
    }

    Component {
        id: viewerPage
        PhotoViewer { property var photo: null }
    }
}
