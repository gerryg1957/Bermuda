#![allow(clippy::too_many_arguments)]
use cxx_qt::{CxxQtType, Threading};
use cxx_qt_lib::QString;

use std::{
    path::{Path, PathBuf},
    pin::Pin,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

use bermuda::{
    database_task::{self, DatabaseTask},
    import_directory::{self, ImportDirectoryOutcome, ImportProgress, ImportStage, ImportSummary},
    index_build::{
        self, IndexBuildOutcome, IndexBuildProgress, IndexBuildStage, IndexBuildSummary,
    },
    project::Project,
    project_manager::ProjectManager,
    task_progress::ProgressEstimate,
};

#[cxx_qt::bridge]
mod ffi {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");

        type QString = cxx_qt_lib::QString;
    }

    unsafe extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qproperty(bool, in_progress)]
        #[qproperty(bool, cancel_requested)]
        #[qproperty(bool, cancelled)]
        #[qproperty(bool, cancelled_during_import)]
        #[qproperty(QString, database_name)]
        #[qproperty(bool, includes_index)]
        #[qproperty(bool, can_continue)]
        #[qproperty(QString, pending_project_path)]
        #[qproperty(QString, pending_database_name)]
        #[qproperty(QString, continuation_message)]
        #[qproperty(QString, continuation_error)]
        #[qproperty(QString, operation_name)]
        #[qproperty(QString, stage)]
        #[qproperty(QString, status_message)]
        #[qproperty(QString, error_message)]
        #[qproperty(QString, result_project_path)]
        #[qproperty(QString, current_item)]
        #[qproperty(QString, import_error_log)]
        #[qproperty(QString, index_error_log)]
        #[qproperty(i64, discovered_sgf_files)]
        #[qproperty(i64, total_sgf_files)]
        #[qproperty(i64, processed_sgf_files)]
        #[qproperty(i64, imported_games)]
        #[qproperty(i64, added_sources)]
        #[qproperty(i64, duplicates)]
        #[qproperty(i64, skipped)]
        #[qproperty(i64, import_errors)]
        #[qproperty(i64, total_index_games)]
        #[qproperty(i64, processed_index_games)]
        #[qproperty(i64, indexed_games)]
        #[qproperty(i64, indexed_positions)]
        #[qproperty(i64, index_errors)]
        #[qproperty(f64, elapsed_seconds)]
        #[qproperty(f64, rate)]
        #[qproperty(f64, remaining_seconds)]
        type DatabaseOperationModel = super::DatabaseOperationModelRust;

        #[qinvokable]
        #[cxx_name = "createDatabase"]
        fn create_database(
            self: Pin<&mut DatabaseOperationModel>,
            project_name: &QString,
            project_path: &QString,
            sgf_directory: &QString,
            source_name: &QString,
            source_version: &QString,
            build_index: bool,
        ) -> bool;

        #[qinvokable]
        #[cxx_name = "addGames"]
        fn add_games(
            self: Pin<&mut DatabaseOperationModel>,
            project_path: &QString,
            sgf_directory: &QString,
            source_name: &QString,
            source_version: &QString,
            update_index: bool,
        ) -> bool;

        #[qinvokable]
        #[cxx_name = "updatePositionIndex"]
        fn update_position_index(
            self: Pin<&mut DatabaseOperationModel>,
            project_path: &QString,
        ) -> bool;

        #[qinvokable]
        #[cxx_name = "cancelOperation"]
        fn cancel_operation(self: Pin<&mut DatabaseOperationModel>);

        #[qinvokable]
        #[cxx_name = "databaseChoices"]
        fn database_choices(
            &self,
            managed_path: &QString,
            legacy_path: &QString,
            remembered_json: &QString,
        ) -> QString;

        #[qinvokable]
        #[cxx_name = "refreshContinuation"]
        fn refresh_continuation(self: Pin<&mut DatabaseOperationModel>, project_path: &QString);

        #[qinvokable]
        #[cxx_name = "continueSetup"]
        fn continue_setup(self: Pin<&mut DatabaseOperationModel>) -> bool;

        #[qinvokable]
        #[cxx_name = "clearStatus"]
        fn clear_status(self: Pin<&mut DatabaseOperationModel>);
    }

    impl cxx_qt::Threading for DatabaseOperationModel {}
}

pub struct DatabaseOperationModelRust {
    pub(crate) in_progress: bool,
    pub(crate) cancel_requested: bool,
    pub(crate) cancelled: bool,
    pub(crate) cancelled_during_import: bool,

