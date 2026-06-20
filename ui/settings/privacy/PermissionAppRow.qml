// PermissionAppRow.qml – One app row in the privacy permission list
import QtQuick 2.15
import QtQuick.Layouts 1.15
import OneOS.Shell 1.0

Rectangle {
    property var appData: null
    height: 52; color: ThemeEngine.surface

    RowLayout {
        anchors { fill: parent; leftMargin: 16; rightMargin: 16 }
        Image { width: 28; height: 28; source: appData ? appData.iconPath : ""; fillMode: Image.PreserveAspectFit }
        Text { Layout.fillWidth: true; text: appData ? appData.label : ""; font: ThemeEngine.body; color: ThemeEngine.labelPrimary }
        Text {
            text: appData ? appData.grantState : ""
            font: ThemeEngine.subheadline
            color: appData && appData.grantState === "Allowed" ? ThemeEngine.accentSecondary : ThemeEngine.labelTertiary
        }
        Text { text: "›"; font.pixelSize: 20; color: ThemeEngine.labelTertiary }
    }

    MouseArea { anchors.fill: parent; onClicked: PrivacyController.openAppPermissions(appData) }
}
