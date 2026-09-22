#![allow(clippy::too_many_arguments)]

use std::{
    collections::HashSet,
    env,
    fmt::Write as _,
    fs,
    path::{Path, PathBuf},
    pin::Pin,
    sync::{
        atomic::{AtomicU64, Ordering},
        mpsc,
    },
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use bermuda::{
    AnalysisOutcome, AnalysisRequest, AnalysisResult, AnalysisVertex, Board, Collection, Colour,
    GameRecord, KataGoConfiguration, KataGoWorker as CoreKataGoWorker, KataGoWorkerEvent, Metadata,
    Move, Node, PositionOccurrence, PositionState, SetupStone, StudyAnnotationKind,
    StudyDocumentMetadata, StudyMarkupKind, StudyOrigin, StudySourceLocation, StudyTree,
    analysis_position_from_states, build_study_tree, extract_main_variation,
    importer::ImportOutcome, indexer::POSITION_INDEX_VERSION, main_variation_comments,
    parse_collection, position_fingerprint, project::Project, project_manager::ProjectManager,
    replay_positions, write_collection_sgf, write_game_record_sgf,
};
use cxx_qt::{CxxQtType, Threading};
use cxx_qt_lib::QString;
use directories::ProjectDirs;

const PERSONAL_PROJECT_NAME: &str = "My Games";
const PERSONAL_PROJECT_DIRECTORY: &str = "personal-corpus";
const STUDY_LIBRARY_DIRECTORY: &str = "study-library";

const PLAYED_GAME_SOURCE_NAME: &str = "Bermuda";
const PLAYED_GAME_SOURCE_VERSION: &str = "play-v1";

static PLAYED_GAME_LOCATOR_SEQUENCE: AtomicU64 = AtomicU64::new(0);
static STUDY_DOCUMENT_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[cxx_qt::bridge]
mod ffi {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");

        type QString = cxx_qt_lib::QString;
    }

    unsafe extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qproperty(i32, board_size)]
        #[qproperty(QString, stones_json)]
        #[qproperty(i32, move_number)]
        #[qproperty(i32, move_count)]
        #[qproperty(i32, last_move_x)]
        #[qproperty(i32, last_move_y)]
        #[qproperty(QString, black_player)]
        #[qproperty(QString, white_player)]
        #[qproperty(QString, komi)]
        #[qproperty(QString, source_comment)]
        #[qproperty(bool, has_source_comments)]
        #[qproperty(QString, sgf_tree_json)]
        #[qproperty(i32, sgf_tree_current_node)]
        #[qproperty(QString, board_markup_json)]
        #[qproperty(bool, can_undo_study_edit)]
        #[qproperty(bool, can_redo_study_edit)]
        #[qproperty(QString, error_message)]
        #[qproperty(bool, katago_analysis_in_progress)]
        #[qproperty(QString, katago_analysis_text)]
        #[qproperty(QString, katago_candidate_points_json)]
        type BermudaApp = super::BermudaAppRust;

        #[qinvokable]
        #[cxx_name = "projectExists"]
        fn project_exists(self: &BermudaApp, project_path: &QString) -> bool;

        #[qinvokable]
        #[cxx_name = "personalProjectPath"]
        fn personal_project_path(self: &BermudaApp) -> QString;

        #[qinvokable]
        #[cxx_name = "studyLibraryPath"]
        fn study_library_path(self: &BermudaApp) -> QString;

        #[qinvokable]
        #[cxx_name = "studyLibraryEntriesJson"]
        fn study_library_entries_json(self: Pin<&mut BermudaApp>) -> QString;

        #[qinvokable]
        #[cxx_name = "deleteStudyLibraryDocument"]
        fn delete_study_library_document(self: Pin<&mut BermudaApp>, path: &QString) -> bool;

        #[qinvokable]
        #[cxx_name = "ensurePersonalProject"]
        fn ensure_personal_project(self: Pin<&mut BermudaApp>) -> bool;

        #[qinvokable]
        #[cxx_name = "loadGame"]
        fn load_game(self: Pin<&mut BermudaApp>, project_path: &QString, game_id: i64) -> bool;

        #[qinvokable]
        #[cxx_name = "loadSgf"]
        fn load_sgf(self: Pin<&mut BermudaApp>, sgf_path: &QString) -> bool;

        #[qinvokable]
        #[cxx_name = "loadJosekiStudy"]
        fn load_joseki_study(
            self: Pin<&mut BermudaApp>,
            sgf_path: &QString,
            node_id: &QString,
            current_move_count: i32,
        ) -> bool;

        #[qinvokable]
        #[cxx_name = "savePlayedGameSgf"]
        fn save_played_game_sgf(self: Pin<&mut BermudaApp>, sgf_path: &QString) -> bool;

        #[qinvokable]
        #[cxx_name = "addPlayedGameToMyGames"]
        fn add_played_game_to_my_games(self: Pin<&mut BermudaApp>) -> bool;

        #[qinvokable]
        #[cxx_name = "restorePlayedGame"]
        fn restore_played_game(self: Pin<&mut BermudaApp>) -> bool;

        #[qinvokable]
        #[cxx_name = "discardPlayedGame"]
        fn discard_played_game(self: Pin<&mut BermudaApp>) -> bool;

        #[qinvokable]
        #[cxx_name = "removeGameFromMyGames"]
        fn remove_game_from_my_games(self: Pin<&mut BermudaApp>, game_source_id: i64) -> bool;

        #[qinvokable]
        #[cxx_name = "newPosition"]
        fn new_position(self: Pin<&mut BermudaApp>, board_size: i32) -> bool;

        #[qinvokable]
        #[cxx_name = "newGame"]
        fn new_game(
            self: Pin<&mut BermudaApp>,
            board_size: i32,
            black_player: &QString,
            white_player: &QString,
            komi: &QString,
        ) -> bool;

        #[qinvokable]
        #[cxx_name = "playGamePoint"]
        fn play_game_point(self: Pin<&mut BermudaApp>, x: i32, y: i32) -> bool;

        #[qinvokable]
        #[cxx_name = "playGamePass"]
        fn play_game_pass(self: Pin<&mut BermudaApp>) -> bool;

        #[qinvokable]
        #[cxx_name = "undoGameMove"]
        fn undo_game_move(self: Pin<&mut BermudaApp>) -> bool;

        #[qinvokable]
        #[cxx_name = "resignGame"]
        fn resign_game(self: Pin<&mut BermudaApp>) -> QString;

        #[qinvokable]
        #[cxx_name = "finishGame"]
        fn finish_game(self: Pin<&mut BermudaApp>, result: &QString) -> bool;

        #[qinvokable]
        #[cxx_name = "snapshotSearchSource"]
        fn snapshot_search_source(self: Pin<&mut BermudaApp>) -> bool;

        #[qinvokable]
        #[cxx_name = "restoreSearchSource"]
        fn restore_search_source(self: Pin<&mut BermudaApp>) -> bool;

        #[qinvokable]
        #[cxx_name = "snapshotWorkspace"]
        fn snapshot_workspace(self: Pin<&mut BermudaApp>) -> bool;

        #[qinvokable]
        #[cxx_name = "swapWorkspace"]
        fn swap_workspace(self: Pin<&mut BermudaApp>) -> bool;

        #[qinvokable]
        #[cxx_name = "editPositionPoint"]
        fn edit_position_point(self: Pin<&mut BermudaApp>, x: i32, y: i32, tool: &QString) -> bool;

        #[qinvokable]
        #[cxx_name = "showPosition"]
        fn show_position(self: Pin<&mut BermudaApp>, move_number: i32) -> bool;

        #[qinvokable]
        #[cxx_name = "stoneMoveNumber"]
        fn stone_move_number(self: Pin<&mut BermudaApp>, move_number: i32, x: i32, y: i32) -> i32;

        #[qinvokable]
        #[cxx_name = "studyBoardMarkupJson"]
        fn study_board_markup_json(self: &BermudaApp, move_number: i32) -> QString;

        #[qinvokable]
        #[cxx_name = "setStudyAnnotation"]
        fn set_study_annotation(
            self: Pin<&mut BermudaApp>,
            move_number: i32,
            x: i32,
            y: i32,
            tool: &QString,
            text: &QString,
        ) -> bool;

        #[qinvokable]
        #[cxx_name = "undoStudyEdit"]
        fn undo_study_edit(self: Pin<&mut BermudaApp>) -> bool;

        #[qinvokable]
        #[cxx_name = "redoStudyEdit"]
        fn redo_study_edit(self: Pin<&mut BermudaApp>) -> bool;

        #[qinvokable]
        #[cxx_name = "setStudyComment"]
        fn set_study_comment(self: Pin<&mut BermudaApp>, move_number: i32, text: &QString) -> bool;

        #[qinvokable]
        #[cxx_name = "deleteStudyFromHere"]
        fn delete_study_from_here(self: Pin<&mut BermudaApp>) -> bool;

        #[qinvokable]
        #[cxx_name = "insertStudyNode"]
        fn insert_study_node(self: Pin<&mut BermudaApp>) -> bool;

        #[qinvokable]
        #[cxx_name = "addStudyMove"]
        fn add_study_move(self: Pin<&mut BermudaApp>, x: i32, y: i32) -> bool;

        #[qinvokable]
        #[cxx_name = "studyBranchAvailable"]
        fn study_branch_available(self: Pin<&mut BermudaApp>, move_number: i32) -> bool;

        #[qinvokable]
        #[cxx_name = "addStudyBranch"]
        fn add_study_branch(self: Pin<&mut BermudaApp>, x: i32, y: i32) -> bool;

        #[qinvokable]
        #[cxx_name = "addStudyPass"]
        fn add_study_pass(self: Pin<&mut BermudaApp>) -> bool;

        #[qinvokable]
        #[cxx_name = "addStudyBranchPass"]
        fn add_study_branch_pass(self: Pin<&mut BermudaApp>) -> bool;

        #[qinvokable]
        #[cxx_name = "showStudyStructureNode"]
        fn show_study_structure_node(self: Pin<&mut BermudaApp>, node_id: i32) -> bool;

        #[qinvokable]
        #[cxx_name = "showSgfNode"]
        fn show_sgf_node(self: Pin<&mut BermudaApp>, node_id: i32) -> bool;

        #[qinvokable]
        #[cxx_name = "analyseCurrentPosition"]
        fn analyse_current_position(
            self: Pin<&mut BermudaApp>,
            visit_budget: i32,
            analysis_komi: &QString,
            saved_executable: &QString,
            saved_model: &QString,
            saved_config: &QString,
        ) -> bool;

        #[qinvokable]
        #[cxx_name = "cancelKataGoAnalysis"]
        fn cancel_katago_analysis(self: Pin<&mut BermudaApp>) -> bool;

        #[qinvokable]
        #[cxx_name = "hypotheticalMoveStones"]
        fn hypothetical_move_stones(
            self: Pin<&mut BermudaApp>,
            move_number: i32,
            x: i32,
            y: i32,
            colour: &QString,
        ) -> QString;

        #[qinvokable]
        #[cxx_name = "hypotheticalSequenceStones"]
        fn hypothetical_sequence_stones(
            self: Pin<&mut BermudaApp>,
            move_number: i32,
            first_x: i32,
            first_y: i32,
            first_colour: &QString,
            second_x: i32,
            second_y: i32,
            second_colour: &QString,
        ) -> QString;
    }

    impl cxx_qt::Threading for BermudaApp {}
}

#[derive(Debug, Clone)]
struct StudyHistoryEntry {
    collection: Collection,
    metadata: StudyDocumentMetadata,
    selection: Option<StudySourceLocation>,
    move_number: i32,
}

#[derive(Debug, Clone, Default)]
struct StudyEditHistory {
    undo: Vec<StudyHistoryEntry>,
    redo: Vec<StudyHistoryEntry>,
}

impl StudyEditHistory {
    const LIMIT: usize = 100;

    fn push_undo(&mut self, entry: StudyHistoryEntry) {
        Self::push_limited(&mut self.undo, entry);
    }

    fn push_redo(&mut self, entry: StudyHistoryEntry) {
        Self::push_limited(&mut self.redo, entry);
    }

    fn record_edit(&mut self, entry: StudyHistoryEntry) {
        self.push_undo(entry);
        self.redo.clear();
    }

    fn push_limited(stack: &mut Vec<StudyHistoryEntry>, entry: StudyHistoryEntry) {
        if stack.len() >= Self::LIMIT {
            stack.remove(0);
        }

        stack.push(entry);
    }
}

#[derive(Debug, Clone)]
struct StudyDocumentState {
    metadata: StudyDocumentMetadata,
    collection: Option<Collection>,
    library_path: Option<PathBuf>,
    history: StudyEditHistory,
}

impl StudyDocumentState {
    fn new(origin: StudyOrigin) -> Self {
        Self {
            metadata: StudyDocumentMetadata::new(origin),
            collection: None,
            library_path: None,
            history: StudyEditHistory::default(),
        }
    }
}

#[derive(Debug, Clone)]
struct LoadedDocument {
    description: String,
    positions: Vec<PositionState>,
    source_comments: Vec<String>,
    study_tree: Option<StudyTree>,
    study: StudyDocumentState,
    editable: bool,
    playable: bool,
    finished: bool,
    result: Option<String>,
    black_player: Option<String>,
    white_player: Option<String>,
    komi: Option<f32>,
    played_source_locator: Option<String>,
}

#[derive(Debug, Clone)]
struct SearchSourceSnapshot {
    document: LoadedDocument,
    move_number: i32,
}

#[derive(Debug)]
struct WorkspaceSnapshot {
    document: Option<LoadedDocument>,
    move_number: i32,
    search_source_snapshot: Option<SearchSourceSnapshot>,
}

struct KataGoWorker {
    sender: mpsc::Sender<KataGoWorkerCommand>,
}

impl Drop for KataGoWorker {
    fn drop(&mut self) {
        /*
         * Do not block the Qt thread waiting for KataGo. The bridge
         * receives Shutdown asynchronously; its core worker then
         * terminates any active analysis and shuts KataGo down cleanly.
         */
        let _ = self.sender.send(KataGoWorkerCommand::Shutdown);
    }
}

enum KataGoWorkerCommand {
    Analyse(KataGoWorkerRequest),
    Cancel { analysis_id: u64 },
    Shutdown,
}

struct KataGoWorkerRequest {
    analysis_id: u64,
    configuration: KataGoConfiguration,
    request: AnalysisRequest,
    expected_player: Colour,
}

pub struct BermudaAppRust {
    board_size: i32,
    stones_json: QString,
    move_number: i32,
    move_count: i32,
    last_move_x: i32,
    last_move_y: i32,
    black_player: QString,
    white_player: QString,
    komi: QString,
    source_comment: QString,
    has_source_comments: bool,
    sgf_tree_json: QString,
    sgf_tree_current_node: i32,
    board_markup_json: QString,
    can_undo_study_edit: bool,
    can_redo_study_edit: bool,
    error_message: QString,
    katago_analysis_in_progress: bool,
    katago_analysis_text: QString,
    katago_candidate_points_json: QString,

    loaded_document: Option<LoadedDocument>,
    played_game_document: Option<LoadedDocument>,
    search_source_snapshot: Option<SearchSourceSnapshot>,
    workspace_snapshot: Option<WorkspaceSnapshot>,

    katago_analysis_id: u64,
    katago_worker: Option<KataGoWorker>,
}

impl Default for BermudaAppRust {
    fn default() -> Self {
        Self {
            board_size: 19,
            stones_json: QString::from("[]"),
            move_number: 0,
            move_count: 0,
            last_move_x: -1,
            last_move_y: -1,
            black_player: QString::default(),
            white_player: QString::default(),
            komi: QString::default(),
            source_comment: QString::default(),
            has_source_comments: false,
            sgf_tree_json: QString::from("[]"),
            sgf_tree_current_node: -1,
            board_markup_json: QString::from("[]"),
            can_undo_study_edit: false,
            can_redo_study_edit: false,
            error_message: QString::default(),
            katago_analysis_in_progress: false,
            katago_analysis_text: QString::default(),
            katago_candidate_points_json: QString::from("[]"),

            loaded_document: None,
            played_game_document: None,
            search_source_snapshot: None,
            workspace_snapshot: None,

            katago_analysis_id: 0,
            katago_worker: None,
        }
    }
}

struct KataGoLatestRequest {
    analysis_id: u64,
    request_id: String,
    expected_player: Colour,
    board_size: u8,
}

struct KataGoPresentation {
    text: String,
    candidate_points_json: String,
}

fn queue_katago_completion(
    qt_thread: &cxx_qt::CxxQtThread<ffi::BermudaApp>,
    analysis_id: u64,
    completion: Result<KataGoPresentation, String>,
) {
    qt_thread
        .queue(move |mut app| {
            /*
             * A superseded analysis may still produce a termination event.
             * Never allow that obsolete completion to overwrite state
             * belonging to the newest request.
             */
            if app.rust().katago_analysis_id != analysis_id {
                return;
            }

            app.as_mut().set_katago_analysis_in_progress(false);

            match completion {
                Ok(presentation) => {
                    app.as_mut()
                        .set_katago_analysis_text(QString::from(presentation.text));

                    app.as_mut().set_katago_candidate_points_json(QString::from(
                        presentation.candidate_points_json,
                    ));
                }

                Err(error) => {
                    app.as_mut()
                        .set_katago_candidate_points_json(QString::from("[]"));

                    app.as_mut().set_error_message(QString::from(error.clone()));

                    app.as_mut().set_katago_analysis_text(QString::from(format!(
                        "Analysis failed: {error}"
                    )));
                }
            }
        })
        .ok();
}