    pub(crate) database_name: QString,
    pub(crate) includes_index: bool,
    pub(crate) can_continue: bool,
    pub(crate) pending_project_path: QString,
    pub(crate) pending_database_name: QString,
    pub(crate) continuation_message: QString,
    pub(crate) continuation_error: QString,
    pub(crate) operation_name: QString,
    pub(crate) stage: QString,
    pub(crate) status_message: QString,
    pub(crate) error_message: QString,

    pub(crate) result_project_path: QString,
    pub(crate) current_item: QString,
    pub(crate) import_error_log: QString,
    pub(crate) index_error_log: QString,

    pub(crate) discovered_sgf_files: i64,
    pub(crate) total_sgf_files: i64,
    pub(crate) processed_sgf_files: i64,
    pub(crate) imported_games: i64,
    pub(crate) added_sources: i64,
    pub(crate) duplicates: i64,
    pub(crate) skipped: i64,
    pub(crate) import_errors: i64,

    pub(crate) total_index_games: i64,
    pub(crate) processed_index_games: i64,
    pub(crate) indexed_games: i64,
    pub(crate) indexed_positions: i64,
    pub(crate) index_errors: i64,

    pub(crate) elapsed_seconds: f64,
    pub(crate) rate: f64,
    pub(crate) remaining_seconds: f64,
    estimate: ProgressEstimate,
    estimate_stage: String,

    cancel_token: Option<Arc<AtomicBool>>,
    operation_id: u64,
}

impl Default for DatabaseOperationModelRust {
    fn default() -> Self {
        Self {
            in_progress: false,
            cancel_requested: false,
            cancelled: false,
            cancelled_during_import: false,

            database_name: QString::default(),
            includes_index: false,
            can_continue: false,
            pending_project_path: QString::default(),
            pending_database_name: QString::default(),
            continuation_message: QString::default(),
            continuation_error: QString::default(),
            operation_name: QString::default(),
            stage: QString::default(),
            status_message: QString::default(),
            error_message: QString::default(),

            result_project_path: QString::default(),
            current_item: QString::default(),
            import_error_log: QString::default(),
            index_error_log: QString::default(),

            discovered_sgf_files: 0,
            total_sgf_files: 0,
            processed_sgf_files: 0,
            imported_games: 0,
            added_sources: 0,
            duplicates: 0,
            skipped: 0,
            import_errors: 0,

            total_index_games: 0,
            processed_index_games: 0,
            indexed_games: 0,
            indexed_positions: 0,
            index_errors: 0,

            elapsed_seconds: 0.0,
            rate: 0.0,
            remaining_seconds: -1.0,
            estimate: ProgressEstimate::default(),
            estimate_stage: String::new(),

            cancel_token: None,
            operation_id: 0,
        }
    }
}

#[derive(Debug)]
enum DatabaseOperationRequest {
    Create {
        project_name: String,
        project_path: PathBuf,
        sgf_directory: PathBuf,
        source_name: String,
        source_version: String,
        build_index: bool,
    },

    AddGames {
        project_path: PathBuf,
        sgf_directory: PathBuf,
        source_name: String,
        source_version: String,
        update_index: bool,
    },

    UpdateIndex {
        project_path: PathBuf,
    },
}

impl DatabaseOperationRequest {
    fn operation_name(&self) -> &'static str {
        match self {
            Self::Create { .. } => "create-database",
            Self::AddGames { .. } => "add-games",
            Self::UpdateIndex { .. } => "update-index",
        }
    }

    fn project_path(&self) -> &Path {
        match self {
            Self::Create { project_path, .. }
            | Self::AddGames { project_path, .. }
            | Self::UpdateIndex { project_path } => project_path,
        }
    }
}

#[derive(Debug, Default)]
struct OperationSummary {
    project_path: PathBuf,

    import_summary: Option<ImportSummary>,
    index_summary: Option<IndexBuildSummary>,

    elapsed_seconds: f64,
}

enum BackgroundCompletion {
    Completed(OperationSummary),
    Cancelled(OperationSummary),
    Failed {
        project_path: PathBuf,
        error: String,
    },
}

impl ffi::DatabaseOperationModel {
    fn database_choices(
        &self,
        managed_path: &QString,
        legacy_path: &QString,
        remembered_json: &QString,
    ) -> QString {
        let remembered: Vec<PathBuf> =
            serde_json::from_str(&remembered_json.to_string()).unwrap_or_default();
        let choices = bermuda::database_choices::discover(
            Path::new(&managed_path.to_string()),
            Path::new(&legacy_path.to_string()),
            &remembered,
        );
        QString::from(serde_json::to_string(&choices).unwrap_or_else(|_| "[]".into()))
    }

