import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import QtQuick.Dialogs

Dialog {
    id: root

    property var operationModel
    property string currentProjectPath: ""
    property string mode: "create"
    property string formError: ""

    signal operationStarted()

    modal: true
    width: Math.min(720, parent ? parent.width - 48 : 720)

    title: mode === "managed-create"
        ? qsTr("Create Games Database")
        : mode === "create"
          ? qsTr("Create Database")
          : qsTr("Add Games")

    closePolicy: Popup.CloseOnEscape

    function pathFromUrl(url) {
        let path = decodeURIComponent(
                    new URL(url.toString()).pathname)

        if (path.length > 1)
            path = path.replace(/\/+$/, "")

        return path
    }

    function joinedPath(parentPath, name) {
        if (parentPath === "/")
            return "/" + name

        return parentPath.replace(/\/+$/, "")
                + "/" + name
    }

    function destinationPath() {
        return joinedPath(
                    parentFolderField.text.trim(),
                    databaseNameField.text.trim())
    }

    function resetCommonFields() {
        sourceFolderField.text = ""
        sourceNameField.text = ""
        sourceVersionField.text = ""
        buildIndexCheckBox.checked = true
        formError = ""
    }

    function openCreate() {
        mode = "create"
        currentProjectPath = ""

        databaseNameField.text = ""
        parentFolderField.text = ""

        resetCommonFields()
        open()
    }

    function openManagedCreate(projectPath) {
        mode = "managed-create"
        currentProjectPath = projectPath

        databaseNameField.text = ""
        parentFolderField.text = ""

        resetCommonFields()
        open()
    }

    function openManagedAdd(projectPath) {
        mode = "managed-add"
        currentProjectPath = projectPath

        resetCommonFields()
        open()
    }

    function openAdd(projectPath) {
        mode = "add"
        currentProjectPath = projectPath

        resetCommonFields()
        open()
    }

    function validateForm() {
        formError = ""

        if (mode === "create") {
            const databaseName =
                databaseNameField.text.trim()

            if (databaseName.length === 0) {
                formError = qsTr(
                            "Enter a database name.")
                return false
            }

            if (databaseName.indexOf("/") >= 0) {
                formError = qsTr(
                            "The database name must not contain '/'.")
                return false
            }

            if (parentFolderField.text.trim().length
                    === 0) {
                formError = qsTr(
                            "Choose a parent folder.")
                return false
            }
        } else if (mode === "managed-create") {
            if (currentProjectPath.length === 0) {
                formError = qsTr(
                            "The managed database location is unavailable.")
                return false
            }
        } else if (currentProjectPath.length === 0) {
            formError = qsTr(
                        "No database is currently open.")
            return false
        }

        if (sourceFolderField.text.trim().length
                === 0) {
            formError = qsTr(
                        "Choose a folder containing SGF files.")
            return false
        }

        if (sourceNameField.text.trim().length === 0) {
            formError = qsTr(
                        "Enter a source name.")
            return false
        }

        if (sourceVersionField.text.trim().length
                === 0) {
            formError = qsTr(
                        "Enter a source version.")
            return false
        }

        return true
    }

    function startOperation() {
        if (!validateForm())
            return

        operationModel.clearStatus()

        let started = false

        if (mode === "create") {
            started = operationModel.createDatabase(
                        databaseNameField.text.trim(),
                        destinationPath(),
                        sourceFolderField.text.trim(),
                        sourceNameField.text.trim(),
                        sourceVersionField.text.trim(),
                        buildIndexCheckBox.checked)
        } else if (mode === "managed-create") {
            started = operationModel.createDatabase(
                        "Games Database",
                        currentProjectPath,
                        sourceFolderField.text.trim(),
                        sourceNameField.text.trim(),
                        sourceVersionField.text.trim(),
                        true)
        } else if (mode === "managed-add") {
            started = operationModel.addGames(
                        currentProjectPath,
                        sourceFolderField.text.trim(),
                        sourceNameField.text.trim(),
                        sourceVersionField.text.trim(),
                        true)
        } else {
            started = operationModel.addGames(
                        currentProjectPath,
                        sourceFolderField.text.trim(),
                        sourceNameField.text.trim(),
                        sourceVersionField.text.trim(),
                        buildIndexCheckBox.checked)
        }

        if (!started) {
            formError = operationModel.error_message
            return
        }

        close()
        operationStarted()
    }

    FolderDialog {
        id: parentFolderDialog
        title: qsTr("Choose Parent Folder")

        onAccepted: {
            parentFolderField.text =
                root.pathFromUrl(selectedFolder)
        }
    }

    FolderDialog {
        id: sourceFolderDialog
        title: qsTr("Choose SGF Folder")

        onAccepted: {
            sourceFolderField.text =
                root.pathFromUrl(selectedFolder)
        }
    }

    contentItem: ColumnLayout {
        spacing: 10

        Label {
            Layout.fillWidth: true
            visible: root.mode === "add"

            text: qsTr("Database")
            font.bold: true
        }

        TextField {
            Layout.fillWidth: true
            visible: root.mode === "add"

            text: root.currentProjectPath
            readOnly: true
            selectByMouse: true
        }

        Label {
            Layout.fillWidth: true
            visible: root.mode === "create"

            text: qsTr("Database name")
            font.bold: true
        }

        TextField {
            id: databaseNameField

            Layout.fillWidth: true
            visible: root.mode === "create"

            placeholderText: qsTr("For example: Professional Games")
            selectByMouse: true
        }

        Label {
            Layout.fillWidth: true
            visible: root.mode === "create"

            text: qsTr("Parent folder")
            font.bold: true
        }

        RowLayout {
            Layout.fillWidth: true
            visible: root.mode === "create"

            TextField {
                id: parentFolderField

                Layout.fillWidth: true
                placeholderText: qsTr(
                                     "Folder in which the database will be created")
                selectByMouse: true
            }

            Button {
                text: qsTr("Choose…")
                onClicked: parentFolderDialog.open()
            }
        }

        Label {
            Layout.fillWidth: true
            visible: root.mode === "create"
                     && databaseNameField.text.trim().length > 0
                     && parentFolderField.text.trim().length > 0

            text: qsTr("Database path: %1")
                .arg(root.destinationPath())

            wrapMode: Text.WrapAnywhere
            opacity: 0.75
        }

        Label {
            Layout.fillWidth: true
            text: qsTr("SGF source folder")
            font.bold: true
        }

        RowLayout {
            Layout.fillWidth: true

            TextField {
                id: sourceFolderField

                Layout.fillWidth: true
                placeholderText: qsTr(
                                     "Folder containing SGF files")
                selectByMouse: true
            }

            Button {
                text: qsTr("Choose…")
                onClicked: sourceFolderDialog.open()
            }
        }

        GridLayout {
            Layout.fillWidth: true

            columns: 2
            columnSpacing: 10
            rowSpacing: 8

            Label {
                text: qsTr("Source name")
            }

            TextField {
                id: sourceNameField

                Layout.fillWidth: true
                placeholderText: qsTr(
                                     "For example: GoGoD or go4go")
                selectByMouse: true
            }

            Label {
                text: qsTr("Source version")
            }

            TextField {
                id: sourceVersionField

                Layout.fillWidth: true
                placeholderText: qsTr(
                                     "For example: 2026")
                selectByMouse: true
            }
        }

        CheckBox {
            id: buildIndexCheckBox

            Layout.fillWidth: true
            visible: root.mode === "create"
                     || root.mode === "add"

            text: root.mode === "create"
                ? qsTr("Build the position index after importing")
                : qsTr("Update the position index after importing")
        }

        Label {
            Layout.fillWidth: true
            visible: root.formError.length > 0

            text: root.formError
            wrapMode: Text.Wrap
            font.bold: true
        }

        Label {
            Layout.fillWidth: true
            visible: root.mode === "create"

            text: qsTr(
                      "The destination path must not already exist. "
                      + "MoyoDB will not overwrite an existing folder.")

            wrapMode: Text.Wrap
            opacity: 0.75
        }

        RowLayout {
            Layout.fillWidth: true

            Item {
                Layout.fillWidth: true
            }

            Button {
                text: qsTr("Cancel")
                onClicked: root.close()
            }

            Button {
                text: root.mode === "create"
                      || root.mode === "managed-create"
                    ? qsTr("Create")
                    : qsTr("Add Games")

                highlighted: true
                onClicked: root.startOperation()
            }
        }
    }
}