fn start_katago_worker(qt_thread: cxx_qt::CxxQtThread<ffi::BermudaApp>) -> KataGoWorker {
    let (sender, receiver) = mpsc::channel();

    /*
     * This lightweight Qt bridge remains responsive to new commands while
     * CoreKataGoWorker owns the long-lived KataGo process on its own thread.
     *
     * That separation is what allows a newer GUI request to call
     * submit_latest(), terminate obsolete analysis, and replace pending work
     * without blocking either the Qt thread or this command receiver.
     */
    std::thread::spawn(move || {
        let mut core_worker: Option<(KataGoConfiguration, CoreKataGoWorker)> = None;
        let mut latest: Option<KataGoLatestRequest> = None;

        loop {
            match receiver.recv_timeout(Duration::from_millis(10)) {
                Ok(KataGoWorkerCommand::Shutdown) => break,

                Ok(KataGoWorkerCommand::Cancel { analysis_id }) => {
                    /*
                     * The GUI has already invalidated the old analysis
                     * generation. Its eventual Terminated event must not
                     * become a visible failure.
                     */
                    latest = None;

                    let cancel_result = core_worker.as_ref().map(|(_, worker)| worker.cancel_all());

                    if let Some(Err(error)) = cancel_result {
                        core_worker.take();

                        queue_katago_completion(
                            &qt_thread,
                            analysis_id,
                            Err(format!("cancelling KataGo analysis: {error}")),
                        );
                    }
                }

                Ok(KataGoWorkerCommand::Analyse(work)) => {
                    let KataGoWorkerRequest {
                        analysis_id,
                        configuration,
                        request,
                        expected_player,
                    } = work;

                    let request_id = request.id.clone();
                    let board_size = request.board_size;

                    let configuration_changed = core_worker
                        .as_ref()
                        .is_none_or(|(current, _)| current != &configuration);

                    if configuration_changed {
                        if let Some((_, old_worker)) = core_worker.take() {
                            let _ = old_worker.shutdown();
                        }

                        match CoreKataGoWorker::start(&configuration) {
                            Ok(worker) => {
                                core_worker = Some((configuration.clone(), worker));
                            }

                            Err(error) => {
                                latest = None;

                                queue_katago_completion(
                                    &qt_thread,
                                    analysis_id,
                                    Err(format!("starting KataGo: {error}")),
                                );

                                continue;
                            }
                        }
                    }

                    latest = Some(KataGoLatestRequest {
                        analysis_id,
                        request_id: request_id.clone(),
                        expected_player,
                        board_size,
                    });

                    let submit_result = core_worker
                        .as_ref()
                        .expect("KataGo core worker was started above")
                        .1
                        .submit_latest(request);

                    if let Err(error) = submit_result {
                        /*
                         * A failed control write means this engine instance
                         * can no longer be trusted. Drop it; the next request
                         * will start a fresh process.
                         */
                        core_worker.take();
                        latest = None;

                        queue_katago_completion(
                            &qt_thread,
                            analysis_id,
                            Err(format!("sending analysis to KataGo worker: {error}")),
                        );
                    }
                }

                Err(mpsc::RecvTimeoutError::Timeout) => {}

                Err(mpsc::RecvTimeoutError::Disconnected) => break,
            }

            /*
             * Drain all events currently available from the core worker.
             * Obsolete termination/completion events are harmless because
             * only the newest request is represented by `latest`.
             */
            loop {
                let event = match core_worker.as_ref() {
                    Some((_, worker)) => match worker.try_recv() {
                        Ok(event) => event,
                        Err(mpsc::TryRecvError::Empty) => break,

                        Err(mpsc::TryRecvError::Disconnected) => {
                            let current_analysis_id =
                                latest.as_ref().map(|request| request.analysis_id);

                            core_worker.take();
                            latest = None;

                            if let Some(analysis_id) = current_analysis_id {
                                queue_katago_completion(
                                    &qt_thread,
                                    analysis_id,
                                    Err("KataGo worker stopped unexpectedly".to_owned()),
                                );
                            }

                            break;
                        }
                    },

                    None => break,
                };

                match event {
                    KataGoWorkerEvent::Analysis {
                        request_id,
                        outcome,
                    } => {
                        let Some(current) = latest.as_ref() else {
                            continue;
                        };

                        if current.request_id != request_id {
                            // Completion or termination of superseded work.
                            continue;
                        }

                        let analysis_id = current.analysis_id;
                        let expected_player = current.expected_player;
                        let board_size = current.board_size;

                        latest = None;

                        let completion = match outcome {
                            AnalysisOutcome::Complete(analysis) => {
                                if analysis.current_player != expected_player {
                                    /*
                                     * Preserve the previous safety rule:
                                     * a side-to-move mismatch means this
                                     * engine instance should not be reused.
                                     */
                                    core_worker.take();

                                    Err(format!(
                                        "KataGo reports {} to move, but Bermuda reports {} to move; \
                                         this position is not safe to analyse yet",
                                        colour_name(analysis.current_player),
                                        colour_name(expected_player),
                                    ))
                                } else {
                                    let candidate_points_json =
                                        format_katago_candidate_points(&analysis);

                                    format_katago_analysis(&analysis, board_size).map(|text| {
                                        KataGoPresentation {
                                            text,
                                            candidate_points_json,
                                        }
                                    })
                                }
                            }

                            AnalysisOutcome::Terminated => {
                                Err("KataGo analysis was terminated".to_owned())
                            }
                        };

                        queue_katago_completion(&qt_thread, analysis_id, completion);
                    }

                    KataGoWorkerEvent::Error {
                        request_id,
                        message,
                    } => {
                        /*
                         * A process/protocol error invalidates this engine.
                         * The newest GUI request, if any, must be told that
                         * its analysis cannot complete.
                         */
                        let current_analysis_id =
                            latest.as_ref().map(|request| request.analysis_id);

                        core_worker.take();
                        latest = None;

                        if let Some(analysis_id) = current_analysis_id {
                            queue_katago_completion(
                                &qt_thread,
                                analysis_id,
                                Err(format!(
                                    "analysing the displayed position \
                                     (request {request_id:?}): {message}"
                                )),
                            );
                        }
                    }
                }
            }
        }

        if let Some((_, worker)) = core_worker.take() {
            let _ = worker.shutdown();
        }
    });

    KataGoWorker { sender }
}

fn format_katago_analysis(analysis: &AnalysisResult, board_size: u8) -> Result<String, String> {
    let candidate = analysis
        .candidates
        .first()
        .ok_or_else(|| "KataGo returned no candidate moves".to_owned())?;

    let candidate_name = analysis_vertex_name(&candidate.vertex, board_size)?;

    Ok(format!(
        "Suggested: {candidate_name}     Black ({:.1} pts, {:.0}% win)     {} visits",
        candidate.score_lead,
        candidate.win_rate * 100.0,
        analysis.visits,
    ))
}

fn format_katago_candidate_points(analysis: &AnalysisResult) -> String {
    const MAX_CANDIDATES: usize = 5;

    let mut candidates = analysis
        .candidates
        .iter()
        .filter_map(|candidate| {
            let AnalysisVertex::Point(point) = candidate.vertex else {
                return None;
            };

            Some((point, candidate.visits))
        })
        .collect::<Vec<_>>();

    candidates.sort_by(|left, right| right.1.cmp(&left.1));

    let mut json = String::from("[");
    let mut first = true;

    for (point, visits) in candidates.into_iter().take(MAX_CANDIDATES) {
        if !first {
            json.push(',');
        }

        first = false;

        /*
         * Bermuda analysis coordinates have their origin at the lower
         * edge. QML converts y to the goban's top-down coordinates.
         */
        let _ = write!(
            json,
            r#"{{"x":{},"y":{},"visits":{}}}"#,
            point.x, point.y, visits,
        );
    }

    json.push(']');
    json
}

fn cancel_katago_analysis_impl(mut app: Pin<&mut ffi::BermudaApp>, report_cancelled: bool) -> bool {
    app.as_mut().set_error_message(QString::default());
    app.as_mut()
        .set_katago_candidate_points_json(QString::from("[]"));

    if !app.as_ref().rust().katago_analysis_in_progress {
        return true;
    }

    /*
     * Invalidate this generation immediately. Any result already queued
     * for the old request will therefore be ignored by the Qt callback.
     */
    let analysis_id = {
        let mut rust = app.as_mut().rust_mut();

        rust.katago_analysis_id = rust.katago_analysis_id.wrapping_add(1);
        rust.katago_analysis_id
    };

    let send_result = match app.as_ref().rust().katago_worker.as_ref() {
        Some(worker) => worker
            .sender
            .send(KataGoWorkerCommand::Cancel { analysis_id }),

        None => {
            app.as_mut().set_katago_analysis_in_progress(false);

            if report_cancelled {
                app.as_mut()
                    .set_katago_analysis_text(QString::from("Analysis cancelled"));
            } else {
                app.as_mut().set_katago_analysis_text(QString::default());
            }

            return true;
        }
    };

    match send_result {
        Ok(()) => {
            app.as_mut().set_katago_analysis_in_progress(false);

            if report_cancelled {
                app.as_mut()
                    .set_katago_analysis_text(QString::from("Analysis cancelled"));
            } else {
                app.as_mut().set_katago_analysis_text(QString::default());
            }

            true
        }

        Err(error) => {
            app.as_mut().rust_mut().katago_worker = None;
            app.as_mut().set_katago_analysis_in_progress(false);

            let message = format!("cancelling KataGo analysis: {error}");

            app.as_mut()
                .set_error_message(QString::from(message.clone()));

            app.as_mut()
                .set_katago_analysis_text(QString::from(format!("Analysis failed: {message}")));

            false
        }
    }
}

impl ffi::BermudaApp {
    fn project_exists(&self, project_path: &QString) -> bool {
        let path = project_path.to_string();

        !path.trim().is_empty() && ProjectManager::new().open(Path::new(&path)).is_ok()
    }

    fn personal_project_path(&self) -> QString {
        match personal_project_root() {
            Ok(path) => QString::from(path.to_string_lossy().as_ref()),
            Err(_) => QString::default(),
        }
    }

    fn study_library_path(&self) -> QString {
        match study_library_root() {
            Ok(path) => QString::from(path.to_string_lossy().as_ref()),
            Err(_) => QString::default(),
        }
    }

    fn study_library_entries_json(mut self: Pin<&mut Self>) -> QString {
        match study_library_catalogue_json() {
            Ok(json) => {
                self.as_mut().set_error_message(QString::default());
                QString::from(json)
            }

            Err(error) => {
                self.as_mut().set_error_message(QString::from(error));
                QString::from("[]")
            }
        }
    }

    fn delete_study_library_document(mut self: Pin<&mut Self>, path: &QString) -> bool {
        let path = PathBuf::from(path.to_string());

        let result = {
            let self_ref = self.as_ref();
            delete_study_library_document_path(self_ref.rust(), &path)
        };

        match result {
            Ok(()) => {
                self.as_mut().set_error_message(QString::default());
                true
            }

            Err(error) => {
                self.as_mut().set_error_message(QString::from(error));
                false
            }
        }
    }

    fn ensure_personal_project(mut self: Pin<&mut Self>) -> bool {
        self.as_mut().set_error_message(QString::default());

        match open_or_create_personal_project() {
            Ok(_) => true,
            Err(error) => {
                self.as_mut().set_error_message(QString::from(error));
                false
            }
        }
    }

    fn load_game(mut self: Pin<&mut Self>, project_path: &QString, game_id: i64) -> bool {
        let path = project_path.to_string();

        let played_document = {
            let self_ref = self.as_ref();
            let rust = self_ref.rust();

            rust.loaded_document
                .as_ref()
                .filter(|document| document.playable)
                .cloned()
        };

        if let Some(document) = played_document {
            self.as_mut().rust_mut().played_game_document = Some(document);
        }

        let document = match load_game_document(&path, game_id) {
            Ok(document) => document,

            Err(error) => {
                self.as_mut().rust_mut().loaded_document = None;

                self.as_mut().reset_position_display();
                self.as_mut().set_error_message(QString::from(error));

                return false;
            }
        };

        self.as_mut().set_black_player(QString::from(
            document.black_player.clone().unwrap_or_default(),
        ));
        self.as_mut().set_white_player(QString::from(
            document.white_player.clone().unwrap_or_default(),
        ));
        self.as_mut().set_komi(QString::from(
            document
                .komi
                .map(|value| value.to_string())
                .unwrap_or_default(),
        ));
        self.as_mut().rust_mut().loaded_document = Some(document);

        self.as_mut().show_cached_position(0)
    }

    fn load_sgf(mut self: Pin<&mut Self>, sgf_path: &QString) -> bool {
        let path = sgf_path.to_string();

        let played_document = {
            let self_ref = self.as_ref();
            let rust = self_ref.rust();

            rust.loaded_document
                .as_ref()
                .filter(|document| document.playable)
                .cloned()
        };

        if let Some(document) = played_document {
            self.as_mut().rust_mut().played_game_document = Some(document);
        }

        let document = match load_sgf_document(&path) {
            Ok(document) => document,

            Err(error) => {
                self.as_mut().rust_mut().loaded_document = None;

                self.as_mut().reset_position_display();
                self.as_mut().set_error_message(QString::from(error));

                return false;
            }
        };

        self.as_mut().set_black_player(QString::from(
            document.black_player.clone().unwrap_or_default(),
        ));
        self.as_mut().set_white_player(QString::from(
            document.white_player.clone().unwrap_or_default(),
        ));
        self.as_mut().set_komi(QString::from(
            document
                .komi
                .map(|value| value.to_string())
                .unwrap_or_default(),
        ));
        self.as_mut().rust_mut().loaded_document = Some(document);

        self.as_mut().show_cached_position(0)
    }

    fn load_joseki_study(
        mut self: Pin<&mut Self>,
        sgf_path: &QString,
        node_id: &QString,
        current_move_count: i32,
    ) -> bool {
        let path = sgf_path.to_string();
        let node_id = node_id.to_string();

        let played_document = {
            let self_ref = self.as_ref();
            let rust = self_ref.rust();

            rust.loaded_document
                .as_ref()
                .filter(|document| document.playable)
                .cloned()
        };

        if let Some(document) = played_document {
            self.as_mut().rust_mut().played_game_document = Some(document);
        }

        let document = match load_joseki_study_document(&path, &node_id) {
            Ok(document) => document,

            Err(error) => {
                self.as_mut().rust_mut().loaded_document = None;
                self.as_mut().reset_position_display();
                self.as_mut().set_error_message(QString::from(error));
                return false;
            }
        };

        let requested_move = match usize::try_from(current_move_count) {
            Ok(move_number) if move_number < document.positions.len() => current_move_count,

            _ => {
                self.as_mut().rust_mut().loaded_document = None;
                self.as_mut().reset_position_display();
                self.as_mut().set_error_message(QString::from(format!(
                    "OGS joseki position {node_id} has no move {current_move_count}"
                )));
                return false;
            }
        };

        self.as_mut().set_black_player(QString::from(
            document.black_player.clone().unwrap_or_default(),
        ));
        self.as_mut().set_white_player(QString::from(
            document.white_player.clone().unwrap_or_default(),
        ));
        self.as_mut().set_komi(QString::from(
            document
                .komi
                .map(|value| value.to_string())
                .unwrap_or_default(),
        ));

        self.as_mut().rust_mut().loaded_document = Some(document);

        self.as_mut().show_cached_position(requested_move)
    }

    fn save_played_game_sgf(mut self: Pin<&mut Self>, sgf_path: &QString) -> bool {
        self.as_mut().set_error_message(QString::default());

        let sgf_path = sgf_path.to_string();

        let result = {
            let self_ref = self.as_ref();
            let rust = self_ref.rust();

            match rust.loaded_document.as_ref() {
                Some(document) => save_played_document_sgf(document, &sgf_path),

                None => Err("no game is being played".to_owned()),
            }
        };

        match result {
            Ok(()) => true,

            Err(error) => {
                self.as_mut().set_error_message(QString::from(error));

                false
            }
        }
    }

    fn add_played_game_to_my_games(mut self: Pin<&mut Self>) -> bool {
        self.as_mut().set_error_message(QString::default());

        let result = {
            let self_ref = self.as_ref();
            let rust = self_ref.rust();

            match rust.loaded_document.as_ref() {
                Some(document) => add_played_document_to_my_games(document),

                None => Err("no game is being played".to_owned()),
            }
        };

        match result {
            Ok(_) => true,

            Err(error) => {
                self.as_mut().set_error_message(QString::from(error));

                false
            }
        }
    }

    fn restore_played_game(mut self: Pin<&mut Self>) -> bool {
        self.as_mut().set_error_message(QString::default());

        /*
         * If the playable document is still current, use it directly.
         * Otherwise restore the copy retained before another document
         * replaced the board view.
         */
        let document = {
            let self_ref = self.as_ref();
            let rust = self_ref.rust();

            rust.loaded_document
                .as_ref()
                .filter(|document| document.playable)
                .cloned()
                .or_else(|| rust.played_game_document.clone())
        };

        let document = match document {
            Some(document) => document,

            None => {
                self.as_mut()
                    .set_error_message(QString::from("no played game is available"));
                return false;
            }
        };

        let final_move =
            i32::try_from(document.positions.len().saturating_sub(1)).unwrap_or(i32::MAX);

        self.as_mut().set_black_player(QString::from(
            document.black_player.clone().unwrap_or_default(),
        ));

        self.as_mut().set_white_player(QString::from(
            document.white_player.clone().unwrap_or_default(),
        ));

        self.as_mut().set_komi(QString::from(
            document
                .komi
                .map(|value| value.to_string())
                .unwrap_or_default(),
        ));

        self.as_mut().rust_mut().played_game_document = Some(document.clone());

        self.as_mut().rust_mut().loaded_document = Some(document);

        self.as_mut().show_cached_position(final_move)
    }

    fn discard_played_game(mut self: Pin<&mut Self>) -> bool {
        self.as_mut().set_error_message(QString::default());

        /*
         * Forget the retained Play Game session. If the same game is still
         * being viewed for study, keep that position viewable but no longer
         * treat it as a playable session.
         */
        {
            let mut rust = self.as_mut().rust_mut();

            rust.played_game_document = None;

            if let Some(document) = rust.loaded_document.as_mut() {
                if document.playable {
                    document.playable = false;
                }
            }

            if let Some(snapshot) = rust.search_source_snapshot.as_mut() {
                if snapshot.document.playable {
                    snapshot.document.playable = false;
                }
            }

            if let Some(snapshot) = rust.workspace_snapshot.as_mut() {
                if let Some(document) = snapshot.document.as_mut() {
                    if document.playable {
                        document.playable = false;
                    }
                }

                if let Some(search_source) = snapshot.search_source_snapshot.as_mut() {
                    if search_source.document.playable {
                        search_source.document.playable = false;
                    }
                }
            }
        }

        true
    }

