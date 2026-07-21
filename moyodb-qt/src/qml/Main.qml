import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import org.moyodb.app

ApplicationWindow {
    id: root

    visible: true
    title: qsTr("MoyoDB")

    // Initial size only. The user can resize or maximize normally.
    width: 1280
    height: 850

    minimumWidth: 900
    minimumHeight: 600

    property string projectPath: Qt.application.arguments.length > 1
        ? Qt.application.arguments[1]
        : ""

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

            SplitView.minimumWidth: 420
            SplitView.preferredWidth: 720
            SplitView.fillWidth: true
        }

        // Board and game-details pane
        Pane {
            id: boardPane

            padding: 0

            SplitView.minimumWidth: 420
            SplitView.preferredWidth: 660
            SplitView.fillWidth: true

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

                            text: qsTr("No game selected")
                            font.pixelSize: 20
                            elide: Text.ElideRight
                        }

                        Label {
                            Layout.fillWidth: true

                            text: qsTr("Select a game from the database")
                            color: palette.mid
                            font.pixelSize: 16
                            elide: Text.ElideRight
                        }
                    }
                }
            }
        }
    }
}
