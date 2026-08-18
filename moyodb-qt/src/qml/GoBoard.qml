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

    property int viewA: 1
    property int viewB: 0
    property int viewC: 0
    property int viewD: 1

    function viewOffsetX() {
        const maximum = root.boardSize - 1
        return (root.viewA < 0 ? maximum : 0)
             + (root.viewB < 0 ? maximum : 0)
    }

    function viewOffsetY() {
        const maximum = root.boardSize - 1
        return (root.viewC < 0 ? maximum : 0)
             + (root.viewD < 0 ? maximum : 0)
    }

    function boardToViewPoint(x, y) {
        return {
            "x": root.viewA * x
                 + root.viewB * y
                 + root.viewOffsetX(),
            "y": root.viewC * x
                 + root.viewD * y
                 + root.viewOffsetY()
        }
    }

    function viewToBoardPoint(x, y) {
        const shiftedX = x - root.viewOffsetX()
        const shiftedY = y - root.viewOffsetY()

        return {
            "x": root.viewA * shiftedX
                 + root.viewC * shiftedY,
            "y": root.viewB * shiftedX
                 + root.viewD * shiftedY
        }
    }

    function currentViewTransform() {
        return {
            "a": root.viewA,
            "b": root.viewB,
            "c": root.viewC,
            "d": root.viewD
        }
    }

    function setViewTransform(transform) {
        if (transform === undefined || transform === null)
            return

        root.viewA = Number(transform.a)
        root.viewB = Number(transform.b)
        root.viewC = Number(transform.c)
        root.viewD = Number(transform.d)

        boardCanvas.requestPaint()
    }

    function flipViewLeftRight() {
        root.viewA = -root.viewA
        root.viewB = -root.viewB
        boardCanvas.requestPaint()
    }

    function flipViewTopBottom() {
        root.viewC = -root.viewC
        root.viewD = -root.viewD
        boardCanvas.requestPaint()
    }

    function rotateViewCounterClockwise() {
        const oldA = root.viewA
        const oldB = root.viewB
        const oldC = root.viewC
        const oldD = root.viewD

        root.viewA = oldC
        root.viewB = oldD
        root.viewC = -oldA
        root.viewD = -oldB

        boardCanvas.requestPaint()
    }

    /*
     * A lightweight strategic influence field derived from the current
     * board position. Positive values represent Black influence and
     * negative values White influence.
     */
    property bool influenceVisible: false
    property var influenceValues: []
    property var enclosureValues: []

    property int selectedContinuationX: -1
    property int selectedContinuationY: -1

    onContinuationPointsChanged: boardCanvas.requestPaint()
    onSelectedContinuationXChanged: boardCanvas.requestPaint()

    onInfluenceVisibleChanged: {
        rebuildInfluenceValues()
        boardCanvas.requestPaint()
    }
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

    function evaluateInfluence(stones) {
        if (stones === null || stones.length === 0) {
            return {
                "influence": [],
                "enclosure": []
            }
        }

        const size = root.boardSize
        const pointCount = size * size
        const occupied = new Array(pointCount).fill("")

        for (const stone of stones) {
            const x = Number(stone.x)
            const y = Number(stone.y)

            if (x >= 0 && x < size && y >= 0 && y < size)
                occupied[y * size + x] = stone.color
        }

        /*
         * Propagate one colour's influence through the board.
         *
         * Own stones are sources. Empty points transmit influence with
         * attenuation. Opposing stones are barriers: influence must go
         * around them rather than passing straight through them.
         *
         * We retain the strongest path to each point rather than summing
         * every nearby stone. This prevents a dense group from acquiring
         * exaggerated influence simply because it contains many stones.
         */
        function propagatedField(colour) {
            const opponent = colour === "black" ? "white" : "black"
            const values = new Array(pointCount).fill(0.0)
            const queue = []
            let queueIndex = 0

            for (let index = 0; index < pointCount; ++index) {
                if (occupied[index] === colour) {
                    values[index] = 1.0
                    queue.push(index)
                }
            }

            const directions = [
                [ 1,  0, 0.76],
                [-1,  0, 0.76],
                [ 0,  1, 0.76],
                [ 0, -1, 0.76],

                /*
                * Diagonal propagation keeps the field visually rounded,
                * but is weaker than orthogonal propagation.

                */
                [ 1,  1, 0.60],
                [ 1, -1, 0.60],
                [-1,  1, 0.60],
                [-1, -1, 0.60]
                                ]

            while (queueIndex < queue.length) {
                const index = queue[queueIndex++]
                const x = index % size
                const y = Math.floor(index / size)
                const current = values[index]

                for (const direction of directions) {
                    const nx = x + direction[0]
                    const ny = y + direction[1]

                    if (nx < 0 || nx >= size || ny < 0 || ny >= size)
                        continue

                    const neighbour = ny * size + nx

                    /*
                     * Enemy stones stop this colour's field completely.
                     */
                    if (occupied[neighbour] === opponent)
                        continue

                    /*
                     * Own stones are already full-strength sources.
                     */
                    if (occupied[neighbour] === colour)
                        continue

                    const candidate = current * direction[2]

                    /*
                     * Once the field becomes visually insignificant there
                     * is no benefit in propagating it farther.
                     */
                    if (candidate < 0.025)
                        continue

                    if (candidate <= values[neighbour] + 0.001)
                        continue

                    values[neighbour] = candidate
                    queue.push(neighbour)
                }
            }

            return values
        }

        const black = propagatedField("black")
        const white = propagatedField("white")
        const combined = new Array(pointCount).fill(0.0)

        /*
         * The displayed value is comparative influence.
         *
         * A value near zero deliberately means "neither side has a clear
         * claim here". This is important: the map should be able to show
         * contested or dame-like areas instead of always choosing a colour.
         */
        for (let index = 0; index < pointCount; ++index) {
            if (occupied[index] !== "") {
                combined[index] =
                    occupied[index] === "black" ? 1.0 : -1.0
                continue
            }

            combined[index] = black[index] - white[index]
        }

        /*
         * Estimate enclosure by escape resistance.
         *
         * The Go board is treated as a discrete graph of intersections.
         * Empty intersections connect orthogonally. Stones are hard barriers.
         *
         * First identify genuinely open / contested empty intersections.
         * Then ask how difficult it is to reach that open space from every
         * other empty intersection.
         *
         * A point inside Black's sphere should require crossing substantial
         * Black influence to escape; likewise for White. Merely being far
         * from open space is not enough, so weighted escape distance is
         * compared with ordinary graph distance.
         */
        const enclosure = new Array(pointCount).fill(0.0)

        const orthogonalDirections = [
            [ 1,  0],
            [-1,  0],
            [ 0,  1],
            [ 0, -1]
        ]

        const openSeeds = []

        for (let index = 0; index < pointCount; ++index) {
            if (occupied[index] !== "")
                continue

            /*
             * Near-balance is deliberately treated as open/contested.
             *
             * K2/L2-type points should therefore act as exits rather than
             * accidentally becoming territorial centres.
             */
            if (Math.abs(combined[index]) <= 0.10)
                openSeeds.push(index)
        }

        /*
         * A position can theoretically contain no near-balanced empty point.
         * In that case use the least-dominated empty points as fallback exits.
         */
        if (openSeeds.length === 0) {
            const candidates = []

            for (let index = 0; index < pointCount; ++index) {
                if (occupied[index] === "") {
                    candidates.push({
                        index: index,
                        magnitude: Math.abs(combined[index])
                    })
                }
            }

            candidates.sort(
                (a, b) => a.magnitude - b.magnitude
            )

            for (let n = 0;
                 n < Math.min(8, candidates.length);
                 ++n) {
                openSeeds.push(candidates[n].index)
            }
        }

        /*
         * Dijkstra on a 19 x 19 board is tiny. Using a simple O(N^2)
         * implementation keeps the code clear and deterministic.
         *
         * colourSign:
         *   0  -> ordinary graph distance
         *  +1  -> resistance through Black influence
         *  -1  -> resistance through White influence
         */
        function escapeDistances(colourSign) {
            const infinity = 1.0e30
            const distances =
                new Array(pointCount).fill(infinity)
            const visited =
                new Array(pointCount).fill(false)

            for (const seed of openSeeds)
                distances[seed] = 0.0

            for (let iteration = 0;
                 iteration < pointCount;
                 ++iteration) {
                let bestIndex = -1
                let bestDistance = infinity

                for (let index = 0;
                     index < pointCount;
                     ++index) {
                    if (!visited[index]
                            && distances[index] < bestDistance) {
                        bestDistance = distances[index]
                        bestIndex = index
                    }
                }

                if (bestIndex < 0)
                    break

                visited[bestIndex] = true

                const x = bestIndex % size
                const y = Math.floor(bestIndex / size)

                for (const direction of orthogonalDirections) {
                    const nx = x + direction[0]
                    const ny = y + direction[1]

                    if (nx < 0 || nx >= size
                            || ny < 0 || ny >= size) {
                        /*
                         * The physical board edge is a wall, not an escape.
                         */
                        continue
                    }

                    const neighbour = ny * size + nx

                    if (occupied[neighbour] !== "")
                        continue

                    let stepCost = 1.0

                    if (colourSign !== 0) {
                        const support =
                            Math.max(
                                0.0,
                                colourSign * combined[neighbour]
                            )

                        /*
                         * Moving through intersections dominated by this
                         * colour is increasingly difficult. The quadratic
                         * component makes genuinely strong influence much
                         * more significant than a faint local preference.
                         */
                        stepCost +=
                            3.2 * support
                            + 3.8 * support * support
                    }

                    const candidate =
                        distances[bestIndex] + stepCost

                    if (candidate < distances[neighbour])
                        distances[neighbour] = candidate
                }
            }

            return distances
        }

        const plainEscape =
            escapeDistances(0)

        const blackEscape =
            escapeDistances(1)

        const whiteEscape =
            escapeDistances(-1)

        for (let index = 0; index < pointCount; ++index) {
            if (occupied[index] !== "")
                continue

            if (plainEscape[index] >= 1.0e29)
                continue

            const blackResistance =
                Math.max(
                    0.0,
                    blackEscape[index] - plainEscape[index]
                )

            const whiteResistance =
                Math.max(
                    0.0,
                    whiteEscape[index] - plainEscape[index]
                )

            /*
             * Compare the two possible enclosing colours directly.
             */
            const resistanceDifference =
                blackResistance - whiteResistance

            const totalResistance =
                blackResistance + whiteResistance

            if (totalResistance < 0.20)
                continue

            /*
             * Strength measures how difficult escape is beyond mere
             * geometrical distance.
             */
            let strength =
                Math.abs(resistanceDifference)
                / (2.2 + Math.abs(resistanceDifference))

            /*
             * Board edges provide containing geometry. They do not choose
             * a colour, but they strengthen an already-supported enclosure.
             *
             * This is particularly relevant to genuine corner pockets such
             * as A19.
             */
            const x = index % size
            const y = Math.floor(index / size)

            const edgeDistances = [
                x,
                size - 1 - x,
                y,
                size - 1 - y
            ]

            let edgeSupport = 0.0

            for (const distance of edgeDistances) {
                if (distance === 0)
                    edgeSupport += 0.15
                else if (distance === 1)
                    edgeSupport += 0.09
                else if (distance === 2)
                    edgeSupport += 0.04
            }

            strength =
                Math.min(
                    0.95,
                    strength + Math.min(0.24, edgeSupport)
                )

            /*
             * A tiny resistance difference should remain unresolved even
             * near an edge.
             */
            if (Math.abs(resistanceDifference) < 0.18)
                continue

            if (strength < 0.18)
                continue

            enclosure[index] =
                Math.sign(resistanceDifference) * strength
        }

        return {
            "influence": combined,
            "enclosure": enclosure
        }
    }

    function rebuildInfluenceValues() {
        if (!root.influenceVisible) {
            root.influenceValues = []
            root.enclosureValues = []
            return
        }

        const result =
            evaluateInfluence(root.stones)

        root.influenceValues = result.influence
        root.enclosureValues = result.enclosure
    }

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

            drawInfluenceMap(ctx, left, top, spacing)

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

        function drawInfluenceMap(ctx, left, top, spacing) {
            if (!root.influenceVisible
                    || root.influenceValues === null
                    || root.influenceValues.length
                       !== root.boardSize * root.boardSize) {
                return
            }

            ctx.save()

            /*
             * Each intersection retains its calculated influence value.
             * Adjacent cells overlap slightly so there are no gaps, but
             * values are deliberately not interpolated: the differences
             * in strength are useful strategic information.
             */
            const halfCell = spacing * 0.53

            for (let y = 0; y < root.boardSize; ++y) {
                for (let x = 0; x < root.boardSize; ++x) {
                    const index = y * root.boardSize + x
                    const influence =
                        Number(root.influenceValues[index])

                    const enclosure =
                        root.enclosureValues !== null
                        && root.enclosureValues.length
                           === root.boardSize * root.boardSize
                        ? Number(root.enclosureValues[index])
                        : 0.0

                    let score = influence

                    /*
                     * Strong enclosure reinforces an influence field of the
                     * same colour. It can also make an otherwise quiet point
                     * visibly territory-like, but opposing evidence is not
                     * allowed simply to overwrite the influence result.
                     */
                    if (Math.abs(enclosure) >= 0.45) {
                        const sameDirection =
                            influence === 0.0
                            || Math.sign(influence)
                               === Math.sign(enclosure)

                        if (sameDirection) {
                            const enclosureStrength =
                                0.42
                                + 0.52 * Math.abs(enclosure)

                            if (Math.abs(score)
                                    < enclosureStrength) {
                                score =
                                    Math.sign(enclosure)
                                    * enclosureStrength
                            }
                        }
                    }

                    const magnitude = Math.abs(score)
                    const enclosureMagnitude = Math.abs(enclosure)

                    /*
                     * This is a discrete Go board, not a sampled continuous
                     * image. Render useful categories at intersections rather
                     * than implying precision through a continuous gradient.
                     *
                     * Strong enclosure supplies a consistent regional base
                     * colour. Local influence still matters, but no longer
                     * makes a recognised territorial region look patchy.
                     */
                    const enclosureAgrees =
                        enclosureMagnitude >= 0.50
                        && (
                            score === 0.0
                            || Math.sign(enclosure) === Math.sign(score)
                        )

                    let displayScore = score

                    if (enclosureAgrees) {
                        const enclosureBase =
                            0.52 + 0.34 * Math.min(1.0, enclosureMagnitude)

                        if (Math.abs(displayScore) < enclosureBase) {
                            displayScore =
                                Math.sign(enclosure) * enclosureBase
                        }
                    }

                    const displayMagnitude = Math.abs(displayScore)

                    /*
                     * Leave genuinely neutral points uncoloured, but retain
                     * a faint fourth band for low yet meaningful influence.
                     */
                    if (displayMagnitude < 0.06)
                        continue

                    let alpha

                    if (enclosureAgrees) {
                        /*
                         * Territory-like / strongly enclosed region.
                         * Use a firm, consistent base colour.
                         */
                        alpha = displayScore > 0.0 ? 0.40 : 0.52
                    } else if (displayMagnitude >= 0.40) {
                        /*
                         * Clear but not necessarily enclosed influence.
                         */
                        alpha = displayScore > 0.0 ? 0.25 : 0.33
                    } else if (displayMagnitude >= 0.15) {
                        /*
                         * Weak influence.
                         */
                        alpha = displayScore > 0.0 ? 0.13 : 0.17
                    } else {
                        /*
                         * Very faint influence: enough to show that the
                         * intersection is not strategically blank, without
                         * implying secure control.
                         */
                        alpha = displayScore > 0.0 ? 0.065 : 0.085
                    }

                    const viewPoint =
                        root.boardToViewPoint(x, y)

                    const centreX =
                        left + viewPoint.x * spacing

                    const centreY =
                        top + viewPoint.y * spacing

                    if (displayScore > 0.0) {
                        ctx.fillStyle =
                            "rgba(20, 20, 22, "
                            + alpha.toFixed(3) + ")"
                    } else {
                        ctx.fillStyle =
                            "rgba(255, 255, 246, "
                            + alpha.toFixed(3) + ")"
                    }

                    ctx.fillRect(
                        centreX - halfCell,
                        centreY - halfCell,
                        halfCell * 2,
                        halfCell * 2
                    )
                }
            }

            ctx.restore()
        }

        function drawStones(ctx, left, top, spacing) {
            const radius = spacing * 0.46

            for (const stone of root.stones) {
                const viewPoint =
                    root.boardToViewPoint(
                        Number(stone.x),
                        Number(stone.y))

                const x = left + viewPoint.x * spacing
                const y = top + viewPoint.y * spacing
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

            let selectedRadius = -1
            let selectedX = 0
            let selectedY = 0

            ctx.save()

            for (const point of root.continuationPoints) {
                const count = Number(point.count)

                if (count <= 0)
                    continue

                /*
                 * Circle area grows approximately with frequency while
                 * keeping uncommon professional continuations visible.
                 */
                const strength =
                    Math.sqrt(count / maximumCount)

                const radius =
                    spacing * (0.24 + 0.20 * strength)

                const viewPoint =
                    root.boardToViewPoint(
                        Number(point.x),
                        Number(point.y))

                const x = left + viewPoint.x * spacing
                const y = top + viewPoint.y * spacing

                /*
                 * Frequency is communicated by size rather than colour
                 * intensity. Every marker has the same visual status:
                 * "a professional continuation was played here".
                 */
                ctx.fillStyle = "rgba(190, 48, 35, 0.14)"
                ctx.strokeStyle = "rgba(125, 30, 22, 0.92)"
                ctx.lineWidth = Math.max(1.25, spacing * 0.060)

                ctx.beginPath()
                ctx.arc(x, y, radius, 0, Math.PI * 2)
                ctx.fill()
                ctx.stroke()

                /*
                 * Anchor the marker precisely on the Go intersection.
                 */
                ctx.fillStyle = "rgba(110, 25, 19, 0.88)"
                ctx.beginPath()
                ctx.arc(
                            x,
                            y,
                            Math.max(1.5, spacing * 0.055),
                            0,
                            Math.PI * 2)
                ctx.fill()

                if (point.x === root.selectedContinuationX
                        && point.y === root.selectedContinuationY) {
                    selectedX = x
                    selectedY = y
                    selectedRadius = radius
                }
            }

            if (selectedRadius >= 0) {
                ctx.strokeStyle = "rgba(75, 18, 14, 0.98)"
                ctx.lineWidth = Math.max(2, spacing * 0.085)

                ctx.beginPath()
                ctx.arc(
                            selectedX,
                            selectedY,
                            selectedRadius + spacing * 0.10,
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

            const viewPoint =
                root.boardToViewPoint(
                    root.lastMoveX,
                    root.lastMoveY)

            const x = left + viewPoint.x * spacing
            const y = top + viewPoint.y * spacing
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

            const firstPoint =
                root.boardToViewPoint(
                    root.patternStartX,
                    root.patternStartY)

            const lastPoint =
                root.boardToViewPoint(
                    root.patternEndX,
                    root.patternEndY)

            const firstX =
                Math.min(firstPoint.x, lastPoint.x)

            const firstY =
                Math.min(firstPoint.y, lastPoint.y)

            const lastX =
                Math.max(firstPoint.x, lastPoint.x)

            const lastY =
                Math.max(firstPoint.y, lastPoint.y)

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

            const viewPoint =
                root.boardToViewPoint(
                    root.hoverX,
                    root.hoverY)

            const x = left + viewPoint.x * spacing
            const y = top + viewPoint.y * spacing
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

            const viewX = Math.round(
                (mouseX - geometry.left) / geometry.spacing
            )

            const viewY = Math.round(
                (mouseY - geometry.top) / geometry.spacing
            )

            if (
                viewX < 0 ||
                viewY < 0 ||
                viewX >= root.boardSize ||
                viewY >= root.boardSize
            ) {
                return null
            }

            return root.viewToBoardPoint(viewX, viewY)
        }

        function pointAt(mouseX, mouseY) {
            const geometry = boardGeometry()

            const viewX = Math.round(
                (mouseX - geometry.left) / geometry.spacing
            )

            const viewY = Math.round(
                (mouseY - geometry.top) / geometry.spacing
            )

            if (
                viewX < 0 ||
                viewY < 0 ||
                viewX >= root.boardSize ||
                viewY >= root.boardSize
            ) {
                return null
            }

            const pointX =
                geometry.left + viewX * geometry.spacing

            const pointY =
                geometry.top + viewY * geometry.spacing

            const distanceX = mouseX - pointX
            const distanceY = mouseY - pointY

            const distance = Math.sqrt(
                distanceX * distanceX +
                distanceY * distanceY
            )

            if (distance > geometry.spacing * 0.48)
                return null

            return root.viewToBoardPoint(viewX, viewY)
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
            root.rebuildInfluenceValues()
            boardCanvas.requestPaint()
        }

        function onBoardSizeChanged() {
            root.rebuildInfluenceValues()
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