    fn remove_game_from_my_games(mut self: Pin<&mut Self>, game_source_id: i64) -> bool {
        self.as_mut().set_error_message(QString::default());

        let result = (|| -> Result<(), String> {
            let root = personal_project_root()?;

            let project = ProjectManager::new()
                .open(&root)
                .map_err(|error| format!("opening My Games at {}: {error}", root.display()))?;

            let mut store = project
                .game_store()
                .map_err(|error| format!("opening My Games game store: {error}"))?;

            store.remove_source(game_source_id).map_err(|error| {
                format!("removing game occurrence {game_source_id} from My Games: {error}")
            })
        })();

        match result {
            Ok(()) => {
                self.as_mut().rust_mut().loaded_document = None;
                self.as_mut().reset_position_display();
                true
            }

            Err(error) => {
                self.as_mut().set_error_message(QString::from(error));
                false
            }
        }
    }

    fn new_position(mut self: Pin<&mut Self>, board_size: i32) -> bool {
        let played_document = {
            let self_ref = self.as_ref();
            let rust = self_ref.rust();

            rust.loaded_document
                .as_ref()
                .filter(|document| document.playable)
                .cloned()
        };

        if let Some(document) = played_document {
            self.as_mut().rust_mut().played_game_document = Some(document);
        }

        let document = match new_position_document(board_size) {
            Ok(document) => document,

            Err(error) => {
                self.as_mut().rust_mut().loaded_document = None;

                self.as_mut().reset_position_display();
                self.as_mut().set_error_message(QString::from(error));

                return false;
            }
        };

        self.as_mut().set_black_player(QString::from(
            document.black_player.clone().unwrap_or_default(),
        ));
        self.as_mut().set_white_player(QString::from(
            document.white_player.clone().unwrap_or_default(),
        ));
        self.as_mut().set_komi(QString::from(
            document
                .komi
                .map(|value| value.to_string())
                .unwrap_or_default(),
        ));
        self.as_mut().rust_mut().loaded_document = Some(document);

        self.as_mut().show_cached_position(0)
    }

    fn new_game(
        mut self: Pin<&mut Self>,
        board_size: i32,
        black_player: &QString,
        white_player: &QString,
        komi: &QString,
    ) -> bool {
        self.as_mut().set_error_message(QString::default());

        let black_player = optional_player_name(&black_player.to_string());
        let white_player = optional_player_name(&white_player.to_string());

        let komi_text = komi.to_string();
        let komi_value = match komi_text.trim().parse::<f32>() {
            Ok(value) if value.is_finite() => value,

            _ => {
                self.as_mut()
                    .set_error_message(QString::from("komi must be a number, for example 6.5"));
                return false;
            }
        };

        let document = match new_game_document(board_size, black_player, white_player, komi_value) {
            Ok(document) => document,

            Err(error) => {
                self.as_mut().rust_mut().loaded_document = None;
                self.as_mut().reset_position_display();
                self.as_mut().set_error_message(QString::from(error));
                return false;
            }
        };

        self.as_mut().set_black_player(QString::from(
            document.black_player.clone().unwrap_or_default(),
        ));
        self.as_mut().set_white_player(QString::from(
            document.white_player.clone().unwrap_or_default(),
        ));
        self.as_mut().set_komi(QString::from(
            document
                .komi
                .map(|value| value.to_string())
                .unwrap_or_default(),
        ));

        self.as_mut().rust_mut().played_game_document = None;
        self.as_mut().rust_mut().loaded_document = Some(document);

        self.as_mut().show_cached_position(0)
    }

    fn play_game_point(mut self: Pin<&mut Self>, x: i32, y: i32) -> bool {
        self.as_mut().set_error_message(QString::default());

        let result = {
            let mut rust = self.as_mut().rust_mut();

            match rust.loaded_document.as_mut() {
                Some(document) => play_document_point(document, x, y),
                None => Err("no game is being played".to_owned()),
            }
        };

        match result {
            Ok(move_number) => self.as_mut().show_cached_position(move_number),

            Err(error) => {
                self.as_mut().set_error_message(QString::from(error));
                false
            }
        }
    }

    fn play_game_pass(mut self: Pin<&mut Self>) -> bool {
        self.as_mut().set_error_message(QString::default());

        let result = {
            let mut rust = self.as_mut().rust_mut();

            match rust.loaded_document.as_mut() {
                Some(document) => play_document_pass(document),
                None => Err("no game is being played".to_owned()),
            }
        };

        match result {
            Ok(move_number) => self.as_mut().show_cached_position(move_number),

            Err(error) => {
                self.as_mut().set_error_message(QString::from(error));
                false
            }
        }
    }

    fn undo_game_move(mut self: Pin<&mut Self>) -> bool {
        self.as_mut().set_error_message(QString::default());

        let result = {
            let mut rust = self.as_mut().rust_mut();

            match rust.loaded_document.as_mut() {
                Some(document) => undo_document_move(document),
                None => Err("no game is being played".to_owned()),
            }
        };

        match result {
            Ok(move_number) => self.as_mut().show_cached_position(move_number),

            Err(error) => {
                self.as_mut().set_error_message(QString::from(error));
                false
            }
        }
    }

    fn resign_game(mut self: Pin<&mut Self>) -> QString {
        self.as_mut().set_error_message(QString::default());

        let result = {
            let mut rust = self.as_mut().rust_mut();

            match rust.loaded_document.as_mut() {
                Some(document) => resign_document_game(document),
                None => Err("no game is being played".to_owned()),
            }
        };

        match result {
            Ok(result) => QString::from(result),

            Err(error) => {
                self.as_mut().set_error_message(QString::from(error));
                QString::default()
            }
        }
    }

    fn finish_game(mut self: Pin<&mut Self>, result: &QString) -> bool {
        self.as_mut().set_error_message(QString::default());

        let result_text = result.to_string();

        let result = {
            let mut rust = self.as_mut().rust_mut();

            match rust.loaded_document.as_mut() {
                Some(document) => finish_document_game(document, &result_text),

                None => Err("no game is being played".to_owned()),
            }
        };

        match result {
            Ok(()) => true,

            Err(error) => {
                self.as_mut().set_error_message(QString::from(error));
                false
            }
        }
    }

    fn snapshot_workspace(mut self: Pin<&mut Self>) -> bool {
        self.as_mut().set_error_message(QString::default());

        let snapshot = {
            let self_ref = self.as_ref();
            let rust = self_ref.rust();

            WorkspaceSnapshot {
                document: rust.loaded_document.clone(),
                move_number: rust.move_number,
                search_source_snapshot: rust.search_source_snapshot.clone(),
            }
        };

        self.as_mut().rust_mut().workspace_snapshot = Some(snapshot);
        true
    }

    fn swap_workspace(mut self: Pin<&mut Self>) -> bool {
        self.as_mut().set_error_message(QString::default());

        let parked = self.as_mut().rust_mut().workspace_snapshot.take();

        let Some(parked) = parked else {
            self.as_mut()
                .set_error_message(QString::from("no other workspace is available"));
            return false;
        };

        let active = {
            let mut rust = self.as_mut().rust_mut();
            let document = rust.loaded_document.take();

            if let Some(played) = document
                .as_ref()
                .filter(|document| document.playable)
                .cloned()
            {
                rust.played_game_document = Some(played);
            }

            let search_source_snapshot = rust.search_source_snapshot.take();

            WorkspaceSnapshot {
                document,
                move_number: rust.move_number,
                search_source_snapshot,
            }
        };

        self.as_mut().rust_mut().workspace_snapshot = Some(active);

        let WorkspaceSnapshot {
            document,
            move_number,
            search_source_snapshot,
        } = parked;

        self.as_mut().rust_mut().search_source_snapshot = search_source_snapshot;

        let Some(document) = document else {
            self.as_mut().rust_mut().loaded_document = None;
            self.as_mut().reset_position_display();
            return true;
        };

        self.as_mut().set_black_player(QString::from(
            document.black_player.clone().unwrap_or_default(),
        ));
        self.as_mut().set_white_player(QString::from(
            document.white_player.clone().unwrap_or_default(),
        ));
        self.as_mut().set_komi(QString::from(
            document
                .komi
                .map(|value| value.to_string())
                .unwrap_or_default(),
        ));

        self.as_mut().rust_mut().loaded_document = Some(document);
        self.as_mut().show_cached_position(move_number)
    }

    fn snapshot_search_source(mut self: Pin<&mut Self>) -> bool {
        self.as_mut().set_error_message(QString::default());

        let snapshot = {
            let self_ref = self.as_ref();
            let rust = self_ref.rust();

            rust.loaded_document
                .as_ref()
                .cloned()
                .map(|document| SearchSourceSnapshot {
                    document,
                    move_number: rust.move_number,
                })
        };

        match snapshot {
            Some(snapshot) => {
                self.as_mut().rust_mut().search_source_snapshot = Some(snapshot);
                true
            }
            None => {
                self.as_mut()
                    .set_error_message(QString::from("no position is loaded"));
                false
            }
        }
    }

    fn restore_search_source(mut self: Pin<&mut Self>) -> bool {
        self.as_mut().set_error_message(QString::default());

        let snapshot = self.as_mut().rust_mut().search_source_snapshot.take();

        let Some(snapshot) = snapshot else {
            self.as_mut()
                .set_error_message(QString::from("no search source is available"));
            return false;
        };

        let SearchSourceSnapshot {
            document,
            move_number,
        } = snapshot;

        self.as_mut().set_black_player(QString::from(
            document.black_player.clone().unwrap_or_default(),
        ));
        self.as_mut().set_white_player(QString::from(
            document.white_player.clone().unwrap_or_default(),
        ));
        self.as_mut().set_komi(QString::from(
            document
                .komi
                .map(|value| value.to_string())
                .unwrap_or_default(),
        ));

        self.as_mut().rust_mut().loaded_document = Some(document);

        self.as_mut().show_cached_position(move_number)
    }

    fn edit_position_point(mut self: Pin<&mut Self>, x: i32, y: i32, tool: &QString) -> bool {
        self.as_mut().set_error_message(QString::default());

        let tool = tool.to_string();

        let result = {
            let mut rust = self.as_mut().rust_mut();

            match rust.loaded_document.as_mut() {
                Some(document) => edit_document_position(document, x, y, &tool),

                None => Err("no position is loaded".to_owned()),
            }
        };

        match result {
            Ok(()) => self.as_mut().show_cached_position(0),

            Err(error) => {
                self.as_mut().set_error_message(QString::from(error));
                false
            }
        }
    }

    fn show_position(mut self: Pin<&mut Self>, move_number: i32) -> bool {
        let position_changed = self.as_ref().rust().move_number != move_number;

        if position_changed && self.as_ref().rust().katago_analysis_in_progress {
            /*
             * Analysis belongs to the position on which it was requested.
             * Navigation remains immediate: cancel obsolete work silently
             * and do not start analysis for the newly displayed position.
             */
            let _ = cancel_katago_analysis_impl(self.as_mut(), false);
        }

        self.as_mut().show_cached_position(move_number)
    }

    fn stone_move_number(self: Pin<&mut Self>, move_number: i32, x: i32, y: i32) -> i32 {
        let Ok(position_index) = usize::try_from(move_number) else {
            return -1;
        };

        let Ok(qml_x) = u8::try_from(x) else {
            return -1;
        };

        let Ok(qml_y) = u8::try_from(y) else {
            return -1;
        };

        let self_ref = self.as_ref();
        let rust = self_ref.rust();

        let Some(document) = rust.loaded_document.as_ref() else {
            return -1;
        };

        let Some(current_position) = document.positions.get(position_index) else {
            return -1;
        };

        let board_size = current_position.board.size();

        if qml_x >= board_size || qml_y >= board_size {
            return -1;
        }

        let Ok(core_y) = qml_y_to_core(board_size, qml_y) else {
            return -1;
        };

        let point = u16::from(core_y) * u16::from(board_size) + u16::from(qml_x);

        let Some(current_colour) = current_position.board.colour_at(point) else {
            return -1;
        };

        for position in document.positions[..=position_index].iter().rev() {
            let Some(mv) = position.last_move else {
                continue;
            };

            if mv.point == Some(point) && mv.colour == current_colour {
                return i32::try_from(position.occurrence.move_number).unwrap_or(-1);
            }
        }

        -1
    }

    fn study_board_markup_json(&self, move_number: i32) -> QString {
        let json = self
            .rust()
            .loaded_document
            .as_ref()
            .map(|document| render_study_board_markup_json(document, move_number))
            .unwrap_or_else(|| "[]".to_owned());

        QString::from(json)
    }

    fn set_study_annotation(
        mut self: Pin<&mut Self>,
        move_number: i32,
        x: i32,
        y: i32,
        tool: &QString,
        text: &QString,
    ) -> bool {
        self.as_mut().set_error_message(QString::default());

        let tool = tool.to_string();
        let text = text.to_string();

        let result = {
            let mut rust = self.as_mut().rust_mut();

            match rust.loaded_document.as_mut() {
                Some(document) => {
                    update_study_annotation(document, move_number, x, y, &tool, &text)
                }
                None => Err("no Study document is loaded".to_owned()),
            }
        };

        match result {
            Ok(()) => {
                let (can_undo, can_redo) = self
                    .as_ref()
                    .rust()
                    .loaded_document
                    .as_ref()
                    .map(|document| {
                        (
                            !document.study.history.undo.is_empty(),
                            !document.study.history.redo.is_empty(),
                        )
                    })
                    .unwrap_or((false, false));

                self.as_mut().set_can_undo_study_edit(can_undo);
                self.as_mut().set_can_redo_study_edit(can_redo);

                true
            }
            Err(error) => {
                self.as_mut().set_error_message(QString::from(error));
                false
            }
        }
    }

    fn undo_study_edit(mut self: Pin<&mut Self>) -> bool {
        self.as_mut().set_error_message(QString::default());

        let move_number = self.as_ref().rust().move_number;
        let structural_node = self.as_ref().rust().sgf_tree_current_node;

        let result = {
            let mut rust = self.as_mut().rust_mut();

            match rust.loaded_document.as_mut() {
                Some(document) => step_study_history(document, move_number, structural_node, true),
                None => Err("no SGF is loaded".to_owned()),
            }
        };

        match result {
            Ok((display_move, structure_node)) => {
                let shown = self.as_mut().show_cached_position(display_move);

                if shown {
                    self.as_mut().set_sgf_tree_current_node(structure_node);
                }

                shown
            }

            Err(error) => {
                self.as_mut().set_error_message(QString::from(error));
                false
            }
        }
    }

    fn redo_study_edit(mut self: Pin<&mut Self>) -> bool {
        self.as_mut().set_error_message(QString::default());

        let move_number = self.as_ref().rust().move_number;
        let structural_node = self.as_ref().rust().sgf_tree_current_node;

        let result = {
            let mut rust = self.as_mut().rust_mut();

            match rust.loaded_document.as_mut() {
                Some(document) => step_study_history(document, move_number, structural_node, false),
                None => Err("no SGF is loaded".to_owned()),
            }
        };

        match result {
            Ok((display_move, structure_node)) => {
                let shown = self.as_mut().show_cached_position(display_move);

                if shown {
                    self.as_mut().set_sgf_tree_current_node(structure_node);
                }

                shown
            }

            Err(error) => {
                self.as_mut().set_error_message(QString::from(error));
                false
            }
        }
    }

    fn set_study_comment(mut self: Pin<&mut Self>, move_number: i32, text: &QString) -> bool {
        self.as_mut().set_error_message(QString::default());

        let text = text.to_string();

        let result = {
            let mut rust = self.as_mut().rust_mut();

            match rust.loaded_document.as_mut() {
                Some(document) => update_study_comment(document, move_number, &text),
                None => Err("no Study document is loaded".to_owned()),
            }
        };

        match result {
            Ok(display_move) => self.as_mut().show_cached_position(display_move),

            Err(error) => {
                self.as_mut().set_error_message(QString::from(error));
                false
            }
        }
    }

    fn delete_study_from_here(mut self: Pin<&mut Self>) -> bool {
        self.as_mut().set_error_message(QString::default());

        let slider_move_number = self.as_ref().rust().move_number;
        let selected_structure_node = self.as_ref().rust().sgf_tree_current_node;

        let result = {
            let mut rust = self.as_mut().rust_mut();

            match rust.loaded_document.as_mut() {
                Some(document) => update_study_delete_from_here(
                    document,
                    slider_move_number,
                    selected_structure_node,
                ),
                None => Err("no SGF is loaded".to_owned()),
            }
        };

        match result {
            Ok((display_move, structure_node)) => {
                let shown = self.as_mut().show_cached_position(display_move);

                if shown {
                    self.as_mut().set_sgf_tree_current_node(structure_node);
                }

                shown
            }

            Err(error) => {
                self.as_mut().set_error_message(QString::from(error));
                false
            }
        }
    }

    fn insert_study_node(mut self: Pin<&mut Self>) -> bool {
        self.as_mut().set_error_message(QString::default());

        let slider_move_number = self.as_ref().rust().move_number;
        let selected_structure_node = self.as_ref().rust().sgf_tree_current_node;

        let result = {
            let mut rust = self.as_mut().rust_mut();

            match rust.loaded_document.as_mut() {
                Some(document) => update_study_node_insertion(
                    document,
                    slider_move_number,
                    selected_structure_node,
                ),
                None => Err("no SGF is loaded".to_owned()),
            }
        };

        match result {
            Ok((display_move, structure_node)) => {
                let shown = self.as_mut().show_cached_position(display_move);

                if shown {
                    self.as_mut().set_sgf_tree_current_node(structure_node);
                }

                shown
            }

            Err(error) => {
                self.as_mut().set_error_message(QString::from(error));
                false
            }
        }
    }

