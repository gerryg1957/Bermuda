import QtQuick
import org.kde.kirigami as Kirigami

Item {
    id: root

    signal pointClicked(int x, int y)
    signal patternSelected(int left, int top,
                           int right, int bottom)

        property int boardSize: 19
    property var stones: []

    property bool showCoordinates: true
    property int lastMoveX: -1
    property int lastMoveY: -1
    property int lastMoveNumber: 0
    property int hoverX: -1
    property int hoverY: -1
    property bool hoverValid: false

    property bool patternSelectionEnabled: false
    property bool patternSelectionDragging: false
    property int patternStartX: -1
    property int patternStartY: -1
    property int patternEndX: -1
    property int patternEndY: -1

    readonly property bool patternSelectionValid:
        patternStartX >= 0
        && patternStartY >= 0
        && patternEndX >= 0
        && patternEndY >= 0

    property real boardPadding: Kirigami.Units.gridUnit * 1.5
    property color boardColor: "#d8a45b"
    property color lineColor: "#30251a"

    implicitWidth: Kirigami.Units.gridUnit * 28
    implicitHeight: implicitWidth

    function clearPatternSelection() {
        patternSelectionDragging = false
        patternStartX = -1
        patternStartY = -1
        patternEndX = -1
        patternEndY = -1

        boardCanvas.requestPaint()
    }

    function setPatternSelection(left, top, right, bottom) {
        patternSelectionDragging = false
        patternStartX = left
        patternStartY = top
        patternEndX = right
        patternEndY = bottom

        boardCanvas.requestPaint()
    }

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

            drawWoodGrain(
                ctx,
                (width - side) / 2,
                (height - side) / 2,
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
            drawCoordinates(ctx, left, top, usable, spacing)
            drawStones(ctx, left, top, spacing)
            drawPatternSelection(ctx, left, top, spacing)
            drawLastMoveNumber(ctx, left, top, spacing)
            drawHoverMarker(ctx, left, top, spacing)
        }

                function drawWoodGrain(ctx, left, top, side) {
                    const grainLines = 104
                    const segments = 72

                    ctx.save()

                    ctx.beginPath()
                    ctx.rect(left, top, side, side)
                    ctx.clip()

                    /*
                     * Broad, nearly imperceptible tonal bands prevent the surface
                     * from looking like a flat colour while keeping the board calm.
                     */
                    for (let band = 0; band < 14; ++band) {
                        const bandX =
                            left + side * (band + 0.5) / 14

                        const bandWidth =
                            side * (0.032 + 0.009 * Math.sin(band * 1.73))

                        const gradient = ctx.createLinearGradient(
                            bandX - bandWidth,
                            top,
                            bandX + bandWidth,
                            top
                        )

                        gradient.addColorStop(
                            0.0,
                            "rgba(255, 225, 165, 0.00)"
                        )

                        gradient.addColorStop(
                            0.5,
                            band % 3 === 0
                                ? "rgba(255, 226, 166, 0.055)"
                                : "rgba(115, 72, 30, 0.028)"
                        )

                        gradient.addColorStop(
                            1.0,
                            "rgba(255, 225, 165, 0.00)"
                        )

                        ctx.fillStyle = gradient
                        ctx.fillRect(
                            bandX - bandWidth,
                            top,
                            bandWidth * 2,
                            side
                        )
                    }

                    /*
                     * Fine vertical fibres. Most are very restrained; occasional
                     * paired lines suggest carefully selected straight-grained kaya.
                     */
                    for (let grain = 0; grain < grainLines; ++grain) {
                        const progressX = (grain + 0.5) / grainLines

                        const groupedOffset =
                            Math.sin(grain * 0.41) * side * 0.00065
                            + Math.sin(grain * 1.87) * side * 0.00025

                        const baseX =
                            left + side * progressX + groupedOffset

                        const phase = grain * 0.71

                        const amplitude =
                            side * (
                                0.00042
                                + 0.00008 * (grain % 5)
                            )

                        const featureLine = grain % 17 === 0
                        const companionLine = grain % 17 === 1

                        if (featureLine) {
                            ctx.strokeStyle =
                                "rgba(82, 48, 19, 0.25)"

                            ctx.lineWidth =
                                Math.max(0.70, side / 1250)
                        } else if (companionLine) {
                            ctx.strokeStyle =
                                "rgba(126, 78, 31, 0.18)"

                            ctx.lineWidth =
                                Math.max(0.52, side / 1550)
                        } else {
                            const opacity =
                                0.105 + (grain % 7) * 0.006

                            ctx.strokeStyle =
                                "rgba(105, 65, 27, "
                                + opacity.toFixed(3)
                                + ")"

                            ctx.lineWidth =
                                Math.max(0.38, side / 1900)
                        }

                        ctx.beginPath()

                        for (let segment = 0;
                             segment <= segments;
                             ++segment) {
                            const progressY = segment / segments
                            const y = top + progressY * side

                            /*
                             * Long, slow movement dominates. The small secondary
                             * movements prevent the fibres looking computer-straight.
                             */
                            const wave =
                                Math.sin(
                                    progressY * Math.PI * 1.55
                                    + phase
                                )
                                + 0.16 * Math.sin(
                                    progressY * Math.PI * 4.8
                                    + phase * 1.37
                                )
                                + 0.045 * Math.sin(
                                    progressY * Math.PI * 12.5
                                    + grain * 0.29
                                )

                            const x = baseX + amplitude * wave

                            if (segment === 0)
                                ctx.moveTo(x, y)
                            else
                                ctx.lineTo(x, y)
                        }

                        ctx.stroke()

                        /*
                         * A restrained highlight beside only the strongest fibres
                         * suggests a polished surface without making it glossy.
                         */
                        if (featureLine) {
                            ctx.strokeStyle =
                                "rgba(255, 226, 170, 0.13)"

                            ctx.lineWidth =
                                Math.max(0.34, side / 2300)

                            ctx.translate(0.75, 0)
                            ctx.stroke()
                            ctx.translate(-0.75, 0)
                        }
                    }

                    ctx.restore()
                }

        function coordinateLetter(index) {
            const skippedI = index >= 8 ? 1 : 0
            return String.fromCharCode(
                        "A".charCodeAt(0) + index + skippedI)
        }

        function drawCoordinates(ctx, left, top, usable, spacing) {
            if (!root.showCoordinates)
                return

            const offset = root.boardPadding * 0.52

            ctx.fillStyle = root.lineColor
            ctx.font = Math.max(10, spacing * 0.36)
                    + "px sans-serif"
            ctx.textAlign = "center"
            ctx.textBaseline = "middle"

            for (let index = 0;
                 index < root.boardSize;
                 ++index) {
                const x = left + index * spacing
                const y = top + index * spacing
                const letter = coordinateLetter(index)
                const number = root.boardSize - index

                ctx.fillText(letter, x, top - offset)
                ctx.fillText(letter, x, top + usable + offset)

                ctx.fillText(number, left - offset, y)
                ctx.fillText(number, left + usable + offset, y)
            }
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

                function drawLastMoveNumber(ctx, left, top, spacing) {
            if (root.lastMoveNumber <= 0
                    || root.lastMoveX < 0
                    || root.lastMoveY < 0) {
                return
            }

            let stoneColor = ""

            for (const stone of root.stones) {
                if (stone.x === root.lastMoveX
                        && stone.y === root.lastMoveY) {
                    stoneColor = stone.color
                    break
                }
            }

            if (stoneColor.length === 0)
                return

            const x = left + root.lastMoveX * spacing
            const y = top + root.lastMoveY * spacing
            const digits = root.lastMoveNumber.toString().length

            let fontScale = 0.50

            if (digits === 2)
                fontScale = 0.42
            else if (digits >= 3)
                fontScale = 0.34

            ctx.fillStyle = stoneColor === "black"
                    ? "#ffffff"
                    : "#000000"

            ctx.font = "bold "
                    + Math.max(9, spacing * fontScale)
                    + "px sans-serif"

            ctx.textAlign = "center"
            ctx.textBaseline = "middle"

            ctx.fillText(
                        root.lastMoveNumber.toString(),
                        x,
                        y)
        }

        function drawPatternSelection(ctx, left, top, spacing) {
            if (!root.patternSelectionValid)
                return

            const firstX = Math.min(
                root.patternStartX,
                root.patternEndX
            )

            const firstY = Math.min(
                root.patternStartY,
                root.patternEndY
            )

            const lastX = Math.max(
                root.patternStartX,
                root.patternEndX
            )

            const lastY = Math.max(
                root.patternStartY,
                root.patternEndY
            )

            const halfSpacing = spacing * 0.5
            const x = left + firstX * spacing - halfSpacing
            const y = top + firstY * spacing - halfSpacing
            const width = (lastX - firstX + 1) * spacing
            const height = (lastY - firstY + 1) * spacing

            ctx.fillStyle = "rgba(60, 120, 220, 0.22)"
            ctx.strokeStyle = "rgba(25, 75, 180, 0.95)"
            ctx.lineWidth = 2

            ctx.fillRect(x, y, width, height)
            ctx.strokeRect(x, y, width, height)
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

        property bool suppressSelectionClick: false

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

        function nearestPointAt(mouseX, mouseY) {
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

            return {
                "x": boardX,
                "y": boardY
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

        cursorShape: root.patternSelectionEnabled
            ? Qt.CrossCursor
            : Qt.ArrowCursor

        onPressed: mouse => {
            if (!root.patternSelectionEnabled)
                return

            const point = nearestPointAt(mouse.x, mouse.y)

            if (point === null)
                return

            root.patternSelectionDragging = true
            root.patternStartX = point.x
            root.patternStartY = point.y
            root.patternEndX = point.x
            root.patternEndY = point.y
            root.hoverValid = false

            boardCanvas.requestPaint()
        }

        onPositionChanged: mouse => {
            if (root.patternSelectionDragging) {
                const point = nearestPointAt(mouse.x, mouse.y)

                if (point !== null) {
                    root.patternEndX = point.x
                    root.patternEndY = point.y
                }

                boardCanvas.requestPaint()
                return
            }

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

        onReleased: mouse => {
            if (!root.patternSelectionDragging)
                return

            const point = nearestPointAt(mouse.x, mouse.y)

            if (point !== null) {
                root.patternEndX = point.x
                root.patternEndY = point.y
            }

            root.patternSelectionDragging = false

            // Completing a rubber-band selection also generates
            // MouseArea.onClicked after onReleased. By then,
            // pattern-selection mode has been switched off by
            // Main.qml, so consume that click rather than treating
            // it as a board-edit click.
            suppressSelectionClick = true

            Qt.callLater(function() {
                suppressSelectionClick = false
            })

            const left = Math.min(
                root.patternStartX,
                root.patternEndX
            )

            const top = Math.min(
                root.patternStartY,
                root.patternEndY
            )

            const right = Math.max(
                root.patternStartX,
                root.patternEndX
            )

            const bottom = Math.max(
                root.patternStartY,
                root.patternEndY
            )

            root.patternSelected(left, top, right, bottom)
            boardCanvas.requestPaint()
        }

        onCanceled: {
            root.patternSelectionDragging = false
            boardCanvas.requestPaint()
        }

        onExited: {
            root.hoverValid = false
            boardCanvas.requestPaint()
        }

        onClicked: mouse => {
            if (suppressSelectionClick) {
                suppressSelectionClick = false
                return
            }

            if (root.patternSelectionEnabled)
                return

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

                function onShowCoordinatesChanged() {
            boardCanvas.requestPaint()
        }

        function onLastMoveXChanged() {
            boardCanvas.requestPaint()
        }

        function onLastMoveYChanged() {
            boardCanvas.requestPaint()
        }

        function onLastMoveNumberChanged() {
            boardCanvas.requestPaint()
        }

    }
}