    fn create_database(
        self: Pin<&mut Self>,
        project_name: &QString,
        project_path: &QString,
        sgf_directory: &QString,
        source_name: &QString,
        source_version: &QString,
        build_index: bool,
    ) -> bool {
        let request = DatabaseOperationRequest::Create {
            project_name: project_name.to_string(),
            project_path: PathBuf::from(project_path.to_string()),
            sgf_directory: PathBuf::from(sgf_directory.to_string()),
            source_name: source_name.to_string(),
            source_version: source_version.to_string(),
            build_index,
        };

        start_operation(self, request, false)
    }

    fn add_games(
        self: Pin<&mut Self>,
        project_path: &QString,
        sgf_directory: &QString,
        source_name: &QString,
        source_version: &QString,
        update_index: bool,
    ) -> bool {
        let request = DatabaseOperationRequest::AddGames {
            project_path: PathBuf::from(project_path.to_string()),
            sgf_directory: PathBuf::from(sgf_directory.to_string()),
            source_name: source_name.to_string(),
            source_version: source_version.to_string(),
            update_index,
        };

        start_operation(self, request, false)
    }

    fn update_position_index(self: Pin<&mut Self>, project_path: &QString) -> bool {
        let request = DatabaseOperationRequest::UpdateIndex {
            project_path: PathBuf::from(project_path.to_string()),
        };

        start_operation(self, request, false)
    }

    fn cancel_operation(mut self: Pin<&mut Self>) {
        let cancel_token = self.as_ref().get_ref().rust().cancel_token.clone();

        if let Some(cancel_token) = cancel_token {
            cancel_token.store(true, Ordering::Relaxed);

            self.as_mut().set_cancel_requested(true);
            self.as_mut()
                .set_status_message(QString::from("Cancelling…"));
        }
    }

    fn refresh_continuation(mut self: Pin<&mut Self>, project_path: &QString) {
        if self.as_ref().get_ref().rust().in_progress {
            return;
        }
        self.as_mut().set_can_continue(false);
        self.as_mut().set_pending_project_path(project_path.clone());
        self.as_mut().set_pending_database_name(QString::default());
        self.as_mut().set_continuation_message(QString::default());
        self.as_mut().set_continuation_error(QString::default());
        let path = PathBuf::from(project_path.to_string());
        if path.as_os_str().is_empty() {
            return;
        }
        match database_task::load(&path) {
            Ok(Some(task)) => {
                let name = ProjectManager::new()
                    .open(&path)
                    .map(|p| p.name().to_owned())
                    .unwrap_or_else(|_| path.display().to_string());
                self.as_mut().set_pending_database_name(QString::from(name));
                self.as_mut().set_continuation_message(QString::from(match task {
                DatabaseTask::Import { .. } => "Import unfinished. Continue will check the source folder and recognise files already imported.",
                DatabaseTask::Index => "Search preparation unfinished. Continue will keep games already indexed and finish the remaining work.",
            }));
                self.as_mut().set_can_continue(true);
            }
            Ok(None) => {}
            Err(error) => self.as_mut().set_continuation_error(QString::from(format!(
                "Cannot read saved setup: {error:#}"
            ))),
        }
    }

    fn continue_setup(mut self: Pin<&mut Self>) -> bool {
        if self.as_ref().get_ref().rust().in_progress {
            return false;
        }
        let project_path = PathBuf::from(
            self.as_ref()
                .get_ref()
                .rust()
                .pending_project_path
                .to_string(),
        );
        let task = match database_task::load(&project_path) {
            Ok(Some(task)) => task,
            Ok(None) => {
                self.as_mut().set_error_message(QString::from(
                    "No unfinished setup was found for this database.",
                ));
                return false;
            }
            Err(error) => {
                self.as_mut().set_error_message(QString::from(format!(
                    "Cannot read saved setup: {error:#}"
                )));
                return false;
            }
        };
        let request = match task {
            DatabaseTask::Import {
                directory,
                source,
                version,
                build_index,
            } => DatabaseOperationRequest::AddGames {
                project_path,
                sgf_directory: directory,
                source_name: source,
                source_version: version,
                update_index: build_index,
            },
            DatabaseTask::Index => DatabaseOperationRequest::UpdateIndex { project_path },
        };
        start_operation(self, request, true)
    }

    fn clear_status(mut self: Pin<&mut Self>) {
        if self.as_ref().get_ref().rust().in_progress {
            return;
        }

        reset_display(self.as_mut());

        self.as_mut().set_operation_name(QString::default());

        self.as_mut().set_stage(QString::default());
    }
}

