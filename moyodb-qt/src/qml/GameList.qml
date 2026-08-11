import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import QtQml
import org.kde.kirigami as Kirigami
import org.moyodb.app

Kirigami.AbstractCard {
    id: root

    signal gameSelected(var game)
    signal continuationCandidateSelected(int boardX, int coreY, int count)
    signal continuationFilterCleared()

    property string projectPath: ""
    property int selectedRow: -1
    property int selectedSearchRow: -1
    property bool projectLoaded: false
    property bool searchHasRun: false
    readonly property bool searchInProgress:
        searchModel.search_in_progress
    property int searchPatternWidth: 0
    property int searchPatternHeight: 0
    property var pendingSearchGame: null
    property bool continuationFilterActive: false
    property int continuationFilterAppearances: 0
    property int selectedContinuationX: -1
    property int selectedContinuationCoreY: -1
    property var continuationCandidates: []
    property var comparisonCandidateA: null
    property var comparisonCandidateB: null

    readonly property var nextMoveDistribution: {
        const json = searchModel.next_move_distribution_json

        if (json.length === 0 || json === "{}")
            return null

        try {
            return JSON.parse(json)
        } catch (error) {
            console.warn(
                        "Could not decode next-move distribution: "
                        + error)

            return null
        }
    }

    readonly property int nextMoveLocalCount: {
        const distribution = nextMoveDistribution

        if (distribution === null
                || distribution.points === undefined) {
            return 0
        }

        let total = 0

        for (const point of distribution.points)
            total += point.count

        return total
    }

    readonly property bool searchResultsSelected:
    catalogueTabs.currentIndex === 1

    readonly property string searchErrorMessage:
        searchModel.error_message

    property string sortColumn: "date"

    property bool sortAscending: false
    readonly property bool catalogueLoading: gameModel.loading
    property string loadingIndicatorStyle: "stones"
    padding: 0



        function loadProject() {
        selectedRow = -1
        projectLoaded = false

        if (projectPath.length === 0) {
            return
        }

        if (!gameModel.loadSortedProject(
                    projectPath,
                    sortColumn,
                    sortAscending)) {
            projectLoaded = false
        }
    }

    function sortBy(column, firstAscending) {
        if (sortColumn === column) {
            sortAscending = !sortAscending
        } else {
            sortColumn = column
            sortAscending = firstAscending
        }

        loadProject()
    }

    function sortHeaderText(column, title) {
    if (sortColumn !== column)
        return title + " ↕"

    return title + (sortAscending ? " ▲" : " ▼")
}

    Connections {
        target: gameModel

        function onLoadFinished(success) {
            root.projectLoaded = success
        }
    }

    Rectangle {
        anchors.fill: parent
        visible: root.catalogueLoading
        z: 1000
        color: "#66000000"

        MouseArea {
            anchors.fill: parent
        }

        ColumnLayout {
            anchors.centerIn: parent
            spacing: Kirigami.Units.largeSpacing

            Rectangle {
                Layout.alignment: Qt.AlignHCenter
                radius: Kirigami.Units.largeSpacing
                color: Kirigami.Theme.backgroundColor
                border.color: Kirigami.Theme.disabledTextColor
                implicitWidth: Kirigami.Units.gridUnit * 20
                implicitHeight: Kirigami.Units.gridUnit * 11

                ColumnLayout {
                    anchors.fill: parent
                    anchors.margins: Kirigami.Units.largeSpacing
                    spacing: Kirigami.Units.smallSpacing

                    Item {
                        Layout.alignment: Qt.AlignHCenter
                        visible: root.loadingIndicatorStyle === "stones"
                        implicitWidth: Kirigami.Units.gridUnit * 10
                        implicitHeight: Kirigami.Units.gridUnit * 6.5

                        Item {
                            anchors.fill: parent

                            Rectangle {
                                id: boardSurface
                                anchors.horizontalCenter: parent.horizontalCenter
                                anchors.bottom: parent.bottom
                                width: parent.implicitWidth * 0.92
                                height: Kirigami.Units.gridUnit * 2.4
                                radius: Kirigami.Units.smallSpacing
                                color: "#D8B57A"
                                border.color: "#8C6A3A"
                                border.width: 1
                                opacity: 0.96
                            }

                            Repeater {
                                model: 5

                                delegate: Rectangle {
                                    width: 1
                                    height: boardSurface.height - Kirigami.Units.smallSpacing * 2
                                    color: "#8C6A3A"
                                    opacity: 0.40
                                    x: boardSurface.x + Kirigami.Units.smallSpacing
                                       + index * ((boardSurface.width - Kirigami.Units.smallSpacing * 2) / 4)
                                    y: boardSurface.y + Kirigami.Units.smallSpacing
                                }
                            }

                            Repeater {
                                model: 4

                                delegate: Rectangle {
                                    width: boardSurface.width - Kirigami.Units.smallSpacing * 2
                                    height: 1
                                    color: "#8C6A3A"
                                    opacity: 0.35
                                    x: boardSurface.x + Kirigami.Units.smallSpacing
                                    y: boardSurface.y + Kirigami.Units.smallSpacing
                                       + index * ((boardSurface.height - Kirigami.Units.smallSpacing * 2) / 3)
                                }
                            }

                            Rectangle {
                                width: Kirigami.Units.gridUnit * 2.8
                                height: Kirigami.Units.gridUnit * 0.35
                                radius: height / 2
                                color: "#30000000"
                                x: Kirigami.Units.gridUnit * 0.9
                                y: Kirigami.Units.gridUnit * 2.0
                            }

                            Item {
                                id: loadingBowl
                                width: Kirigami.Units.gridUnit * 3.3
                                height: Kirigami.Units.gridUnit * 2.0
                                x: Kirigami.Units.gridUnit * 0.9
                                y: Kirigami.Units.gridUnit * 0.35
                                rotation: -24

                                Rectangle {
                                    anchors.fill: parent
                                    radius: height * 0.48
                                    color: "#9B6230"
                                    border.color: "#6E441E"
                                    border.width: 2
                                }

                                Rectangle {
                                    x: width * 0.12
                                    y: height * 0.22
                                    width: parent.width * 0.76
                                    height: parent.height * 0.28
                                    radius: height / 2
                                    color: "#B97C42"
                                    opacity: 0.90
                                }

                                Rectangle {
                                    x: width * 0.18
                                    y: height * 0.52
                                    width: parent.width * 0.52
                                    height: parent.height * 0.16
                                    radius: height / 2
                                    color: "#D9A56D"
                                    opacity: 0.35
                                }

                                Repeater {
                                    model: 4

                                    delegate: Rectangle {
                                        required property int index
                                        width: Kirigami.Units.gridUnit * 0.40
                                        height: width
                                        radius: width / 2
                                        color: index % 2 === 0 ? "#202020" : "#F4F0E8"
                                        border.color: index % 2 === 0 ? "#111111" : "#8A8478"
                                        border.width: 1
                                        x: Kirigami.Units.gridUnit * (0.40 + index * 0.38)
                                        y: Kirigami.Units.gridUnit * (0.52 + (index % 2) * 0.10)
                                    }
                                }
                            }

                            Repeater {
                                model: 7

                                delegate: Rectangle {
                                    required property int index

                                    width: Kirigami.Units.gridUnit * 0.46
                                    height: width
                                    radius: width / 2
                                    color: index % 2 === 0 ? "#202020" : "#F4F0E8"
                                    border.color: index % 2 === 0 ? "#111111" : "#8A8478"
                                    border.width: 1
                                    opacity: 0
                                    scale: 0.90

                                    property real startX: loadingBowl.x + loadingBowl.width * 0.76
                                    property real startY: loadingBowl.y + loadingBowl.height * 0.22
                                    property real endX: Kirigami.Units.gridUnit * (5.15 + (index % 3) * 0.50)
                                    property real endY: boardSurface.y + Kirigami.Units.gridUnit * (0.62 + (index % 2) * 0.18)

                                    x: startX
                                    y: startY

                                    SequentialAnimation on x {
                                        running: root.catalogueLoading
                                                 && root.loadingIndicatorStyle === "stones"
                                        loops: Animation.Infinite

                                        PauseAnimation { duration: index * 135 }

                                        NumberAnimation {
                                            from: parent.startX
                                            to: parent.endX
                                            duration: 980
                                            easing.type: Easing.InOutQuad
                                        }

                                        PauseAnimation { duration: 260 }
                                    }

                                    SequentialAnimation on y {
                                        running: root.catalogueLoading
                                                 && root.loadingIndicatorStyle === "stones"
                                        loops: Animation.Infinite

                                        PauseAnimation { duration: index * 135 }

                                        NumberAnimation {
                                            from: parent.startY
                                            to: boardSurface.y - Kirigami.Units.gridUnit * 0.18
                                            duration: 450
                                            easing.type: Easing.OutQuad
                                        }

                                        NumberAnimation {
                                            to: parent.endY
                                            duration: 530
                                            easing.type: Easing.InQuad
                                        }

                                        PauseAnimation { duration: 260 }
                                    }

                                    SequentialAnimation on opacity {
                                        running: root.catalogueLoading
                                                 && root.loadingIndicatorStyle === "stones"
                                        loops: Animation.Infinite

                                        PauseAnimation { duration: index * 135 }
                                        NumberAnimation { from: 0; to: 1; duration: 90 }
                                        PauseAnimation { duration: 700 }
                                        NumberAnimation { from: 1; to: 0; duration: 180 }
                                        PauseAnimation { duration: 190 }
                                    }

                                    SequentialAnimation on scale {
                                        running: root.catalogueLoading
                                                 && root.loadingIndicatorStyle === "stones"
                                        loops: Animation.Infinite

                                        PauseAnimation { duration: index * 135 }
                                        NumberAnimation { from: 0.90; to: 1.00; duration: 300 }
                                        PauseAnimation { duration: 520 }
                                        NumberAnimation { from: 1.00; to: 0.94; duration: 340 }
                                        PauseAnimation { duration: 300 }
                                    }
                                }
                            }

                            Rectangle {
                                width: Kirigami.Units.gridUnit * 2.6
                                height: Kirigami.Units.gridUnit * 0.32
                                radius: height / 2
                                color: "#24000000"
                                x: Kirigami.Units.gridUnit * 4.9
                                y: boardSurface.y + Kirigami.Units.gridUnit * 1.1
                            }

                            Repeater {
                                model: 8

                                delegate: Rectangle {
                                    required property int index

                                    width: Kirigami.Units.gridUnit * 0.48
                                    height: width
                                    radius: width / 2
                                    color: index % 2 === 0 ? "#202020" : "#F4F0E8"
                                    border.color: index % 2 === 0 ? "#111111" : "#8A8478"
                                    border.width: 1

                                    x: Kirigami.Units.gridUnit * (
                                           5.00
                                           + [0.00, 0.38, 0.76, 0.22, 0.60, 0.98, 0.42, 0.82][index]
                                       )
                                    y: boardSurface.y + Kirigami.Units.gridUnit * (
                                           [0.78, 0.72, 0.82, 0.48, 0.52, 0.58, 0.28, 0.34][index]
                                       )
                                }
                            }
                        }
                    }

                    Label {
                        id: loadingCat
                        Layout.alignment: Qt.AlignHCenter
                        visible: root.loadingIndicatorStyle === "cat"
                        text: "🐱"
                        font.pixelSize: Kirigami.Units.gridUnit * 3
                        horizontalAlignment: Text.AlignHCenter

                        SequentialAnimation on rotation {
                            running: root.catalogueLoading
                                     && root.loadingIndicatorStyle === "cat"
                            loops: Animation.Infinite

                            NumberAnimation {
                                from: -12
                                to: 12
                                duration: 220
                                easing.type: Easing.InOutQuad
                            }

                            NumberAnimation {
                                from: 12
                                to: -12
                                duration: 220
                                easing.type: Easing.InOutQuad
                            }
                        }
                    }

                    Label {
                        Layout.alignment: Qt.AlignHCenter
                        text: qsTr("Loading catalogue…")
                        font.bold: true
                    }

                    Label {
                        Layout.alignment: Qt.AlignHCenter
                        text: root.loadingIndicatorStyle === "stones"
                              ? qsTr("Preparing the full game catalogue.")
                              : qsTr("Please wait while the games are loaded.")
                        wrapMode: Text.WordWrap
                        horizontalAlignment: Text.AlignHCenter
                    }
                }
            }
        }
    }

    function clearSearchResults() {
        selectedSearchRow = -1
        pendingSearchGame = null
        searchHasRun = false
        searchPatternWidth = 0
        searchPatternHeight = 0
        continuationFilterActive = false
        continuationFilterAppearances = 0
        selectedContinuationX = -1
        selectedContinuationCoreY = -1
        continuationCandidates = []
        comparisonCandidateA = null
        comparisonCandidateB = null
        searchModel.clearResults()
    }

    function searchProject(boardSize,
                           stonesJson,
                           left,
                           bottom,
                           width,
                           height) {
        selectedSearchRow = -1
        pendingSearchGame = null
        searchHasRun = true
        searchPatternWidth = width
        searchPatternHeight = height
        continuationFilterActive = false
        continuationFilterAppearances = 0
        selectedContinuationX = -1
        selectedContinuationCoreY = -1
        continuationCandidates = []
        comparisonCandidateA = null
        comparisonCandidateB = null

        searchModel.clearResults()
        catalogueTabs.currentIndex = 1

        const started = searchModel.searchProject(
                            projectPath,
                            boardSize,
                            stonesJson,
                            left,
                            bottom,
                            width,
                            height)

        if (!started)
            console.warn(searchModel.error_message)
    }

    function appearanceCountText(count) {
        return count === 1
               ? qsTr("1 appearance")
               : qsTr("%1 appearances").arg(count)
    }

    function gameCountText(count) {
        return count === 1
               ? qsTr("1 game")
               : qsTr("%1 games").arg(count)
    }

    function endedGameCountText(count) {
        return count === 1
               ? qsTr("1 game ended")
               : qsTr("%1 games ended").arg(count)
    }

    function goCoordinate(boardX, coreY) {
        const columns = "ABCDEFGHJKLMNOPQRST"

        if (boardX < 0 || boardX >= columns.length || coreY < 0)
            return "?"

        return columns.charAt(boardX) + (coreY + 1)
    }

    function setContinuationCandidates(points,
                                       boardSize,
                                       left,
                                       bottom,
                                       transformation) {
        const candidates = []

        comparisonCandidateA = null
        comparisonCandidateB = null

        if (points === undefined || points === null) {
            continuationCandidates = candidates
            return
        }

        for (const point of points) {
            let outcomes = {
                "games": 0,
                "blackWins": 0,
                "whiteWins": 0,
                "draws": 0,
                "unknown": 0
            }

            try {
                const outcomeJson =
                    searchModel.continuationOutcomeSummaryAtOccurrence(
                        point.x,
                        point.coreY,
                        left,
                        bottom,
                        transformation)

                if (outcomeJson.length > 0 && outcomeJson !== "{}")
                    outcomes = JSON.parse(outcomeJson)
            } catch (error) {
                console.warn(
                            "Could not decode continuation outcomes: "
                            + error)
            }

            candidates.push({
                "x": point.x,
                "coreY": point.coreY,
                "count": Number(point.count),
                "gameCount": Number(outcomes.games),
                "coordinate": goCoordinate(point.x, point.coreY),
                "blackWins": Number(outcomes.blackWins),
                "whiteWins": Number(outcomes.whiteWins),
                "draws": Number(outcomes.draws),
                "unknown": Number(outcomes.unknown)
            })
        }

        candidates.sort(function(a, b) {
            if (a.count !== b.count)
                return b.count - a.count

            if (a.gameCount !== b.gameCount)
                return b.gameCount - a.gameCount

            if (a.coreY !== b.coreY)
                return b.coreY - a.coreY

            return a.x - b.x
        })

        continuationCandidates = candidates
    }

    function clearContinuationCandidates() {
        continuationCandidates = []
        comparisonCandidateA = null
        comparisonCandidateB = null
    }

    function sameContinuationCandidate(first, second) {
        return first !== null
               && second !== null
               && first.x === second.x
               && first.coreY === second.coreY
    }

    function selectComparisonCandidate(slot, candidate) {
        if (slot === "A") {
            comparisonCandidateA = candidate

            if (sameContinuationCandidate(
                        comparisonCandidateA,
                        comparisonCandidateB)) {
                comparisonCandidateB = null
            }

            return
        }

        comparisonCandidateB = candidate

        if (sameContinuationCandidate(
                    comparisonCandidateA,
                    comparisonCandidateB)) {
            comparisonCandidateA = null
        }
    }

    function showComparisonCandidate(candidate) {
        if (candidate === null)
            return

        continuationCandidateSelected(
                    candidate.x,
                    candidate.coreY,
                    candidate.count)
    }

    function filterContinuationAtOccurrence(boardX, coreY, left, bottom, transformation, appearanceCount) {
        selectedSearchRow = -1
        pendingSearchGame = null
        const filtered = searchModel.filterContinuationAtOccurrence(
                             boardX, coreY, left, bottom, transformation)
        if (!filtered)
            return false
        continuationFilterActive = true
        continuationFilterAppearances = appearanceCount
        selectedContinuationX = boardX
        selectedContinuationCoreY = coreY
        return true
    }

    function clearContinuationFilter() {
        selectedSearchRow = -1
        pendingSearchGame = null
        continuationFilterActive = false
        continuationFilterAppearances = 0
        selectedContinuationX = -1
        selectedContinuationCoreY = -1
        searchModel.clearContinuationFilter()
        continuationFilterCleared()
    }

    onProjectPathChanged: {
        clearSearchResults()
        loadProject()
    }

    Component.onCompleted: loadProject()
    GameListModel {
        id: gameModel
    }

    SearchResultModel {
        id: searchModel
    }

    Connections {
        target: searchModel

        function onOccurrencesLoaded(rowNumber,
                                     occurrencesJson,
                                     errorMessage) {
            if (rowNumber !== root.selectedSearchRow
                    || root.pendingSearchGame === null) {
                return
            }

            const game = root.pendingSearchGame
            root.pendingSearchGame = null

            if (errorMessage.length > 0) {
                console.warn(errorMessage)
                return
            }

            try {
                game.matchOccurrences = JSON.parse(occurrencesJson)
            } catch (error) {
                console.warn("Could not decode match occurrences: "
                             + error)
                return
            }

            game.preferredMatchIndex = 0

            if (root.continuationFilterActive) {
                for (let occurrenceIndex = 0;
                     occurrenceIndex < game.matchOccurrences.length;
                     ++occurrenceIndex) {
                    if (game.matchOccurrences[occurrenceIndex]
                            .selectedContinuationMatch === true) {
                        game.preferredMatchIndex = occurrenceIndex
                        break
                    }
                }
            }

            root.gameSelected(game)
        }
    }

   contentItem: ColumnLayout {
    anchors.fill: parent
    spacing: 0

    RowLayout {
        Layout.fillWidth: true
        spacing: 0

        TabBar {
            id: catalogueTabs

            TabButton {
                text: qsTr("Game database")
            }

            TabButton {
                text: qsTr("Search results")
            }
        }

        Item {
            Layout.fillWidth: true
        }

    }

    Frame {
        Layout.fillWidth: true
        Layout.preferredHeight: visible
            ? Kirigami.Units.gridUnit * 4.2
            : 0

        visible: root.searchResultsSelected
        padding: Kirigami.Units.smallSpacing

        contentItem: ColumnLayout {
            spacing: Kirigami.Units.smallSpacing

            RowLayout {
                Layout.fillWidth: true

                Label {
                    text: qsTr("Continuation map")
                    font.bold: true
                    Layout.fillWidth: true
                }

                ToolButton {
                    visible: root.continuationFilterActive
                    text: qsTr("Clear filter")
                    onClicked: root.clearContinuationFilter()
                }
            }

            Label {
                Layout.fillWidth: true
                wrapMode: Text.WordWrap

                text: {
                    if (searchModel.search_in_progress) {
                        return qsTr("Searching the database… %1 of %2 games examined")
                            .arg(searchModel.games_examined)
                            .arg(searchModel.total_games)
                    }

                    const distribution = root.nextMoveDistribution

                    if (distribution === null)
                        return qsTr("Run a pattern search to build a continuation map.")

                    if (root.continuationFilterActive) {
                        return qsTr("Selected %1 · %2 · %3 supporting games")
                            .arg(root.goCoordinate(
                                     root.selectedContinuationX,
                                     root.selectedContinuationCoreY))
                            .arg(root.appearanceCountText(
                                     root.continuationFilterAppearances))
                            .arg(searchView.count)
                    }

                    return qsTr("%1 · %2 · %3 local · %4 outside · %5 passes · %6")
                        .arg(root.appearanceCountText(distribution.appearances))
                        .arg(root.gameCountText(distribution.matchingGames))
                        .arg(root.nextMoveLocalCount)
                        .arg(distribution.outsideDisplayedArea)
                        .arg(distribution.passes)
                        .arg(root.endedGameCountText(distribution.gameEnded))
                }

                opacity: root.nextMoveDistribution === null
                         && !searchModel.search_in_progress
                         ? 0.55
                         : 0.82
            }

            Label {
                visible: root.nextMoveDistribution !== null
                         && !root.continuationFilterActive
                         && !searchModel.search_in_progress
                text: qsTr("Larger circles indicate more frequently played immediate continuations.")
                opacity: 0.62
                font.italic: true
                Layout.fillWidth: true
            }
        }
    }

    Frame {
        Layout.fillWidth: true
        Layout.preferredHeight: visible
            ? Math.min(Kirigami.Units.gridUnit * 7,
                       Kirigami.Units.gridUnit
                       * (2.9 + root.continuationCandidates.length * 1.45))
            : 0

        visible: root.searchResultsSelected
                 && !searchModel.search_in_progress
                 && root.continuationCandidates.length > 0

        padding: Kirigami.Units.smallSpacing

        contentItem: ColumnLayout {
            spacing: Kirigami.Units.smallSpacing

            Label {
                text: qsTr("Professional continuations")
                font.bold: true
                Layout.fillWidth: true
            }

            RowLayout {
                Layout.fillWidth: true
                spacing: Kirigami.Units.smallSpacing

                Label {
                    text: qsTr("Move")
                    font.bold: true
                    opacity: 0.72
                    Layout.minimumWidth: Kirigami.Units.gridUnit * 4
                    Layout.maximumWidth: Kirigami.Units.gridUnit * 4
                }

                Label {
                    text: qsTr("Appearances")
                    font.bold: true
                    opacity: 0.72
                    Layout.minimumWidth: Kirigami.Units.gridUnit * 7
                    Layout.maximumWidth: Kirigami.Units.gridUnit * 7
                }

                Label {
                    text: qsTr("Games")
                    font.bold: true
                    opacity: 0.72
                    Layout.minimumWidth: Kirigami.Units.gridUnit * 5
                    Layout.maximumWidth: Kirigami.Units.gridUnit * 5
                }

                Label {
                    text: qsTr("Compare")
                    font.bold: true
                    opacity: 0.72
                    Layout.minimumWidth: Kirigami.Units.gridUnit * 5
                    Layout.maximumWidth: Kirigami.Units.gridUnit * 5
                    horizontalAlignment: Text.AlignHCenter
                }

                Item {
                    Layout.fillWidth: true
                }
            }

            Kirigami.Separator {
                Layout.fillWidth: true
            }

            ListView {
                id: continuationCandidateView

                Layout.fillWidth: true
                Layout.fillHeight: true
                clip: true
                model: root.continuationCandidates

                ScrollBar.vertical: ScrollBar {}

                delegate: Item {
                    id: candidateDelegate

                    required property var modelData

                    readonly property bool selected:
                        root.continuationFilterActive
                        && root.selectedContinuationX === modelData.x
                        && root.selectedContinuationCoreY === modelData.coreY

                    width: continuationCandidateView.width
                    height: Math.round(Kirigami.Units.gridUnit * 1.4)

                    Rectangle {
                        anchors.fill: parent
                        color: candidateDelegate.selected
                               ? Kirigami.Theme.highlightColor
                               : rowMouse.containsMouse
                                 ? Kirigami.Theme.alternateBackgroundColor
                                 : "transparent"
                    }

                    MouseArea {
                        id: rowMouse
                        anchors.fill: parent
                        hoverEnabled: true
                        onClicked: {
                            root.continuationCandidateSelected(
                                        candidateDelegate.modelData.x,
                                        candidateDelegate.modelData.coreY,
                                        candidateDelegate.modelData.count)
                        }
                    }

                    RowLayout {
                        anchors.fill: parent
                        spacing: Kirigami.Units.smallSpacing

                        Label {
                            text: candidateDelegate.modelData.coordinate
                            font.bold: true
                            Layout.minimumWidth: Kirigami.Units.gridUnit * 4
                            Layout.maximumWidth: Kirigami.Units.gridUnit * 4
                        }

                        Label {
                            text: root.appearanceCountText(
                                      candidateDelegate.modelData.count)
                            Layout.minimumWidth: Kirigami.Units.gridUnit * 7
                            Layout.maximumWidth: Kirigami.Units.gridUnit * 7
                        }

                        Label {
                            text: root.gameCountText(
                                      candidateDelegate.modelData.gameCount)
                            Layout.minimumWidth: Kirigami.Units.gridUnit * 5
                            Layout.maximumWidth: Kirigami.Units.gridUnit * 5
                        }

                        RowLayout {
                            Layout.minimumWidth: Kirigami.Units.gridUnit * 5
                            Layout.maximumWidth: Kirigami.Units.gridUnit * 5
                            spacing: 0

                            ToolButton {
                                text: qsTr("A")
                                checkable: true

                                checked:
                                    root.sameContinuationCandidate(
                                        root.comparisonCandidateA,
                                        candidateDelegate.modelData)

                                onClicked:
                                    root.selectComparisonCandidate(
                                        "A",
                                        candidateDelegate.modelData)
                            }

                            ToolButton {
                                text: qsTr("B")
                                checkable: true

                                checked:
                                    root.sameContinuationCandidate(
                                        root.comparisonCandidateB,
                                        candidateDelegate.modelData)

                                onClicked:
                                    root.selectComparisonCandidate(
                                        "B",
                                        candidateDelegate.modelData)
                            }
                        }

                        Item {
                            Layout.fillWidth: true
                        }
                    }
                }
            }
        }
    }

    Frame {
        Layout.fillWidth: true
        Layout.preferredHeight: visible
            ? Kirigami.Units.gridUnit
              * (3.3
                 + (root.comparisonCandidateA !== null ? 1.75 : 0)
                 + (root.comparisonCandidateB !== null ? 1.75 : 0))
            : 0

        visible: root.searchResultsSelected
                 && (root.comparisonCandidateA !== null
                     || root.comparisonCandidateB !== null)

        padding: Kirigami.Units.smallSpacing

        contentItem: ColumnLayout {
            spacing: Kirigami.Units.smallSpacing

            Label {
                text: qsTr("Candidate comparison")
                font.bold: true
                Layout.fillWidth: true
            }

            RowLayout {
                visible: root.comparisonCandidateA !== null
                Layout.fillWidth: true
                spacing: Kirigami.Units.smallSpacing

                Label {
                    text: qsTr("A")
                    font.bold: true
                    Layout.preferredWidth: Kirigami.Units.gridUnit * 2
                }

                Label {
                    text: root.comparisonCandidateA === null
                          ? ""
                          : root.comparisonCandidateA.coordinate
                    font.bold: true
                    Layout.preferredWidth: Kirigami.Units.gridUnit * 3
                }

                Label {
                    text: root.comparisonCandidateA === null
                          ? ""
                          : qsTr("%1 · %2")
                                .arg(root.appearanceCountText(
                                     root.comparisonCandidateA.count))
                                .arg(root.gameCountText(
                                     root.comparisonCandidateA.gameCount))
                    Layout.preferredWidth: Kirigami.Units.gridUnit * 12
                }

                Label {
                    text: root.comparisonCandidateA === null
                          ? ""
                          : qsTr("Black %1 · White %2 · Draw %3 · Unknown %4")
                                .arg(root.comparisonCandidateA.blackWins)
                                .arg(root.comparisonCandidateA.whiteWins)
                                .arg(root.comparisonCandidateA.draws)
                                .arg(root.comparisonCandidateA.unknown)
                    Layout.fillWidth: true
                    elide: Text.ElideRight
                }

                Button {
                    text: qsTr("Show games")
                    onClicked:
                        root.showComparisonCandidate(
                            root.comparisonCandidateA)
                }

                ToolButton {
                    text: qsTr("×")
                    onClicked: root.comparisonCandidateA = null
                }
            }

            RowLayout {
                visible: root.comparisonCandidateB !== null
                Layout.fillWidth: true
                spacing: Kirigami.Units.smallSpacing

                Label {
                    text: qsTr("B")
                    font.bold: true
                    Layout.preferredWidth: Kirigami.Units.gridUnit * 2
                }

                Label {
                    text: root.comparisonCandidateB === null
                          ? ""
                          : root.comparisonCandidateB.coordinate
                    font.bold: true
                    Layout.preferredWidth: Kirigami.Units.gridUnit * 3
                }

                Label {
                    text: root.comparisonCandidateB === null
                          ? ""
                          : qsTr("%1 · %2")
                                .arg(root.appearanceCountText(
                                     root.comparisonCandidateB.count))
                                .arg(root.gameCountText(
                                     root.comparisonCandidateB.gameCount))
                    Layout.preferredWidth: Kirigami.Units.gridUnit * 12
                }

                Label {
                    text: root.comparisonCandidateB === null
                          ? ""
                          : qsTr("Black %1 · White %2 · Draw %3 · Unknown %4")
                                .arg(root.comparisonCandidateB.blackWins)
                                .arg(root.comparisonCandidateB.whiteWins)
                                .arg(root.comparisonCandidateB.draws)
                                .arg(root.comparisonCandidateB.unknown)
                    Layout.fillWidth: true
                    elide: Text.ElideRight
                }

                Button {
                    text: qsTr("Show games")
                    onClicked:
                        root.showComparisonCandidate(
                            root.comparisonCandidateB)
                }

                ToolButton {
                    text: qsTr("×")
                    onClicked: root.comparisonCandidateB = null
                }
            }

            Label {
                text: qsTr(
                          "Recorded game colours and outcomes are descriptive, "
                          + "not an evaluation of either continuation.")
                opacity: 0.62
                font.italic: true
                wrapMode: Text.WordWrap
                Layout.fillWidth: true
            }
        }
    }




        Rectangle {
            Layout.fillWidth: true
            implicitHeight: headerRow.implicitHeight
            visible: catalogueTabs.currentIndex === 0
            color: Kirigami.Theme.alternateBackgroundColor

          RowLayout {
    id: headerRow

    anchors.fill: parent
    spacing: 0

    Label {
        id: blackHeader

        text: root.sortHeaderText(
                  "black",
                  qsTr("Black"))

        Layout.preferredWidth: 150
        padding: Kirigami.Units.smallSpacing
        font.bold: true

        color: blackSortArea.containsMouse
               ? Kirigami.Theme.highlightColor
               : Kirigami.Theme.textColor

        MouseArea {
            id: blackSortArea

            anchors.fill: parent
            hoverEnabled: true
            cursorShape: Qt.PointingHandCursor

            onClicked: root.sortBy("black", true)
        }
    }

    Label {
        id: whiteHeader

        text: root.sortHeaderText(
                  "white",
                  qsTr("White"))

        Layout.preferredWidth: 150
        padding: Kirigami.Units.smallSpacing
        font.bold: true

        color: whiteSortArea.containsMouse
               ? Kirigami.Theme.highlightColor
               : Kirigami.Theme.textColor

        MouseArea {
            id: whiteSortArea

            anchors.fill: parent
            hoverEnabled: true
            cursorShape: Qt.PointingHandCursor

            onClicked: root.sortBy("white", true)
        }
    }

    Label {
        id: dateHeader

        text: root.sortHeaderText(
                  "date",
                  qsTr("Date"))

        Layout.preferredWidth: 105
        padding: Kirigami.Units.smallSpacing
        font.bold: true

        color: dateSortArea.containsMouse
               ? Kirigami.Theme.highlightColor
               : Kirigami.Theme.textColor

        MouseArea {
            id: dateSortArea

            anchors.fill: parent
            hoverEnabled: true
            cursorShape: Qt.PointingHandCursor

            onClicked: root.sortBy("date", false)
        }
    }

    Label {
        id: resultHeader

        text: root.sortHeaderText(
                  "result",
                  qsTr("Result"))

        Layout.preferredWidth: 80
        padding: Kirigami.Units.smallSpacing
        font.bold: true

        color: resultSortArea.containsMouse
               ? Kirigami.Theme.highlightColor
               : Kirigami.Theme.textColor

        MouseArea {
            id: resultSortArea

            anchors.fill: parent
            hoverEnabled: true
            cursorShape: Qt.PointingHandCursor

            onClicked: root.sortBy("result", true)
        }
    }

    Label {
        text: qsTr("Komi")

        Layout.preferredWidth: 60
        padding: Kirigami.Units.smallSpacing
        horizontalAlignment: Text.AlignHCenter
        font.bold: true
    }

    Label {
        id: eventHeader

        text: root.sortHeaderText(
                  "event",
                  qsTr("Event"))

        Layout.fillWidth: true
        padding: Kirigami.Units.smallSpacing
        font.bold: true

        color: eventSortArea.containsMouse
               ? Kirigami.Theme.highlightColor
               : Kirigami.Theme.textColor

        MouseArea {
            id: eventSortArea

            anchors.fill: parent
            hoverEnabled: true
            cursorShape: Qt.PointingHandCursor

            onClicked: root.sortBy("event", true)
        }
    }
}



        }

        Kirigami.Separator {
            Layout.fillWidth: true
            visible: catalogueTabs.currentIndex === 0
        }

        Rectangle {
            Layout.fillWidth: true
            implicitHeight: searchHeaderRow.implicitHeight
            visible: catalogueTabs.currentIndex === 1
            color: Kirigami.Theme.alternateBackgroundColor

            RowLayout {
                id: searchHeaderRow

                anchors.fill: parent
                spacing: 0

                Label {
                    text: qsTr("Black")
                    Layout.preferredWidth: 150
                    padding: Kirigami.Units.smallSpacing
                    font.bold: true
                }

                Label {
                    text: qsTr("White")
                    Layout.preferredWidth: 150
                    padding: Kirigami.Units.smallSpacing
                    font.bold: true
                }

                Label {
                    text: qsTr("Date")
                    Layout.preferredWidth: 105
                    padding: Kirigami.Units.smallSpacing
                    font.bold: true
                }

                Label {
                    text: qsTr("Result")
                    Layout.preferredWidth: 80
                    padding: Kirigami.Units.smallSpacing
                    font.bold: true
                }

                Label {
                    text: qsTr("Matches")
                    Layout.preferredWidth: 80
                    padding: Kirigami.Units.smallSpacing
                    horizontalAlignment: Text.AlignHCenter
                    font.bold: true
                }

                Label {
                    text: qsTr("Event")
                    Layout.fillWidth: true
                    padding: Kirigami.Units.smallSpacing
                    font.bold: true
                }
            }
        }

        Kirigami.Separator {
            Layout.fillWidth: true
            visible: catalogueTabs.currentIndex === 1
        }

        ListView {
            id: searchView

            Layout.fillWidth: true
            Layout.fillHeight: true

            visible: catalogueTabs.currentIndex === 1

            clip: true
            model: searchModel
            currentIndex: root.selectedSearchRow

            ScrollBar.vertical: ScrollBar {
                id: searchVerticalScrollBar
            }

            delegate: ItemDelegate {
                id: searchRowDelegate

                required property int index
                required property var gameId
                required property string blackPlayer
                required property string whitePlayer
                required property string playedDate
                required property string result
                required property string event
                required property string komi
                required property int matchCount
                required property int firstMatchMove
                required property int firstMatchLeft
                required property int firstMatchBottom

                width: searchView.width
                height: Math.round(Kirigami.Units.gridUnit * 1.75)
                rightPadding: searchVerticalScrollBar.width + 4
                clip: true

                highlighted: root.selectedSearchRow === index
                enabled: !searchModel.occurrence_load_in_progress

                background: Rectangle {
                    anchors {
                        left: parent.left
                        right: parent.right
                        verticalCenter: parent.verticalCenter
                        leftMargin: 4
                        rightMargin: searchVerticalScrollBar.width + 4
                    }

                    height: parent.height - 4
                    radius: 4

                    color: searchRowDelegate.highlighted
                        ? searchRowDelegate.palette.highlight
                        : searchRowDelegate.hovered
                            ? searchRowDelegate.palette.alternateBase
                            : "transparent"
                }

                onClicked: {
                    root.selectedSearchRow = index

                    root.pendingSearchGame = {
                        gameId: gameId,
                        black: blackPlayer,
                        white: whitePlayer,
                        gameDate: playedDate,
                        result: result,
                        eventName: event,
                        komi: searchRowDelegate.komi,
                        matchMove: firstMatchMove,
                        matchLeft: firstMatchLeft,
                        matchBottom: firstMatchBottom,
                        matchOccurrences: [],
                        matchWidth: root.searchPatternWidth,
                        matchHeight: root.searchPatternHeight,
                        fromSearchResults: true
                    }

                    if (!searchModel.loadOccurrences(index)) {
                        root.pendingSearchGame = null
                        console.warn(
                            "Could not start loading match occurrences")
                    }
                }

                contentItem: RowLayout {
                    spacing: 0

                    Label {
                        text: searchRowDelegate.blackPlayer
                        Layout.preferredWidth: 150
                        elide: Text.ElideRight
                        leftPadding: Kirigami.Units.smallSpacing
                    }

                    Label {
                        text: searchRowDelegate.whitePlayer
                        Layout.preferredWidth: 150
                        elide: Text.ElideRight
                        leftPadding: Kirigami.Units.smallSpacing
                    }

                    Label {
                        text: searchRowDelegate.playedDate
                        Layout.preferredWidth: 105
                        Layout.maximumWidth: 105
                        elide: Text.ElideRight
                        clip: true
                        leftPadding: Kirigami.Units.smallSpacing
                    }

                    Label {
                        text: searchRowDelegate.result
                        Layout.preferredWidth: 80
                        Layout.maximumWidth: 80
                        elide: Text.ElideRight
                        clip: true
                        leftPadding: Kirigami.Units.smallSpacing
                    }

                    Label {
                        text: searchRowDelegate.matchCount
                        Layout.preferredWidth: 80
                        horizontalAlignment: Text.AlignHCenter
                    }

                    Label {
                        text: searchRowDelegate.event
                        Layout.fillWidth: true
                        Layout.minimumWidth: 0
                        elide: Text.ElideRight
                        clip: true
                        leftPadding: Kirigami.Units.smallSpacing
                        rightPadding: Kirigami.Units.smallSpacing
                    }
                }
            }

            ColumnLayout {
                anchors.centerIn: parent
                spacing: Kirigami.Units.largeSpacing
                visible: searchView.count === 0

                Kirigami.PlaceholderMessage {
                    Layout.alignment: Qt.AlignHCenter

                    text: {
                        if (root.searchInProgress) {
                            if (searchModel.cancel_requested)
                                return qsTr("Cancelling…")

                            if (searchModel.total_games > 0
                                    && searchModel.games_examined
                                    >= searchModel.total_games) {
                                return qsTr("Preparing results…")
                            }

                            return qsTr("Searching…")
                        }

                        if (searchModel.search_cancelled)
                            return qsTr("Search cancelled")

                        if (searchModel.error_message.length > 0)
                            return qsTr("Search failed")

                        if (!root.searchHasRun)
                            return qsTr("No search has been run")

                        return qsTr("No matching games")
                    }

                    explanation: {
                        if (root.searchInProgress) {
                            if (searchModel.total_games <= 0)
                                return qsTr(
                                    "Preparing the project database")

                            if (searchModel.games_examined
                                    >= searchModel.total_games) {
                                return qsTr(
                                    "The game scan is complete\n"
                                    + "Preparing %1 matching games "
                                    + "for display")
                                    .arg(
                                        searchModel.matching_games)
                            }

                            const matchingText =
                                searchModel.matching_games === 1
                                ? qsTr("1 matching game")
                                : qsTr("%1 matching games")
                                    .arg(searchModel.matching_games)

                            const occurrenceText =
                                searchModel.matches_found === 1
                                ? qsTr("1 occurrence")
                                : qsTr("%1 occurrences")
                                    .arg(searchModel.matches_found)

                            return qsTr(
                                "%1 of %2 games searched\n"
                                + "%3 · %4 found so far")
                                .arg(searchModel.games_examined)
                                .arg(searchModel.total_games)
                                .arg(matchingText)
                                .arg(occurrenceText)
                        }

                        if (searchModel.search_cancelled) {
                            return qsTr(
                                "Cancelled after %1 of %2 games\n"
                                + "Partial results were discarded")
                                .arg(searchModel.games_examined)
                                .arg(searchModel.total_games)
                        }

                        return searchModel.error_message
                    }
                }

                Button {
                    Layout.alignment: Qt.AlignHCenter
                    visible: root.searchInProgress

                    text: searchModel.cancel_requested
                        ? qsTr("Cancelling…")
                        : qsTr("Cancel")

                    enabled: !searchModel.cancel_requested

                    onClicked: searchModel.cancelSearch()
                }
            }
        }

        ListView {
            id: gameView

            Layout.fillWidth: true
            Layout.fillHeight: true

            visible: catalogueTabs.currentIndex === 0

            clip: true
            model: gameModel
            currentIndex: root.selectedRow

            ScrollBar.vertical: ScrollBar {
                id: verticalScrollBar
            }

            delegate: ItemDelegate {
                id: rowDelegate

                required property int index
                required property var gameId
                required property string blackPlayer
                required property string whitePlayer
                required property string playedDate
                required property string result
                required property string event
                required property string komi

                width: gameView.width
                height: Math.round(Kirigami.Units.gridUnit * 1.75)
                rightPadding: verticalScrollBar.width + 4
                clip: true

                highlighted: root.selectedRow === index

                background: Rectangle {
                    anchors {
                        left: parent.left
                        right: parent.right
                        verticalCenter: parent.verticalCenter
                        leftMargin: 4
                        rightMargin: verticalScrollBar.width + 4
                    }

                    height: parent.height - 4

                    radius: 4

                    color: rowDelegate.highlighted
                        ? rowDelegate.palette.highlight
                        : rowDelegate.hovered
                            ? rowDelegate.palette.alternateBase
                            : "transparent"
                }

                onClicked: {
                    root.selectedRow = index

                    root.gameSelected({
                        gameId: gameId,
                        black: blackPlayer,
                        white: whitePlayer,
                        gameDate: playedDate,
                        result: result,
                        eventName: event,
                        komi: rowDelegate.komi,
                        fromSearchResults: false
                    })
                }

                contentItem: RowLayout {
                    spacing: 0

                    Label {
                        text: rowDelegate.blackPlayer
                        Layout.preferredWidth: 150
                        elide: Text.ElideRight
                        leftPadding: Kirigami.Units.smallSpacing
                    }

                    Label {
                        text: rowDelegate.whitePlayer
                        Layout.preferredWidth: 150
                        elide: Text.ElideRight
                        leftPadding: Kirigami.Units.smallSpacing
                    }

                    Label {
                        text: rowDelegate.playedDate
                        Layout.preferredWidth: 105
                        Layout.maximumWidth: 105
                        elide: Text.ElideRight
                        clip: true
                        leftPadding: Kirigami.Units.smallSpacing
                    }

                    Label {
                        text: rowDelegate.result
                        Layout.preferredWidth: 80
                        Layout.maximumWidth: 80
                        elide: Text.ElideRight
                        clip: true
                        leftPadding: Kirigami.Units.smallSpacing
                    }

                    Label {
                        text: rowDelegate.komi
                        Layout.preferredWidth: 60
                        horizontalAlignment: Text.AlignHCenter
                    }

                   Label {
                        text: rowDelegate.event
                        Layout.fillWidth: true
                        Layout.minimumWidth: 0
                        elide: Text.ElideRight
                        clip: true
                        leftPadding: Kirigami.Units.smallSpacing
                        rightPadding: Kirigami.Units.smallSpacing
                    }
                }
            }

            Kirigami.PlaceholderMessage {
                anchors.centerIn: parent
                visible: gameView.count === 0

                text: {
                                     if (root.projectPath.length === 0) {
                        return qsTr("No project selected")
                    }

                    if (!root.projectLoaded
                            && gameModel.error_message.length > 0) {
                        return qsTr("Could not load project")
                    }

                    return qsTr("No games found")
                }

                explanation: {
                                       if (!root.projectLoaded) {
                        return gameModel.error_message
                    }

                    return ""
                }
            }
        }

    }
}
