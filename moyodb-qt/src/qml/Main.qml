import QtCore
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
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


    header: ToolBar {
        implicitHeight: 52

        Label {
            anchors {
                left: parent.left
                leftMargin: 18
                verticalCenter: parent.verticalCenter
            }

            text: qsTr("Game Database")
            font.pixelSize: 24
        }
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

                if (gameController.showPosition(
                            root.projectPath,
                            selectedGame.gameId,
                            moveNumber)) {
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

                            text: boardPane.selectedGame
                                ? qsTr("%1 — %2")
                                      .arg(boardPane.selectedGame.black)
                                      .arg(boardPane.selectedGame.white)
                                : qsTr("No game selected")

                            font.pixelSize: 20
                            elide: Text.ElideRight
                        }

                        Label {
                            Layout.fillWidth: true

                            text: {
                                if (!boardPane.selectedGame) {
                                    return qsTr(
                                                "Select a game from the catalogue")
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

                                text: boardPane.selectedGame
                                    ? qsTr("Move %1 of %2")
                                          .arg(gameController.move_number)
                                          .arg(gameController.move_count)
                                    : qsTr("Move 0 of 0")

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