fn start_operation(
    mut model: Pin<&mut ffi::DatabaseOperationModel>,
    request: DatabaseOperationRequest,
    continuing: bool,
) -> bool {
    if model.as_ref().get_ref().rust().in_progress {
        model
            .as_mut()
            .set_error_message(QString::from("a database operation is already running"));

        return false;
    }

    if let Err(error) = validate_request(&request) {
        model.as_mut().set_error_message(QString::from(error));

        return false;
    }

    if !continuing && !matches!(&request, DatabaseOperationRequest::Create { .. }) {
        match database_task::load(request.project_path()) {
            Ok(None) => {}
            Ok(Some(_)) => {
                model.as_mut().set_error_message(QString::from(
                    "This database has unfinished work. Choose Continue setup first.",
                ));
                return false;
            }
            Err(error) => {
                model.as_mut().set_error_message(QString::from(format!(
                    "Cannot read saved setup: {error:#}"
                )));
                return false;
            }
        }
    }
    let database_name = match &request {
        DatabaseOperationRequest::Create { project_name, .. } => project_name.clone(),
        _ => ProjectManager::new()
            .open(request.project_path())
            .map(|p| p.name().to_owned())
            .unwrap_or_else(|_| request.project_path().display().to_string()),
    };
    let includes_index = match &request {
        DatabaseOperationRequest::Create { build_index, .. } => *build_index,
        DatabaseOperationRequest::AddGames { update_index, .. } => *update_index,
        DatabaseOperationRequest::UpdateIndex { .. } => true,
    };
    let operation_name = request.operation_name();
    let project_path = request.project_path().to_path_buf();

    let cancel_token = Arc::new(AtomicBool::new(false));

    let operation_id;

    {
        let mut rust = model.as_mut().rust_mut();

        rust.operation_id = rust.operation_id.wrapping_add(1);

        operation_id = rust.operation_id;

        rust.cancel_token = Some(Arc::clone(&cancel_token));
    }

    reset_display(model.as_mut());
    model
        .as_mut()
        .set_database_name(QString::from(database_name));
    model.as_mut().set_includes_index(includes_index);
    model.as_mut().set_can_continue(false);

    model
        .as_mut()
        .set_operation_name(QString::from(operation_name));

    model
        .as_mut()
        .set_result_project_path(QString::from(project_path.to_string_lossy().into_owned()));

    model.as_mut().set_stage(QString::from("starting"));

    model
        .as_mut()
        .set_status_message(QString::from("Preparing database operation…"));

    model.as_mut().set_in_progress(true);

    let qt_thread = model.qt_thread();
    let progress_thread = qt_thread.clone();

    std::thread::spawn(move || {
        let started = Instant::now();

        let completion = execute_operation(
            request,
            Arc::clone(&cancel_token),
            &progress_thread,
            operation_id,
            started,
        );

        qt_thread
            .queue(move |model| {
                finish_operation(model, operation_id, completion);
            })
            .ok();
    });

    true
}