    fn add_study_move(mut self: Pin<&mut Self>, x: i32, y: i32) -> bool {
        self.as_mut().set_error_message(QString::default());

        let move_number = self.as_ref().rust().move_number;

        let result = {
            let mut rust = self.as_mut().rust_mut();

            match rust.loaded_document.as_mut() {
                Some(document) => update_study_move(document, move_number, Some((x, y))),
                None => Err("no Study document is loaded".to_owned()),
            }
        };

        match result {
            Ok(display_move) => self.as_mut().show_cached_position(display_move),

            Err(error) => {
                self.as_mut().set_error_message(QString::from(error));
                false
            }
        }
    }

    fn study_branch_available(self: Pin<&mut Self>, move_number: i32) -> bool {
        let position_index = match usize::try_from(move_number) {
            Ok(position_index) => position_index,
            Err(_) => return false,
        };

        let app = self.as_ref();
        let rust = app.rust();
        let Some(document) = rust.loaded_document.as_ref() else {
            return false;
        };

        if let Some(tree) = document.study_tree.as_ref() {
            return study_tree_has_continuation(tree, position_index);
        }

        let collection = match study_collection(document) {
            Ok(collection) => collection,
            Err(_) => return false,
        };

        let tree = match build_study_tree(&collection) {
            Ok(tree) => tree,
            Err(_) => return false,
        };

        study_tree_has_continuation(&tree, position_index)
    }

    fn add_study_branch(mut self: Pin<&mut Self>, x: i32, y: i32) -> bool {
        self.as_mut().set_error_message(QString::default());

        let move_number = self.as_ref().rust().move_number;

        let result = {
            let mut rust = self.as_mut().rust_mut();

            match rust.loaded_document.as_mut() {
                Some(document) => update_study_branch(document, move_number, Some((x, y))),
                None => Err("no Study document is loaded".to_owned()),
            }
        };

        match result {
            Ok(display_move) => self.as_mut().show_cached_position(display_move),

            Err(error) => {
                self.as_mut().set_error_message(QString::from(error));
                false
            }
        }
    }

    fn add_study_pass(mut self: Pin<&mut Self>) -> bool {
        self.as_mut().set_error_message(QString::default());

        let move_number = self.as_ref().rust().move_number;

        let result = {
            let mut rust = self.as_mut().rust_mut();

            match rust.loaded_document.as_mut() {
                Some(document) => update_study_move(document, move_number, None),
                None => Err("no Study document is loaded".to_owned()),
            }
        };

        match result {
            Ok(display_move) => self.as_mut().show_cached_position(display_move),

            Err(error) => {
                self.as_mut().set_error_message(QString::from(error));
                false
            }
        }
    }

    fn add_study_branch_pass(mut self: Pin<&mut Self>) -> bool {
        self.as_mut().set_error_message(QString::default());

        let move_number = self.as_ref().rust().move_number;

        let result = {
            let mut rust = self.as_mut().rust_mut();

            match rust.loaded_document.as_mut() {
                Some(document) => update_study_branch(document, move_number, None),
                None => Err("no Study document is loaded".to_owned()),
            }
        };

        match result {
            Ok(display_move) => self.as_mut().show_cached_position(display_move),

            Err(error) => {
                self.as_mut().set_error_message(QString::from(error));
                false
            }
        }
    }

    fn show_study_structure_node(mut self: Pin<&mut Self>, node_id: i32) -> bool {
        self.as_mut().set_error_message(QString::default());

        let node_id = match usize::try_from(node_id) {
            Ok(node_id) => node_id,

            Err(_) => {
                self.as_mut()
                    .set_error_message(QString::from("invalid structural SGF node"));
                return false;
            }
        };

        let requested_tree_node = i32::try_from(node_id).unwrap_or(i32::MAX);

        if self.as_ref().rust().sgf_tree_current_node != requested_tree_node
            && self.as_ref().rust().katago_analysis_in_progress
        {
            let _ = cancel_katago_analysis_impl(self.as_mut(), false);
        }

        let result = {
            let mut rust = self.as_mut().rust_mut();

            match rust.loaded_document.as_mut() {
                Some(document) => activate_study_structure_node(document, node_id),
                None => Err("no SGF is loaded".to_owned()),
            }
        };

        match result {
            Ok(move_number) => {
                let shown = self.as_mut().show_cached_position(move_number);

                if shown {
                    self.as_mut().set_sgf_tree_current_node(requested_tree_node);
                }

                shown
            }

            Err(error) => {
                self.as_mut().set_error_message(QString::from(error));
                false
            }
        }
    }

    fn show_sgf_node(mut self: Pin<&mut Self>, node_id: i32) -> bool {
        self.as_mut().set_error_message(QString::default());

        let node_id = match usize::try_from(node_id) {
            Ok(node_id) => node_id,

            Err(_) => {
                self.as_mut()
                    .set_error_message(QString::from("invalid SGF tree node"));
                return false;
            }
        };

        let current_tree_node = self.as_ref().rust().sgf_tree_current_node;
        let requested_tree_node = i32::try_from(node_id).unwrap_or(i32::MAX);

        if current_tree_node != requested_tree_node
            && self.as_ref().rust().katago_analysis_in_progress
        {
            let _ = cancel_katago_analysis_impl(self.as_mut(), false);
        }

        let result = {
            let mut rust = self.as_mut().rust_mut();

            match rust.loaded_document.as_mut() {
                Some(document) => activate_study_tree_node(document, node_id),
                None => Err("no SGF is loaded".to_owned()),
            }
        };

        match result {
            Ok(move_number) => self.as_mut().show_cached_position(move_number),

            Err(error) => {
                self.as_mut().set_error_message(QString::from(error));
                false
            }
        }
    }

    fn analyse_current_position(
        mut self: Pin<&mut Self>,
        visit_budget: i32,
        analysis_komi: &QString,
        saved_executable: &QString,
        saved_model: &QString,
        saved_config: &QString,
    ) -> bool {
        self.as_mut().set_error_message(QString::default());

        self.as_mut().set_katago_analysis_text(QString::default());
        self.as_mut()
            .set_katago_candidate_points_json(QString::from("[]"));

        let analysis_id = {
            let mut rust = self.as_mut().rust_mut();

            rust.katago_analysis_id = rust.katago_analysis_id.wrapping_add(1);

            rust.katago_analysis_id
        };

        let work: Result<KataGoWorkerRequest, String> = (|| {
            if !(10..=100_000).contains(&visit_budget) {
                return Err("KataGo visit budget must be between 10 and 100000".to_owned());
            }

            let visit_budget = u64::try_from(visit_budget)
                .map_err(|_| "invalid KataGo visit budget".to_owned())?;

            /*
             * Analysis komi is deliberately separate from game metadata.
             * The GUI normally starts with the recorded komi, but can
             * supply an explicit assumption when the source has none or
             * when the user wants to investigate another komi.
             */
            let komi_text = analysis_komi.to_string();
            let komi = komi_text
                .trim()
                .parse::<f64>()
                .map_err(|_| "KataGo analysis komi must be a number, for example 6.5".to_owned())?;

            if !komi.is_finite() {
                return Err("KataGo analysis komi must be finite".to_owned());
            }

            let (position, move_number) = {
                let self_ref = self.as_ref();
                let rust = self_ref.rust();

                let document = rust
                    .loaded_document
                    .as_ref()
                    .ok_or_else(|| "no game is loaded".to_owned())?;

                let move_number = usize::try_from(rust.move_number)
                    .map_err(|_| "move number cannot be negative".to_owned())?;

                let position =
                    analysis_position_from_states(
                        &document.positions,
                        move_number,
                    )
                    .map_err(|error| {
                        format!(
                            "preparing the displayed position                              for KataGo: {error}"
                        )
                    })?;

                (position, move_number)
            };

            let executable = katago_executable_path(saved_executable)?;

            let model = configured_katago_file("BERMUDA_KATAGO_MODEL", saved_model, "model")?;

            let config =
                configured_katago_file("BERMUDA_KATAGO_CONFIG", saved_config, "configuration")?;

            let working_directory = katago_working_directory()?;

            fs::create_dir_all(&working_directory).map_err(|error| {
                format!(
                    "creating KataGo working directory {}:                          {error}",
                    working_directory.display()
                )
            })?;

            let configuration =
                KataGoConfiguration::new(executable, model, config, &working_directory);

            let expected_player = position.current_player;

            let board_size = position.board_size;

            let request = AnalysisRequest {
                id: format!(
                    "bermuda-gui-analysis-{analysis_id}-                     position-{move_number}"
                ),
                board_size,
                rules: "japanese".to_owned(),
                komi,
                initial_stones: position.initial_stones,
                initial_player: position.initial_player,
                moves: position.moves,
                max_visits: visit_budget,
            };

            Ok(KataGoWorkerRequest {
                analysis_id,
                configuration,
                request,
                expected_player,
            })
        })();

        let work = match work {
            Ok(work) => work,

            Err(error) => {
                self.as_mut()
                    .set_error_message(QString::from(error.clone()));

                self.as_mut()
                    .set_katago_analysis_text(QString::from(format!("Analysis failed: {error}")));

                return false;
            }
        };

        if self.as_ref().rust().katago_worker.is_none() {
            let qt_thread = self.qt_thread();

            let worker = start_katago_worker(qt_thread);

            self.as_mut().rust_mut().katago_worker = Some(worker);
        }

        let send_result = self
            .as_ref()
            .rust()
            .katago_worker
            .as_ref()
            .expect("KataGo worker was created above")
            .sender
            .send(KataGoWorkerCommand::Analyse(work));

        match send_result {
            Ok(()) => {
                self.as_mut().set_katago_analysis_in_progress(true);

                true
            }

            Err(error) => {
                self.as_mut().rust_mut().katago_worker = None;

                let message = format!("sending analysis to KataGo worker: {error}");

                self.as_mut()
                    .set_error_message(QString::from(message.clone()));

                self.as_mut()
                    .set_katago_analysis_text(QString::from(format!("Analysis failed: {message}")));

                false
            }
        }
    }

    fn cancel_katago_analysis(self: Pin<&mut Self>) -> bool {
        cancel_katago_analysis_impl(self, true)
    }

    fn hypothetical_move_stones(
        mut self: Pin<&mut Self>,
        move_number: i32,
        x: i32,
        y: i32,
        colour: &QString,
    ) -> QString {
        self.as_mut().set_error_message(QString::default());

        let result = {
            let self_ref = self.as_ref();
            let rust = self_ref.rust();

            let document = match rust.loaded_document.as_ref() {
                Some(document) => document,
                None => {
                    self.as_mut()
                        .set_error_message(QString::from("no game is loaded"));
                    return QString::default();
                }
            };

            let requested_move = match usize::try_from(move_number) {
                Ok(value) => value,
                Err(_) => {
                    self.as_mut()
                        .set_error_message(QString::from("move number cannot be negative"));
                    return QString::default();
                }
            };

            let position = match document.positions.get(requested_move) {
                Some(position) => position,
                None => {
                    self.as_mut().set_error_message(QString::from(format!(
                        "requested move {move_number} is outside the loaded game"
                    )));
                    return QString::default();
                }
            };

            let board_size = position.board.size();

            let qml_x = match u8::try_from(x) {
                Ok(value) if value < board_size => value,
                _ => {
                    self.as_mut()
                        .set_error_message(QString::from("x-coordinate lies outside the board"));
                    return QString::default();
                }
            };

            let qml_y = match u8::try_from(y) {
                Ok(value) if value < board_size => value,
                _ => {
                    self.as_mut()
                        .set_error_message(QString::from("y-coordinate lies outside the board"));
                    return QString::default();
                }
            };

            let core_y = match qml_y_to_core(board_size, qml_y) {
                Ok(value) => value,
                Err(error) => {
                    self.as_mut().set_error_message(QString::from(error));
                    return QString::default();
                }
            };

            let colour = match colour.to_string().as_str() {
                "black" => Colour::Black,
                "white" => Colour::White,
                other => {
                    self.as_mut().set_error_message(QString::from(format!(
                        "unknown hypothetical move colour {other:?}"
                    )));
                    return QString::default();
                }
            };

            let point = match position.board.point(qml_x, core_y) {
                Ok(point) => point,
                Err(error) => {
                    self.as_mut()
                        .set_error_message(QString::from(error.to_string()));
                    return QString::default();
                }
            };

            let mut board = position.board.clone();

            match board.play(Move {
                colour,
                point: Some(point),
            }) {
                Ok(_) => Ok(board_stones_json(&board)),
                Err(error) => Err(error.to_string()),
            }
        };

        match result {
            Ok(stones) => stones,

            Err(error) => {
                self.as_mut().set_error_message(QString::from(error));
                QString::default()
            }
        }
    }

    fn hypothetical_sequence_stones(
        mut self: Pin<&mut Self>,
        move_number: i32,
        first_x: i32,
        first_y: i32,
        first_colour: &QString,
        second_x: i32,
        second_y: i32,
        second_colour: &QString,
    ) -> QString {
        self.as_mut().set_error_message(QString::default());

        fn parse_colour(value: &QString) -> Result<Colour, String> {
            match value.to_string().as_str() {
                "black" => Ok(Colour::Black),
                "white" => Ok(Colour::White),
                other => Err(format!("unknown hypothetical move colour {other:?}")),
            }
        }

        fn qml_point(board: &Board, x: i32, y: i32) -> Result<u16, String> {
            let board_size = board.size();

            let qml_x =
                u8::try_from(x).map_err(|_| "x-coordinate lies outside the board".to_owned())?;

            if qml_x >= board_size {
                return Err("x-coordinate lies outside the board".to_owned());
            }

            let qml_y =
                u8::try_from(y).map_err(|_| "y-coordinate lies outside the board".to_owned())?;

            if qml_y >= board_size {
                return Err("y-coordinate lies outside the board".to_owned());
            }

            let core_y = qml_y_to_core(board_size, qml_y)?;

            board
                .point(qml_x, core_y)
                .map_err(|error| error.to_string())
        }

        let result: Result<QString, String> = (|| {
            let self_ref = self.as_ref();
            let rust = self_ref.rust();

            let document = rust
                .loaded_document
                .as_ref()
                .ok_or_else(|| "no game is loaded".to_owned())?;

            let requested_move = usize::try_from(move_number)
                .map_err(|_| "move number cannot be negative".to_owned())?;

            let position =
                document.positions.get(requested_move)
                    .ok_or_else(|| {
                        format!(
                            "requested move {move_number}                              is outside the loaded game"
                        )
                    })?;

            let first_colour = parse_colour(first_colour)?;

            let second_colour = parse_colour(second_colour)?;

            let first_point = qml_point(&position.board, first_x, first_y)?;

            let mut board = position.board.clone();

            board
                .play(Move {
                    colour: first_colour,
                    point: Some(first_point),
                })
                .map_err(|error| format!("first hypothetical move: {error}"))?;

            let second_point = qml_point(&board, second_x, second_y)?;

            board
                .play(Move {
                    colour: second_colour,
                    point: Some(second_point),
                })
                .map_err(|error| format!("second hypothetical move: {error}"))?;

            Ok(board_stones_json(&board))
        })();

        match result {
            Ok(stones) => stones,

            Err(error) => {
                self.as_mut().set_error_message(QString::from(error));

                QString::default()
            }
        }
    }

    fn show_cached_position(mut self: Pin<&mut Self>, move_number: i32) -> bool {
        self.as_mut().set_error_message(QString::default());

        let result: Result<(LoadedPosition, String, i32, String), String> = (|| {
            let self_ref = self.as_ref();
            let rust = self_ref.rust();

            let document = rust
                .loaded_document
                .as_ref()
                .ok_or_else(|| "no game is loaded".to_owned())?;

            let position = position_data(
                &document.positions,
                &document.source_comments,
                &document.description,
                move_number,
            )?;

            let current_tree_node = document
                .study_tree
                .as_ref()
                .and_then(|tree| {
                    usize::try_from(move_number)
                        .ok()
                        .and_then(|index| tree.active_path.get(index))
                })
                .and_then(|node_id| i32::try_from(*node_id).ok())
                .unwrap_or(-1);

            let markup_json = study_markup_json(document.study_tree.as_ref(), current_tree_node);

            let (tree_json, current_tree_node) =
                study_tree_presentation(document, current_tree_node);

            Ok((position, tree_json, current_tree_node, markup_json))
        })();

        match result {
            Ok((position, tree_json, current_tree_node, markup_json)) => {
                self.as_mut().set_board_size(position.board_size);
                self.as_mut().set_stones_json(position.stones_json);
                self.as_mut().set_move_number(position.move_number);
                self.as_mut().set_move_count(position.move_count);
                self.as_mut().set_last_move_x(position.last_move_x);
                self.as_mut().set_last_move_y(position.last_move_y);
                self.as_mut().set_source_comment(position.source_comment);
                self.as_mut()
                    .set_has_source_comments(position.has_source_comments);

                self.as_mut().set_sgf_tree_json(QString::from(tree_json));
                self.as_mut().set_sgf_tree_current_node(current_tree_node);
                self.as_mut()
                    .set_board_markup_json(QString::from(markup_json));
                let (can_undo, can_redo) = self
                    .as_ref()
                    .rust()
                    .loaded_document
                    .as_ref()
                    .map(|document| {
                        (
                            !document.study.history.undo.is_empty(),
                            !document.study.history.redo.is_empty(),
                        )
                    })
                    .unwrap_or((false, false));

                self.as_mut().set_can_undo_study_edit(can_undo);
                self.as_mut().set_can_redo_study_edit(can_redo);

                true
            }

            Err(error) => {
                self.as_mut().set_error_message(QString::from(error));
                false
            }
        }
    }

    fn reset_position_display(mut self: Pin<&mut Self>) {
        self.as_mut().set_board_size(19);
        self.as_mut().set_stones_json(QString::from("[]"));
        self.as_mut().set_move_number(0);
        self.as_mut().set_move_count(0);
        self.as_mut().set_last_move_x(-1);
        self.as_mut().set_last_move_y(-1);
        self.as_mut().set_black_player(QString::default());
        self.as_mut().set_white_player(QString::default());
        self.as_mut().set_komi(QString::default());
        self.as_mut().set_source_comment(QString::default());
        self.as_mut().set_has_source_comments(false);
        self.as_mut().set_sgf_tree_json(QString::from("[]"));
        self.as_mut().set_sgf_tree_current_node(-1);
        self.as_mut().set_board_markup_json(QString::from("[]"));
        self.as_mut().set_can_undo_study_edit(false);
        self.as_mut().set_can_redo_study_edit(false);
    }
}

