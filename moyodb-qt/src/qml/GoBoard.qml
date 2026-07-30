import QtQuick
import org.kde.kirigami as Kirigami

Item {
    id: root

    signal pointClicked(int x, int y)

    property int boardSize: 19
    property var stones: []
    property int hoverX: -1
    property int hoverY: -1
    property bool hoverValid: false

    property real boardPadding: Kirigami.Units.gridUnit * 1.5
    property color boardColor: "#d8a45b"
    property color lineColor: "#30251a"

    implicitWidth: Kirigami.Units.gridUnit * 28
    implicitHeight: implicitWidth

    Canvas {
        id: boardCanvas
        anchors.fill: parent
        z: 0

        onPaint: {
            const ctx = getContext("2d")
            ctx.reset()

            const side = Math.min(width, height)
            const left = (width - side) / 2 + root.boardPadding
            const top = (height - side) / 2 + root.boardPadding
            const usable = side - root.boardPadding * 2
            const spacing = usable / (root.boardSize - 1)

            ctx.fillStyle = root.boardColor
            ctx.fillRect(
                (width - side) / 2,
                (height - side) / 2,
                side,
                side
            )

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
            drawHoverMarker(ctx, left, top, spacing)
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

        function drawHoverMarker(ctx, left, top, spacing) {
            if (!root.hoverValid)
                return

            const x = left + root.hoverX * spacing
            const y = top + root.hoverY * spacing
            const radius = spacing * 0.42

            ctx.fillStyle = "rgba(80, 140, 220, 0.35)"
            ctx.strokeStyle = "rgba(30, 80, 170, 0.9)"
            ctx.lineWidth = 2

            ctx.beginPath()
            ctx.arc(x, y, radius, 0, Math.PI * 2)
            ctx.fill()
            ctx.stroke()
        }

        onWidthChanged: requestPaint()
        onHeightChanged: requestPaint()
    }

    MouseArea {
        anchors.fill: parent
        z: 1
        hoverEnabled: true

        function boardGeometry() {
            const side = Math.min(width, height)
            const left = (width - side) / 2 + root.boardPadding
            const top = (height - side) / 2 + root.boardPadding
            const usable = side - root.boardPadding * 2
            const spacing = usable / (root.boardSize - 1)

            return {
                "left": left,
                "top": top,
                "spacing": spacing
            }
        }

        function pointAt(mouseX, mouseY) {
            const geometry = boardGeometry()

            const boardX = Math.round(
                (mouseX - geometry.left) / geometry.spacing
            )

            const boardY = Math.round(
                (mouseY - geometry.top) / geometry.spacing
            )

            if (
                boardX < 0 ||
                boardY < 0 ||
                boardX >= root.boardSize ||
                boardY >= root.boardSize
            ) {
                return null
            }

            const pointX =
                geometry.left + boardX * geometry.spacing

            const pointY =
                geometry.top + boardY * geometry.spacing

            const distanceX = mouseX - pointX
            const distanceY = mouseY - pointY

            const distance = Math.sqrt(
                distanceX * distanceX +
                distanceY * distanceY
            )

            if (distance > geometry.spacing * 0.48)
                return null

            return {
                "x": boardX,
                "y": boardY
            }
        }

        onPositionChanged: mouse => {
            const point = pointAt(mouse.x, mouse.y)

            if (point === null) {
                root.hoverValid = false
            } else {
                root.hoverX = point.x
                root.hoverY = point.y
                root.hoverValid = true
            }

            boardCanvas.requestPaint()
        }

        onExited: {
            root.hoverValid = false
            boardCanvas.requestPaint()
        }

        onClicked: mouse => {
            const point = pointAt(mouse.x, mouse.y)

            if (point !== null)
                root.pointClicked(point.x, point.y)
        }
    }

    Rectangle {
        anchors.fill: parent
        z: 2
        color: "transparent"
        border.width: 1
        border.color: Kirigami.Theme.textColor
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
}
