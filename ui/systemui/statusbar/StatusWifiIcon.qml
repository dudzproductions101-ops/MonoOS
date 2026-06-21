// StatusWifiIcon.qml – Wi-Fi signal arc indicator
import QtQuick 2.15

Canvas {
    id: root
    property int strength: 0  // 0-3
    width: 16; height: 12

    onStrengthChanged: requestPaint()

    onPaint: {
        var ctx = getContext("2d")
        ctx.clearRect(0, 0, width, height)
        ctx.strokeStyle = "white"
        ctx.lineCap = "round"
        var cx = width / 2, cy = height

        for (var i = 0; i < 3; i++) {
            ctx.beginPath()
            ctx.globalAlpha = i < strength ? 1.0 : 0.25
            ctx.lineWidth = 1.5
            var r = 4 + i * 4
            ctx.arc(cx, cy, r, Math.PI * 1.2, Math.PI * 1.8)
            ctx.stroke()
        }
        // Dot
        ctx.globalAlpha = strength > 0 ? 1.0 : 0.25
        ctx.beginPath()
        ctx.arc(cx, cy, 1.5, 0, Math.PI * 2)
        ctx.fillStyle = "white"
        ctx.fill()
    }
}