struct LoadedPosition {
    board_size: i32,
    stones_json: QString,
    source_comment: QString,
    has_source_comments: bool,
    move_number: i32,
    move_count: i32,
    last_move_x: i32,
    last_move_y: i32,
}

fn study_replay_tree_json(tree: Option<&StudyTree>) -> String {
    let Some(tree) = tree else {
        return "[]".to_owned();
    };

    let nodes = tree
        .nodes
        .iter()
        .map(|node| {
            let colour = match node.move_colour {
                Some(Colour::Black) => "black",
                Some(Colour::White) => "white",
                None => "root",
            };

            serde_json::json!({
                "id": node.id,
                "parent": node.parent,
                "row": node.row,
                "lane": node.lane,
                "moveNumber": node.position.occurrence.move_number,
                "colour": colour,
                "isMove": node.move_colour.is_some(),
                "isEmpty": false,
                "root": node.parent.is_none(),
                "hasComment": !node.comment.is_empty(),
                "branchPoint": node.children.len() > 1,
            })
        })
        .collect::<Vec<_>>();

    serde_json::to_string(&nodes).unwrap_or_else(|_| "[]".to_owned())
}

fn study_structure_tree_json(tree: &bermuda::StudyStructureTree) -> String {
    let nodes = tree
        .nodes
        .iter()
        .map(|node| {
            let colour = match node.move_colour {
                Some(Colour::Black) => "black",
                Some(Colour::White) => "white",
                None => "node",
            };

            serde_json::json!({
                "id": node.id,
                "parent": node.parent,
                "row": node.row,
                "lane": node.lane,
                "moveNumber": node.move_number,
                "colour": colour,
                "isMove": node.move_colour.is_some(),
                "isEmpty": node.is_empty,
                "root": node.parent.is_none(),
                "hasComment": node.has_comment,
                "branchPoint": node.children.len() > 1,
            })
        })
        .collect::<Vec<_>>();

    serde_json::to_string(&nodes).unwrap_or_else(|_| "[]".to_owned())
}

fn study_tree_presentation(document: &LoadedDocument, current_replay_node: i32) -> (String, i32) {
    let Some(collection) = document.study.collection.as_ref() else {
        return (
            study_replay_tree_json(document.study_tree.as_ref()),
            current_replay_node,
        );
    };

    let Ok(structure) = bermuda::build_study_structure_tree(collection) else {
        return (
            study_replay_tree_json(document.study_tree.as_ref()),
            current_replay_node,
        );
    };

    let current_structure_node = usize::try_from(current_replay_node)
        .ok()
        .and_then(|replay_id| document.study_tree.as_ref()?.nodes.get(replay_id))
        .and_then(|replay_node| structure.node_id_for_source(&replay_node.source))
        .and_then(|id| i32::try_from(id).ok())
        .unwrap_or(-1);

    (
        study_structure_tree_json(&structure),
        current_structure_node,
    )
}

fn study_markup_json(tree: Option<&StudyTree>, node_id: i32) -> String {
    let Some(tree) = tree else {
        return "[]".to_owned();
    };

    let Ok(node_id) = usize::try_from(node_id) else {
        return "[]".to_owned();
    };

    let Some(node) = tree.nodes.get(node_id) else {
        return "[]".to_owned();
    };

    let size = u16::from(node.position.board.size());

    let marks = node
        .markup
        .iter()
        .map(|mark| {
            let x = mark.point % size;
            let core_y = mark.point / size;
            let y = core_y_to_qml(size, core_y);

            match &mark.kind {
                StudyMarkupKind::Label(text) => serde_json::json!({
                    "type": "label",
                    "x": x,
                    "y": y,
                    "text": text,
                }),
                StudyMarkupKind::Triangle => serde_json::json!({
                    "type": "triangle",
                    "x": x,
                    "y": y,
                }),
                StudyMarkupKind::Square => serde_json::json!({
                    "type": "square",
                    "x": x,
                    "y": y,
                }),
                StudyMarkupKind::Circle => serde_json::json!({
                    "type": "circle",
                    "x": x,
                    "y": y,
                }),
                StudyMarkupKind::Cross => serde_json::json!({
                    "type": "cross",
                    "x": x,
                    "y": y,
                }),
                StudyMarkupKind::Selected => serde_json::json!({
                    "type": "selected",
                    "x": x,
                    "y": y,
                }),
            }
        })
        .collect::<Vec<_>>();

    serde_json::to_string(&marks).unwrap_or_else(|_| "[]".to_owned())
}

fn replay_node_for_structure(
    structure: &bermuda::StudyStructureTree,
    replay: &StudyTree,
    structure_node_id: usize,
) -> Option<usize> {
    let selected = structure.nodes.get(structure_node_id)?;

    if let Some(replay_id) = replay.node_id_for_source(&selected.source) {
        return Some(replay_id);
    }

    let mut queue = std::collections::VecDeque::new();
    queue.extend(selected.children.iter().copied());

    while let Some(candidate) = queue.pop_front() {
        let node = structure.nodes.get(candidate)?;

        if let Some(replay_id) = replay.node_id_for_source(&node.source) {
            return Some(replay_id);
        }

        queue.extend(node.children.iter().copied());
    }

    let mut parent = selected.parent;

    while let Some(candidate) = parent {
        let node = structure.nodes.get(candidate)?;

        if let Some(replay_id) = replay.node_id_for_source(&node.source) {
            return Some(replay_id);
        }

        parent = node.parent;
    }

    None
}

fn activate_study_structure_node(
    document: &mut LoadedDocument,
    structure_node_id: usize,
) -> Result<i32, String> {
    let (replay_node_id, display_move) = {
        let collection = document
            .study
            .collection
            .as_ref()
            .ok_or_else(|| "Study SGF collection is unavailable".to_owned())?;

        let structure = bermuda::build_study_structure_tree(collection)
            .map_err(|error| format!("building structural Study tree: {error}"))?;

        let selected = structure
            .nodes
            .get(structure_node_id)
            .ok_or_else(|| format!("structural SGF node {structure_node_id} does not exist"))?;

        let replay = document
            .study_tree
            .as_ref()
            .ok_or_else(|| "the Study document has no SGF game tree".to_owned())?;

        let replay_node_id = replay_node_for_structure(&structure, replay, structure_node_id)
            .ok_or_else(|| "the selected SGF node has no replayable Study position".to_owned())?;

        let display_move = i32::try_from(selected.move_number)
            .map_err(|_| "selected SGF move is too large for the Qt interface".to_owned())?;

        (replay_node_id, display_move)
    };

    activate_study_tree_node(document, replay_node_id)?;

    Ok(display_move)
}

fn activate_study_tree_node(document: &mut LoadedDocument, node_id: usize) -> Result<i32, String> {
    let (path, selected_index, positions, comments) = {
        let tree = document
            .study_tree
            .as_ref()
            .ok_or_else(|| "the loaded document has no SGF game tree".to_owned())?;

        let path = tree
            .path_through(node_id)
            .ok_or_else(|| format!("SGF tree node {node_id} does not exist"))?;

        let selected_index = path
            .iter()
            .position(|candidate| *candidate == node_id)
            .ok_or_else(|| "selected SGF node is not on its own replay path".to_owned())?;

        let positions = path
            .iter()
            .map(|id| tree.nodes[*id].position.clone())
            .collect::<Vec<_>>();

        let comments = path
            .iter()
            .map(|id| tree.nodes[*id].comment.clone())
            .collect::<Vec<_>>();

        (path, selected_index, positions, comments)
    };

    document.positions = positions;
    document.source_comments = comments;

    document
        .study_tree
        .as_mut()
        .expect("Study tree was checked above")
        .active_path = path;

    i32::try_from(selected_index)
        .map_err(|_| "selected SGF move is too large for the Qt interface".to_owned())
}

fn study_document_state_from_collection(
    collection: &Collection,
    path: &Path,
) -> Result<StudyDocumentState, String> {
    let metadata = StudyDocumentMetadata::from_collection(collection).map_err(|error| {
        format!(
            "reading Bermuda Study metadata from {}: {error}",
            path.display()
        )
    })?;

    match metadata {
        Some(metadata) => {
            /*
             * Bermuda owns and updates Study documents only inside its
             * Study Library. A Study SGF copied/exported elsewhere is an
             * external source and must itself remain untouched.
             */
            let library_path = study_library_path_matches(path).then(|| path.to_path_buf());

            Ok(StudyDocumentState {
                metadata,
                collection: Some(collection.clone()),
                library_path,
                history: StudyEditHistory::default(),
            })
        }

        None => Ok(StudyDocumentState {
            metadata: StudyDocumentMetadata::new(StudyOrigin::ExternalSgf {
                path: path.to_string_lossy().into_owned(),
            }),
            collection: Some(collection.clone()),
            library_path: None,
            history: StudyEditHistory::default(),
        }),
    }
}

fn study_library_path_matches(path: &Path) -> bool {
    let Ok(root) = study_library_root() else {
        return false;
    };

    let Ok(canonical_root) = fs::canonicalize(root) else {
        return false;
    };

    let Ok(canonical_path) = fs::canonicalize(path) else {
        return false;
    };

    canonical_path.parent() == Some(canonical_root.as_path())
}

fn document_uses_study_library_path(document: Option<&LoadedDocument>, target: &Path) -> bool {
    document
        .and_then(|document| document.study.library_path.as_deref())
        .and_then(|path| fs::canonicalize(path).ok())
        .is_some_and(|path| path == target)
}

fn delete_study_library_document_path(
    app: &BermudaAppRust,
    requested: &Path,
) -> Result<(), String> {
    let root = study_library_root()?;
    let canonical_root = fs::canonicalize(&root)
        .map_err(|error| format!("opening Study Library {}: {error}", root.display()))?;
    let target = fs::canonicalize(requested)
        .map_err(|error| format!("opening Study document {}: {error}", requested.display()))?;

    if target.parent() != Some(canonical_root.as_path()) {
        return Err("refusing to delete a file outside the Study Library".to_owned());
    }

    let bytes = fs::read(&target)
        .map_err(|error| format!("reading Study document {}: {error}", target.display()))?;
    let collection = parse_collection(&bytes)
        .map_err(|error| format!("parsing Study document {}: {error}", target.display()))?;

    let is_study_document = StudyDocumentMetadata::from_collection(&collection)
        .map_err(|error| {
            format!(
                "reading Bermuda Study metadata from {}: {error}",
                target.display()
            )
        })?
        .is_some();

    if !is_study_document {
        return Err("refusing to delete a file without Bermuda Study metadata".to_owned());
    }

    let active = document_uses_study_library_path(app.loaded_document.as_ref(), &target);
    let parked = app.workspace_snapshot.as_ref().is_some_and(|snapshot| {
        document_uses_study_library_path(snapshot.document.as_ref(), &target)
            || snapshot
                .search_source_snapshot
                .as_ref()
                .is_some_and(|source| {
                    document_uses_study_library_path(Some(&source.document), &target)
                })
    });
    let search_source = app
        .search_source_snapshot
        .as_ref()
        .is_some_and(|source| document_uses_study_library_path(Some(&source.document), &target));

    if active || parked || search_source {
        return Err(
            "close or replace this Study document before deleting it from the library".to_owned(),
        );
    }

    fs::remove_file(&target)
        .map_err(|error| format!("deleting Study document {}: {error}", target.display()))
}

fn study_library_catalogue_json() -> Result<String, String> {
    let root = study_library_root()?;

    if !root.exists() {
        return Ok("[]".to_owned());
    }

    let directory = fs::read_dir(&root)
        .map_err(|error| format!("reading Study Library {}: {error}", root.display()))?;

    let mut entries = Vec::new();

    for item in directory {
        let entry = match item {
            Ok(entry) => entry,
            Err(_) => continue,
        };

        let path = entry.path();

        let is_sgf = path
            .extension()
            .and_then(|extension| extension.to_str())
            .map(|extension| extension.eq_ignore_ascii_case("sgf"))
            .unwrap_or(false);

        if !is_sgf || !path.is_file() {
            continue;
        }

        let bytes = match fs::read(&path) {
            Ok(bytes) => bytes,
            Err(_) => continue,
        };

        let collection = match parse_collection(&bytes) {
            Ok(collection) => collection,
            Err(_) => continue,
        };

        let metadata = match StudyDocumentMetadata::from_collection(&collection) {
            Ok(Some(metadata)) => metadata,
            Ok(None) | Err(_) => continue,
        };

        let record = match extract_main_variation(&collection) {
            Ok(record) => record,
            Err(_) => continue,
        };

        let modified_millis = entry
            .metadata()
            .ok()
            .and_then(|metadata| metadata.modified().ok())
            .and_then(|modified| modified.duration_since(UNIX_EPOCH).ok())
            .and_then(|duration| u64::try_from(duration.as_millis()).ok())
            .unwrap_or(0);

        let file_name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("Study document")
            .to_owned();

        let black_player = record.metadata.black_player.unwrap_or_default();
        let white_player = record.metadata.white_player.unwrap_or_default();

        let fallback_title = match &metadata.origin {
            StudyOrigin::ProjectGame { game_id, .. } => {
                format!("Game {game_id}")
            }

            StudyOrigin::ExternalSgf { path } => Path::new(path)
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("External SGF")
                .to_owned(),

            StudyOrigin::Detached { description } => description.clone(),
        };

        let title = match (
            black_player.trim().is_empty(),
            white_player.trim().is_empty(),
        ) {
            (false, false) => format!("{} – {}", black_player.trim(), white_player.trim()),
            (false, true) => black_player.trim().to_owned(),
            (true, false) => white_player.trim().to_owned(),
            (true, true) => fallback_title,
        };

        let origin = match &metadata.origin {
            StudyOrigin::ProjectGame { game_id, .. } => {
                format!("Game database · game {game_id}")
            }

            StudyOrigin::ExternalSgf { path } => {
                let name = Path::new(path)
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or(path.as_str());

                format!("External SGF · {name}")
            }

            StudyOrigin::Detached { description } => description.clone(),
        };

        entries.push((
            modified_millis,
            serde_json::json!({
                "path": path.to_string_lossy(),
                "fileName": file_name,
                "title": title,
                "blackPlayer": black_player,
                "whitePlayer": white_player,
                "date": record.metadata.date.unwrap_or_default(),
                "event": record.metadata.event.unwrap_or_default(),
                "result": record.metadata.result.unwrap_or_default(),
                "annotationCount": metadata.annotations.len(),
                "origin": origin,
                "modifiedMillis": modified_millis,
            }),
        ));
    }

    entries.sort_by(|left, right| right.0.cmp(&left.0));

    let entries = entries
        .into_iter()
        .map(|(_, value)| value)
        .collect::<Vec<_>>();

    serde_json::to_string(&entries)
        .map_err(|error| format!("encoding Study Library catalogue: {error}"))
}

fn study_library_root() -> Result<PathBuf, String> {
    let project_dirs = ProjectDirs::from("org", "Bermuda", "Bermuda")
        .ok_or_else(|| "could not determine the per-user Bermuda data directory".to_owned())?;

    Ok(project_dirs.data_local_dir().join(STUDY_LIBRARY_DIRECTORY))
}

fn new_study_library_path() -> Result<PathBuf, String> {
    let root = study_library_root()?;

    fs::create_dir_all(&root)
        .map_err(|error| format!("creating Study Library {}: {error}", root.display()))?;

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| format!("reading system clock for Study Library: {error}"))?
        .as_nanos();

    let sequence = STUDY_DOCUMENT_SEQUENCE.fetch_add(1, Ordering::Relaxed);

    Ok(root.join(format!("study-{timestamp}-{sequence}.sgf")))
}

fn study_game_record(document: &LoadedDocument) -> Result<GameRecord, String> {
    let initial = document
        .positions
        .first()
        .ok_or_else(|| "the Study document has no position".to_owned())?;

    let board_size = initial.board.size();
    let point_count = u16::from(board_size) * u16::from(board_size);
    let mut setup = Vec::new();

    for point in 0..point_count {
        if let Some(colour) = initial.board.colour_at(point) {
            setup.push(SetupStone::Add { colour, point });
        }
    }

    let mut moves = Vec::with_capacity(document.positions.len().saturating_sub(1));

    for (index, position) in document.positions.iter().enumerate().skip(1) {
        let mv = position
            .last_move
            .ok_or_else(|| format!("Study position {index} has no recorded move"))?;

        moves.push(mv);
    }

    Ok(GameRecord {
        board_size,
        metadata: Metadata {
            black_player: document.black_player.clone(),
            white_player: document.white_player.clone(),
            date: None,
            event: None,
            result: document.result.clone(),
            komi: document.komi,
            handicap: None,
        },
        setup,
        moves,
    })
}

fn study_collection(document: &LoadedDocument) -> Result<Collection, String> {
    if let Some(collection) = &document.study.collection {
        return Ok(collection.clone());
    }

    let record = study_game_record(document)?;
    let sgf =
        write_game_record_sgf(&record).map_err(|error| format!("creating Study SGF: {error}"))?;

    parse_collection(sgf.as_bytes())
        .map_err(|error| format!("parsing generated Study SGF: {error}"))
}

fn study_history_entry(
    document: &LoadedDocument,
    move_number: i32,
    structural_node: Option<i32>,
) -> Result<StudyHistoryEntry, String> {
    let collection = study_collection(document)?;

    let structural_selection = structural_node
        .and_then(|node_id| usize::try_from(node_id).ok())
        .and_then(|node_id| {
            bermuda::build_study_structure_tree(&collection)
                .ok()?
                .nodes
                .get(node_id)
                .map(|node| node.source.clone())
        });

    let position_index = usize::try_from(move_number).ok();

    let replay_selection = position_index.and_then(|position_index| {
        let tree = document.study_tree.as_ref()?;

        let node_id = tree
            .active_path
            .get(position_index)
            .copied()
            .or_else(|| tree.main_path.get(position_index).copied())?;

        tree.nodes.get(node_id).map(|node| node.source.clone())
    });

    Ok(StudyHistoryEntry {
        collection,
        metadata: document.study.metadata.clone(),
        selection: structural_selection.or(replay_selection),
        move_number,
    })
}