fn execute_operation(
    request: DatabaseOperationRequest,
    cancel_token: Arc<AtomicBool>,
    progress_thread: &cxx_qt::CxxQtThread<ffi::DatabaseOperationModel>,
    operation_id: u64,
    started: Instant,
) -> BackgroundCompletion {
    let project_path = request.project_path().to_path_buf();

    let result = match request {
        DatabaseOperationRequest::Create {
            project_name,
            project_path,
            sgf_directory,
            source_name,
            source_version,
            build_index,
        } => {
            queue_stage(
                progress_thread,
                operation_id,
                "creating",
                "Creating database…",
            );

            let project = match ProjectManager::new().create(project_name, &project_path) {
                Ok(project) => project,

                Err(error) => {
                    return BackgroundCompletion::Failed {
                        project_path,
                        error: error.to_string(),
                    };
                }
            };

            if let Err(error) = database_task::save(
                &project_path,
                &DatabaseTask::Import {
                    directory: sgf_directory.clone(),
                    source: source_name.clone(),
                    version: source_version.clone(),
                    build_index,
                },
            ) {
                return BackgroundCompletion::Failed {
                    project_path,
                    error: format!("Cannot save setup: {error:#}"),
                };
            }

            if cancel_token.load(Ordering::Relaxed) {
                Ok(ExecutionOutcome::Cancelled(OperationSummary {
                    project_path,
                    elapsed_seconds: started.elapsed().as_secs_f64(),
                    ..OperationSummary::default()
                }))
            } else {
                execute_import_and_optional_index(
                    &project,
                    &sgf_directory,
                    &source_name,
                    &source_version,
                    build_index,
                    cancel_token,
                    progress_thread,
                    operation_id,
                    started,
                )
            }
        }

        DatabaseOperationRequest::AddGames {
            project_path,
            sgf_directory,
            source_name,
            source_version,
            update_index,
        } => {
            let project = match ProjectManager::new().open(&project_path) {
                Ok(project) => project,

                Err(error) => {
                    return BackgroundCompletion::Failed {
                        project_path,
                        error: error.to_string(),
                    };
                }
            };

            execute_import_and_optional_index(
                &project,
                &sgf_directory,
                &source_name,
                &source_version,
                update_index,
                cancel_token,
                progress_thread,
                operation_id,
                started,
            )
        }

        DatabaseOperationRequest::UpdateIndex { project_path } => {
            let project = match ProjectManager::new().open(&project_path) {
                Ok(project) => project,

                Err(error) => {
                    return BackgroundCompletion::Failed {
                        project_path,
                        error: error.to_string(),
                    };
                }
            };

            execute_index(
                &project,
                cancel_token,
                progress_thread,
                operation_id,
                OperationSummary {
                    project_path,
                    ..OperationSummary::default()
                },
                started,
            )
        }
    };

    match result {
        Ok(ExecutionOutcome::Completed(mut summary)) => {
            if let Err(error) = database_task::clear(&project_path) {
                return BackgroundCompletion::Failed {
                    project_path,
                    error: format!(
                        "Work completed but its saved task could not be cleared: {error:#}"
                    ),
                };
            }
            summary.elapsed_seconds = started.elapsed().as_secs_f64();

            BackgroundCompletion::Completed(summary)
        }

        Ok(ExecutionOutcome::Cancelled(mut summary)) => {
            summary.elapsed_seconds = started.elapsed().as_secs_f64();

            BackgroundCompletion::Cancelled(summary)
        }

        Err(error) => BackgroundCompletion::Failed {
            project_path,
            error,
        },
    }
}

enum ExecutionOutcome {
    Completed(OperationSummary),
    Cancelled(OperationSummary),
}

fn execute_import_and_optional_index(
    project: &Project,
    sgf_directory: &Path,
    source_name: &str,
    source_version: &str,
    build_index: bool,
    cancel_token: Arc<AtomicBool>,
    progress_thread: &cxx_qt::CxxQtThread<ffi::DatabaseOperationModel>,
    operation_id: u64,
    started: Instant,
) -> Result<ExecutionOutcome, String> {
    database_task::save(
        project.root(),
        &DatabaseTask::Import {
            directory: sgf_directory.to_path_buf(),
            source: source_name.to_owned(),
            version: source_version.to_owned(),
            build_index,
        },
    )
    .map_err(|e| format!("Cannot save import continuation: {e:#}"))?;
    let mut last_update: Option<Instant> = None;
    let mut last_stage: Option<ImportStage> = None;

    let import_cancel = Arc::clone(&cancel_token);

    let import_outcome = import_directory::run_with_progress(
        project,
        source_name,
        source_version,
        sgf_directory,
        move || import_cancel.load(Ordering::Relaxed),
        |progress| {
            let now = Instant::now();

            let stage_changed = last_stage != Some(progress.stage);

            let final_update =
                progress.total_sgf_files > 0 && progress.processed == progress.total_sgf_files;

            let should_send = stage_changed
                || progress.processed == 0
                || final_update
                || last_update.is_none_or(|previous| {
                    now.duration_since(previous) >= Duration::from_millis(100)
                });

            if !should_send {
                return;
            }

            last_stage = Some(progress.stage);
            last_update = Some(now);

            queue_import_progress(progress_thread, operation_id, progress);
        },
    )
    .map_err(|error| error.to_string())?;

    let (import_summary, cancelled) = match import_outcome {
        ImportDirectoryOutcome::Completed(summary) => (summary, false),

        ImportDirectoryOutcome::Cancelled(summary) => (summary, true),
    };

    let summary = OperationSummary {
        project_path: project.root().to_path_buf(),
        import_summary: Some(import_summary),
        ..OperationSummary::default()
    };

    if cancelled {
        return Ok(ExecutionOutcome::Cancelled(summary));
    }

    if !build_index {
        return Ok(ExecutionOutcome::Completed(summary));
    }

    database_task::save(project.root(), &DatabaseTask::Index)
        .map_err(|e| format!("Cannot save search preparation: {e:#}"))?;
    if cancel_token.load(Ordering::Relaxed) {
        return Ok(ExecutionOutcome::Cancelled(summary));
    }

    execute_index(
        project,
        cancel_token,
        progress_thread,
        operation_id,
        summary,
        started,
    )
}

