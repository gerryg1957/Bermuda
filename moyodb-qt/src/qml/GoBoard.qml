import QtQuick
import org.kde.kirigami as Kirigami

Item {
    id: root

    property int boardSize: 19

    // Each stone is an object such as:
    // { x: 3, y: 3, color: "black" }
    property var stones: []

    property real boardPadding: Kirigami.Units.gridUnit * 1.5
    property color boardColor: "#d8a45b"
    property color lineColor: "#30251a"

    implicitWidth: Kirigami.Units.gridUnit * 28
    implicitHeight: implicitWidth

    Canvas {
        id: boardCanvas
        anchors.fill: parent

        onPaint: {
            const ctx = getContext("2d")
            ctx.reset()

            const side = Math.min(width, height)
            const left = (width - side) / 2 + root.boardPadding
            const top = (height - side) / 2 + root.boardPadding
            const usable = side - root.boardPadding * 2
            const spacing = usable / (root.boardSize - 1)

            // Wooden board background.
            ctx.fillStyle = root.boardColor
            ctx.fillRect(
                (width - side) / 2,
                (height - side) / 2,
                side,
                side
            )

            // Grid.
            ctx.strokeStyle = root.lineColor
            ctx.lineWidth = 1

            for (let index = 0; index < root.boardSize; ++index) {
                const offset = index * spacing

                ctx.beginPath()
                ctx.moveTo(left, top + offset)
                ctx.lineTo(left + usable, top + offset)
                ctx.stroke()

                ctx.beginPath()
                ctx.moveTo(left + offset, top)
                ctx.lineTo(left + offset, top + usable)
                ctx.stroke()
            }

            drawStarPoints(ctx, left, top, spacing)
            drawStones(ctx, left, top, spacing)
        }

        function drawStarPoints(ctx, left, top, spacing) {
            if (root.boardSize !== 19)
                return

            const starCoordinates = [
                [3, 3], [9, 3], [15, 3],
                [3, 9], [9, 9], [15, 9],
                [3, 15], [9, 15], [15, 15]
            ]

            ctx.fillStyle = root.lineColor

            for (const point of starCoordinates) {
                const x = left + point[0] * spacing
                const y = top + point[1] * spacing

                ctx.beginPath()
                ctx.arc(
                    x,
                    y,
                    Math.max(2.5, spacing * 0.11),
                    0,
                    Math.PI * 2
                )
                ctx.fill()
            }
        }

        function drawStones(ctx, left, top, spacing) {
            const radius = spacing * 0.46

            for (const stone of root.stones) {
                const x = left + stone.x * spacing
                const y = top + stone.y * spacing

                const gradient = ctx.createRadialGradient(
                    x - radius * 0.35,
                    y - radius * 0.35,
                    radius * 0.1,
                    x,
                    y,
                    radius
                )

                if (stone.color === "black") {
                    gradient.addColorStop(0, "#666666")
                    gradient.addColorStop(0.4, "#202020")
                    gradient.addColorStop(1, "#050505")
                    ctx.strokeStyle = "#000000"
                } else {
                    gradient.addColorStop(0, "#ffffff")
                    gradient.addColorStop(0.65, "#eeeeee")
                    gradient.addColorStop(1, "#b8b8b8")
                    ctx.strokeStyle = "#777777"
                }

                ctx.fillStyle = gradient
                ctx.lineWidth = 1

                ctx.beginPath()
                ctx.arc(x, y, radius, 0, Math.PI * 2)
                ctx.fill()
                ctx.stroke()
            }
        }

        Connections {
            target: root

            function onStonesChanged() {
                boardCanvas.requestPaint()
            }

            function onBoardSizeChanged() {
                boardCanvas.requestPaint()
            }
        }

        onWidthChanged: requestPaint()
        onHeightChanged: requestPaint()
    }

    Rectangle {
        anchors.fill: boardCanvas
        color: "transparent"
        border.width: 1
        border.color: Kirigami.Theme.textColor
    }
}
