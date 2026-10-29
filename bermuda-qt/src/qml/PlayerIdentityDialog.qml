import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import org.bermuda.app

Dialog {
    id: root

    parent: Overlay.overlay
    anchors.centerIn: parent

    modal: true
    closePolicy: Popup.CloseOnEscape
    standardButtons: Dialog.Close

    title: personalMode ? qsTr("My games — player names") : qsTr("Player Names")
    property bool personalMode: false
    property bool advancedMode: false
    property var checkedNames: []
    property bool reviewingNames: false
    readonly property var myNameChoices: parseArray(identityModel.my_names_json)
    function nameChecked(item) {
        return checkedNames.some(function(n) { return n.sourceId === item.sourceId && n.name === item.name })
    }
    function toggleName(item, checked) {
        checkedNames = checkedNames.filter(function(n) { return n.sourceId !== item.sourceId || n.name !== item.name })
        if (checked) checkedNames = checkedNames.concat([item])
        reviewingNames = false
    }
    function resetSimpleForm() {
        checkedNames = []
        reviewingNames = false
        myDisplayName.text = personalPlayerName() || qsTr("Me")
    }
    property bool projectLoaded: false
    property string projectPath: ""

    width: Math.min(parent.width - 80, personalMode && !advancedMode ? 800 : 1180)
    height: Math.min(parent.height - 80, personalMode && !advancedMode ? 650 : 840)

    signal identitiesChanged()

    property int selectedSourceId: -1
    property string selectedSourceName: ""
    property string selectedSourceVersion: ""
    property string selectedRawName: ""

    property string sourceNameFilter: ""
    property string identityNameFilter: ""

    /*
     * Selecting a new source spelling deliberately invalidates any identity
     * that happened to be selected previously. The user must choose the
     * intended identity after choosing the source name before linking.
     */
    property bool identityChosenForSource: false

    property int selectedAliasId: -1
    property string selectedAliasName: ""

    function parseArray(text) {
        if (text.length === 0)
            return []

        try {
            const value = JSON.parse(text)
            return Array.isArray(value) ? value : []
        } catch (error) {
            console.warn("Invalid player identity JSON:", error)
            return []
        }
    }

    readonly property var players:
        parseArray(identityModel.players_json)

    readonly property var unresolvedNames:
        parseArray(identityModel.unresolved_json)

    readonly property var knownNames:
        parseArray(identityModel.known_names_json)

    function filterItems(items, filterText, fieldName) {
        const needle = filterText.trim().toLowerCase()

        if (needle.length === 0)
            return items

        const filtered = []

        for (const item of items) {
            const value = item[fieldName]

            if (value !== undefined
                    && value !== null
                    && String(value).toLowerCase().indexOf(needle) >= 0) {
                filtered.push(item)
            }
        }

        return filtered
    }

    readonly property var filteredUnresolvedNames:
        filterItems(unresolvedNames, sourceNameFilter, "name")

    readonly property var filteredPlayers:
        filterItems(players, identityNameFilter, "preferredName")

    readonly property int selectedPlayerId:
        Number(identityModel.selected_player_id)

    function personalPlayerName() {
        for (const player of players)
            if (Number(player.id) === Number(identityModel.personal_player_id)) return player.preferredName
        return ""
    }

    function selectedPlayerName() {
        for (const player of players) {
            if (Number(player.id) === selectedPlayerId)
                return player.preferredName
        }

        return ""
    }

    function clearUnresolvedSelection() {
        selectedSourceId = -1
        selectedSourceName = ""
        selectedSourceVersion = ""
        selectedRawName = ""
        identityChosenForSource = false
        newPlayerNameField.text = ""
    }

    function clearAliasSelection() {
        selectedAliasId = -1
        selectedAliasName = ""
    }

    function restoreSourceListPosition(contentY) {
        /*
         * Updating unresolved_json replaces the ListView model. Preserve the
         * user's approximate position in a long list rather than returning
         * them to its beginning after every successful link.
         */
        Qt.callLater(function() {
            unresolvedList.contentY = contentY
            unresolvedList.returnToBounds()
        })
    }

    function openForProject(path, personal) {
        personalMode = personal === true
        projectPath = path
        sourceNameFilter = ""
        identityNameFilter = ""
        clearUnresolvedSelection()
        clearAliasSelection()
        renamePlayerField.text = ""

        advancedMode = false
        projectLoaded = identityModel.loadProject(path)
        resetSimpleForm()
        open()
    }

    PlayerIdentityModel {
        id: identityModel
    }

    Dialog {
        id: unlinkAliasConfirmation

        parent: Overlay.overlay
        anchors.centerIn: parent
        modal: true

        title: qsTr("Remove this name link?")
        standardButtons: Dialog.Ok | Dialog.Cancel

        contentItem: Label {
            width: 460

            text: qsTr(
                "Remove your link for “%1”? Matching source-name records "
                + "will return to the unrecognised list. No games or "
                + "original source names will be deleted.")
                .arg(root.selectedAliasName)

            wrapMode: Text.WordWrap
        }

        onAccepted: {
            if (identityModel.removeAlias(root.selectedAliasId)) {
                root.clearAliasSelection()
                root.identityChosenForSource = false
                root.identitiesChanged()
            }
        }
    }

    Dialog {
        id: removeIdentityConfirmation

        parent: Overlay.overlay
        anchors.centerIn: parent
        modal: true

        title: qsTr("Remove player?")
        standardButtons: Dialog.Ok | Dialog.Cancel

        contentItem: Label {
            width: 500

            text: qsTr(
                "Remove player “%1”? Names you linked to this player will "
                + "return to the source-name list. No games or original "
                + "source names will be deleted.")
                .arg(root.selectedPlayerName())

            wrapMode: Text.WordWrap
        }

        onAccepted: {
            if (identityModel.deletePlayer(root.selectedPlayerId)) {
                root.clearAliasSelection()
                root.identityChosenForSource = false
                renamePlayerField.text = ""
                root.identitiesChanged()
            }
        }
    }

    contentItem: ColumnLayout {
        enabled: root.projectLoaded
        spacing: 10

        ColumnLayout {
            visible: root.personalMode && !root.advancedMode
            Layout.fillWidth: true
            Layout.fillHeight: true
            spacing: 12
            Label { text: qsTr("Which player is you?"); font.bold: true; font.pixelSize: 20 }
            RowLayout {
                Layout.fillWidth: true
                Label { text: qsTr("Show my names as:") }
                TextField {
                    id: myDisplayName
                    objectName: "myDisplayName"
                    Layout.fillWidth: true
                    placeholderText: qsTr("Me")
                    enabled: !root.reviewingNames
                    onTextEdited: root.reviewingNames = false
                }
            }
            Label {
                Layout.fillWidth: true; wrapMode: Text.Wrap
                text: qsTr("Tick the usernames you use. Names already linked to you are marked below. Other players can be managed under Advanced name management.")
            }
            Frame {
                Layout.fillWidth: true; Layout.fillHeight: true
                padding: 0
                ListView {
                    anchors.fill: parent; clip: true
                    model: root.myNameChoices
                    delegate: CheckDelegate {
                        required property var modelData
                        width: ListView.view.width
                        checked: modelData.alreadyMine || root.nameChecked(modelData)
                        enabled: !modelData.alreadyMine && !root.reviewingNames
                        text: modelData.name + " — " + qsTr("%1 games").arg(modelData.gameCount)
                            + " · " + modelData.sourceName + " " + modelData.sourceVersion
                            + (modelData.alreadyMine ? qsTr(" · Already yours") : "")
                        onToggled: root.toggleName(modelData, checked)
                    }
                    ScrollBar.vertical: ScrollBar {}
                    Label { anchors.centerIn: parent; visible: root.myNameChoices.length === 0; text: qsTr("No unlinked names. Use Advanced name management to review existing links."); width: parent.width - 24; wrapMode: Text.Wrap }
                }
            }
            Label {
                visible: root.reviewingNames
                Layout.fillWidth: true; wrapMode: Text.Wrap
                text: qsTr("%1 will be shown as “%2”. Original SGF names will be kept.")
                    .arg(root.checkedNames.length ? root.checkedNames.map(function(n) { return n.name }).join(", ") : qsTr("Your linked names"))
                    .arg(myDisplayName.text.trim())
                    + qsTr(" This also sets your identity for My games. Existing links are kept.")
            }
            Label { Layout.fillWidth: true; wrapMode: Text.Wrap; text: identityModel.status_message; visible: text.length > 0 }
            Label { Layout.fillWidth: true; wrapMode: Text.Wrap; text: identityModel.error_message; visible: text.length > 0; color: Kirigami.Theme.negativeTextColor }
            RowLayout {
                Layout.fillWidth: true
                Button {
                    text: qsTr("Advanced name management…")
                    onClicked: { root.checkedNames = []; root.reviewingNames = false; root.advancedMode = true }
                }
                Item { Layout.fillWidth: true }
                Button { visible: root.reviewingNames; text: qsTr("Back"); onClicked: root.reviewingNames = false }
                Button {
                    text: root.reviewingNames ? qsTr("Save my names") : qsTr("Review names")
                    enabled: myDisplayName.text.trim().length > 0 && (root.checkedNames.length > 0 || identityModel.personal_player_id >= 0)
                    onClicked: {
                        if (!root.reviewingNames) { root.reviewingNames = true; return }
                        const names = root.checkedNames.map(function(n) { return [n.sourceId, n.name] })
                        if (identityModel.saveMyNames(myDisplayName.text, JSON.stringify(names))) {
                            root.resetSimpleForm()
                            root.identitiesChanged()
                        }
                    }
                }
            }
        }

        ColumnLayout {
            visible: !root.personalMode || root.advancedMode
            Layout.fillWidth: true
            Layout.fillHeight: true
            spacing: 10
            Button {
                visible: root.personalMode
                text: qsTr("Back to my names")
                onClicked: { root.resetSimpleForm(); root.advancedMode = false }
            }

        Label {
            Layout.fillWidth: true

            text: root.personalMode
                ? qsTr("Link your usernames to one preferred name, then select that player and click Mark as me. Choose a source name on the left, then its player on the right. Your original SGF names stay unchanged; links also apply to future imports with that exact name and source.")
                : qsTr(
                "Bermuda recognises different spellings of the same "
                + "player's name and groups their games together. When "
                + "Bermuda knows that two names belong to the same player, "
                + "searching either name finds all of that player's games. "
                + "You only need to make changes here if you spot a source "
                + "name that should be linked to a player.")
            wrapMode: Text.WordWrap
        }

        RowLayout {
            visible: root.personalMode
            Layout.fillWidth: true
            Label {
                Layout.fillWidth: true
                wrapMode: Text.Wrap
                text: identityModel.personal_player_id < 0 ? qsTr("Me: not selected")
                    : qsTr("Me: %1").arg(root.personalPlayerName())
            }
            Button {
                text: qsTr("Mark as me")
                enabled: root.selectedPlayerId >= 0 && root.selectedPlayerId !== Number(identityModel.personal_player_id)
                onClicked: if (identityModel.setPersonalPlayer(root.selectedPlayerId)) root.identitiesChanged()
            }
            Button {
                text: qsTr("Clear Me")
                enabled: identityModel.personal_player_id >= 0
                onClicked: if (identityModel.setPersonalPlayer(-1)) root.identitiesChanged()
            }
        }

        RowLayout {
            Layout.fillWidth: true
            Layout.fillHeight: true
            spacing: 12

            ColumnLayout {
                Layout.fillWidth: true
                Layout.fillHeight: true
                Layout.preferredWidth: 1
                spacing: 6

                Label {
                    text: qsTr("Source names")
                    font.bold: true
                }

                Label {
                    Layout.fillWidth: true

                    text: root.sourceNameFilter.trim().length === 0
                          ? root.personalMode
                            ? qsTr("Usernames and names not yet linked to a player. Select one of yours to begin.")
                            : qsTr(
                                "Names as they appear in game sources that "
                                + "Bermuda has not grouped with a player. "
                                + "Most can simply be left alone.")
                          : qsTr("%1 matching source-name entries")
                                .arg(root.filteredUnresolvedNames.length)

                    wrapMode: Text.WordWrap
                    opacity: 0.7
                }

                TextField {
                    Layout.fillWidth: true

                    text: root.sourceNameFilter
                    placeholderText: qsTr("Search source names")

                    onTextEdited: {
                        root.sourceNameFilter = text

                        /*
                         * A filtered-out selection must not remain as a
                         * hidden recognition candidate.
                         */
                        root.clearUnresolvedSelection()
                    }
                }

                Frame {
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    Layout.minimumHeight: 180
                    padding: 0

                    ListView {
                        id: unresolvedList

                        anchors.fill: parent
                        clip: true
                        model: root.filteredUnresolvedNames

                        delegate: ItemDelegate {
                            width: ListView.view.width

                            highlighted:
                                root.selectedSourceId
                                    === Number(modelData.sourceId)
                                && root.selectedRawName
                                    === modelData.name

                            contentItem: Column {
                                spacing: 2

                                Label {
                                    width: parent.width
                                    text: modelData.name
                                    font.bold: true
                                    elide: Text.ElideRight
                                }

                                Label {
                                    width: parent.width

                                    text: qsTr("%1 %2 · %3 occurrence(s)")
                                          .arg(modelData.sourceName)
                                          .arg(modelData.sourceVersion)
                                          .arg(modelData.occurrenceCount)

                                    opacity: 0.7
                                    elide: Text.ElideRight
                                }
                            }

                            onClicked: {
                                root.identityChosenForSource = false
                                root.selectedSourceId =
                                    Number(modelData.sourceId)
                                root.selectedSourceName =
                                    modelData.sourceName
                                root.selectedSourceVersion =
                                    modelData.sourceVersion
                                root.selectedRawName =
                                    modelData.name

                                newPlayerNameField.text =
                                    modelData.name
                            }
                        }

                        ScrollBar.vertical: ScrollBar {}
                    }
                }
            }

            ColumnLayout {
                Layout.fillWidth: true
                Layout.fillHeight: true
                Layout.preferredWidth: 1
                spacing: 6

                Label {
                    text: qsTr("Players with grouped names")
                    font.bold: true
                }

                Label {
                    Layout.fillWidth: true

                    text: root.identityNameFilter.trim().length === 0
                          ? qsTr(
                                "Each entry represents one player. Different "
                                + "source names can be grouped with that "
                                + "player.")
                          : qsTr("%1 matching players")
                                .arg(root.filteredPlayers.length)

                    wrapMode: Text.WordWrap
                    opacity: 0.7
                }

                TextField {
                    Layout.fillWidth: true

                    text: root.identityNameFilter
                    placeholderText: qsTr("Search players")

                    onTextEdited: {
                        root.identityNameFilter = text

                        /*
                         * Searching may hide the player previously selected.
                         * Require an explicit click before using a player as
                         * the target for an unrecognised name.
                         */
                        root.identityChosenForSource = false
                    }
                }

                Frame {
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    Layout.minimumHeight: 180
                    padding: 0

                    ListView {
                        anchors.fill: parent
                        clip: true
                        model: root.filteredPlayers

                        delegate: ItemDelegate {
                            width: ListView.view.width
                            text: modelData.preferredName + (root.personalMode && Number(modelData.id) === Number(identityModel.personal_player_id) ? qsTr(" · Me") : "")

                            highlighted:
                                root.selectedPlayerId
                                    === Number(modelData.id)

                            onClicked: {
                                if (identityModel.loadAliases(
                                            Number(modelData.id))) {
                                    root.clearAliasSelection()

                                    renamePlayerField.text =
                                        modelData.preferredName

                                    root.identityChosenForSource =
                                        root.selectedSourceId >= 0
                                }
                            }
                        }

                        ScrollBar.vertical: ScrollBar {}
                    }
                }
            }
        }

        Frame {
            Layout.fillWidth: true
            Layout.minimumHeight: 330
            Layout.preferredHeight: 330

            contentItem: RowLayout {
                spacing: 16

                ColumnLayout {
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    Layout.preferredWidth: 1
                    spacing: 7

                    Label {
                        text: qsTr("Selected source name")
                        font.bold: true
                    }

                    Label {
                        Layout.fillWidth: true

                        text: root.selectedSourceId < 0
                              ? qsTr(
                                    "Select a source name above to see "
                                    + "what you can do with it.")
                              : qsTr(
                                    "“%1” — %2 %3")
                                    .arg(root.selectedRawName)
                                    .arg(root.selectedSourceName)
                                    .arg(root.selectedSourceVersion)

                        wrapMode: Text.WordWrap
                    }

                    Label {
                        Layout.fillWidth: true
                        visible: root.selectedSourceId >= 0

                        text: qsTr(
                            "Bermuda has not grouped this source name with "
                            + "a player. If you know who it refers to, "
                            + "select that player above. If the player is "
                            + "not listed, you can create one. Otherwise, "
                            + "leave the name alone.")

                        wrapMode: Text.WordWrap
                        opacity: 0.75
                    }

                    Label {
                        text: qsTr("Create a new player:")
                        opacity: root.selectedSourceId >= 0 ? 1.0 : 0.55
                    }

                    RowLayout {
                        Layout.fillWidth: true

                        TextField {
                            id: newPlayerNameField
                            Layout.fillWidth: true

                            enabled: root.selectedSourceId >= 0
                            placeholderText: qsTr(
                                "Preferred player name")
                        }

                        Button {
                            text: qsTr("Create player and link")

                            enabled: root.selectedSourceId >= 0
                                     && newPlayerNameField.text
                                            .trim().length > 0

                            onClicked: {
                                const previousContentY =
                                    unresolvedList.contentY

                                if (identityModel.createPlayerAndAssign(
                                            newPlayerNameField.text,
                                            root.selectedSourceId,
                                            root.selectedRawName)) {
                                    root.clearUnresolvedSelection()
                                    root.identitiesChanged()
                                    root.restoreSourceListPosition(
                                                previousContentY)
                                }
                            }
                        }
                    }

                    Label {
                        text: qsTr("Link to an existing player:")
                        opacity: root.selectedSourceId >= 0 ? 1.0 : 0.55
                    }

                    RowLayout {
                        Layout.fillWidth: true

                        Label {
                            Layout.fillWidth: true

                            text: root.selectedPlayerId < 0
                                  ? qsTr(
                                        "Select a player above")
                                  : !root.identityChosenForSource
                                    ? qsTr(
                                        "Select the intended player above "
                                        + "for this name")
                                    : root.selectedPlayerName()

                            font.bold: root.identityChosenForSource
                            elide: Text.ElideRight
                        }

                        Button {
                            text: !root.identityChosenForSource
                                  ? qsTr("Select player above")
                                  : qsTr("Link to “%1”")
                                        .arg(root.selectedPlayerName())

                            enabled: root.selectedSourceId >= 0
                                     && root.selectedPlayerId >= 0
                                     && root.identityChosenForSource

                            onClicked: {
                                const previousContentY =
                                    unresolvedList.contentY

                                if (identityModel.assignSourceName(
                                            root.selectedPlayerId,
                                            root.selectedSourceId,
                                            root.selectedRawName)) {
                                    root.clearUnresolvedSelection()
                                    root.identitiesChanged()
                                    root.restoreSourceListPosition(
                                                previousContentY)
                                }
                            }
                        }
                    }
                }

                ColumnLayout {
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    Layout.preferredWidth: 1
                    spacing: 7

                    Label {
                        Layout.fillWidth: true

                        text: root.selectedPlayerId < 0
                              ? qsTr("Names for this player")
                              : qsTr("Names for %1")
                                    .arg(root.selectedPlayerName())

                        font.bold: true
                        elide: Text.ElideRight
                    }

                    Label {
                        Layout.fillWidth: true

                        text: root.selectedPlayerId < 0
                              ? qsTr(
                                    "Select a player above to see the "
                                    + "names Bermuda knows for them.")
                              : qsTr(
                                    "These names are treated as the same "
                                    + "player when searching games.")

                        wrapMode: Text.WordWrap
                        opacity: 0.75
                    }

                    Frame {
                        Layout.fillWidth: true
                        Layout.preferredHeight: 105
                        padding: 0

                        ListView {
                            anchors.fill: parent
                            clip: true
                            model: root.knownNames

                            delegate: ItemDelegate {
                                width: ListView.view.width

                                highlighted:
                                    modelData.kind === "local"
                                    && modelData.localAliasId !== null
                                    && root.selectedAliasId
                                        === Number(modelData.localAliasId)

                                contentItem: Column {
                                    spacing: 1

                                    Label {
                                        width: parent.width
                                        text: modelData.name
                                        elide: Text.ElideRight
                                    }

                                    Label {
                                        width: parent.width

                                        text:
                                            modelData.kind === "preferred"
                                            ? qsTr("Preferred name")
                                            : modelData.kind === "supplied"
                                              ? qsTr("Bermuda catalogue")
                                              : modelData.sourceId === null
                                                ? qsTr("Your name")
                                                : qsTr(
                                                    "Your link · %1 %2")
                                                    .arg(
                                                        modelData.sourceName)
                                                    .arg(
                                                        modelData
                                                            .sourceVersion)

                                        opacity: 0.7
                                        elide: Text.ElideRight
                                    }
                                }

                                onClicked: {
                                    if (modelData.kind === "local"
                                            && modelData.localAliasId
                                                !== null
                                            && modelData.localAliasId
                                                !== undefined) {
                                        root.selectedAliasId =
                                            Number(modelData.localAliasId)
                                        root.selectedAliasName =
                                            modelData.name
                                    } else {
                                        root.clearAliasSelection()
                                    }
                                }
                            }

                            ScrollBar.vertical: ScrollBar {}
                        }
                    }

                    RowLayout {
                        Layout.fillWidth: true

                        Button {
                            text: root.selectedAliasId < 0
                                  ? qsTr(
                                        "Select one of your links to remove")
                                  : qsTr("Remove link “%1”")
                                        .arg(root.selectedAliasName)

                            enabled: root.selectedAliasId >= 0
                            onClicked: unlinkAliasConfirmation.open()
                        }

                        Item {
                            Layout.fillWidth: true
                        }

                        Button {
                            text: qsTr("Remove player")
                            enabled: root.selectedPlayerId >= 0
                            onClicked: removeIdentityConfirmation.open()
                        }
                    }

                    RowLayout {
                        Layout.fillWidth: true

                        TextField {
                            id: renamePlayerField

                            Layout.fillWidth: true
                            Layout.minimumWidth: 240

                            enabled: root.selectedPlayerId >= 0
                            placeholderText: qsTr(
                                "Preferred player name")
                        }

                        Button {
                            text: qsTr("Rename")

                            enabled: root.selectedPlayerId >= 0
                                     && renamePlayerField.text
                                            .trim().length > 0
                                     && renamePlayerField.text
                                        !== root.selectedPlayerName()

                            onClicked: {
                                if (identityModel.renamePlayer(
                                            root.selectedPlayerId,
                                            renamePlayerField.text)) {
                                    root.identitiesChanged()
                                }
                            }
                        }
                    }

                }
            }
        }

        Label {
            Layout.fillWidth: true

            visible: identityModel.status_message.length > 0
            text: identityModel.status_message
            wrapMode: Text.WordWrap
        }

        Label {
            Layout.fillWidth: true

            visible: identityModel.error_message.length > 0
            text: identityModel.error_message
            color: Kirigami.Theme.negativeTextColor
            wrapMode: Text.WordWrap
        }
        }
    }
}