fn restore_study_history_entry(
    document: &mut LoadedDocument,
    entry: StudyHistoryEntry,
) -> Result<(i32, i32), String> {
    let library_path = document.study.library_path.clone();

    document.study.metadata = entry.metadata;
    document.study.collection = Some(entry.collection);
    document.study.library_path = library_path;

    let collection = document
        .study
        .collection
        .as_ref()
        .expect("Study history restored its SGF collection");

    let tree = build_study_tree(collection)
        .map_err(|error| format!("rebuilding Study tree from edit history: {error}"))?;

    document.study_tree = Some(tree);

    let structure = bermuda::build_study_structure_tree(collection)
        .map_err(|error| format!("rebuilding structural Study tree from edit history: {error}"))?;

    let selected_structure_node = entry
        .selection
        .as_ref()
        .and_then(|source| structure.node_id_for_source(source));

    let (display_move, structure_node) = if let Some(structure_node) = selected_structure_node {
        let display_move = activate_study_structure_node(document, structure_node)?;

        let structure_node = i32::try_from(structure_node)
            .map_err(|_| "restored SGF node is too large for the Qt interface".to_owned())?;

        (display_move, structure_node)
    } else {
        let position_index = usize::try_from(entry.move_number).map_err(|_| {
            format!(
                "invalid Study move number {} in edit history",
                entry.move_number
            )
        })?;

        let replay_node = document
            .study_tree
            .as_ref()
            .and_then(|tree| tree.main_path.get(position_index).copied())
            .ok_or_else(|| "restored Study position is outside the SGF tree".to_owned())?;

        let display_move = activate_study_tree_node(document, replay_node)?;

        let source = document
            .study_tree
            .as_ref()
            .and_then(|tree| tree.nodes.get(replay_node))
            .map(|node| node.source.clone());

        let structure_node = source
            .as_ref()
            .and_then(|source| structure.node_id_for_source(source))
            .and_then(|node_id| i32::try_from(node_id).ok())
            .unwrap_or(-1);

        (display_move, structure_node)
    };

    persist_study_document(document)?;

    Ok((display_move, structure_node))
}

fn step_study_history(
    document: &mut LoadedDocument,
    move_number: i32,
    structural_node: i32,
    undo: bool,
) -> Result<(i32, i32), String> {
    let previous = document.clone();

    let result = (|| {
        let current = study_history_entry(document, move_number, Some(structural_node))?;

        let target = if undo {
            document
                .study
                .history
                .undo
                .pop()
                .ok_or_else(|| "nothing to undo".to_owned())?
        } else {
            document
                .study
                .history
                .redo
                .pop()
                .ok_or_else(|| "nothing to redo".to_owned())?
        };

        if undo {
            document.study.history.push_redo(current);
        } else {
            document.study.history.push_undo(current);
        }

        restore_study_history_entry(document, target)
    })();

    match result {
        Ok(value) => Ok(value),

        Err(error) => {
            *document = previous;
            Err(error)
        }
    }
}

fn persist_study_document(document: &mut LoadedDocument) -> Result<(), String> {
    let mut collection = study_collection(document)?;

    document
        .study
        .metadata
        .apply_to_collection(&mut collection)
        .map_err(|error| format!("preparing Bermuda Study metadata: {error}"))?;

    let sgf = write_collection_sgf(&collection);

    let first_save = document.study.library_path.is_none();

    let path = match &document.study.library_path {
        Some(path) => path.clone(),
        None => new_study_library_path()?,
    };

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("creating {}: {error}", parent.display()))?;
    }

    fs::write(&path, sgf)
        .map_err(|error| format!("writing Study document {}: {error}", path.display()))?;

    document.study.collection = Some(collection);
    document.study.library_path = Some(path.clone());

    if first_save {
        eprintln!("Saved Study Library document: {}", path.display());
    }

    Ok(())
}

fn study_location(
    document: &LoadedDocument,
    slider_move_number: i32,
) -> Result<(Option<usize>, usize, usize), String> {
    let position_index = usize::try_from(slider_move_number)
        .map_err(|_| format!("invalid Study move number {slider_move_number}"))?;

    if document.positions.get(position_index).is_none() {
        return Err(format!(
            "Study move {slider_move_number} is outside the loaded variation"
        ));
    }

    let node_id = document
        .study_tree
        .as_ref()
        .and_then(|tree| tree.active_path.get(position_index))
        .copied();

    Ok((node_id, position_index, position_index))
}

fn qml_study_point(
    document: &LoadedDocument,
    position_index: usize,
    x: i32,
    y: i32,
) -> Result<u16, String> {
    let position = document
        .positions
        .get(position_index)
        .ok_or_else(|| "Study position is not available".to_owned())?;

    let qml_x = u8::try_from(x).map_err(|_| format!("invalid board coordinate {x},{y}"))?;
    let qml_y = u8::try_from(y).map_err(|_| format!("invalid board coordinate {x},{y}"))?;

    let core_y = qml_y_to_core(position.board.size(), qml_y)?;

    position
        .board
        .point(qml_x, core_y)
        .map_err(|error| error.to_string())
}

fn sgf_node_at_mut<'a>(
    collection: &'a mut Collection,
    location: &StudySourceLocation,
) -> Result<&'a mut Node, String> {
    let mut tree = collection
        .trees
        .first_mut()
        .ok_or_else(|| "Study SGF has no game tree".to_owned())?;

    for &variation_index in &location.variation_path {
        tree = tree.variations.get_mut(variation_index).ok_or_else(|| {
            format!(
                "Study SGF variation path {:?} no longer exists",
                location.variation_path
            )
        })?;
    }

    tree.sequence
        .get_mut(location.sequence_index)
        .ok_or_else(|| {
            format!(
                "Study SGF node {} at variation path {:?} no longer exists",
                location.sequence_index, location.variation_path
            )
        })
}

fn replace_collection_comment(
    collection: &mut Collection,
    source: &StudySourceLocation,
    comment_sources: &[StudySourceLocation],
    text: &str,
) -> Result<(), String> {
    /*
     * One displayed Study comment can be assembled from several SGF
     * comment-only nodes. Collapse those C properties onto the canonical
     * displayed node when the user edits the comment. The SGF nodes
     * themselves remain in place, so unrelated properties are preserved.
     */
    for location in comment_sources {
        if location == source {
            continue;
        }

        sgf_node_at_mut(collection, location)?
            .properties
            .remove("C");
    }

    let node = sgf_node_at_mut(collection, source)?;

    if text.trim().is_empty() {
        node.properties.remove("C");
    } else {
        node.properties
            .insert("C".to_owned(), vec![text.to_owned()]);
    }

    Ok(())
}

fn study_comment_edit_target(
    document: &mut LoadedDocument,
    slider_move_number: i32,
) -> Result<(StudySourceLocation, Vec<StudySourceLocation>), String> {
    let position_index = usize::try_from(slider_move_number)
        .map_err(|_| format!("invalid Study move number {slider_move_number}"))?;

    if document.study_tree.is_none() {
        let collection = study_collection(document)?;
        let tree = build_study_tree(&collection)
            .map_err(|error| format!("building Study tree for editing: {error}"))?;

        if position_index >= tree.main_path.len() {
            return Err(format!(
                "Study move {slider_move_number} is outside the SGF tree"
            ));
        }

        document.study.collection = Some(collection);
        document.study_tree = Some(tree);
    }

    let tree = document
        .study_tree
        .as_ref()
        .ok_or_else(|| "the Study document has no SGF game tree".to_owned())?;

    let node_id = tree
        .active_path
        .get(position_index)
        .copied()
        .or_else(|| tree.main_path.get(position_index).copied())
        .ok_or_else(|| {
            format!("Study move {slider_move_number} is outside the active SGF variation")
        })?;

    let node = tree
        .nodes
        .get(node_id)
        .ok_or_else(|| format!("Study tree node {node_id} does not exist"))?;

    Ok((node.source.clone(), node.comment_sources.clone()))
}

fn update_study_comment(
    document: &mut LoadedDocument,
    slider_move_number: i32,
    text: &str,
) -> Result<i32, String> {
    let previous = document.clone();
    let history_entry = study_history_entry(&previous, slider_move_number, None)?;

    let result = (|| {
        let (source, comment_sources) = study_comment_edit_target(document, slider_move_number)?;

        let mut collection = document
            .study
            .collection
            .clone()
            .ok_or_else(|| "Study SGF collection is unavailable".to_owned())?;

        replace_collection_comment(&mut collection, &source, &comment_sources, text)?;

        let tree = build_study_tree(&collection)
            .map_err(|error| format!("rebuilding Study tree after comment edit: {error}"))?;

        let node_id = tree
            .node_id_for_source(&source)
            .ok_or_else(|| "edited SGF node disappeared while rebuilding Study".to_owned())?;

        document.study.collection = Some(collection);
        document.study_tree = Some(tree);

        let display_move = activate_study_tree_node(document, node_id)?;

        persist_study_document(document)?;

        document.study.history.record_edit(history_entry);

        Ok(display_move)
    })();

    match result {
        Ok(display_move) => Ok(display_move),

        Err(error) => {
            *document = previous;
            Err(error)
        }
    }
}

fn update_study_delete_from_here(
    document: &mut LoadedDocument,
    slider_move_number: i32,
    selected_structure_node: i32,
) -> Result<(i32, i32), String> {
    let previous = document.clone();
    let history_entry =
        study_history_entry(&previous, slider_move_number, Some(selected_structure_node))?;

    let result = (|| {
        let position_index = usize::try_from(slider_move_number)
            .map_err(|_| format!("invalid Study move number {slider_move_number}"))?;

        if document.study_tree.is_none() {
            let collection = study_collection(document)?;
            let tree = build_study_tree(&collection)
                .map_err(|error| format!("building Study tree for deletion: {error}"))?;

            document.study.collection = Some(collection);
            document.study_tree = Some(tree);
        }

        let structure = {
            let collection = document
                .study
                .collection
                .as_ref()
                .ok_or_else(|| "Study SGF collection is unavailable".to_owned())?;

            bermuda::build_study_structure_tree(collection)
                .map_err(|error| format!("building structural Study tree for deletion: {error}"))?
        };

        let selected_id = usize::try_from(selected_structure_node)
            .ok()
            .filter(|id| *id < structure.nodes.len())
            .or_else(|| {
                let replay = document.study_tree.as_ref()?;
                let replay_id = replay
                    .active_path
                    .get(position_index)
                    .copied()
                    .or_else(|| replay.main_path.get(position_index).copied())?;
                let source = &replay.nodes.get(replay_id)?.source;
                structure.node_id_for_source(source)
            })
            .ok_or_else(|| "the current Study position has no structural SGF node".to_owned())?;

        let source = structure.nodes[selected_id].source.clone();

        let mut collection = document
            .study
            .collection
            .clone()
            .ok_or_else(|| "Study SGF collection is unavailable".to_owned())?;

        let selected_source = bermuda::delete_study_from_source(&mut collection, &source)?;

        let tree = build_study_tree(&collection)
            .map_err(|error| format!("rebuilding Study tree after deletion: {error}"))?;

        let new_structure = bermuda::build_study_structure_tree(&collection)
            .map_err(|error| format!("rebuilding structural Study tree after deletion: {error}"))?;

        let selected_id = new_structure
            .node_id_for_source(&selected_source)
            .ok_or_else(|| "the node before the deleted Study branch disappeared".to_owned())?;

        document.study.collection = Some(collection);
        document.study_tree = Some(tree);

        let display_move = activate_study_structure_node(document, selected_id)?;

        persist_study_document(document)?;
        document.study.history.record_edit(history_entry);

        let selected_id = i32::try_from(selected_id)
            .map_err(|_| "selected SGF node is too large for the Qt interface".to_owned())?;

        Ok((display_move, selected_id))
    })();

    match result {
        Ok(value) => Ok(value),
        Err(error) => {
            *document = previous;
            Err(error)
        }
    }
}

fn update_study_node_insertion(
    document: &mut LoadedDocument,
    slider_move_number: i32,
    selected_structure_node: i32,
) -> Result<(i32, i32), String> {
    let previous = document.clone();
    let history_entry =
        study_history_entry(&previous, slider_move_number, Some(selected_structure_node))?;

    let result = (|| {
        let position_index = usize::try_from(slider_move_number)
            .map_err(|_| format!("invalid Study move number {slider_move_number}"))?;

        if document.study_tree.is_none() {
            let collection = study_collection(document)?;
            let tree = build_study_tree(&collection)
                .map_err(|error| format!("building Study tree for node insertion: {error}"))?;

            document.study.collection = Some(collection);
            document.study_tree = Some(tree);
        }

        let structure = {
            let collection = document
                .study
                .collection
                .as_ref()
                .ok_or_else(|| "Study SGF collection is unavailable".to_owned())?;

            bermuda::build_study_structure_tree(collection)
                .map_err(|error| format!("building structural Study tree for editing: {error}"))?
        };

        let selected_id = usize::try_from(selected_structure_node)
            .ok()
            .filter(|id| *id < structure.nodes.len())
            .or_else(|| {
                let replay = document.study_tree.as_ref()?;
                let replay_id = replay
                    .active_path
                    .get(position_index)
                    .copied()
                    .or_else(|| replay.main_path.get(position_index).copied())?;
                let source = &replay.nodes.get(replay_id)?.source;
                structure.node_id_for_source(source)
            })
            .ok_or_else(|| "the current Study position has no structural SGF node".to_owned())?;

        let source = structure.nodes[selected_id].source.clone();

        let mut collection = document
            .study
            .collection
            .clone()
            .ok_or_else(|| "Study SGF collection is unavailable".to_owned())?;

        let inserted_source = bermuda::insert_study_node_after_source(&mut collection, &source)?;

        let tree = build_study_tree(&collection)
            .map_err(|error| format!("rebuilding Study tree after node insertion: {error}"))?;

        let new_structure = bermuda::build_study_structure_tree(&collection).map_err(|error| {
            format!("rebuilding structural Study tree after insertion: {error}")
        })?;

        let inserted_id = new_structure
            .node_id_for_source(&inserted_source)
            .ok_or_else(|| "inserted SGF node disappeared while rebuilding Study".to_owned())?;

        document.study.collection = Some(collection);
        document.study_tree = Some(tree);

        let display_move = activate_study_structure_node(document, inserted_id)?;

        persist_study_document(document)?;

        document.study.history.record_edit(history_entry);

        let inserted_id = i32::try_from(inserted_id)
            .map_err(|_| "inserted SGF node is too large for the Qt interface".to_owned())?;

        Ok((display_move, inserted_id))
    })();

    match result {
        Ok(value) => Ok(value),

        Err(error) => {
            *document = previous;
            Err(error)
        }
    }
}

fn update_study_move(
    document: &mut LoadedDocument,
    slider_move_number: i32,
    qml_point: Option<(i32, i32)>,
) -> Result<i32, String> {
    let previous = document.clone();
    let history_entry = study_history_entry(&previous, slider_move_number, None)?;

    let result = (|| {
        let position_index = usize::try_from(slider_move_number)
            .map_err(|_| format!("invalid Study move number {slider_move_number}"))?;

        /*
         * Database games do not carry an SGF Study tree while merely being
         * browsed. The first structural Study operation promotes the
         * displayed game into an in-memory SGF collection/tree.
         */
        if document.study_tree.is_none() {
            let collection = study_collection(document)?;
            let tree = build_study_tree(&collection)
                .map_err(|error| format!("building Study tree for editing: {error}"))?;

            document.study.collection = Some(collection);
            document.study_tree = Some(tree);
        }

        let (node_id, colour, point) = {
            let tree = document
                .study_tree
                .as_ref()
                .ok_or_else(|| "the Study document has no SGF game tree".to_owned())?;

            let node_id = tree
                .active_path
                .get(position_index)
                .copied()
                .or_else(|| tree.main_path.get(position_index).copied())
                .ok_or_else(|| {
                    format!("Study move {slider_move_number} is outside the active SGF variation")
                })?;

            let node = tree
                .nodes
                .get(node_id)
                .ok_or_else(|| format!("Study tree node {node_id} does not exist"))?;

            let point = match qml_point {
                None => None,

                Some((x, y)) => {
                    let qml_x =
                        u8::try_from(x).map_err(|_| format!("invalid board coordinate {x},{y}"))?;
                    let qml_y =
                        u8::try_from(y).map_err(|_| format!("invalid board coordinate {x},{y}"))?;

                    let board_size = node.position.board.size();
                    let core_y = qml_y_to_core(board_size, qml_y)?;

                    Some(
                        node.position
                            .board
                            .point(qml_x, core_y)
                            .map_err(|error| error.to_string())?,
                    )
                }
            };

            (node_id, node.position.occurrence.side_to_move, point)
        };

        let mv = Move { colour, point };

        let mut collection = document
            .study
            .collection
            .clone()
            .ok_or_else(|| "Study SGF collection is unavailable".to_owned())?;

        let insertion = {
            let tree = document
                .study_tree
                .as_ref()
                .expect("Study tree was created above");

            bermuda::extend_study_move(&mut collection, tree, node_id, mv)?
        };

        /*
         * Collection remains the editable source of truth. Rebuild the
         * displayed Study tree after every structural operation.
         */
        let tree = build_study_tree(&collection)
            .map_err(|error| format!("rebuilding Study tree after move edit: {error}"))?;

        let new_node_id = tree.node_id_for_source(&insertion.source).ok_or_else(|| {
            "inserted SGF continuation disappeared while rebuilding Study".to_owned()
        })?;

        document.study.collection = Some(collection);
        document.study_tree = Some(tree);

        let display_move = activate_study_tree_node(document, new_node_id)?;

        if insertion.inserted {
            persist_study_document(document)?;
            document.study.history.record_edit(history_entry);
        }

        Ok(display_move)
    })();

    match result {
        Ok(display_move) => Ok(display_move),

        Err(error) => {
            *document = previous;
            Err(error)
        }
    }
}