fn execute_index(
    project: &Project,
    cancel_token: Arc<AtomicBool>,
    progress_thread: &cxx_qt::CxxQtThread<ffi::DatabaseOperationModel>,
    operation_id: u64,
    mut summary: OperationSummary,
    _started: Instant,
) -> Result<ExecutionOutcome, String> {
    queue_stage(
        progress_thread,
        operation_id,
        "indexing",
        "Building position index…",
    );

    database_task::save(project.root(), &DatabaseTask::Index)
        .map_err(|e| format!("Cannot save search preparation: {e:#}"))?;
    let mut last_update: Option<Instant> = None;

    let index_cancel = Arc::clone(&cancel_token);

    let index_outcome = index_build::run_with_cancel_token(project, index_cancel, |progress| {
        let now = Instant::now();

        let final_update =
            progress.total_games > 0 && progress.processed_games == progress.total_games;

        let should_send = progress.stage != IndexBuildStage::Indexing
            || progress.processed_games == 0
            || final_update
            || last_update
                .is_none_or(|previous| now.duration_since(previous) >= Duration::from_millis(100));

        if !should_send {
            return;
        }

        last_update = Some(now);

        queue_index_progress(progress_thread, operation_id, progress);
    })
    .map_err(|error| error.to_string())?;

    match index_outcome {
        IndexBuildOutcome::Completed(index_summary) => {
            summary.index_summary = Some(index_summary);

            Ok(ExecutionOutcome::Completed(summary))
        }

        IndexBuildOutcome::Cancelled(index_summary) => {
            summary.index_summary = Some(index_summary);

            Ok(ExecutionOutcome::Cancelled(summary))
        }
    }
}

fn queue_stage(
    qt_thread: &cxx_qt::CxxQtThread<ffi::DatabaseOperationModel>,
    operation_id: u64,
    stage: &'static str,
    message: &'static str,
) {
    qt_thread
        .queue(move |mut model| {
            if !is_current_operation(model.as_ref().get_ref(), operation_id) {
                return;
            }

            model.as_mut().set_stage(QString::from(stage));

            if !model.as_ref().get_ref().rust().cancel_requested {
                model.as_mut().set_status_message(QString::from(message));
            }

            model.as_mut().set_current_item(QString::default());
        })
        .ok();
}

fn update_estimate(
    mut model: Pin<&mut ffi::DatabaseOperationModel>,
    stage: &str,
    elapsed: f64,
    completed: i64,
    total: i64,
) {
    let estimate = {
        let mut rust = model.as_mut().rust_mut();
        if rust.estimate_stage != stage {
            rust.estimate = ProgressEstimate::default();
            rust.estimate_stage = stage.to_owned();
        }
        if stage == "importing" || stage == "indexing" {
            rust.estimate
                .update(elapsed, completed.max(0) as u64, total.max(0) as u64)
        } else {
            None
        }
    };
    let (rate, remaining) = estimate.unwrap_or((0.0, -1.0));
    model.as_mut().set_rate(rate);
    model.as_mut().set_remaining_seconds(remaining);
}

fn queue_import_progress(
    qt_thread: &cxx_qt::CxxQtThread<ffi::DatabaseOperationModel>,
    operation_id: u64,
    progress: ImportProgress,
) {
    let stage = match progress.stage {
        ImportStage::Discovering => "discovering",
        ImportStage::Importing => "importing",
    };

    let message = match progress.stage {
        ImportStage::Discovering => "Discovering SGF files…",

        ImportStage::Importing => "Importing SGF files…",
    };

    let current_item = progress
        .current_file
        .as_ref()
        .map(|path| path.to_string_lossy().into_owned())
        .unwrap_or_default();

    let discovered = usize_to_i64(progress.discovered_sgf_files);

    let total = usize_to_i64(progress.total_sgf_files);

    let processed = usize_to_i64(progress.processed);
    let imported = usize_to_i64(progress.imported);

    let added_sources = usize_to_i64(progress.added_sources);

    let duplicates = usize_to_i64(progress.duplicates);

    let skipped = usize_to_i64(progress.skipped);
    let errors = usize_to_i64(progress.errors);

    let elapsed_seconds = progress.elapsed_seconds;

    qt_thread
        .queue(move |mut model| {
            if !is_current_operation(model.as_ref().get_ref(), operation_id) {
                return;
            }

            model.as_mut().set_stage(QString::from(stage));

            if !model.as_ref().get_ref().rust().cancel_requested {
                model.as_mut().set_status_message(QString::from(message));
            }

            model.as_mut().set_current_item(QString::from(current_item));

            model.as_mut().set_discovered_sgf_files(discovered);

            model.as_mut().set_total_sgf_files(total);

            model.as_mut().set_processed_sgf_files(processed);

            model.as_mut().set_imported_games(imported);

            model.as_mut().set_added_sources(added_sources);

            model.as_mut().set_duplicates(duplicates);

            model.as_mut().set_skipped(skipped);
            model.as_mut().set_import_errors(errors);

            model.as_mut().set_elapsed_seconds(elapsed_seconds);

            update_estimate(model.as_mut(), stage, elapsed_seconds, processed, total);
        })
        .ok();
}

