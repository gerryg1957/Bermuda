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

            boardPane.editingPosition = false

            boardPane.selectedGame = {
                gameId: -1,
                black: qsTr("External SGF"),
                white: fileName,
                gameDate: "",
                result: "",
                eventName: "",
                komi: ""
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

                       onGameSelected: function(game) {
                boardPane.editingPosition = false
                boardPane.selectedGame = game

                if (gameController.loadGame(
                            root.projectPath,
                            game.gameId)) {
                    boardPane.applyLoadedPosition()
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

            function applyLoadedPosition() {
                goBoard.boardSize = gameController.board_size
                goBoard.stones = JSON.parse(
                            gameController.stones_json)

                goBoard.lastMoveX = gameController.last_move_x
                goBoard.lastMoveY = gameController.last_move_y
                goBoard.lastMoveNumber = gameController.move_number
            }

            function showMove(moveNumber) {
                if (!selectedGame) {
                    return
                }

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
                    padding: 4

                      GoBoard {
                          id: goBoard
                          anchors.fill: parent

                          onPointClicked: function(x, y) {
                              if (!boardPane.editingPosition)
                                  return

                              if (gameController.editPositionPoint(
                                          x,
                                          y,
                                          boardPane.editTool)) {
                                  boardPane.applyLoadedPosition()
                              } else {
                                  console.warn(
                                              gameController.error_message)
                              }
                          }
                      }
                  }

                  RowLayout {
                      Layout.fillWidth: true
                      Layout.leftMargin: 8
                      Layout.rightMargin: 8
                      spacing: 4

                      visible: boardPane.editingPosition

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
                    Layout.minimumHeight: 112
                    Layout.preferredHeight: 122
                    Layout.maximumHeight: 160

                    padding: 8

                    ColumnLayout {
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
