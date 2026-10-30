import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import org.bermuda.app

Frame {
    id: panel
    padding: 16
    property string projectPath: ""
    property string executable: ""
    property string network: ""
    property string configuration: ""
    property int visitBudget: 200
    readonly property bool batchBusy: reportModel.batch_busy
    property bool pauseRequested: false
    function requestPause() {
        if (!batchBusy || pauseRequested) return
        pauseRequested = true
        reportModel.pauseBatch()
    }
    function syncBatchDialog() {
        if (panel.visible && panel.batchBusy) batchProgress.open()
        else batchProgress.close()
    }
    onVisibleChanged: Qt.callLater(syncBatchDialog)
    onBatchBusyChanged: {
        if (!batchBusy) pauseRequested = false
        Qt.callLater(syncBatchDialog)
    }
    property var queue: {
        try { return JSON.parse(reportModel.queue_json || "{}") }
        catch (error) { return ({}) }
    }
    signal batchRequested(bool resume)
    function startBatch(resume) {
        pauseRequested = false
        reportModel.startBatch(projectPath, resume, batchVisits.value, batchRules.currentText,
            batchKomi.text, executable, network, configuration)
    }
    property string openError: ""
    property var report: {
        try { return JSON.parse(reportModel.report_json || "{}") }
        catch (error) { return ({}) }
    }
    readonly property var groups: report.groups || []
    readonly property var group: groups[settingsChoice.currentIndex] || ({})
    function contextText(d) {
        return qsTr("Your win estimate %1% → %2%")
            .arg((d.win_before * 100).toFixed(1)).arg((d.win_after * 100).toFixed(1))
            + (d.already_unlikely ? qsTr(" · Already ≤5% before this move") : "")
    }
    property string reviewTitle: ""
    property int reviewMove: 0
    signal continueReview()
    signal reportReady()
    function defaultGame(previousId, previousMove) {
        const analysed = groups.reduce(function(all, g) { return all.concat(g.analysed || []) }, [])
        const previous = analysed.find(function(g) { return g.game_id === previousId })
        if (previous) return Object.assign({}, previous, {move: previousMove + 1})
        const game = analysed.slice().sort(function(a, b) {
            return String(b.date || "").localeCompare(String(a.date || "")) || b.game_id - a.game_id
        })[0]
        if (!game) return null
        const examples = groups.reduce(function(all, g) { return all.concat(g.win_examples || []) }, [])
            .filter(function(d) { return d.game_id === game.game_id })
            .sort(function(a, b) { return b.win_loss - a.win_loss })
        return Object.assign({}, game, {move: examples.length ? examples[0].move : 1})
    }
    signal closeReport()
    signal manageNames()
    signal openDecision(var decision)
    signal openGame(var game)
    function refresh() { openError = ""; reportModel.refresh(projectPath) }
    background: Rectangle { color: panel.palette.base; border.color: panel.palette.mid }
    PersonalReportModel {
        id: reportModel
        onBatch_busyChanged: if (!batch_busy) panel.refresh()
        onLoadingChanged: if (!loading && !error_message.length) Qt.callLater(function() { panel.reportReady() })
    }
    Dialog {
        id: batchProgress
        parent: Overlay.overlay
        anchors.centerIn: parent
        width: Math.min(650, parent.width - 40)
        title: qsTr("Analysing My games")
        modal: true
        closePolicy: Popup.NoAutoClose
        contentItem: ColumnLayout {
            spacing: 12
            Label { text: panel.pauseRequested ? qsTr("Pausing — waiting for the worker to stop. Saved positions will be kept.") : reportModel.batch_status; wrapMode: Text.Wrap; Layout.fillWidth: true }
            Label { text: qsTr("%1 of %2 games complete").arg(reportModel.completed_games).arg(reportModel.total_games) }
            ProgressBar { Layout.fillWidth: true; from: 0; to: Math.max(1, reportModel.total_games); value: reportModel.completed_games; indeterminate: reportModel.total_games === 0 }
            Label { text: qsTr("Current game") }
            ProgressBar { Layout.fillWidth: true; from: 0; to: Math.max(1, reportModel.total_positions); value: reportModel.completed_positions; indeterminate: reportModel.total_positions === 0 }
            Button { text: panel.pauseRequested ? qsTr("Pausing…") : qsTr("Pause batch"); enabled: !panel.pauseRequested; onClicked: panel.requestPause() }
            Label { text: qsTr("Saved positions are kept. You can resume here after restarting Bermuda."); wrapMode: Text.Wrap; Layout.fillWidth: true }
        }
    }
    contentItem: ColumnLayout {
        spacing: 10
        RowLayout {
            Layout.fillWidth: true
            Label { text: qsTr("My games · Report card"); font.pixelSize: 22; font.bold: true; Layout.fillWidth: true; wrapMode: Text.Wrap }
            Button { text: qsTr("Back to Home"); onClicked: panel.closeReport() }
        }
        Label { text: qsTr("Summary across your most recent 20 identified games. Each decision below identifies its game; click it to open the position."); wrapMode: Text.Wrap; Layout.fillWidth: true }
        Frame {
            visible: panel.reviewTitle.length > 0
            Layout.fillWidth: true
            contentItem: ColumnLayout {
                Label {
                    text: qsTr("Board shown: %1 · before move %2").arg(panel.reviewTitle).arg(panel.reviewMove + 1)
                    font.bold: true; wrapMode: Text.Wrap; Layout.fillWidth: true
                }
                Button { text: qsTr("Continue reviewing"); onClicked: panel.continueReview() }
            }
        }
        ColumnLayout {
            Layout.fillWidth: true
            spacing: 8
            Label { text: qsTr("Analysis"); font.bold: true }
            Flow {
                Layout.fillWidth: true
                spacing: 8
                Button {
                    text: panel.queue.remaining > 0 ? qsTr("Resume batch (%1 games left)").arg(panel.queue.remaining) : qsTr("Analyse pending games")
                    visible: !panel.batchBusy
                    enabled: !reportModel.loading && (panel.queue.remaining > 0 || (panel.report.pending || []).length + (panel.report.partial || 0) > 0)
                    onClicked: panel.batchRequested(panel.queue.remaining > 0)
                }
                Button {
                    text: panel.pauseRequested ? qsTr("Pausing…") : qsTr("Pause batch")
                    visible: panel.batchBusy
                    enabled: !panel.pauseRequested
                    onClicked: panel.requestPause()
                }
                Button {
                    text: qsTr("Refresh report")
                    enabled: !reportModel.loading && !panel.batchBusy
                    onClicked: panel.refresh()
                    ToolTip.visible: hovered
                    ToolTip.text: qsTr("Update the report from saved results without starting analysis.")
                }
                Button {
                    text: qsTr("Batch settings")
                    visible: !(panel.queue.remaining > 0)
                    enabled: !reportModel.loading && !panel.batchBusy
                    checkable: true; checked: batchSettings.visible
                    onClicked: batchSettings.visible = !batchSettings.visible
                }
                Button { text: qsTr("Show batch progress"); visible: panel.batchBusy; onClicked: batchProgress.open() }
            }
            Label {
                Layout.fillWidth: true; wrapMode: Text.Wrap
                visible: text.length > 0
                text: panel.pauseRequested ? qsTr("Pausing — saved positions will be kept.") : reportModel.batch_status
            }
            ProgressBar {
                visible: panel.batchBusy
                Layout.fillWidth: true
                from: 0; to: Math.max(1, reportModel.total_positions)
                value: reportModel.completed_positions
                indeterminate: reportModel.total_positions === 0
            }
            Label {
                Layout.fillWidth: true; wrapMode: Text.Wrap
                visible: panel.report.needs_names === false || panel.queue.remaining > 0
                text: panel.queue.remaining > 0
                    ? qsTr("Saved queue · %1 visits · %2 rules. Resume uses its original settings. New imports do not join this queue.").arg(panel.queue.visits).arg(panel.queue.rules)
                    : qsTr("Pending and partial games in this report · %1 visits · %2 rules · recorded komi (fallback %3). Matching checkpoints are reused; other saved runs are retained.").arg(batchVisits.value).arg(batchRules.currentText).arg(batchKomi.text)
            }
            Flow {
                visible: panel.queue.remaining > 0
                Layout.fillWidth: true
                spacing: 8
                Button {
                    text: qsTr("Clear paused queue")
                    enabled: !reportModel.loading && !panel.batchBusy
                    onClicked: reportModel.clearQueue(panel.projectPath)
                }
                Label {
                    width: Math.min(implicitWidth, parent.width)
                    wrapMode: Text.Wrap
                    text: qsTr("Keeps saved analysis. Start a fresh batch afterwards to include new games.")
                }
            }
            GridLayout {
                id: batchSettings
                visible: false
                enabled: !(panel.queue.remaining > 0) && !reportModel.loading && !panel.batchBusy
                columns: 2
                Layout.fillWidth: true
                Label { text: qsTr("Visits / position") }
                SpinBox { id: batchVisits; from: 10; to: 100000; value: panel.visitBudget; editable: true }
                Label { text: qsTr("Analysis rules") }
                ComboBox { id: batchRules; model: ["japanese", "chinese", "aga", "tromp-taylor"] }
                Label { text: qsTr("Komi when absent from game") }
                TextField { id: batchKomi; text: "6.5" }
            }
        }
        BusyIndicator { running: reportModel.loading; visible: running; Layout.alignment: Qt.AlignHCenter }
        Label {
            visible: text.length > 0
            text: panel.openError || reportModel.error_message
            color: "#a42d30"; wrapMode: Text.Wrap; Layout.fillWidth: true
        }
        ScrollView {
            Layout.fillWidth: true; Layout.fillHeight: true
            clip: true
            contentWidth: availableWidth
            ColumnLayout {
                width: parent.width
                spacing: 14
                visible: !reportModel.loading && reportModel.error_message.length === 0
                Label {
                    visible: panel.report.needs_names === true
                    text: qsTr("First tell Bermuda which player names are yours.")
                    wrapMode: Text.Wrap; Layout.fillWidth: true
                }
                Button { text: qsTr("Manage my names…"); onClicked: panel.manageNames() }
                ColumnLayout {
                    visible: panel.report.selected !== undefined
                    Layout.fillWidth: true
                    Label {
                        text: qsTr("%1 complete · %2 partial · %3 awaiting analysis")
                            .arg(panel.report.complete || 0).arg(panel.report.partial || 0).arg((panel.report.pending || []).length)
                        font.bold: true; wrapMode: Text.Wrap; Layout.fillWidth: true
                    }
                    ProgressBar { Layout.fillWidth: true; from: 0; to: Math.max(1, panel.report.selected || 0); value: panel.report.complete || 0 }
                    Label {
                        text: qsTr("Showing %1 of %2 identified games. %3 other games are excluded because your colour is unknown or ambiguous.")
                            .arg(panel.report.selected || 0).arg(panel.report.identified || 0)
                            .arg((panel.report.total || 0) - (panel.report.identified || 0))
                        wrapMode: Text.Wrap; Layout.fillWidth: true
                    }
                }
                Label {
                    visible: panel.groups.length === 0 && panel.report.needs_names === false
                    text: qsTr("No saved whole-game analysis in this selection yet. Use Analyse pending games, or open an individual game below.")
                    wrapMode: Text.Wrap; Layout.fillWidth: true
                }
                ComboBox {
                    id: settingsChoice
                    Layout.fillWidth: true
                    visible: panel.groups.length > 1
                    model: panel.groups.map(function(g, i) {
                        return qsTr("Analysis set %1 · %2 games · %3 visits · %4 · komi %5")
                            .arg(i+1).arg(g.games).arg(g.visits).arg(g.rules).arg(g.komi)
                    })
                }
                Label {
                    visible: panel.groups.length > 1
                    text: qsTr("Different engine or analysis settings are kept in separate sets.")
                    wrapMode: Text.Wrap; Layout.fillWidth: true
                }
                ColumnLayout {
                    visible: panel.groups.length > 0
                    Layout.fillWidth: true
                    spacing: 8
                    Label { text: qsTr("Consistency"); font.pixelSize: 18; font.bold: true }
                    Label {
                        text: qsTr("%1 of your moves assessed across %2 games · %3 visits")
                            .arg(panel.group.decisions || 0).arg(panel.group.games || 0).arg(panel.group.visits || 0)
                        wrapMode: Text.Wrap; Layout.fillWidth: true
                    }
                    Repeater {
                        model: [qsTr("Below 1 point"), qsTr("1–3 points"), qsTr("3–5 points"), qsTr("5+ points")]
                        delegate: RowLayout {
                            required property string modelData
                            required property int index
                            Layout.fillWidth: true
                            Label { text: modelData; Layout.preferredWidth: 105 }
                            Rectangle {
                                Layout.fillWidth: true; implicitHeight: 18; color: "#edf0f2"; radius: 3
                                Rectangle {
                                    height: parent.height; radius: 3; color: ["#32877d", "#6b93ae", "#b58a45", "#ab6262"][index]
                                    width: parent.width * ((panel.group.buckets || [0,0,0,0])[index] / Math.max(1, panel.group.decisions || 0))
                                }
                            }
                            Label { text: (panel.group.buckets || [0,0,0,0])[index]; Layout.preferredWidth: 42; horizontalAlignment: Text.AlignRight }
                        }
                    }
                    Label {
                        text: qsTr("Estimated points lost per move. Below 1 includes estimated gains. Partial games contribute only assessed moves; this is not a playing-strength grade.")
                        wrapMode: Text.Wrap; Layout.fillWidth: true
                    }
                    Label { text: qsTr("Changes in winning chances"); font.pixelSize: 18; font.bold: true; Layout.topMargin: 10 }
                    Label {
                        Layout.fillWidth: true; wrapMode: Text.Wrap
                        text: qsTr("Largest drops of at least 5 percentage points in your estimated winning chance. These are prompts to investigate, not a verdict on where the game was lost.")
                    }
                    Label { visible: (panel.group.win_examples || []).length === 0; text: qsTr("No saved drops above this threshold.") }
                    Repeater {
                        model: panel.group.win_examples || []
                        delegate: ItemDelegate {
                            required property var modelData
                            Layout.fillWidth: true
                            contentItem: ColumnLayout {
                                Label {
                                    text: modelData.title + " · " + (modelData.date || qsTr("Undated"))
                                    font.bold: true; wrapMode: Text.Wrap; Layout.fillWidth: true
                                }
                                Label {
                                    text: qsTr("Move %1 · You: %2 · −%3 percentage points")
                                        .arg(modelData.move).arg(modelData.colour).arg(modelData.win_loss.toFixed(1))
                                    wrapMode: Text.Wrap; Layout.fillWidth: true
                                }
                                Label { text: panel.contextText(modelData); wrapMode: Text.Wrap; Layout.fillWidth: true }
                                Label { text: qsTr("Open this game at the decision →"); opacity: 0.75 }
                            }
                            onClicked: panel.openDecision(modelData)
                        }
                    }
                    Label { text: qsTr("Large point losses"); font.pixelSize: 18; font.bold: true; Layout.topMargin: 10 }
                    Label {
                        visible: (panel.group.examples || []).length === 0
                        text: qsTr("No assessed moves lost an estimated point or more in this set.")
                        wrapMode: Text.Wrap; Layout.fillWidth: true
                    }
                    Repeater {
                        model: panel.group.examples || []
                        delegate: ItemDelegate {
                            required property var modelData
                            Layout.fillWidth: true
                            contentItem: ColumnLayout {
                                Label {
                                    text: modelData.title + " · " + (modelData.date || qsTr("Undated"))
                                    font.bold: true; wrapMode: Text.Wrap; Layout.fillWidth: true
                                }
                                Label {
                                    text: qsTr("Move %1 · You: %2 · ≈%3 points")
                                        .arg(modelData.move).arg(modelData.colour).arg(modelData.loss.toFixed(1))
                                    wrapMode: Text.Wrap; Layout.fillWidth: true
                                }
                                Label { text: panel.contextText(modelData); wrapMode: Text.Wrap; Layout.fillWidth: true }
                                Label { text: qsTr("Open this game at the decision →"); opacity: 0.75 }
                            }
                            onClicked: panel.openDecision(modelData)
                        }
                    }
                }
                Label { visible: panel.groups.length > 0; text: qsTr("Analysed games"); font.pixelSize: 18; font.bold: true }
                Repeater {
                    model: panel.group.analysed || []
                    delegate: ItemDelegate {
                        required property var modelData
                        Layout.fillWidth: true
                        text: modelData.title + " · " + (modelData.complete ? qsTr("Complete")
                            : qsTr("Partial · %1/%2 positions").arg(modelData.positions).arg(modelData.total))
                        onClicked: panel.openGame(modelData)
                    }
                }
                Label { visible: (panel.report.pending || []).length > 0; text: qsTr("Awaiting analysis"); font.pixelSize: 18; font.bold: true }
                Repeater {
                    model: panel.report.pending || []
                    delegate: ItemDelegate {
                        required property var modelData
                        Layout.fillWidth: true
                        text: modelData.title + " · " + (modelData.date || qsTr("Undated"))
                        onClicked: panel.openGame(modelData)
                    }
                }
                Label {
                    text: qsTr("Bermuda organises evidence; KataGo evaluates; you investigate.")
                    wrapMode: Text.Wrap; Layout.fillWidth: true; font.italic: true
                }
            }
        }
    }
}
