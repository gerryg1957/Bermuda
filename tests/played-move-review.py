"""Exercise the real review panel with a minimal controller (requires PySide6).
Run: QT_QPA_PLATFORM=offscreen python3 tests/played-move-review.py
"""
import os
from pathlib import Path
import sys

os.environ.setdefault("QT_QPA_PLATFORM", "offscreen")
from PySide6.QtCore import QMetaObject, Q_RETURN_ARG, QUrl
from PySide6.QtGui import QGuiApplication
from PySide6.QtQml import QQmlComponent, QQmlEngine

app = QGuiApplication(sys.argv)
engine = QQmlEngine()
qml_dir = Path(__file__).resolve().parents[1] / "bermuda-qt/src/qml"
component = QQmlComponent(engine)
component.setData(('''
import QtQuick
import QtQuick.Controls
import "''' + qml_dir.as_uri() + '''" as Bermuda
Item {
    width: 1000; height: 900
    QtObject {
        id: fake
        property int move_number: 98
        property int move_count: 229
        property string stones_json: "[]"
        property string sgf_tree_json: "{}"
        property string whole_game_report: "{}"
        property string whole_game_status: "Analysis complete — saved"
        property bool whole_game_busy: false
        property bool candidate_analysis_busy: false
        property string error_message: ""
        property bool matches: true
        property var investigated: null
        function wholeGameMatchesLoaded() { return matches }
        function reviewPlayerRanks() { return "{}" }
        function analyseReviewMove(turn, x, y, visits) {
            investigated = {turn: turn, x: x, y: y}; return true
        }
    }
    Bermuda.WholeGameReviewPanel {
        id: panel
        anchors.fill: parent
        controller: fake
        positionOnly: true
        selectedTurn: 98
    }
    function check(ok, message) { if (!ok) throw new Error(message) }
    function load(vertex, candidate, positionOnly) {
        panel.clearOwnMove(); panel.endPreview()
        panel.positionOnly = positionOnly
        const moves = Array(99).fill({colour: "White", vertex: "Pass"})
        moves[98] = {colour: "Black", vertex: vertex}
        const results = Array(99).fill(null)
        results[98] = {current_player: "Black", score_lead: 10, win_rate: 0.99, visits: 1000,
            candidates: [{vertex: candidate, visits: 100, score_lead: 10, win_rate: 0.99}]}
        fake.whole_game_report = JSON.stringify({moves: moves, results: results, board_size: 19,
            total: positionOnly ? 1 : 100, completed: 1, focus_turn: positionOnly ? 98 : null})
    }
    function runTests(): string {
        const d16 = {Point: {x: 3, y: 15}}, g11 = {Point: {x: 6, y: 10}}
        load(d16, g11, true)
        check(panel.playedMovePoint.x === 3 && panel.playedMovePoint.y === 3, "D16 must be marked before move 99")
        check(panel.candidatePoints.length === 1 && panel.candidatePoints[0].x === 6, "Keep candidate G11")
        check(panel.vertexName(panel.recordedMove.vertex) === "D16", "Recorded move label")
        panel.investigatePlayedMove()
        check(fake.investigated.turn === 98 && fake.investigated.x === 3 && fake.investigated.y === 3, "Click investigates D16")
        panel.returnToGamePosition()
        check(panel.playedMovePoint !== null, "Return restores marker")
        load(d16, d16, true)
        check(panel.playedMovePoint !== null && panel.candidatePoints[0].recommended, "Played best move retains both markers")
        load(d16, g11, false)
        check(panel.playedMovePoint !== null, "Whole-game marker retained")
        panel.previewState = {stones: []}
        check(panel.playedMovePoint === null, "Hide during candidate preview")
        panel.endPreview()
        panel.choosingOwnMove = true
        check(panel.playedMovePoint === null, "Hide during own-move choice")
        panel.clearOwnMove()
        fake.matches = false
        check(panel.playedMovePoint === null, "Hide stale report marker")
        fake.matches = true
        load("Pass", g11, true)
        check(panel.playedMovePoint === null && panel.vertexName(panel.recordedMove.vertex) === "pass", "Pass has text, no board marker")
        fake.move_number = 229
        check(panel.recordedMove === null && panel.playedMovePoint === null, "No next move at game end")
        fake.move_number = 98
        load(d16, g11, true)
        return "PASS: single-position, whole-game, overlapping candidate, investigate/return, previews, stale report, pass and end"
    }
}
''').encode(), QUrl())
root = component.create()
if root is None:
    raise RuntimeError("\n".join(error.toString() for error in component.errors()))
result = QMetaObject.invokeMethod(root, "runTests", Q_RETURN_ARG(str))
assert result and result.startswith("PASS:"), result
# Labels are QQuickItems; QObject lookup avoids concrete generated QML types.
from PySide6.QtCore import QObject
heading = root.findChild(QObject, "recordedMoveHeading")
assert heading is not None and heading.property("visible")
assert heading.property("text") == "□ Played: Black D16 · move 99"
print(result)
