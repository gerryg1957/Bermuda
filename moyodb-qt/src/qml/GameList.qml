import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
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
    padding: 0



        function loadProject() {
        selectedRow = -1

        if (projectPath.length === 0) {
            projectLoaded = false
            return
        }

        projectLoaded = gameModel.loadSortedProject(
                    projectPath,
                    sortColumn,
                    sortAscending)
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

        if (points === undefined || points === null) {
            continuationCandidates = candidates
            return
        }

        for (const point of points) {
            const gameCount =
                searchModel.continuationGameCountAtOccurrence(
                    point.x,
                    point.coreY,
                    left,
                    bottom,
                    transformation)

            candidates.push({
                "x": point.x,
                "coreY": point.coreY,
                "count": Number(point.count),
                "gameCount": gameCount,
                "coordinate": goCoordinate(point.x, point.coreY)
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

    Item {
        Layout.fillWidth: true
        Layout.preferredHeight: root.searchResultsSelected
                                ? Kirigami.Units.gridUnit * 7
                                : 0

        visible: root.searchResultsSelected

          Label {
           anchors.fill: parent
           anchors.margins: Kirigami.Units.largeSpacing

           horizontalAlignment: Text.AlignHCenter
           verticalAlignment: Text.AlignVCenter
           wrapMode: Text.WordWrap

           text: {
               if (searchModel.search_in_progress) {
                   return qsTr(
                               "Searching the database…\n"
                               + "%1 of %2 games examined")
                       .arg(searchModel.games_examined)
                       .arg(searchModel.total_games)
               }

               const distribution =
                   root.nextMoveDistribution

               if (distribution === null) {
                   return qsTr(
                               "Run a pattern search to build "
                               + "a continuation map")
               }

               if (root.continuationFilterActive) {
                   return qsTr(
                               "Continuation map\n"
                               + "Selected continuation: "
                               + "%1 appearances in %2 games\n"
                               + "The search results below show the "
                               + "supporting games.")
                       .arg(root.continuationFilterAppearances)
                       .arg(searchView.count)
               }

               return qsTr(
                           "Continuation map\n"
                           + "%1 appearances in %2 games · "
                           + "%3 shown locally · "
                           + "%4 outside area · "
                           + "%5 passes · "
                           + "%6 games ended\n"
                           + "Larger circles indicate more frequently "
                           + "played immediate continuations.")
                   .arg(distribution.appearances)
                   .arg(distribution.matchingGames)
                   .arg(root.nextMoveLocalCount)
                   .arg(distribution.outsideDisplayedArea)
                   .arg(distribution.passes)
                   .arg(distribution.gameEnded)
           }

           opacity: root.nextMoveDistribution === null
                    && !searchModel.search_in_progress
                    ? 0.55
                    : 0.80

           font.italic:
               root.nextMoveDistribution === null
               && !searchModel.search_in_progress
       }


        ToolButton {
            anchors.right: parent.right
            anchors.bottom: parent.bottom
            anchors.rightMargin: Kirigami.Units.largeSpacing
            anchors.bottomMargin: Kirigami.Units.smallSpacing
            visible: root.continuationFilterActive
            text: qsTr("Show all continuations")
            onClicked: root.clearContinuationFilter()
        }
}

    Frame {
        Layout.fillWidth: true
        Layout.preferredHeight: visible
            ? Math.min(Kirigami.Units.gridUnit * 8,
                       Kirigami.Units.gridUnit
                       * (2.2 + root.continuationCandidates.length * 1.55))
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
                leftPadding: Kirigami.Units.smallSpacing
            }

            ListView {
                id: continuationCandidateView

                Layout.fillWidth: true
                Layout.fillHeight: true

                clip: true
                model: root.continuationCandidates

                ScrollBar.vertical: ScrollBar {}

                delegate: ItemDelegate {
                    required property var modelData

                    width: continuationCandidateView.width
                    height: Math.round(Kirigami.Units.gridUnit * 1.45)

                    highlighted:
                        root.continuationFilterActive
                        && root.selectedContinuationX === modelData.x
                        && root.selectedContinuationCoreY === modelData.coreY

                    onClicked: {
                        root.continuationCandidateSelected(
                                    modelData.x,
                                    modelData.coreY,
                                    modelData.count)
                    }

                    contentItem: RowLayout {
                        spacing: Kirigami.Units.smallSpacing

                        Label {
                            text: modelData.coordinate
                            font.bold: true
                            Layout.preferredWidth: Kirigami.Units.gridUnit * 3
                            horizontalAlignment: Text.AlignHCenter
                        }

                        Label {
                            text: qsTr("%1 appearances")
                                      .arg(modelData.count)
                            Layout.preferredWidth: Kirigami.Units.gridUnit * 7
                        }

                        Label {
                            text: qsTr("%1 games")
                                      .arg(modelData.gameCount)
                            Layout.fillWidth: true
                        }
                    }
                }
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
