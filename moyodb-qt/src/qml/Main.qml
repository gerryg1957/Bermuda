import QtCore
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import QtQuick.Dialogs
import org.moyodb.app

ApplicationWindow {
    id: root

    visible: true
    title: qsTr("MoyoDB")

    // Initial size only. The user can resize or maximize normally.
       width: 1500
    height: 850

    minimumWidth: 900
    minimumHeight: 600
       property string projectPath: Qt.application.arguments.length > 1
        ? Qt.application.arguments[1]
        : ""

    MoyoDbApp {
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

  FolderDialog {
      id: openDatabaseDialog

      title: qsTr("Open Database")

      onAccepted: {
          const folderUrl = new URL(selectedFolder)
          const folderPath =
              decodeURIComponent(folderUrl.pathname)

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
            text: qsTr("&Open Database…")

            enabled: !databaseOperation.in_progress

            onTriggered: openDatabaseDialog.open()
        }

        Action {
            text: qsTr("&Create Database…")

            enabled: !databaseOperation.in_progress

            onTriggered:
                databaseImportDialog.openCreate()
        }

        Action {
            text: qsTr("&Add Games…")

            enabled: root.projectPath.length > 0
                     && !databaseOperation.in_progress

            onTriggered:
                databaseImportDialog.openAdd(
                    root.projectPath)
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
    }
}

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
                if (matchIndex < 0
                        || matchIndex >= matchOccurrences.length) {
                    return
                }

                const visualY = goBoard.boardSize - 1 - coreY

                /*
                 * Selecting the active continuation again restores the
                 * complete search result set. This makes board candidates
                 * behave like natural toggle controls.
                 */
                if (gameList.continuationFilterActive
                        && goBoard.selectedContinuationX === boardX
                        && goBoard.selectedContinuationY === visualY) {
                    gameList.clearContinuationFilter()
                    return
                }

                const occurrence = matchOccurrences[matchIndex]

                if (!gameList.filterContinuationAtOccurrence(
                            boardX,
                            coreY,
                            occurrence.left,
                            occurrence.bottom,
                            occurrence.transformation,
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

                    /*
                     * Keep the goban physically stable while search filters,
                     * game metadata and other information below it change.
                     * The board-pane width is the authority for board size.
                     */
                    Layout.preferredHeight: width
                    Layout.minimumHeight: width
                    Layout.maximumHeight: width

                    padding: 4

                      GoBoard {
                          id: goBoard
                          anchors.fill: parent

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

                      visible: boardPane.selectedGame !== null

                      ToolButton {
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
                          text: qsTr("Clear Selection")
                          enabled: goBoard.patternSelectionValid

                          onClicked: boardPane.clearPatternSelection()
                      }

                      ToolButton {
                          text: qsTr("Influence")
                          checkable: true
                          checked: goBoard.influenceVisible

                          onToggled: goBoard.influenceVisible = checked
                      }

                      ToolButton {
                          text: qsTr("Search Database")

                          enabled: goBoard.patternSelectionValid
                                   && boardPane.selectedGame !== null
                                   && root.projectPath.length > 0
                                   && !gameList.searchInProgress

                          onClicked: {
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
                                  height)
                          }
                      }

                      Item {
                          Layout.fillWidth: true
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

                      visible: boardPane.editingPosition && !boardPane.selectingPattern

                      Label {
                          text: qsTr("Place:")
                      }

                      ToolButton {
                          text: qsTr("Black")
                          checkable: true
                          checked: boardPane.editTool === "black"

                          onClicked: boardPane.editTool = "black"
                      }

                      ToolButton {
                          text: qsTr("White")
                          checkable: true
                          checked: boardPane.editTool === "white"

                          onClicked: boardPane.editTool = "white"
                      }

                      ToolButton {
                          text: qsTr("Erase")
                          checkable: true
                          checked: boardPane.editTool === "erase"

                          onClicked: boardPane.editTool = "erase"
                      }

                      Item {
                          Layout.fillWidth: true
                      }

                      Label {
                          text: qsTr("Click an intersection to edit")
                          opacity: 0.75
                      }
                  }

                  Frame {
                      id: gameDetailsFrame

                    Layout.fillWidth: true

                    Layout.minimumHeight:
                        gameDetailsContent.implicitHeight
                        + gameDetailsFrame.topPadding
                        + gameDetailsFrame.bottomPadding

                    Layout.preferredHeight:
                        Layout.minimumHeight

                    padding: 8

                    ColumnLayout {
                        id: gameDetailsContent
                        anchors.fill: parent
                        spacing: 4

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
                                    if (boardPane.matchIndex < 0)
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
                } // closes the new gameDetailsFrame


            } // surrounding ColumnLayout
        } // boardPane
    } // mainSplitView
} // ApplicationWindow
