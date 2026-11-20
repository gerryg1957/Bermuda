import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import org.bermuda.app

Dialog {
    id: dialog
    property string projectPath: ""
    property var games: []
    property var archive: ({groups: [], snapshots: []})
    property var selectedIds: []
    property int selectedGroupId: -1
    property string browserError: ""
    property bool creatingGroup: false
    property bool buildingWeekly: false
    readonly property var weeklyPlan: parse(archiveModel.weekly_preview_json, {})
    function todayText() { return Qt.formatDate(new Date(), "yyyy-MM-dd") }
    function previewWeekly() { archiveModel.previewWeekly(projectPath, todayText(), resetWeekly.checked) }
    readonly property var savedGroups: archive.groups || []
    readonly property var selectedGroup: savedGroups.find(function(g) { return g.id === selectedGroupId }) || null
    readonly property var snapshots: (archive.snapshots || []).filter(function(s) { return s.group_id === selectedGroupId })
    function validDate(value) {
        if (!value.length) return true
        if (!/^\d{4}-\d{2}-\d{2}$/.test(value)) return false
        const parsed = new Date(value + "T00:00:00Z")
        return !isNaN(parsed.getTime()) && parsed.toISOString().slice(0, 10) === value
    }
    readonly property bool dateRangeValid: validDate(fromDate.text) && validDate(toDate.text)
        && (!fromDate.text.length || !toDate.text.length || fromDate.text <= toDate.text)
    readonly property var visibleGames: games.filter(function(g) {
        if (!dateRangeValid) return false
        const date = String(g.date || "")
        // A date range includes only unambiguous full calendar dates. Undated
        // and partial-date records remain selectable with the filter cleared.
        if ((fromDate.text.length || toDate.text.length) && !/^\d{4}-\d{2}-\d{2}$/.test(date)) return false
        return (!fromDate.text.length || date >= fromDate.text) && (!toDate.text.length || date <= toDate.text)
    })
    function parse(text, fallback) { try { return JSON.parse(text) } catch (e) { return fallback } }
    function dateText(seconds) { return new Date(seconds * 1000).toLocaleString() }
    function chooseGame(id, checked) {
        let ids = selectedIds.slice()
        if (checked && ids.indexOf(id) < 0) ids.push(id)
        if (!checked) ids = ids.filter(function(v) { return v !== id })
        selectedIds = ids
    }
    function openForProject(path) {
        projectPath = path
        browserError = ""
        open()
        archiveModel.refresh(path)
    }
    parent: Overlay.overlay
    anchors.centerIn: parent
    width: Math.min(1050, parent.width - 32)
    height: Math.min(780, parent.height - 32)
    title: qsTr("My games · Reports")
    modal: true
    closePolicy: Popup.NoAutoClose
    standardButtons: Dialog.Close
    PersonalSnapshotModel {
        id: archiveModel
        onGames_jsonChanged: dialog.games = dialog.parse(games_json, [])
        onArchive_jsonChanged: {
            dialog.archive = dialog.parse(archive_json, {groups: [], snapshots: []})
            if (!dialog.selectedGroup && dialog.savedGroups.length) dialog.selectedGroupId = dialog.savedGroups[0].id
            if (!dialog.savedGroups.length) dialog.selectedGroupId = -1
        }
        onBusyChanged: {
            if (!busy && dialog.buildingWeekly) {
                dialog.buildingWeekly = false
                if (!error_message.length) tabs.currentIndex = 0
            }
            if (!busy && dialog.creatingGroup) {
                dialog.creatingGroup = false
                if (!error_message.length) tabs.currentIndex = 1
            }
        }
        onReport_urlChanged: {
            if (report_url.toString().length && !Qt.openUrlExternally(report_url))
                dialog.browserError = qsTr("The browser could not open the report: %1").arg(report_url)
        }
    }
    Dialog {
        id: removal
        property int targetId: -1
        property bool group: false
        property string targetName: ""
        parent: Overlay.overlay
        anchors.centerIn: parent
        modal: true
        title: group ? qsTr("Remove game group?") : qsTr("Remove saved report?")
        standardButtons: Dialog.Ok | Dialog.Cancel
        Label { text: qsTr("Remove “%1” from the library?\nYour games and saved analysis will be kept.").arg(removal.targetName); wrapMode: Text.Wrap }
        onAccepted: {
            if (group) archiveModel.removeGroup(dialog.projectPath, targetId)
            else archiveModel.removeReport(dialog.projectPath, targetId)
        }
    }
    Dialog {
        id: weeklyConfirmation
        parent: Overlay.overlay
        anchors.centerIn: parent
        width: Math.min(560, parent.width - 48)
        modal: true
        title: qsTr("Replace the report library?")
        standardButtons: Dialog.Ok | Dialog.Cancel
        contentItem: Label {
            wrapMode: Text.Wrap
            text: qsTr("Replace %1 reports and %2 game groups with %3 weekly reports starting %4?\n\nYour games and saved KataGo analyses will be kept. No new analysis will run.")
                .arg(dialog.weeklyPlan.replace_reports || 0).arg(dialog.weeklyPlan.replace_groups || 0)
                .arg((dialog.weeklyPlan.periods || []).length).arg(dialog.weeklyPlan.anchor || "")
        }
        onAccepted: {
            if (dialog.weeklyPlan.today !== dialog.todayText()) { dialog.previewWeekly(); return }
            dialog.buildingWeekly = true
            archiveModel.buildWeekly(dialog.projectPath, JSON.stringify(dialog.weeklyPlan))
        }
    }
    function confirmRemoval(id, name, group) {
        removal.targetId = id; removal.targetName = name; removal.group = group; removal.open()
    }
    contentItem: ColumnLayout {
        spacing: 10
        Label {
            text: qsTr("Create weekly reports from saved analysis, or choose your own groups. Saved reports stay unchanged until you replace or remove them.")
            wrapMode: Text.Wrap; Layout.fillWidth: true
        }
        TabBar {
            id: tabs; objectName: "snapshotTabs"
            Layout.fillWidth: true
            TabButton { text: qsTr("Reports library") }
            TabButton { text: qsTr("New report") }
            TabButton { text: qsTr("Choose games") }
            TabButton { text: qsTr("Trends") }
            TabButton { text: qsTr("Saved comparisons") }
            TabButton { text: qsTr("Weekly reports") }
        }
        Label {
            text: dialog.browserError || archiveModel.error_message || archiveModel.notice
            visible: text.length > 0
            color: (dialog.browserError || archiveModel.error_message) ? "#a42d30" : dialog.palette.text
            wrapMode: Text.Wrap; Layout.fillWidth: true
        }
        BusyIndicator { visible: archiveModel.busy; running: visible; Layout.alignment: Qt.AlignHCenter }
        StackLayout {
            currentIndex: tabs.currentIndex
            Layout.fillWidth: true; Layout.fillHeight: true
            enabled: !archiveModel.busy
            ColumnLayout {
                RowLayout {
                    Button { text: qsTr("Create weekly reports…"); onClicked: tabs.currentIndex = 5 }
                    Button { text: qsTr("New report…"); onClicked: tabs.currentIndex = dialog.savedGroups.length ? 1 : 2 }
                    Button { text: qsTr("New group of games…"); onClicked: tabs.currentIndex = 2 }
                    Button { text: qsTr("Refresh list"); onClicked: archiveModel.refresh(dialog.projectPath) }
                }
                Label { visible: !(dialog.archive.snapshots || []).length; text: qsTr("No saved reports yet. Choose games, then save their analysis as a report."); Layout.fillWidth: true; wrapMode: Text.Wrap }
                ListView {
                    Layout.fillWidth: true; Layout.fillHeight: true; clip: true
                    model: dialog.archive.snapshots || []
                    ScrollBar.vertical: ScrollBar {}
                    delegate: Frame {
                        required property var modelData
                        width: ListView.view.width
                        contentItem: ColumnLayout {
                        Label { Layout.fillWidth: true; wrapMode: Text.Wrap; text: modelData.group_name + " — " + modelData.label + " (#" + modelData.id + ")\n"
                            + dialog.dateText(modelData.created_at) + " · "
                            + qsTr("%1 complete · %2 partial · %3 awaiting · %4 excluded")
                                .arg(modelData.report.complete).arg(modelData.report.partial)
                                .arg((modelData.report.pending || []).length).arg(modelData.report.excluded || 0)
                        }
                        RowLayout {
                            Button { text: qsTr("Open report…"); onClicked: archiveModel.openReport(dialog.projectPath, modelData.id) }
                            Button { text: qsTr("Remove report…"); onClicked: dialog.confirmRemoval(modelData.id, modelData.label, false) }
                        }
                        }
                    }
                }
                Label { text: qsTr("Open a report to see its charts and board diagrams in your browser, where you can print or save as PDF."); Layout.fillWidth: true; wrapMode: Text.Wrap }
            }
            ColumnLayout {
                RowLayout {
                    Layout.fillWidth: true
                    Label { text: qsTr("Game group") }
                    ComboBox {
                        id: groupChoice
                        Layout.fillWidth: true
                        textRole: "name"; model: dialog.savedGroups
                        currentIndex: dialog.savedGroups.findIndex(function(g) { return g.id === dialog.selectedGroupId })
                        onActivated: dialog.selectedGroupId = dialog.savedGroups[currentIndex].id
                    }
                    Button { text: qsTr("Refresh list"); onClicked: archiveModel.refresh(dialog.projectPath) }
                }
                RowLayout {
                    Label { text: qsTr("%1 saved reports in this group").arg(dialog.snapshots.length) }
                    Button {
                        text: qsTr("Remove group…")
                        enabled: !!dialog.selectedGroup && dialog.snapshots.length === 0
                        onClicked: dialog.confirmRemoval(dialog.selectedGroupId, dialog.selectedGroup.name, true)
                    }
                    Label { visible: dialog.snapshots.length > 0; text: qsTr("Remove its reports first to remove this group."); wrapMode: Text.Wrap; Layout.fillWidth: true }
                }
                Label {
                    visible: !dialog.selectedGroup
                    text: qsTr("Start with Choose games. Save a named group, then preserve its analysis here.")
                    wrapMode: Text.Wrap; Layout.fillWidth: true
                }
                Label {
                    visible: !!dialog.selectedGroup
                    text: dialog.selectedGroup ? qsTr("%1 games · created %2 · %3 with an identified colour")
                        .arg(dialog.selectedGroup.members.length).arg(dialog.dateText(dialog.selectedGroup.created_at))
                        .arg(dialog.selectedGroup.members.filter(function(m) { return !!m.colour }).length) : ""
                    wrapMode: Text.Wrap; Layout.fillWidth: true
                }
                Label {
                    text: qsTr("These are the games you kept together. Adding more games to My games will not change this group.")
                    wrapMode: Text.Wrap; Layout.fillWidth: true
                }
                Label { text: qsTr("Save a new report for this group"); font.bold: true }
                Label {
                    text: qsTr("This saves a baseline of what KataGo currently says. It does not run analysis or measure improvement. If analysis changes later, you can save another snapshot.")
                    wrapMode: Text.Wrap; Layout.fillWidth: true
                }
                RowLayout {
                    Layout.fillWidth: true
                    TextField { id: snapshotName; Layout.fillWidth: true; maximumLength: 120; placeholderText: qsTr("Report name, e.g. First review or CPU baseline") }
                    Button {
                        text: qsTr("Save new report")
                        enabled: !!dialog.selectedGroup && snapshotName.text.trim().length > 0
                        onClicked: archiveModel.saveSnapshot(dialog.projectPath, dialog.selectedGroupId, snapshotName.text)
                    }
                }
                Label {
                    text: qsTr("Any unfinished or missing analysis will be labelled. Existing snapshots stay unchanged.")
                    wrapMode: Text.Wrap; Layout.fillWidth: true
                }
                ScrollView {
                    Layout.fillWidth: true; Layout.fillHeight: true
                    clip: true; contentWidth: availableWidth
                    ColumnLayout {
                        width: parent.width
                        Label { text: qsTr("Earlier reports for this group"); font.bold: true }
                        Label { visible: dialog.snapshots.length === 0; text: qsTr("No snapshot saved yet. Give the current analysis a name above and click Save new report.") }
                        Repeater {
                            model: dialog.snapshots
                            delegate: Frame {
                                required property var modelData
                                Layout.fillWidth: true
                                contentItem: ColumnLayout {
                                    Label { text: modelData.label + " · " + dialog.dateText(modelData.created_at); font.bold: true; wrapMode: Text.Wrap; Layout.fillWidth: true }
                                    Label {
                                        text: qsTr("%1 complete · %2 partial · %3 awaiting · %4 excluded")
                                            .arg(modelData.report.complete).arg(modelData.report.partial)
                                            .arg((modelData.report.pending || []).length).arg(modelData.report.excluded || 0)
                                        wrapMode: Text.Wrap; Layout.fillWidth: true
                                    }
                                    RowLayout {
                                        Button { text: qsTr("View report / Print…"); onClicked: { dialog.browserError = ""; archiveModel.openReport(dialog.projectPath, modelData.id) } }
                                        Button { text: qsTr("Remove report…"); onClicked: dialog.confirmRemoval(modelData.id, modelData.label, false) }
                                    }
                                }
                            }
                        }
                        Label { text: qsTr("Games in this group"); font.bold: true; Layout.topMargin: 12 }
                        Repeater {
                            model: dialog.selectedGroup ? dialog.selectedGroup.members : []
                            delegate: Label {
                                required property var modelData
                                text: modelData.date + " · " + modelData.black + " — " + modelData.white
                                    + " · " + qsTr("You: %1").arg(modelData.colour || modelData.identity_status)
                                wrapMode: Text.Wrap; Layout.fillWidth: true
                            }
                        }
                    }
                }
            }
            ColumnLayout {
                Label { text: qsTr("1. Choose games to keep together"); font.bold: true }
                Label {
                    text: qsTr("For example, keep your October games together so you can later compare them with a different period. There is no required group size.")
                    wrapMode: Text.Wrap; Layout.fillWidth: true
                }
                RowLayout {
                    Layout.fillWidth: true
                    Label { text: qsTr("Most recent games") }
                    SpinBox { id: groupSize; from: 1; to: Math.max(1, dialog.visibleGames.length); value: Math.min(50, to); editable: true }
                    Button { text: qsTr("Select this many"); enabled: dialog.dateRangeValid; onClicked: dialog.selectedIds = dialog.visibleGames.slice(0, groupSize.value).map(function(g) { return g.game_id }) }
                }
                TextField { id: groupName; Layout.fillWidth: true; maximumLength: 120; placeholderText: qsTr("Group name, e.g. October 2026") }
                RowLayout {
                    Layout.fillWidth: true
                    Label { text: qsTr("From") }
                    TextField { id: fromDate; objectName: "fromDate"; Layout.fillWidth: true; placeholderText: qsTr("YYYY-MM-DD (optional)"); maximumLength: 10 }
                    Label { text: qsTr("To") }
                    TextField { id: toDate; objectName: "toDate"; Layout.fillWidth: true; placeholderText: qsTr("YYYY-MM-DD (optional)"); maximumLength: 10 }
                    Button { text: qsTr("Clear dates"); onClicked: { fromDate.clear(); toDate.clear() } }
                }
                Label { visible: !dialog.dateRangeValid; text: qsTr("Enter valid YYYY-MM-DD dates, with From no later than To."); color: "#a42d30"; wrapMode: Text.Wrap; Layout.fillWidth: true }
                Label { text: qsTr("Dates filter the list; they do not select games automatically. Partial or unknown dates are shown when the date filter is cleared."); wrapMode: Text.Wrap; Layout.fillWidth: true }
                Flow {
                    Layout.fillWidth: true; spacing: 8
                    Button { text: qsTr("Use shown games (%1)").arg(dialog.visibleGames.length); enabled: dialog.dateRangeValid; onClicked: dialog.selectedIds = dialog.visibleGames.map(function(g) { return g.game_id }) }
                    Button { text: qsTr("Clear selection"); onClicked: dialog.selectedIds = [] }
                    Label { text: qsTr("%1 selected, including any hidden by the date filter").arg(dialog.selectedIds.length) }
                }
                ListView {
                    Layout.fillWidth: true; Layout.fillHeight: true
                    clip: true; model: dialog.visibleGames
                    ScrollBar.vertical: ScrollBar {}
                    delegate: CheckDelegate {
                        required property var modelData
                        width: ListView.view.width
                        text: (modelData.date || qsTr("Undated")) + " · " + modelData.black + " — " + modelData.white
                            + " · " + qsTr("You: %1").arg(modelData.colour || modelData.identity_status)
                        checked: dialog.selectedIds.indexOf(modelData.game_id) >= 0
                        onToggled: dialog.chooseGame(modelData.game_id, checked)
                    }
                }
                Label {
                    text: qsTr("Names and your playing colour will be kept as shown. Check your name mappings first; unidentified games are excluded from personal statistics.")
                    wrapMode: Text.Wrap; Layout.fillWidth: true
                }
                Button {
                    text: qsTr("Keep these %1 games together →").arg(dialog.selectedIds.length)
                    enabled: groupName.text.trim().length > 0 && dialog.selectedIds.length > 0 && dialog.dateRangeValid
                    onClicked: {
                        // Select the newly created group when the refreshed list arrives.
                        dialog.selectedGroupId = -1
                        dialog.creatingGroup = true
                        archiveModel.createGroup(dialog.projectPath, groupName.text, JSON.stringify(dialog.selectedIds))
                    }
                }
            }
            LiveTrends {
                report: dialog.parse(archiveModel.live_json, {})
                errorMessage: archiveModel.live_error
                weeklyAnchor: dialog.archive.weekly_anchor || ""
                onRefreshRequested: archiveModel.refresh(dialog.projectPath)
            }
            ReportTrends {
                snapshots: dialog.archive.snapshots || []
                onOpenReport: function(snapshotId) { archiveModel.openReport(dialog.projectPath, snapshotId) }
            }
            ColumnLayout {
                Label { text: qsTr("Seven days at a time"); font.bold: true }
                Label {
                    Layout.fillWidth: true; wrapMode: Text.Wrap
                    text: qsTr("Start with your first dated game, then keep consecutive seven-day periods through today. Empty weeks are included; the current period may be unfinished.")
                }
                Label {
                    Layout.fillWidth: true; wrapMode: Text.Wrap
                    text: qsTr("This replaces all reports and groups in the library using saved analysis. Your games and KataGo analyses are kept. Exported HTML or PDF files are not deleted.")
                }
                CheckBox {
                    id: resetWeekly
                    visible: !!dialog.archive.weekly_anchor
                    text: qsTr("Reset the starting date to my earliest game")
                    checked: false
                }
                RowLayout {
                    Button { text: qsTr("Preview weekly reports"); onClicked: dialog.previewWeekly() }
                    Label { text: dialog.archive.weekly_anchor ? qsTr("Saved starting date: %1").arg(dialog.archive.weekly_anchor) : ""; Layout.fillWidth: true; wrapMode: Text.Wrap }
                }
                Label {
                    Layout.fillWidth: true; wrapMode: Text.Wrap
                    visible: !!dialog.weeklyPlan.periods
                    text: qsTr("%1 weekly reports · replaces %2 reports and %3 groups\nOmitted: %4 without a complete date · %5 before the starting date · %6 future-dated")
                        .arg((dialog.weeklyPlan.periods || []).length).arg(dialog.weeklyPlan.replace_reports || 0)
                        .arg(dialog.weeklyPlan.replace_groups || 0).arg(dialog.weeklyPlan.undated || 0)
                        .arg(dialog.weeklyPlan.before_anchor || 0).arg(dialog.weeklyPlan.future || 0)
                }
                ListView {
                    Layout.fillWidth: true; Layout.fillHeight: true; clip: true
                    model: dialog.weeklyPlan.periods || []
                    ScrollBar.vertical: ScrollBar {}
                    delegate: ItemDelegate {
                        required property var modelData
                        width: ListView.view.width
                        text: modelData.from + " — " + modelData.to + " · " + qsTr("%1 games").arg(modelData.games)
                            + (modelData.unfinished ? qsTr(" · unfinished period") : "")
                            + (modelData.excluded ? qsTr(" · %1 unidentified").arg(modelData.excluded) : "")
                    }
                }
                Button {
                    text: qsTr("Replace library with weekly reports…")
                    enabled: !!dialog.weeklyPlan.periods && dialog.weeklyPlan.reset_anchor === resetWeekly.checked
                    onClicked: weeklyConfirmation.open()
                }
            }
        }
    }
}