fn study_tree_has_continuation(tree: &StudyTree, position_index: usize) -> bool {
    let node_id = tree
        .active_path
        .get(position_index)
        .copied()
        .or_else(|| tree.main_path.get(position_index).copied());

    node_id
        .and_then(|node_id| tree.nodes.get(node_id))
        .is_some_and(|node| !node.children.is_empty())
}

fn update_study_branch(
    document: &mut LoadedDocument,
    slider_move_number: i32,
    qml_point: Option<(i32, i32)>,
) -> Result<i32, String> {
    let previous = document.clone();
    let history_entry = study_history_entry(&previous, slider_move_number, None)?;

    let result = (|| {
        let position_index = usize::try_from(slider_move_number)
            .map_err(|_| format!("invalid Study move number {slider_move_number}"))?;

        /*
         * Database games do not carry an SGF Study tree while merely being
         * browsed. The first structural Study operation promotes the
         * displayed game into an in-memory SGF collection/tree.
         */
        if document.study_tree.is_none() {
            let collection = study_collection(document)?;
            let tree = build_study_tree(&collection)
                .map_err(|error| format!("building Study tree for editing: {error}"))?;

            document.study.collection = Some(collection);
            document.study_tree = Some(tree);
        }

        let (node_id, colour, point) = {
            let tree = document
                .study_tree
                .as_ref()
                .ok_or_else(|| "the Study document has no SGF game tree".to_owned())?;

            let node_id = tree
                .active_path
                .get(position_index)
                .copied()
                .or_else(|| tree.main_path.get(position_index).copied())
                .ok_or_else(|| {
                    format!("Study move {slider_move_number} is outside the active SGF variation")
                })?;

            let node = tree
                .nodes
                .get(node_id)
                .ok_or_else(|| format!("Study tree node {node_id} does not exist"))?;

            if node.children.is_empty() {
                return Err(
                    "the selected Study position has no continuation to branch from; ".to_owned()
                        + "use Add move instead",
                );
            }

            let point = match qml_point {
                None => None,

                Some((x, y)) => {
                    let qml_x =
                        u8::try_from(x).map_err(|_| format!("invalid board coordinate {x},{y}"))?;
                    let qml_y =
                        u8::try_from(y).map_err(|_| format!("invalid board coordinate {x},{y}"))?;

                    let board_size = node.position.board.size();
                    let core_y = qml_y_to_core(board_size, qml_y)?;

                    Some(
                        node.position
                            .board
                            .point(qml_x, core_y)
                            .map_err(|error| error.to_string())?,
                    )
                }
            };

            (node_id, node.position.occurrence.side_to_move, point)
        };

        let mv = Move { colour, point };

        let mut collection = document
            .study
            .collection
            .clone()
            .ok_or_else(|| "Study SGF collection is unavailable".to_owned())?;

        let insertion = {
            let tree = document
                .study_tree
                .as_ref()
                .expect("Study tree was created above");

            bermuda::branch_study_move(&mut collection, tree, node_id, mv)?
        };

        /*
         * Collection remains the editable source of truth. Rebuild the
         * displayed Study tree after every structural operation.
         */
        let tree = build_study_tree(&collection)
            .map_err(|error| format!("rebuilding Study tree after branch edit: {error}"))?;

        let new_node_id = tree.node_id_for_source(&insertion.source).ok_or_else(|| {
            "inserted SGF continuation disappeared while rebuilding Study".to_owned()
        })?;

        document.study.collection = Some(collection);
        document.study_tree = Some(tree);

        let display_move = activate_study_tree_node(document, new_node_id)?;

        if insertion.inserted {
            persist_study_document(document)?;
            document.study.history.record_edit(history_entry);
        }

        Ok(display_move)
    })();

    match result {
        Ok(display_move) => Ok(display_move),

        Err(error) => {
            *document = previous;
            Err(error)
        }
    }
}

fn study_annotation_kind(tool: &str, text: &str) -> Result<StudyAnnotationKind, String> {
    match tool {
        "cross" => Ok(StudyAnnotationKind::Cross),
        "triangle" => Ok(StudyAnnotationKind::Triangle),
        "circle" => Ok(StudyAnnotationKind::Circle),
        "square" => Ok(StudyAnnotationKind::Square),
        "letter" => Ok(StudyAnnotationKind::Letter),

        "number" => {
            let move_number = text
                .parse::<u32>()
                .map_err(|_| format!("invalid Study move-number label {text:?}"))?;

            Ok(StudyAnnotationKind::MoveNumber { move_number })
        }

        "label" => {
            let text = text.trim();

            if text.is_empty() {
                return Err("Study label must not be empty".to_owned());
            }

            Ok(StudyAnnotationKind::Label {
                text: text.to_owned(),
            })
        }

        other => Err(format!("unknown Study annotation tool {other:?}")),
    }
}

fn update_study_annotation(
    document: &mut LoadedDocument,
    slider_move_number: i32,
    x: i32,
    y: i32,
    tool: &str,
    text: &str,
) -> Result<(), String> {
    let (node_id, move_number, position_index) = study_location(document, slider_move_number)?;

    let point = qml_study_point(document, position_index, x, y)?;

    /*
     * Markup that came from the source SGF is evidence belonging to that
     * source document.  Study annotations may coexist with the source
     * analysis, but must not replace or obscure an existing source mark at
     * the same point.
     */
    if let (Some(tree), Some(node_id)) = (&document.study_tree, node_id)
        && let Some(node) = tree.nodes.get(node_id)
        && node.markup.iter().any(|mark| mark.point == point)
    {
        return Err("source SGF markup at this point is read-only".to_owned());
    }

    let kind = study_annotation_kind(tool, text)?;

    let previous = document.clone();
    let history_entry = study_history_entry(&previous, slider_move_number, None)?;

    document
        .study
        .metadata
        .set_annotation(node_id, move_number, point, kind);

    if let Err(error) = persist_study_document(document) {
        *document = previous;
        return Err(error);
    }

    document.study.history.record_edit(history_entry);

    Ok(())
}

fn looks_like_study_letter(text: &str) -> bool {
    !text.is_empty() && text.bytes().all(|byte| byte.is_ascii_uppercase())
}

fn study_letter(mut index: usize) -> String {
    index += 1;
    let mut characters = Vec::new();

    while index > 0 {
        index -= 1;
        characters.push((b'A' + (index % 26) as u8) as char);
        index /= 26;
    }

    characters.iter().rev().collect()
}

fn render_study_board_markup_json(document: &LoadedDocument, slider_move_number: i32) -> String {
    let Ok((node_id, move_number, position_index)) = study_location(document, slider_move_number)
    else {
        return "[]".to_owned();
    };

    let Some(position) = document.positions.get(position_index) else {
        return "[]".to_owned();
    };

    let board_size = u16::from(position.board.size());

    let source_json = match node_id {
        Some(node_id) => i32::try_from(node_id)
            .ok()
            .map(|node_id| study_markup_json(document.study_tree.as_ref(), node_id))
            .unwrap_or_else(|| "[]".to_owned()),

        None => "[]".to_owned(),
    };

    let mut rendered =
        serde_json::from_str::<Vec<serde_json::Value>>(&source_json).unwrap_or_default();

    let mut reserved_labels = HashSet::new();

    if let (Some(tree), Some(node_id)) = (&document.study_tree, node_id)
        && let Some(node) = tree.nodes.get(node_id)
    {
        for mark in &node.markup {
            if let StudyMarkupKind::Label(text) = &mark.kind
                && looks_like_study_letter(text)
            {
                reserved_labels.insert(text.clone());
            }
        }
    }

    for annotation in document.study.metadata.annotations_at(node_id, move_number) {
        if let StudyAnnotationKind::Label { text } = &annotation.kind
            && looks_like_study_letter(text)
        {
            reserved_labels.insert(text.clone());
        }
    }

    let mut letter_index = 0usize;

    for annotation in document.study.metadata.annotations_at(node_id, move_number) {
        let x = annotation.point % board_size;
        let core_y = annotation.point / board_size;
        let y = core_y_to_qml(board_size, core_y);

        let value = match &annotation.kind {
            StudyAnnotationKind::Cross => serde_json::json!({
                "type": "cross",
                "x": x,
                "y": y,
            }),

            StudyAnnotationKind::Triangle => serde_json::json!({
                "type": "triangle",
                "x": x,
                "y": y,
            }),

            StudyAnnotationKind::Circle => serde_json::json!({
                "type": "circle",
                "x": x,
                "y": y,
            }),

            StudyAnnotationKind::Square => serde_json::json!({
                "type": "square",
                "x": x,
                "y": y,
            }),

            StudyAnnotationKind::MoveNumber { move_number } => serde_json::json!({
                "type": "label",
                "x": x,
                "y": y,
                "text": move_number.to_string(),
            }),

            StudyAnnotationKind::Label { text } => serde_json::json!({
                "type": "label",
                "x": x,
                "y": y,
                "text": text,
            }),

            StudyAnnotationKind::Letter => {
                let text = loop {
                    let candidate = study_letter(letter_index);
                    letter_index += 1;

                    if reserved_labels.insert(candidate.clone()) {
                        break candidate;
                    }
                };

                serde_json::json!({
                    "type": "label",
                    "x": x,
                    "y": y,
                    "text": text,
                })
            }
        };

        rendered.push(value);
    }

    serde_json::to_string(&rendered).unwrap_or_else(|_| "[]".to_owned())
}

fn load_game_document(project_path: &str, game_id: i64) -> Result<LoadedDocument, String> {
    let project = ProjectManager::new()
        .open(Path::new(project_path))
        .map_err(|error| error.to_string())?;

    /*
     * Board positions come from the canonical game record, while metadata
     * comes from the catalogue's preferred source metadata.  A canonical
     * game can have several source records, so the move file is not the
     * authoritative source for the metadata Bermuda presents to the user.
     */
    let catalogue = project.catalogue().map_err(|error| error.to_string())?;
    let game = catalogue.get(game_id).map_err(|error| error.to_string())?;

    let store = project.game_store().map_err(|error| error.to_string())?;

    let positions = store
        .positions(game_id)
        .map_err(|error| error.to_string())?;

    Ok(LoadedDocument {
        description: format!("game {game_id}"),
        positions,
        source_comments: Vec::new(),
        study_tree: None,
        study: StudyDocumentState::new(StudyOrigin::ProjectGame {
            project_path: project_path.to_owned(),
            game_id,
        }),
        editable: false,
        playable: false,
        finished: false,
        result: game.result,
        black_player: game.black_player,
        white_player: game.white_player,
        komi: game.komi,
        played_source_locator: None,
    })
}

fn load_sgf_document(sgf_path: &str) -> Result<LoadedDocument, String> {
    let path = Path::new(sgf_path);

    let bytes = fs::read(path).map_err(|error| format!("reading {}: {error}", path.display()))?;

    let collection =
        parse_collection(&bytes).map_err(|error| format!("parsing {}: {error}", path.display()))?;

    let study = study_document_state_from_collection(&collection, path)?;

    let source_comments = main_variation_comments(&collection);

    let study_tree = build_study_tree(&collection).map_err(|error| {
        format!(
            "building the Study game tree from {}: {error}",
            path.display()
        )
    })?;

    let record = extract_main_variation(&collection).map_err(|error| {
        format!(
            "extracting the main variation from {}: {error}",
            path.display()
        )
    })?;

    let positions = replay_positions(&record)
        .map_err(|error| format!("replaying {}: {error}", path.display()))?;

    if study_tree.main_path.len() != positions.len() {
        return Err(format!(
            "Study tree for {} has {} main-line positions, but replay has {}",
            path.display(),
            study_tree.main_path.len(),
            positions.len(),
        ));
    }

    Ok(LoadedDocument {
        description: format!("SGF {}", path.display()),
        positions,
        source_comments,
        study_tree: Some(study_tree),
        study,
        editable: false,
        playable: false,
        finished: false,
        result: record.metadata.result.clone(),
        black_player: record.metadata.black_player.clone(),
        white_player: record.metadata.white_player.clone(),
        komi: record.metadata.komi,
        played_source_locator: None,
    })
}

fn load_joseki_study_document(sgf_path: &str, node_id: &str) -> Result<LoadedDocument, String> {
    let mut document = load_sgf_document(sgf_path)?;

    let description = format!("OGS Joseki Explorer · position {node_id}");

    document.description = description.clone();
    document.study.metadata = StudyDocumentMetadata::new(StudyOrigin::Detached { description });
    document.study.library_path = None;

    Ok(document)
}

fn new_position_document(board_size: i32) -> Result<LoadedDocument, String> {
    let board_size =
        u8::try_from(board_size).map_err(|_| format!("invalid board size {board_size}"))?;

    let board = Board::new(board_size).map_err(|error| error.to_string())?;

    Ok(LoadedDocument {
        description: "untitled position".to_owned(),
        positions: vec![editable_position_state(board)],
        source_comments: Vec::new(),
        study_tree: None,
        study: StudyDocumentState::new(StudyOrigin::Detached {
            description: "untitled position".to_owned(),
        }),
        editable: true,
        playable: false,
        finished: false,
        result: None,
        black_player: None,
        white_player: None,
        komi: None,
        played_source_locator: None,
    })
}

fn new_game_document(
    board_size: i32,
    black_player: Option<String>,
    white_player: Option<String>,
    komi: f32,
) -> Result<LoadedDocument, String> {
    let board_size =
        u8::try_from(board_size).map_err(|_| format!("invalid board size {board_size}"))?;

    let board = Board::new(board_size).map_err(|error| error.to_string())?;

    Ok(LoadedDocument {
        description: "untitled game".to_owned(),
        positions: vec![editable_position_state(board)],
        source_comments: Vec::new(),
        study_tree: None,
        study: StudyDocumentState::new(StudyOrigin::Detached {
            description: "untitled game".to_owned(),
        }),
        editable: false,
        playable: true,
        finished: false,
        result: None,
        black_player,
        white_player,
        komi: Some(komi),
        played_source_locator: Some(new_played_source_locator()),
    })
}

fn optional_player_name(name: &str) -> Option<String> {
    let name = name.trim();

    if name.is_empty() {
        None
    } else {
        Some(name.to_owned())
    }
}

fn editable_position_state(board: Board) -> PositionState {
    let side_to_move = Colour::Black;

    let occurrence = PositionOccurrence {
        move_number: 0,
        side_to_move,
        ko_point: board.ko_point(),
        fingerprint: position_fingerprint(&board, side_to_move),
    };

    PositionState {
        board,
        occurrence,
        last_move: None,
    }
}

fn new_played_source_locator() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();

    let sequence = PLAYED_GAME_LOCATOR_SEQUENCE.fetch_add(1, Ordering::Relaxed);

    format!("played:{nanos}:{}:{sequence}", std::process::id())
}

fn katago_environment_path(variable: &str) -> Result<Option<PathBuf>, String> {
    let Some(value) = env::var_os(variable) else {
        return Ok(None);
    };

    if value.is_empty() {
        return Err(format!("{variable} is empty"));
    }

    Ok(Some(PathBuf::from(value)))
}

fn validate_katago_file(path: PathBuf, source: &str) -> Result<PathBuf, String> {
    if !path.is_file() {
        return Err(format!("{source} does not name a file: {}", path.display()));
    }

    Ok(path)
}

fn katago_executable_path(saved_value: &QString) -> Result<PathBuf, String> {
    if let Some(path) = katago_environment_path("BERMUDA_KATAGO_EXECUTABLE")? {
        return validate_katago_file(path, "BERMUDA_KATAGO_EXECUTABLE");
    }

    let saved = saved_value.to_string();

    if saved.trim().is_empty() {
        /*
         * A bare command name is intentional here.  Process startup will
         * resolve it through PATH in the normal platform-specific way.
         */
        return Ok(PathBuf::from("katago"));
    }

    validate_katago_file(PathBuf::from(saved), "saved KataGo executable")
}

fn configured_katago_file(
    variable: &str,
    saved_value: &QString,
    description: &str,
) -> Result<PathBuf, String> {
    if let Some(path) = katago_environment_path(variable)? {
        return validate_katago_file(path, variable);
    }

    let saved = saved_value.to_string();

    if saved.trim().is_empty() {
        return Err(format!("KataGo {description} is not configured"));
    }

    validate_katago_file(PathBuf::from(saved), &format!("saved KataGo {description}"))
}

fn katago_working_directory() -> Result<PathBuf, String> {
    let project_dirs = ProjectDirs::from("org", "Bermuda", "Bermuda")
        .ok_or_else(|| "could not determine the per-user Bermuda data directory".to_owned())?;

    Ok(project_dirs.data_local_dir().join("katago"))
}

fn analysis_vertex_name(vertex: &AnalysisVertex, board_size: u8) -> Result<String, String> {
    match vertex {
        AnalysisVertex::Pass => Ok("Pass".to_owned()),

        AnalysisVertex::Point(point) => {
            let board = Board::new(board_size).map_err(|error| error.to_string())?;

            let core_point = board
                .point(point.x, point.y)
                .map_err(|error| error.to_string())?;

            board
                .point_name(core_point)
                .map_err(|error| error.to_string())
        }
    }
}

fn colour_name(colour: Colour) -> &'static str {
    match colour {
        Colour::Black => "Black",
        Colour::White => "White",
    }
}

fn personal_project_root() -> Result<PathBuf, String> {
    let project_dirs = ProjectDirs::from("org", "Bermuda", "Bermuda")
        .ok_or_else(|| "could not determine the per-user Bermuda data directory".to_owned())?;

    Ok(project_dirs
        .data_local_dir()
        .join(PERSONAL_PROJECT_DIRECTORY))
}

fn open_or_create_personal_project() -> Result<Project, String> {
    let root = personal_project_root()?;
    let manager = ProjectManager::new();

    if root.exists() {
        manager
            .open(&root)
            .map_err(|error| format!("opening My Games at {}: {error}", root.display()))
    } else {
        manager
            .create(PERSONAL_PROJECT_NAME, &root)
            .map_err(|error| format!("creating My Games at {}: {error}", root.display()))
    }
}

