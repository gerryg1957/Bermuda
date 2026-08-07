import QtQuick
import org.kde.kirigami as Kirigami

Item {
    id: root

    signal pointClicked(int x, int y)
    signal continuationPointClicked(int x, int y, int count)
    signal patternSelected(int left, int top,
                           int right, int bottom)

        property int boardSize: 19
    property var stones: []
    property var continuationPoints: []
    property int selectedContinuationX: -1
    property int selectedContinuationY: -1

    onContinuationPointsChanged: boardCanvas.requestPaint()
    onSelectedContinuationXChanged: boardCanvas.requestPaint()
    onSelectedContinuationYChanged: boardCanvas.requestPaint()

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
            drawContinuationMap(ctx, left, top, spacing)
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

                if (stone.color === "black") {
                    const gradient = ctx.createRadialGradient(
                        x - radius * 0.35,
                        y - radius * 0.35,
                        radius * 0.1,
                        x,
                        y,
                        radius
                    )

                    gradient.addColorStop(0, "#666666")
                    gradient.addColorStop(0.4, "#202020")
                    gradient.addColorStop(1, "#050505")

                    ctx.fillStyle = gradient
                    ctx.strokeStyle = "#000000"
                    ctx.lineWidth = 1

                    ctx.beginPath()
                    ctx.arc(x, y, radius, 0, Math.PI * 2)
                    ctx.fill()
                    ctx.stroke()

                    continue
                }

                /*
                 * Warm shell body. The highlight is offset towards the
                 * upper left, while the lower-right edge is slightly darker.
                 */
                const shellGradient = ctx.createRadialGradient(
                    x - radius * 0.38,
                    y - radius * 0.42,
                    radius * 0.06,
                    x + radius * 0.08,
                    y + radius * 0.10,
                    radius * 1.08
                )

                shellGradient.addColorStop(0.00, "#ffffff")
                shellGradient.addColorStop(0.32, "#fbfaf5")
                shellGradient.addColorStop(0.70, "#eeeae0")
                shellGradient.addColorStop(1.00, "#bdb8ae")

                ctx.fillStyle = shellGradient
                ctx.strokeStyle = "#77736b"
                ctx.lineWidth = Math.max(0.8, radius * 0.045)

                ctx.beginPath()
                ctx.arc(x, y, radius, 0, Math.PI * 2)
                ctx.fill()
                ctx.stroke()

                /*
                 * Everything below is clipped to the circular stone.
                 */
                ctx.save()

                ctx.beginPath()
                ctx.arc(
                    x,
                    y,
                    radius * 0.96,
                    0,
                    Math.PI * 2
                )
                ctx.clip()

                /*
                 * A very soft pearlescent wash helps the stone look like
                 * polished shell rather than plain white plastic.
                 */
                const pearlGradient = ctx.createLinearGradient(
                    x - radius,
                    y - radius,
                    x + radius,
                    y + radius
                )

                pearlGradient.addColorStop(
                    0.00,
                    "rgba(255, 255, 255, 0.30)"
                )
                pearlGradient.addColorStop(
                    0.43,
                    "rgba(255, 252, 241, 0.08)"
                )
                pearlGradient.addColorStop(
                    0.72,
                    "rgba(181, 173, 157, 0.06)"
                )
                pearlGradient.addColorStop(
                    1.00,
                    "rgba(110, 104, 94, 0.12)"
                )

                ctx.fillStyle = pearlGradient
                ctx.fillRect(
                    x - radius,
                    y - radius,
                    radius * 2,
                    radius * 2
                )

                /*
                 * Clamshell growth bands. Their small deterministic variation
                 * means neighbouring stones do not all look identical, while
                 * remaining stable whenever the Canvas is repainted.
                 */
                const phase =
                    (stone.x * 7 + stone.y * 11) * 0.37

                const bandCount = 5

                for (let band = 0;
                     band < bandCount;
                     ++band) {
                    const offset =
                        (band - (bandCount - 1) / 2)
                        * radius * 0.29

                    const bend =
                        Math.sin(phase + band * 1.23)
                        * radius * 0.085

                    ctx.strokeStyle = band % 2 === 0
                        ? "rgba(116, 108, 94, 0.12)"
                        : "rgba(143, 132, 112, 0.09)"

                    ctx.lineWidth =
                        Math.max(0.35, radius * 0.020)

                    ctx.beginPath()

                    ctx.moveTo(
                        x - radius * 0.90,
                        y + offset - radius * 0.10
                    )

                    ctx.bezierCurveTo(
                        x - radius * 0.34,
                        y + offset + bend - radius * 0.06,
                        x + radius * 0.34,
                        y + offset - bend + radius * 0.06,
                        x + radius * 0.90,
                        y + offset + radius * 0.10
                    )

                    ctx.stroke()

                    /*
                     * A neighbouring pale edge suggests light catching the
                     * shallow ridge of the shell growth line.
                     */
                    ctx.strokeStyle =
                        "rgba(255, 255, 255, 0.10)"

                    ctx.lineWidth =
                        Math.max(0.25, radius * 0.010)

                    ctx.beginPath()

                    ctx.moveTo(
                        x - radius * 0.90,
                        y + offset
                            - radius * 0.065
                    )

                    ctx.bezierCurveTo(
                        x - radius * 0.34,
                        y + offset + bend
                            - radius * 0.025,
                        x + radius * 0.34,
                        y + offset - bend
                            + radius * 0.095,
                        x + radius * 0.90,
                        y + offset
                            + radius * 0.135
                    )

                    ctx.stroke()
                }

                ctx.restore()

                /*
                 * Redraw a clean rim after applying the clipped texture.
                 */
                ctx.strokeStyle = "rgba(91, 87, 80, 0.58)"
                ctx.lineWidth = Math.max(0.75, radius * 0.040)

                ctx.beginPath()
                ctx.arc(x, y, radius, 0, Math.PI * 2)
                ctx.stroke()
            }
        }

                function drawContinuationMap(ctx, left, top, spacing) {
            if (root.continuationPoints === null
                    || root.continuationPoints.length === 0) {
                return
            }

            let maximumCount = 0

            for (const point of root.continuationPoints) {
                maximumCount = Math.max(
                            maximumCount,
                            Number(point.count))
            }

            if (maximumCount <= 0)
                return

            ctx.save()

            for (const point of root.continuationPoints) {
                const count = Number(point.count)

                if (count <= 0)
                    continue

                const strength =
                    Math.sqrt(count / maximumCount)

                const radius =
                    spacing * (0.20 + 0.27 * strength)

                const alpha =
                    0.18 + 0.50 * strength

                const x = left + point.x * spacing
                const y = top + point.y * spacing

                ctx.fillStyle =
                    "rgba(190, 48, 35, " + alpha + ")"

                ctx.strokeStyle =
                    "rgba(100, 24, 18, "
                    + Math.min(0.88, alpha + 0.18)
                    + ")"

                ctx.lineWidth =
                    Math.max(1, spacing * 0.055)

                ctx.beginPath()
                ctx.arc(x, y, radius, 0, Math.PI * 2)
                ctx.fill()
                ctx.stroke()
            }

                        if (root.selectedContinuationX >= 0
                    && root.selectedContinuationY >= 0) {
                const selectedX =
                    left + root.selectedContinuationX * spacing

                const selectedY =
                    top + root.selectedContinuationY * spacing

                ctx.strokeStyle = "rgba(80, 20, 16, 0.95)"
                ctx.lineWidth = Math.max(2, spacing * 0.075)

                ctx.beginPath()
                ctx.arc(selectedX,
                        selectedY,
                        spacing * 0.53,
                        0,
                        Math.PI * 2)
                ctx.stroke()
            }

ctx.restore()
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

            if (point === null)
                return

            if (root.continuationPoints !== null) {
                for (const continuation of root.continuationPoints) {
                    if (continuation.x === point.x
                            && continuation.y === point.y) {
                        root.continuationPointClicked(
                                    point.x,
                                    point.y,
                                    Number(continuation.count))
                        return
                    }
                }
            }

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
