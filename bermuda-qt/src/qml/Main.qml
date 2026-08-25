import QtCore
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import QtQuick.Dialogs
import org.kde.kirigami as Kirigami
import org.bermuda.app

ApplicationWindow {
    id: root

    visible: true
    title: qsTr("Bermuda")

    // Initial size only. The user can resize or maximize normally.
       width: 1500
    height: 850

    minimumWidth: 900
    minimumHeight: 600
       function localPathFromUrl(url) {
        const text = url.toString()

        if (text.length === 0)
            return ""

        const parsed = new URL(text)
        let path = decodeURIComponent(parsed.pathname)

        /*
         * URL paths for Windows drive letters conventionally begin
         * with '/', for example /C:/Users/...
         */
        if (Qt.platform.os === "windows"
                && path.length >= 3
                && path.charAt(0) === "/"
                && path.charAt(2) === ":") {
            path = path.substring(1)
        }

        return path
    }

    function managedProjectPathForDirectory(directoryName) {
        const location =
            StandardPaths.writableLocation(
                StandardPaths.GenericDataLocation)

        const basePath = localPathFromUrl(location)

        if (basePath.length === 0)
            return ""

        return basePath.replace(/\/+$/, "")
                + "/" + directoryName + "/games-database"
    }

    /*
     * New installations use the Bermuda application-data directory.
     * The former MoyoDB location remains recognised so existing managed
     * databases continue to open without an implicit filesystem move.
     */
    readonly property string managedProjectPath:
        managedProjectPathForDirectory("Bermuda")

    readonly property string legacyManagedProjectPath:
        managedProjectPathForDirectory("MoyoDB")

    function isManagedProjectPath(path) {
        return path === root.managedProjectPath
                || path === root.legacyManagedProjectPath
    }

    property string projectPath: Qt.application.arguments.length > 1
        ? Qt.application.arguments[1]
        : ""

    BermudaApp {
        id: gameController
    }

    function clearProjectSelection() {
        gameList.clearSearchResults()
        boardPane.clearMatchNavigation()
        boardPane.resetPatternSelection()

        boardPane.editingPosition = false
        boardPane.selectedGame = null

        goBoard.stones = []
        goBoard.lastMoveX = -1
        goBoard.lastMoveY = -1
        goBoard.lastMoveNumber = 0
    }

    function applyFinishedDatabaseOperation() {
        const operation = databaseOperation.operation_name
        const resultPath =
            databaseOperation.result_project_path

        if (operation === "create-database"
                && resultPath.length > 0) {
            clearProjectSelection()
            root.projectPath = resultPath
            return
        }

        if (operation === "add-games") {
            clearProjectSelection()
            gameList.loadProject()
        }
    }

    DatabaseOperationModel {
        id: databaseOperation

        onStageChanged: {
            if (stage === "complete"
                    || stage === "cancelled") {
                root.applyFinishedDatabaseOperation()
            }
        }
    }

    DatabaseImportDialog {
        id: databaseImportDialog

        operationModel: databaseOperation
        currentProjectPath: root.projectPath

        onOperationStarted:
            databaseProgressDialog.open()
    }

    DatabaseProgressDialog {
        id: databaseProgressDialog
        operationModel: databaseOperation
    }

    PlayerIdentityDialog {
        id: playerIdentityDialog

        onIdentitiesChanged: {
            /*
             * Existing catalogue/search rows contain presentation metadata
             * captured before the identity edit. Refresh the catalogue and
             * discard stale pattern-result presentation rows.
             */
            gameList.clearSearchResults()
            gameList.loadProject()
        }
    }

    AboutDialog {
        id: aboutDialog
    }

  FolderDialog {
      id: openDatabaseDialog

      title: qsTr("Open Database")

      onAccepted: {
          const folderPath =
              root.localPathFromUrl(selectedFolder)

          root.clearProjectSelection()
          root.projectPath = folderPath
      }
  }

  FileDialog {
    id: openSgfDialog

    title: qsTr("Open SGF")
    fileMode: FileDialog.OpenFile

    nameFilters: [
        qsTr("SGF files (*.sgf)"),
        qsTr("All files (*)")
    ]

    onAccepted: {
        const fileUrl = new URL(selectedFile)
        const filePath = decodeURIComponent(fileUrl.pathname)
        const fileName = filePath.substring(
                           filePath.lastIndexOf("/") + 1)

        if (gameController.loadSgf(filePath)) {
            gameList.clearSearchResults()
            boardPane.clearMatchNavigation()
            boardPane.resetPatternSelection()

            boardPane.editingPosition = false

            const blackPlayer = gameController.black_player
            const whitePlayer = gameController.white_player
            const hasPlayerData = blackPlayer.length > 0
                                  || whitePlayer.length > 0

            boardPane.selectedGame = {
                gameId: -1,
                black: hasPlayerData
                       ? qsTr("(B) %1").arg(
                             blackPlayer.length > 0 ? blackPlayer : qsTr("Black"))
                       : qsTr("External SGF"),
                white: hasPlayerData
                       ? qsTr("(W) %1").arg(
                             whitePlayer.length > 0 ? whitePlayer : qsTr("White"))
                       : fileName,
                gameDate: "",
                result: "",
                eventName: "",
                komi: gameController.komi
            }

            boardPane.applyLoadedPosition()
        } else {
            boardPane.selectedGame = null

            goBoard.stones = []
            goBoard.lastMoveX = -1
            goBoard.lastMoveY = -1
            goBoard.lastMoveNumber = 0

            console.warn(gameController.error_message)
        }
    }
}

menuBar: MenuBar {
    Menu {
        title: qsTr("&File")

        Action {
            text: qsTr("&New Position")

            onTriggered: {
                if (gameController.newPosition(19)) {
                    gameList.clearSearchResults()
                    boardPane.clearMatchNavigation()
                    boardPane.resetPatternSelection()
                    boardPane.editingPosition = true
                    boardPane.editTool = "black"

                    boardPane.selectedGame = {
                        gameId: -1,
                        black: qsTr("Untitled position"),
                        white: "",
                        gameDate: "",
                        result: "",
                        eventName: "",
                        komi: ""
                    }

                    boardPane.applyLoadedPosition()
                } else {
                    boardPane.editingPosition = false
                    boardPane.selectedGame = null

                    goBoard.stones = []
                    goBoard.lastMoveX = -1
                    goBoard.lastMoveY = -1
                    goBoard.lastMoveNumber = 0

                    console.warn(gameController.error_message)
                }
            }
        }

        Action {
            text: qsTr("&Open SGF…")

            onTriggered: openSgfDialog.open()
        }
    }

    Menu {
        title: qsTr("&Database")

        Action {
            text: qsTr("&Add Games…")

            enabled: root.projectPath.length > 0
                     && !databaseOperation.in_progress

            onTriggered: {
                if (root.isManagedProjectPath(root.projectPath)) {
                    databaseImportDialog.openManagedAdd(
                        root.projectPath)
                } else {
                    databaseImportDialog.openAdd(
                        root.projectPath)
                }
            }
        }

        Action {
            text: qsTr("Player &Names…")

            enabled: root.projectPath.length > 0
                     && !databaseOperation.in_progress

            onTriggered:
                playerIdentityDialog.openForProject(root.projectPath)
        }

        MenuSeparator {}

        Action {
            text: qsTr("&Show Current Operation")

            enabled: databaseOperation.in_progress
                     || databaseOperation.stage.length > 0

            onTriggered:
                databaseProgressDialog.open()
        }

        Action {
            text: qsTr("&Cancel Current Operation")

            enabled: databaseOperation.in_progress
                     && !databaseOperation.cancel_requested

            onTriggered: {
                databaseProgressDialog.open()
                databaseOperation.cancelOperation()
            }
        }

        MenuSeparator {}

        Menu {
            title: qsTr("&Advanced")

            Action {
                text: qsTr("&Open Another Database…")

                enabled: !databaseOperation.in_progress

                onTriggered: openDatabaseDialog.open()
            }

            Action {
                text: qsTr("&Create Another Database…")

                enabled: !databaseOperation.in_progress

                onTriggered:
                    databaseImportDialog.openCreate()
            }

            Action {
                text: qsTr("&Update Position Index")

                enabled: root.projectPath.length > 0
                         && !databaseOperation.in_progress

                onTriggered: {
                    databaseOperation.clearStatus()

                    if (databaseOperation.updatePositionIndex(
                                root.projectPath)) {
                        databaseProgressDialog.open()
                    } else {
                        databaseProgressDialog.open()
                    }
                }
            }
        }
    }
    Menu {
        title: qsTr("&Settings")

        Action {
            text: qsTr("Include &handicap games in pattern searches")
            checkable: true
            checked: root.includeHandicapGames

            onToggled:
                root.includeHandicapGames = checked
        }
    }

    Menu {
        title: qsTr("&Help")

        Action {
            text: qsTr("&About Bermuda")

            onTriggered: aboutDialog.open()
        }
    }

}

    property bool includeHandicapGames: false

    Settings {
        id: uiSettings

        location: StandardPaths.writableLocation(
                      StandardPaths.ConfigLocation)
                  + "/moyodb.ini"

        category: "MainWindow"

        property alias windowWidth: root.width
        property alias windowHeight: root.height
        property var splitViewState
    }

    Component.onCompleted: {
        if (uiSettings.splitViewState) {
            mainSplitView.restoreState(uiSettings.splitViewState)
        }

        if (root.projectPath.length === 0
                && root.managedProjectPath.length > 0) {
            if (gameController.projectExists(
                    root.managedProjectPath)) {
                root.projectPath = root.managedProjectPath
            } else if (root.legacyManagedProjectPath.length > 0
                    && gameController.projectExists(
                        root.legacyManagedProjectPath)) {
                /*
                 * Compatibility with managed databases created before
                 * the application was renamed from MoyoDB to Bermuda.
                 */
                root.projectPath = root.legacyManagedProjectPath
            } else {
                databaseImportDialog.openManagedCreate(
                    root.managedProjectPath)
            }
        }
    }

    Component.onDestruction: {
        uiSettings.splitViewState = mainSplitView.saveState()
    }

    SplitView {
        id: mainSplitView

        anchors {
            fill: parent
            margins: 6
        }

        orientation: Qt.Horizontal

        // Database browser pane
        GameList {
            id: gameList

            projectPath: root.projectPath

            onContinuationCandidateSelected:
                function(boardX, coreY, count) {
                    boardPane.filterContinuationPoint(
                                boardX,
                                coreY,
                                count)
                }

            onContinuationFilterCleared: {
                goBoard.selectedContinuationX = -1
                goBoard.selectedContinuationY = -1
            }

            onSourceContinuationMapReady: function(points) {
                goBoard.selectedContinuationX = -1
                goBoard.selectedContinuationY = -1

                goBoard.continuationPoints =
                    points.map(function(point) {
                        return {
                            "x": point.x,
                            "y": goBoard.boardSize - 1 - point.coreY,
                            "count": point.count
                        }
                    })
            }

                       onGameSelected: function(game) {
                if (!game.fromSearchResults)
                    gameList.clearSearchResults()

                boardPane.clearMatchNavigation()
                boardPane.resetPatternSelection()
                boardPane.editingPosition = false
                boardPane.selectedGame = game

                if (gameController.loadGame(
                            root.projectPath,
                            game.gameId)) {
                    if (game.fromSearchResults
                            && game.matchOccurrences !== undefined
                            && game.matchOccurrences.length > 0) {
                        boardPane.matchOccurrences =
                            game.matchOccurrences

                        boardPane.matchWidth = game.matchWidth
                        boardPane.matchHeight = game.matchHeight

                        const preferredMatchIndex =
                            game.preferredMatchIndex === undefined
                            ? 0
                            : Number(game.preferredMatchIndex)

                        boardPane.showMatch(
                                    Math.max(
                                        0,
                                        Math.min(
                                            preferredMatchIndex,
                                            game.matchOccurrences.length - 1)))
                    } else {
                        boardPane.applyLoadedPosition()
                    }
                } else {
                    goBoard.stones = []
                    goBoard.lastMoveX = -1
                    goBoard.lastMoveY = -1
                    goBoard.lastMoveNumber = 0
                    console.warn(gameController.error_message)
                }
            }

            SplitView.minimumWidth: 420
            SplitView.preferredWidth: 820
            SplitView.fillWidth: true
        }

        // Board and game-details pane
                Pane {
            id: boardPane

            property var selectedGame: null

            property bool editingPosition: false

            property string editTool: "black"
            property bool selectingPattern: false
            property int patternLeft: -1
            property int patternTop: -1
            property int patternRight: -1
            property int patternBottom: -1

            property var matchOccurrences: []
            property int matchIndex: -1
            property int matchWidth: 0
            property int matchHeight: 0
            property bool showingMatchPosition: false

            property var searchSourceGame: null
            property bool searchSourceEditingPosition: false
            property var searchSourceViewTransform: null

            property bool comparingContinuations: false
            property string comparisonStep: "A"

            readonly property bool investigatingSearch:
                gameList.searchHasRun && !gameList.searchInProgress

            readonly property bool showingContinuationComparison:
                gameList.comparisonCandidateA !== null
                && gameList.comparisonCandidateB !== null

            function beginSearchSession() {
                if (!gameController.snapshotSearchSource()) {
                    console.warn(gameController.error_message)
                    return false
                }

                searchSourceGame = selectedGame
                searchSourceEditingPosition = editingPosition
                searchSourceViewTransform =
                    goBoard.currentViewTransform()
                return true
            }

            function beginContinuationComparison() {
                if (gameList.continuationCandidates.length < 2)
                    return

                if (gameList.continuationFilterActive)
                    gameList.clearContinuationFilter()

                gameList.comparisonCandidateA = null
                gameList.comparisonCandidateB = null

                comparingContinuations = true
                comparisonStep = "A"
            }

            function cancelContinuationComparison() {
                comparingContinuations = false
                comparisonStep = "A"
                gameList.comparisonCandidateA = null
                gameList.comparisonCandidateB = null
            }

            function chooseComparisonContinuation(boardX, coreY) {
                let candidate = null

                for (const current of gameList.continuationCandidates) {
                    if (current.x === boardX
                            && current.coreY === coreY) {
                        candidate = current
                        break
                    }
                }

                if (candidate === null)
                    return false

                if (comparisonStep === "A") {
                    gameList.selectComparisonCandidate("A", candidate)
                    comparisonStep = "B"
                    return true
                }

                if (gameList.sameContinuationCandidate(
                            gameList.comparisonCandidateA,
                            candidate)) {
                    return true
                }

                gameList.selectComparisonCandidate("B", candidate)

                comparingContinuations = false
                comparisonStep = "A"
                return true
            }

            function beginNewSearch() {
                comparingContinuations = false
                comparisonStep = "A"

                if (!gameController.restoreSearchSource()) {
                    console.warn(gameController.error_message)
                    return
                }

                selectedGame = searchSourceGame
                editingPosition = searchSourceEditingPosition
                searchSourceGame = null
                searchSourceEditingPosition = false

                applyLoadedPosition()

                if (searchSourceViewTransform !== null)
                    goBoard.setViewTransform(
                                searchSourceViewTransform)

                searchSourceViewTransform = null

                gameList.clearSearchResults()
                resetPatternSelection()
            }

            function clearPatternSelection() {
                selectingPattern = false
                clearMatchNavigation()

                patternLeft = -1
                patternTop = -1
                patternRight = -1
                patternBottom = -1

                goBoard.clearPatternSelection()
            }

            function resetPatternSelection() {
                selectingPattern = false
                clearPatternSelection()
            }

            function clearContinuationMap() {
                showingMatchPosition = false
                comparingContinuations = false
                comparisonStep = "A"
                goBoard.continuationPoints = []
                goBoard.selectedContinuationX = -1
                goBoard.selectedContinuationY = -1
                gameList.clearContinuationCandidates()
            }

            function clearMatchNavigation() {
                matchOccurrences = []
                matchIndex = -1
                matchWidth = 0
                matchHeight = 0

                clearContinuationMap()
            }

            function matchSpanText(occurrence) {
                const firstMove = Number(occurrence.move)
                const lastMove = occurrence.lastMove === undefined
                    ? firstMove
                    : Number(occurrence.lastMove)
                const duration = occurrence.durationMoves === undefined
                    ? Math.max(0, lastMove - firstMove)
                    : Number(occurrence.durationMoves)

                const durationText = duration === 1
                    ? qsTr("duration 1 move")
                    : qsTr("duration %1 moves").arg(duration)

                if (lastMove === firstMove) {
                    return qsTr("after move %1 · %2")
                        .arg(firstMove)
                        .arg(durationText)
                }

                return qsTr("after moves %1–%2 · %3")
                    .arg(firstMove)
                    .arg(lastMove)
                    .arg(durationText)
            }

            function filterContinuationPoint(boardX,
                                                 coreY,
                                                 count) {
                if (comparingContinuations
                        && chooseComparisonContinuation(
                            boardX,
                            coreY)) {
                    return
                }

                const visualY = goBoard.boardSize - 1 - coreY

                let left
                let bottom
                let transformation

                if (matchIndex >= 0
                        && matchIndex < matchOccurrences.length) {
                    const occurrence = matchOccurrences[matchIndex]

                    left = occurrence.left
                    bottom = occurrence.bottom
                    transformation = occurrence.transformation
                } else if (gameList.searchHasRun) {
                    /*
                     * The source continuation map is displayed in the
                     * original identity orientation.
                     */
                    left = gameList.searchPatternLeft
                    bottom = gameList.searchPatternBottom
                    transformation = "identity"
                } else {
                    return
                }

                /*
                 * A continuation's identity is its normalised search
                 * coordinate, not the physical intersection on this
                 * particular transformed occurrence.
                 */
                if (gameList.continuationFilterActive
                        && gameList.continuationAtOccurrenceIsSelected(
                            boardX,
                            coreY,
                            left,
                            bottom,
                            transformation)) {
                    gameList.clearContinuationFilter()
                    return
                }

                if (!gameList.filterContinuationAtOccurrence(
                            boardX,
                            coreY,
                            left,
                            bottom,
                            transformation,
                            count)) {
                    console.warn(
                                "Could not filter continuation results")
                    return
                }

                goBoard.selectedContinuationX = boardX
                goBoard.selectedContinuationY = visualY
            }

            function showMatch(index) {
                if (index < 0 || index >= matchOccurrences.length)
                    return

                const occurrence = matchOccurrences[index]

                if (!gameController.showPosition(occurrence.move)) {
                    console.warn(gameController.error_message)
                    return
                }

                applyLoadedPosition()
                matchIndex = index
                showingMatchPosition = true

                const swapsDimensions =
                    occurrence.transformation === "rotate90Clockwise"
                    || occurrence.transformation === "rotate270Clockwise"
                    || occurrence.transformation === "mirrorMainDiagonal"
                    || occurrence.transformation === "mirrorAntiDiagonal"

                const width =
                    swapsDimensions ? matchHeight : matchWidth

                const height =
                    swapsDimensions ? matchWidth : matchHeight

                const left = occurrence.left
                const right = left + width - 1

                const bottom =
                    goBoard.boardSize - 1 - occurrence.bottom

                const top = bottom - height + 1

                patternLeft = left
                patternTop = top
                patternRight = right
                patternBottom = bottom

                goBoard.setPatternSelection(
                            left,
                            top,
                            right,
                            bottom)

                const continuationPoints =
                    occurrence.continuationPoints === undefined
                    ? []
                    : occurrence.continuationPoints

                goBoard.continuationPoints =
                    continuationPoints.map(function(point) {
                        return {
                            "x": point.x,
                            "y": goBoard.boardSize
                                 - 1
                                 - point.coreY,
                            "count": point.count
                        }
                    })

                gameList.setContinuationCandidates(
                            continuationPoints,
                            goBoard.boardSize,
                            occurrence.left,
                            occurrence.bottom,
                            occurrence.transformation)
            }

            ReplyInfluenceAnalysis {
                id: replyInfluenceAnalysis

                board: goBoard
                controller: gameController
            }

            function applyLoadedPosition() {
                goBoard.boardSize = gameController.board_size
                goBoard.stones = JSON.parse(
                            gameController.stones_json)

                goBoard.lastMoveX = gameController.last_move_x
                goBoard.lastMoveY = gameController.last_move_y
                goBoard.lastMoveNumber = gameController.move_number

                /*
                 * Temporary regression harness for reply-aware influence.
                 * The analysis implementation itself lives in
                 * ReplyInfluenceAnalysis.qml.
                 */
                if (gameController.move_number === 92) {
                    function goLabel(x, y) {
                        const letters =
                            "ABCDEFGHJKLMNOPQRST"

                        return letters.charAt(x)
                            + String(goBoard.boardSize - y)
                    }

                    function logAnalysis(
                            label,
                            x,
                            y,
                            firstColour,
                            replyColour) {
                        const result =
                            replyInfluenceAnalysis.analyse(
                                gameController.move_number,
                                x,
                                y,
                                firstColour,
                                replyColour)

                        if (!result.legal) {
                            console.log(
                                "Reply-aware influence:",
                                label,
                                "illegal:",
                                result.error)
                            return
                        }

                        const bestReply =
                            result.bestReplyX >= 0
                            ? goLabel(
                                  result.bestReplyX,
                                  result.bestReplyY)
                            : "none"

                        console.log(
                            "Reply-aware influence:",
                            label,
                            "bestReply=" + bestReply,
                            "firstEffect="
                                + result.firstEffect.toFixed(4),
                            "remaining="
                                + result.remainingEffect.toFixed(4),
                            "persistence="
                                + (100.0 * result.persistence)
                                  .toFixed(1) + "%",
                            "legalReplies="
                                + result.legalReplies)
                    }

                    logAnalysis(
                        "J19",
                        8,
                        0,
                        "black",
                        "white")

                    logAnalysis(
                        "S12",
                        17,
                        7,
                        "white",
                        "black")

                    logAnalysis(
                        "D11",
                        3,
                        8,
                        "black",
                        "white")

                    logAnalysis(
                        "K2",
                        9,
                        17,
                        "black",
                        "white")
                }
            }

            function showMove(moveNumber) {
                if (!selectedGame) {
                    return
                }

                /*
                 * Preserve the known match locations while replaying
                 * the surrounding game. Only the continuation map is
                 * specific to the matched position.
                 */
                clearContinuationMap()

                if (gameController.showPosition(moveNumber)) {
                    applyLoadedPosition()
                } else {
                    goBoard.stones = []
                    goBoard.lastMoveX = -1
                    goBoard.lastMoveY = -1
                    goBoard.lastMoveNumber = 0
                    console.warn(gameController.error_message)
                }
            }

            padding: 0




            SplitView.minimumWidth: 420
            SplitView.preferredWidth: 640

            ColumnLayout {
                anchors.fill: parent
                spacing: 6

                Frame {
                    id: boardFrame

                    Layout.fillWidth: true
                    Layout.fillHeight: true

                    /*
                     * Prefer a full-width square goban when vertical space
                     * permits, but allow the board area to shrink so that the
                     * controls and game information below it remain visible.
                     */
                    Layout.preferredHeight: width
                    Layout.minimumHeight: Kirigami.Units.gridUnit * 8
                    Layout.maximumHeight: width

                    padding: 4

                    GoBoard {
                        id: goBoard

                        anchors.centerIn: parent

                        width: Math.min(
                            boardFrame.availableWidth,
                            boardFrame.availableHeight)
                        height: width

                          patternSelectionEnabled: boardPane.selectingPattern

                          onContinuationPointClicked: function(x, y, count) {
                              boardPane.filterContinuationPoint(
                                          x,
                                          goBoard.boardSize - 1 - y,
                                          count)
                          }


                          onPointClicked: function(x, y) {
                              if (!boardPane.editingPosition)
                                  return

                              if (gameController.editPositionPoint(
                                          x,
                                          y,
                                          boardPane.editTool)) {
                                  gameList.clearSearchResults()
                                  boardPane.applyLoadedPosition()
                              } else {
                                  console.warn(
                                              gameController.error_message)
                              }
                          }

                          onPatternSelected: function(left, top, right, bottom) {
                              boardPane.clearMatchNavigation()

                              boardPane.patternLeft = left
                              boardPane.patternTop = top
                              boardPane.patternRight = right
                              boardPane.patternBottom = bottom

                              boardPane.selectingPattern = false
                          }
                      }
                  }

                  RowLayout {
                      Layout.fillWidth: true
                      Layout.leftMargin: 8
                      Layout.rightMargin: 8
                      spacing: 4

                      ToolButton {
                          visible: !gameList.searchHasRun
                          text: qsTr("Select Pattern")
                          checkable: true

                              checked: boardPane.selectingPattern

                          onToggled: {
                              boardPane.selectingPattern = checked

                              if (checked) {
                                  boardPane.clearMatchNavigation()
                                  gameList.clearSearchResults()
                                  goBoard.hoverValid = false
                              }
                          }
                      }

                      ToolButton {
                          visible: !gameList.searchHasRun
                          text: qsTr("Clear Selection")
                          enabled: goBoard.patternSelectionValid

                          onClicked: boardPane.clearPatternSelection()
                      }

                      ToolButton {
                          visible: !gameList.searchHasRun
                          text: qsTr("Search Database")

                          enabled: goBoard.patternSelectionValid
                                   && boardPane.selectedGame !== null
                                   && root.projectPath.length > 0
                                   && !gameList.searchInProgress

                          onClicked: {
                              if (!boardPane.beginSearchSession())
                                  return

                              const width =
                                  boardPane.patternRight
                                  - boardPane.patternLeft + 1

                              const height =
                                  boardPane.patternBottom
                                  - boardPane.patternTop + 1

                              const bottom =
                                  goBoard.boardSize - 1
                                  - boardPane.patternBottom

                              goBoard.continuationPoints = []

                              gameList.searchProject(
                                  gameController.board_size,
                                  gameController.stones_json,
                                  boardPane.patternLeft,
                                  bottom,
                                  width,
                                  height,
                                  root.includeHandicapGames)
                          }
                      }

                      Label {
                          visible: boardPane.investigatingSearch
                                   && goBoard.continuationPoints !== null
                                   && goBoard.continuationPoints.length > 0
                                   && (!boardPane.showingContinuationComparison
                                       || gameList.continuationFilterActive)

                          text: {
                              if (boardPane.comparingContinuations) {
                                  if (boardPane.comparisonStep === "A")
                                      return qsTr("● Choose A")

                                  if (gameList.comparisonCandidateA !== null) {
                                      return qsTr("A %1 · ● Choose B")
                                          .arg(
                                              gameList
                                                  .comparisonCandidateA
                                                  .coordinate)
                                  }

                                  return qsTr("● Choose B")
                              }

                              if (gameList.continuationFilterActive) {
                                  return qsTr("● %1 · %2 games")
                                      .arg(
                                          gameList.goCoordinate(
                                              gameList.selectedContinuationX,
                                              gameList.selectedContinuationCoreY))
                                      .arg(gameList.searchResultCount)
                              }

                              return qsTr("● Choose continuation")
                          }

                          color: "#7d1e16"
                          font.bold: true
                      }

                      ToolButton {
                          visible: boardPane.investigatingSearch
                                   && gameList.continuationFilterActive
                                   && !boardPane.comparingContinuations
                          text: qsTr("Clear filter")
                          onClicked: gameList.clearContinuationFilter()
                      }

                      ToolButton {
                          visible: boardPane.investigatingSearch
                                   && gameList.continuationCandidates.length >= 2

                          text: boardPane.comparingContinuations
                                ? qsTr("Cancel compare")
                                : boardPane.showingContinuationComparison
                                  ? qsTr("Clear comparison")
                                  : qsTr("Compare")

                          ToolTip.visible: hovered
                          ToolTip.text: boardPane.comparingContinuations
                                        ? qsTr("Stop choosing continuations to compare")
                                        : boardPane.showingContinuationComparison
                                          ? qsTr("Clear the continuation comparison")
                                          : qsTr("Compare two professional continuations")

                          onClicked: {
                              if (boardPane.comparingContinuations) {
                                  boardPane.cancelContinuationComparison()
                              } else if (boardPane.showingContinuationComparison) {
                                  gameList.comparisonCandidateA = null
                                  gameList.comparisonCandidateB = null
                              } else {
                                  boardPane.beginContinuationComparison()
                              }
                          }
                      }

                      ToolButton {
                          visible: boardPane.investigatingSearch
                          text: qsTr("New search")

                          onClicked: boardPane.beginNewSearch()
                      }

                      ToolButton {
                          text: qsTr("Influence")
                          checkable: true
                          checked: goBoard.influenceVisible

                          onToggled: goBoard.influenceVisible = checked
                      }

                      Item {
                          Layout.fillWidth: true
                      }

                      ToolButton {
                          text: qsTr("↔")
                          Layout.preferredWidth: Kirigami.Units.gridUnit * 2
                          font.pixelSize: Kirigami.Units.gridUnit * 1.15

                          ToolTip.visible: hovered
                          ToolTip.text: qsTr("Flip board left to right")

                          onClicked: goBoard.flipViewLeftRight()
                      }

                      ToolButton {
                          text: qsTr("↕")
                          Layout.preferredWidth: Kirigami.Units.gridUnit * 2
                          font.pixelSize: Kirigami.Units.gridUnit * 1.15

                          ToolTip.visible: hovered
                          ToolTip.text: qsTr("Flip board top to bottom")

                          onClicked: goBoard.flipViewTopBottom()
                      }

                      ToolButton {
                          text: qsTr("↺")
                          Layout.preferredWidth: Kirigami.Units.gridUnit * 2
                          font.pixelSize: Kirigami.Units.gridUnit * 1.15

                          ToolTip.visible: hovered
                          ToolTip.text: qsTr("Rotate board 90° counter-clockwise")

                          onClicked: goBoard.rotateViewCounterClockwise()
                      }

                      Label {
                          text: {
                              if (goBoard.patternSelectionValid) {
                                  return qsTr("%1 × %2 intersections")
                                      .arg(boardPane.patternRight
                                           - boardPane.patternLeft + 1)
                                      .arg(boardPane.patternBottom
                                           - boardPane.patternTop + 1)
                              }

                              return boardPane.selectingPattern
                                  ? qsTr("Drag over the board")
                                  : ""
                          }
                          opacity: 0.75
                      }
                  }

                  RowLayout {
                      Layout.fillWidth: true
                      Layout.leftMargin: 8
                      Layout.rightMargin: 8
                      spacing: 4

                      visible: boardPane.editingPosition
                               || boardPane.investigatingSearch
                      enabled: !boardPane.selectingPattern
                      opacity: boardPane.selectingPattern ? 0 : 1

                      Label {
                          visible: boardPane.editingPosition
                          text: qsTr("Place:")
                      }

                      ToolButton {
                          visible: boardPane.editingPosition
                          text: qsTr("Black")
                          checkable: true
                          checked: boardPane.editTool === "black"

                          onClicked: boardPane.editTool = "black"
                      }

                      ToolButton {
                          visible: boardPane.editingPosition
                          text: qsTr("White")
                          checkable: true
                          checked: boardPane.editTool === "white"

                          onClicked: boardPane.editTool = "white"
                      }

                      ToolButton {
                          visible: boardPane.editingPosition
                          text: qsTr("Erase")
                          checkable: true
                          checked: boardPane.editTool === "erase"

                          onClicked: boardPane.editTool = "erase"
                      }

                      Item {
                          Layout.fillWidth: true
                      }

                      Label {
                          visible: boardPane.investigatingSearch
                                   && gameList.searchOutcomeText.length > 0
                          text: gameList.searchOutcomeText
                          font.bold: true
                      }

                      Item {
                          Layout.fillWidth: true
                      }

                      Label {
                          visible: boardPane.editingPosition
                          text: qsTr("Click an intersection to edit")
                          opacity: 0.75
                      }
                  }

                  Frame {
                      id: gameDetailsFrame

                    Layout.fillWidth: true

                    Layout.minimumHeight:
                        Math.max(
                            gameDetailsContent.implicitHeight,
                            continuationComparisonContent.implicitHeight)
                        + gameDetailsFrame.topPadding
                        + gameDetailsFrame.bottomPadding

                    Layout.preferredHeight:
                        Layout.minimumHeight

                    padding: 8

                    ColumnLayout {
                        id: gameDetailsContent
                        anchors.fill: parent
                        spacing: 4
                        visible: !boardPane.showingContinuationComparison

                        Label {
                            Layout.fillWidth: true

                            text: {
                                if (boardPane.selectedGame) {
                                    if (boardPane.selectedGame.white.length > 0) {
                                        return qsTr("%1 — %2")
                                            .arg(boardPane.selectedGame.black)
                                            .arg(boardPane.selectedGame.white)
                                    }

                                    return boardPane.selectedGame.black
                                }

                                return gameList.searchResultsSelected
                                    ? qsTr("No search result selected")
                                    : qsTr("No game selected")
                            }

                            font.pixelSize: 20
                            elide: Text.ElideRight
                        }

                        Label {
                            Layout.fillWidth: true

                            text: {
                                if (!boardPane.selectedGame) {
                                return gameList.searchResultsSelected
                                ? qsTr("Run a search, then select a matching game")
                                : qsTr("Select a game from the catalogue")
                            }

                                let details = []

                                if (boardPane.selectedGame.gameDate.length > 0) {
                                    details.push(
                                                boardPane.selectedGame.gameDate)
                                }

                                if (boardPane.selectedGame.result.length > 0) {
                                    details.push(
                                                boardPane.selectedGame.result)
                                }

                                if (boardPane.selectedGame.komi.length > 0) {
                                    details.push(
                                                qsTr("Komi %1").arg(
                                                    boardPane.selectedGame.komi))
                                }

                                if (boardPane.selectedGame.eventName.length > 0) {
                                    details.push(
                                                boardPane.selectedGame.eventName)
                                }

                                return details.join(" · ")
                            }

                            color: palette.text
                            opacity: 0.75
                            font.pixelSize: 16
                            elide: Text.ElideRight
                        }

                        RowLayout {
                            Layout.fillWidth: true
                            spacing: 4

                            visible:
                                boardPane.matchOccurrences.length > 0

                            ToolButton {
                                text: qsTr("Previous Match")

                                enabled: boardPane.matchIndex > 0

                                onClicked: boardPane.showMatch(
                                               boardPane.matchIndex - 1)
                            }

                            Label {
                                Layout.fillWidth: true

                                text: {
                                    if (boardPane.matchIndex < 0
                                            || boardPane.matchIndex
                                               >= boardPane.matchOccurrences.length)
                                        return ""

                                    const occurrence =
                                        boardPane.matchOccurrences[
                                            boardPane.matchIndex]

                                    const spanText =
                                        boardPane.matchSpanText(occurrence)

                                    if (boardPane.showingMatchPosition) {
                                        return qsTr(
                                                    "Match %1 of %2 · %3")
                                            .arg(boardPane.matchIndex + 1)
                                            .arg(
                                                boardPane.matchOccurrences.length)
                                            .arg(spanText)
                                    }

                                    return qsTr(
                                                "Match %1 of %2 · %3 · viewing move %4")
                                        .arg(boardPane.matchIndex + 1)
                                        .arg(
                                            boardPane.matchOccurrences.length)
                                        .arg(spanText)
                                        .arg(gameController.move_number)
                                }

                                horizontalAlignment:
                                    Text.AlignHCenter

                                elide: Text.ElideRight
                            }

                            ToolButton {
                                text: qsTr("Return")

                                visible:
                                    !boardPane.showingMatchPosition
                                    && boardPane.matchIndex >= 0

                                onClicked:
                                    boardPane.showMatch(
                                        boardPane.matchIndex)

                                ToolTip.visible: hovered

                                ToolTip.text:
                                    qsTr("Return to the matched position")
                            }

                            ToolButton {
                                text: qsTr("Next Match")

                                enabled:
                                    boardPane.matchIndex >= 0
                                    && boardPane.matchIndex
                                       < boardPane.matchOccurrences.length - 1

                                onClicked: boardPane.showMatch(
                                               boardPane.matchIndex + 1)
                            }
                        }

                        Slider {
    id: moveSlider

    Layout.fillWidth: true

    from: 0
    to: Math.max(0, gameController.move_count)
    value: gameController.move_number

    stepSize: 1
    snapMode: Slider.SnapAlways

    enabled: boardPane.selectedGame
             && gameController.move_count > 0

    onMoved: {
        const requestedMove = Math.round(value)

        if (requestedMove !== gameController.move_number)
            boardPane.showMove(requestedMove)
    }

    ToolTip.visible: hovered || pressed
    ToolTip.text: qsTr("Move %1").arg(Math.round(value))
}

                        RowLayout {
                            Layout.fillWidth: true
                            spacing: 4

                            Label {
                                text: qsTr("Moves:")
                                font.bold: true
                                opacity: 0.75
                            }

                            ToolButton {
                                text: "|<"
                                enabled: boardPane.selectedGame
                                         && gameController.move_number > 0

                                onClicked: boardPane.showMove(0)

                                ToolTip.visible: hovered
                                ToolTip.text: qsTr("First position")
                            }

                            ToolButton {
                                text: "<<"
                                enabled: boardPane.selectedGame
                                         && gameController.move_number > 0

                                onClicked: boardPane.showMove(
                                               Math.max(
                                                   0,
                                                   gameController.move_number
                                                       - 10))

                                ToolTip.visible: hovered
                                ToolTip.text: qsTr("Back 10 moves")
                            }

                            ToolButton {
                                text: "<"
                                enabled: boardPane.selectedGame
                                         && gameController.move_number > 0

                                onClicked: boardPane.showMove(
                                               gameController.move_number - 1)

                                ToolTip.visible: hovered
                                ToolTip.text: qsTr("Previous move")
                            }

                            Label {
                                Layout.fillWidth: true

                                text: {
                                    if (boardPane.editingPosition)
                                        return qsTr("Editing position")

                                    if (boardPane.selectedGame) {
                                        return qsTr("Move %1 of %2")
                                            .arg(gameController.move_number)
                                            .arg(gameController.move_count)
                                    }

                                    return qsTr("Move 0 of 0")
                                }

                                horizontalAlignment: Text.AlignHCenter
                            }

                            ToolButton {
                                text: ">"
                                enabled: boardPane.selectedGame
                                         && gameController.move_number
                                            < gameController.move_count

                                onClicked: boardPane.showMove(
                                               gameController.move_number + 1)

                                ToolTip.visible: hovered
                                ToolTip.text: qsTr("Next move")
                            }

                            ToolButton {
                                text: ">>"
                                enabled: boardPane.selectedGame
                                         && gameController.move_number
                                            < gameController.move_count

                                onClicked: boardPane.showMove(
                                               Math.min(
                                                   gameController.move_count,
                                                   gameController.move_number
                                                       + 10))

                                ToolTip.visible: hovered
                                ToolTip.text: qsTr("Forward 10 moves")
                            }

                            ToolButton {
                                text: ">|"
                                enabled: boardPane.selectedGame
                                         && gameController.move_number
                                            < gameController.move_count

                                onClicked: boardPane.showMove(
                                               gameController.move_count)

                                ToolTip.visible: hovered
                                ToolTip.text: qsTr("Final position")
                            }
                        }
                    }

                    ColumnLayout {
                        id: continuationComparisonContent
                        anchors.fill: parent
                        spacing: 4
                        visible: boardPane.showingContinuationComparison

                        RowLayout {
                            Layout.fillWidth: true
                            spacing: 4

                            Label {
                                text: qsTr("A")
                                font.bold: true
                                Layout.preferredWidth:
                                    Kirigami.Units.gridUnit * 1.5
                            }

                            Label {
                                text: gameList.comparisonCandidateA === null
                                      ? ""
                                      : gameList.comparisonCandidateA.coordinate
                                font.bold: true
                                Layout.preferredWidth:
                                    Kirigami.Units.gridUnit * 3
                            }

                            Label {
                                text: gameList.comparisonCandidateA === null
                                      ? ""
                                      : qsTr("%1 · %2")
                                            .arg(gameList.appearanceCountText(
                                                gameList
                                                    .comparisonCandidateA
                                                    .count))
                                            .arg(gameList.gameCountText(
                                                gameList
                                                    .comparisonCandidateA
                                                    .gameCount))
                                Layout.preferredWidth:
                                    Kirigami.Units.gridUnit * 12
                            }

                            Label {
                                text: gameList.comparisonCandidateA === null
                                      ? ""
                                      : qsTr(
                                          "Black %1 · White %2 · Draw %3 · Unknown %4")
                                            .arg(gameList
                                                .comparisonCandidateA
                                                .blackWins)
                                            .arg(gameList
                                                .comparisonCandidateA
                                                .whiteWins)
                                            .arg(gameList
                                                .comparisonCandidateA
                                                .draws)
                                            .arg(gameList
                                                .comparisonCandidateA
                                                .unknown)
                                Layout.fillWidth: true
                                elide: Text.ElideRight
                            }

                            Button {
                                text: qsTr("Show games")

                                onClicked:
                                    gameList.showComparisonCandidate(
                                        gameList.comparisonCandidateA)
                            }
                        }

                        RowLayout {
                            Layout.fillWidth: true
                            spacing: 4

                            Label {
                                text: qsTr("B")
                                font.bold: true
                                Layout.preferredWidth:
                                    Kirigami.Units.gridUnit * 1.5
                            }

                            Label {
                                text: gameList.comparisonCandidateB === null
                                      ? ""
                                      : gameList.comparisonCandidateB.coordinate
                                font.bold: true
                                Layout.preferredWidth:
                                    Kirigami.Units.gridUnit * 3
                            }

                            Label {
                                text: gameList.comparisonCandidateB === null
                                      ? ""
                                      : qsTr("%1 · %2")
                                            .arg(gameList.appearanceCountText(
                                                gameList
                                                    .comparisonCandidateB
                                                    .count))
                                            .arg(gameList.gameCountText(
                                                gameList
                                                    .comparisonCandidateB
                                                    .gameCount))
                                Layout.preferredWidth:
                                    Kirigami.Units.gridUnit * 12
                            }

                            Label {
                                text: gameList.comparisonCandidateB === null
                                      ? ""
                                      : qsTr(
                                          "Black %1 · White %2 · Draw %3 · Unknown %4")
                                            .arg(gameList
                                                .comparisonCandidateB
                                                .blackWins)
                                            .arg(gameList
                                                .comparisonCandidateB
                                                .whiteWins)
                                            .arg(gameList
                                                .comparisonCandidateB
                                                .draws)
                                            .arg(gameList
                                                .comparisonCandidateB
                                                .unknown)
                                Layout.fillWidth: true
                                elide: Text.ElideRight
                            }

                            Button {
                                text: qsTr("Show games")

                                onClicked:
                                    gameList.showComparisonCandidate(
                                        gameList.comparisonCandidateB)
                            }
                        }

                    }
                } // closes the new gameDetailsFrame


            } // surrounding ColumnLayout
        } // boardPane
    } // mainSplitView
} // ApplicationWindow
