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
    height: Math.min(parent.height - 80, 720)

    signal identitiesChanged()

    property int selectedSourceId: -1
    property string selectedSourceName: ""
    property string selectedSourceVersion: ""
    property string selectedRawName: ""

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
    }

    function openForProject(projectPath) {
        clearUnresolvedSelection()
        preferredNameField.text = ""

        identityModel.loadProject(projectPath)
        open()
    }

    PlayerIdentityModel {
        id: identityModel
    }

    contentItem: ColumnLayout {
        spacing: 10

        Label {
            Layout.fillWidth: true

            text: qsTr(
                "Bermuda never merges player names automatically. "
                + "Select an unresolved source spelling and the player "
                + "identity it belongs to, then assign it explicitly.")
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
                    text: qsTr("Unresolved source names")
                    font.bold: true
                }

                Label {
                    Layout.fillWidth: true
                    text: qsTr("%1 unresolved spelling(s)")
                          .arg(root.unresolvedNames.length)
                    opacity: 0.7
                }

                Frame {
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    padding: 0

                    ListView {
                        id: unresolvedList

                        anchors.fill: parent
                        clip: true

                        model: root.unresolvedNames

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
                                root.selectedSourceId =
                                    Number(modelData.sourceId)

                                root.selectedSourceName =
                                    modelData.sourceName

                                root.selectedSourceVersion =
                                    modelData.sourceVersion

                                root.selectedRawName =
                                    modelData.name
                            }
                        }

                        ScrollBar.vertical: ScrollBar {}
                    }
                }

                Label {
                    Layout.fillWidth: true

                    text: root.selectedSourceId < 0
                          ? qsTr("No unresolved name selected")
                          : qsTr("Selected: %1 — %2 %3")
                                .arg(root.selectedRawName)
                                .arg(root.selectedSourceName)
                                .arg(root.selectedSourceVersion)

                    elide: Text.ElideRight
                }
            }

            ColumnLayout {
                Layout.fillWidth: true
                Layout.fillHeight: true
                Layout.preferredWidth: 1
                spacing: 8

                Label {
                    text: qsTr("Bermuda players")
                    font.bold: true
                }

                Frame {
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    Layout.preferredHeight: 280
                    padding: 0

                    ListView {
                        id: playerList

                        anchors.fill: parent
                        clip: true

                        model: root.players

                        delegate: ItemDelegate {
                            width: ListView.view.width
                            text: modelData.preferredName

                            highlighted:
                                root.selectedPlayerId
                                    === Number(modelData.id)

                            onClicked: {
                                if (identityModel.loadAliases(
                                            Number(modelData.id))) {
                                    preferredNameField.text =
                                        modelData.preferredName
                                }
                            }
                        }

                        ScrollBar.vertical: ScrollBar {}
                    }
                }

                RowLayout {
                    Layout.fillWidth: true

                    TextField {
                        id: preferredNameField

                        Layout.fillWidth: true
                        placeholderText: qsTr("Preferred player name")

                        onAccepted: {
                            if (root.selectedPlayerId >= 0) {
                                if (identityModel.renamePlayer(
                                            root.selectedPlayerId,
                                            text)) {
                                    root.identitiesChanged()
                                }
                            } else {
                                if (identityModel.createPlayer(text)) {
                                    root.identitiesChanged()
                                    text = ""
                                }
                            }
                        }
                    }

                    Button {
                        text: qsTr("Create")
                        enabled: preferredNameField.text.trim().length > 0

                        onClicked: {
                            if (identityModel.createPlayer(
                                        preferredNameField.text)) {
                                root.identitiesChanged()
                                preferredNameField.text = ""
                            }
                        }
                    }

                    Button {
                        text: qsTr("Rename")

                        enabled: root.selectedPlayerId >= 0
                                 && preferredNameField.text.trim().length > 0

                        onClicked: {
                            if (identityModel.renamePlayer(
                                        root.selectedPlayerId,
                                        preferredNameField.text)) {
                                root.identitiesChanged()
                            }
                        }
                    }
                }

                Label {
                    text: qsTr("Known aliases")
                    font.bold: true
                }

                Frame {
                    Layout.fillWidth: true
                    Layout.preferredHeight: 150
                    padding: 0

                    ListView {
                        anchors.fill: parent
                        clip: true

                        model: root.aliases

                        delegate: ItemDelegate {
                            width: ListView.view.width

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
                        }

                        ScrollBar.vertical: ScrollBar {}
                    }
                }

                Button {
                    Layout.fillWidth: true

                    text: root.selectedSourceId < 0
                          || root.selectedPlayerId < 0
                          ? qsTr("Assign to selected player")
                          : qsTr("Assign “%1” to %2")
                                .arg(root.selectedRawName)
                                .arg(root.selectedPlayerName())

                    enabled: root.selectedSourceId >= 0
                             && root.selectedPlayerId >= 0

                    onClicked: {
                        if (identityModel.assignSourceName(
                                    root.selectedPlayerId,
                                    root.selectedSourceId,
                                    root.selectedRawName)) {
                            root.clearUnresolvedSelection()
                            root.identitiesChanged()
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
