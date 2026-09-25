mod app;
mod katago_install;
mod database_operation_model;
mod game_list_model;
mod joseki_model;
mod player_identity_model;
mod search_result_model;
use bermuda::Board;
use cxx_qt_lib::{QGuiApplication, QQmlApplicationEngine, QQuickStyle, QString, QUrl};
use cxx_qt_lib_extras::QApplication;
use std::env;

fn main() {
    // Confirm that the GUI crate is connected to bermuda.
    Board::new(19).expect("create 19x19 board");

    let mut app = QApplication::new();

    if let Some(app) = app.as_mut() {
        app.set_application_version(&QString::from(env!("CARGO_PKG_VERSION")));
    }

    QGuiApplication::set_desktop_file_name(&QString::from("org.bermuda.app"));

    if env::var("QT_QUICK_CONTROLS_STYLE").is_err() {
        QQuickStyle::set_style(&QString::from("org.kde.desktop"));
    }

    let mut engine = QQmlApplicationEngine::new();

    if let Some(engine) = engine.as_mut() {
        // Load the interface embedded by build.rs, independent of the source
        // checkout and the directory from which Bermuda was launched.
        engine.load(&QUrl::from(
            "qrc:/qt/qml/org/bermuda/app/src/qml/Main.qml",
        ));
    }

    if let Some(app) = app.as_mut() {
        app.exec();
    }
}
