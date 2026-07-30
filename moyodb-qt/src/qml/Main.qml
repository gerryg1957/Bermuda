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
            }

            SplitView.minimumWidth: 420
            SplitView.preferredWidth: 820
            SplitView.fillWidth: true
        }

        // Board and game-details pane
        Pane {
            id: boardPane

            property var selectedGame: null
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
                    Layout.minimumHeight: 74
                    Layout.preferredHeight: 82
                    Layout.maximumHeight: 130

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
                                    return qsTr("Select a game from the catalogue")
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
                    }
                }
            }
        }
    }
}
