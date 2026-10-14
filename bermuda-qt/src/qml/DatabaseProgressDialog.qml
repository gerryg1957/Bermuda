import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Dialog {
    id: root

    property var operationModel

    modal: true
    width: Math.min(760, parent ? parent.width - 48 : 760)

    title: {
        switch (operationModel.operation_name) {
        case "create-database":
            return qsTr("Create Database")

        case "add-games":
            return qsTr("Add Games")

        case "update-index":
            return qsTr("Update Position Index")

        default:
            return qsTr("Database Operation")
        }
    }

    closePolicy: operationModel.in_progress
        ? Popup.NoAutoClose
        : Popup.CloseOnEscape

    readonly property bool importing:
        operationModel.stage === "importing"

    readonly property bool indexing:
        operationModel.stage === "indexing"

    readonly property bool hasIndexSummary:
        operationModel.operation_name === "update-index"
        || operationModel.total_index_games > 0
        || operationModel.processed_index_games > 0
        || operationModel.indexed_games > 0
        || operationModel.indexed_positions > 0
        || operationModel.index_errors > 0

    readonly property bool rateIsIndexRate:
        root.indexing
        || operationModel.operation_name === "update-index"
        || operationModel.total_index_games > 0
        || operationModel.processed_index_games > 0
        || operationModel.indexed_games > 0
        || operationModel.indexed_positions > 0

    readonly property bool determinate:
        (importing
         && operationModel.total_sgf_files > 0)
        || (indexing
            && operationModel.total_index_games > 0)

    readonly property real progressMaximum:
        importing
        ? Math.max(1, operationModel.total_sgf_files)
        : Math.max(1, operationModel.total_index_games)

    readonly property real progressValue:
        importing
        ? operationModel.processed_sgf_files
        : operationModel.processed_index_games

    readonly property string phaseText: {
        const stage = operationModel.stage
        if (stage === "discovering" || stage === "importing")
            return operationModel.includes_index
                ? qsTr("Step 1 of 2 · Import games") : qsTr("Import games")
        if (stage === "indexing")
            return operationModel.operation_name === "update-index"
                ? qsTr("Prepare games for searching")
                : qsTr("Step 2 of 2 · Prepare games for searching")
        if (stage === "finalising-index")
            return qsTr("Finishing · Prepare the search index")
        return ""
    }

    function countText(current, total) {
        return qsTr("%1 of %2").arg(current).arg(total)
    }

    contentItem: ColumnLayout {
        spacing: 10

        Label {
            Layout.fillWidth: true
            text: qsTr("Database: %1").arg(operationModel.database_name)
            font.bold: true
            wrapMode: Text.Wrap
        }

        Label {
            Layout.fillWidth: true
            text: operationModel.result_project_path
            wrapMode: Text.WrapAnywhere
            opacity: 0.7
        }

        Label {
            Layout.fillWidth: true
            visible: root.phaseText.length > 0 && operationModel.in_progress
            text: root.phaseText
            font.bold: true
        }

        Label {
            Layout.fillWidth: true

            text: operationModel.status_message
            font.pixelSize: 18
            font.bold: true
            wrapMode: Text.Wrap
        }

        ProgressBar {
            Layout.fillWidth: true
            visible: operationModel.in_progress

            from: 0
            to: root.progressMaximum
            value: root.progressValue
            indeterminate: operationModel.in_progress
                           && !root.determinate
        }

        Label {
            Layout.fillWidth: true
            visible: operationModel.stage === "discovering"

            text: qsTr("%1 SGF files discovered")
                .arg(operationModel.discovered_sgf_files)
        }

        GridLayout {
            Layout.fillWidth: true
            visible: operationModel.total_sgf_files > 0
                     || operationModel.processed_sgf_files > 0

            columns: 2
            columnSpacing: 20
            rowSpacing: 4

            Label {
                text: qsTr("SGF files processed")
                font.bold: true
            }

            Label {
                text: root.countText(
                          operationModel.processed_sgf_files,
                          operationModel.total_sgf_files)
            }

            Label {
                text: qsTr("Games imported")
                font.bold: true
            }

            Label {
                text: operationModel.imported_games
            }

            Label {
                text: qsTr("Sources added")
                font.bold: true
            }

            Label {
                text: operationModel.added_sources
            }

            Label {
                text: qsTr("Duplicates")
                font.bold: true
            }

            Label {
                text: operationModel.duplicates
            }

            Label {
                text: qsTr("Skipped")
                font.bold: true
            }

            Label {
                text: operationModel.skipped
            }

            Label {
                text: qsTr("Import errors")
                font.bold: true
            }

            Label {
                text: operationModel.import_errors
            }
        }

        GridLayout {
            Layout.fillWidth: true
            visible: root.hasIndexSummary

            columns: 2
            columnSpacing: 20
            rowSpacing: 4

            Label {
                text: qsTr("Games processed for indexing")
                font.bold: true
            }

            Label {
                text: root.countText(
                          operationModel.processed_index_games,
                          operationModel.total_index_games)
            }

            Label {
                text: qsTr("Games indexed")
                font.bold: true
            }

            Label {
                text: operationModel.indexed_games
            }

            Label {
                text: qsTr("Positions indexed")
                font.bold: true
            }

            Label {
                text: operationModel.indexed_positions
            }

            Label {
                text: qsTr("Index errors")
                font.bold: true
            }

            Label {
                text: operationModel.index_errors
            }
        }

        Label {
            Layout.fillWidth: true
            visible: operationModel.current_item.length > 0

            text: operationModel.current_item
            wrapMode: Text.WrapAnywhere
            maximumLineCount: 3
            elide: Text.ElideMiddle
            opacity: 0.75
        }

        Label {
            Layout.fillWidth: true
            visible: operationModel.elapsed_seconds > 0

            text: {
                let result = qsTr("%1 seconds")
                    .arg(operationModel.elapsed_seconds.toFixed(1))

                if (operationModel.rate > 0) {
                    const unit = root.rateIsIndexRate
                        ? qsTr("games/second")
                        : qsTr("SGF files/second")

                    result += qsTr(" · %1 %2")
                        .arg(operationModel.rate.toFixed(1))
                        .arg(unit)
                }

                return result
            }

            opacity: 0.75
        }

        Label {
            Layout.fillWidth: true
            visible: operationModel.in_progress
                     && (root.importing || root.indexing
                         || operationModel.stage === "finalising-index")
            wrapMode: Text.Wrap
            text: {
                if (operationModel.cancel_requested)
                    return qsTr("Stopping safely…")
                if (operationModel.stage === "finalising-index")
                    return qsTr("All games have been processed. Preparing the search index; remaining time is not yet known.")
                if (root.progressValue >= root.progressMaximum)
                    return qsTr("Finishing this phase…")
                const seconds = operationModel.remaining_seconds
                if (seconds < 0)
                    return qsTr("Estimating time remaining for this phase…")
                if (seconds < 60)
                    return qsTr("Less than a minute remaining for this phase")
                return qsTr("About %1 minutes remaining for this phase")
                    .arg(Math.ceil(seconds / 60))
            }
        }

        Label {
            Layout.fillWidth: true
            visible: operationModel.cancelled

            text: qsTr("Completed work has been kept.") + " "
                  + (operationModel.can_continue
                     ? operationModel.continuation_message
                     : qsTr("Close this window to return to the database."))

            wrapMode: Text.Wrap
            font.bold: true
        }

        Label {
            Layout.fillWidth: true
            visible: operationModel.error_message.length > 0

            text: operationModel.error_message
            wrapMode: Text.Wrap
            font.bold: true
        }

        Label {
            Layout.fillWidth: true
            visible: operationModel.import_error_log.length > 0

            text: qsTr("Import error log: %1")
                .arg(operationModel.import_error_log)

            wrapMode: Text.WrapAnywhere
        }

        Label {
            Layout.fillWidth: true
            visible: operationModel.index_error_log.length > 0

            text: qsTr("Index error log: %1")
                .arg(operationModel.index_error_log)

            wrapMode: Text.WrapAnywhere
        }

        RowLayout {
            Layout.fillWidth: true

            Item {
                Layout.fillWidth: true
            }

            Button {
                visible: operationModel.in_progress

                text: operationModel.cancel_requested
                    ? qsTr("Cancelling…")
                    : qsTr("Cancel")

                enabled: !operationModel.cancel_requested

                onClicked:
                    operationModel.cancelOperation()
            }

            Button {
                visible: !operationModel.in_progress
                         && operationModel.can_continue
                         && operationModel.pending_project_path === operationModel.result_project_path
                text: qsTr("Continue setup")
                onClicked: operationModel.continueSetup()
            }

            Button {
                visible: !operationModel.in_progress

                text: qsTr("Close")
                highlighted: true
                onClicked: root.close()
            }
        }
    }
}