fn add_played_document_to_project(
    document: &LoadedDocument,
    project: &Project,
) -> Result<i64, String> {
    if !document.playable {
        return Err("the loaded document is not a game being played".to_owned());
    }

    /*
     * The first implementation deliberately adds only completed games.
     * Correct support for an unfinished game requires update semantics.
     */
    if !document.finished {
        return Err("finish the game before adding it to My Games".to_owned());
    }

    let source_locator = document
        .played_source_locator
        .as_deref()
        .ok_or_else(|| "the played game has no provenance identifier".to_owned())?;

    let record = played_game_record(document)?;

    let mut importer = project
        .importer()
        .map_err(|error| format!("opening My Games importer: {error}"))?;

    let outcome = importer
        .import_record(
            PLAYED_GAME_SOURCE_NAME,
            PLAYED_GAME_SOURCE_VERSION,
            source_locator,
            &record,
        )
        .map_err(|error| format!("adding game to My Games: {error}"))?;

    let game_id = match outcome {
        ImportOutcome::Imported { game_id, .. } => game_id,

        ImportOutcome::AddedSource { game_id } => game_id,

        ImportOutcome::AlreadyImported { game_id } => game_id,

        ImportOutcome::SkippedBoardSize { board_size } => {
            return Err(format!(
                "My Games does not yet support {board_size}x{board_size} games"
            ));
        }
    };

    /*
     * Release the importer's database connection before opening the
     * position indexer.
     */
    drop(importer);

    /*
     * A newly added personal game should be searchable immediately.
     */
    let mut indexer = project
        .position_indexer()
        .map_err(|error| format!("opening My Games position index: {error}"))?;

    indexer
        .index_game_by_id(game_id, POSITION_INDEX_VERSION)
        .map_err(|error| format!("indexing My Games game {game_id}: {error}"))?;

    Ok(game_id)
}

fn add_played_document_to_my_games(document: &LoadedDocument) -> Result<i64, String> {
    let project = open_or_create_personal_project()?;

    add_played_document_to_project(document, &project)
}

fn played_game_record(document: &LoadedDocument) -> Result<GameRecord, String> {
    if !document.playable {
        return Err("the loaded document is not a game being played".to_owned());
    }

    let initial = document
        .positions
        .first()
        .ok_or_else(|| "the game has no initial position".to_owned())?;

    if initial.last_move.is_some() {
        return Err("the initial game position unexpectedly has a last move".to_owned());
    }

    let mut moves = Vec::with_capacity(document.positions.len().saturating_sub(1));

    for (index, position) in document.positions.iter().enumerate().skip(1) {
        let mv = position
            .last_move
            .ok_or_else(|| format!("game position {index} has no recorded move"))?;

        moves.push(mv);
    }

    Ok(GameRecord {
        board_size: initial.board.size(),

        metadata: Metadata {
            black_player: document.black_player.clone(),
            white_player: document.white_player.clone(),
            date: None,
            event: None,
            result: document.result.clone(),
            komi: document.komi,
            handicap: None,
        },

        /*
         * Play Game currently starts from an empty board.
         *
         * When handicap/setup play is added, the initial board will
         * be converted into SetupStone entries here.
         */
        setup: Vec::new(),

        moves,
    })
}

fn save_played_document_sgf(document: &LoadedDocument, sgf_path: &str) -> Result<(), String> {
    let sgf_path = sgf_path.trim();

    if sgf_path.is_empty() {
        return Err("no SGF filename was selected".to_owned());
    }

    let record = played_game_record(document)?;

    let sgf = write_game_record_sgf(&record).map_err(|error| format!("creating SGF: {error}"))?;

    let path = Path::new(sgf_path);

    fs::write(path, sgf).map_err(|error| format!("writing {}: {error}", path.display()))?;

    Ok(())
}

fn play_document_point(document: &mut LoadedDocument, x: i32, y: i32) -> Result<i32, String> {
    if !document.playable {
        return Err("the loaded document is not a game being played".to_owned());
    }

    let current = document
        .positions
        .last()
        .ok_or_else(|| "the game has no initial position".to_owned())?;

    let x = u8::try_from(x).map_err(|_| format!("invalid board coordinate {x},{y}"))?;

    let qml_y = u8::try_from(y).map_err(|_| format!("invalid board coordinate {x},{y}"))?;

    let core_y = qml_y_to_core(current.board.size(), qml_y)?;

    let point = current
        .board
        .point(x, core_y)
        .map_err(|error| error.to_string())?;

    let mv = Move {
        colour: current.occurrence.side_to_move,
        point: Some(point),
    };

    append_game_move(document, mv)
}

fn play_document_pass(document: &mut LoadedDocument) -> Result<i32, String> {
    if !document.playable {
        return Err("the loaded document is not a game being played".to_owned());
    }

    let colour = document
        .positions
        .last()
        .ok_or_else(|| "the game has no initial position".to_owned())?
        .occurrence
        .side_to_move;

    append_game_move(
        document,
        Move {
            colour,
            point: None,
        },
    )
}

fn append_game_move(document: &mut LoadedDocument, mv: Move) -> Result<i32, String> {
    if !document.playable {
        return Err("the loaded document is not a game being played".to_owned());
    }

    if document.finished {
        return Err("the game has already finished".to_owned());
    }

    let current = document
        .positions
        .last()
        .cloned()
        .ok_or_else(|| "the game has no initial position".to_owned())?;

    let mut board = current.board.clone();

    board.play(mv).map_err(|error| error.to_string())?;

    let move_number = current
        .occurrence
        .move_number
        .checked_add(1)
        .ok_or_else(|| "move number overflow".to_owned())?;

    let side_to_move = mv.colour.opponent();

    let occurrence = PositionOccurrence {
        move_number,
        side_to_move,
        ko_point: board.ko_point(),
        fingerprint: position_fingerprint(&board, side_to_move),
    };

    document.positions.push(PositionState {
        board,
        occurrence,
        last_move: Some(mv),
    });

    i32::try_from(move_number)
        .map_err(|_| "move number is too large for the Qt interface".to_owned())
}

fn undo_document_move(document: &mut LoadedDocument) -> Result<i32, String> {
    if !document.playable {
        return Err("the loaded document is not a game being played".to_owned());
    }

    if document.finished {
        return Err("the game has already finished".to_owned());
    }

    if document.positions.len() <= 1 {
        return Err("there are no moves to undo".to_owned());
    }

    document.positions.pop();

    let move_number = document
        .positions
        .last()
        .ok_or_else(|| "the game has no initial position".to_owned())?
        .occurrence
        .move_number;

    i32::try_from(move_number)
        .map_err(|_| "move number is too large for the Qt interface".to_owned())
}

fn resign_document_game(document: &mut LoadedDocument) -> Result<String, String> {
    if !document.playable {
        return Err("the loaded document is not a game being played".to_owned());
    }

    if document.finished {
        return Err("the game has already finished".to_owned());
    }

    let side_to_move = document
        .positions
        .last()
        .ok_or_else(|| "the game has no initial position".to_owned())?
        .occurrence
        .side_to_move;

    /*
     * The player whose turn it is resigns.
     */
    let result = match side_to_move {
        Colour::Black => "W+R",
        Colour::White => "B+R",
    }
    .to_owned();

    document.finished = true;
    document.result = Some(result.clone());

    Ok(result)
}

fn finish_document_game(document: &mut LoadedDocument, result: &str) -> Result<(), String> {
    if !document.playable {
        return Err("the loaded document is not a game being played".to_owned());
    }

    if document.finished {
        return Err("the game has already finished".to_owned());
    }

    let result = result.trim();

    if result.is_empty() {
        return Err("enter a result, for example B+3.5, W+0.5 or 0".to_owned());
    }

    document.finished = true;
    document.result = Some(result.to_owned());

    Ok(())
}

fn edit_document_position(
    document: &mut LoadedDocument,
    x: i32,
    y: i32,
    tool: &str,
) -> Result<(), String> {
    if !document.editable {
        return Err("the loaded document is read-only".to_owned());
    }

    let position = document
        .positions
        .first_mut()
        .ok_or_else(|| "the editable position is missing".to_owned())?;

    let x = u8::try_from(x).map_err(|_| format!("invalid board coordinate {x},{y}"))?;

    let qml_y = u8::try_from(y).map_err(|_| format!("invalid board coordinate {x},{y}"))?;

    let core_y = qml_y_to_core(position.board.size(), qml_y)?;

    let point = position
        .board
        .point(x, core_y)
        .map_err(|error| error.to_string())?;

    match tool {
        "black" => position
            .board
            .set_setup(Colour::Black, point)
            .map_err(|error| error.to_string())?,

        "white" => position
            .board
            .set_setup(Colour::White, point)
            .map_err(|error| error.to_string())?,

        "erase" => position
            .board
            .clear_setup(point)
            .map_err(|error| error.to_string())?,

        _ => {
            return Err(format!("unknown position-editing tool {tool:?}"));
        }
    }

    position.occurrence.ko_point = position.board.ko_point();

    position.occurrence.fingerprint =
        position_fingerprint(&position.board, position.occurrence.side_to_move);

    position.last_move = None;

    Ok(())
}

fn position_data(
    positions: &[PositionState],
    source_comments: &[String],
    document_description: &str,
    move_number: i32,
) -> Result<LoadedPosition, String> {
    let requested_move =
        usize::try_from(move_number).map_err(|_| "move number cannot be negative".to_owned())?;

    let move_count_usize = positions.len().saturating_sub(1);

    let position = positions.get(requested_move).ok_or_else(|| {
        format!(
            "requested move {move_number}, but {document_description} contains only \
     {move_count_usize} moves"
        )
    })?;

    let source_comment = source_comments
        .get(requested_move)
        .map(String::as_str)
        .unwrap_or("");

    let has_source_comments = source_comments.iter().any(|comment| !comment.is_empty());

    let board_size = i32::from(position.board.size());

    let (last_move_x, last_move_y) = match position.last_move.and_then(|mv| mv.point) {
        Some(point) => {
            let size = u16::from(position.board.size());

            let core_y = point / size;
            let qml_y = core_y_to_qml(size, core_y);

            (i32::from(point % size), i32::from(qml_y))
        }

        None => (-1, -1),
    };

    let current_move = i32::try_from(position.occurrence.move_number)
        .map_err(|_| "move number is too large for the Qt interface".to_owned())?;

    let move_count = i32::try_from(move_count_usize)
        .map_err(|_| "game contains too many moves for the Qt interface".to_owned())?;

    Ok(LoadedPosition {
        board_size,
        stones_json: board_stones_json(&position.board),
        source_comment: QString::from(source_comment),
        has_source_comments,
        move_number: current_move,
        move_count,
        last_move_x,
        last_move_y,
    })
}

fn qml_y_to_core(board_size: u8, qml_y: u8) -> Result<u8, String> {
    if qml_y >= board_size {
        return Err(format!(
            "board y-coordinate {qml_y} lies outside a {board_size}×{board_size} board"
        ));
    }

    Ok(board_size - 1 - qml_y)
}

fn core_y_to_qml(board_size: u16, core_y: u16) -> u16 {
    debug_assert!(core_y < board_size);
    board_size - 1 - core_y
}

fn board_stones_json(board: &Board) -> QString {
    let size = u16::from(board.size());
    let point_count = size * size;

    let mut json = String::from("[");
    let mut first = true;

    for point in 0..point_count {
        let Some(colour) = board.colour_at(point) else {
            continue;
        };

        if !first {
            json.push(',');
        }

        first = false;

        let x = point % size;
        let core_y = point / size;
        let y = core_y_to_qml(size, core_y);

        let colour_name = match colour {
            Colour::Black => "black",
            Colour::White => "white",
        };

        write!(json, r#"{{"x":{x},"y":{y},"color":"{colour_name}"}}"#)
            .expect("writing JSON to a String cannot fail");
    }

    json.push(']');

    QString::from(json)
} // closes board_stones_json()

#[cfg(test)]
mod played_game_record_tests {
    use super::*;

    #[test]
    fn replaces_aggregated_sgf_comment_without_touching_other_properties() {
        let mut collection =
            parse_collection(b"(;FF[4]GM[1]SZ[19];B[dd]C[first];C[second]TR[pp];W[qq])")
                .expect("parse SGF");

        let tree = build_study_tree(&collection).expect("build Study tree");

        let node = &tree.nodes[1];

        assert_eq!(node.comment, "first\n\nsecond");
        assert_eq!(node.comment_sources.len(), 2);

        replace_collection_comment(
            &mut collection,
            &node.source,
            &node.comment_sources,
            "edited comment",
        )
        .expect("replace comment");

        let rebuilt = build_study_tree(&collection).expect("rebuild Study tree");

        assert_eq!(rebuilt.nodes[1].comment, "edited comment");

        /*
         * The comment-only node remains because it also carries TR.
         * Only its C property was removed.
         */
        assert_eq!(collection.trees[0].sequence[2].first("TR"), Some("pp"));
        assert_eq!(collection.trees[0].sequence[2].first("C"), None);
    }

    #[test]
    fn converts_live_game_to_core_game_record() {
        let mut document = new_game_document(
            19,
            Some("Black Player".to_owned()),
            Some("White Player".to_owned()),
            6.5,
        )
        .expect("create live game");

        let first_point = document.positions[0]
            .board
            .point(3, 3)
            .expect("board point");

        append_game_move(
            &mut document,
            Move {
                colour: Colour::Black,
                point: Some(first_point),
            },
        )
        .expect("play black move");

        append_game_move(
            &mut document,
            Move {
                colour: Colour::White,
                point: None,
            },
        )
        .expect("play white pass");

        document.finished = true;
        document.result = Some("B+R".to_owned());

        let record = played_game_record(&document).expect("convert live game");

        assert_eq!(record.board_size, 19);

        assert_eq!(
            record.metadata.black_player.as_deref(),
            Some("Black Player"),
        );

        assert_eq!(
            record.metadata.white_player.as_deref(),
            Some("White Player"),
        );

        assert_eq!(record.metadata.komi, Some(6.5));
        assert_eq!(record.metadata.result.as_deref(), Some("B+R"),);

        assert_eq!(record.metadata.date, None);
        assert_eq!(record.metadata.event, None);
        assert_eq!(record.metadata.handicap, None);

        assert!(record.setup.is_empty());
        assert_eq!(record.moves.len(), 2);

        assert_eq!(
            record.moves[0],
            Move {
                colour: Colour::Black,
                point: Some(first_point),
            },
        );

        assert_eq!(
            record.moves[1],
            Move {
                colour: Colour::White,
                point: None,
            },
        );
    }

    #[test]
    fn converts_unfinished_live_game() {
        let document =
            new_game_document(19, Some("Black".to_owned()), Some("White".to_owned()), 6.5)
                .expect("create live game");

        let record = played_game_record(&document).expect("convert live game");

        assert!(record.metadata.result.is_none());
        assert!(record.moves.is_empty());
    }

    #[test]
    fn rejects_non_playable_document() {
        let document = new_position_document(19).expect("create position");

        let error = played_game_record(&document).expect_err("position is not a played game");

        assert_eq!(error, "the loaded document is not a game being played",);
    }

    #[test]
    fn played_game_source_locators_are_unique() {
        let first = new_game_document(19, Some("Black".to_owned()), Some("White".to_owned()), 6.5)
            .expect("create first game");

        let second = new_game_document(19, Some("Black".to_owned()), Some("White".to_owned()), 6.5)
            .expect("create second game");

        assert_ne!(first.played_source_locator, second.played_source_locator,);
    }

    #[test]
    fn adds_finished_game_once_and_indexes_it() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();

        let root =
            std::env::temp_dir().join(format!("bermuda-my-games-{unique}-{}", std::process::id()));

        let manager = ProjectManager::new();

        let project = manager
            .create("My Games Test", &root)
            .expect("create personal project");

        let mut document =
            new_game_document(19, Some("Black".to_owned()), Some("White".to_owned()), 6.5)
                .expect("create live game");

        let point = document.positions[0]
            .board
            .point(3, 3)
            .expect("board point");

        append_game_move(
            &mut document,
            Move {
                colour: Colour::Black,
                point: Some(point),
            },
        )
        .expect("play move");

        document.finished = true;
        document.result = Some("B+R".to_owned());

        let first_game_id =
            add_played_document_to_project(&document, &project).expect("first Add to My Games");

        /*
         * Same session, same source locator: this must be idempotent.
         */
        let second_game_id =
            add_played_document_to_project(&document, &project).expect("second Add to My Games");

        assert_eq!(first_game_id, second_game_id,);

        let connection =
            bermuda::database::open(&project.database_root()).expect("open personal database");

        let game_count: i64 = connection
            .query_row("SELECT COUNT(*) FROM games", [], |row| row.get(0))
            .expect("count games");

        let source_count: i64 = connection
            .query_row("SELECT COUNT(*) FROM game_sources", [], |row| row.get(0))
            .expect("count sources");

        let indexed_count: i64 = connection
            .query_row("SELECT COUNT(*) FROM indexed_games", [], |row| row.get(0))
            .expect("count indexed games");

        let position_count: i64 = connection
            .query_row("SELECT COUNT(*) FROM exact_positions", [], |row| row.get(0))
            .expect("count indexed positions");

        assert_eq!(game_count, 1);
        assert_eq!(source_count, 1);
        assert_eq!(indexed_count, 1);

        /*
         * Initial position + one move.
         */
        assert_eq!(position_count, 2);

        drop(connection);

        fs::remove_dir_all(&root).expect("remove personal test project");
    }

    #[test]
    fn refuses_to_add_unfinished_live_game() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();

        let root = std::env::temp_dir().join(format!(
            "bermuda-my-games-unfinished-{unique}-{}",
            std::process::id()
        ));

        let manager = ProjectManager::new();

        let project = manager
            .create("My Games Test", &root)
            .expect("create personal project");

        let document =
            new_game_document(19, Some("Black".to_owned()), Some("White".to_owned()), 6.5)
                .expect("create live game");

        let error = add_played_document_to_project(&document, &project)
            .expect_err("unfinished game must not be added");

        assert_eq!(error, "finish the game before adding it to My Games");

        fs::remove_dir_all(&root).expect("remove personal test project");
    }
}
