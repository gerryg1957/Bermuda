use cxx_qt::{CxxQtType, Threading};
use cxx_qt_lib::{QString, QUrl};
use std::{path::Path, pin::Pin};

#[cxx_qt::bridge]
mod ffi {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
        include!("cxx-qt-lib/qurl.h");
        type QUrl = cxx_qt_lib::QUrl;
    }
    unsafe extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qproperty(bool, busy)]
        #[qproperty(QString, games_json)]
        #[qproperty(QString, archive_json)]
        #[qproperty(QString, error_message)]
        #[qproperty(QString, notice)]
        #[qproperty(QUrl, report_url)]
        type PersonalSnapshotModel = super::PersonalSnapshotModelRust;
        #[qinvokable]
        fn refresh(self: Pin<&mut PersonalSnapshotModel>, project_path: &QString);
        #[qinvokable]
        #[cxx_name = "createGroup"]
        fn create_group(
            self: Pin<&mut PersonalSnapshotModel>,
            project_path: &QString,
            name: &QString,
            ids: &QString,
        );
        #[qinvokable]
        #[cxx_name = "saveSnapshot"]
        fn save_snapshot(
            self: Pin<&mut PersonalSnapshotModel>,
            project_path: &QString,
            group_id: i32,
            label: &QString,
        );
        #[qinvokable]
        #[cxx_name = "openReport"]
        fn open_report(
            self: Pin<&mut PersonalSnapshotModel>,
            project_path: &QString,
            snapshot_id: i32,
        );
    }
    impl cxx_qt::Threading for PersonalSnapshotModel {}
}
#[derive(Default)]
pub struct PersonalSnapshotModelRust {
    busy: bool,
    games_json: QString,
    archive_json: QString,
    error_message: QString,
    notice: QString,
    report_url: QUrl,
}
enum Action {
    Refresh,
    Create(String, String),
    Capture(i64, String),
    Export(i64),
}
impl ffi::PersonalSnapshotModel {
    fn refresh(self: Pin<&mut Self>, path: &QString) {
        self.perform(path.to_string(), Action::Refresh);
    }
    fn create_group(self: Pin<&mut Self>, path: &QString, name: &QString, ids: &QString) {
        self.perform(
            path.to_string(),
            Action::Create(name.to_string(), ids.to_string()),
        );
    }
    fn save_snapshot(self: Pin<&mut Self>, path: &QString, group_id: i32, label: &QString) {
        self.perform(
            path.to_string(),
            Action::Capture(i64::from(group_id), label.to_string()),
        );
    }
    fn open_report(self: Pin<&mut Self>, path: &QString, snapshot_id: i32) {
        self.perform(path.to_string(), Action::Export(i64::from(snapshot_id)));
    }
    fn perform(mut self: Pin<&mut Self>, path: String, action: Action) {
        if self.rust().busy {
            return;
        }
        self.as_mut().set_busy(true);
        self.as_mut().set_error_message(QString::default());
        self.as_mut().set_notice(QString::default());
        // Reset first so opening the same saved report again emits a change.
        self.as_mut().set_report_url(QUrl::default());
        let thread = self.qt_thread();
        std::thread::spawn(move || {
            let result = (|| -> anyhow::Result<_> {
                let project =
                    bermuda::project_manager::ProjectManager::new().open(Path::new(&path))?;
                let store = bermuda::personal_snapshots::SnapshotStore::open(&project)?;
                let mut exported = None;
                let notice = match action {
                    Action::Refresh => String::new(),
                    Action::Create(name, ids) => {
                        let ids: Vec<i64> = serde_json::from_str(&ids)?;
                        store.create_group(&project, &name, &ids)?;
                        "Group saved. Next, name and preserve its current analysis in step 2."
                            .into()
                    }
                    Action::Capture(id, label) => {
                        let analysis = crate::app::katago_working_directory()
                            .map_err(anyhow::Error::msg)?
                            .join("game-analysis.sqlite3");
                        store.capture(id, &label, &analysis)?;
                        "Snapshot saved. Step 3: open it with View report / Print. No new analysis was run.".into()
                    }
                    Action::Export(id) => {
                        let dir = project.cache_path().join("snapshot-reports");
                        std::fs::create_dir_all(&dir)?;
                        let file = dir.join(format!("snapshot-{id}.html"));
                        store.export_html(id, &file)?;
                        exported = Some(file);
                        "Opening the preserved report in your browser. Use Print / Save as PDF there.".into()
                    }
                };
                Ok((
                    serde_json::to_string(&bermuda::personal_snapshots::available_games(
                        &project,
                    )?)?,
                    store.listing()?.to_string(),
                    notice,
                    exported,
                ))
            })();
            let _ =
                thread.queue(move |mut model| {
                    match result {
                        Ok((games, archive, notice, exported)) => {
                            model.as_mut().set_games_json(QString::from(games));
                            model.as_mut().set_archive_json(QString::from(archive));
                            model.as_mut().set_notice(QString::from(notice));
                            if let Some(path) = exported {
                                model.as_mut().set_report_url(QUrl::from_local_file(
                                    &QString::from(path.to_string_lossy().as_ref()),
                                ));
                            }
                        }
                        Err(error) => model
                            .as_mut()
                            .set_error_message(QString::from(format!("{error:#}"))),
                    }
                    model.as_mut().set_busy(false);
                });
        });
    }
}
