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

    title: qsTr("Player Identities")

    width: Math.min(parent.width - 80, 1120)
    height: Math.min(parent.height - 80, 760)

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

    readonly property var aliases:
        parseArray(identityModel.aliases_json)

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

    function openForProject(projectPath) {
        sourceNameFilter = ""
        identityNameFilter = ""
        clearUnresolvedSelection()
        clearAliasSelection()
        renamePlayerField.text = ""

        identityModel.loadProject(projectPath)
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

        title: qsTr("Unlink alias?")
        standardButtons: Dialog.Ok | Dialog.Cancel

        contentItem: Label {
            width: 460

            text: qsTr(
                "Unlink alias “%1”? Matching source-name records will "
                + "return to the unlinked list. No games or original "
                + "source names will be deleted.")
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

        title: qsTr("Remove player identity?")
        standardButtons: Dialog.Ok | Dialog.Cancel

        contentItem: Label {
            width: 500

            text: qsTr(
                "Remove player identity “%1”? Its aliases will be removed "
                + "and linked games will return to their original unlinked "
                + "source names. No games or original source names will "
                + "be deleted.")
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
        spacing: 10

        Label {
            Layout.fillWidth: true

            text: qsTr(
                "Player identities are optional. Use them only when "
                + "different source names or spellings refer to the same "
                + "person. You do not need to create an identity for every "
                + "player; unlinked names continue to work normally.")
            wrapMode: Text.WordWrap
        }

        RowLayout {
            Layout.fillWidth: true
            Layout.fillHeight: true
            spacing: 12

            ColumnLayout {
                Layout.fillWidth: true
                Layout.fillHeight: true
                Layout.preferredWidth: 1

                Label {
                    text: qsTr("Unlinked source names (optional)")
                    font.bold: true
                }

                Label {
                    Layout.fillWidth: true

                    text: root.sourceNameFilter.trim().length === 0
                          ? qsTr("%1 unlinked source spelling(s)")
                                .arg(root.unresolvedNames.length)
                          : qsTr("%1 of %2 unlinked source spelling(s)")
                                .arg(root.filteredUnresolvedNames.length)
                                .arg(root.unresolvedNames.length)

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
                         * hidden link candidate.
                         */
                        root.clearUnresolvedSelection()
                    }
                }

                Frame {
                    Layout.fillWidth: true
                    Layout.fillHeight: true
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

                                /*
                                 * A new identity will usually use the source
                                 * spelling as its preferred name, so make that
                                 * the default rather than requiring retyping.
                                 */
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
                spacing: 8

                Label {
                    text: qsTr("Player identities")
                    font.bold: true
                }

                TextField {
                    Layout.fillWidth: true

                    text: root.identityNameFilter
                    placeholderText: qsTr("Search player identities")

                    onTextEdited: {
                        root.identityNameFilter = text

                        /*
                         * Searching may hide the identity that was previously
                         * selected. Require an explicit click on the intended
                         * identity before allowing a link.
                         */
                        root.identityChosenForSource = false
                    }
                }

                Frame {
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    Layout.preferredHeight: 250
                    padding: 0

                    ListView {
                        anchors.fill: parent
                        clip: true

                        model: root.filteredPlayers

                        delegate: ItemDelegate {
                            width: ListView.view.width
                            text: modelData.preferredName

                            highlighted:
                                root.selectedPlayerId
                                    === Number(modelData.id)

                            onClicked: {
                                if (identityModel.loadAliases(
                                            Number(modelData.id))) {
                                    root.clearAliasSelection()

                                    renamePlayerField.text =
                                        modelData.preferredName

                                    /*
                                     * This click, rather than a stale
                                     * selection from an earlier operation,
                                     * authorises this identity as the current
                                     * source name's proposed link target.
                                     */
                                    root.identityChosenForSource =
                                        root.selectedSourceId >= 0
                                }
                            }
                        }

                        ScrollBar.vertical: ScrollBar {}
                    }
                }

                Label {
                    Layout.fillWidth: true

                    text: root.selectedPlayerId < 0
                          ? qsTr("No player identity selected")
                          : qsTr("Selected identity: %1")
                                .arg(root.selectedPlayerName())

                    font.bold: root.selectedPlayerId >= 0
                    elide: Text.ElideRight
                }

                Label {
                    text: qsTr("Known aliases")
                    font.bold: true
                }

                Frame {
                    Layout.fillWidth: true
                    Layout.preferredHeight: 130
                    padding: 0

                    ListView {
                        anchors.fill: parent
                        clip: true

                        model: root.aliases

                        delegate: ItemDelegate {
                            width: ListView.view.width

                            highlighted:
                                root.selectedAliasId === Number(modelData.id)

                            contentItem: Column {
                                spacing: 2

                                Label {
                                    width: parent.width
                                    text: modelData.name
                                    elide: Text.ElideRight
                                }

                                Label {
                                    width: parent.width

                                    text: modelData.sourceId === null
                                          ? qsTr("Global alias")
                                          : qsTr("%1 %2")
                                                .arg(modelData.sourceName)
                                                .arg(modelData.sourceVersion)

                                    opacity: 0.7
                                    elide: Text.ElideRight
                                }
                            }

                            onClicked: {
                                root.selectedAliasId =
                                    Number(modelData.id)
                                root.selectedAliasName =
                                    modelData.name
                            }
                        }

                        ScrollBar.vertical: ScrollBar {}
                    }
                }

                RowLayout {
                    Layout.fillWidth: true

                    Item {
                        Layout.fillWidth: true
                    }

                    Button {
                        text: root.selectedAliasId < 0
                              ? qsTr("Select an alias to unlink")
                              : qsTr("Unlink alias “%1”")
                                    .arg(root.selectedAliasName)

                        enabled: root.selectedAliasId >= 0

                        onClicked: unlinkAliasConfirmation.open()
                    }
                }

                RowLayout {
                    Layout.fillWidth: true

                    Label {
                        text: qsTr("Rename selected player:")
                    }

                    TextField {
                        id: renamePlayerField

                        Layout.fillWidth: true

                        enabled: root.selectedPlayerId >= 0
                        placeholderText: qsTr("Preferred name")
                    }

                    Button {
                        text: qsTr("Rename")

                        enabled: root.selectedPlayerId >= 0
                                 && renamePlayerField.text.trim().length > 0
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

                    Button {
                        text: qsTr("Remove identity")
                        enabled: root.selectedPlayerId >= 0

                        onClicked: removeIdentityConfirmation.open()
                    }
                }
            }
        }

        Frame {
            Layout.fillWidth: true

            contentItem: ColumnLayout {
                spacing: 8

                Label {
                    text: qsTr("Link selected source name")
                    font.bold: true
                }

                Label {
                    Layout.fillWidth: true

                    text: root.selectedSourceId < 0
                          ? qsTr(
                                "Select an unlinked source name above.")
                          : qsTr(
                                "“%1” from %2 %3")
                                .arg(root.selectedRawName)
                                .arg(root.selectedSourceName)
                                .arg(root.selectedSourceVersion)

                    wrapMode: Text.WordWrap
                }

                RowLayout {
                    Layout.fillWidth: true

                    Label {
                        text: qsTr("Create new identity:")
                    }

                    TextField {
                        id: newPlayerNameField

                        Layout.fillWidth: true

                        enabled: root.selectedSourceId >= 0
                        placeholderText: qsTr("Preferred player name")
                    }

                    Button {
                        text: qsTr("Create and link")

                        enabled: root.selectedSourceId >= 0
                                 && newPlayerNameField.text.trim().length > 0

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

                RowLayout {
                    Layout.fillWidth: true

                    Label {
                        text: qsTr("Or use existing identity:")
                    }

                    Label {
                        Layout.fillWidth: true

                        text: root.selectedPlayerId < 0
                              ? qsTr("Select a player identity above")
                              : !root.identityChosenForSource
                                ? qsTr(
                                    "Choose the intended identity above "
                                    + "for this source name")
                                : root.selectedPlayerName()

                        font.bold: root.identityChosenForSource
                        elide: Text.ElideRight
                    }

                    Button {
                        text: !root.identityChosenForSource
                              ? qsTr("Choose identity above")
                              : qsTr("Link “%1” to “%2”")
                                    .arg(root.selectedRawName)
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