fn queue_index_progress(
    qt_thread: &cxx_qt::CxxQtThread<ffi::DatabaseOperationModel>,
    operation_id: u64,
    progress: IndexBuildProgress,
) {
    let (stage, message) = match progress.stage {
        IndexBuildStage::Indexing => ("indexing", "Indexing games…"),
        IndexBuildStage::Finalising => ("finalising-index", "Finalising search index…"),
        IndexBuildStage::Complete => ("index-finished", "Position index ready"),
    };
    let current_item = progress
        .current_move_file
        .as_ref()
        .map(|path| path.to_string_lossy().into_owned())
        .unwrap_or_default();

    let total = usize_to_i64(progress.total_games);

    let processed = usize_to_i64(progress.processed_games);

    let indexed = usize_to_i64(progress.indexed_games);

    let positions = u64_to_i64(progress.indexed_positions);

    let errors = usize_to_i64(progress.errors);

    let elapsed_seconds = progress.elapsed_seconds;

    qt_thread
        .queue(move |mut model| {
            if !is_current_operation(model.as_ref().get_ref(), operation_id) {
                return;
            }

            model.as_mut().set_stage(QString::from(stage));

            if !model.as_ref().get_ref().rust().cancel_requested {
                model.as_mut().set_status_message(QString::from(message));
            }

            model.as_mut().set_current_item(QString::from(current_item));

            model.as_mut().set_total_index_games(total);

            model.as_mut().set_processed_index_games(processed);

            model.as_mut().set_indexed_games(indexed);

            model.as_mut().set_indexed_positions(positions);

            model.as_mut().set_index_errors(errors);

            model.as_mut().set_elapsed_seconds(elapsed_seconds);

            update_estimate(model.as_mut(), stage, elapsed_seconds, processed, total);
        })
        .ok();
}

fn finish_operation(
    mut model: Pin<&mut ffi::DatabaseOperationModel>,
    operation_id: u64,
    completion: BackgroundCompletion,
) {
    if !is_current_operation(model.as_ref().get_ref(), operation_id) {
        return;
    }

    match completion {
        BackgroundCompletion::Completed(summary) => {
            apply_summary(model.as_mut(), &summary);

            model.as_mut().set_stage(QString::from("complete"));

            model
                .as_mut()
                .set_status_message(QString::from("Operation complete"));

            model.as_mut().set_cancelled(false);
            model.as_mut().set_error_message(QString::default());
        }

        BackgroundCompletion::Cancelled(summary) => {
            model.as_mut().set_cancelled_during_import(
                summary.import_summary.is_some() && summary.index_summary.is_none(),
            );
            apply_summary(model.as_mut(), &summary);

            model.as_mut().set_stage(QString::from("cancelled"));

            model
                .as_mut()
                .set_status_message(QString::from("Stopped. Your progress is saved."));

            model.as_mut().set_cancelled(true);
            model.as_mut().set_error_message(QString::default());
        }

        BackgroundCompletion::Failed {
            project_path,
            error,
        } => {
            model.as_mut().set_result_project_path(QString::from(
                project_path.to_string_lossy().into_owned(),
            ));

            model.as_mut().set_stage(QString::from("failed"));

            model
                .as_mut()
                .set_status_message(QString::from("Operation failed"));

            model.as_mut().set_cancelled(false);

            model.as_mut().set_error_message(QString::from(error));
        }
    }

    model.as_mut().set_cancel_requested(false);
    model.as_mut().set_in_progress(false);

    model.as_mut().rust_mut().cancel_token = None;
    let path = model.as_ref().get_ref().rust().result_project_path.clone();
    model.as_mut().refresh_continuation(&path);
}

