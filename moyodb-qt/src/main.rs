mod app;
use cxx_qt_lib::{QGuiApplication, QQmlApplicationEngine, QQuickStyle, QString, QUrl};
use cxx_qt_lib_extras::QApplication;
use moyodb_core::Board;
use std::env;

fn main() {
    // Confirm that the GUI crate is connected to moyodb-core.
    Board::new(19).expect("create 19x19 board");

    let mut app = QApplication::new();

    QGuiApplication::set_desktop_file_name(&QString::from("org.moyodb.app"));

    if env::var("QT_QUICK_CONTROLS_STYLE").is_err() {
        QQuickStyle::set_style(&QString::from("org.kde.desktop"));
    }

    let mut engine = QQmlApplicationEngine::new();

    if let Some(engine) = engine.as_mut() {
        let qml_url = format!("file://{}/src/qml/Main.qml", env!("CARGO_MANIFEST_DIR"));

        engine.load(&QUrl::from(qml_url.as_str()));
    }

    if let Some(app) = app.as_mut() {
        app.exec();
    }
}
