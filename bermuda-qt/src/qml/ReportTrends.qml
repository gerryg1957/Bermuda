import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import "SnapshotTrends.js" as Trends

ColumnLayout {
    id: view
    property var snapshots: []
    property var selected: []
    signal openReport(int snapshotId)
    onSnapshotsChanged: {
        selected = selected.filter(function(id) { return snapshots.some(function(s) { return s.id === id }) })
        let seen = {}
        // Default to the newest saved evidence for each fixed group, once.
        if (!selected.length) selected = snapshots.filter(function(s) {
            if (seen[s.group_id]) return false
            seen[s.group_id] = true
            return true
        }).map(function(s) { return s.id })
    }
    readonly property var individualSets: Trends.settings(snapshots, selected)
    readonly property var sets: individualSets.concat(individualSets.length ? [{key:"*",label:qsTr("All")}] : [])
    readonly property bool allAnalysis: !!sets[setChoice.currentIndex] && sets[setChoice.currentIndex].key === "*"
    readonly property var points: Trends.points(snapshots, selected,
        sets[setChoice.currentIndex] ? sets[setChoice.currentIndex].key : "", Math.max(0, Math.min(metric.currentIndex, 3)))
    readonly property bool overlapping: Trends.overlaps(points)
    readonly property bool allCategories: metric.currentIndex === 4
    readonly property var categoryColours: ["#32877d", "#6b93ae", "#b58a45", "#ab6262"]
    readonly property var categoryNames: [qsTr("Below 1 point"), qsTr("1–3 points"), qsTr("3–5 points"), qsTr("5+ points")]
    readonly property var series: (allCategories ? [0,1,2,3] : [Math.max(0, Math.min(metric.currentIndex, 3))]).map(function(bucket) {
        return {bucket: bucket, points: Trends.points(snapshots, selected, sets[setChoice.currentIndex] ? sets[setChoice.currentIndex].key : "", bucket)}
    })
    onSeriesChanged: graph.requestPaint()
    function percentages(index) {
        return series.map(function(s) {
            const p = s.points[index]
            return categoryNames[s.bucket] + ": " + (!p || p.percent === null ? qsTr("No assessed moves") : p.percent.toFixed(1) + "%")
        }).join(" · ")
    }
    onPointsChanged: graph.requestPaint()
    Label { text: qsTr("Estimated losses per move over time"); font.bold: true }
    Label {
        text: qsTr("Compare the same bar-chart category across fixed groups. Dates are when the games were played, not when analysis was saved. Choose one report per group below.")
        Layout.fillWidth: true; wrapMode: Text.Wrap
    }
    RowLayout {
        Layout.fillWidth: true
        ComboBox {
            id: metric; objectName: "savedMetric"; Layout.fillWidth: true
            model: [qsTr("Below 1 point — higher is generally better"), qsTr("1–3 points"), qsTr("3–5 points"), qsTr("5+ points — lower is generally better"), qsTr("All categories")]
        }
        ComboBox { id: setChoice; objectName: "savedAnalysis"; Layout.fillWidth: true; model: view.sets; textRole: "label" }
    }
    Label {
        text: view.allAnalysis ? qsTr("Percentages are weighted by assessed moves; saved reports stay unchanged.") : qsTr("Engine, network, configuration, visits, rules and board size must match. Weekly reports combine komi values; older custom reports retain their grouping. Percentages are per assessed personal move, not an average of game percentages. Opponent strength, colour and handicap can also affect the result; this is not a rank estimate.")
        Layout.fillWidth: true; wrapMode: Text.Wrap
    }
    Label {
        visible: view.points.filter(function(p) { return p.percent !== null }).length < 2 || view.overlapping
        text: view.overlapping ? qsTr("These groups share games. Points are shown without a connecting line; they are not independent periods.")
            : qsTr("Choose at least two dated groups with assessed moves under matching settings to see a trend. One report provides a baseline, not a trend.")
        Layout.fillWidth: true; wrapMode: Text.Wrap; color: "#805600"
    }
    Flow {
        Layout.fillWidth: true; spacing: 18
        Repeater {
            model: view.series
            delegate: Row {
                required property var modelData
                spacing: 6
                Canvas {
                    width: 30; height: 18
                    onPaint: {
                        const ctx = getContext("2d")
                        ctx.clearRect(0,0,width,height)
                        ctx.strokeStyle = view.categoryColours[modelData.bucket]; ctx.lineWidth = 2
                        ctx.setLineDash([[],[7,3],[2,3],[8,3,2,3]][modelData.bucket])
                        ctx.beginPath(); ctx.moveTo(0,9); ctx.lineTo(30,9); ctx.stroke()
                    }
                }
                Label { text: view.categoryNames[modelData.bucket] }
            }
        }
    }
    Canvas {
        id: graph
        Layout.fillWidth: true; Layout.preferredHeight: 210; Layout.minimumHeight: 160
        onWidthChanged: requestPaint()
        onHeightChanged: requestPaint()
        onVisibleChanged: if (visible) requestPaint()
        onPaint: {
            const ctx = getContext("2d"), p = view.points
            ctx.clearRect(0, 0, width, height)
            ctx.fillStyle = "white"; ctx.fillRect(0, 0, width, height)
            const left = 48, right = width - 30, top = 18, bottom = height - 35
            ctx.font = "12px sans-serif"
            for (let value = 0; value <= 100; value += 25) {
                const y = bottom - value / 100 * (bottom - top)
                ctx.strokeStyle = "#dce1e5"; ctx.beginPath(); ctx.moveTo(left,y); ctx.lineTo(right,y); ctx.stroke()
                ctx.fillStyle = "#333333"; ctx.fillText(value + "%", 5, y + 4)
            }
            if (!p.length) return
            const start = p[0].time, end = p[p.length-1].time
            function x(point) { return end === start ? (left+right)/2 : left + (point.time-start)/(end-start)*(right-left) }
            function y(point) { return bottom-point.percent/100*(bottom-top) }
            Trends.ticks(start,end,right-left,p.every(function(point) { return point.weekly })).forEach(function(tick) {
                const position = x({time:tick.time})
                ctx.strokeStyle = "#dce1e5"; ctx.beginPath(); ctx.moveTo(position,top); ctx.lineTo(position,bottom); ctx.stroke()
                ctx.fillStyle = "#333333"
                ctx.fillText(tick.label,Math.max(left,Math.min(right-75,position-35)),height-10)
            })
            view.series.forEach(function(series) {
                const colour = view.categoryColours[series.bucket]
                ctx.strokeStyle = colour; ctx.lineWidth = 2
                ctx.setLineDash([[],[7,3],[2,3],[8,3,2,3]][series.bucket])
                ctx.beginPath()
                series.points.forEach(function(point,i) {
                    if (point.percent === null) return
                    if (!view.overlapping && point.connect) ctx.lineTo(x(point),y(point))
                    else ctx.moveTo(x(point),y(point))
                })
                ctx.stroke(); ctx.setLineDash([])
                series.points.forEach(function(point,i) {
                    if (point.percent === null) return
                    ctx.fillStyle = point.unfinished ? "white" : colour
                    ctx.strokeStyle = colour
                    ctx.beginPath(); ctx.arc(x(point),y(point),point.unfinished ? 5 : 3.5,0,Math.PI*2)
                    ctx.fill(); if (point.unfinished) ctx.stroke()
                    if (!view.allCategories) {
                        ctx.fillStyle = "#222222"; ctx.fillText(String(i+1),x(point)+7,y(point)-6)
                    }
                })
            })

        }
    }
    Label {
        text: qsTr("Weekly points use the period start; custom reports use the last game date. Gaps mean no matching assessed moves. Hollow circles mark periods unfinished when saved. Partial analysis contributes only assessed moves.")
        Layout.fillWidth: true; wrapMode: Text.Wrap
    }
    ScrollView {
        Layout.fillWidth: true; Layout.fillHeight: true; contentWidth: availableWidth; clip: true
        ColumnLayout {
            width: parent.width
            Repeater {
                model: view.points
                delegate: Frame {
                    required property var modelData
                    required property int index
                    Layout.fillWidth: true
                    contentItem: ColumnLayout {
                        Label { text: (index+1) + ". " + modelData.name + " · " + modelData.label + " · " + (view.allCategories ? qsTr("All categories") : (modelData.percent === null ? qsTr("No assessed moves") : modelData.percent.toFixed(1) + "%")); font.bold: true; wrapMode: Text.Wrap; Layout.fillWidth: true }
                        Label { visible: view.allCategories; text: view.percentages(index); wrapMode: Text.Wrap; Layout.fillWidth: true }
                        Label {
                            text: modelData.from + " – " + modelData.to + " · " + qsTr("%1 contributing games (%2 complete), %3 assessed moves · %4 awaiting analysis, %5 excluded in report")
                                .arg(modelData.games).arg(modelData.complete).arg(modelData.moves).arg(modelData.pending).arg(modelData.excluded)
                            wrapMode: Text.Wrap; Layout.fillWidth: true
                        }
                        Label { text: qsTr("%1 games in period · %2 contributing games partially analysed").arg(modelData.total).arg(modelData.partial)
                            + (modelData.unfinished ? qsTr(" · unfinished when saved") : ""); Layout.fillWidth: true; wrapMode: Text.Wrap }
                        Button { text: qsTr("Open this report…"); onClicked: view.openReport(modelData.id) }
                    }
                }
            }
            Label { text: qsTr("Reports to compare (one per fixed group)"); font.bold: true }
            Repeater {
                model: view.snapshots
                delegate: CheckDelegate {
                    required property var modelData
                    Layout.fillWidth: true
                    text: modelData.group_name + " — " + modelData.label + " (#" + modelData.id + ")"
                    checked: view.selected.indexOf(modelData.id) >= 0
                    onToggled: view.selected = Trends.choose(view.snapshots, view.selected, modelData.id, checked)
                }
            }
        }
    }
}
