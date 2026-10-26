import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Frame {
    id: panel
    padding: 16
    property var studies: []
    property string currentTitle: ""
    property bool canContinue: false
    property string sortKey: "modifiedMillis"
    property bool sortAscending: false
    readonly property var columns: [
        {key: "blackPlayer", title: qsTr("Black"), width: 135},
        {key: "whitePlayer", title: qsTr("White"), width: 135},
        {key: "date", title: qsTr("Date"), width: 100},
        {key: "title", title: qsTr("Title"), width: 230},
        {key: "result", title: qsTr("Result"), width: 65},
        {key: "event", title: qsTr("Event"), width: 180},
        {key: "modifiedMillis", title: qsTr("Saved"), width: 155},
        {key: "annotationCount", title: qsTr("Notes"), width: 65},
        {key: "origin", title: qsTr("Source"), width: 230}
    ]
    readonly property real tableWidth: columns.reduce(function(total, c) { return total + c.width }, 90)
    readonly property var sortedStudies: studies.map(function(entry, index) {
        return {entry: entry, sourceIndex: index}
    }).sort(function(a, b) {
        const av = a.entry[panel.sortKey] || ""
        const bv = b.entry[panel.sortKey] || ""
        const order = panel.sortKey === "modifiedMillis" || panel.sortKey === "annotationCount"
            ? Number(av) - Number(bv) : String(av).localeCompare(String(bv))
        return panel.sortAscending ? order : -order
    })
    signal openFile()
    signal openCollection(int index)
    signal newPosition()
    signal exploreJoseki()
    signal continueWork()
    signal openStudy(int index)
    signal deleteStudy(int index)
    signal refreshStudies()
    function focusStudies() { studyList.forceActiveFocus() }
    function displayValue(entry, key) {
        if (key === "modifiedMillis") return entry.modifiedMillis > 0
            ? Qt.formatDateTime(new Date(Number(entry.modifiedMillis)), "yyyy-MM-dd hh:mm") : "—"
        if (key === "blackPlayer" && !entry.blackPlayer && !entry.whitePlayer)
            return entry.title || "—"
        const value = entry[key]
        return value === undefined || value === null || value === "" ? "—" : String(value)
    }
    background: Rectangle { color: panel.palette.window; border.color: panel.palette.mid }
    contentItem: ColumnLayout {
        spacing: 8
        Label { text: qsTr("Study"); font.bold: true; font.pixelSize: 20 }
        Flow {
            Layout.fillWidth: true
            spacing: 8
            Button {
                text: qsTr("Open…")
                onClicked: openMenu.open()
                Menu {
                    id: openMenu
                    MenuItem { text: qsTr("Professional games"); onTriggered: panel.openCollection(0) }
                    MenuItem { text: qsTr("My games"); onTriggered: panel.openCollection(1) }
                    MenuItem { text: qsTr("Saved studies"); onTriggered: { panel.refreshStudies(); panel.focusStudies() } }
                    MenuSeparator {}
                    MenuItem { text: qsTr("SGF file…"); onTriggered: panel.openFile() }
                }
            }
            Button { text: qsTr("New pattern…"); onClicked: panel.newPosition() }
            Button { text: qsTr("Explore joseki"); onClicked: panel.exploreJoseki() }
        }
        Label { text: qsTr("Current work"); font.bold: true; Layout.topMargin: 8 }
        RowLayout {
            Layout.fillWidth: true
            Label { text: panel.canContinue ? panel.currentTitle : qsTr("Open a game or create a position to begin."); Layout.fillWidth: true; wrapMode: Text.Wrap }
            Button { text: qsTr("Continue"); enabled: panel.canContinue; onClicked: panel.continueWork() }
        }
        RowLayout {
            Layout.fillWidth: true
            Label { text: qsTr("Saved studies · %1").arg(Number(panel.studies.length).toLocaleString(Qt.locale(), "f", 0)); font.bold: true; Layout.fillWidth: true }
            Button { text: qsTr("Refresh"); onClicked: panel.refreshStudies() }
        }
        Rectangle {
            Layout.fillWidth: true
            implicitHeight: 36
            color: panel.palette.alternateBase
            clip: true
            Row {
                x: 90 - studyList.contentX
                Repeater {
                    model: panel.columns
                    delegate: ToolButton {
                        required property var modelData
                        width: modelData.width
                        height: 36
                        text: modelData.title + (panel.sortKey === modelData.key ? (panel.sortAscending ? " ↑" : " ↓") : " ↕")
                        onClicked: {
                            panel.sortAscending = panel.sortKey === modelData.key ? !panel.sortAscending : true
                            panel.sortKey = modelData.key
                        }
                    }
                }
            }
            Rectangle {
                width: 90; height: parent.height
                color: panel.palette.alternateBase
                Label { anchors.centerIn: parent; text: qsTr("Actions") }
            }
        }
        Frame {
            Layout.fillWidth: true
            Layout.fillHeight: true
            Layout.minimumHeight: 120
            padding: 0
            background: Rectangle { color: panel.palette.base }
            ListView {
                id: studyList
                objectName: "savedStudiesTable"
                anchors.fill: parent
                clip: true
                model: panel.sortedStudies
                contentWidth: Math.max(width, panel.tableWidth)
                flickableDirection: Flickable.AutoFlickDirection
                ScrollBar.vertical: ScrollBar {}
                ScrollBar.horizontal: ScrollBar { policy: studyList.contentWidth > studyList.width ? ScrollBar.AlwaysOn : ScrollBar.AlwaysOff }
                Keys.onReturnPressed: if (currentIndex >= 0) panel.openStudy(panel.sortedStudies[currentIndex].sourceIndex)
                delegate: ItemDelegate {
                    id: studyRow
                    required property var modelData
                    required property int index
                    width: studyList.contentWidth
                    height: 38
                    padding: 0
                    highlighted: studyList.currentIndex === index
                    onClicked: { studyList.currentIndex = index; panel.openStudy(modelData.sourceIndex) }
                    contentItem: Item {
                        Row {
                            x: 90
                        Repeater {
                            model: panel.columns
                            delegate: Label {
                                required property var modelData
                                width: modelData.width
                                height: 38
                                leftPadding: 8
                                rightPadding: 8
                                verticalAlignment: Text.AlignVCenter
                                elide: Text.ElideRight
                                text: panel.displayValue(studyRow.modelData.entry, modelData.key)
                                HoverHandler { id: cellHover }
                                ToolTip.visible: cellHover.hovered
                                ToolTip.text: text
                            }
                        }
                        }
                        Rectangle {
                            x: studyList.contentX
                            width: 90; height: 38
                            color: panel.palette.base
                            Button { anchors.fill: parent; text: qsTr("Delete…"); onClicked: panel.deleteStudy(studyRow.modelData.sourceIndex) }
                        }
                    }
                }
                Label { anchors.centerIn: parent; visible: panel.studies.length === 0; text: qsTr("No saved studies yet.") }
            }
        }
    }
}