fn apply_summary(mut model: Pin<&mut ffi::DatabaseOperationModel>, summary: &OperationSummary) {
    model.as_mut().set_result_project_path(QString::from(
        summary.project_path.to_string_lossy().into_owned(),
    ));

    model.as_mut().set_elapsed_seconds(summary.elapsed_seconds);

    if let Some(import) = &summary.import_summary {
        model
            .as_mut()
            .set_total_sgf_files(usize_to_i64(import.total_sgf_files));

        model
            .as_mut()
            .set_processed_sgf_files(usize_to_i64(import.processed));

        model
            .as_mut()
            .set_imported_games(usize_to_i64(import.imported));

        model
            .as_mut()
            .set_added_sources(usize_to_i64(import.added_sources));

        model
            .as_mut()
            .set_duplicates(usize_to_i64(import.duplicates));

        model.as_mut().set_skipped(usize_to_i64(import.skipped));

        model
            .as_mut()
            .set_import_errors(usize_to_i64(import.errors));

        model
            .as_mut()
            .set_import_error_log(optional_path(&import.error_log));
    }

    if let Some(index) = &summary.index_summary {
        model
            .as_mut()
            .set_total_index_games(usize_to_i64(index.total_games));

        model
            .as_mut()
            .set_processed_index_games(usize_to_i64(index.processed_games));

        model
            .as_mut()
            .set_indexed_games(usize_to_i64(index.indexed_games));

        model
            .as_mut()
            .set_indexed_positions(u64_to_i64(index.indexed_positions));

        model.as_mut().set_index_errors(usize_to_i64(index.errors));

        model
            .as_mut()
            .set_index_error_log(optional_path(&index.error_log));
    }
}

fn reset_display(mut model: Pin<&mut ffi::DatabaseOperationModel>) {
    model.as_mut().set_cancel_requested(false);
    model.as_mut().set_cancelled(false);
    model.as_mut().set_cancelled_during_import(false);

    model.as_mut().set_status_message(QString::default());

    model.as_mut().set_error_message(QString::default());

    model.as_mut().set_result_project_path(QString::default());

    model.as_mut().set_current_item(QString::default());

    model.as_mut().set_import_error_log(QString::default());

    model.as_mut().set_index_error_log(QString::default());

    model.as_mut().set_discovered_sgf_files(0);
    model.as_mut().set_total_sgf_files(0);
    model.as_mut().set_processed_sgf_files(0);
    model.as_mut().set_imported_games(0);
    model.as_mut().set_added_sources(0);
    model.as_mut().set_duplicates(0);
    model.as_mut().set_skipped(0);
    model.as_mut().set_import_errors(0);

    model.as_mut().set_total_index_games(0);
    model.as_mut().set_processed_index_games(0);
    model.as_mut().set_indexed_games(0);
    model.as_mut().set_indexed_positions(0);
    model.as_mut().set_index_errors(0);

    model.as_mut().set_elapsed_seconds(0.0);
    model.as_mut().set_rate(0.0);
    model.as_mut().set_remaining_seconds(-1.0);
    let mut rust = model.as_mut().rust_mut();
    rust.estimate = ProgressEstimate::default();
    rust.estimate_stage.clear();
}

fn validate_request(request: &DatabaseOperationRequest) -> Result<(), String> {
    if request.project_path().as_os_str().is_empty() {
        return Err("project path must not be empty".to_owned());
    }

    match request {
        DatabaseOperationRequest::Create {
            project_name,
            sgf_directory,
            source_name,
            source_version,
            ..
        } => {
            validate_import_fields(project_name, sgf_directory, source_name, source_version)?;
        }

        DatabaseOperationRequest::AddGames {
            sgf_directory,
            source_name,
            source_version,
            ..
        } => {
            validate_import_fields(
                "existing project",
                sgf_directory,
                source_name,
                source_version,
            )?;
        }

        DatabaseOperationRequest::UpdateIndex { .. } => {}
    }

    Ok(())
}

fn validate_import_fields(
    project_name: &str,
    sgf_directory: &Path,
    source_name: &str,
    source_version: &str,
) -> Result<(), String> {
    if project_name.trim().is_empty() {
        return Err("project name must not be empty".to_owned());
    }

    if sgf_directory.as_os_str().is_empty() {
        return Err("SGF directory must not be empty".to_owned());
    }

    if source_name.trim().is_empty() {
        return Err("source name must not be empty".to_owned());
    }

    if source_version.trim().is_empty() {
        return Err("source version must not be empty".to_owned());
    }

    Ok(())
}

fn is_current_operation(model: &ffi::DatabaseOperationModel, operation_id: u64) -> bool {
    model.rust().operation_id == operation_id
}

fn optional_path(path: &Option<PathBuf>) -> QString {
    match path {
        Some(path) => QString::from(path.to_string_lossy().into_owned()),

        None => QString::default(),
    }
}

fn usize_to_i64(value: usize) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}

fn u64_to_i64(value: u64) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}
