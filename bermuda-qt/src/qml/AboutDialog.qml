import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Dialog {
    id: root

    parent: Overlay.overlay
    anchors.centerIn: parent

    modal: true
    closePolicy: Popup.CloseOnEscape
    standardButtons: Dialog.Close

    title: qsTr("About Bermuda")

    width: Math.min(parent.width - 80, 680)

    contentItem: ColumnLayout {
        spacing: 12

        Label {
            text: qsTr("Bermuda")
            font.pixelSize: 28
            font.bold: true
        }

        Label {
            text: qsTr("Version %1").arg(Qt.application.version)
            opacity: 0.75
        }

        Label {
            Layout.fillWidth: true

            text: qsTr(
                "Bermuda is an open-source desktop application for studying "
                + "Go games. It combines an SGF game database with an "
                + "interactive Go board and pattern search, helping you "
                + "explore candidate moves based on professional play.")

            wrapMode: Text.WordWrap
        }

        Label {
            text: qsTr("Licence")
            font.bold: true
        }

        Label {
            Layout.fillWidth: true

            text: qsTr(
                "Bermuda is free software licensed under the GNU General "
                + "Public License, version 3 or later.")

            wrapMode: Text.WordWrap
        }

        Label {
            text: qsTr("Player-name data")
            font.bold: true
        }

        Label {
            Layout.fillWidth: true

            text: qsTr(
                "Bermuda's supplied player-name catalogue includes data "
                + "from the u-go.net Go Player List, maintained by Ulrich "
                + "Görtz. Bermuda uses the 10 August 2026 snapshot, made "
                + "available under CC0 1.0.")

            wrapMode: Text.WordWrap
        }

        Label {
            text: qsTr("Project")
            font.bold: true
        }

        RowLayout {
            Layout.fillWidth: true

            Button {
                text: qsTr("Bermuda on GitHub")

                onClicked:
                    Qt.openUrlExternally(
                        "https://github.com/gerryg1957/Bermuda")
            }

            Button {
                text: qsTr("u-go.net Go Player List")

                onClicked:
                    Qt.openUrlExternally("https://db.u-go.net/")
            }

            Item {
                Layout.fillWidth: true
            }
        }
    }
}
