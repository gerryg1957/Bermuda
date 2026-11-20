import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import "LiveTrends.js" as Live
import "SnapshotTrends.js" as Axis

ColumnLayout {
    id: view
    property var report: ({})
    property string errorMessage: ""
    property string weeklyAnchor: ""
    onWeeklyAnchorChanged: { if (weeklyAnchor.length) grouping.currentIndex = 1 }
    property string today: Qt.formatDate(new Date(), "yyyy-MM-dd")
    signal refreshRequested()
    readonly property var sets: Live.settings(report, from, to).map(function(g,i,all) {
        const sameLabel = all.filter(function(other) { return other.visits === g.visits && other.rules === g.rules && other.board_size === g.board_size }).length > 1
        return Object.assign({},g,{label:qsTr("%1 visits · %2 · %3×%3 · %4 contributing games")
            .arg(g.visits).arg(g.rules).arg(g.board_size || 19).arg(g.games)
            + (sameLabel ? qsTr(" · engine/configuration %1").arg(i+1) : "")})
    }).concat(Live.allOption(report,from,to).map(function(g) {
        return Object.assign({},g,{label:qsTr("All")})
    }))
    property string selectedAnalysisKey: ""
    readonly property var selectedAnalysis: sets.find(function(s) { return s.key === selectedAnalysisKey }) || sets[0] || null
    readonly property var selectedKeys: selectedAnalysis ? selectedAnalysis.keys : []
    readonly property string groupingMode: ["each","week","month","games"][Math.max(0,grouping.currentIndex)]
    readonly property bool gameOrder: groupingMode === "each" && axisChoice.currentIndex === 1
    onGameOrderChanged: graph.requestPaint()
    readonly property string from: period.currentIndex === 5 ? fromDate.text : period.currentIndex === 4 ? "" : Live.startForMonths(today,[1,3,6,12][period.currentIndex])
    readonly property string to: period.currentIndex === 5 ? toDate.text : period.currentIndex === 4 ? "" : today
    readonly property var prepared: Live.prepare(report, selectedKeys,
        Math.max(0, Math.min(metric.currentIndex, 3)), from, to, groupingMode, blockSize.value, weeklyAnchor, today)
    readonly property var points: prepared.points
    readonly property bool allCategories: metric.currentIndex === 4
    readonly property var categoryColours: ["#32877d", "#6b93ae", "#b58a45", "#ab6262"]
    readonly property var categoryNames: [qsTr("Below 1 point"), qsTr("1–3 points"), qsTr("3–5 points"), qsTr("5+ points")]
    readonly property var series: (allCategories ? [0,1,2,3] : [Math.max(0, Math.min(metric.currentIndex, 3))]).map(function(bucket) {
        return {bucket: bucket, points: Live.prepare(report, selectedKeys, bucket, from, to, groupingMode, blockSize.value, weeklyAnchor, today).points}
    })
    onSeriesChanged: graph.requestPaint()
    function percentages(index) {
        return series.map(function(s) {
            const p = s.points[index]
            return categoryNames[s.bucket] + ": " + (!p || p.percent === null ? qsTr("No assessed moves") : p.percent.toFixed(1) + "%")
        }).join(" · ")
    }
    onPointsChanged: graph.requestPaint()
    Label {
        visible: view.weeklyAnchor.length > 0 && view.groupingMode === "week"
        Layout.fillWidth: true; wrapMode: Text.Wrap
        text: qsTr("Seven-day periods starting %1, matching weekly reports. This graph uses current saved analysis; reports preserve the analysis available when created.").arg(view.weeklyAnchor)
    }
    Label { text: qsTr("My progress · saved game analysis"); font.bold: true }
    RowLayout {
        Layout.fillWidth: true
        ComboBox { id: period; model: [qsTr("1 month"),qsTr("3 months"),qsTr("6 months"),qsTr("1 year"),qsTr("All"),qsTr("Custom")]; currentIndex: 4 }
        ComboBox { id: grouping; objectName: "liveGrouping"; currentIndex: view.weeklyAnchor.length ? 1 : 0; model: [qsTr("Each game"),qsTr("Weekly"),qsTr("Monthly"),qsTr("Blocks of games")] }
        ComboBox { id: axisChoice; objectName: "liveAxis"; visible: grouping.currentIndex === 0; model: [qsTr("Game dates"),qsTr("Game order")] }
        SpinBox { id: blockSize; visible: grouping.currentIndex === 3; from:1; to:10000; value:20; editable:true }
        Item { Layout.fillWidth: true }
        Button { text: qsTr("Refresh analysis"); onClicked: view.refreshRequested() }
    }
    RowLayout {
        visible: period.currentIndex === 5
        Label { text: qsTr("From") }
        TextField { id: fromDate; placeholderText: "YYYY-MM-DD"; maximumLength:10; Layout.fillWidth:true }
        Label { text: qsTr("To") }
        TextField { id: toDate; placeholderText: "YYYY-MM-DD"; maximumLength:10; Layout.fillWidth:true }
    }
    RowLayout {
        Layout.fillWidth:true
        ComboBox { id: metric; objectName: "liveMetric"; Layout.fillWidth:true; model:[qsTr("Below 1 point — higher is generally better"),qsTr("1–3 points"),qsTr("3–5 points"),qsTr("5+ points — lower is generally better"), qsTr("All categories")] }
        ComboBox {
            id:setChoice; objectName: "liveAnalysis"; Layout.fillWidth:true; model:view.sets; textRole:"label"
            currentIndex: view.selectedAnalysis ? view.sets.findIndex(function(s) { return s.key === view.selectedAnalysis.key }) : -1
            onActivated: view.selectedAnalysisKey = view.sets[currentIndex].key
        }
    }
    Label {
        text: view.selectedAnalysisKey === "*" ? qsTr("Uses the latest saved analysis for each game; no new KataGo run is needed.") : qsTr("Komi values are combined. Analysis depth, engine, network, configuration, rules and board size stay separate. Uses the latest saved analysis for each game; no new KataGo run is needed.")
        Layout.fillWidth:true; wrapMode:Text.Wrap
    }
    Label {
        text: view.errorMessage || view.prepared.error || (view.report.needs_names ? qsTr("Set up your names before viewing personal trends.") : view.points.length === 0 ? qsTr("No games with assessed personal moves for this analysis in the selected period.") : view.points.filter(function(p) { return p.percent !== null }).length < 2 ? qsTr("Only one point has assessed moves. Choose a wider period or a smaller grouping to see a trend.") : "")
        visible:text.length>0; Layout.fillWidth:true; wrapMode:Text.Wrap; color:"#805600"
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
            function x(point) {
                if (view.gameOrder) return p.length === 1 ? (left+right)/2 : left + point.order/(p.length-1)*(right-left)
                return end === start ? (left+right)/2 : left + (point.time-start)/(end-start)*(right-left)
            }
            function y(point) { return bottom-point.percent/100*(bottom-top) }
            if (view.gameOrder) {
                ctx.fillStyle = "#333333"
                ctx.fillText(qsTr("Game 1"),left,height-10)
                if (p.length>1) ctx.fillText(qsTr("Game %1").arg(p.length),right-75,height-10)
            } else {
            Axis.ticks(start,end,right-left,view.groupingMode === "week").forEach(function(tick) {
                const position = x({time:tick.time})
                ctx.strokeStyle = "#dce1e5"; ctx.beginPath(); ctx.moveTo(position,top); ctx.lineTo(position,bottom); ctx.stroke()
                ctx.fillStyle = "#333333"
                ctx.fillText(tick.label,Math.max(left,Math.min(right-75,position-35)),height-10)
            })
            }
            view.series.forEach(function(series) {
                const colour = view.categoryColours[series.bucket]
                ctx.strokeStyle = colour; ctx.lineWidth = 2
                ctx.setLineDash([[],[7,3],[2,3],[8,3,2,3]][series.bucket])
                ctx.beginPath()
                series.points.forEach(function(point,i) {
                    if (point.percent === null) return
                    if (point.connect) ctx.lineTo(x(point),y(point))
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
        text: qsTr("%1 contributing / %2 games in range · %3 awaiting analysis · %4 with other analysis settings · %5 without assessed personal moves · %6 undated omitted")
            .arg(view.prepared.contributing || 0).arg(view.prepared.games || 0).arg(view.prepared.pending || 0)
            .arg(view.prepared.other || 0).arg(view.prepared.unassessed || 0).arg(view.prepared.undated || 0)
        Layout.fillWidth:true; wrapMode:Text.Wrap
    }
    Label {
        text: (view.gameOrder ? qsTr("Games are equally spaced in date order; games on the same date use a stable order, not their unknown playing times.")
            : view.groupingMode === "each" ? qsTr("Points use game dates. Games on the same date can overlap; choose Game order to spread them out.")
            : qsTr("Weekly/monthly points mark the period start. Hollow circles mark the unfinished week; gaps have no assessed moves. Blocks end on their last game date."))
            + " " + qsTr("Percentages use assessed personal moves, including partial games. Opponents, colour and handicap can affect the trend.")
        Layout.fillWidth:true; wrapMode:Text.Wrap
    }
    ScrollView {
        Layout.fillWidth:true; Layout.fillHeight:true; contentWidth:availableWidth; clip:true
        ColumnLayout {
            width: parent.width
            Repeater {
                model: view.points
                delegate: Frame {
                    required property var modelData
                    required property int index
                    Layout.fillWidth:true
                    contentItem: ColumnLayout {
                        Label { text: (index+1)+". "+modelData.name+(modelData.unfinished ? qsTr(" · unfinished week") : "")+" · "+(view.allCategories ? qsTr("All categories") : modelData.percent===null ? qsTr("No assessed moves") : modelData.percent.toFixed(1)+"%"); font.bold:true; wrapMode:Text.Wrap; Layout.fillWidth:true }
                        Label { visible: view.allCategories; text: view.percentages(index); wrapMode: Text.Wrap; Layout.fillWidth: true }
                        Label {
                            text: modelData.from+" – "+modelData.to+" · "+qsTr("%1 games · %2 complete, %3 partial · %4 assessed moves · %5 awaiting analysis, %6 with other settings")
                                .arg(modelData.games.length).arg(modelData.complete).arg(modelData.partial).arg(modelData.moves).arg(modelData.pending).arg(modelData.other)
                            wrapMode:Text.Wrap; Layout.fillWidth:true
                        }
                        Button { text: checked ? qsTr("Hide games") : qsTr("Show games"); checkable:true; onToggled: gameNames.visible=checked }
                        Label {
                            id:gameNames; visible:false; wrapMode:Text.Wrap; Layout.fillWidth:true
                            text: modelData.games.map(function(g) {return g.date+" · "+g.title+" · "+(g.colour || "")}).join("\n")
                        }
                    }
                }
            }
        }
    }
}
