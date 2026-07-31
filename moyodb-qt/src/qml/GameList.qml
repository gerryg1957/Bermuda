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
                rightPadding: verticalScrollBar.width + 4
                clip: true

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
