// LockClock.qml – Large ambient time display on the lock screen
import QtQuick 2.15

Column {
    spacing: 4

    Text {
        id: timeText
        anchors.horizontalCenter: parent.horizontalCenter
        text: Qt.formatTime(new Date(), "HH:mm")
        color: "white"
        font { pixelSize: 72; weight: Font.Light }
        style: Text.Raised; styleColor: "#40000000"
    }

    Text {
        anchors.horizontalCenter: parent.horizontalCenter
        text: Qt.formatDate(new Date(), "dddd, MMMM d")
        color: "#CCFFFFFF"
        font.pixelSize: 17
    }

    Timer {
        interval: 10000; running: true; repeat: true
        onTriggered: timeText.text = Qt.formatTime(new Date(), "HH:mm")
    }
}
