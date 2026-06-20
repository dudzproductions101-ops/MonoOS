// FileRow.qml – A single file/folder entry in the Files app
import QtQuick 2.15
import QtQuick.Layouts 1.15
import OneOS.Shell 1.0

Rectangle {
    id: root
    property var entry: null
    signal tapped()
    signal longPressed()
    height: 60
    color: pressed ? ThemeEngine.surfaceRaised : ThemeEngine.background
    property bool pressed: false

    RowLayout {
        anchors { fill: parent; leftMargin: 16; rightMargin: 16 }
        spacing: 14

        Text {
            text: root.entry ? (root.entry.isDir ? "📁" : fileIcon(root.entry.mimeType)) : "📄"
            font.pixelSize: 28
        }

        Column {
            Layout.fillWidth: true
            Text { text: root.entry ? root.entry.name : ""; font.pixelSize: 15; color: ThemeEngine.labelPrimary; elide: Text.ElideRight }
            Text {
                text: root.entry ? (root.entry.isDir ? (root.entry.childCount + " items") : formatSize(root.entry.size)) : ""
                font.pixelSize: 12; color: ThemeEngine.labelSecondary
            }
        }

        Text { text: root.entry ? root.entry.modifiedAgo : ""; font.pixelSize: 12; color: ThemeEngine.labelTertiary }
    }

    Rectangle { anchors { bottom: parent.bottom; left: parent.left; right: parent.right; leftMargin: 60 }; height: 0.5; color: ThemeEngine.border }

    function fileIcon(mime) {
        if (!mime) return "📄"
        if (mime.startsWith("image")) return "🖼"
        if (mime.startsWith("video")) return "🎬"
        if (mime.startsWith("audio")) return "🎵"
        if (mime.includes("pdf"))    return "📕"
        if (mime.includes("zip") || mime.includes("tar")) return "🗜"
        return "📄"
    }

    function formatSize(bytes) {
        if (bytes < 1024) return bytes + " B"
        if (bytes < 1048576) return (bytes / 1024).toFixed(1) + " KB"
        if (bytes < 1073741824) return (bytes / 1048576).toFixed(1) + " MB"
        return (bytes / 1073741824).toFixed(2) + " GB"
    }

    MouseArea {
        anchors.fill: parent
        onPressed: root.pressed = true; onReleased: root.pressed = false
        onClicked: root.tapped()
        onPressAndHold: root.longPressed()
    }
}
