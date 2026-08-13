import QtQml

QtObject {
    id: root

    required property var board
    required property var controller

    property int searchRadius: 3
    property int measureRadius: 4

    function localDistance(
            fromEvaluation,
            toEvaluation,
            firstX,
            firstY,
            excludeX,
            excludeY) {
        const size = root.board.boardSize

        let total = 0.0
        let points = 0

        for (let y = Math.max(
                 0,
                 firstY - root.measureRadius);
             y <= Math.min(
                 size - 1,
                 firstY + root.measureRadius);
             ++y) {
            for (let x = Math.max(
                     0,
                     firstX - root.measureRadius);
                 x <= Math.min(
                     size - 1,
                     firstX + root.measureRadius);
                 ++x) {
                const dx = x - firstX
                const dy = y - firstY

                if (dx * dx + dy * dy
                        > root.measureRadius
                          * root.measureRadius) {
                    continue
                }

                /*
                 * The played intersections themselves contain automatic
                 * stone-value changes. Reply-aware influence is intended to
                 * measure what happened to the surrounding position.
                 */
                if (x === firstX && y === firstY)
                    continue

                if (excludeX >= 0
                        && x === excludeX
                        && y === excludeY) {
                    continue
                }

                const index = y * size + x

                total +=
                    Math.abs(
                        Number(
                            fromEvaluation.influence[index])
                        - Number(
                            toEvaluation.influence[index])
                    )

                ++points
            }
        }

        return points > 0
            ? total / points
            : 0.0
    }

    function analyse(
            moveNumber,
            firstX,
            firstY,
            firstColour,
            replyColour) {
        const originalEvaluation =
            root.board.evaluateInfluence(
                root.board.stones)

        const firstJson =
            root.controller.hypotheticalMoveStones(
                moveNumber,
                firstX,
                firstY,
                firstColour)

        if (firstJson.length === 0) {
            return {
                "legal": false,
                "error": root.controller.error_message
            }
        }

        const firstEvaluation =
            root.board.evaluateInfluence(
                JSON.parse(firstJson))

        const firstEffect =
            localDistance(
                originalEvaluation,
                firstEvaluation,
                firstX,
                firstY,
                -1,
                -1)

        const size = root.board.boardSize
        const results = []

        for (let replyY =
                 Math.max(
                     0,
                     firstY - root.searchRadius);
             replyY <=
                 Math.min(
                     size - 1,
                     firstY + root.searchRadius);
             ++replyY) {
            for (let replyX =
                     Math.max(
                         0,
                         firstX - root.searchRadius);
                 replyX <=
                     Math.min(
                         size - 1,
                         firstX + root.searchRadius);
                 ++replyX) {
                const dx = replyX - firstX
                const dy = replyY - firstY

                if (dx * dx + dy * dy
                        > root.searchRadius
                          * root.searchRadius) {
                    continue
                }

                if (replyX === firstX
                        && replyY === firstY) {
                    continue
                }

                const sequenceJson =
                    root.controller
                    .hypotheticalSequenceStones(
                        moveNumber,
                        firstX,
                        firstY,
                        firstColour,
                        replyX,
                        replyY,
                        replyColour)

                if (sequenceJson.length === 0)
                    continue

                const replyEvaluation =
                    root.board.evaluateInfluence(
                        JSON.parse(sequenceJson))

                const remainingEffect =
                    localDistance(
                        originalEvaluation,
                        replyEvaluation,
                        firstX,
                        firstY,
                        replyX,
                        replyY)

                results.push({
                    "x": replyX,
                    "y": replyY,
                    "remainingEffect":
                        remainingEffect,
                    "neutralised":
                        firstEffect - remainingEffect
                })
            }
        }

        results.sort(
            function(a, b) {
                return b.neutralised
                    - a.neutralised
            })

        if (results.length === 0) {
            return {
                "legal": true,
                "firstEffect": firstEffect,
                "legalReplies": 0,
                "bestReplyX": -1,
                "bestReplyY": -1,
                "remainingEffect": firstEffect,
                "persistence": 1.0
            }
        }

        const best = results[0]

        const persistence =
            firstEffect > 0.0
            ? best.remainingEffect / firstEffect
            : 0.0

        return {
            "legal": true,
            "firstEffect": firstEffect,
            "legalReplies": results.length,
            "bestReplyX": best.x,
            "bestReplyY": best.y,
            "remainingEffect": best.remainingEffect,
            "persistence": persistence
        }
    }
}
