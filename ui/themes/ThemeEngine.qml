// ThemeEngine.qml – OneOS global theme definitions
pragma Singleton
import QtQuick 2.15

QtObject {
    id: root

    // ── Active theme ──────────────────────────────────────────────────────────
    property string currentTheme: "dark"   // "dark" | "light" | "amoled"

    // ── Semantic colour palette ───────────────────────────────────────────────
    property color accent:          "#4DA6FF"
    property color accentSecondary: "#30D158"
    property color destructive:     "#FF3B30"
    property color warning:         "#FF9F0A"

    property color background:   currentTheme === "light" ? "#F2F2F7" : currentTheme === "amoled" ? "#000000" : "#111111"
    property color surface:      currentTheme === "light" ? "#FFFFFF"  : currentTheme === "amoled" ? "#0A0A0A" : "#1C1C1E"
    property color surfaceRaised: currentTheme === "light" ? "#F0F0F5" : "#2C2C2E"
    property color border:       currentTheme === "light" ? "#D1D1D6" : "#38383A"

    property color labelPrimary:   currentTheme === "light" ? "#000000" : "#FFFFFF"
    property color labelSecondary: currentTheme === "light" ? "#3C3C43" : "#EBEBF5"
    property color labelTertiary:  currentTheme === "light" ? "#60606A" : "#60606A"
    property color labelDisabled:  currentTheme === "light" ? "#C7C7CC" : "#3A3A3C"

    // ── Typography ────────────────────────────────────────────────────────────
    property font largeTitle:  Qt.font({ pixelSize: 34, weight: Font.Bold })
    property font title1:      Qt.font({ pixelSize: 28, weight: Font.Bold })
    property font title2:      Qt.font({ pixelSize: 22, weight: Font.Bold })
    property font title3:      Qt.font({ pixelSize: 20, weight: Font.SemiBold })
    property font headline:    Qt.font({ pixelSize: 17, weight: Font.SemiBold })
    property font body:        Qt.font({ pixelSize: 17, weight: Font.Normal })
    property font callout:     Qt.font({ pixelSize: 16, weight: Font.Normal })
    property font subheadline: Qt.font({ pixelSize: 15, weight: Font.Normal })
    property font footnote:    Qt.font({ pixelSize: 13, weight: Font.Normal })
    property font caption1:    Qt.font({ pixelSize: 12, weight: Font.Normal })
    property font caption2:    Qt.font({ pixelSize: 11, weight: Font.Normal })

    // ── Spacing ───────────────────────────────────────────────────────────────
    property real xs: 4
    property real sm: 8
    property real md: 16
    property real lg: 24
    property real xl: 32
    property real xxl: 48

    // ── Corner radii ──────────────────────────────────────────────────────────
    property real radiusSm:  8
    property real radiusMd: 12
    property real radiusLg: 16
    property real radiusFull: 999

    // ── Shadows ───────────────────────────────────────────────────────────────
    property var shadow: { color: "#40000000", blur: 12, x: 0, y: 4 }
    property var shadowLarge: { color: "#60000000", blur: 24, x: 0, y: 8 }

    // ── Animation durations ───────────────────────────────────────────────────
    property int durationFast:   120
    property int durationNormal: 240
    property int durationslow:   400

    function setTheme(name) {
        if (["dark","light","amoled"].includes(name)) currentTheme = name
    }

    function isDark() { return currentTheme !== "light" }
}
