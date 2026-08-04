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
    property int selectedSearchRow: -1
    property bool projectLoaded: false
    property bool searchHasRun: false
    readonly property bool searchInProgress:
        searchModel.search_in_progress
    property int searchPatternWidth: 0
    property int searchPatternHeight: 0

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
        searchHasRun = false
        searchPatternWidth = 0
        searchPatternHeight = 0
        searchModel.clearResults()
    }

    function searchProject(boardSize,
                           stonesJson,
                           left,
                           bottom,
                           width,
                           height) {
        selectedSearchRow = -1
        searchHasRun = true
        searchPatternWidth = width
        searchPatternHeight = height

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

   contentItem: ColumnLayout {
    anchors.fill: parent
    spacing: 0

    Item {
        Layout.fillWidth: true
        Layout.preferredHeight: root.searchResultsSelected
                                ? Kirigami.Units.gridUnit * 7
                                : 0

        visible: root.searchResultsSelected

       Label {
    anchors.centerIn: parent

    text: qsTr(
              "Search-result Go board will appear here")

    font.italic: true
    opacity: 0.55
}

    }

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

                    const occurrences = JSON.parse(
                                          searchModel.occurrencesJson(index))

                    root.gameSelected({
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
                        matchOccurrences: occurrences,
                        matchWidth: root.searchPatternWidth,
                        matchHeight: root.searchPatternHeight,
                        fromSearchResults: true
                    })
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
