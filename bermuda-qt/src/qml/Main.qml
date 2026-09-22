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

    readonly property string personalProjectPath:
        gameController.personalProjectPath()

    function clearProjectSelection() {
        gameList.clearSearchResults()
        boardPane.clearMatchNavigation()
        boardPane.resetPatternSelection()

        boardPane.editingPosition = false
        root.playingGame = false
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
            gameList.reloadDatabaseProject()
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

    JosekiModel {
        id: josekiModel
    }

    Connections {
        target: josekiModel

        function onLoadingChanged() {
            if (!josekiModel.loading
                    && root.studyPaneMode === "joseki"
                    && josekiModel.node_id.length > 0) {
                boardPane.applyJosekiPosition()
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
            gameList.reloadDatabaseProject()
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

  function openStudySgfPath(filePath, displayName, sourceName) {
      if (!root.prepareStudyReplacement()) {
          console.warn(gameController.error_message)
          return false
      }

      root.playingGame = false

      if (gameController.loadSgf(filePath)) {
          root.finishStudyReplacement()
          boardPane.clearMatchNavigation()
          boardPane.resetPatternSelection()
          boardPane.editingPosition = false

          const blackPlayer = gameController.black_player
          const whitePlayer = gameController.white_player
          const hasPlayerData = blackPlayer.length > 0
                                || whitePlayer.length > 0
          const fallbackSource =
              sourceName !== undefined && sourceName.length > 0
              ? sourceName
              : qsTr("External SGF")

          boardPane.selectedGame = {
              gameId: -1,
              black: hasPlayerData
                     ? qsTr("(B) %1").arg(
                           blackPlayer.length > 0
                           ? blackPlayer
                           : qsTr("Black"))
                     : fallbackSource,
              white: hasPlayerData
                     ? qsTr("(W) %1").arg(
                           whitePlayer.length > 0
                           ? whitePlayer
                           : qsTr("White"))
                     : displayName,
              gameDate: "",
              result: "",
              eventName: "",
              komi: gameController.komi
          }

          boardPane.applyLoadedPosition()
          return true
      }

      const error = gameController.error_message
      const rolledBack = root.cancelStudyReplacement()

      if (!rolledBack) {
          boardPane.selectedGame = null
          goBoard.stones = []
          goBoard.lastMoveX = -1
          goBoard.lastMoveY = -1
          goBoard.lastMoveNumber = 0
      }

      console.warn(error)
      return false
  }


  function studyCurrentJosekiPosition() {
      if (josekiModel.loading
              || josekiModel.study_sgf_path.length === 0
              || josekiModel.node_id.length === 0) {
          return false
      }

      if (!root.prepareStudyReplacement()) {
          console.warn(gameController.error_message)
          return false
      }

      root.playingGame = false

      if (!gameController.loadJosekiStudy(
              josekiModel.study_sgf_path,
              josekiModel.node_id,
              josekiModel.move_count)) {
          const error = gameController.error_message
          root.cancelStudyReplacement()
          console.warn(error)
          return false
      }

      root.finishStudyReplacement()
      boardPane.clearMatchNavigation()
      boardPane.resetPatternSelection()
      boardPane.editingPosition = false

      boardPane.selectedGame = {
          gameId: -1,
          black: qsTr("OGS Joseki Explorer"),
          white: qsTr("Position %1").arg(josekiModel.node_id),
          blackRank: "",
          whiteRank: "",
          gameDate: "",
          result: "",
          eventName: qsTr("OGS Joseki Explorer"),
          komi: "",
          handicap: "",
          fromSearchResults: false
      }

      boardPane.applyLoadedPosition()
      return true
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

        root.openStudySgfPath(
            filePath,
            fileName,
            qsTr("External SGF"))
    }
  }

  Dialog {
      id: studyLibraryDialog

      title: qsTr("Study Library")
      modal: true
      focus: true

      width: Math.min(
          root.width - Kirigami.Units.gridUnit * 4,
          Kirigami.Units.gridUnit * 48)
      height: Math.min(
          root.height - Kirigami.Units.gridUnit * 4,
          Kirigami.Units.gridUnit * 32)

      x: Math.round((root.width - width) / 2)
      y: Math.round((root.height - height) / 2)

      property var entries: []
      property int selectedIndex: -1
      property var pendingDeleteEntry: null

      function refreshEntries() {
          let parsed = []

          try {
              parsed = JSON.parse(
                  gameController.studyLibraryEntriesJson())
          } catch (error) {
              console.warn(
                  "Could not decode Study Library catalogue: "
                  + error)
          }

          entries = parsed
          selectedIndex = -1
          studyLibraryList.currentIndex = -1
      }

      function details(entry) {
          const parts = []

          if (entry.date.length > 0)
              parts.push(entry.date)

          if (entry.event.length > 0)
              parts.push(entry.event)

          if (entry.result.length > 0)
              parts.push(entry.result)

          if (entry.annotationCount === 1)
              parts.push(qsTr("1 annotation"))
          else if (entry.annotationCount > 1)
              parts.push(
                  qsTr("%1 annotations")
                      .arg(entry.annotationCount))

          if (entry.modifiedMillis > 0) {
              const saved = new Date(
                  Number(entry.modifiedMillis))

              parts.push(
                  qsTr("Saved %1")
                      .arg(Qt.formatDateTime(
                          saved,
                          "yyyy-MM-dd hh:mm")))
          }

          if (entry.origin.length > 0)
              parts.push(entry.origin)

          return parts.join(" · ")
      }

      function openSelected() {
          if (selectedIndex < 0
                  || selectedIndex >= entries.length) {
              return
          }

          const entry = entries[selectedIndex]

          if (root.openStudySgfPath(
                      entry.path,
                      entry.title,
                      qsTr("Study Library"))) {
              close()
          }
      }

      onOpened: refreshEntries()

      contentItem: ColumnLayout {
          spacing: Kirigami.Units.smallSpacing

          Label {
              Layout.fillWidth: true
              visible: studyLibraryDialog.entries.length === 0

              text: gameController.error_message.length > 0
                    ? gameController.error_message
                    : qsTr("No saved Study documents yet.")

              wrapMode: Text.WordWrap
              horizontalAlignment: Text.AlignHCenter
              verticalAlignment: Text.AlignVCenter
          }

          ListView {
              id: studyLibraryList

              Layout.fillWidth: true
              Layout.fillHeight: true

              visible: studyLibraryDialog.entries.length > 0
              clip: true
              spacing: 1
              model: studyLibraryDialog.entries
              currentIndex: -1

              onCurrentIndexChanged:
                  studyLibraryDialog.selectedIndex = currentIndex

              delegate: ItemDelegate {
                  required property int index
                  required property var modelData

                  width: ListView.view.width
                  highlighted:
                      studyLibraryList.currentIndex === index

                  onClicked:
                      studyLibraryList.currentIndex = index

                  contentItem: ColumnLayout {
                      spacing: 2

                      Label {
                          Layout.fillWidth: true
                          text: modelData.title
                          font.bold: true
                          elide: Text.ElideRight
                      }

                      Label {
                          Layout.fillWidth: true
                          text: studyLibraryDialog.details(modelData)
                          opacity: 0.72
                          elide: Text.ElideRight
                      }
                  }
              }

              ScrollBar.vertical: ScrollBar {}
          }

          RowLayout {
              Layout.fillWidth: true

              Button {
                  text: qsTr("Refresh")
                  onClicked: studyLibraryDialog.refreshEntries()
              }

              Button {
                  text: qsTr("Delete…")
                  enabled: studyLibraryDialog.selectedIndex >= 0

                  onClicked: {
                      const index =
                          studyLibraryDialog.selectedIndex

                      if (index < 0
                              || index
                                 >= studyLibraryDialog.entries.length) {
                          return
                      }

                      studyLibraryDialog.pendingDeleteEntry =
                          studyLibraryDialog.entries[index]
                      studyLibraryDeleteDialog.open()
                  }
              }

              Item {
                  Layout.fillWidth: true
              }

              Button {
                  text: qsTr("Cancel")
                  onClicked: studyLibraryDialog.close()
              }

              Button {
                  text: qsTr("Open")
                  highlighted: true
                  enabled: studyLibraryDialog.selectedIndex >= 0
                  onClicked: studyLibraryDialog.openSelected()
              }
          }
      }
  }


  Dialog {
      id: studyLibraryDeleteDialog

      title: qsTr("Delete Study document?")
      modal: true
      focus: true
      standardButtons: Dialog.Yes | Dialog.No

      width: Math.min(
          root.width - Kirigami.Units.gridUnit * 4,
          Kirigami.Units.gridUnit * 28)

      x: Math.round((root.width - width) / 2)
      y: Math.round((root.height - height) / 2)

      onAccepted: {
          const entry = studyLibraryDialog.pendingDeleteEntry

          if (entry !== null) {
              if (gameController.deleteStudyLibraryDocument(
                          entry.path)) {
                  studyLibraryDialog.refreshEntries()
              } else {
                  console.warn(gameController.error_message)
              }
          }

          studyLibraryDialog.pendingDeleteEntry = null
      }

      onRejected:
          studyLibraryDialog.pendingDeleteEntry = null

      contentItem: Label {
          width: parent !== null ? parent.width : implicitWidth
          wrapMode: Text.WordWrap

          text: {
              const entry =
                  studyLibraryDialog.pendingDeleteEntry

              if (entry === null)
                  return ""

              return qsTr(
                  "Delete “%1” from the Study Library?\n\n"
                  + "This deletes Bermuda's saved Study document. "
                  + "The original game or SGF source is not changed.")
                  .arg(entry.title)
          }
      }
  }

    function safeSgfFilenamePart(value, fallback) {
        let cleaned = value === undefined || value === null
                      ? ""
                      : value.toString().trim()

        const forbidden = [
            "<", ">", ":", "\"", "/", "\\", "|", "?", "*"
        ]

        for (let i = 0; i < forbidden.length; ++i)
            cleaned = cleaned.split(forbidden[i]).join(" ")

        while (cleaned.indexOf("  ") >= 0)
            cleaned = cleaned.split("  ").join(" ")

        cleaned = cleaned.trim()

        return cleaned.length > 0 ? cleaned : fallback
    }

    function twoDateDigits(value) {
        return value < 10 ? "0" + value : value.toString()
    }

    function playedGameSgfFilename() {
        const black =
            boardPane.selectedGame !== null
            ? safeSgfFilenamePart(
                  boardPane.selectedGame.black,
                  qsTr("Black"))
            : qsTr("Black")

        const white =
            boardPane.selectedGame !== null
            ? safeSgfFilenamePart(
                  boardPane.selectedGame.white,
                  qsTr("White"))
            : qsTr("White")

        const startedAt =
            root.localGameStartedAt !== null
            ? root.localGameStartedAt
            : new Date()

        const dateText =
            startedAt.getFullYear().toString()
            + twoDateDigits(startedAt.getMonth() + 1)
            + twoDateDigits(startedAt.getDate())

        const timeText =
            twoDateDigits(startedAt.getHours())
            + twoDateDigits(startedAt.getMinutes())
            + twoDateDigits(startedAt.getSeconds())

        return black
               + " v "
               + white
               + " "
               + dateText
               + " "
               + timeText
               + ".sgf"
    }

    function openSaveSgfDialog() {
        let folder =
            StandardPaths.writableLocation(
                StandardPaths.DownloadLocation).toString()

        /*
         * Downloads should normally exist, but Home is a harmless
         * fallback if the platform does not provide one.
         */
        if (folder.length === 0) {
            folder =
                StandardPaths.writableLocation(
                    StandardPaths.HomeLocation).toString()
        }

        if (folder.length > 0) {
            saveSgfDialog.currentFolder = folder

            let baseUrl = folder

            if (!baseUrl.endsWith("/"))
                baseUrl += "/"

            saveSgfDialog.selectedFile =
                new URL(
                    playedGameSgfFilename(),
                    baseUrl).toString()
        }

        saveSgfDialog.open()
    }

    FileDialog {
        id: saveSgfDialog

        title: qsTr("Save SGF")
        fileMode: FileDialog.SaveFile
        defaultSuffix: "sgf"

        nameFilters: [
            qsTr("SGF files (*.sgf)"),
            qsTr("All files (*)")
        ]

        onAccepted: {
            const fileUrl = new URL(selectedFile)
            let filePath =
                decodeURIComponent(fileUrl.pathname)

            if (!filePath.toLowerCase().endsWith(".sgf"))
                filePath += ".sgf"

            if (!gameController.savePlayedGameSgf(
                        filePath)) {
                console.warn(
                    gameController.error_message)
                return
            }

            // Played-game SGF exports start in Downloads each time.
        }
    }

    Dialog {
        id: newGameDialog

        title: qsTr("New Game")
        modal: true
        focus: true

        width: Math.min(
            root.width - Kirigami.Units.gridUnit * 4,
            Kirigami.Units.gridUnit * 24)

        x: Math.round((root.width - width) / 2)
        y: Math.round((root.height - height) / 2)

        function startGame() {
            errorLabel.text = ""

            const blackName = blackPlayerField.text.trim()
            const whiteName = whitePlayerField.text.trim()
            const komiText = komiField.text.trim()

            if (!root.prepareStudyReplacement()) {
                errorLabel.text = gameController.error_message
                return
            }

            if (!gameController.newGame(
                        19,
                        blackName,
                        whiteName,
                        komiText)) {
                const error = gameController.error_message
                root.cancelStudyReplacement()
                errorLabel.text = error
                return
            }

            boardPane.clearMatchNavigation()
            boardPane.resetPatternSelection()
            boardPane.editingPosition = false

            root.playingGame = true
            root.finishStudyReplacement()
            root.localGameSessionAvailable = true
            root.localGameStartedAt = new Date()
            root.localGameFinished = false
            root.localGameResult = ""
            root.localGameAddedToMyGames = false

            boardPane.selectedGame = {
                gameId: -1,
                black: blackName.length > 0
                       ? blackName
                       : qsTr("Black"),
                white: whiteName.length > 0
                       ? whiteName
                       : qsTr("White"),
                blackRank: "",
                whiteRank: "",
                gameDate: "",
                result: "",
                event: "",
                komi: gameController.komi,
                handicap: "",
                fromSearchResults: false
            }

            boardPane.applyLoadedPosition()
            close()
        }

        onOpened: {
            errorLabel.text = ""
            blackPlayerField.forceActiveFocus()
        }

        contentItem: ColumnLayout {
            spacing: Kirigami.Units.smallSpacing

            GridLayout {
                Layout.fillWidth: true
                columns: 2
                columnSpacing: Kirigami.Units.largeSpacing
                rowSpacing: Kirigami.Units.smallSpacing

                Label {
                    text: qsTr("Black:")
                }

                TextField {
                    id: blackPlayerField

                    Layout.fillWidth: true
                    placeholderText: qsTr("Black player")
                }

                Label {
                    text: qsTr("White:")
                }

                TextField {
                    id: whitePlayerField

                    Layout.fillWidth: true
                    placeholderText: qsTr("White player")
                }

                Label {
                    text: qsTr("Komi:")
                }

                TextField {
                    id: komiField

                    Layout.fillWidth: true
                    text: "6.5"
                    inputMethodHints: Qt.ImhFormattedNumbersOnly

                    onAccepted: newGameDialog.startGame()
                }

                Label {
                    text: qsTr("Board:")
                }

                Label {
                    text: qsTr("19 × 19")
                }
            }

            Label {
                id: errorLabel

                Layout.fillWidth: true

                visible: text.length > 0
                wrapMode: Text.WordWrap
            }

            Item {
                Layout.preferredHeight:
                    Kirigami.Units.smallSpacing
            }

            RowLayout {
                Layout.fillWidth: true

                Item {
                    Layout.fillWidth: true
                }

                Button {
                    text: qsTr("Cancel")
                    onClicked: newGameDialog.close()
                }

                Button {
                    text: qsTr("Start Game")
                    highlighted: true

                    onClicked: newGameDialog.startGame()
                }
            }
        }
    }

    Dialog {
        id: finishGameDialog

        title: qsTr("Finish Game")
        modal: true
        focus: true

        width: Math.min(
            root.width - Kirigami.Units.gridUnit * 4,
            Kirigami.Units.gridUnit * 22)

        x: Math.round((root.width - width) / 2)
        y: Math.round((root.height - height) / 2)

        function finishGame() {
            finishGameErrorLabel.text = ""

            const result =
                finishGameResultField.text.trim()

            if (!gameController.finishGame(result)) {
                finishGameErrorLabel.text =
                    gameController.error_message
                return
            }

            root.localGameResult = result
            root.localGameFinished = true

            close()
        }

        onOpened: {
            finishGameErrorLabel.text = ""
            finishGameResultField.text = ""
            finishGameResultField.forceActiveFocus()
        }

        contentItem: ColumnLayout {
            spacing: Kirigami.Units.smallSpacing

            Label {
                Layout.fillWidth: true

                text: qsTr(
                    "Enter the result, for example B+3.5, W+0.5 or 0.")

                wrapMode: Text.WordWrap
            }

            TextField {
                id: finishGameResultField

                Layout.fillWidth: true
                placeholderText: qsTr("Result")

                onAccepted: finishGameDialog.finishGame()
            }

            Label {
                id: finishGameErrorLabel

                Layout.fillWidth: true
                visible: text.length > 0
                wrapMode: Text.WordWrap
            }

            RowLayout {
                Layout.fillWidth: true

                Item {
                    Layout.fillWidth: true
                }

                Button {
                    text: qsTr("Cancel")
                    onClicked: finishGameDialog.close()
                }

                Button {
                    text: qsTr("Finish Game")
                    highlighted: true
                    onClicked: finishGameDialog.finishGame()
                }
            }
        }
    }


    Dialog {
        id: studyLabelDialog

        title: qsTr("Board label")
        modal: true
        standardButtons: Dialog.Ok | Dialog.Cancel

        onAccepted:
            boardPane.commitPendingStudyLabel(studyLabelField.text)

        onRejected:
            boardPane.cancelPendingStudyLabel()

        onOpened: {
            studyLabelField.selectAll()
            studyLabelField.forceActiveFocus()
        }

        contentItem: ColumnLayout {
            spacing: 6

            Label {
                text: qsTr("Label text:")
            }

            TextField {
                id: studyLabelField

                Layout.preferredWidth: Kirigami.Units.gridUnit * 14
                placeholderText: qsTr("Text")

                onAccepted: {
                    if (text.trim().length > 0)
                        studyLabelDialog.accept()
                }
            }
        }
    }


    Dialog {
        id: studyCommentDialog

        title: qsTr("Edit SGF comment")
        modal: true

        property int moveNumber: -1

        function openForCurrentPosition() {
            moveNumber = gameController.move_number
            studyCommentField.text =
                gameController.source_comment
            studyCommentError.text = ""
            open()
        }

        onOpened:
            studyCommentField.forceActiveFocus()

        contentItem: ColumnLayout {
            spacing: 8

            Label {
                Layout.preferredWidth:
                    Kirigami.Units.gridUnit * 28

                text: qsTr(
                    "This edits Bermuda's Study copy. "
                    + "The original game database or SGF "
                    + "is left untouched.")

                wrapMode: Text.WordWrap
            }

            ScrollView {
                id: studyCommentScroll

                Layout.preferredWidth:
                    Kirigami.Units.gridUnit * 28
                Layout.preferredHeight:
                    Kirigami.Units.gridUnit * 12

                clip: true
                contentWidth: availableWidth
                ScrollBar.horizontal.policy:
                    ScrollBar.AlwaysOff

                TextArea {
                    id: studyCommentField

                    width: studyCommentScroll.availableWidth
                    implicitWidth: 0

                    placeholderText: qsTr(
                        "Comment for this position")

                    wrapMode: TextEdit.WordWrap
                    selectByMouse: true
                }
            }

            Label {
                id: studyCommentError

                Layout.fillWidth: true
                visible: text.length > 0
                color: Kirigami.Theme.negativeTextColor
                wrapMode: Text.WordWrap
            }

            RowLayout {
                Layout.fillWidth: true

                Item {
                    Layout.fillWidth: true
                }

                Button {
                    text: qsTr("Cancel")
                    onClicked: studyCommentDialog.close()
                }

                Button {
                    text: qsTr("Save")
                    highlighted: true

                    onClicked: {
                        if (gameController.setStudyComment(
                                studyCommentDialog.moveNumber,
                                studyCommentField.text)) {
                            boardPane.applyLoadedPosition()
                            studyCommentDialog.close()
                        } else {
                            studyCommentError.text =
                                gameController.error_message
                        }
                    }
                }
            }
        }
    }

menuBar: MenuBar {
    Menu {
        title: qsTr("&Game")

        Action {
            text: qsTr("&Play Game…")
            onTriggered: newGameDialog.open()
        }

        Menu {
            title: qsTr("&Open")

            Action {
                text: qsTr("&SGF File…")
                onTriggered: openSgfDialog.open()
            }

            Action {
                text: qsTr("From Study &Library…")

                onTriggered: {
                    if (root.studyWorkspaceActive
                            && !root.browserExpanded) {
                        root.showStudyPaneMode("library")
                    } else {
                        studyLibraryDialog.open()
                    }
                }
            }

            MenuSeparator {}

            Action {
                text: qsTr("From &Game Database")
                onTriggered: root.showBrowserTab(0)
            }

            Action {
                text: qsTr("From &My Games")
                onTriggered: root.showBrowserTab(1)
            }
        }

        MenuSeparator {}

        Action {
            text: root.localGameFinished
                  ? qsTr("Return to &Played Game")
                  : qsTr("Return to &Game")

            enabled: root.localGameSessionAvailable
                     && !root.playingGame

            onTriggered: root.returnToPlayedGame()
        }

        Action {
            text: root.localGameFinished
                  ? qsTr("&Close Played Game…")
                  : qsTr("&Abandon Game…")

            enabled: root.localGameSessionAvailable

            onTriggered: discardPlayedGameDialog.open()
        }

        MenuSeparator {}

        Action {
            text: qsTr("&Save SGF…")

            /*
             * At present this saves games created by Play Game.
             * It remains available after the game has finished.
             */
            enabled: root.playingGame

            onTriggered: root.openSaveSgfDialog()
        }
    }


    Menu {
        title: qsTr("&Pattern")

        Action {
            text: qsTr("&New Pattern")

            onTriggered: {
                if (!root.prepareStudyReplacement()) {
                    console.warn(gameController.error_message)
                    return
                }

                root.playingGame = false

                if (gameController.newPosition(19)) {
                    root.finishStudyReplacement()

                    /*
                     * New Pattern is a genuinely fresh investigation.
                     * The previous manually-created position is discarded
                     * by newPosition(), and all evidence derived from its
                     * search must be discarded as well.
                     */
                    gameList.clearSearchResults()
                    boardPane.clearMatchNavigation()
                    boardPane.resetPatternSelection()

                    boardPane.previousSearchProjectPath = ""
                    boardPane.searchSourceGame = null
                    boardPane.searchSourceEditingPosition = false
                    boardPane.searchSourceViewTransform = null
                    boardPane.investigationMode = "pattern"

                    goBoard.influenceVisible = false
                    katagoPanel.analysisMoveNumber = -1
                    katagoPanel.analysisVisitBudget = -1
                    katagoPanel.analysisKomi = ""

                    boardPane.editingPosition = true
                    boardPane.editTool = "black"

                    boardPane.selectedGame = {
                        gameId: -1,
                        black: qsTr("Manual pattern"),
                        white: "",
                        gameDate: "",
                        result: "",
                        eventName: "",
                        komi: ""
                    }

                    boardPane.applyLoadedPosition()
                } else {
                    const error = gameController.error_message
                    const rolledBack = root.cancelStudyReplacement()

                    if (!rolledBack) {
                        boardPane.editingPosition = false
                        boardPane.selectedGame = null
                        goBoard.stones = []
                        goBoard.lastMoveX = -1
                        goBoard.lastMoveY = -1
                        goBoard.lastMoveNumber = 0
                    }

                    console.warn(error)
                }
            }
        }

        MenuSeparator {}

        Action {
            text: qsTr("Select Search &Area")

            enabled: !root.playingGame
                     && boardPane.selectedGame !== null
                     && !gameList.searchInProgress
                     && !boardPane.investigatingSearch

            onTriggered: {
                root.showStudy()

                boardPane.investigationMode = "pattern"
                boardPane.selectingPattern = true
                boardPane.clearMatchNavigation()
                gameList.clearSearchResults()
                goBoard.hoverValid = false
            }
        }

        Action {
            text:
                gameList.searchHasRunFor(
                    gameList.databaseProjectPath)
                ? qsTr("Professional Games &Results")
                : qsTr("Find Matches in &Professional Games")

            enabled: !root.playingGame
                     && !gameList.searchInProgress
                     && gameList.databaseProjectPath.length > 0
                     && (gameList.searchHasRunFor(
                             gameList.databaseProjectPath)
                         || boardPane.investigatingSearch
                         || (goBoard.patternSelectionValid
                             && boardPane.selectedGame !== null))

            onTriggered: {
                if (gameList.searchHasRunFor(
                        gameList.databaseProjectPath)) {
                    boardPane.showSamePatternResults(
                        gameList.databaseProjectPath)
                } else if (boardPane.investigatingSearch) {
                    boardPane.searchSamePatternIn(
                        gameList.databaseProjectPath)
                } else {
                    boardPane.searchSelectedPattern(
                        gameList.databaseProjectPath)
                }
            }
        }

        Action {
            text:
                gameList.searchHasRunFor(
                    gameList.myGamesProjectPath)
                ? qsTr("My Games &Results")
                : qsTr("Find Matches in &My Games")

            enabled: !root.playingGame
                     && !gameList.searchInProgress
                     && gameList.myGamesProjectPath.length > 0
                     && (gameList.searchHasRunFor(
                             gameList.myGamesProjectPath)
                         || boardPane.investigatingSearch
                         || (goBoard.patternSelectionValid
                             && boardPane.selectedGame !== null))

            onTriggered: {
                if (gameList.searchHasRunFor(
                        gameList.myGamesProjectPath)) {
                    boardPane.showSamePatternResults(
                        gameList.myGamesProjectPath)
                } else if (boardPane.investigatingSearch) {
                    boardPane.searchSamePatternIn(
                        gameList.myGamesProjectPath)
                } else {
                    boardPane.searchSelectedPattern(
                        gameList.myGamesProjectPath)
                }
            }
        }

        MenuSeparator {}

        Action {
            text: qsTr("New &Search")
            enabled: boardPane.investigatingSearch
            onTriggered: boardPane.beginNewSearch()
        }

        MenuSeparator {}

        Action {
            text: qsTr("Include &handicap games")
            checkable: true
            checked: root.includeHandicapGames

            onToggled:
                root.includeHandicapGames = checked
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
            text: qsTr("Manage Player &Names…")

            enabled: root.projectPath.length > 0
                     && !databaseOperation.in_progress

            onTriggered:
                playerIdentityDialog.openForProject(root.projectPath)
        }

        MenuSeparator {}

        Action {
            text: qsTr("Add Current Game to &My Games")

            /*
             * For now, only completed live games are added.
             */
            enabled: root.playingGame
                     && root.localGameFinished
                     && !root.localGameAddedToMyGames

            onTriggered:
                root.addCurrentPlayedGameToMyGames()
        }

        Action {
            text: qsTr("Remove Selected Game from My &Games…")
            enabled: gameList.showingMyGames
                     && gameList.removeSelectedMyGameEnabled

            onTriggered:
                gameList.removeSelectedMyGameRequested()
        }

        MenuSeparator {}

        Menu {
            title: qsTr("&Maintenance")

            Action {
                text: qsTr("&Current Operation…")

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
        title: qsTr("&Help")

        Action {
            text: qsTr("&About Bermuda")
            onTriggered: aboutDialog.open()
        }
    }
}

    property bool playingGame: false

    /*
     * Browser and Study are mutually exclusive workspace modes.
     *
     * Browsing may show a board preview once a game is selected.
     * Study gives the browser space to the board and Study inspector.
     */
    property bool browserExpanded: true
    property bool studyWorkspaceActive: false
    property bool patternResultsWorkspaceIsStudy: false
    property int lastBrowserTabIndex: 0

    property bool studyWorkspaceAvailable: false
    property var parkedWorkspaceUi: null
    property bool studyReplacementFromBrowser: false
    property bool replacementHadStudyWorkspace: false
    property string studyPaneMode: "document"

    function showStudyPaneMode(mode) {
        if (mode !== "document"
                && mode !== "library"
                && mode !== "joseki") {
            return
        }

        root.studyPaneMode = mode

        if (mode === "library") {
            studyLibraryDialog.refreshEntries()
            studyLibraryEmbeddedList.currentIndex = -1
        }

        if (mode === "joseki") {
            if (josekiModel.node_id.length === 0) {
                if (!josekiModel.loading)
                    josekiModel.loadPosition("root")
            } else if (!josekiModel.loading) {
                boardPane.applyJosekiPosition()
            }

            return
        }

        if (boardPane.selectedGame !== null) {
            boardPane.applyLoadedPosition()
        } else {
            goBoard.stones = []
            goBoard.markup = []
            goBoard.continuationPoints = []
            goBoard.lastMoveX = -1
            goBoard.lastMoveY = -1
            goBoard.lastMoveNumber = 0
        }
    }

    function captureWorkspaceUi() {
        return {
            selectedGame: boardPane.selectedGame,
            editingPosition: boardPane.editingPosition,
            editTool: boardPane.editTool,
            alternateEditColour: boardPane.alternateEditColour,
            selectingPattern: boardPane.selectingPattern,
            patternLeft: boardPane.patternLeft,
            patternTop: boardPane.patternTop,
            patternRight: boardPane.patternRight,
            patternBottom: boardPane.patternBottom,
            matchOccurrences:
                boardPane.matchOccurrences !== null
                ? boardPane.matchOccurrences.slice(0)
                : [],
            matchIndex: boardPane.matchIndex,
            matchWidth: boardPane.matchWidth,
            matchHeight: boardPane.matchHeight,
            showingMatchPosition: boardPane.showingMatchPosition,
            searchSourceGame: boardPane.searchSourceGame,
            searchSourceEditingPosition: boardPane.searchSourceEditingPosition,
            searchSourceViewTransform: boardPane.searchSourceViewTransform,
            previousSearchProjectPath: boardPane.previousSearchProjectPath,
            investigationMode: boardPane.investigationMode,
            comparingContinuations: boardPane.comparingContinuations,
            comparisonStep: boardPane.comparisonStep,
            playingGame: root.playingGame,
            viewTransform: goBoard.currentViewTransform(),
            continuationPoints:
                goBoard.continuationPoints !== null
                ? goBoard.continuationPoints.slice(0)
                : [],
            selectedContinuationX: goBoard.selectedContinuationX,
            selectedContinuationY: goBoard.selectedContinuationY
        }
    }

    function applyWorkspaceUi(state) {
        if (state === null)
            return

        root.playingGame = state.playingGame
        boardPane.selectedGame = state.selectedGame
        boardPane.editingPosition = state.editingPosition
        boardPane.editTool = state.editTool
        boardPane.alternateEditColour = state.alternateEditColour
        boardPane.patternLeft = state.patternLeft
        boardPane.patternTop = state.patternTop
        boardPane.patternRight = state.patternRight
        boardPane.patternBottom = state.patternBottom
        boardPane.matchOccurrences =
            state.matchOccurrences !== undefined
            ? state.matchOccurrences
            : []
        boardPane.matchIndex = state.matchIndex
        boardPane.matchWidth = state.matchWidth
        boardPane.matchHeight = state.matchHeight
        boardPane.showingMatchPosition = state.showingMatchPosition
        boardPane.searchSourceGame = state.searchSourceGame
        boardPane.searchSourceEditingPosition = state.searchSourceEditingPosition
        boardPane.searchSourceViewTransform = state.searchSourceViewTransform
        boardPane.previousSearchProjectPath = state.previousSearchProjectPath
        boardPane.comparingContinuations = state.comparingContinuations
        boardPane.comparisonStep = state.comparisonStep
        boardPane.investigationMode = state.investigationMode
        boardPane.selectingPattern = state.selectingPattern

        boardPane.applyLoadedPosition()

        if (state.viewTransform !== null)
            goBoard.setViewTransform(state.viewTransform)

        if (state.patternLeft >= 0
                && state.patternTop >= 0
                && state.patternRight >= state.patternLeft
                && state.patternBottom >= state.patternTop) {
            goBoard.setPatternSelection(
                state.patternLeft,
                state.patternTop,
                state.patternRight,
                state.patternBottom)
        } else {
            goBoard.clearPatternSelection()
        }

        goBoard.continuationPoints =
            state.continuationPoints !== undefined
            ? state.continuationPoints
            : []
        goBoard.selectedContinuationX = state.selectedContinuationX
        goBoard.selectedContinuationY = state.selectedContinuationY
        goBoard.hoverValid = false
    }

    function exchangeWorkspaceContexts() {
        if (root.parkedWorkspaceUi === null)
            return false

        const activeUi = root.captureWorkspaceUi()
        const parkedUi = root.parkedWorkspaceUi

        if (!gameController.swapWorkspace()) {
            console.warn(gameController.error_message)
            return false
        }

        root.parkedWorkspaceUi = activeUi
        root.applyWorkspaceUi(parkedUi)
        return true
    }

    function saveBrowserSplitState() {
        uiSettings.browserSplitViewStateV3 = mainSplitView.saveState()
    }

    function saveStudySplitState() {
        if (!root.browserExpanded && studyPane.visible) {
            uiSettings.studySplitViewStateV1 =
                mainSplitView.saveState()
        }
    }

    function restoreStudySplitState() {
        Qt.callLater(function() {
            if (uiSettings.studySplitViewStateV1) {
                mainSplitView.restoreState(
                    uiSettings.studySplitViewStateV1)
            }
        })
    }

    function prepareStudyReplacement() {
        root.studyReplacementFromBrowser = false
        root.replacementHadStudyWorkspace =
            root.studyWorkspaceAvailable

        /*
         * If Study is already the active workspace, simply hide any
         * results/browser pane before replacing the Study document.
         */
        if (root.studyWorkspaceActive) {
            root.browserExpanded = false
            return true
        }

        root.saveBrowserSplitState()

        if (root.studyWorkspaceAvailable) {
            if (!root.exchangeWorkspaceContexts())
                return false
        } else {
            if (!gameController.snapshotWorkspace()) {
                console.warn(gameController.error_message)
                return false
            }

            root.parkedWorkspaceUi =
                root.captureWorkspaceUi()
            root.studyWorkspaceAvailable = true
        }

        root.studyWorkspaceActive = true
        root.browserExpanded = false
        root.studyReplacementFromBrowser = true
        root.restoreStudySplitState()
        return true
    }

    function finishStudyReplacement() {
        root.studyWorkspaceAvailable = true
        root.studyWorkspaceActive = true
        root.studyPaneMode = "document"
        root.studyReplacementFromBrowser = false
        root.replacementHadStudyWorkspace = false
    }

    function cancelStudyReplacement() {
        if (!root.studyReplacementFromBrowser)
            return false

        if (!root.exchangeWorkspaceContexts()) {
            root.studyReplacementFromBrowser = false
            return false
        }

        const hadStudy =
            root.replacementHadStudyWorkspace

        root.studyWorkspaceActive = false
        root.browserExpanded = true
        root.studyWorkspaceAvailable = hadStudy

        if (!hadStudy)
            root.parkedWorkspaceUi = null

        root.studyReplacementFromBrowser = false
        root.replacementHadStudyWorkspace = false

        Qt.callLater(function() {
            if (uiSettings.browserSplitViewStateV3) {
                mainSplitView.restoreState(
                    uiSettings.browserSplitViewStateV3)
            }
        })

        return true
    }

    function showStudy() {
        /*
         * Pattern results can be visible while the Study document remains
         * active. In that case Study just hides the results pane; no
         * workspace exchange is required.
         */
        if (root.studyWorkspaceActive) {
            root.browserExpanded = false
            return
        }

        root.saveBrowserSplitState()

        if (root.studyWorkspaceAvailable) {
            if (!root.exchangeWorkspaceContexts())
                return
        } else {
            if (!gameController.snapshotWorkspace()) {
                console.warn(gameController.error_message)
                return
            }

            root.parkedWorkspaceUi =
                root.captureWorkspaceUi()
            root.studyWorkspaceAvailable = true
        }

        root.studyWorkspaceActive = true
        root.browserExpanded = false
        root.restoreStudySplitState()

        if (boardPane.selectedGame === null
                && root.studyPaneMode === "document") {
            root.showStudyPaneMode("library")
        } else if (root.studyPaneMode === "joseki"
                   && josekiModel.node_id.length > 0
                   && !josekiModel.loading) {
            boardPane.applyJosekiPosition()
        }
    }

    function showPatternResults() {
        root.saveStudySplitState()

        /*
         * Pattern results are a pane, not a workspace.
         *
         * If existing results belong to the parked workspace, restore that
         * workspace first. Merely showing the results pane must never replace
         * the document that produced those results.
         */
        const haveSearchContext =
            gameList.searchHasRun
            || gameList.searchInProgress

        if (haveSearchContext
                && root.patternResultsWorkspaceIsStudy
                   !== root.studyWorkspaceActive
                && root.studyWorkspaceAvailable) {
            if (!root.exchangeWorkspaceContexts())
                return

            root.studyWorkspaceActive =
                !root.studyWorkspaceActive
        }

        root.browserExpanded = true
        gameList.currentTabIndex = 2
    }

    function showBrowserTab(index) {
        root.saveStudySplitState()

        if (index === 2) {
            root.showPatternResults()
            return
        }

        let restoreBrowserSplit =
            !root.browserExpanded

        /*
         * Professional games and My games are genuine Browser destinations.
         * Leaving a Study-owned results pane therefore swaps back to the
         * retained Browser document.
         */
        if (root.studyWorkspaceActive) {
            if (!root.exchangeWorkspaceContexts())
                return

            root.studyWorkspaceActive = false
            restoreBrowserSplit = true
        }

        root.browserExpanded = true
        root.lastBrowserTabIndex = index
        gameList.currentTabIndex = index

        if (restoreBrowserSplit) {
            Qt.callLater(function() {
                if (uiSettings.browserSplitViewStateV3) {
                    mainSplitView.restoreState(
                        uiSettings.browserSplitViewStateV3)
                }
            })
        }
    }

    property bool localGameSessionAvailable: false
    property var localGameStartedAt: null
    property bool localGameFinished: false
    property string localGameResult: ""
    property bool localGameAddedToMyGames: false

    function returnToPlayedGame() {
        if (!root.localGameSessionAvailable)
            return false

        if (!root.studyWorkspaceActive
                && root.studyWorkspaceAvailable
                && root.parkedWorkspaceUi !== null
                && root.parkedWorkspaceUi.playingGame === true) {
            root.showStudy()
            return root.studyWorkspaceActive && root.playingGame
        }

        if (!root.studyWorkspaceActive
                && !root.prepareStudyReplacement()) {
            return false
        }

        if (!gameController.restorePlayedGame()) {
            const error = gameController.error_message
            root.cancelStudyReplacement()
            console.warn(error)
            return false
        }

        boardPane.clearMatchNavigation()
        boardPane.resetPatternSelection()
        boardPane.editingPosition = false
        root.playingGame = true
        root.finishStudyReplacement()

        const blackName = gameController.black_player
        const whiteName = gameController.white_player

        boardPane.selectedGame = {
            gameId: -1,
            black: blackName.length > 0
                   ? blackName
                   : qsTr("Black"),
            white: whiteName.length > 0
                   ? whiteName
                   : qsTr("White"),
            blackRank: "",
            whiteRank: "",
            gameDate: "",
            result: root.localGameResult,
            event: "",
            komi: gameController.komi,
            handicap: "",
            fromSearchResults: false
        }

        boardPane.applyLoadedPosition()
        return true
    }

    function reviewPlayedGame() {
        if (!root.playingGame || !root.localGameFinished)
            return false

        gameList.clearSearchResults()
        gameList.clearCurrentCatalogueSelection()
        boardPane.clearMatchNavigation()
        boardPane.resetPatternSelection()
        boardPane.editingPosition = false

        /*
         * Do not replace the played-game document. The same loaded game is
         * simply being viewed through Bermuda's ordinary study controls, so
         * it remains available through Return to played game.
         */
        root.playingGame = false

        boardPane.applyLoadedPosition()
        return true
    }

    function discardPlayedGame() {
        if (!root.localGameSessionAvailable)
            return false

        const wasPlaying = root.playingGame

        if (!gameController.discardPlayedGame()) {
            console.warn(gameController.error_message)
            return false
        }

        root.localGameSessionAvailable = false
        root.playingGame = false
        root.localGameStartedAt = null
        root.localGameFinished = false
        root.localGameResult = ""
        root.localGameAddedToMyGames = false

        if (root.parkedWorkspaceUi !== null
                && root.parkedWorkspaceUi.playingGame === true) {
            root.parkedWorkspaceUi.playingGame = false
        }

        /*
         * Closing the retained session must not blank an unrelated game
         * currently being studied. If the played game itself is on screen,
         * simply leave it there as an ordinary study position.
         */
        if (wasPlaying) {
            boardPane.clearMatchNavigation()
            boardPane.resetPatternSelection()
            boardPane.editingPosition = false
            boardPane.applyLoadedPosition()
        }

        return true
    }

    function addCurrentPlayedGameToMyGames() {
        if (gameController.addPlayedGameToMyGames()) {
            root.localGameAddedToMyGames = true
            gameList.reloadMyGamesProject()

            console.log(
                qsTr("Game added to My Games"))

            return true
        }

        console.warn(gameController.error_message)
        return false
    }

    function removeSelectedMyGame() {
        const game = boardPane.selectedGame

        if (game === null
                || !gameList.showingMyGames
                || gameList.searchResultsSelected
                || game.fromSearchResults === true
                || game.gameId < 0
                || game.gameSourceId === undefined
                || game.gameSourceId < 0) {
            return false
        }

        if (gameController.removeGameFromMyGames(
                    game.gameSourceId)) {
            clearProjectSelection()
            gameList.reloadMyGamesProject()
            return true
        }

        console.warn(gameController.error_message)
        return false
    }

    property bool includeHandicapGames: false

    Dialog {
        id: discardPlayedGameDialog

        modal: true
        anchors.centerIn: parent

        title: root.localGameFinished
               ? qsTr("Close played game?")
               : qsTr("Abandon game?")

        standardButtons: Dialog.Ok | Dialog.Cancel

        onAccepted: root.discardPlayedGame()

        contentItem: Label {
            width: 380
            wrapMode: Text.WordWrap

            text: root.localGameFinished
                  ? qsTr(
                      "This game will no longer be available through "
                      + "Return to Played Game.\n\n"
                      + "Any copy already added to My Games or saved "
                      + "as an SGF will not be affected.")
                  : qsTr(
                      "This game will no longer be available through "
                      + "Return to Game.\n\n"
                      + "Any SGF saved separately will not be affected.")
        }
    }

    Dialog {
        id: removeMyGameDialog

        modal: true
        anchors.centerIn: parent
        title: qsTr("Remove this game from My Games?")
        standardButtons: Dialog.Ok | Dialog.Cancel

        onAccepted: root.removeSelectedMyGame()

        contentItem: Label {
            width: 360
            wrapMode: Text.WordWrap

            text: {
                const game = boardPane.selectedGame

                if (game === null)
                    return ""

                return qsTr(
                    "Remove %1 — %2 from My games?\n\n"
                    + "Any SGF you saved separately will not be affected.")
                    .arg(game.black)
                    .arg(game.white)
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
        property int katagoVisitBudget: 200
        property var browserSplitViewStateV3
        property var studySplitViewStateV1
    }

    Settings {
        id: katagoSettings

        location: StandardPaths.writableLocation(
                      StandardPaths.ConfigLocation)
                  + "/moyodb.ini"

        category: "KataGo"

        property string executable: ""
        property string model: ""
        property string config: ""
    }

    Component.onCompleted: {
        /*
         * Start with the user's previous Browser/Board proportions.
         * A new installation has no saved state and therefore uses the
         * preferred widths below.
         */
        if (uiSettings.browserSplitViewStateV3) {
            mainSplitView.restoreState(
                uiSettings.browserSplitViewStateV3)
        }

        /*
         * My Games is a built-in Bermuda collection.  A new user should
         * see an empty collection rather than having to create or know
         * about its backing project directory.
         */
        if (!gameController.ensurePersonalProject())
            console.warn(gameController.error_message)

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
        /*
         * A hidden browser has no useful split width to remember:
         * showStudy() saved the browsing proportions before hiding it.
         */
        if (root.browserExpanded) {
            uiSettings.browserSplitViewStateV3 =
                mainSplitView.saveState()
        } else {
            root.saveStudySplitState()
        }
    }

    RowLayout {
        id: browserTabStrip

        anchors {
            left: parent.left
            right: parent.right
            top: parent.top
            leftMargin: 6
            rightMargin: 6
            topMargin: 6
        }

        spacing: 0

        TabButton {
            text: qsTr("Professional games")
            checkable: true
            checked:
                root.browserExpanded
                && gameList.currentTabIndex === 0
            onClicked: root.showBrowserTab(0)
        }

        TabButton {
            text: qsTr("My games")
            checkable: true
            checked:
                root.browserExpanded
                && gameList.currentTabIndex === 1
            onClicked: root.showBrowserTab(1)
        }

        TabButton {
            text: qsTr("Pattern results")
            checkable: true
            checked:
                root.browserExpanded
                && gameList.currentTabIndex === 2
            onClicked: root.showPatternResults()
        }

        TabButton {
            text: qsTr("Study")
            checkable: true

            checked:
                root.studyWorkspaceActive
                && !root.browserExpanded

            /*
             * Study is also the home of the persistent Study Library and
             * the online/cached Joseki Library, so it is useful even when
             * no game is currently loaded.
             */
            enabled: true

            onClicked: {
                if (root.studyWorkspaceActive
                        && !root.browserExpanded) {
                    root.showBrowserTab(
                        root.lastBrowserTabIndex)
                } else {
                    root.showStudy()
                }
            }
        }

        Item {
            Layout.fillWidth: true
        }
    }

    SplitView {
        id: mainSplitView

        anchors {
            left: parent.left
            right: parent.right
            top: browserTabStrip.bottom
            bottom: parent.bottom

            leftMargin: 6
            rightMargin: 6
            topMargin: 6
            bottomMargin: 6
        }

        orientation: Qt.Horizontal

        // Database browser pane
        GameList {
            id: gameList

            visible: root.browserExpanded

            databaseProjectPath: root.projectPath
            myGamesProjectPath: root.personalProjectPath

            onShowingMyGamesChanged:
                root.clearProjectSelection()

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

            removeSelectedMyGameEnabled:
                gameList.showingMyGames
                && boardPane.selectedGame !== null
                && boardPane.selectedGame.gameId >= 0
                && boardPane.selectedGame.gameSourceId !== undefined
                && boardPane.selectedGame.gameSourceId >= 0
                && boardPane.selectedGame.fromSearchResults !== true

            onRemoveSelectedMyGameRequested:
                removeMyGameDialog.open()

                       onGameSelected: function(game) {
                if (!game.fromSearchResults)
                    gameList.clearSearchResults()

                /*
                 * My Games is a review collection rather than a browsing
                 * catalogue. Opening one of its ordinary rows should make
                 * that game the Study document immediately.
                 *
                 * Search-result rows deliberately stay in the existing
                 * Pattern Results workflow.
                 */
                const openMyGameInStudy =
                    gameList.showingMyGames
                    && game.fromSearchResults !== true

                if (openMyGameInStudy
                        && !root.prepareStudyReplacement()) {
                    return
                }

                boardPane.clearMatchNavigation()
                boardPane.resetPatternSelection()
                boardPane.editingPosition = false
                root.playingGame = false
                boardPane.selectedGame = game

                const loadProjectPath =
                    game.fromSearchResults
                    ? gameList.currentSearchProjectPath
                    : gameList.projectPath

                if (gameController.loadGame(
                            loadProjectPath,
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

                    if (openMyGameInStudy)
                        root.finishStudyReplacement()
                } else {
                    if (openMyGameInStudy)
                        root.cancelStudyReplacement()

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
        /*
         * Study material uses the space vacated by the browser rather than
         * consuming board height. Keep a maximum of two principal panes on
         * screen: Browser + Board, or Board + Study.
         *
         * Source comments are the first occupant of this pane. My notes and
         * other study material can later share this same area rather than
         * creating additional permanent panes.
         */
        /*
         * Study is the inspector beside the goban.
         *
         * Goban + long move slider stay together on the right.
         * Navigation, investigation, metadata, annotations
         * and board-view controls live here.
         */
        Frame {
            id: studyPane

            visible:
                !root.browserExpanded
                && (boardPane.selectedGame !== null
                    || root.studyPaneMode !== "document")

            SplitView.minimumWidth: 360
            SplitView.preferredWidth: 520
            SplitView.maximumWidth: 700
            SplitView.fillWidth: false

            padding: Kirigami.Units.smallSpacing

            contentItem: ColumnLayout {
                spacing: Kirigami.Units.smallSpacing

                RowLayout {
                    Layout.fillWidth: true
                    spacing: 0

                    TabButton {
                        text: qsTr("Current study")
                        checkable: true
                        checked: root.studyPaneMode === "document"
                        enabled: boardPane.selectedGame !== null

                        onClicked:
                            root.showStudyPaneMode("document")
                    }

                    TabButton {
                        text: qsTr("Study Library")
                        checkable: true
                        checked: root.studyPaneMode === "library"

                        onClicked:
                            root.showStudyPaneMode("library")
                    }

                    TabButton {
                        text: qsTr("Joseki Library")
                        checkable: true
                        checked: root.studyPaneMode === "joseki"

                        onClicked:
                            root.showStudyPaneMode("joseki")
                    }

                    Item {
                        Layout.fillWidth: true
                    }

                    ToolButton {
                        text: qsTr("Done")
                        visible:
                            root.studyPaneMode === "document"
                            && boardPane.selectedGame !== null
                            && !root.playingGame

                        ToolTip.visible: hovered
                        ToolTip.text: qsTr(
                            "Study changes are saved automatically. "
                            + "Return to the Study Library.")

                        onClicked: {
                            boardPane.annotationTool = ""
                            boardPane.sgfEditTool = ""
                            root.showStudyPaneMode("library")
                        }
                    }
                }

                Kirigami.Separator {
                    Layout.fillWidth: true
                }

                StackLayout {
                    Layout.fillWidth: true
                    Layout.fillHeight: true

                    currentIndex:
                        root.studyPaneMode === "library"
                        ? 1
                        : root.studyPaneMode === "joseki"
                          ? 2
                          : 0

                    ColumnLayout {
                        spacing: Kirigami.Units.smallSpacing

                                          Frame {
                                              id: gameDetailsFrame

                                            Layout.fillWidth: true

                                            Layout.minimumHeight:
                                                (boardPane.showingContinuationComparison
                                                 ? continuationComparisonContent.implicitHeight
                                                 : gameDetailsContent.implicitHeight)
                                                + gameDetailsFrame.topPadding
                                                + gameDetailsFrame.bottomPadding

                                            Layout.preferredHeight:
                                                Layout.minimumHeight
                                            Layout.maximumHeight:
                                                Layout.minimumHeight
                                            Layout.fillHeight: false

                                            padding: 5

                                            ColumnLayout {
                                                id: gameDetailsContent
                                                anchors.fill: parent
                                                spacing: 2
                                                visible: root.playingGame
                                                         || !boardPane.showingContinuationComparison

                                                RowLayout {
                                                    Layout.fillWidth: true
                                                    spacing: 8

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
                                                                : ""
                                                        }

                                                        font.pixelSize: 16
                                                        elide: Text.ElideRight
                                                    }

                                                    Label {
                                                        visible: !root.playingGame
                                                                 && boardPane.selectedGame !== null
                                                                 && boardPane.selectedGame.komi.length > 0

                                                        text: qsTr("Komi %1")
                                                              .arg(boardPane.selectedGame
                                                                   ? boardPane.selectedGame.komi
                                                                   : "")

                                                        opacity: 0.75
                                                        font.pixelSize: 14
                                                    }
                                                }

                                                Label {
                                                    visible: !root.playingGame
                                                             && (
                                                                 (!boardPane.selectedGame
                                                                  && gameList.searchResultsSelected)
                                                                 || (boardPane.selectedGame
                                                                     && (
                                                                         boardPane.selectedGame
                                                                             .gameDate.length > 0
                                                                         || boardPane.selectedGame
                                                                             .result.length > 0
                                                                         || boardPane.selectedGame
                                                                             .eventName.length > 0
                                                                     ))
                                                             )
                                                    Layout.fillWidth: true

                                                    text: {
                                                        if (!boardPane.selectedGame) {
                                                        return gameList.searchResultsSelected
                                                        ? qsTr("Run a search, then select a matching game")
                                                        : ""
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
                                                    visible: !root.playingGame
                                                    Layout.fillWidth: true
                                                    spacing: 4

                                                    /*
                                                     * Keep this row in the layout even when there is
                                                     * no match. Removing it with visible:false changes
                                                     * the game-details height and makes the Go board
                                                     * shrink when a search-result game is selected.
                                                     */
                                                    enabled:
                                                        boardPane.matchOccurrences.length > 0
                                                    opacity: enabled ? 1 : 0

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


                                            }

                                            ColumnLayout {
                                                id: continuationComparisonContent
                                                anchors.fill: parent
                                                spacing: 4
                                                visible: !root.playingGame
                                                         && boardPane.showingContinuationComparison

                                                RowLayout {
                                                    Layout.fillWidth: true
                                                    spacing: Kirigami.Units.smallSpacing

                                                    Label {
                                                        text: qsTr("A")
                                                        font.bold: true
                                                        Layout.preferredWidth:
                                                            Kirigami.Units.gridUnit * 1.5
                                                        Layout.alignment: Qt.AlignTop
                                                    }

                                                    Label {
                                                        text: gameList.comparisonCandidateA === null
                                                              ? ""
                                                              : gameList.comparisonCandidateA.coordinate
                                                        font.bold: true
                                                        Layout.preferredWidth:
                                                            Kirigami.Units.gridUnit * 3
                                                        Layout.alignment: Qt.AlignTop
                                                    }

                                                    ColumnLayout {
                                                        Layout.fillWidth: true
                                                        spacing: 2

                                                        Label {
                                                            text: gameList.comparisonCandidateA === null
                                                                  ? ""
                                                                  : qsTr("%1 · %2")
                                                                        .arg(
                                                                            gameList.appearanceCountText(
                                                                                gameList
                                                                                    .comparisonCandidateA
                                                                                    .count))
                                                                        .arg(
                                                                            gameList.gameCountText(
                                                                                gameList
                                                                                    .comparisonCandidateA
                                                                                    .gameCount))
                                                            Layout.fillWidth: true
                                                            elide: Text.ElideRight
                                                        }

                                                        Label {
                                                            text: gameList.comparisonCandidateA === null
                                                                  ? ""
                                                                  : qsTr(
                                                                      "Black %1 · White %2 · Draw %3 · Unknown %4")
                                                                        .arg(
                                                                            gameList
                                                                                .comparisonCandidateA
                                                                                .blackWins)
                                                                        .arg(
                                                                            gameList
                                                                                .comparisonCandidateA
                                                                                .whiteWins)
                                                                        .arg(
                                                                            gameList
                                                                                .comparisonCandidateA
                                                                                .draws)
                                                                        .arg(
                                                                            gameList
                                                                                .comparisonCandidateA
                                                                                .unknown)
                                                            Layout.fillWidth: true
                                                            elide: Text.ElideRight
                                                            opacity: 0.85
                                                        }
                                                    }

                                                    Button {
                                                        text: qsTr("Show games")
                                                        Layout.alignment: Qt.AlignTop

                                                        onClicked:
                                                            gameList.showComparisonCandidate(
                                                                gameList.comparisonCandidateA)
                                                    }
                                                }

                                                RowLayout {
                                                    Layout.fillWidth: true
                                                    spacing: Kirigami.Units.smallSpacing

                                                    Label {
                                                        text: qsTr("B")
                                                        font.bold: true
                                                        Layout.preferredWidth:
                                                            Kirigami.Units.gridUnit * 1.5
                                                        Layout.alignment: Qt.AlignTop
                                                    }

                                                    Label {
                                                        text: gameList.comparisonCandidateB === null
                                                              ? ""
                                                              : gameList.comparisonCandidateB.coordinate
                                                        font.bold: true
                                                        Layout.preferredWidth:
                                                            Kirigami.Units.gridUnit * 3
                                                        Layout.alignment: Qt.AlignTop
                                                    }

                                                    ColumnLayout {
                                                        Layout.fillWidth: true
                                                        spacing: 2

                                                        Label {
                                                            text: gameList.comparisonCandidateB === null
                                                                  ? ""
                                                                  : qsTr("%1 · %2")
                                                                        .arg(
                                                                            gameList.appearanceCountText(
                                                                                gameList
                                                                                    .comparisonCandidateB
                                                                                    .count))
                                                                        .arg(
                                                                            gameList.gameCountText(
                                                                                gameList
                                                                                    .comparisonCandidateB
                                                                                    .gameCount))
                                                            Layout.fillWidth: true
                                                            elide: Text.ElideRight
                                                        }

                                                        Label {
                                                            text: gameList.comparisonCandidateB === null
                                                                  ? ""
                                                                  : qsTr(
                                                                      "Black %1 · White %2 · Draw %3 · Unknown %4")
                                                                        .arg(
                                                                            gameList
                                                                                .comparisonCandidateB
                                                                                .blackWins)
                                                                        .arg(
                                                                            gameList
                                                                                .comparisonCandidateB
                                                                                .whiteWins)
                                                                        .arg(
                                                                            gameList
                                                                                .comparisonCandidateB
                                                                                .draws)
                                                                        .arg(
                                                                            gameList
                                                                                .comparisonCandidateB
                                                                                .unknown)
                                                            Layout.fillWidth: true
                                                            elide: Text.ElideRight
                                                            opacity: 0.85
                                                        }
                                                    }

                                                    Button {
                                                        text: qsTr("Show games")
                                                        Layout.alignment: Qt.AlignTop

                                                        onClicked:
                                                            gameList.showComparisonCandidate(
                                                                gameList.comparisonCandidateB)
                                                    }
                                                }

                                            }
                                        }

                                        Frame {
                                            id: sourceCommentSection

                                            visible:
                                                !root.playingGame
                                                && boardPane.selectedGame !== null

                                            Layout.fillWidth: true
                                            Layout.fillHeight: false
                                            Layout.minimumHeight:
                                                Kirigami.Units.gridUnit * 7
                                            Layout.preferredHeight:
                                                Kirigami.Units.gridUnit * 11
                                            Layout.maximumHeight:
                                                Kirigami.Units.gridUnit * 16

                                            contentItem: ColumnLayout {
                                                spacing:
                                                    Kirigami.Units.smallSpacing

                                                                                                RowLayout {
                                                    Layout.fillWidth: true
                                                    spacing: Kirigami.Units.smallSpacing

                                                    Label {
                                                        Layout.fillWidth: true
                                                        text: qsTr("SGF comment")
                                                        font.bold: true
                                                    }

                                                                                                        Button {
                                                        text:
                                                            gameController.source_comment.length > 0
                                                            ? qsTr("Edit comment…")
                                                            : qsTr("Add comment…")

                                                        ToolTip.visible: hovered
                                                        ToolTip.text: qsTr(
                                                            "Edit the SGF comment in the Study copy. "
                                                            + "The original source is never changed.")

                                                        onClicked:
                                                            studyCommentDialog.openForCurrentPosition()
                                                    }
                                                }

                                                Kirigami.Separator {
                                                    Layout.fillWidth: true
                                                }

                                                ScrollView {
                                                    id: sourceCommentScroll

                                                    Layout.fillWidth: true
                                                    Layout.fillHeight: true
                                                    clip: true
                                                    contentWidth: availableWidth
                                                    ScrollBar.horizontal.policy: ScrollBar.AlwaysOff

                                                    TextArea {
                                                        width:
                                                            sourceCommentScroll.availableWidth
                                                        implicitWidth: 0

                                                        text:
                                                            gameController
                                                                .source_comment.length > 0
                                                            ? gameController
                                                                .source_comment
                                                            : qsTr(
                                                                "No SGF comment at "
                                                                + "this position.")

                                                        readOnly: true
                                                        selectByMouse: true
                                                        wrapMode: TextEdit.WordWrap
                                                        background: null

                                                        opacity:
                                                            gameController
                                                                .source_comment.length > 0
                                                            ? 1.0
                                                            : 0.55

                                                        font.italic:
                                                            gameController
                                                                .source_comment.length === 0
                                                    }
                                                }
                                            }
                                        }

                                        Kirigami.Separator {
                                            Layout.fillWidth: true
                                            Layout.fillHeight: false
                                            Layout.preferredHeight: implicitHeight
                                            Layout.maximumHeight: implicitHeight
                                        }

                                        TabBar {
                                            id: studyToolsTabs

                                            visible:
                                                !root.playingGame
                                                && boardPane.selectedGame !== null
                                                && !boardPane.editingPosition

                                            Layout.fillWidth: true
                                            Layout.leftMargin: 8
                                            Layout.rightMargin: 8

                                            currentIndex: 0

                                            onCurrentIndexChanged: {
                                                /*
                                                 * A hidden annotation tool must not
                                                 * continue consuming board clicks.
                                                 * Likewise, leaving Analyse stops an
                                                 * unfinished rubber-band drag without
                                                 * throwing away an existing result.
                                                 */
                                                if (currentIndex !== 0) {
                                                    boardPane.annotationTool = ""
                                                    boardPane.sgfEditTool = ""
                                                }

                                                if (currentIndex !== 1)
                                                    boardPane.selectingPattern = false
                                            }

                                            TabButton {
                                                width: studyToolsTabs.width / 3
                                                text: qsTr("Study")
                                            }

                                            TabButton {
                                                width: studyToolsTabs.width / 3
                                                text: qsTr("Analyse")
                                            }

                                            TabButton {
                                                width: studyToolsTabs.width / 3
                                                text: qsTr("View")
                                            }
                                        }

                                        RowLayout {
                                            id: studyAnalysisControls

                                            visible:
                                                !root.playingGame
                                                && boardPane.selectedGame !== null
                                                && !boardPane.editingPosition
                                                && studyToolsTabs.currentIndex === 1

                                            Layout.fillWidth: true
                                            Layout.fillHeight: false
                                            Layout.preferredHeight: implicitHeight
                                            Layout.maximumHeight: implicitHeight
                                            Layout.leftMargin: 8
                                            Layout.rightMargin: 8
                                            spacing: 4

                                            Label {
                                                text: qsTr("Analyse:")
                                            }

                                            Button {
                                                id: studyPatternSearchButton

                                                text: qsTr("Pattern Search")
                                                highlighted:
                                                    boardPane.investigationMode === "pattern"

                                                onClicked: {
                                                    if (boardPane.investigationMode === "pattern") {
                                                        boardPane.clearPatternSelection()
                                                        boardPane.investigationMode = ""
                                                    } else {
                                                        boardPane.investigationMode = "pattern"
                                                    }
                                                }
                                            }

                                            Button {
                                                text: qsTr("KataGo")
                                                highlighted:
                                                    boardPane.investigationMode === "katago"

                                                onClicked: {
                                                    if (boardPane.investigationMode === "pattern")
                                                        boardPane.clearPatternSelection()
                                                    else
                                                        boardPane.selectingPattern = false

                                                    boardPane.investigationMode =
                                                        boardPane.investigationMode === "katago"
                                                        ? ""
                                                        : "katago"
                                                }
                                            }

                                            Item {
                                                Layout.fillWidth: true
                                            }
                                        }

                                        RowLayout {
                                            id: positionEditControls

                                            visible:
                                                boardPane.editingPosition
                                                && !boardPane.selectingPattern

                                            Layout.fillWidth: true
                                            Layout.fillHeight: false
                                            Layout.preferredHeight: implicitHeight
                                            Layout.maximumHeight: implicitHeight
                                            Layout.leftMargin: 8
                                            Layout.rightMargin: 8
                                            spacing: 4

                                            Label {
                                                text: qsTr("Place:")
                                            }

                                            ToolButton {
                                                id: editBlackButton
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
                                                text: qsTr("Alternate")
                                                checkable: true
                                                checked: boardPane.editTool === "alternate"

                                                ToolTip.visible: hovered
                                                ToolTip.text:
                                                    qsTr("Place Black and White alternately; next %1")
                                                        .arg(
                                                            boardPane.alternateEditColour === "black"
                                                            ? qsTr("Black")
                                                            : qsTr("White"))

                                                onClicked: {
                                                    boardPane.editTool = "alternate"
                                                    boardPane.alternateEditColour = "black"
                                                }
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
                                        }

                        Label {
                            id: finishedPlayedGameStatus

                            visible: root.playingGame
                                     && root.localGameFinished
                            Layout.fillWidth: true
                            Layout.leftMargin: 8
                            Layout.rightMargin: 8

                            text: qsTr("Game finished — %1")
                                      .arg(root.localGameResult)

                            font.bold: true
                        }

                                                         RowLayout {
                                                             id: patternInvestigationControls

                                                             Layout.fillWidth: true
                                                             visible: root.playingGame
                                                                      || (boardPane.selectedGame !== null
                                                                          && studyToolsTabs.currentIndex === 1
                                                                          && boardPane.investigationMode === "pattern")

                                                             Layout.fillHeight: false
                                                             Layout.preferredHeight: implicitHeight
                                                             Layout.maximumHeight: implicitHeight

                                                             Layout.leftMargin: 8
                                                             Layout.rightMargin: 8
                                                             spacing: 4

                                                             Label {
                                                                 visible: root.playingGame
                                                                          && !root.localGameFinished

                                                                 text: qsTr("Move %1 — %2 to play")
                                                                           .arg(gameController.move_count + 1)
                                                                           .arg(gameController.move_count % 2 === 0
                                                                                ? qsTr("Black")
                                                                                : qsTr("White"))

                                                                 font.bold: true
                                                             }

                                                             ToolButton {
                                                                 visible: root.playingGame
                                                                          && !root.localGameFinished
                                                                 text: qsTr("Pass")

                                                                 onClicked: {
                                                                     if (gameController.playGamePass()) {
                                                                         boardPane.applyLoadedPosition()
                                                                     } else {
                                                                         console.warn(
                                                                                     gameController.error_message)
                                                                     }
                                                                 }
                                                             }

                                                             ToolButton {
                                                                 visible: root.playingGame
                                                                          && !root.localGameFinished
                                                                 text: qsTr("Undo")
                                                                 enabled: gameController.move_count > 0

                                                                 onClicked: {
                                                                     if (gameController.undoGameMove()) {
                                                                         boardPane.applyLoadedPosition()
                                                                     } else {
                                                                         console.warn(
                                                                                     gameController.error_message)
                                                                     }
                                                                 }
                                                             }

                                                             ToolButton {
                                                                 visible: root.playingGame
                                                                          && !root.localGameFinished
                                                                 text: qsTr("Resign")

                                                                 onClicked: {
                                                                     const result =
                                                                         gameController.resignGame()

                                                                     if (result.length > 0) {
                                                                         root.localGameResult = result
                                                                         root.localGameFinished = true
                                                                     } else {
                                                                         console.warn(
                                                                             gameController.error_message)
                                                                     }
                                                                 }
                                                             }

                                                             ToolButton {
                                                                 visible: root.playingGame
                                                                          && !root.localGameFinished
                                                                 text: qsTr("Finish Game")

                                                                 onClicked: finishGameDialog.open()
                                                             }

                                                             Item {
                                                                 visible: root.playingGame
                                                                          && root.localGameFinished

                                                                 implicitWidth:
                                                                     reviewPlayedGameButton.implicitWidth + 6
                                                                 implicitHeight:
                                                                     reviewPlayedGameButton.implicitHeight + 6

                                                                 Rectangle {
                                                                     anchors.fill: parent
                                                                     radius: 6
                                                                     color: Kirigami.Theme.highlightColor
                                                                     opacity: 0.45
                                                                 }

                                                                 Button {
                                                                     id: reviewPlayedGameButton

                                                                     anchors.centerIn: parent

                                                                     text: qsTr("Review game")
                                                                     highlighted: true

                                                                     onClicked: root.reviewPlayedGame()
                                                                 }
                                                             }

                                                             Item {
                                                                 visible: root.playingGame
                                                                          && root.localGameFinished

                                                                 implicitWidth:
                                                                     addToMyGamesButton.implicitWidth + 6
                                                                 implicitHeight:
                                                                     addToMyGamesButton.implicitHeight + 6

                                                                 Rectangle {
                                                                     anchors.fill: parent
                                                                     radius: 6
                                                                     color: Kirigami.Theme.highlightColor
                                                                     opacity: addToMyGamesButton.enabled ? 0.45 : 0.0
                                                                 }

                                                                 Button {
                                                                     id: addToMyGamesButton

                                                                     anchors.centerIn: parent

                                                                     text: root.localGameAddedToMyGames
                                                                           ? qsTr("Added to My Games")
                                                                           : qsTr("Add to My Games")

                                                                     enabled: !root.localGameAddedToMyGames
                                                                     highlighted: !root.localGameAddedToMyGames

                                                                     onClicked:
                                                                         root.addCurrentPlayedGameToMyGames()
                                                                 }
                                                             }

                                                             Item {
                                                                 visible: root.playingGame
                                                                          && root.localGameFinished

                                                                 implicitWidth:
                                                                     savePlayedGameSgfButton.implicitWidth + 6
                                                                 implicitHeight:
                                                                     savePlayedGameSgfButton.implicitHeight + 6

                                                                 Rectangle {
                                                                     anchors.fill: parent
                                                                     radius: 6
                                                                     color: Kirigami.Theme.highlightColor
                                                                     opacity: 0.45
                                                                 }

                                                                 Button {
                                                                     id: savePlayedGameSgfButton

                                                                     anchors.centerIn: parent

                                                                     text: qsTr("Save SGF…")
                                                                     highlighted: true

                                                                     onClicked: root.openSaveSgfDialog()
                                                                 }
                                                             }

                                                             Button {
                                                                 visible: !root.playingGame
                                                                          && !gameList.searchHasRun
                                                                 text: qsTr("Select Search Area")
                                                                 checkable: true

                                                                 ToolTip.visible: hovered
                                                                 ToolTip.text: qsTr(
                                                                     "Drag around the complete context to match. "
                                                                     + "Only stones and board edges inside the blue "
                                                                     + "area are part of the search.")

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

                                                             Button {
                                                                 visible: !root.playingGame
                                                                          && !gameList.searchHasRun
                                                                 text: qsTr("Clear Selection")
                                                                 enabled: goBoard.patternSelectionValid

                                                                 onClicked: boardPane.clearPatternSelection()
                                                             }

                                                             Button {
                                                                 visible: !root.playingGame
                                                                          && !gameList.searchHasRun
                                                                 text: qsTr("Search Database")

                                                                 enabled: goBoard.patternSelectionValid
                                                                          && boardPane.selectedGame !== null
                                                                          && gameList.databaseProjectPath.length > 0
                                                                          && !gameList.searchInProgress

                                                                 onClicked:
                                                                     boardPane.searchSelectedPattern(
                                                                         gameList.databaseProjectPath)
                                                             }

                                                             Button {
                                                                 visible: !root.playingGame
                                                                          && !gameList.searchHasRun
                                                                 text: qsTr("Search My Games")

                                                                 enabled: goBoard.patternSelectionValid
                                                                          && boardPane.selectedGame !== null
                                                                          && gameList.myGamesProjectPath.length > 0
                                                                          && !gameList.searchInProgress

                                                                 onClicked:
                                                                     boardPane.searchSelectedPattern(
                                                                         gameList.myGamesProjectPath)
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
                                                                 visible: !root.playingGame
                                                                          && boardPane.investigatingSearch
                                                                          && gameList.currentSearchProjectPath
                                                                             !== gameList.databaseProjectPath
                                                                 text:
                                                                     gameList.searchHasRunFor(
                                                                         gameList.databaseProjectPath)
                                                                     ? qsTr("Database Results")
                                                                     : qsTr("Search Database")

                                                                 enabled: gameList.databaseProjectPath.length > 0
                                                                          && !gameList.searchInProgress

                                                                 onClicked: {
                                                                     boardPane.rememberSearchReturn(
                                                                         gameList.databaseProjectPath)

                                                                     if (gameList.searchHasRunFor(
                                                                             gameList.databaseProjectPath)) {
                                                                         boardPane.showSamePatternResults(
                                                                             gameList.databaseProjectPath)
                                                                     } else {
                                                                         boardPane.searchSamePatternIn(
                                                                             gameList.databaseProjectPath)
                                                                     }
                                                                 }
                                                             }

                                                             ToolButton {
                                                                 visible: !root.playingGame
                                                                          && boardPane.investigatingSearch
                                                                          && gameList.currentSearchProjectPath
                                                                             !== gameList.myGamesProjectPath
                                                                 text:
                                                                     gameList.searchHasRunFor(
                                                                         gameList.myGamesProjectPath)
                                                                     ? qsTr("My Games Results")
                                                                     : qsTr("Search My Games")

                                                                 enabled: gameList.myGamesProjectPath.length > 0
                                                                          && !gameList.searchInProgress

                                                                 onClicked: {
                                                                     boardPane.rememberSearchReturn(
                                                                         gameList.myGamesProjectPath)

                                                                     if (gameList.searchHasRunFor(
                                                                             gameList.myGamesProjectPath)) {
                                                                         boardPane.showSamePatternResults(
                                                                             gameList.myGamesProjectPath)
                                                                     } else {
                                                                         boardPane.searchSamePatternIn(
                                                                             gameList.myGamesProjectPath)
                                                                     }
                                                                 }
                                                             }

                                                             Item {
                                                                 visible: {
                                                                     const summary =
                                                                         gameList.searchOutcomeSummary

                                                                     return !root.playingGame
                                                                         && boardPane.investigatingSearch
                                                                         && summary !== null
                                                                         && gameList.nextMoveInPatternCount === 0
                                                                 }

                                                                 implicitWidth:
                                                                     adjustAreaButton.implicitWidth + 6
                                                                 implicitHeight:
                                                                     adjustAreaButton.implicitHeight + 6

                                                                 Rectangle {
                                                                     anchors.fill: parent
                                                                     radius: 6
                                                                     color: Kirigami.Theme.highlightColor
                                                                     opacity: 0.45
                                                                 }

                                                                 Button {
                                                                     id: adjustAreaButton

                                                                     anchors.centerIn: parent

                                                                     text: qsTr("Adjust area")
                                                                     highlighted: true

                                                                     ToolTip.visible: hovered
                                                                     ToolTip.text:
                                                                         qsTr("Return to the source position and resize the current search area")

                                                                     onClicked:
                                                                         boardPane.adjustSearchArea()
                                                                 }
                                                             }

                                                             ToolButton {
                                                                 visible: !root.playingGame
                                                                          && boardPane.canReturnInInvestigation
                                                                 text: qsTr("Back")

                                                                 ToolTip.visible: hovered
                                                                 ToolTip.text:
                                                                     qsTr("Return to the previous investigation step")

                                                                 onClicked:
                                                                     boardPane.returnToPreviousInvestigation()
                                                             }

                                                             ToolButton {
                                                                 visible: !root.playingGame
                                                                          && boardPane.investigatingSearch
                                                                 text: qsTr("New search")

                                                                 onClicked: boardPane.beginNewSearch()
                                                             }

                                                             Item {
                                                                 Layout.fillWidth: true
                                                             }

                                                         }

                                                         RowLayout {
                                                             id: patternStatusRow

                                                             Layout.fillWidth: true
                                                             Layout.fillHeight: false
                                                             Layout.preferredHeight: implicitHeight
                                                             Layout.maximumHeight: implicitHeight
                                                             visible:
                                                                 !root.playingGame
                                                                 && (
                                                                     (studyToolsTabs.currentIndex === 1
                                                                         && boardPane.investigationMode
                                                                            === "pattern"
                                                                         && boardPane.investigatingSearch
                                                                         && gameList.searchOutcomeText.length > 0)
                                                                     || (studyToolsTabs.currentIndex === 1
                                                                         && boardPane.investigationMode
                                                                            === "pattern"
                                                                         && goBoard.patternSelectionValid
                                                                         && !gameList.searchHasRun)
                                                                     || (boardPane.editingPosition
                                                                         && boardPane.editTool
                                                                            === "alternate")
                                                                 )
                                                             Layout.leftMargin: 8
                                                             Layout.rightMargin: 8
                                                             spacing: 4

                                                             Label {
                                                                 visible:
                                                                     boardPane.investigationMode === "pattern"
                                                                     && goBoard.patternSelectionValid
                                                                     && !gameList.searchHasRun

                                                                 text: boardPane.patternContextText()
                                                                 color: "#194bb4"
                                                                 font.bold: true

                                                                 ToolTip.visible:
                                                                     contextMouse.containsMouse
                                                                 ToolTip.text: qsTr(
                                                                     "Only stones and board edges inside the "
                                                                     + "selected blue area are matched. "
                                                                     + "Move the selection to the edge of the "
                                                                     + "board when the side or corner is part "
                                                                     + "of the pattern.")

                                                                 MouseArea {
                                                                     id: contextMouse
                                                                     anchors.fill: parent
                                                                     hoverEnabled: true
                                                                     acceptedButtons: Qt.NoButton
                                                                 }
                                                             }

                                                             Item {
                                                                 Layout.fillWidth: true
                                                             }

                                                             RowLayout {
                                                                 visible:
                                                                     boardPane.investigationMode === "pattern"
                                                                     && boardPane.investigatingSearch
                                                                     && gameList.searchOutcomeText.length > 0

                                                                 spacing: Kirigami.Units.largeSpacing * 2

                                                                 Label {
                                                                     visible: {
                                                                         const summary =
                                                                             gameList.searchOutcomeSummary

                                                                         return summary !== null
                                                                             && Number(summary.games) > 0
                                                                             && gameList.nextMoveInPatternCount === 0
                                                                     }

                                                                     text:
                                                                         qsTr("No immediate continuation")
                                                                     color: "#287d78"
                                                                     font.bold: true
                                                                 }

                                                                 Label {
                                                                     visible:
                                                                         goBoard.continuationPoints !== null
                                                                         && goBoard.continuationPoints.length > 0
                                                                         && (boardPane.comparingContinuations
                                                                             || gameList.continuationFilterActive
                                                                             || gameList.nextMoveInPatternCount > 0)

                                                                     text: {
                                                                         if (boardPane.comparingContinuations) {
                                                                             if (boardPane.comparisonStep === "A")
                                                                                 return qsTr("● Choose A")

                                                                             if (gameList.comparisonCandidateA
                                                                                     !== null) {
                                                                                 return qsTr("A %1 · ● Choose B")
                                                                                     .arg(
                                                                                         gameList
                                                                                             .comparisonCandidateA
                                                                                             .coordinate)
                                                                             }

                                                                             return qsTr("● Choose B")
                                                                         }

                                                                         if (gameList.continuationFilterActive) {
                                                                             return qsTr("● %1")
                                                                                 .arg(
                                                                                     gameList.goCoordinate(
                                                                                         gameList
                                                                                             .selectedContinuationX,
                                                                                         gameList
                                                                                             .selectedContinuationCoreY))
                                                                         }

                                                                         return qsTr("● Choose a continuation")
                                                                     }

                                                                     color: "#7d1e16"
                                                                     font.bold: true
                                                                 }

                                                                 Label {
                                                                     text: gameList.searchOutcomeText
                                                                     font.bold: true
                                                                 }
                                                             }

                                                             Item {
                                                                 Layout.fillWidth: true
                                                             }




                                                             Label {
                                                                 visible: !root.playingGame

                                                                 text: {
                                                                     if (boardPane.investigationMode === "pattern"
                                                                             && boardPane.selectingPattern)
                                                                         return qsTr("Drag over the board")

                                                                     if (boardPane.editingPosition
                                                                             && boardPane.editTool === "alternate") {
                                                                         return qsTr("Next: %1")
                                                                             .arg(
                                                                                 boardPane.alternateEditColour
                                                                                     === "black"
                                                                                 ? qsTr("Black")
                                                                                 : qsTr("White"))
                                                                     }

                                                                     return ""
                                                                 }

                                                                 opacity: 0.75
                                                             }
                                                         }

                                                         Frame {
                                                             id: katagoPanel

                                                             Layout.fillWidth: true
                                                             Layout.leftMargin: 8
                                                             Layout.rightMargin: 8

                                                             visible: !root.playingGame
                                                                      && boardPane.selectedGame !== null
                                                                      && studyToolsTabs.currentIndex === 1
                                                                      && boardPane.investigationMode === "katago"

                                                             Layout.fillHeight: false
                                                             Layout.preferredHeight: implicitHeight
                                                             Layout.maximumHeight: implicitHeight

                                                             property int analysisMoveNumber: -1
                                                             property int analysisVisitBudget: -1
                                                             property string analysisKomi: ""

                                                             Connections {
                                                                 target: boardPane

                                                                 function onSelectedGameChanged() {
                                                                     katagoKomiField.text =
                                                                         gameController.komi.length > 0
                                                                         ? gameController.komi
                                                                         : "6.5"
                                                                 }
                                                             }

                                                             contentItem: ColumnLayout {
                                                                 spacing: 4

                                                                 RowLayout {
                                                                     Layout.fillWidth: true
                                                                     spacing: 6

                                                                     Label {
                                                                         text: qsTr("Visit budget:")
                                                                     }

                                                                     SpinBox {
                                                                         id: katagoVisitBudgetSpinBox
                                                                         from: 10
                                                                         to: 100000
                                                                         stepSize: 50
                                                                         editable: true
                                                                         value: uiSettings.katagoVisitBudget
                                                                         enabled:
                                                                             !gameController
                                                                                 .katago_analysis_in_progress

                                                                         onValueModified:
                                                                             uiSettings.katagoVisitBudget = value
                                                                     }

                                                                     Label {
                                                                         text: qsTr("Komi:")
                                                                     }

                                                                     TextField {
                                                                         id: katagoKomiField

                                                                         Layout.preferredWidth:
                                                                             Kirigami.Units.gridUnit * 4

                                                                         text:
                                                                             gameController.komi.length > 0
                                                                             ? gameController.komi
                                                                             : "6.5"

                                                                         enabled:
                                                                             !gameController
                                                                                 .katago_analysis_in_progress

                                                                         inputMethodHints:
                                                                             Qt.ImhFormattedNumbersOnly
                                                                     }

                                                                     Label {
                                                                         visible: gameController.komi.length === 0
                                                                         text: qsTr("(assumed)")
                                                                         opacity: 0.7
                                                                     }

                                                                     Button {
                                                                         text:
                                                                             gameController
                                                                                 .katago_analysis_in_progress
                                                                             ? qsTr("Analysing…")
                                                                             : qsTr("Analyse")

                                                                         enabled:
                                                                             !gameController
                                                                                 .katago_analysis_in_progress

                                                                         onClicked: {
                                                                             katagoPanel.analysisMoveNumber =
                                                                                 gameController.move_number

                                                                             katagoPanel.analysisVisitBudget =
                                                                                 uiSettings.katagoVisitBudget

                                                                             katagoPanel.analysisKomi =
                                                                                 katagoKomiField.text

                                                                             gameController
                                                                                 .analyseCurrentPosition(
                                                                                     uiSettings
                                                                                         .katagoVisitBudget,
                                                                                     katagoKomiField.text,
                                                                                     katagoSettings.executable,
                                                                                     katagoSettings.model,
                                                                                     katagoSettings.config)
                                                                         }
                                                                     }

                                                                     Button {
                                                                         text: qsTr("Cancel")
                                                                         visible:
                                                                             gameController
                                                                                 .katago_analysis_in_progress

                                                                         onClicked:
                                                                             gameController.cancelKataGoAnalysis()
                                                                     }
                                                                 }

                                                                 Item {
                                                                     Layout.fillWidth: true
                                                                 }

                                                                 Label {
                                                                     Layout.fillWidth: true

                                                                     visible:
                                                                         gameController
                                                                             .katago_analysis_text.length > 0
                                                                         && katagoPanel.analysisMoveNumber
                                                                            === gameController.move_number
                                                                         && katagoPanel.analysisVisitBudget
                                                                            === uiSettings.katagoVisitBudget
                                                                         && katagoPanel.analysisKomi
                                                                            === katagoKomiField.text

                                                                     wrapMode: Text.NoWrap
                                                                     elide: Text.ElideRight
                                                                     text:
                                                                         gameController.katago_analysis_text
                                                                 }
                                                             }
                                                         }
                                        RowLayout {
                                            id: studyDisplayControls

                                            visible:
                                                !root.playingGame
                                                && boardPane.selectedGame !== null
                                                && !boardPane.editingPosition
                                                && studyToolsTabs.currentIndex === 2

                                            Layout.fillWidth: true
                                            Layout.fillHeight: false
                                            Layout.preferredHeight: implicitHeight
                                            Layout.maximumHeight: implicitHeight
                                            Layout.leftMargin: 8
                                            Layout.rightMargin: 8
                                            spacing: 4

                                            Label {
                                                text: qsTr("View:")
                                            }

                                            Button {
                                                text: qsTr("Influence")
                                                checkable: true
                                                checked: goBoard.influenceVisible
                                                highlighted: checked

                                                onToggled:
                                                    goBoard.influenceVisible = checked
                                            }




                                            Button {
                                                text: qsTr("↔")

                                                ToolTip.visible: hovered
                                                ToolTip.text:
                                                    qsTr("Flip board left to right")

                                                onClicked:
                                                    goBoard.flipViewLeftRight()
                                            }

                                            Button {
                                                text: qsTr("↕")

                                                ToolTip.visible: hovered
                                                ToolTip.text:
                                                    qsTr("Flip board top to bottom")

                                                onClicked:
                                                    goBoard.flipViewTopBottom()
                                            }

                                            Button {
                                                text: qsTr("↺")

                                                ToolTip.visible: hovered
                                                ToolTip.text:
                                                    qsTr("Rotate board 90° counter-clockwise")

                                                onClicked:
                                                    goBoard.rotateViewCounterClockwise()
                                            }

                                            Item {
                                                Layout.fillWidth: true
                                            }
                                        }

                                                                                TabBar {
                                            id: studyActionTabs
                                            property int branchSourceNode: -1
                                            property int branchSourceMove: -1

                                            visible:
                                                !root.playingGame
                                                && boardPane.selectedGame !== null
                                                && !boardPane.editingPosition
                                                && studyToolsTabs.currentIndex === 0

                                            Layout.alignment: Qt.AlignLeft
                                            Layout.leftMargin: 8
                                            Layout.rightMargin: 8
                                            Layout.preferredWidth: implicitWidth
                                            Layout.preferredHeight: visible ? implicitHeight : 0

                                            TabButton {
                                                text: qsTr("Annotate")
                                            }

                                            TabButton {
                                                text: qsTr("Moves")
                                            }

                                            TabButton {
                                                text: qsTr("Edit")
                                            }

                                                                                        onCurrentIndexChanged: {
                                                if (currentIndex !== 0)
                                                    boardPane.annotationTool = ""

                                                if (currentIndex !== 1) {
                                                    boardPane.sgfEditTool = ""
                                                    branchSourceNode = -1
                                                    branchSourceMove = -1
                                                }
                                            }
                                        }

RowLayout {
                                            id: studyMarkupControls

                                            visible:
                                                !root.playingGame
                                                && boardPane.selectedGame !== null
                                                && !boardPane.editingPosition
                                                && studyToolsTabs.currentIndex === 0
                                              && studyActionTabs.currentIndex === 0

                                            Layout.fillWidth: true
                                            Layout.fillHeight: false
                                            Layout.preferredHeight: implicitHeight
                                            Layout.maximumHeight: implicitHeight
                                            Layout.leftMargin: 8
                                            Layout.rightMargin: 8
                                            spacing: 4

                                            Label {
                                                text: qsTr("Tool:")
                                            }

                                            ComboBox {
                                                id: studyMarkupToolSelector

                                                Layout.preferredWidth:
                                                    Kirigami.Units.gridUnit * 9
                                                Layout.maximumWidth:
                                                    Kirigami.Units.gridUnit * 11

                                                model: [
                                                    { "text": qsTr("None"), "tool": "" },
                                                    { "text": qsTr("Cross"), "tool": "cross" },
                                                    { "text": qsTr("Triangle"), "tool": "triangle" },
                                                    { "text": qsTr("Circle"), "tool": "circle" },
                                                    { "text": qsTr("Square"), "tool": "square" },
                                                    { "text": qsTr("Letter"), "tool": "letter" },
                                                    { "text": qsTr("Number"), "tool": "number" },
                                                    { "text": qsTr("Label"), "tool": "label" }
                                                ]

                                                textRole: "text"

                                                currentIndex: {
                                                    for (let index = 0;
                                                         index < model.length;
                                                         ++index) {
                                                        if (model[index].tool
                                                                === boardPane.annotationTool) {
                                                            return index
                                                        }
                                                    }

                                                    return 0
                                                }

                                                onActivated: {
                                                    boardPane.annotationTool =
                                                        model[index].tool

                                                    if (model[index].tool.length > 0)
                                                        boardPane.sgfEditTool = ""
                                                }
                                            }

                                            Item {
                                                Layout.fillWidth: true
                                            }
                                        }



                                                                                ColumnLayout {
                                            id: studySgfEditControls

                                            visible:
                                                !root.playingGame
                                                && boardPane.selectedGame !== null
                                                && !boardPane.editingPosition
                                                && studyToolsTabs.currentIndex === 0

                                            Layout.fillWidth: true
                                            Layout.fillHeight: false
                                            Layout.preferredHeight: implicitHeight
                                            Layout.maximumHeight: implicitHeight
                                            Layout.leftMargin: 8
                                            Layout.rightMargin: 8
                                            spacing: 4

                                            RowLayout {
            visible: studyActionTabs.currentIndex === 1
                                                Layout.fillWidth: true
                                                spacing: 4

                                                Button {
                                                    text: qsTr("Add move")
                                                                                                        checkable: true
                                                    checked: boardPane.sgfEditTool === "move"
                                                    highlighted: checked

                                                    ToolTip.visible: hovered
                                                    ToolTip.text: qsTr(
                                                        "Click an empty point to add a continuation. "
                                                        + "Clicking an existing continuation follows it.")

                                                                                                        onClicked: {
                                                        boardPane.annotationTool = ""
                                                        studyActionTabs.branchSourceNode = -1
                                                        studyActionTabs.branchSourceMove = -1
                                                        boardPane.sgfEditTool = checked ? "move" : ""
                                                    }
                                                }

                                                Button {
                                                    text: qsTr("Pass")

                                                    ToolTip.visible: hovered
                                                    ToolTip.text: qsTr(
                                                        "Add a pass as the next SGF continuation.")

                                                                                                        onClicked: {
                                                        boardPane.annotationTool = ""

                                                        const branching =
                                                            boardPane.sgfEditTool === "branch"

                                                        if (branching) {
                                                            if (studyActionTabs.branchSourceNode < 0
                                                                    || !gameController.showStudyStructureNode(
                                                                        studyActionTabs.branchSourceNode)) {
                                                                console.warn(gameController.error_message)
                                                                return
                                                            }
                                                        }

                                                        const ok = branching
                                                            ? gameController.addStudyBranchPass()
                                                            : gameController.addStudyPass()

                                                        if (ok) {
                                                            if (branching
                                                                    && !gameController.showStudyStructureNode(
                                                                        studyActionTabs.branchSourceNode)) {
                                                                console.warn(gameController.error_message)
                                                            }

                                                            boardPane.applyLoadedPosition()
                                                        } else {
                                                            console.warn(gameController.error_message)
                                                        }
                                                    }
                                                }

                                                Label {
                                                    visible: boardPane.sgfEditTool === "branch"
                                                    text: qsTr("Branching from move %1 — add alternatives here").arg(studyActionTabs.branchSourceMove)
                                                    font.italic: true
                                                    color: Kirigami.Theme.highlightColor
                                                }

                                                Button {
                                                    visible: boardPane.sgfEditTool === "branch"
                                                    text: qsTr("Cancel")

                                                    ToolTip.visible: hovered
                                                    ToolTip.text: qsTr("Cancel branch creation.")

                                                    onClicked: {
                                                        boardPane.sgfEditTool = ""
                                                        studyActionTabs.branchSourceNode = -1
                                                        studyActionTabs.branchSourceMove = -1
                                                    }
                                                }

                                                Item {
                                                    Layout.fillWidth: true
                                                }
                                            }

                                            RowLayout {
            visible: studyActionTabs.currentIndex === 2
                                                Layout.fillWidth: true
                                                spacing: 4

                                                Button {
                                                    text: qsTr("Undo")
                                                    enabled: gameController.can_undo_study_edit

                                                    ToolTip.visible: hovered
                                                    ToolTip.text: qsTr(
                                                        "Undo the most recent edit made since this Study was opened.")

                                                    onClicked: {
                                                        boardPane.annotationTool = ""
                                                        boardPane.sgfEditTool = ""

                                                        if (gameController.undoStudyEdit()) {
                                                            boardPane.applyLoadedPosition()
                                                        } else {
                                                            console.warn(gameController.error_message)
                                                        }
                                                    }
                                                }

                                                Button {
                                                    text: qsTr("Redo")
                                                    enabled: gameController.can_redo_study_edit

                                                    ToolTip.visible: hovered
                                                    ToolTip.text: qsTr(
                                                        "Redo the most recently undone Study edit.")

                                                    onClicked: {
                                                        boardPane.annotationTool = ""
                                                        boardPane.sgfEditTool = ""

                                                        if (gameController.redoStudyEdit()) {
                                                            boardPane.applyLoadedPosition()
                                                        } else {
                                                            console.warn(gameController.error_message)
                                                        }
                                                    }
                                                }

                                                Button {
                                                    text: qsTr("Tree ▾")

                                                    ToolTip.visible: hovered
                                                    ToolTip.text: qsTr(
                                                        "Structural game-tree editing commands.")

                                                    onClicked: studyTreeEditMenu.open()

                                                    Menu {
                                                        id: studyTreeEditMenu

                                                                                                                MenuItem {
                                                            text: qsTr("Branch")
                                                            enabled:
                                                                gameController.studyBranchAvailable(
                                                                    gameController.move_number)

                                                                                                                        onTriggered: {
                                                                boardPane.annotationTool = ""
                                                                studyActionTabs.branchSourceNode =
                                                                    gameController.sgf_tree_current_node
                                                                studyActionTabs.branchSourceMove =
                                                                    gameController.move_number
                                                                studyActionTabs.currentIndex = 1
                                                                boardPane.sgfEditTool = "branch"
                                                            }
                                                        }
MenuItem {
                                                            text: qsTr("Insert node")

                                                            onTriggered: {
                                                                boardPane.annotationTool = ""
                                                                boardPane.sgfEditTool = ""

                                                                if (gameController.insertStudyNode()) {
                                                                    boardPane.applyLoadedPosition()
                                                                } else {
                                                                    console.warn(
                                                                        gameController.error_message)
                                                                }
                                                            }
                                                        }

                                                        MenuSeparator {}

                                                        MenuItem {
                                                            text: qsTr("Prune from here")
                                                            enabled:
                                                                gameController.sgf_tree_current_node > 0

                                                            onTriggered: {
                                                                boardPane.annotationTool = ""
                                                                boardPane.sgfEditTool = ""

                                                                if (gameController.deleteStudyFromHere()) {
                                                                    boardPane.applyLoadedPosition()
                                                                } else {
                                                                    console.warn(
                                                                        gameController.error_message)
                                                                }
                                                            }
                                                        }
                                                    }
                                                }

                                                Item {
                                                    Layout.fillWidth: true
                                                }
                                            }
                                        }


                                        // closes the new gameDetailsFrame
                                        Item {
                                            id: studyBottomSpacer
                                            Layout.fillWidth: true
                                            Layout.fillHeight: true
                                        }
                    }

                    ColumnLayout {
                        spacing: Kirigami.Units.smallSpacing

                        RowLayout {
                            Layout.fillWidth: true

                            Label {
                                text: qsTr("Saved studies")
                                font.bold: true
                                font.pixelSize: 16
                            }

                            Item {
                                Layout.fillWidth: true
                            }

                            ToolButton {
                                text: qsTr("Refresh")

                                onClicked: {
                                    studyLibraryDialog.refreshEntries()
                                    studyLibraryEmbeddedList.currentIndex = -1
                                }
                            }
                        }

                        Label {
                            Layout.fillWidth: true

                            visible:
                                studyLibraryDialog.entries.length === 0

                            text:
                                gameController.error_message.length > 0
                                ? gameController.error_message
                                : qsTr("No saved Study documents yet.")

                            wrapMode: Text.WordWrap
                            horizontalAlignment: Text.AlignHCenter
                            verticalAlignment: Text.AlignVCenter
                        }

                        ListView {
                            id: studyLibraryEmbeddedList

                            Layout.fillWidth: true
                            Layout.fillHeight: true

                            visible:
                                studyLibraryDialog.entries.length > 0

                            clip: true
                            spacing: 1
                            model: studyLibraryDialog.entries
                            currentIndex: -1

                            delegate: ItemDelegate {
                                required property int index
                                required property var modelData

                                width: ListView.view.width

                                highlighted:
                                    studyLibraryEmbeddedList.currentIndex
                                    === index

                                onClicked:
                                    studyLibraryEmbeddedList.currentIndex =
                                        index

                                contentItem: ColumnLayout {
                                    spacing: 2

                                    Label {
                                        Layout.fillWidth: true
                                        text: modelData.title
                                        font.bold: true
                                        elide: Text.ElideRight
                                    }

                                    Label {
                                        Layout.fillWidth: true

                                        text:
                                            studyLibraryDialog.details(
                                                modelData)

                                        opacity: 0.72
                                        elide: Text.ElideRight
                                    }
                                }
                            }

                            ScrollBar.vertical: ScrollBar {}
                        }

                        RowLayout {
                            Layout.fillWidth: true

                            Button {
                                text: qsTr("Delete…")

                                enabled:
                                    studyLibraryEmbeddedList.currentIndex >= 0

                                onClicked: {
                                    const index =
                                        studyLibraryEmbeddedList.currentIndex

                                    if (index < 0
                                            || index
                                               >= studyLibraryDialog
                                                   .entries.length) {
                                        return
                                    }

                                    studyLibraryDialog.pendingDeleteEntry =
                                        studyLibraryDialog.entries[index]

                                    studyLibraryDeleteDialog.open()
                                }
                            }

                            Item {
                                Layout.fillWidth: true
                            }

                            Button {
                                text: qsTr("Open")
                                highlighted: true

                                enabled:
                                    studyLibraryEmbeddedList.currentIndex >= 0

                                onClicked: {
                                    const index =
                                        studyLibraryEmbeddedList.currentIndex

                                    if (index < 0
                                            || index
                                               >= studyLibraryDialog
                                                   .entries.length) {
                                        return
                                    }

                                    const entry =
                                        studyLibraryDialog.entries[index]

                                    root.openStudySgfPath(
                                        entry.path,
                                        entry.title,
                                        qsTr("Study Library"))
                                }
                            }
                        }
                    }

                    ColumnLayout {
                        spacing: Kirigami.Units.smallSpacing

                        RowLayout {
                            Layout.fillWidth: true

                            Label {
                                text: qsTr("OGS Joseki Explorer")
                                font.bold: true
                                font.pixelSize: 16
                            }

                            Item {
                                Layout.fillWidth: true
                            }

                            BusyIndicator {
                                running: josekiModel.loading
                                visible: running
                                implicitWidth:
                                    Kirigami.Units.gridUnit * 1.5
                                implicitHeight: implicitWidth
                            }
                        }

                        Label {
                            Layout.fillWidth: true
                            visible: josekiModel.status_message.length > 0

                            text: josekiModel.status_message
                            opacity: 0.65
                            elide: Text.ElideRight
                        }

                        Label {
                            Layout.fillWidth: true
                            visible: josekiModel.error_message.length > 0

                            text: josekiModel.error_message
                            wrapMode: Text.WordWrap
                        }

                        Frame {
                            Layout.fillWidth: true
                            visible:
                                josekiModel.node_id.length > 0
                                && !josekiModel.loading

                            contentItem: ColumnLayout {
                                spacing: 3

                                Label {
                                    Layout.fillWidth: true

                                    text: josekiModel.node_id === "root"
                                          ? qsTr("Root position")
                                          : qsTr("Position %1")
                                                .arg(josekiModel.node_id)

                                    font.bold: true
                                }

                                Label {
                                    Layout.fillWidth: true
                                    visible:
                                        josekiModel.category.length > 0

                                    text: qsTr("Last move: %1")
                                          .arg(josekiModel.category)

                                    opacity: 0.75
                                }

                                Label {
                                    Layout.fillWidth: true
                                    visible:
                                        josekiModel.tags_text.length > 0

                                    text: josekiModel.tags_text
                                    wrapMode: Text.WordWrap
                                    opacity: 0.75
                                }

                                Label {
                                    Layout.fillWidth: true
                                    visible:
                                        josekiModel.source_description
                                            .length > 0

                                    text: qsTr("Source: %1")
                                          .arg(
                                              josekiModel
                                                  .source_description)

                                    wrapMode: Text.WordWrap
                                    opacity: 0.75
                                }

                                Label {
                                    Layout.fillWidth: true
                                    visible:
                                        josekiModel.description.length > 0

                                    text: josekiModel.description
                                    textFormat: Text.MarkdownText
                                    wrapMode: Text.WordWrap

                                    onLinkActivated:
                                        function(link) {
                                            Qt.openUrlExternally(link)
                                        }
                                }
                            }
                        }


                        Item {
                            Layout.fillWidth: true
                            Layout.fillHeight: true
                        }

                        RowLayout {
                            Layout.fillWidth: true

                            Button {
                                text: qsTr("Back")
                                enabled:
                                    !josekiModel.loading
                                    && josekiModel
                                           .parent_node_id.length > 0

                                onClicked: josekiModel.goBack()
                            }

                            Button {
                                text: qsTr("Root")
                                enabled:
                                    !josekiModel.loading
                                    && josekiModel.node_id !== "root"

                                onClicked:
                                    josekiModel.loadPosition("root")
                            }

                            Button {
                                text: qsTr("Refresh")
                                enabled: !josekiModel.loading
                                onClicked: josekiModel.refresh()
                            }

                            Item {
                                Layout.fillWidth: true
                            }
                        }

                        RowLayout {
                            Layout.fillWidth: true

                            Button {
                                text: qsTr("View on OGS")

                                enabled:
                                    josekiModel.node_id.length > 0

                                onClicked: {
                                    const suffix =
                                        josekiModel.node_id === "root"
                                        ? ""
                                        : "/" + josekiModel.node_id

                                    Qt.openUrlExternally(
                                        "https://online-go.com/joseki"
                                        + suffix)
                                }
                            }

                            Item {
                                Layout.fillWidth: true
                            }

                            Button {
                                text: qsTr("Study this position")
                                highlighted: true

                                enabled:
                                    !josekiModel.loading
                                    && josekiModel
                                           .study_sgf_path.length > 0

                                onClicked:
                                    root.studyCurrentJosekiPosition()
                            }
                        }
                    }
                }
            }
        }

                Pane {
            id: boardPane

            visible: true

            property var selectedGame: null


            property string annotationTool: ""
            property string sgfEditTool: ""
            property int pendingStudyLabelX: -1
            property int pendingStudyLabelY: -1

            function patternContextText() {
                if (!goBoard.patternSelectionValid)
                    return ""

                const left =
                    Math.min(
                        goBoard.patternStartX,
                        goBoard.patternEndX)
                const right =
                    Math.max(
                        goBoard.patternStartX,
                        goBoard.patternEndX)
                const top =
                    Math.min(
                        goBoard.patternStartY,
                        goBoard.patternEndY)
                const bottom =
                    Math.max(
                        goBoard.patternStartY,
                        goBoard.patternEndY)

                const includesLeft = left === 0
                const includesRight =
                    right === goBoard.boardSize - 1
                const includesTop = top === 0
                const includesBottom =
                    bottom === goBoard.boardSize - 1

                const edgeCount =
                    (includesLeft ? 1 : 0)
                    + (includesRight ? 1 : 0)
                    + (includesTop ? 1 : 0)
                    + (includesBottom ? 1 : 0)

                const includesCorner =
                    (includesLeft
                     && (includesTop || includesBottom))
                    || (includesRight
                        && (includesTop || includesBottom))

                if (includesCorner)
                    return qsTr("Search context: corner")

                if (edgeCount === 1)
                    return qsTr("Search context: side")

                if (edgeCount === 0)
                    return qsTr("Search context: no board edge")

                return qsTr("Search context: %1 board edges")
                    .arg(edgeCount)
            }

            function toggleAnnotationTool(tool) {
                annotationTool =
                    annotationTool === tool ? "" : tool
            }

            function sourceBoardMarkup() {
                try {
                    return JSON.parse(
                        gameController.studyBoardMarkupJson(
                            gameController.move_number))
                } catch (error) {
                    console.warn(
                        "Could not parse Study board markup:",
                        error)
                    return []
                }
            }

            function refreshBoardMarkup() {
                goBoard.markup = sourceBoardMarkup()
            }

            function annotatePoint(x, y) {
                const tool = annotationTool

                if (tool.length === 0)
                    return false

                if (tool === "label") {
                    pendingStudyLabelX = x
                    pendingStudyLabelY = y
                    studyLabelField.text = ""
                    studyLabelDialog.open()
                    return true
                }

                let text = ""

                if (tool === "number") {
                    const moveNumber =
                        gameController.stoneMoveNumber(
                            gameController.move_number,
                            x,
                            y)

                    if (moveNumber < 0)
                        return false

                    text = String(moveNumber)
                }

                if (!gameController.setStudyAnnotation(
                        gameController.move_number,
                        x,
                        y,
                        tool,
                        text)) {
                    console.warn(gameController.error_message)
                    return false
                }

                refreshBoardMarkup()
                return true
            }

            function commitPendingStudyLabel(text) {
                const label = text.trim()

                if (label.length === 0) {
                    cancelPendingStudyLabel()
                    return
                }

                if (pendingStudyLabelX < 0
                        || pendingStudyLabelY < 0) {
                    return
                }

                if (!gameController.setStudyAnnotation(
                        gameController.move_number,
                        pendingStudyLabelX,
                        pendingStudyLabelY,
                        "label",
                        label)) {
                    console.warn(gameController.error_message)
                    return
                }

                pendingStudyLabelX = -1
                pendingStudyLabelY = -1

                refreshBoardMarkup()
            }

            function cancelPendingStudyLabel() {
                pendingStudyLabelX = -1
                pendingStudyLabelY = -1
            }

            function searchSelectedPattern(destinationProjectPath) {
                if (destinationProjectPath.length === 0)
                    return false

                investigationMode = "pattern"

                if (!boardPane.investigatingSearch)
                    previousSearchProjectPath = ""

                if (!boardPane.beginSearchSession())
                    return false

                /*
                 * Capture every query input while the source workspace is
                 * still active. Showing Pattern results must not alter them.
                 */
                const resultsBelongToStudy =
                    root.studyWorkspaceActive

                const boardSize =
                    gameController.board_size

                const stonesJson =
                    gameController.stones_json

                const left =
                    boardPane.patternLeft

                const width =
                    boardPane.patternRight
                    - left + 1

                const height =
                    boardPane.patternBottom
                    - boardPane.patternTop + 1

                const bottom =
                    goBoard.boardSize - 1
                    - boardPane.patternBottom

                goBoard.continuationPoints = []

                gameList.searchProject(
                    destinationProjectPath,
                    boardSize,
                    stonesJson,
                    left,
                    bottom,
                    width,
                    height,
                    root.includeHandicapGames)

                root.patternResultsWorkspaceIsStudy =
                    resultsBelongToStudy

                root.showPatternResults()

                return true
            }

            function showSamePatternResults(
                    destinationProjectPath) {
                if (!gameList.searchHasRunFor(destinationProjectPath))
                    return false

                const left = gameList.searchPatternLeft
                const bottom = gameList.searchPatternBottom
                const width = gameList.searchPatternWidth
                const height = gameList.searchPatternHeight

                const sourceGame = boardPane.searchSourceGame
                const sourceEditing =
                    boardPane.searchSourceEditingPosition
                const sourceTransform =
                    boardPane.searchSourceViewTransform

                /*
                 * restoreSearchSource consumes the backend snapshot.
                 * Recreate it immediately so Database Results / My Games
                 * Results can be switched repeatedly.
                 */
                if (!gameController.restoreSearchSource()) {
                    console.warn(gameController.error_message)
                    return false
                }

                boardPane.selectedGame = sourceGame
                boardPane.editingPosition = sourceEditing
                boardPane.applyLoadedPosition()

                if (sourceTransform !== null)
                    goBoard.setViewTransform(sourceTransform)

                boardPane.clearMatchNavigation()

                boardPane.patternLeft = left
                boardPane.patternRight = left + width - 1
                boardPane.patternBottom =
                    goBoard.boardSize - 1 - bottom
                boardPane.patternTop =
                    boardPane.patternBottom - height + 1

                goBoard.setPatternSelection(
                    boardPane.patternLeft,
                    boardPane.patternTop,
                    boardPane.patternRight,
                    boardPane.patternBottom)

                if (!gameController.snapshotSearchSource()) {
                    console.warn(gameController.error_message)
                    return false
                }

                root.showPatternResults()

                return gameList.showSearchResults(
                    destinationProjectPath)
            }

            function searchSamePatternIn(destinationProjectPath) {
                if (!boardPane.investigatingSearch
                        || destinationProjectPath.length === 0) {
                    return false
                }

                /*
                 * A selected search result moves the blue rectangle to that
                 * occurrence. Reuse the geometry of the original query,
                 * rather than the geometry of whichever match is visible.
                 */
                const left = gameList.searchPatternLeft
                const bottom = gameList.searchPatternBottom
                const width = gameList.searchPatternWidth
                const height = gameList.searchPatternHeight

                const sourceGame = boardPane.searchSourceGame
                const sourceEditing =
                    boardPane.searchSourceEditingPosition
                const sourceTransform =
                    boardPane.searchSourceViewTransform

                if (!gameController.restoreSearchSource()) {
                    console.warn(gameController.error_message)
                    return false
                }

                boardPane.selectedGame = sourceGame
                boardPane.editingPosition = sourceEditing
                boardPane.applyLoadedPosition()

                if (sourceTransform !== null)
                    goBoard.setViewTransform(sourceTransform)

                boardPane.clearMatchNavigation()

                boardPane.patternLeft = left
                boardPane.patternRight = left + width - 1
                boardPane.patternBottom =
                    goBoard.boardSize - 1 - bottom
                boardPane.patternTop =
                    boardPane.patternBottom - height + 1

                goBoard.setPatternSelection(
                    boardPane.patternLeft,
                    boardPane.patternTop,
                    boardPane.patternRight,
                    boardPane.patternBottom)

                return boardPane.searchSelectedPattern(
                    destinationProjectPath)
            }

            property bool editingPosition: false

            property string editTool: "black"
            property string alternateEditColour: "black"
            property bool selectingPattern: false

            onSelectingPatternChanged: {
                if (selectingPattern)
                    investigationMode = "pattern"
            }

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
            property string previousSearchProjectPath: ""

            property bool comparingContinuations: false
            property string comparisonStep: "A"

            /*
             * The user explicitly chooses which evidence source owns the
             * investigation area.  The empty string is ordinary game review.
             */
            property string investigationMode: ""

            /*
             * Pattern Search and KataGo share one stable-height
             * investigation slot so switching modes does not resize
             * the goban.
             */
            readonly property real investigationBodyHeight:
                Kirigami.Units.gridUnit * 3

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

            function rememberSearchReturn(destinationProjectPath) {
                const currentProjectPath =
                    gameList.currentSearchProjectPath

                if (currentProjectPath.length > 0
                        && currentProjectPath
                           !== destinationProjectPath) {
                    previousSearchProjectPath =
                        currentProjectPath
                }
            }

            readonly property bool canReturnInInvestigation:
                investigatingSearch
                && (comparingContinuations
                    || gameList.continuationFilterActive
                    || showingContinuationComparison
                    || (selectedGame !== null
                        && selectedGame.fromSearchResults === true)
                    || (previousSearchProjectPath.length > 0
                        && gameList.searchHasRunFor(
                            previousSearchProjectPath)))

            function returnToPreviousInvestigation() {
                /*
                 * Back undoes the current investigative step while
                 * preserving the pattern-search session.
                 */

                if (comparingContinuations) {
                    cancelContinuationComparison()
                    return true
                }

                /*
                 * A comparison's "Show games" route produces a
                 * continuation filter.  Clear that first so the
                 * comparison itself remains available.
                 */
                if (gameList.continuationFilterActive) {
                    gameList.clearContinuationFilter()
                    return true
                }

                if (showingContinuationComparison) {
                    gameList.comparisonCandidateA = null
                    gameList.comparisonCandidateB = null
                    return true
                }

                /*
                 * A selected search-result game is one level below
                 * the result overview.  Restore that overview without
                 * abandoning the search pattern.
                 */
                if (selectedGame !== null
                        && selectedGame.fromSearchResults === true) {
                    return boardPane.showSamePatternResults(
                        gameList.currentSearchProjectPath)
                }

                /*
                 * Finally handle a switch between Database and My
                 * Games evidence for the same pattern.
                 */
                const previousProjectPath =
                    previousSearchProjectPath

                if (previousProjectPath.length === 0
                        || !gameList.searchHasRunFor(
                            previousProjectPath)) {
                    return false
                }

                previousSearchProjectPath = ""

                return boardPane.showSamePatternResults(
                    previousProjectPath)
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

            function adjustSearchArea() {
                if (!investigatingSearch)
                    return false

                /*
                 * patternLeft/patternTop may currently describe the
                 * occurrence displayed in a result game.  Preserve the
                 * geometry of the original query before clearing the
                 * search results.
                 */
                const left = gameList.searchPatternLeft
                const bottom = gameList.searchPatternBottom
                const width = gameList.searchPatternWidth
                const height = gameList.searchPatternHeight

                const sourceGame = searchSourceGame
                const sourceEditing = searchSourceEditingPosition
                const sourceTransform = searchSourceViewTransform

                previousSearchProjectPath = ""
                investigationMode = "pattern"
                comparingContinuations = false
                comparisonStep = "A"

                /*
                 * restoreSearchSource consumes the backend snapshot.
                 * That is intentional here: resizing starts a revised
                 * search, whose Search action will snapshot the source
                 * again through beginSearchSession().
                 */
                if (!gameController.restoreSearchSource()) {
                    console.warn(gameController.error_message)
                    return false
                }

                selectedGame = sourceGame
                editingPosition = sourceEditing
                applyLoadedPosition()

                if (sourceTransform !== null)
                    goBoard.setViewTransform(sourceTransform)

                gameList.clearSearchResults()
                clearMatchNavigation()

                patternLeft = left
                patternRight = left + width - 1
                patternBottom =
                    goBoard.boardSize - 1 - bottom
                patternTop =
                    patternBottom - height + 1

                goBoard.setPatternSelection(
                    patternLeft,
                    patternTop,
                    patternRight,
                    patternBottom)

                searchSourceGame = null
                searchSourceEditingPosition = false
                searchSourceViewTransform = null
                goBoard.hoverValid = false

                return true
            }

            function beginNewSearch() {
                previousSearchProjectPath = ""
                investigationMode = "pattern"
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


            function applyJosekiPosition() {
                if (root.studyPaneMode !== "joseki"
                        || josekiModel.loading
                        || josekiModel.node_id.length === 0) {
                    return
                }

                try {
                    goBoard.boardSize = 19
                    goBoard.stones =
                        JSON.parse(josekiModel.stones_json)
                    goBoard.markup =
                        JSON.parse(josekiModel.markup_json)

                    const continuations =
                        JSON.parse(
                            josekiModel.continuations_json)

                    goBoard.continuationPoints =
                        continuations
                            .filter(function(point) {
                                return Number(point.x) >= 0
                                       && Number(point.y) >= 0
                            })
                            .map(function(point) {
                                return {
                                    "x": Number(point.x),
                                    "y": Number(point.y),
                                    "count": 1,
                                    "category": point.category,
                                    "label": point.label,
                                    "placement": point.placement
                                }
                            })

                    goBoard.selectedContinuationX = -1
                    goBoard.selectedContinuationY = -1
                    goBoard.lastMoveX =
                        josekiModel.last_move_x
                    goBoard.lastMoveY =
                        josekiModel.last_move_y
                    goBoard.lastMoveNumber =
                        josekiModel.move_count

                    boardPane.selectingPattern = false
                    goBoard.clearPatternSelection()
                } catch (error) {
                    console.warn(
                        "Could not display OGS joseki position:",
                        error)
                }
            }

            function applyLoadedPosition() {
                goBoard.boardSize = gameController.board_size
                goBoard.stones = JSON.parse(
                            gameController.stones_json)

                boardPane.refreshBoardMarkup()
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
                if (root.playingGame)
                    return

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




            SplitView.minimumWidth:
                Math.min(
                    780,
                    Math.max(
                        420,
                        mainSplitView.width
                        - (root.browserExpanded ? 420 : 360)))
            SplitView.preferredWidth: 780

            ColumnLayout {
                anchors.fill: parent
                spacing: 6

                Frame {
                    id: boardFrame

                    Layout.fillWidth: true
                    Layout.fillHeight: true

                    /*
                     * The GoBoard has a generous implicit size, but the right
                     * pane may legitimately be narrower. Never let that
                     * preferred size make the frame extend beyond the visible
                     * pane.
                     */
                    Layout.minimumWidth: 0
                    Layout.preferredWidth: boardPane.availableWidth
                    Layout.maximumWidth: boardPane.availableWidth

                    /*
                     * Prefer a full-width square goban when vertical space
                     * permits, but allow the board area to shrink so that the
                     * controls and game information below it remain visible.
                     */
                    Layout.preferredHeight: width
                    Layout.minimumHeight:
                        Kirigami.Units.gridUnit * 8
                    Layout.maximumHeight: width

                    padding: 4

                    Frame {
                        id: sgfTreeFrame

                        anchors.left: parent.left
                        anchors.top: parent.top
                        anchors.bottom: parent.bottom

                        width:
                            Math.max(
                                0,
                                goBoard.x
                                - Kirigami.Units.smallSpacing)

                        visible:
                            root.studyPaneMode !== "joseki"
                            && gameController.sgf_tree_json !== "[]"
                            && width >= Kirigami.Units.gridUnit * 8

                        padding: 4
                        clip: true

                        contentItem: ColumnLayout {
                            spacing: 2

                            Label {
                                Layout.fillWidth: true
                                text: qsTr("Game tree")
                                font.bold: true
                            }

                            Flickable {
                                id: sgfTreeFlick

                                Layout.fillWidth: true
                                Layout.fillHeight: true

                                clip: true
                                boundsBehavior: Flickable.StopAtBounds
                                flickableDirection:
                                    Flickable.HorizontalAndVerticalFlick

                                contentWidth:
                                    Math.max(
                                        width,
                                        sgfTreeCanvas.treeWidth)

                                contentHeight:
                                    Math.max(
                                        height,
                                        sgfTreeCanvas.treeHeight)

                                ScrollBar.vertical: ScrollBar {
                                    policy:
                                        sgfTreeFlick.contentHeight
                                            > sgfTreeFlick.height
                                        ? ScrollBar.AlwaysOn
                                        : ScrollBar.AsNeeded
                                }

                                ScrollBar.horizontal: ScrollBar {
                                    policy:
                                        sgfTreeFlick.contentWidth
                                            > sgfTreeFlick.width
                                        ? ScrollBar.AlwaysOn
                                        : ScrollBar.AsNeeded
                                }

                                Canvas {
                                    id: sgfTreeCanvas

                                    property var nodes: {
                                        try {
                                            return JSON.parse(
                                                gameController
                                                    .sgf_tree_json)
                                        } catch (error) {
                                            return []
                                        }
                                    }

                                    property int currentNode:
                                        gameController
                                            .sgf_tree_current_node

                                    property real rowSpacing: 24
                                    property real laneSpacing: 30

                                    property int maxRow: {
                                        let result = 0

                                        for (const node of nodes) {
                                            result = Math.max(
                                                result,
                                                Number(node.row))
                                        }

                                        return result
                                    }

                                    property int maxLane: {
                                        let result = 0

                                        for (const node of nodes) {
                                            result = Math.max(
                                                result,
                                                Number(node.lane))
                                        }

                                        return result
                                    }

                                    property real treeWidth:
                                        44
                                        + (maxLane + 1)
                                          * laneSpacing

                                    property real treeHeight:
                                        44
                                        + (maxRow + 1)
                                          * rowSpacing

                                    width:
                                        Math.max(
                                            sgfTreeFlick.width,
                                            treeWidth)

                                    height:
                                        Math.max(
                                            sgfTreeFlick.height,
                                            treeHeight)

                                    function nodeX(node) {
                                        return 22
                                               + Number(node.lane)
                                                 * laneSpacing
                                    }

                                    function nodeY(node) {
                                        return 22
                                               + Number(node.row)
                                                 * rowSpacing
                                    }

                                    function nodeById(id) {
                                        for (const node of nodes) {
                                            if (Number(node.id)
                                                    === Number(id)) {
                                                return node
                                            }
                                        }

                                        return null
                                    }

                                    function nodeAt(x, y) {
                                        let nearest = null
                                        let nearestDistance = 11

                                        for (const node of nodes) {
                                            const dx =
                                                x - nodeX(node)
                                            const dy =
                                                y - nodeY(node)
                                            const distance =
                                                Math.sqrt(
                                                    dx * dx
                                                    + dy * dy)

                                            if (distance
                                                    < nearestDistance) {
                                                nearest = node
                                                nearestDistance =
                                                    distance
                                            }
                                        }

                                        return nearest
                                    }

                                    function ensureCurrentVisible() {
                                        const node =
                                            nodeById(currentNode)

                                        if (node === null)
                                            return

                                        const x = nodeX(node)
                                        const y = nodeY(node)
                                        const margin = 36

                                        if (y - margin
                                                < sgfTreeFlick
                                                    .contentY) {
                                            sgfTreeFlick.contentY =
                                                Math.max(
                                                    0,
                                                    y - margin)
                                        } else if (
                                            y + margin
                                            > sgfTreeFlick.contentY
                                              + sgfTreeFlick.height) {
                                            sgfTreeFlick.contentY =
                                                Math.min(
                                                    Math.max(
                                                        0,
                                                        sgfTreeFlick
                                                            .contentHeight
                                                        - sgfTreeFlick
                                                            .height),
                                                    y + margin
                                                    - sgfTreeFlick
                                                        .height)
                                        }

                                        if (x - margin
                                                < sgfTreeFlick
                                                    .contentX) {
                                            sgfTreeFlick.contentX =
                                                Math.max(
                                                    0,
                                                    x - margin)
                                        } else if (
                                            x + margin
                                            > sgfTreeFlick.contentX
                                              + sgfTreeFlick.width) {
                                            sgfTreeFlick.contentX =
                                                Math.min(
                                                    Math.max(
                                                        0,
                                                        sgfTreeFlick
                                                            .contentWidth
                                                        - sgfTreeFlick
                                                            .width),
                                                    x + margin
                                                    - sgfTreeFlick
                                                        .width)
                                        }
                                    }

                                    onNodesChanged: {
                                        requestPaint()
                                        Qt.callLater(
                                            ensureCurrentVisible)
                                    }

                                    onCurrentNodeChanged: {
                                        requestPaint()
                                        Qt.callLater(
                                            ensureCurrentVisible)
                                    }

                                    onWidthChanged:
                                        requestPaint()

                                    onHeightChanged:
                                        requestPaint()

                                    onPaint: {
                                        const ctx =
                                            getContext("2d")

                                        ctx.reset()

                                        ctx.globalAlpha = 0.35
                                        ctx.strokeStyle =
                                            Kirigami.Theme
                                                .textColor
                                        ctx.lineWidth = 1.5

                                        for (const node of nodes) {
                                            if (node.parent === null
                                                    || node.parent
                                                       === undefined) {
                                                continue
                                            }

                                            const parent =
                                                nodeById(node.parent)

                                            if (parent === null)
                                                continue

                                            const parentX =
                                                nodeX(parent)
                                            const parentY =
                                                nodeY(parent)
                                            const x = nodeX(node)
                                            const y = nodeY(node)

                                            ctx.beginPath()
                                            ctx.moveTo(
                                                parentX,
                                                parentY)
                                            ctx.lineTo(
                                                parentX,
                                                y)
                                            ctx.lineTo(
                                                x,
                                                y)
                                            ctx.stroke()
                                        }

                                        ctx.globalAlpha = 1.0

                                        for (const node of nodes) {
                                            const x = nodeX(node)
                                            const y = nodeY(node)
                                            const current =
                                                Number(node.id)
                                                === currentNode

                                            if (current) {
                                                ctx.strokeStyle =
                                                    Kirigami.Theme
                                                        .highlightColor
                                                ctx.lineWidth = 3
                                                ctx.beginPath()
                                                ctx.arc(
                                                    x,
                                                    y,
                                                    8,
                                                    0,
                                                    Math.PI * 2)
                                                ctx.stroke()
                                            }

                                            ctx.lineWidth = 1.5
                                            ctx.strokeStyle =
                                                Kirigami.Theme
                                                    .textColor

                                            if (node.colour
                                                    === "black") {
                                                ctx.fillStyle =
                                                    "#202020"
                                            } else if (
                                                node.colour
                                                === "white") {
                                                ctx.fillStyle =
                                                    "#f5f5f5"
                                            } else {
                                                ctx.fillStyle =
                                                    Kirigami.Theme
                                                        .backgroundColor
                                            }

                                            if (node.isMove
                                                    || node.root) {
                                                ctx.beginPath()
                                                ctx.arc(
                                                    x,
                                                    y,
                                                    5.5,
                                                    0,
                                                    Math.PI * 2)
                                                ctx.fill()
                                                ctx.stroke()
                                            } else {
                                                ctx.fillRect(
                                                    x - 4.5,
                                                    y - 4.5,
                                                    9,
                                                    9)
                                                ctx.strokeRect(
                                                    x - 4.5,
                                                    y - 4.5,
                                                    9,
                                                    9)
                                            }

                                            if (node.hasComment) {
                                                ctx.fillStyle =
                                                    Kirigami.Theme
                                                        .highlightColor
                                                ctx.beginPath()
                                                ctx.arc(
                                                    x + 7,
                                                    y - 7,
                                                    2.5,
                                                    0,
                                                    Math.PI * 2)
                                                ctx.fill()
                                            }

                                            if (node.branchPoint
                                                    || current) {
                                                ctx.fillStyle =
                                                    Kirigami.Theme
                                                        .textColor
                                                ctx.font =
                                                    "11px sans-serif"
                                                ctx.textAlign =
                                                    "left"
                                                ctx.textBaseline =
                                                    "middle"
                                                ctx.fillText(
                                                    node.isMove
                                                        || node.root
                                                        ? Number(
                                                            node
                                                                .moveNumber)
                                                            .toString()
                                                        : "node",
                                                    x + 11,
                                                    y)
                                            }
                                        }
                                    }

                                    MouseArea {
                                        anchors.fill: parent
                                        preventStealing: false

                                        onClicked:
                                            function(mouse) {
                                                const node =
                                                    sgfTreeCanvas
                                                        .nodeAt(
                                                            mouse.x,
                                                            mouse.y)

                                                if (node === null)
                                                    return

                                                boardPane
                                                    .clearContinuationMap()

                                                if (gameController
                                                        .showStudyStructureNode(
                                                            Number(
                                                                node.id))) {
                                                    boardPane
                                                        .applyLoadedPosition()
                                                } else {
                                                    console.warn(
                                                        gameController
                                                            .error_message)
                                                }
                                            }
                                    }
                                }
                            }
                        }
                    }

                    Frame {
                        id: josekiTreeFrame

                        anchors.left: parent.left
                        anchors.top: parent.top
                        anchors.bottom: parent.bottom

                        width:
                            Math.max(
                                0,
                                goBoard.x
                                - Kirigami.Units.smallSpacing)

                        visible:
                            root.studyPaneMode === "joseki"
                            && josekiModel.tree_json !== "[]"
                            && width >= Kirigami.Units.gridUnit * 8

                        padding: 4
                        clip: true

                        contentItem: ColumnLayout {
                            spacing: 2

                            Label {
                                Layout.fillWidth: true
                                text: qsTr("Joseki tree")
                                font.bold: true
                            }

                            Flickable {
                                id: josekiTreeFlick

                                Layout.fillWidth: true
                                Layout.fillHeight: true

                                clip: true
                                boundsBehavior: Flickable.StopAtBounds
                                flickableDirection:
                                    Flickable.HorizontalAndVerticalFlick

                                contentWidth:
                                    Math.max(
                                        width,
                                        josekiTreeCanvas.treeWidth)

                                contentHeight:
                                    Math.max(
                                        height,
                                        josekiTreeCanvas.treeHeight)

                                ScrollBar.vertical: ScrollBar {
                                    policy:
                                        josekiTreeFlick.contentHeight
                                            > josekiTreeFlick.height
                                        ? ScrollBar.AlwaysOn
                                        : ScrollBar.AsNeeded
                                }

                                ScrollBar.horizontal: ScrollBar {
                                    policy:
                                        josekiTreeFlick.contentWidth
                                            > josekiTreeFlick.width
                                        ? ScrollBar.AlwaysOn
                                        : ScrollBar.AsNeeded
                                }

                                Canvas {
                                    id: josekiTreeCanvas

                                    property var nodes: {
                                        try {
                                            return JSON.parse(
                                                josekiModel.tree_json)
                                        } catch (error) {
                                            return []
                                        }
                                    }

                                    property string currentNode:
                                        josekiModel.node_id

                                    /*
                                     * Move depth runs left-to-right.
                                     * Alternative joseki branches stack
                                     * vertically in the centre pane.
                                     */
                                    property real rowSpacing: 30
                                    property real laneSpacing: 24

                                    property int maxRow: {
                                        let result = 0

                                        for (const node of nodes) {
                                            result = Math.max(
                                                result,
                                                Number(node.row))
                                        }

                                        return result
                                    }

                                    property int maxLane: {
                                        let result = 0

                                        for (const node of nodes) {
                                            result = Math.max(
                                                result,
                                                Number(node.lane))
                                        }

                                        return result
                                    }

                                    property real treeWidth:
                                        44
                                        + (maxRow + 1)
                                          * rowSpacing

                                    property real treeHeight:
                                        44
                                        + (maxLane + 1)
                                          * laneSpacing

                                    width:
                                        Math.max(
                                            josekiTreeFlick.width,
                                            treeWidth)

                                    height:
                                        Math.max(
                                            josekiTreeFlick.height,
                                            treeHeight)

                                    function nodeX(node) {
                                        return 22
                                               + Number(node.row)
                                                 * rowSpacing
                                    }

                                    function nodeY(node) {
                                        return 22
                                               + Number(node.lane)
                                                 * laneSpacing
                                    }

                                    function nodeById(id) {
                                        for (const node of nodes) {
                                            if (String(node.id)
                                                    === String(id)) {
                                                return node
                                            }
                                        }

                                        return null
                                    }

                                    function nodeAt(x, y) {
                                        let nearest = null
                                        let nearestDistance = 11

                                        for (const node of nodes) {
                                            const dx =
                                                x - nodeX(node)
                                            const dy =
                                                y - nodeY(node)
                                            const distance =
                                                Math.sqrt(
                                                    dx * dx
                                                    + dy * dy)

                                            if (distance
                                                    < nearestDistance) {
                                                nearest = node
                                                nearestDistance =
                                                    distance
                                            }
                                        }

                                        return nearest
                                    }

                                    function categoryColour(category) {
                                        const name =
                                            String(category).toUpperCase()

                                        if (name === "IDEAL")
                                            return "#008300"

                                        if (name === "GOOD")
                                            return "#436600"

                                        if (name === "TRICK")
                                            return "#e0c900"

                                        if (name === "MISTAKE")
                                            return "#b3001e"

                                        if (name === "QUESTION")
                                            return "#00a7c4"

                                        return Kirigami.Theme.textColor
                                    }

                                    function ensureCurrentVisible() {
                                        const node =
                                            nodeById(currentNode)

                                        if (node === null)
                                            return

                                        const x = nodeX(node)
                                        const y = nodeY(node)
                                        const margin = 36

                                        if (y - margin
                                                < josekiTreeFlick
                                                    .contentY) {
                                            josekiTreeFlick.contentY =
                                                Math.max(
                                                    0,
                                                    y - margin)
                                        } else if (
                                            y + margin
                                            > josekiTreeFlick.contentY
                                              + josekiTreeFlick.height) {
                                            josekiTreeFlick.contentY =
                                                Math.min(
                                                    Math.max(
                                                        0,
                                                        josekiTreeFlick
                                                            .contentHeight
                                                        - josekiTreeFlick
                                                            .height),
                                                    y + margin
                                                    - josekiTreeFlick
                                                        .height)
                                        }

                                        if (x - margin
                                                < josekiTreeFlick
                                                    .contentX) {
                                            josekiTreeFlick.contentX =
                                                Math.max(
                                                    0,
                                                    x - margin)
                                        } else if (
                                            x + margin
                                            > josekiTreeFlick.contentX
                                              + josekiTreeFlick.width) {
                                            josekiTreeFlick.contentX =
                                                Math.min(
                                                    Math.max(
                                                        0,
                                                        josekiTreeFlick
                                                            .contentWidth
                                                        - josekiTreeFlick
                                                            .width),
                                                    x + margin
                                                    - josekiTreeFlick
                                                        .width)
                                        }
                                    }

                                    onNodesChanged: {
                                        requestPaint()
                                        Qt.callLater(
                                            ensureCurrentVisible)
                                    }

                                    onCurrentNodeChanged: {
                                        requestPaint()
                                        Qt.callLater(
                                            ensureCurrentVisible)
                                    }

                                    onWidthChanged:
                                        requestPaint()

                                    onHeightChanged:
                                        requestPaint()

                                    onPaint: {
                                        const ctx =
                                            getContext("2d")

                                        ctx.reset()

                                        ctx.globalAlpha = 0.35
                                        ctx.strokeStyle =
                                            Kirigami.Theme.textColor
                                        ctx.lineWidth = 1.5

                                        for (const node of nodes) {
                                            if (node.parent === null
                                                    || node.parent
                                                       === undefined) {
                                                continue
                                            }

                                            const parent =
                                                nodeById(node.parent)

                                            if (parent === null)
                                                continue

                                            const parentX =
                                                nodeX(parent)
                                            const parentY =
                                                nodeY(parent)
                                            const x = nodeX(node)
                                            const y = nodeY(node)

                                            ctx.beginPath()
                                            ctx.moveTo(
                                                parentX,
                                                parentY)
                                            ctx.lineTo(
                                                x,
                                                parentY)
                                            ctx.lineTo(
                                                x,
                                                y)
                                            ctx.stroke()
                                        }

                                        ctx.globalAlpha = 1.0

                                        for (const node of nodes) {
                                            const x = nodeX(node)
                                            const y = nodeY(node)
                                            const current =
                                                String(node.id)
                                                === currentNode

                                            if (current) {
                                                ctx.strokeStyle =
                                                    Kirigami.Theme
                                                        .highlightColor
                                                ctx.lineWidth = 3
                                                ctx.beginPath()
                                                ctx.arc(
                                                    x,
                                                    y,
                                                    8,
                                                    0,
                                                    Math.PI * 2)
                                                ctx.stroke()
                                            }

                                            if (node.colour === "root") {
                                                ctx.fillStyle =
                                                    Kirigami.Theme
                                                        .backgroundColor
                                                ctx.strokeStyle =
                                                    Kirigami.Theme
                                                        .textColor
                                                ctx.lineWidth = 1.5
                                            } else {
                                                ctx.fillStyle =
                                                    node.colour
                                                        === "black"
                                                    ? "#202020"
                                                    : "#f5f5f5"
                                                ctx.strokeStyle =
                                                    categoryColour(
                                                        node.category)
                                                ctx.lineWidth =
                                                    String(
                                                        node.category)
                                                        .toUpperCase()
                                                        === "IDEAL"
                                                    ? 3
                                                    : 2
                                            }

                                            ctx.beginPath()
                                            ctx.arc(
                                                x,
                                                y,
                                                5.5,
                                                0,
                                                Math.PI * 2)
                                            ctx.fill()
                                            ctx.stroke()

                                            let label =
                                                node.label
                                                === undefined
                                                ? ""
                                                : String(node.label)

                                            if (label === "_")
                                                label = ""

                                            if (label.length > 0) {
                                                ctx.fillStyle =
                                                    node.colour
                                                        === "black"
                                                    ? "#ffffff"
                                                    : "#111111"
                                                ctx.font =
                                                    "bold 9px sans-serif"
                                                ctx.textAlign =
                                                    "center"
                                                ctx.textBaseline =
                                                    "middle"
                                                ctx.fillText(
                                                    label,
                                                    x,
                                                    y)
                                            }
                                        }
                                    }

                                    MouseArea {
                                        anchors.fill: parent
                                        preventStealing: false

                                        onClicked:
                                            function(mouse) {
                                                const node =
                                                    josekiTreeCanvas
                                                        .nodeAt(
                                                            mouse.x,
                                                            mouse.y)

                                                if (node === null)
                                                    return

                                                if (String(node.id)
                                                        === josekiModel
                                                            .node_id) {
                                                    return
                                                }

                                                josekiModel
                                                    .loadPosition(
                                                        String(
                                                            node.id))
                                            }
                                    }
                                }
                            }
                        }
                    }

                    GoBoard {
                        id: goBoard


                        anchors.right: parent.right
                        anchors.rightMargin: boardFrame.rightPadding
                        anchors.verticalCenter: parent.verticalCenter

                        width: Math.min(
                            boardFrame.availableWidth,
                            boardFrame.availableHeight)
                        height: width

                        /*
                         * KataGo candidates deliberately use the same visual
                         * language as professional continuations, while
                         * remaining a separate non-clickable data source.
                         */
                        katagoCandidatePoints: {
                            if (root.studyPaneMode === "joseki")
                                return []

                            if (boardPane.investigationMode !== "katago")
                                return []

                            if (katagoPanel.analysisMoveNumber
                                    !== gameController.move_number
                                    || katagoPanel.analysisVisitBudget
                                       !== uiSettings.katagoVisitBudget
                                    || katagoPanel.analysisKomi
                                       !== katagoKomiField.text) {
                                return []
                            }

                            const points = JSON.parse(
                                gameController.katago_candidate_points_json)

                            return points.map(function(point) {
                                return {
                                    "x": Number(point.x),
                                    "y": goBoard.boardSize
                                         - 1
                                         - Number(point.y),
                                    "visits": Number(point.visits)
                                }
                            })
                        }

                          patternSelectionEnabled: boardPane.selectingPattern
                          patternSelectionAdjustable:
                              !gameList.searchHasRun
                              && !gameList.searchInProgress

                          onContinuationPointClicked: function(x, y, count) {
                              if (root.studyPaneMode === "joseki") {
                                  josekiModel.followPoint(x, y)
                                  return
                              }

                              boardPane.filterContinuationPoint(
                                          x,
                                          goBoard.boardSize - 1 - y,
                                          count)
                          }


                          onPointClicked: function(x, y) {
                              if (root.studyPaneMode === "joseki") {
                                  josekiModel.followPoint(x, y)
                                  return
                              }

                              if (root.playingGame) {
                                  if (root.localGameFinished)
                                      return

                                  if (gameController.playGamePoint(x, y)) {
                                      gameList.clearSearchResults()
                                      boardPane.applyLoadedPosition()
                                  } else {
                                      console.warn(
                                                  gameController.error_message)
                                  }

                                  return
                              }

                                                            if (boardPane.sgfEditTool === "move"
                                      || boardPane.sgfEditTool === "branch") {
                                  const branching =
                                      boardPane.sgfEditTool === "branch"

                                  if (branching) {
                                      if (studyActionTabs.branchSourceNode < 0
                                              || !gameController.showStudyStructureNode(
                                                  studyActionTabs.branchSourceNode)) {
                                          console.warn(gameController.error_message)
                                          return
                                      }
                                  }

                                  const ok = branching
                                      ? gameController.addStudyBranch(x, y)
                                      : gameController.addStudyMove(x, y)

                                  if (ok) {
                                      if (branching
                                              && !gameController.showStudyStructureNode(
                                                  studyActionTabs.branchSourceNode)) {
                                          console.warn(gameController.error_message)
                                      }

                                      boardPane.applyLoadedPosition()
                                  } else {
                                      console.warn(gameController.error_message)
                                  }

                                  return
                              }

                              if (boardPane.annotationTool.length > 0) {
                                  boardPane.annotatePoint(x, y)
                                  return
                              }

                              if (!boardPane.editingPosition)
                                  return

                              let tool = boardPane.editTool
                              let alternatePlacement = tool === "alternate"
                              let pointWasEmpty = true

                              if (alternatePlacement) {
                                  tool = boardPane.alternateEditColour

                                  for (const stone of goBoard.stones) {
                                      if (Number(stone.x) === x
                                              && Number(stone.y) === y) {
                                          pointWasEmpty = false
                                          break
                                      }
                                  }
                              }

                              if (gameController.editPositionPoint(
                                          x,
                                          y,
                                          tool)) {
                                  if (alternatePlacement && pointWasEmpty) {
                                      boardPane.alternateEditColour =
                                          tool === "black"
                                          ? "white"
                                          : "black"
                                  }

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

                ColumnLayout {
                    id: moveNavigationStrip

                    visible:
                        root.studyPaneMode !== "joseki"
                        && !root.playingGame
                        && boardPane.selectedGame !== null
                        && gameController.move_count > 0

                    /*
                     * Keep game navigation physically attached to the
                     * goban rather than stretching across the whole
                     * board pane when the goban is height-limited.
                     */
                    Layout.fillWidth: false
                    Layout.preferredWidth: goBoard.width
                    Layout.maximumWidth: goBoard.width
                    Layout.leftMargin: 0
                    Layout.rightMargin: 0
                    Layout.bottomMargin: 4
                    Layout.alignment: Qt.AlignRight
                    spacing: 0

                    Label {
                        Layout.fillWidth: true

                        text: qsTr("Move %1 of %2")
                            .arg(gameController.move_number)
                            .arg(gameController.move_count)

                        horizontalAlignment: Text.AlignHCenter
                        opacity: 0.75
                    }

                    Item {
                        id: moveNavigationRow

                        Layout.fillWidth: true
                        Layout.preferredHeight:
                            Math.max(
                                leftMoveButtons.implicitHeight,
                                rightMoveButtons.implicitHeight,
                                moveSlider.implicitHeight)
                        Layout.minimumHeight: Layout.preferredHeight

                        Row {
                            id: leftMoveButtons

                            anchors.left: parent.left
                            anchors.verticalCenter: parent.verticalCenter
                            spacing: 2

                            ToolButton {
                                text: "|<"
                                enabled: gameController.move_number > 0
                                onClicked: boardPane.showMove(0)

                                ToolTip.visible: hovered
                                ToolTip.text: qsTr("First position")
                            }

                            ToolButton {
                                text: "<<"
                                enabled: gameController.move_number > 0

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
                                enabled: gameController.move_number > 0

                                onClicked: boardPane.showMove(
                                               gameController.move_number - 1)

                                ToolTip.visible: hovered
                                ToolTip.text: qsTr("Previous move")
                            }
                        }

                        Row {
                            id: rightMoveButtons

                            anchors.right: parent.right
                            anchors.verticalCenter: parent.verticalCenter
                            spacing: 2

                            ToolButton {
                                text: ">"
                                enabled:
                                    gameController.move_number
                                    < gameController.move_count

                                onClicked: boardPane.showMove(
                                               gameController.move_number + 1)

                                ToolTip.visible: hovered
                                ToolTip.text: qsTr("Next move")
                            }

                            ToolButton {
                                text: ">>"
                                enabled:
                                    gameController.move_number
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
                                enabled:
                                    gameController.move_number
                                    < gameController.move_count

                                onClicked: boardPane.showMove(
                                               gameController.move_count)

                                ToolTip.visible: hovered
                                ToolTip.text: qsTr("Final position")
                            }
                        }

                        Slider {
                            id: moveSlider

                            anchors.left: leftMoveButtons.right
                            anchors.right: rightMoveButtons.left
                            anchors.verticalCenter: parent.verticalCenter
                            anchors.leftMargin: 4
                            anchors.rightMargin: 4

                            from: 0
                            to: Math.max(
                                    0,
                                    gameController.move_count)

                            value: gameController.move_number

                            stepSize: 1
                            snapMode: Slider.SnapAlways

                            enabled:
                                boardPane.selectedGame
                                && gameController.move_count > 0

                            onMoved: {
                                const requestedMove =
                                    Math.round(value)

                                if (requestedMove
                                        !== gameController.move_number) {
                                    boardPane.showMove(
                                        requestedMove)
                                }
                            }

                            ToolTip.visible:
                                hovered || pressed

                            ToolTip.text:
                                qsTr("Move %1")
                                    .arg(Math.round(value))
                        }
                    }
                }




            } // surrounding ColumnLayout
        } // boardPane

    } // mainSplitView

} // ApplicationWindow
