import QtQuick
import QtQuick.Layouts
import QtQuick.Controls as Controls
import org.kde.kirigami as Kirigami

Kirigami.ApplicationWindow {
    id: root

    title: "MoyoDB"

    minimumWidth: Kirigami.Units.gridUnit * 36
    minimumHeight: Kirigami.Units.gridUnit * 28

    width: Kirigami.Units.gridUnit * 48
    height: Kirigami.Units.gridUnit * 36

    pageStack.initialPage: mainPage

    Component {
        id: mainPage

        Kirigami.Page {
            title: "MoyoDB"

            ColumnLayout {
                anchors.centerIn: parent
                spacing: Kirigami.Units.largeSpacing

                Kirigami.Heading {
                    text: "MoyoDB"
                    level: 1
                    Layout.alignment: Qt.AlignHCenter
                }

                Controls.Label {
                    text: "Professional Go game database"
                    Layout.alignment: Qt.AlignHCenter
                }

                Rectangle {
                    Layout.preferredWidth: Kirigami.Units.gridUnit * 20
                    Layout.preferredHeight: Kirigami.Units.gridUnit * 20
                    Layout.alignment: Qt.AlignHCenter

                    color: "#d8a45b"
                    border.width: 1
                    border.color: Kirigami.Theme.textColor

                    Controls.Label {
                        anchors.centerIn: parent
                        text: "19 × 19 board"
                    }
                }

                Controls.Label {
                    text: "Rust core connected"
                    Layout.alignment: Qt.AlignHCenter
                }
            }
        }
    }
}
