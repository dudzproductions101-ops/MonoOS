// SearchSection.qml – A labelled group of search results
import QtQuick 2.15
import QtQuick.Layouts 1.15

Column {
    id: root
    property var section: null
    width: parent ? parent.width : 0
    spacing: 4

    Text {
        text: root.section ? root.section.title : ""
        color: "#80FFFFFF"; font.pixelSize: 12; font.weight: Font.Medium
        leftPadding: 4
    }

    Repeater {
        model: root.section ? root.section.results : []
        delegate: SearchResultRow { result: modelData }
    }
}
