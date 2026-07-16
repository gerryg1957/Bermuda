import QtQuick
import QtQuick.Layouts
import QtQuick.Controls as Controls
import org.kde.kirigami as Kirigami

Kirigami.ApplicationWindow {
    id: root
    
    
    title: "MoyoDB"
   
    Shortcut {
        sequence: StandardKey.Quit
        onActivated: Qt.quit()
    }

    Shortcut {
        sequence: "Escape"
        onActivated: Qt.quit()
    }

    minimumWidth: Kirigami.Units.gridUnit * 36
    minimumHeight: Kirigami.Units.gridUnit * 28

    width: Kirigami.Units.gridUnit * 48
    height: Kirigami.Units.gridUnit * 36

    pageStack.initialPage: Kirigami.Page {
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

            GoBoard {
                Layout.preferredWidth: Kirigami.Units.gridUnit * 28
                Layout.preferredHeight: Layout.preferredWidth
                Layout.alignment: Qt.AlignHCenter

                boardSize: 19

                stones: [
                    { "x": 3, "y": 3, "color": "black" },
                    { "x": 15, "y": 15, "color": "white" },
                    { "x": 15, "y": 3, "color": "black" },
                    { "x": 3, "y": 15, "color": "white" }
                ]
                onPointClicked: (x, y) => {
                    statusLabel.text = "Clicked intersection: " + x + ", " + y
                }
            }
Controls.Label {
    id: statusLabel
    text: "Move the pointer over the board"
    Layout.alignment: Qt.AlignHCenter
}
        }
    }
}
