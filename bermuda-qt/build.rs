use cxx_qt_build::{CxxQtBuilder, QmlModule};

fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        assert_eq!(std::env::var("CARGO_CFG_TARGET_ENV").as_deref(), Ok("msvc"),
            "The Windows package uses the MSVC toolchain");
        let icon = std::path::Path::new("../packaging/windows/bermuda.ico")
            .canonicalize().expect("Windows icon");
        let out = std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
        let resource = out.join("bermuda.rc");
        let compiled = out.join("bermuda.res");
        std::fs::write(&resource, format!("1 ICON \"{}\"\n", icon.display().to_string().replace('\\', "/")))
            .expect("write icon resource");
        let status = std::process::Command::new("rc.exe")
            .arg("/nologo").arg("/fo").arg(&compiled).arg(&resource)
            .status().expect("run Windows resource compiler");
        assert!(status.success(), "Windows resource compiler failed");
        println!("cargo:rustc-link-arg-bin=bermuda-qt={}", compiled.display());
        println!("cargo:rerun-if-changed=../packaging/windows/bermuda.ico");
    }

    CxxQtBuilder::new_qml_module(
        QmlModule::new("org.bermuda.app")
            .qml_file("src/qml/Main.qml")
            .qml_file("src/qml/AboutDialog.qml")
            .qml_file("src/qml/DatabaseImportDialog.qml")
            .qml_file("src/qml/DatabaseProgressDialog.qml")
            .qml_file("src/qml/PlayerIdentityDialog.qml")
            .qml_file("src/qml/GameList.qml")
            .qml_file("src/qml/GoBoard.qml")
            .qml_file("src/qml/ReplyInfluenceAnalysis.qml"),
    )
    .qrc_resources(["src/qml/assets/lgc-logo.png"])
    .file("src/app.rs")
    .file("src/database_operation_model.rs")
    .file("src/joseki_model.rs")
    .file("src/game_list_model.rs")
    .file("src/player_identity_model.rs")
    .build();
}
