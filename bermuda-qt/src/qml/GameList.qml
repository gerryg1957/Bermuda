import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import QtQml
import org.kde.kirigami as Kirigami
import org.bermuda.app

Kirigami.AbstractCard {
    id: root

    signal gameSelected(var game)
    signal removeSelectedMyGameRequested()
    signal continuationCandidateSelected(int boardX, int coreY, int count)
    signal continuationFilterCleared()
    signal sourceContinuationMapReady(var points)

    property string databaseProjectPath: ""
    property string myGamesProjectPath: ""
    property bool showingMyGames: false
    property bool removeSelectedMyGameEnabled: false

    readonly property string projectPath:
        showingMyGames
        ? myGamesProjectPath
        : databaseProjectPath

    readonly property var gameModel:
        showingMyGames
        ? myGamesGameModel
        : databaseGameModel

    property int selectedRow: -1
    property int databaseSelectedRow: -1
    property int myGamesSelectedRow: -1
    property int selectedSearchRow: -1

    property bool databaseProjectLoaded: false
    property bool myGamesProjectLoaded: false

    readonly property bool projectLoaded:
        showingMyGames
        ? myGamesProjectLoaded
        : databaseProjectLoaded

    property var databaseCatalogueState: null
    property var myGamesCatalogueState: null
    property bool searchHasRun: false
    readonly property bool searchInProgress:
        searchModel.search_in_progress
    readonly property int searchResultCount:
        searchView.count
    property int searchPatternWidth: 0
    property int searchPatternHeight: 0
    property int searchBoardSize: 0
    property int searchPatternLeft: 0
    property int searchPatternBottom: 0
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

    readonly property var searchOutcomeSummary: {
        const json = searchModel.outcome_summary_json

        if (json.length === 0 || json === "{}")
            return null

        try {
            return JSON.parse(json)
        } catch (error) {
            console.warn(
                        "Could not decode current search outcomes: "
                        + error)

            return null
        }
    }

    readonly property string searchOutcomeText: {
        const summary = searchOutcomeSummary

        if (summary === null)
            return ""

        const games = Number(summary.games)

        if (games === 0)
            return qsTr("0 games")

        const blackWins = Number(summary.blackWins)
        const whiteWins = Number(summary.whiteWins)
        const decisiveGames = blackWins + whiteWins
        const gameText = games === 1
                         ? qsTr("1 game")
                         : qsTr("%1 games").arg(games)

        if (decisiveGames === 0)
            return qsTr("%1 · no recorded winner").arg(gameText)

        const blackPercent = 100.0 * blackWins / decisiveGames
        const whitePercent = 100.0 * whiteWins / decisiveGames

        return qsTr("%1 · B %2% · W %3%")
            .arg(gameText)
            .arg(blackPercent.toFixed(1))
            .arg(whitePercent.toFixed(1))
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

    function showSourceContinuationMap() {
        const distribution = nextMoveDistribution

        if (distribution === null
                || distribution.points === undefined
                || searchBoardSize <= 0) {
            return
        }

        const points = []

        for (const point of distribution.points) {
            const boardX = searchPatternLeft + Number(point.x)
            const coreY = searchPatternBottom + Number(point.y)

            /*
             * Continuation statistics include a margin around the selected
             * pattern. At an edge or corner, some normalised margin points
             * can therefore lie outside this particular board occurrence.
             */
            if (boardX < 0 || coreY < 0
                    || boardX >= searchBoardSize
                    || coreY >= searchBoardSize) {
                continue
            }

            points.push({
                "x": boardX,
                "coreY": coreY,
                "count": Number(point.count)
            })
        }

        setContinuationCandidates(
                    points,
                    searchBoardSize,
                    searchPatternLeft,
                    searchPatternBottom,
                    "identity")

        sourceContinuationMapReady(points)
    }

    onSearchInProgressChanged: {
        if (searchInProgress)
            resetSearchResultFilters()

        if (!searchInProgress && nextMoveDistribution !== null)
            Qt.callLater(showSourceContinuationMap)
    }

    readonly property bool searchResultsSelected:
    mainTabs.currentIndex === 2

    readonly property string searchErrorMessage:
        searchModel.error_message

    property string sortColumn: "none"
    property bool sortAscending: false
    property string sortThenColumn: "none"
    property bool sortThenAscending: true
    property string sortThirdColumn: "none"
    property bool sortThirdAscending: true

    property bool whiteColumnFirst: false
    readonly property int playerColumnWidth: 150

    property string cataloguePlayer: ""
    property string catalogueVersus: ""
    property string catalogueColour: "either"
    property string catalogueEvent: ""
    property string catalogueDateFrom: ""
    property string catalogueDateTo: ""
    property string catalogueResult: "any"

    // These are the criteria that produced the rows currently displayed.
    // Editing or clearing the fields above does not alter the result set
    // until the user explicitly starts another catalogue search.
    property string appliedCataloguePlayer: ""
    property string appliedCatalogueVersus: ""
    property string appliedCatalogueColour: "either"
    property string appliedCatalogueEvent: ""
    property string appliedCatalogueDateFrom: ""
    property string appliedCatalogueDateTo: ""
    property string appliedCatalogueResult: "any"

    property string searchFilterPlayer: ""
    property string searchFilterVersus: ""
    property string searchFilterColour: "either"
    property string searchFilterEvent: ""
    property string searchFilterDateFrom: ""
    property string searchFilterDateTo: ""
    property string searchFilterResult: "any"

    property string searchSortColumn: "none"
    property bool searchSortAscending: false

    readonly property bool catalogueLoading: gameModel.loading
    property string loadingIndicatorStyle: "stones"
    padding: 0



    function captureCatalogueState() {
        return {
            cataloguePlayer: cataloguePlayer,
            catalogueVersus: catalogueVersus,
            catalogueColour: catalogueColour,
            catalogueEvent: catalogueEvent,
            catalogueDateFrom: catalogueDateFrom,
            catalogueDateTo: catalogueDateTo,
            catalogueResult: catalogueResult,

            appliedCataloguePlayer: appliedCataloguePlayer,
            appliedCatalogueVersus: appliedCatalogueVersus,
            appliedCatalogueColour: appliedCatalogueColour,
            appliedCatalogueEvent: appliedCatalogueEvent,
            appliedCatalogueDateFrom: appliedCatalogueDateFrom,
            appliedCatalogueDateTo: appliedCatalogueDateTo,
            appliedCatalogueResult: appliedCatalogueResult,

            sortColumn: sortColumn,
            sortAscending: sortAscending,
            sortThenColumn: sortThenColumn,
            sortThenAscending: sortThenAscending,
            sortThirdColumn: sortThirdColumn,
            sortThirdAscending: sortThirdAscending
        }
    }

    function applyCatalogueState(state) {
        if (state === null)
            return

        cataloguePlayer = state.cataloguePlayer
        catalogueVersus = state.catalogueVersus
        catalogueColour = state.catalogueColour
        catalogueEvent = state.catalogueEvent
        catalogueDateFrom = state.catalogueDateFrom
        catalogueDateTo = state.catalogueDateTo
        catalogueResult = state.catalogueResult

        appliedCataloguePlayer = state.appliedCataloguePlayer
        appliedCatalogueVersus = state.appliedCatalogueVersus
        appliedCatalogueColour = state.appliedCatalogueColour
        appliedCatalogueEvent = state.appliedCatalogueEvent
        appliedCatalogueDateFrom = state.appliedCatalogueDateFrom
        appliedCatalogueDateTo = state.appliedCatalogueDateTo
        appliedCatalogueResult = state.appliedCatalogueResult

        sortColumn = state.sortColumn
        sortAscending = state.sortAscending
        sortThenColumn = state.sortThenColumn
        sortThenAscending = state.sortThenAscending
        sortThirdColumn = state.sortThirdColumn
        sortThirdAscending = state.sortThirdAscending
    }

    function saveCurrentCatalogueState() {
        if (showingMyGames) {
            myGamesCatalogueState = captureCatalogueState()
            myGamesSelectedRow = selectedRow
        } else {
            databaseCatalogueState = captureCatalogueState()
            databaseSelectedRow = selectedRow
        }
    }

    function resetCatalogueSelection(myGames) {
        if (myGames) {
            myGamesSelectedRow = -1

            if (showingMyGames)
                selectedRow = -1
        } else {
            databaseSelectedRow = -1

            if (!showingMyGames)
                selectedRow = -1
        }
    }

    function loadCatalogue(model, path, state, myGames) {
        resetCatalogueSelection(myGames)

        if (myGames)
            myGamesProjectLoaded = false
        else
            databaseProjectLoaded = false

        if (path.length === 0)
            return

        if (!model.loadFilteredProject(
                    path,
                    state.sortColumn,
                    state.sortAscending,
                    state.sortThenColumn,
                    state.sortThenAscending,
                    state.sortThirdColumn,
                    state.sortThirdAscending,
                    state.appliedCataloguePlayer,
                    state.appliedCatalogueVersus,
                    state.appliedCatalogueColour,
                    state.appliedCatalogueEvent,
                    state.appliedCatalogueDateFrom,
                    state.appliedCatalogueDateTo,
                    state.appliedCatalogueResult)) {
            if (myGames)
                myGamesProjectLoaded = false
            else
                databaseProjectLoaded = false
        }
    }

    function loadProject() {
        saveCurrentCatalogueState()

        if (showingMyGames) {
            loadCatalogue(
                myGamesGameModel,
                myGamesProjectPath,
                myGamesCatalogueState,
                true)
        } else {
            loadCatalogue(
                databaseGameModel,
                databaseProjectPath,
                databaseCatalogueState,
                false)
        }
    }

    function reloadDatabaseProject() {
        if (!componentReadyForProjectLoad) {
            databaseProjectLoaded = false
            return
        }

        if (!showingMyGames)
            databaseCatalogueState = captureCatalogueState()

        loadCatalogue(
            databaseGameModel,
            databaseProjectPath,
            databaseCatalogueState,
            false)
    }

    function reloadMyGamesProject() {
        if (!componentReadyForProjectLoad) {
            myGamesProjectLoaded = false
            return
        }

        if (showingMyGames)
            myGamesCatalogueState = captureCatalogueState()

        loadCatalogue(
            myGamesGameModel,
            myGamesProjectPath,
            myGamesCatalogueState,
            true)
    }

    function ensureCurrentProjectLoaded() {
        if (showingMyGames) {
            if (!myGamesProjectLoaded) {
                loadCatalogue(
                    myGamesGameModel,
                    myGamesProjectPath,
                    myGamesCatalogueState,
                    true)
            }
        } else {
            if (!databaseProjectLoaded) {
                loadCatalogue(
                    databaseGameModel,
                    databaseProjectPath,
                    databaseCatalogueState,
                    false)
            }
        }
    }

    function selectCatalogue(myGames) {
        if (showingMyGames === myGames)
            return

        saveCurrentCatalogueState()
        clearSearchResults()

        showingMyGames = myGames

        if (showingMyGames) {
            applyCatalogueState(myGamesCatalogueState)
            selectedRow = myGamesSelectedRow
        } else {
            applyCatalogueState(databaseCatalogueState)
            selectedRow = databaseSelectedRow
        }

        ensureCurrentProjectLoaded()
    }

    function applyCatalogueSearch() {
        appliedCataloguePlayer = cataloguePlayer
        appliedCatalogueVersus = catalogueVersus
        appliedCatalogueColour = catalogueColour
        appliedCatalogueEvent = catalogueEvent
        appliedCatalogueDateFrom = catalogueDateFrom
        appliedCatalogueDateTo = catalogueDateTo
        appliedCatalogueResult = catalogueResult

        loadProject()
    }

    function filterSearchResults() {
        selectedSearchRow = -1

        searchModel.filterResults(
                    searchFilterPlayer,
                    searchFilterVersus,
                    searchFilterColour,
                    searchFilterEvent,
                    searchFilterDateFrom,
                    searchFilterDateTo,
                    searchFilterResult)
    }

    function resetSearchResultFilters() {
        searchFilterPlayer = ""
        searchFilterVersus = ""
        searchFilterColour = "either"
        searchFilterEvent = ""
        searchFilterDateFrom = ""
        searchFilterDateTo = ""
        searchFilterResult = "any"

        searchFilterPlayerField.text = ""
        searchFilterVersusField.text = ""
        searchFilterEventField.text = ""
        searchFilterDateFromField.text = ""
        searchFilterDateToField.text = ""

        searchFilterColourBox.currentIndex = 0
        searchFilterResultBox.currentIndex = 0
    }

    function clearSearchResultFilters() {
        resetSearchResultFilters()
        filterSearchResults()
    }

    function resetSearchResultSort() {
        searchSortColumn = "none"
        searchSortAscending = false
    }

    function searchSortHeaderText(column, title) {
        if (searchSortColumn !== column)
            return title + " ↕"

        return title + (searchSortAscending ? " ▲" : " ▼")
    }

    function sortSearchResults(column, firstAscending) {
        if (!searchHasRun || searchModel.search_in_progress)
            return

        selectedSearchRow = -1
        pendingSearchGame = null

        if (searchSortColumn === column) {
            searchSortAscending = !searchSortAscending
        } else {
            searchSortColumn = column
            searchSortAscending = firstAscending
        }

        if (!searchModel.sortResults(
                    column,
                    searchSortAscending)) {
            console.warn(searchModel.error_message)
        }
    }

    function catalogueSortDirectionLabels(column) {
        if (column === "none")
            return [qsTr("Direction")]

        if (column === "date")
            return [qsTr("Oldest first"), qsTr("Newest first")]

        return [qsTr("A → Z"), qsTr("Z → A")]
    }

    function normaliseCatalogueSort() {
        if (sortColumn === "none") {
            sortThenColumn = "none"
            sortThirdColumn = "none"
            return
        }

        if (sortThenColumn === sortColumn)
            sortThenColumn = "none"

        if (sortThenColumn === "none") {
            sortThirdColumn = "none"
            return
        }

        if (sortThirdColumn === sortColumn
                || sortThirdColumn === sortThenColumn) {
            sortThirdColumn = "none"
        }
    }

    function setCatalogueSortColumn(level, column) {
        const firstAscending = column === "date" ? false : true

        if (level === 0) {
            sortColumn = column
            sortAscending = firstAscending
        } else if (level === 1) {
            sortThenColumn = column

            if (column !== "none")
                sortThenAscending = firstAscending
        } else {
            sortThirdColumn = column

            if (column !== "none")
                sortThirdAscending = firstAscending
        }

        normaliseCatalogueSort()
        loadProject()
    }

    function setCatalogueSortDirection(level, ascending) {
        if (level === 0)
            sortAscending = ascending
        else if (level === 1)
            sortThenAscending = ascending
        else
            sortThirdAscending = ascending

        loadProject()
    }

    function sortBy(column, firstAscending) {
        if (sortColumn === column) {
            sortAscending = !sortAscending
        } else {
            sortColumn = column
            sortAscending = firstAscending
        }

        normaliseCatalogueSort()
        loadProject()
    }

    function sortHeaderText(column, title) {
    if (sortColumn !== column)
        return title + " ↕"

    return title + (sortAscending ? " ▲" : " ▼")
}

    Connections {
        target: databaseGameModel

        function onLoadFinished(success) {
            root.databaseProjectLoaded = success
        }
    }

    Connections {
        target: myGamesGameModel

        function onLoadFinished(success) {
            root.myGamesProjectLoaded = success
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
                                    id: loadingStone

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
                                            from: loadingStone.startX
                                            to: loadingStone.endX
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
                                            from: loadingStone.startY
                                            to: boardSurface.y - Kirigami.Units.gridUnit * 0.18
                                            duration: 450
                                            easing.type: Easing.OutQuad
                                        }

                                        NumberAnimation {
                                            to: loadingStone.endY
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
        searchBoardSize = 0
        searchPatternLeft = 0
        searchPatternBottom = 0
        continuationFilterActive = false
        continuationFilterAppearances = 0
        selectedContinuationX = -1
        selectedContinuationCoreY = -1
        continuationCandidates = []
        comparisonCandidateA = null
        comparisonCandidateB = null
        resetSearchResultSort()
        searchModel.clearResults()
    }

    function searchProject(boardSize,
                           stonesJson,
                           left,
                           bottom,
                           width,
                           height,
                           includeHandicapGames) {
        selectedSearchRow = -1
        pendingSearchGame = null
        searchHasRun = true
        searchPatternWidth = width
        searchPatternHeight = height
        searchBoardSize = boardSize
        searchPatternLeft = left
        searchPatternBottom = bottom
        continuationFilterActive = false
        continuationFilterAppearances = 0
        selectedContinuationX = -1
        selectedContinuationCoreY = -1
        continuationCandidates = []
        comparisonCandidateA = null
        comparisonCandidateB = null

        resetSearchResultSort()
        searchModel.clearResults()
        mainTabs.currentIndex = 2

        const started = searchModel.searchProject(
                            projectPath,
                            boardSize,
                            stonesJson,
                            left,
                            bottom,
                            width,
                            height,
                            includeHandicapGames)

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

    function continuationAtOccurrenceIsSelected(
            boardX, coreY, left, bottom, transformation) {
        return searchModel.continuationAtOccurrenceIsSelected(
                    boardX,
                    coreY,
                    left,
                    bottom,
                    transformation)
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

    /*
     * A bound projectPath may change while this component is being
     * constructed.  Do not start a catalogue load at that point and then
     * start the same load again from Component.onCompleted.
     *
     * Besides doing duplicate work, two simultaneous database opens can
     * race when an older database needs a schema migration.
     */
    property bool componentReadyForProjectLoad: false

    onDatabaseProjectPathChanged: {
        databaseProjectLoaded = false

        if (componentReadyForProjectLoad && !showingMyGames)
            ensureCurrentProjectLoaded()
    }

    onMyGamesProjectPathChanged: {
        myGamesProjectLoaded = false

        if (componentReadyForProjectLoad && showingMyGames)
            ensureCurrentProjectLoaded()
    }

    Component.onCompleted: {
        databaseCatalogueState = captureCatalogueState()
        myGamesCatalogueState = captureCatalogueState()

        componentReadyForProjectLoad = true
        ensureCurrentProjectLoaded()
    }

    GameListModel {
        id: databaseGameModel
    }

    GameListModel {
        id: myGamesGameModel
        occurrence_mode: true
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
            id: mainTabs

            onCurrentIndexChanged: {
                if (currentIndex === 0)
                    root.selectCatalogue(false)
                else if (currentIndex === 1)
                    root.selectCatalogue(true)
            }

            TabButton {
                text: qsTr("Game database")
            }

            TabButton {
                text: qsTr("My games")
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
            visible: mainTabs.currentIndex !== 2
            padding: Kirigami.Units.smallSpacing

            contentItem: ColumnLayout {
                spacing: Kirigami.Units.smallSpacing

                RowLayout {
                    Layout.fillWidth: true
                    spacing: Kirigami.Units.smallSpacing

                    Label {
                        text: qsTr("Player")
                    }

                    TextField {
                        id: cataloguePlayerField
                        Layout.fillWidth: true
                        placeholderText: qsTr("Exact player name")
                        text: root.cataloguePlayer
                        onTextChanged: root.cataloguePlayer = text
                        onAccepted: root.applyCatalogueSearch()
                    }

                    ComboBox {
                        id: catalogueColourBox
                        model: [
                            qsTr("Either colour"),
                            qsTr("Black"),
                            qsTr("White")
                        ]

                        currentIndex:
                            root.catalogueColour === "black"
                            ? 1
                            : root.catalogueColour === "white"
                              ? 2
                              : 0

                        onActivated: function(index) {
                            root.catalogueColour =
                                index === 1
                                ? "black"
                                : index === 2
                                  ? "white"
                                  : "either"
                        }
                    }

                    Label {
                        text: qsTr("Versus")
                    }

                    TextField {
                        id: catalogueVersusField
                        Layout.fillWidth: true
                        placeholderText: qsTr("Exact opponent name or blank")
                        text: root.catalogueVersus
                        onTextChanged: root.catalogueVersus = text
                        onAccepted: root.applyCatalogueSearch()
                    }
                }

                RowLayout {
                    Layout.fillWidth: true
                    spacing: Kirigami.Units.smallSpacing

                    Label {
                        text: qsTr("Event")
                    }

                    TextField {
                        id: catalogueEventField
                        Layout.fillWidth: true
                        placeholderText: qsTr("Tournament or event contains…")
                        text: root.catalogueEvent
                        onTextChanged: root.catalogueEvent = text
                        onAccepted: root.applyCatalogueSearch()
                    }

                    Label {
                        text: qsTr("From")
                    }

                    TextField {
                        id: catalogueDateFromField
                        Layout.preferredWidth:
                            Kirigami.Units.gridUnit * 7
                        placeholderText: qsTr("YYYY-MM-DD")
                        text: root.catalogueDateFrom
                        onTextChanged:
                            root.catalogueDateFrom = text
                        onAccepted: root.applyCatalogueSearch()
                    }

                    Label {
                        text: qsTr("To")
                    }

                    TextField {
                        id: catalogueDateToField
                        Layout.preferredWidth:
                            Kirigami.Units.gridUnit * 7
                        placeholderText: qsTr("YYYY-MM-DD")
                        text: root.catalogueDateTo
                        onTextChanged:
                            root.catalogueDateTo = text
                        onAccepted: root.applyCatalogueSearch()
                    }

                    Label {
                        text: qsTr("Result")
                    }

                    ComboBox {
                        id: catalogueResultBox

                        model: [
                            qsTr("Any"),
                            qsTr("Black win"),
                            qsTr("White win"),
                            qsTr("Jigo"),
                            qsTr("Void")
                        ]

                        currentIndex:
                            root.catalogueResult === "black-win"
                            ? 1
                            : root.catalogueResult === "white-win"
                              ? 2
                              : root.catalogueResult === "jigo"
                                ? 3
                                : root.catalogueResult === "void"
                                  ? 4
                                  : 0

                        onActivated: function(index) {
                            root.catalogueResult =
                                index === 1
                                ? "black-win"
                                : index === 2
                                  ? "white-win"
                                  : index === 3
                                    ? "jigo"
                                    : index === 4
                                      ? "void"
                                      : "any"
                        }
                    }

                    Button {
                        text: qsTr("Search")
                        enabled: !root.catalogueLoading

                        onClicked: root.applyCatalogueSearch()
                    }

                    Button {
                        text: qsTr("Clear fields")
                        enabled: !root.catalogueLoading

                        onClicked: {
                            root.cataloguePlayer = ""
                            root.catalogueVersus = ""
                            root.catalogueColour = "either"
                            root.catalogueEvent = ""
                            root.catalogueDateFrom = ""
                            root.catalogueDateTo = ""
                            root.catalogueResult = "any"

                            cataloguePlayerField.text = ""
                            catalogueVersusField.text = ""
                            catalogueEventField.text = ""
                            catalogueDateFromField.text = ""
                            catalogueDateToField.text = ""

                            catalogueColourBox.currentIndex = 0
                            catalogueResultBox.currentIndex = 0
                        }
                    }
                }

                RowLayout {
                    Layout.fillWidth: true
                    spacing: Kirigami.Units.smallSpacing

                    Label {
                        text: qsTr("Order")
                    }

                    ComboBox {
                        Layout.preferredWidth:
                            Kirigami.Units.gridUnit * 6

                        model: [
                            qsTr("None"),
                            qsTr("Date"),
                            qsTr("Black"),
                            qsTr("White")
                        ]

                        currentIndex:
                            root.sortColumn === "date"
                            ? 1
                            : root.sortColumn === "black"
                              ? 2
                              : root.sortColumn === "white"
                                ? 3
                                : 0

                        onActivated: function(index) {
                            root.setCatalogueSortColumn(
                                0,
                                index === 1
                                ? "date"
                                : index === 2
                                  ? "black"
                                  : index === 3
                                    ? "white"
                                    : "none")
                        }
                    }

                    ComboBox {
                        Layout.preferredWidth:
                            Kirigami.Units.gridUnit * 7
                        enabled: root.sortColumn !== "none"
                        model:
                            root.catalogueSortDirectionLabels(
                                root.sortColumn)
                        currentIndex:
                            root.sortColumn === "none"
                            ? 0
                            : root.sortAscending ? 0 : 1

                        onActivated: function(index) {
                            root.setCatalogueSortDirection(
                                0, index === 0)
                        }
                    }

                    Label {
                        text: qsTr("then")
                        enabled: root.sortColumn !== "none"
                    }

                    ComboBox {
                        Layout.preferredWidth:
                            Kirigami.Units.gridUnit * 6
                        enabled: root.sortColumn !== "none"

                        model: [
                            qsTr("None"),
                            qsTr("Date"),
                            qsTr("Black"),
                            qsTr("White")
                        ]

                        currentIndex:
                            root.sortThenColumn === "date"
                            ? 1
                            : root.sortThenColumn === "black"
                              ? 2
                              : root.sortThenColumn === "white"
                                ? 3
                                : 0

                        onActivated: function(index) {
                            root.setCatalogueSortColumn(
                                1,
                                index === 1
                                ? "date"
                                : index === 2
                                  ? "black"
                                  : index === 3
                                    ? "white"
                                    : "none")
                        }
                    }

                    ComboBox {
                        Layout.preferredWidth:
                            Kirigami.Units.gridUnit * 7
                        enabled: root.sortThenColumn !== "none"
                        model:
                            root.catalogueSortDirectionLabels(
                                root.sortThenColumn)
                        currentIndex:
                            root.sortThenColumn === "none"
                            ? 0
                            : root.sortThenAscending ? 0 : 1

                        onActivated: function(index) {
                            root.setCatalogueSortDirection(
                                1, index === 0)
                        }
                    }

                    Label {
                        text: qsTr("then")
                        enabled: root.sortThenColumn !== "none"
                    }

                    ComboBox {
                        Layout.preferredWidth:
                            Kirigami.Units.gridUnit * 6
                        enabled: root.sortThenColumn !== "none"

                        model: [
                            qsTr("None"),
                            qsTr("Date"),
                            qsTr("Black"),
                            qsTr("White")
                        ]

                        currentIndex:
                            root.sortThirdColumn === "date"
                            ? 1
                            : root.sortThirdColumn === "black"
                              ? 2
                              : root.sortThirdColumn === "white"
                                ? 3
                                : 0

                        onActivated: function(index) {
                            root.setCatalogueSortColumn(
                                2,
                                index === 1
                                ? "date"
                                : index === 2
                                  ? "black"
                                  : index === 3
                                    ? "white"
                                    : "none")
                        }
                    }

                    ComboBox {
                        Layout.preferredWidth:
                            Kirigami.Units.gridUnit * 7
                        enabled: root.sortThirdColumn !== "none"
                        model:
                            root.catalogueSortDirectionLabels(
                                root.sortThirdColumn)
                        currentIndex:
                            root.sortThirdColumn === "none"
                            ? 0
                            : root.sortThirdAscending ? 0 : 1

                        onActivated: function(index) {
                            root.setCatalogueSortDirection(
                                2, index === 0)
                        }
                    }

                    Item {
                        Layout.fillWidth: true
                    }
                }

                Label {
                    Layout.fillWidth: true
                    visible:
                        root.catalogueVersus.trim().length > 0
                        && root.cataloguePlayer.trim().length === 0

                    text: qsTr(
                        "Enter a Player as well as Versus to search a match-up.")
                    color: Kirigami.Theme.neutralTextColor
                    wrapMode: Text.WordWrap
                }
            }
        }

        Frame {
            Layout.fillWidth: true
            visible: mainTabs.currentIndex === 2
            enabled: root.searchHasRun && !root.searchInProgress
            padding: Kirigami.Units.smallSpacing

            contentItem: ColumnLayout {
                spacing: Kirigami.Units.smallSpacing

                RowLayout {
                    Layout.fillWidth: true
                    spacing: Kirigami.Units.smallSpacing

                    Label {
                        text: qsTr("Player")
                    }

                    TextField {
                        id: searchFilterPlayerField
                        Layout.fillWidth: true
                        placeholderText: qsTr("Exact player name")
                        text: root.searchFilterPlayer
                        onTextChanged: root.searchFilterPlayer = text
                        onAccepted: root.filterSearchResults()
                    }

                    ComboBox {
                        id: searchFilterColourBox

                        model: [
                            qsTr("Either colour"),
                            qsTr("Black"),
                            qsTr("White")
                        ]

                        currentIndex:
                            root.searchFilterColour === "black"
                            ? 1
                            : root.searchFilterColour === "white"
                              ? 2
                              : 0

                        onActivated: function(index) {
                            root.searchFilterColour =
                                index === 1
                                ? "black"
                                : index === 2
                                  ? "white"
                                  : "either"
                        }
                    }

                    Label {
                        text: qsTr("Versus")
                    }

                    TextField {
                        id: searchFilterVersusField
                        Layout.fillWidth: true
                        placeholderText: qsTr("Exact opponent name or blank")
                        text: root.searchFilterVersus
                        onTextChanged: root.searchFilterVersus = text
                        onAccepted: root.filterSearchResults()
                    }
                }

                RowLayout {
                    Layout.fillWidth: true
                    spacing: Kirigami.Units.smallSpacing

                    Label {
                        text: qsTr("Event")
                    }

                    TextField {
                        id: searchFilterEventField
                        Layout.fillWidth: true
                        placeholderText: qsTr("Tournament or event contains…")
                        text: root.searchFilterEvent
                        onTextChanged: root.searchFilterEvent = text
                        onAccepted: root.filterSearchResults()
                    }

                    Label {
                        text: qsTr("From")
                    }

                    TextField {
                        id: searchFilterDateFromField
                        Layout.preferredWidth:
                            Kirigami.Units.gridUnit * 7
                        placeholderText: qsTr("YYYY-MM-DD")
                        text: root.searchFilterDateFrom
                        onTextChanged:
                            root.searchFilterDateFrom = text
                        onAccepted: root.filterSearchResults()
                    }

                    Label {
                        text: qsTr("To")
                    }

                    TextField {
                        id: searchFilterDateToField
                        Layout.preferredWidth:
                            Kirigami.Units.gridUnit * 7
                        placeholderText: qsTr("YYYY-MM-DD")
                        text: root.searchFilterDateTo
                        onTextChanged:
                            root.searchFilterDateTo = text
                        onAccepted: root.filterSearchResults()
                    }

                    Label {
                        text: qsTr("Result")
                    }

                    ComboBox {
                        id: searchFilterResultBox

                        model: [
                            qsTr("Any"),
                            qsTr("Black win"),
                            qsTr("White win"),
                            qsTr("Jigo"),
                            qsTr("Void")
                        ]

                        currentIndex:
                            root.searchFilterResult === "black-win"
                            ? 1
                            : root.searchFilterResult === "white-win"
                              ? 2
                              : root.searchFilterResult === "jigo"
                                ? 3
                                : root.searchFilterResult === "void"
                                  ? 4
                                  : 0

                        onActivated: function(index) {
                            root.searchFilterResult =
                                index === 1
                                ? "black-win"
                                : index === 2
                                  ? "white-win"
                                  : index === 3
                                    ? "jigo"
                                    : index === 4
                                      ? "void"
                                      : "any"
                        }
                    }

                    Button {
                        text: qsTr("Filter")
                        onClicked: root.filterSearchResults()
                    }

                    Button {
                        text: qsTr("Clear")
                        onClicked: root.clearSearchResultFilters()
                    }
                }

                Label {
                    Layout.fillWidth: true

                    visible:
                        root.searchFilterVersus.trim().length > 0
                        && root.searchFilterPlayer.trim().length === 0

                    text: qsTr(
                        "Enter a Player as well as Versus to search a match-up.")
                    color: Kirigami.Theme.neutralTextColor
                    wrapMode: Text.WordWrap
                }
            }
        }


        RowLayout {
            Layout.fillWidth: true
            visible: mainTabs.currentIndex === 1
            spacing: Kirigami.Units.smallSpacing

            Button {
                text: qsTr("Remove selected game…")
                highlighted: root.removeSelectedMyGameEnabled
                enabled: root.removeSelectedMyGameEnabled

                onClicked: root.removeSelectedMyGameRequested()

                ToolTip.visible: hovered
                ToolTip.text: qsTr("Remove selected game from My games")
            }

            Item {
                Layout.fillWidth: true
            }
        }

        Rectangle {
            Layout.fillWidth: true
            implicitHeight: headerRow.implicitHeight
            visible: mainTabs.currentIndex !== 2
            color: Kirigami.Theme.alternateBackgroundColor

          RowLayout {
    id: headerRow

    anchors.fill: parent
    spacing: 0

    Label {
        id: blackHeader

        text: root.sortHeaderText(
                  root.whiteColumnFirst ? "white" : "black",
                  root.whiteColumnFirst
                      ? qsTr("White")
                      : qsTr("Black"))

        Layout.preferredWidth: root.playerColumnWidth
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

            onClicked: root.sortBy(
                           root.whiteColumnFirst ? "white" : "black",
                           true)
        }
    }

    ToolButton {
        text: qsTr("⇄")
        Layout.preferredWidth: 32
        Layout.maximumWidth: 32

        ToolTip.visible: hovered
        ToolTip.text: qsTr("Swap Black and White columns")

        onClicked:
            root.whiteColumnFirst = !root.whiteColumnFirst
    }

    Label {
        id: whiteHeader

        text: root.sortHeaderText(
                  root.whiteColumnFirst ? "black" : "white",
                  root.whiteColumnFirst
                      ? qsTr("Black")
                      : qsTr("White"))

        Layout.preferredWidth: root.playerColumnWidth
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

            onClicked: root.sortBy(
                           root.whiteColumnFirst ? "black" : "white",
                           true)
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
        text: qsTr("Result")
        Layout.preferredWidth: 80
        padding: Kirigami.Units.smallSpacing
        font.bold: true
    }

    Label {
        text: qsTr("Komi")

        Layout.preferredWidth: 60
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
            visible: mainTabs.currentIndex !== 2
        }

        Rectangle {
            Layout.fillWidth: true
            implicitHeight: searchHeaderRow.implicitHeight
            visible: mainTabs.currentIndex === 2
            color: Kirigami.Theme.alternateBackgroundColor

            RowLayout {
                id: searchHeaderRow

                anchors.fill: parent
                spacing: 0

                Label {
                    id: searchFirstPlayerHeader

                    text: root.searchSortHeaderText(
                              root.whiteColumnFirst
                              ? "white"
                              : "black",
                              root.whiteColumnFirst
                              ? qsTr("White")
                              : qsTr("Black"))

                    Layout.preferredWidth: root.playerColumnWidth
                    padding: Kirigami.Units.smallSpacing
                    font.bold: true

                    color: searchFirstPlayerSortArea.containsMouse
                           ? Kirigami.Theme.highlightColor
                           : Kirigami.Theme.textColor

                    MouseArea {
                        id: searchFirstPlayerSortArea

                        anchors.fill: parent
                        hoverEnabled: true
                        enabled:
                            root.searchHasRun
                            && !searchModel.search_in_progress
                        cursorShape: Qt.PointingHandCursor

                        onClicked: root.sortSearchResults(
                                       root.whiteColumnFirst
                                       ? "white"
                                       : "black",
                                       true)
                    }
                }

                ToolButton {
                    text: qsTr("⇄")
                    Layout.preferredWidth: 32
                    Layout.maximumWidth: 32

                    ToolTip.visible: hovered
                    ToolTip.text: qsTr("Swap Black and White columns")

                    onClicked:
                        root.whiteColumnFirst = !root.whiteColumnFirst
                }

                Label {
                    id: searchSecondPlayerHeader

                    text: root.searchSortHeaderText(
                              root.whiteColumnFirst
                              ? "black"
                              : "white",
                              root.whiteColumnFirst
                              ? qsTr("Black")
                              : qsTr("White"))

                    Layout.preferredWidth: root.playerColumnWidth
                    padding: Kirigami.Units.smallSpacing
                    font.bold: true

                    color: searchSecondPlayerSortArea.containsMouse
                           ? Kirigami.Theme.highlightColor
                           : Kirigami.Theme.textColor

                    MouseArea {
                        id: searchSecondPlayerSortArea

                        anchors.fill: parent
                        hoverEnabled: true
                        enabled:
                            root.searchHasRun
                            && !searchModel.search_in_progress
                        cursorShape: Qt.PointingHandCursor

                        onClicked: root.sortSearchResults(
                                       root.whiteColumnFirst
                                       ? "black"
                                       : "white",
                                       true)
                    }
                }

                Label {
                    id: searchDateHeader

                    text: root.searchSortHeaderText(
                              "date",
                              qsTr("Date"))

                    Layout.preferredWidth: 105
                    padding: Kirigami.Units.smallSpacing
                    font.bold: true

                    color: searchDateSortArea.containsMouse
                           ? Kirigami.Theme.highlightColor
                           : Kirigami.Theme.textColor

                    MouseArea {
                        id: searchDateSortArea

                        anchors.fill: parent
                        hoverEnabled: true
                        enabled:
                            root.searchHasRun
                            && !searchModel.search_in_progress
                        cursorShape: Qt.PointingHandCursor

                        onClicked:
                            root.sortSearchResults("date", false)
                    }
                }

                Label {
                    text: qsTr("Result")
                    Layout.preferredWidth: 80
                    padding: Kirigami.Units.smallSpacing
                    font.bold: true
                }

                Label {
                    id: searchMatchesHeader

                    text: root.searchSortHeaderText(
                              "matches",
                              qsTr("Matches"))

                    Layout.preferredWidth: 80
                    padding: Kirigami.Units.smallSpacing
                    horizontalAlignment: Text.AlignHCenter
                    font.bold: true

                    color: searchMatchesSortArea.containsMouse
                           ? Kirigami.Theme.highlightColor
                           : Kirigami.Theme.textColor

                    MouseArea {
                        id: searchMatchesSortArea

                        anchors.fill: parent
                        hoverEnabled: true
                        enabled:
                            root.searchHasRun
                            && !searchModel.search_in_progress
                        cursorShape: Qt.PointingHandCursor

                        onClicked:
                            root.sortSearchResults("matches", false)
                    }
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
            visible: mainTabs.currentIndex === 2
        }

        ListView {
            id: searchView

            Layout.fillWidth: true
            Layout.fillHeight: true

            visible: mainTabs.currentIndex === 2

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
                        text: root.whiteColumnFirst
                              ? searchRowDelegate.whitePlayer
                              : searchRowDelegate.blackPlayer
                        Layout.preferredWidth: root.playerColumnWidth
                        elide: Text.ElideRight
                        leftPadding: Kirigami.Units.smallSpacing
                    }

                    Item {
                        Layout.preferredWidth: 32
                    }

                    Label {
                        text: root.whiteColumnFirst
                              ? searchRowDelegate.blackPlayer
                              : searchRowDelegate.whitePlayer
                        Layout.preferredWidth: root.playerColumnWidth
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

            visible: mainTabs.currentIndex !== 2

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
                required property var gameSourceId
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
                        gameSourceId: gameSourceId,
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
                        text: root.whiteColumnFirst
                              ? rowDelegate.whitePlayer
                              : rowDelegate.blackPlayer
                        Layout.preferredWidth: root.playerColumnWidth
                        elide: Text.ElideRight
                        leftPadding: Kirigami.Units.smallSpacing
                    }

                    Item {
                        Layout.preferredWidth: 32
                    }

                    Label {
                        text: root.whiteColumnFirst
                              ? rowDelegate.blackPlayer
                              : rowDelegate.whitePlayer
                        Layout.preferredWidth: root.playerColumnWidth
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
