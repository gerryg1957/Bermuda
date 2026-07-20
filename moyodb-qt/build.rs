use cxx_qt_build::{CxxQtBuilder, QmlModule};

fn main() {
    CxxQtBuilder::new_qml_module(
        QmlModule::new("org.moyodb.app")
            .qml_file("src/qml/Main.qml")
            .qml_file("src/qml/GameList.qml")
            .qml_file("src/qml/GoBoard.qml"),
    )
    .file("src/app.rs")
    .file("src/game_list_model.rs")
    .build();
}
