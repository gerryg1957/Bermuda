import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Frame {
    id: panel
    padding: 12
    property var controller
    property bool positionOnly: false
    property string executable: ""
    property string network: ""
    property string configuration: ""
    property int visitBudget: 200
    property string komi: "6.5"
    property int selectedTurn: 0
    property string returnText: qsTr("Return")
    property bool showSuggestions: true
    property bool showWinRate: false
    property int selectedCandidate: -1
    property var previewState: null
    property int previewStep: 0
    property bool showEngineLine: true
    property bool deeperRequested: false
    property bool choosingOwnMove: false
    property string pendingWhyVertex: ""
    property int pendingWhyTurn: -1
    function clearOwnMove() { choosingOwnMove = false; pendingWhyVertex = ""; pendingWhyTurn = -1 }
    function candidateLabel(index) { return index < 26 ? String.fromCharCode(65 + index) : String(index + 1) }
    function investigatePoint(x, y) {
        if (!choosingOwnMove || !currentResult || !matchesLoaded) return
        const vertex = x === -1 && y === -1 ? "Pass" : {Point: {x: x, y: report.board_size - 1 - y}}
        pendingWhyVertex = JSON.stringify(vertex)
        pendingWhyTurn = selectedTurn
        deeperRequested = true
        startError = ""
        if (controller.analyseReviewMove(selectedTurn, x, y, deeperVisits.value)) choosingOwnMove = false
        else { pendingWhyVertex = ""; startError = controller.error_message }
    }
    readonly property bool previewActive: previewState !== null
    readonly property var chosenCandidate: candidates[selectedCandidate] || null
    readonly property int continuationLength: chosenCandidate
        ? Math.max(1, (chosenCandidate.principal_variation || []).length) : 0
    signal previewPosition(var position)
    signal savedContinuation()
    onPreviewStateChanged: previewPosition(previewState)
    onVisibleChanged: if (!visible) { endPreview(); clearOwnMove() }
    onMatchesLoadedChanged: if (!matchesLoaded) { endPreview(); clearOwnMove() }
    function endPreview() { previewState = null; previewStep = 0 }
    function chooseCandidate(index) {
        choosingOwnMove = false
        pendingWhyVertex = ""
        endPreview()
        selectedCandidate = index
        deeperVisits.value = Math.min(100000, Math.max(1000, chosenCandidate ? chosenCandidate.visits * 5 : 1000))
        previewCandidate(showEngineLine ? continuationLength : 1)
    }
    function previewCandidate(step) {
        if (!chosenCandidate || !matchesLoaded) return
        const data = JSON.parse(controller.previewWholeGameCandidate(selectedTurn, chosenCandidate.sourceIndex, step))
        if (!data.stones) { startError = controller.error_message; return }
        startError = ""
        previewStep = step
        previewState = data
    }
    function saveCandidate(fullLine) {
        if (!chosenCandidate) return
        const turn = selectedTurn, index = chosenCandidate.sourceIndex
        endPreview()
        if (controller.saveWholeGameCandidate(turn, index, fullLine === true)) savedContinuation()
        else startError = controller.error_message
    }
    property var gameIdentity
    property string gameTitle: ""
    property int contextRevision: 0
    property string startError: ""
    property var rawReport: {
        try { return JSON.parse(controller ? controller.whole_game_report : "{}") }
        catch (error) { return ({}) }
    }
    // Stored results remain on the controller; show them only for this line.
    property var report: matchesLoaded && ((rawReport.focus_turn !== null && rawReport.focus_turn !== undefined) === positionOnly) ? rawReport : ({})
    property var results: report.results || []
    property var currentResult: results[selectedTurn] || null
    property var candidates: {
        if (!currentResult) return []
        const all = (currentResult.candidates || []).map(function(c, index) { return Object.assign({}, c, {sourceIndex: index}) })
        function rank(c) { return c.ranking_visits === undefined ? c.visits : c.ranking_visits }
        const engine = all.filter(function(c) { return !c.user_move }).sort(function(a,b) { return rank(b) - rank(a) || a.sourceIndex - b.sourceIndex })
        const primary = engine.slice(0,5)
        const extra = engine.slice(5).filter(function(c) { return c.focused })
        const own = all.filter(function(c) { return c.user_move }).sort(function(a,b) { return panel.vertexName(a.vertex).localeCompare(panel.vertexName(b.vertex)) })
        return primary.concat(extra, own).map(function(c,index) { return Object.assign({},c,{recommended: engine.length > 0 && c.sourceIndex === engine[0].sourceIndex}) })
    }
    property var decisions: (report.decisions || []).filter(function(d) {
        return d.estimated_loss >= 1 && (colourFilter.currentIndex === 0
            || d.player === (colourFilter.currentIndex === 1 ? "Black" : "White"))
    }).sort(function(a, b) { return b.estimated_loss - a.estimated_loss })
    readonly property bool matchesLoaded: {
        const identity = gameIdentity
        const revision = contextRevision
        const savedReport = rawReport
        if (!controller) return false
        const move = controller.move_number
        const count = controller.move_count
        const stones = controller.stones_json
        const tree = controller.sgf_tree_json
        return controller.wholeGameMatchesLoaded()
    }
    onGameIdentityChanged: { endPreview(); contextRevision++; startError = ""; selectedCandidate = -1; deeperRequested = false }
    readonly property var candidatePoints: {
        if ((previewActive && previewStep > 1) || !showSuggestions || !matchesLoaded || !currentResult
                || controller.move_number !== selectedTurn) return []
        return candidates.map(function(c, index) {
            if (!c.vertex || !c.vertex.Point) return null
            return { x: c.vertex.Point.x, y: report.board_size - 1 - c.vertex.Point.y,
                visits: c.visits, label: candidateLabel(index), recommended: c.recommended, userMove: c.user_move,
                selected: index === selectedCandidate }
        }).filter(function(p) { return p !== null })
    }
    signal navigate(int moveNumber)
    signal leaveReview()
    background: Rectangle {
        color: panel.palette.window
        border.color: panel.palette.mid
        MouseArea { anchors.fill: parent; onWheel: function(wheel) { wheel.accepted = true } }
    }
    function vertexName(v) {
        if (v === "Pass") return qsTr("pass")
        return v && v.Point ? "ABCDEFGHJKLMNOPQRST"[v.Point.x] + (v.Point.y + 1) : "?"
    }
    function scoreName(value) { return (value >= 0 ? "B +" : "W +") + Math.abs(value).toFixed(1) }
    function selectTurn(turn) {
        clearOwnMove()
        endPreview()
        selectedTurn = Math.max(0, Math.min(results.length - 1, turn))
        selectedCandidate = -1
        navigate(selectedTurn)
        graph.requestPaint()
    }
    function navigationUnavailable() { navigationNotice.text = qsTr("Open the analysed game and line to navigate this saved report.") }
    function beginReview() {
        clearOwnMove()
        endPreview()
        deeperRequested = false
        contextRevision++
        startError = ""
        navigationNotice.text = ""
        selectedCandidate = -1
        selectedTurn = controller ? controller.move_number : 0
        tabs.currentIndex = positionOnly ? 1 : 0
        if (matchesLoaded && report.total && (!positionOnly || report.focus_turn === selectedTurn)) {
            visits.value = report.visits
            komiInput.text = String(report.komi)
            rules.currentIndex = Math.max(0, rules.find(report.rules))
        } else {
            visits.value = visitBudget
            komiInput.text = komi
        }
        settings.visible = !report.total
        graph.requestPaint()
    }
    function startAnalysis() {
        clearOwnMove()
        endPreview()
        selectedCandidate = -1
        startError = ""
        if (positionOnly) selectedTurn = controller.move_number
        const ok = positionOnly
            ? controller.analysePositionForReview(visits.value, komiInput.text, rules.currentText, executable, network, configuration)
            : controller.analyseWholeGame(visits.value, komiInput.text, rules.currentText, executable, network, configuration)
        if (!ok) startError = controller.whole_game_status
        contextRevision++
    }
    function openSavedAnalysis() {
        clearOwnMove()
        endPreview()
        selectedCandidate = -1
        deeperRequested = false
        startError = ""
        if (!controller.openLatestSavedAnalysis(positionOnly)) {
            startError = controller.error_message
            return
        }
        contextRevision++
        visits.value = report.visits
        komiInput.text = String(report.komi)
        rules.currentIndex = Math.max(0, rules.find(report.rules))
        settings.visible = false
        if (positionOnly) selectedTurn = controller.move_number
        else selectTurn(Math.min(controller.move_number, results.length - 1))
    }
    function analysePosition(budget, positionKomi) {
        visits.value = budget
        komiInput.text = positionKomi
        rules.currentIndex = 0
        startAnalysis()
    }
    function countBand(player, low, high) {
        return (report.decisions || []).filter(function(d) {
            const loss = Math.max(0, d.estimated_loss)
            return d.player === player && loss >= low && loss < high
        }).length
    }
    onResultsChanged: graph.requestPaint()
    onSelectedTurnChanged: graph.requestPaint()
    onShowWinRateChanged: graph.requestPaint()
    Connections {
        target: panel.controller
        function onWhole_game_reportChanged() {
            Qt.callLater(function() {
                if (panel.pendingWhyVertex.length && panel.pendingWhyTurn === panel.selectedTurn && panel.matchesLoaded) {
                    const index = panel.candidates.findIndex(function(c) { return JSON.stringify(c.vertex) === panel.pendingWhyVertex })
                    if (index >= 0) { panel.pendingWhyVertex = ""; panel.chooseCandidate(index); return }
                }
                if (panel.previewActive && panel.chosenCandidate && panel.matchesLoaded)
                    panel.previewCandidate(Math.min(panel.previewStep, panel.continuationLength))
            })
        }
        function onMove_numberChanged() {
            panel.clearOwnMove()
            panel.endPreview()
            if (panel.matchesLoaded) {
                panel.selectedTurn = panel.controller.move_number
                panel.selectedCandidate = -1
            }
        }
    }
    contentItem: ColumnLayout {
        spacing: 8
        RowLayout {
            Layout.fillWidth: true
            Label { text: panel.positionOnly ? qsTr("Review position %1").arg(panel.selectedTurn) : qsTr("Review game"); font.bold: true; Layout.fillWidth: true }
            Button { text: panel.returnText; onClicked: panel.leaveReview(); ToolTip.visible: hovered; ToolTip.text: qsTr("Return to the previous database or Study view") }
        }
        Label { Layout.fillWidth: true; wrapMode: Text.Wrap; text: (panel.gameTitle || qsTr("Current game")) + (panel.controller ? qsTr(" · %1 moves").arg(panel.controller.move_count) : "") }
        ScrollView {
            id: scroller
            Layout.fillWidth: true
            Layout.fillHeight: true
            clip: true
            contentWidth: availableWidth
            ColumnLayout {
                width: scroller.availableWidth
                spacing: 8
                RowLayout {
                    Layout.fillWidth: true
                    Button { text: qsTr("Analysis settings"); checkable: true; checked: settings.visible; onClicked: settings.visible = !settings.visible }
                    Label { Layout.fillWidth: true; wrapMode: Text.Wrap; text: panel.report.rules ? panel.report.rules + " · komi " + panel.report.komi + " · " + panel.report.visits + qsTr(" visits") : qsTr("Choose rules and komi before starting") }
                }
                GridLayout {
                    id: settings
                    visible: !panel.report.total
                    Layout.fillWidth: true
                    columns: 2
                    enabled: !panel.controller || !panel.controller.whole_game_busy
                    Label { text: qsTr("Visits / position") }
                    SpinBox { id: visits; from: 10; to: 100000; value: panel.visitBudget; editable: true; Layout.fillWidth: true }
                    Label { text: qsTr("Analysis komi") }
                    TextField { id: komiInput; text: panel.komi; Layout.fillWidth: true }
                    Label { text: qsTr("Analysis rules") }
                    ComboBox { id: rules; model: ["japanese", "chinese", "aga", "tromp-taylor"]; Layout.fillWidth: true }
                }
                RowLayout {
                    Button {
                        text: qsTr("Open latest saved")
                        enabled: panel.controller && !panel.controller.whole_game_busy
                                 && !panel.controller.katago_analysis_in_progress
                        onClicked: panel.openSavedAnalysis()
                        ToolTip.visible: hovered
                        ToolTip.text: qsTr("Open the newest saved analysis for this game and review mode without running KataGo.")
                    }
                    Button {
                        text: panel.positionOnly ? qsTr("Analyse position") : panel.report.complete ? qsTr("Load / reanalyse") : panel.results.length ? qsTr("Resume analysis") : qsTr("Start analysis")
                        enabled: panel.controller && !panel.controller.whole_game_busy && !panel.controller.katago_analysis_in_progress
                        onClicked: panel.startAnalysis()
                    }
                    Button { text: panel.controller && panel.controller.candidate_analysis_busy ? qsTr("Stop") : qsTr("Pause"); enabled: panel.controller && panel.controller.whole_game_busy; onClicked: panel.controller.pauseWholeGame() }
                }
                Label { Layout.fillWidth: true; wrapMode: Text.Wrap; text: panel.startError || (panel.matchesLoaded ? panel.controller.whole_game_status
                    : panel.controller && panel.controller.whole_game_busy
                        ? qsTr("Another game is being analysed. Pause it before starting this game.")
                        : panel.positionOnly ? qsTr("Ready to analyse this position. Matching saved results will be reused.") : qsTr("Ready to analyse this game. Matching saved results will be reused.")) }
                ProgressBar { Layout.fillWidth: true; from: 0; to: panel.report.total || 1; value: panel.report.completed || 0; indeterminate: panel.controller && panel.controller.whole_game_busy && (panel.positionOnly || !panel.report.total) }
                RowLayout {
                    Layout.fillWidth: true
                    Label { Layout.fillWidth: true; text: panel.currentResult ? (panel.showWinRate ? qsTr("Black %1%").arg((panel.currentResult.win_rate * 100).toFixed(1)) : panel.scoreName(panel.currentResult.score_lead)) : qsTr("No saved estimate") }
                    Button { text: qsTr("Score"); checkable: true; checked: !panel.showWinRate; onClicked: panel.showWinRate = false }
                    Button { text: qsTr("Win %"); checkable: true; checked: panel.showWinRate; onClicked: panel.showWinRate = true }
                }
                Canvas {
                    id: graph
                    visible: !panel.positionOnly
                    Layout.fillWidth: true
                    Layout.preferredHeight: 160
                    onWidthChanged: requestPaint()
                    onHeightChanged: requestPaint()
                    property real low: -10
                    property real high: 10
                    onPaint: {
                        const ctx = getContext("2d")
                        ctx.reset()
                        if (panel.positionOnly) return
                        ctx.fillStyle = panel.palette.base; ctx.fillRect(0, 0, width, height)
                        low = panel.showWinRate ? 0 : -10; high = panel.showWinRate ? 100 : 10
                        if (!panel.showWinRate) for (const r of panel.results) { low = Math.min(low, r.score_lead - 1); high = Math.max(high, r.score_lead + 1) }
                        const count = Math.max(1, (panel.report.total || 1) - 1)
                        function x(turn) { return 42 + turn / count * Math.max(1, graph.width - 52) }
                        function y(result) { const v = panel.showWinRate ? result.win_rate * 100 : result.score_lead; return 12 + (high - v) / (high - low) * (graph.height - 40) }
                        ctx.font = "11px sans-serif"; ctx.fillStyle = panel.palette.text
                        ctx.fillText(high.toFixed(0), 3, 14); ctx.fillText(low.toFixed(0), 3, height - 28)
                        ctx.fillText("0", 40, height - 5); ctx.fillText(String(count), width - 30, height - 5)
                        const zeroY = 12 + (high - (panel.showWinRate ? 50 : 0)) / (high - low) * (height - 40)
                        ctx.strokeStyle = panel.palette.mid; ctx.beginPath(); ctx.moveTo(42, zeroY); ctx.lineTo(width - 10, zeroY); ctx.stroke()
                        ctx.strokeStyle = panel.palette.highlight; ctx.lineWidth = 2; ctx.beginPath()
                        panel.results.forEach(function(r, i) { if (i === 0) ctx.moveTo(x(r.turn_number), y(r)); else ctx.lineTo(x(r.turn_number), y(r)) }); ctx.stroke()
                        ctx.fillStyle = panel.palette.text
                        panel.decisions.slice(0, 8).forEach(function(d) { const r = panel.results[d.move]; if (r) { ctx.beginPath(); ctx.arc(x(d.move), y(r), 3, 0, Math.PI * 2); ctx.fill() } })
                        ctx.strokeStyle = panel.palette.text; ctx.lineWidth = 1; ctx.beginPath(); ctx.moveTo(x(panel.selectedTurn), 8); ctx.lineTo(x(panel.selectedTurn), height - 25); ctx.stroke()
                    }
                    MouseArea {
                        anchors.fill: parent
                        onClicked: function(mouse) {
                            if (panel.results.length) panel.selectTurn(Math.round((mouse.x - 42) / Math.max(1, width - 52) * Math.max(1, (panel.report.total || 1) - 1)))
                        }
                    }
                }
                RowLayout {
                    visible: !panel.positionOnly
                    Layout.fillWidth: true
                    Label { text: panel.showWinRate ? qsTr("Black win probability (%)") : qsTr("Score: Black positive (points)"); Layout.fillWidth: true; wrapMode: Text.Wrap }
                    Label { text: qsTr("Move number") }
                }
                RowLayout {
                    visible: !panel.positionOnly
                    Button { Layout.preferredWidth: 40; Layout.minimumWidth: 32; text: "|‹"; Accessible.name: qsTr("First position"); enabled: panel.results.length > 0; onClicked: panel.selectTurn(0) }
                    Button { Layout.preferredWidth: 40; Layout.minimumWidth: 32; text: "‹"; Accessible.name: qsTr("Previous position"); enabled: panel.selectedTurn > 0; onClicked: panel.selectTurn(panel.selectedTurn - 1) }
                    Label { text: qsTr("After %1").arg(panel.selectedTurn); Layout.fillWidth: true; horizontalAlignment: Text.AlignHCenter }
                    Button { Layout.preferredWidth: 40; Layout.minimumWidth: 32; text: "›"; Accessible.name: qsTr("Next position"); enabled: panel.selectedTurn + 1 < panel.results.length; onClicked: panel.selectTurn(panel.selectedTurn + 1) }
                    Button { Layout.preferredWidth: 40; Layout.minimumWidth: 32; text: "›|"; Accessible.name: qsTr("Last saved position"); enabled: panel.results.length > 0; onClicked: panel.selectTurn(panel.results.length - 1) }
                }
                CheckBox { text: qsTr("Show AI suggestions on board"); checked: panel.showSuggestions; onToggled: panel.showSuggestions = checked }
                TabBar {
                    id: tabs
                    visible: !panel.positionOnly
                    Layout.fillWidth: true
                    TabButton { text: qsTr("Decisions") }
                    TabButton { text: qsTr("Candidate moves") }
                    TabButton { text: qsTr("Summary") }
                }
                ColumnLayout {
                    visible: tabs.currentIndex === 0
                    Layout.fillWidth: true
                    RowLayout {
                        Label { text: qsTr("Estimated loss ≥1 point"); Layout.fillWidth: true; wrapMode: Text.Wrap }
                        ComboBox { id: colourFilter; model: [qsTr("Both"), qsTr("Black"), qsTr("White")]; onCurrentIndexChanged: graph.requestPaint() }
                    }
                    Repeater {
                        model: panel.decisions.slice(0, 20)
                        delegate: ItemDelegate {
                            required property var modelData
                            Layout.fillWidth: true
                            highlighted: panel.selectedTurn === modelData.move - 1
                            text: qsTr("%1 · %2 · %3 points").arg(modelData.move).arg(modelData.player).arg(modelData.estimated_loss.toFixed(1))
                            onClicked: panel.selectTurn(modelData.move - 1)
                        }
                    }
                    Label { visible: panel.decisions.length === 0; text: qsTr("No saved decisions above this threshold."); wrapMode: Text.Wrap; Layout.fillWidth: true }
                }
                ColumnLayout {
                    visible: tabs.currentIndex === 1
                    Layout.fillWidth: true
                    Flow {
                        Layout.fillWidth: true
                        spacing: 4
                        Button {
                            text: panel.choosingOwnMove ? qsTr("Cancel choosing") : qsTr("Why not this?")
                            enabled: panel.currentResult !== null && panel.matchesLoaded && panel.controller && !panel.controller.whole_game_busy
                            onClicked: { panel.endPreview(); panel.choosingOwnMove = !panel.choosingOwnMove; panel.startError = "" }
                        }
                        Button {
                            text: qsTr("Try pass")
                            visible: panel.choosingOwnMove
                            enabled: panel.controller && !panel.controller.whole_game_busy
                            onClicked: panel.investigatePoint(-1, -1)
                        }
                    }
                    Label {
                        visible: panel.choosingOwnMove
                        Layout.fillWidth: true
                        wrapMode: Text.Wrap
                        text: qsTr("Click your move on the board. KataGo will investigate the replies using the visit budget below.")
                    }
                    Label { text: qsTr("Teal: KataGo recommendation · Blue: alternatives · Purple: your ideas. Click a marker to explore."); wrapMode: Text.Wrap; Layout.fillWidth: true }
                    Repeater {
                        model: panel.candidates
                        delegate: ItemDelegate {
                            required property var modelData
                            required property int index
                            Layout.fillWidth: true
                            highlighted: panel.selectedCandidate === index
                            contentItem: ColumnLayout {
                                Label { text: panel.candidateLabel(index) + " · " + panel.vertexName(modelData.vertex) + " · " + panel.scoreName(modelData.score_lead) + (modelData.recommended ? qsTr(" · Best move (KataGo recommendation)") : modelData.user_move ? qsTr(" · Your idea") : qsTr(" · Alternative")); Layout.fillWidth: true; wrapMode: Text.Wrap }
                                Label { text: qsTr("Black win estimate %1% · %2 visits").arg((modelData.win_rate * 100).toFixed(1)).arg(modelData.visits) + (modelData.focused ? qsTr(" · deeper search") : ""); Layout.fillWidth: true; wrapMode: Text.Wrap; opacity: 0.8 }
                            }
                            onClicked: panel.chooseCandidate(index)
                        }
                    }
                    Flow {
                        Layout.fillWidth: true
                        spacing: 4
                        Button { text: qsTr("Return to position"); enabled: panel.previewActive; onClicked: panel.endPreview() }
                        Button {
                            text: qsTr("Save move to study")
                            enabled: panel.chosenCandidate !== null && panel.matchesLoaded
                                     && panel.controller && !panel.controller.whole_game_busy
                            onClicked: panel.saveCandidate(false)
                        }
                    }
                    Flow {
                        Layout.fillWidth: true
                        spacing: 4
                        Label { text: qsTr("Deeper search visits"); height: deeperVisits.implicitHeight; verticalAlignment: Text.AlignVCenter }
                        SpinBox {
                            id: deeperVisits
                            from: 10; to: 100000; value: 1000; stepSize: 100
                            editable: true
                            enabled: !panel.controller || !panel.controller.whole_game_busy
                            Accessible.name: qsTr("Visits for chosen candidate")
                        }
                        Button {
                            text: qsTr("Analyse this candidate more deeply")
                            enabled: panel.chosenCandidate !== null && panel.matchesLoaded
                                     && panel.controller && !panel.controller.whole_game_busy
                                     && deeperVisits.value > panel.chosenCandidate.visits
                            onClicked: {
                                panel.deeperRequested = true
                                panel.startError = ""
                                if (!panel.controller.deepenWholeGameCandidate(panel.selectedTurn,
                                        panel.chosenCandidate.sourceIndex, deeperVisits.value))
                                    panel.startError = panel.controller.error_message
                            }
                        }
                        Button {
                            text: qsTr("Stop")
                            visible: panel.controller && panel.controller.candidate_analysis_busy
                            onClicked: panel.controller.pauseWholeGame()
                        }
                    }
                    Label {
                        Layout.fillWidth: true
                        wrapMode: Text.Wrap
                        visible: panel.deeperRequested
                        text: panel.startError || (panel.controller ? panel.controller.whole_game_status : "")
                    }
                    Label {
                        Layout.fillWidth: true
                        wrapMode: Text.Wrap
                        visible: panel.previewActive
                        text: panel.showEngineLine
                            ? qsTr("Move %1 of %2%3 · source position unchanged")
                                .arg(panel.previewStep).arg(panel.continuationLength)
                                .arg(panel.previewStep > 0 && panel.chosenCandidate
                                     && panel.chosenCandidate.principal_variation[panel.previewStep - 1] === "Pass"
                                     ? qsTr(" (pass)") : "")
                            : qsTr("Trying candidate %1 · source position unchanged").arg(panel.chosenCandidate ? panel.vertexName(panel.chosenCandidate.vertex) : "")
                    }
                    CheckBox {
                        text: qsTr("Show engine continuation")
                        checked: panel.showEngineLine
                        enabled: panel.chosenCandidate !== null && panel.matchesLoaded
                        onToggled: {
                            panel.showEngineLine = checked
                            panel.previewCandidate(checked ? panel.continuationLength : 1)
                        }
                    }
                    ColumnLayout {
                        visible: panel.showEngineLine && panel.chosenCandidate !== null
                        Layout.fillWidth: true
                        Label {
                            Layout.fillWidth: true
                            wrapMode: Text.Wrap
                            text: qsTr("One illustrative engine line, not a sequence you are expected to follow.")
                        }
                        Slider {
                            Layout.fillWidth: true
                            from: 0
                            to: panel.continuationLength
                            stepSize: 1
                            snapMode: Slider.SnapAlways
                            value: panel.previewStep
                            enabled: panel.matchesLoaded
                            Accessible.name: qsTr("Engine continuation move")
                            onMoved: panel.previewCandidate(Math.round(value))
                        }
                        Flow {
                            Layout.fillWidth: true
                            spacing: 4
                            Button {
                                text: qsTr("Previous")
                                enabled: panel.previewStep > 0
                                onClicked: panel.previewCandidate(panel.previewStep - 1)
                            }
                            Button {
                                text: qsTr("Next")
                                enabled: panel.chosenCandidate !== null && panel.matchesLoaded
                                         && panel.previewStep < panel.continuationLength
                                onClicked: panel.previewCandidate(panel.previewStep + 1)
                            }
                            Button {
                                text: qsTr("Save engine line to study")
                                enabled: panel.chosenCandidate !== null && panel.matchesLoaded
                                         && panel.controller && !panel.controller.whole_game_busy
                                onClicked: panel.saveCandidate(true)
                            }
                        }
                    }
                    Label {
                        Layout.fillWidth: true
                        wrapMode: Text.Wrap
                        visible: panel.controller && panel.controller.whole_game_busy
                        text: qsTr("Pause analysis before saving a continuation to a study.")
                    }
                }
                GridLayout {
                    visible: tabs.currentIndex === 2
                    columns: 3
                    Layout.fillWidth: true
                    Label { text: qsTr("Estimated loss"); Layout.fillWidth: true }
                    Label { text: qsTr("Black"); Layout.preferredWidth: 50; horizontalAlignment: Text.AlignRight }
                    Label { text: qsTr("White"); Layout.preferredWidth: 50; horizontalAlignment: Text.AlignRight }
                    Repeater {
                        model: ["Under 1 pt", "1–3 pts", "3–6 pts", "6+ pts"]
                        delegate: RowLayout {
                            required property string modelData
                            required property int index
                            Layout.columnSpan: 3
                            Layout.fillWidth: true
                            Label { text: modelData; Layout.fillWidth: true }
                            Label { text: panel.countBand("Black", [0,1,3,6][index], [1,3,6,Infinity][index]); Layout.preferredWidth: 50; horizontalAlignment: Text.AlignRight }
                            Label { text: panel.countBand("White", [0,1,3,6][index], [1,3,6,Infinity][index]); Layout.preferredWidth: 50; horizontalAlignment: Text.AlignRight }
                        }
                    }
                }
                Label { text: qsTr("These are estimates to investigate, especially at low visits."); wrapMode: Text.Wrap; Layout.fillWidth: true; opacity: 0.8 }
                Label { visible: panel.currentResult !== null; text: panel.currentResult ? qsTr("Position %1 · %2 actual visits").arg(panel.selectedTurn).arg(panel.currentResult.visits) : ""; wrapMode: Text.Wrap; Layout.fillWidth: true }
                Label { visible: text.length > 0; text: panel.currentResult ? (panel.currentResult.warnings || []).join("\n") : ""; wrapMode: Text.Wrap; Layout.fillWidth: true }
                Label { id: navigationNotice; Layout.fillWidth: true; wrapMode: Text.Wrap; text: "" }
            }
        }
    }
}
