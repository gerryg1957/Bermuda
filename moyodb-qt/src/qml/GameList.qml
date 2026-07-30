import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import org.moyodb.app

Kirigami.AbstractCard {
    id: root

    signal gameSelected(var game)

    property string projectPath: ""
    property int selectedRow: -1
    property bool projectLoaded: false

    padding: 0

    function loadProject() {
        selectedRow = -1

        if (projectPath.length === 0) {
            projectLoaded = false
            return
        }

        projectLoaded = gameModel.loadProject(projectPath)
    }

    onProjectPathChanged: loadProject()

    Component.onCompleted: loadProject()
    GameListModel {
        id: gameModel
    }

    contentItem: ColumnLayout {
        spacing: 0

        RowLayout {
            Layout.fillWidth: true
            Layout.leftMargin: Kirigami.Units.smallSpacing
            Layout.rightMargin: Kirigami.Units.smallSpacing
            Layout.topMargin: 2
            Layout.bottomMargin: 2

            Kirigami.Heading {
                text: qsTr("Games")
                level: 2
                Layout.fillWidth: true
            }

            Label {
                text: qsTr("%1 games").arg(gameView.count)
                opacity: 0.7
            }
        }

        Rectangle {
            Layout.fillWidth: true
            implicitHeight: headerRow.implicitHeight
            color: Kirigami.Theme.alternateBackgroundColor

            RowLayout {
                id: headerRow

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
        }

        ListView {
            id: gameView

            Layout.fillWidth: true
            Layout.fillHeight: true

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
                height: Math.round(Kirigami.Units.gridUnit * 1.5)

                highlighted: root.selectedRow === index

                background: Rectangle {
                    anchors {
                        left: parent.left
                        right: parent.right
                        verticalCenter: parent.verticalCenter
                        verticalCenterOffset: 6
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
                        komi: rowDelegate.komi
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
                        leftPadding: Kirigami.Units.smallSpacing
                    }

                    Label {
                        text: rowDelegate.result
                        Layout.preferredWidth: 80
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
                        elide: Text.ElideRight
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
