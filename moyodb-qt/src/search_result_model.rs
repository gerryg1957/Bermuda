#![allow(clippy::too_many_arguments)]
use cxx_qt::{CxxQtType, Threading};

use std::{
    fmt::Display,
    path::Path,
    pin::Pin,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

use cxx_qt_lib::{QByteArray, QHash, QHashPair_i32_QByteArray, QModelIndex, QString, QVariant};

use moyodb::{
    Board, Colour, Pattern, PatternRect, PatternSearchProgress, PatternSearchQuery,
    PatternSearchScope, SearchEngine, SearchOccurrence, SearchPatternOutcome, SearchResult,
    project_manager::ProjectManager,
};

#[allow(non_camel_case_types)]
type QHash_i32_QByteArray = QHash<QHashPair_i32_QByteArray>;

const GAME_ID_ROLE: i32 = 0x0100;
const BLACK_PLAYER_ROLE: i32 = GAME_ID_ROLE + 1;
const WHITE_PLAYER_ROLE: i32 = GAME_ID_ROLE + 2;
const PLAYED_DATE_ROLE: i32 = GAME_ID_ROLE + 3;
const RESULT_ROLE: i32 = GAME_ID_ROLE + 4;
const EVENT_ROLE: i32 = GAME_ID_ROLE + 5;
const KOMI_ROLE: i32 = GAME_ID_ROLE + 6;
const MATCH_COUNT_ROLE: i32 = GAME_ID_ROLE + 7;
const FIRST_MATCH_MOVE_ROLE: i32 = GAME_ID_ROLE + 8;
const FIRST_MATCH_LEFT_ROLE: i32 = GAME_ID_ROLE + 9;
const FIRST_MATCH_BOTTOM_ROLE: i32 = GAME_ID_ROLE + 10;

#[derive(Clone, Debug, Default)]
struct SearchResultRow {
    game_id: i64,
    black_player: QString,
    white_player: QString,
    played_date: QString,
    result: QString,
    event: QString,
    komi: QString,
    match_count: i32,
    first_match_move: i32,
    first_match_left: i32,
    first_match_bottom: i32,
    occurrences: Vec<SearchOccurrence>,
}

#[derive(Default)]
pub struct SearchResultModelRust {
    rows: Vec<SearchResultRow>,

    pub(crate) error_message: QString,
    pub(crate) total_occurrences: i32,

    pub(crate) search_in_progress: bool,
    pub(crate) cancel_requested: bool,
    pub(crate) search_cancelled: bool,

    pub(crate) games_examined: i32,
    pub(crate) total_games: i32,
    pub(crate) matching_games: i32,
    pub(crate) matches_found: i32,

    cancel_token: Option<Arc<AtomicBool>>,
    search_id: u64,
}

enum BackgroundSearchResult {
    Completed(Vec<SearchResult>),
    Cancelled,
    Failed(String),
}

impl crate::game_list_model::ffi::SearchResultModel {
    pub(crate) fn row_count(&self, parent: &QModelIndex) -> i32 {
        if parent.is_valid() {
            return 0;
        }

        i32::try_from(self.rust().rows.len()).unwrap_or(i32::MAX)
    }

    pub(crate) fn data(&self, index: &QModelIndex, role: i32) -> QVariant {
        if !index.is_valid() {
            return QVariant::default();
        }

        let row_number = index.row();

        if row_number < 0 {
            return QVariant::default();
        }

        let Some(row) = self.rust().rows.get(row_number as usize) else {
            return QVariant::default();
        };

        match role {
            GAME_ID_ROLE => QVariant::from(&row.game_id),
            BLACK_PLAYER_ROLE => QVariant::from(&row.black_player),
            WHITE_PLAYER_ROLE => QVariant::from(&row.white_player),
            PLAYED_DATE_ROLE => QVariant::from(&row.played_date),
            RESULT_ROLE => QVariant::from(&row.result),
            EVENT_ROLE => QVariant::from(&row.event),
            KOMI_ROLE => QVariant::from(&row.komi),
            MATCH_COUNT_ROLE => QVariant::from(&row.match_count),
            FIRST_MATCH_MOVE_ROLE => QVariant::from(&row.first_match_move),
            FIRST_MATCH_LEFT_ROLE => QVariant::from(&row.first_match_left),
            FIRST_MATCH_BOTTOM_ROLE => QVariant::from(&row.first_match_bottom),
            _ => QVariant::default(),
        }
    }

    pub(crate) fn role_names(&self) -> QHash_i32_QByteArray {
        let mut roles = QHash_i32_QByteArray::default();

        roles.insert(GAME_ID_ROLE, QByteArray::from("gameId"));
        roles.insert(BLACK_PLAYER_ROLE, QByteArray::from("blackPlayer"));
        roles.insert(WHITE_PLAYER_ROLE, QByteArray::from("whitePlayer"));
        roles.insert(PLAYED_DATE_ROLE, QByteArray::from("playedDate"));
        roles.insert(RESULT_ROLE, QByteArray::from("result"));
        roles.insert(EVENT_ROLE, QByteArray::from("event"));
        roles.insert(KOMI_ROLE, QByteArray::from("komi"));
        roles.insert(MATCH_COUNT_ROLE, QByteArray::from("matchCount"));
        roles.insert(FIRST_MATCH_MOVE_ROLE, QByteArray::from("firstMatchMove"));
        roles.insert(FIRST_MATCH_LEFT_ROLE, QByteArray::from("firstMatchLeft"));
        roles.insert(
            FIRST_MATCH_BOTTOM_ROLE,
            QByteArray::from("firstMatchBottom"),
        );

        roles
    }

    pub(crate) fn occurrences_json(&self, row_number: i32) -> QString {
        if row_number < 0 {
            return QString::from("[]");
        }

        let Some(row) = self.rust().rows.get(row_number as usize) else {
            return QString::from("[]");
        };

        let occurrences = row
            .occurrences
            .iter()
            .map(|occurrence| SearchOccurrenceJson {
                move_number: i32::try_from(occurrence.move_number).unwrap_or(i32::MAX),

                left: occurrence.left.map_or(-1, i32::from),

                bottom: occurrence.bottom.map_or(-1, i32::from),
            })
            .collect::<Vec<_>>();

        match serde_json::to_string(&occurrences) {
            Ok(json) => QString::from(json),
            Err(_) => QString::from("[]"),
        }
    }

    pub(crate) fn search_project(
        mut self: Pin<&mut Self>,
        project_path: &QString,
        board_size: i32,
        stones_json: &QString,
        left: i32,
        bottom: i32,
        width: i32,
        height: i32,
    ) -> bool {
        if self.as_ref().get_ref().rust().search_in_progress {
            self.as_mut()
                .set_error_message(QString::from("a pattern search is already running"));

            return false;
        }

        let project_path = project_path.to_string();
        let stones_json = stones_json.to_string();

        let cancel_token = Arc::new(AtomicBool::new(false));

        let search_id;

        self.as_mut().begin_reset_model();

        {
            let mut rust = self.as_mut().rust_mut();

            rust.rows.clear();
            rust.cancel_token = Some(Arc::clone(&cancel_token));

            rust.search_id = rust.search_id.wrapping_add(1);

            search_id = rust.search_id;
        }

        self.as_mut().end_reset_model();

        self.as_mut().set_error_message(QString::default());

        self.as_mut().set_total_occurrences(0);
        self.as_mut().set_games_examined(0);
        self.as_mut().set_total_games(0);
        self.as_mut().set_matching_games(0);
        self.as_mut().set_matches_found(0);

        self.as_mut().set_cancel_requested(false);
        self.as_mut().set_search_cancelled(false);
        self.as_mut().set_search_in_progress(true);

        let qt_thread = self.qt_thread();
        let progress_thread = qt_thread.clone();

        std::thread::spawn(move || {
            let mut last_progress_update: Option<Instant> = None;

            let outcome = create_search_outcome(
                &project_path,
                board_size,
                &stones_json,
                left,
                bottom,
                width,
                height,
                || cancel_token.load(Ordering::Relaxed),
                |progress| {
                    let now = Instant::now();

                    let should_send = progress.games_examined == 0
                        || progress.games_examined == progress.total_games
                        || last_progress_update.is_none_or(|previous| {
                            now.duration_since(previous) >= Duration::from_millis(100)
                        });

                    if !should_send {
                        return;
                    }

                    last_progress_update = Some(now);

                    let games_examined = count_to_i32(progress.games_examined);

                    let total_games = count_to_i32(progress.total_games);

                    let matching_games = count_to_i32(progress.matching_games);

                    let matches_found = count_to_i32(progress.matches_found);

                    progress_thread
                        .queue(move |mut model| {
                            if !is_current_search(model.as_ref().get_ref(), search_id) {
                                return;
                            }

                            model.as_mut().set_games_examined(games_examined);

                            model.as_mut().set_total_games(total_games);

                            model.as_mut().set_matching_games(matching_games);

                            model.as_mut().set_matches_found(matches_found);
                        })
                        .ok();
                },
            );

            let completion = match outcome {
                Ok(SearchPatternOutcome::Completed(results)) => {
                    BackgroundSearchResult::Completed(results)
                }

                Ok(SearchPatternOutcome::Cancelled) => BackgroundSearchResult::Cancelled,

                Err(error) => BackgroundSearchResult::Failed(error),
            };

            qt_thread
                .queue(move |model| {
                    finish_search(model, search_id, completion);
                })
                .ok();
        });

        true
    }

    pub(crate) fn cancel_search(mut self: Pin<&mut Self>) {
        let cancel_token = self.as_ref().get_ref().rust().cancel_token.clone();

        if let Some(cancel_token) = cancel_token {
            cancel_token.store(true, Ordering::Relaxed);

            self.as_mut().set_cancel_requested(true);
        }
    }

    pub(crate) fn clear_results(mut self: Pin<&mut Self>) {
        let cancel_token;

        self.as_mut().begin_reset_model();

        {
            let mut rust = self.as_mut().rust_mut();

            cancel_token = rust.cancel_token.take();

            rust.search_id = rust.search_id.wrapping_add(1);

            rust.rows.clear();
        }

        self.as_mut().end_reset_model();

        if let Some(cancel_token) = cancel_token {
            cancel_token.store(true, Ordering::Relaxed);
        }

        self.as_mut().set_error_message(QString::default());

        self.as_mut().set_total_occurrences(0);
        self.as_mut().set_games_examined(0);
        self.as_mut().set_total_games(0);
        self.as_mut().set_matching_games(0);
        self.as_mut().set_matches_found(0);

        self.as_mut().set_cancel_requested(false);
        self.as_mut().set_search_cancelled(false);
        self.as_mut().set_search_in_progress(false);
    }
}

fn create_search_outcome<C, P>(
    project_path: &str,
    board_size: i32,
    stones_json: &str,
    left: i32,
    bottom: i32,
    width: i32,
    height: i32,
    is_cancelled: C,
    on_progress: P,
) -> Result<SearchPatternOutcome, String>
where
    C: FnMut() -> bool,
    P: FnMut(PatternSearchProgress),
{
    if project_path.trim().is_empty() {
        return Err("no project is selected".to_owned());
    }

    let rect = PatternRect {
        left: coordinate_value("left", left)?,
        bottom: coordinate_value("bottom", bottom)?,
        width: dimension_value("width", width)?,
        height: dimension_value("height", height)?,
    };

    let project = ProjectManager::new()
        .open(Path::new(project_path))
        .map_err(|error| error.to_string())?;

    let board = board_from_json(board_size, stones_json)?;

    let pattern = Pattern::extract(&board, rect).map_err(|error| error.to_string())?;

    let query = PatternSearchQuery {
        pattern,
        scope: PatternSearchScope::Project,
    };

    let search_engine = SearchEngine::new(&project).map_err(|error| error.to_string())?;

    search_engine
        .search_pattern_with_progress(&query, is_cancelled, on_progress)
        .map_err(|error| error.to_string())
}

fn finish_search(
    mut model: Pin<&mut crate::game_list_model::ffi::SearchResultModel>,
    search_id: u64,
    completion: BackgroundSearchResult,
) {
    if !is_current_search(model.as_ref().get_ref(), search_id) {
        return;
    }

    let (rows, error_message, cancelled) = match completion {
        BackgroundSearchResult::Completed(results) => match search_results_to_rows(results) {
            Ok(rows) => (rows, None, false),

            Err(error) => (Vec::new(), Some(error), false),
        },

        BackgroundSearchResult::Cancelled => (Vec::new(), None, true),

        BackgroundSearchResult::Failed(error) => (Vec::new(), Some(error), false),
    };

    let total_occurrences = rows
        .iter()
        .fold(0_i32, |total, row| total.saturating_add(row.match_count));

    let matching_games = count_to_i32(rows.len());

    model.as_mut().begin_reset_model();

    {
        let mut rust = model.as_mut().rust_mut();

        rust.rows = rows;
        rust.cancel_token = None;
    }

    model.as_mut().end_reset_model();

    model.as_mut().set_total_occurrences(total_occurrences);

    model.as_mut().set_matching_games(matching_games);

    model.as_mut().set_matches_found(total_occurrences);

    match error_message {
        Some(error) => {
            model.as_mut().set_error_message(QString::from(error));
        }

        None => {
            model.as_mut().set_error_message(QString::default());
        }
    }

    model.as_mut().set_search_cancelled(cancelled);
    model.as_mut().set_cancel_requested(false);
    model.as_mut().set_search_in_progress(false);
}

fn is_current_search(
    model: &crate::game_list_model::ffi::SearchResultModel,
    search_id: u64,
) -> bool {
    model.rust().search_id == search_id
}

fn search_results_to_rows(results: Vec<SearchResult>) -> Result<Vec<SearchResultRow>, String> {
    results
        .into_iter()
        .map(|result| {
            let first_occurrence = result
                .occurrences
                .first()
                .ok_or_else(|| format!("game {} has an empty search result", result.game_id,))?;

            let match_count = i32::try_from(result.occurrences.len()).unwrap_or(i32::MAX);

            let first_match_move = i32::try_from(first_occurrence.move_number).unwrap_or(i32::MAX);

            let first_match_left = first_occurrence.left.map_or(-1, i32::from);

            let first_match_bottom = first_occurrence.bottom.map_or(-1, i32::from);

            Ok(SearchResultRow {
                game_id: result.game_id,

                black_player: optional_text(&result.black_player),

                white_player: optional_text(&result.white_player),

                played_date: optional_text(&result.game_date),

                result: optional_text(&result.result),

                event: optional_text(&result.event),

                komi: optional_number(&result.komi),

                match_count,
                first_match_move,
                first_match_left,
                first_match_bottom,
                occurrences: result.occurrences,
            })
        })
        .collect()
}

#[derive(Debug, serde::Serialize)]
struct SearchOccurrenceJson {
    #[serde(rename = "move")]
    move_number: i32,
    left: i32,
    bottom: i32,
}

#[derive(Debug, serde::Deserialize)]
struct BoardStone {
    x: u8,
    y: u8,
    color: String,
}

fn board_from_json(board_size: i32, stones_json: &str) -> Result<Board, String> {
    let board_size =
        u8::try_from(board_size).map_err(|_| format!("invalid board size {board_size}"))?;

    let mut board = Board::new(board_size).map_err(|error| error.to_string())?;

    let stones: Vec<BoardStone> = serde_json::from_str(stones_json)
        .map_err(|error| format!("reading board stones: {error}"))?;

    for stone in stones {
        let colour = match stone.color.as_str() {
            "black" => Colour::Black,
            "white" => Colour::White,

            other => {
                return Err(format!("unknown stone colour {other:?}"));
            }
        };

        let core_y = qml_y_to_core(board_size, stone.y)?;

        let point = board
            .point(stone.x, core_y)
            .map_err(|error| error.to_string())?;

        board
            .set_setup(colour, point)
            .map_err(|error| error.to_string())?;
    }

    Ok(board)
}

fn qml_y_to_core(board_size: u8, qml_y: u8) -> Result<u8, String> {
    if qml_y >= board_size {
        return Err(format!(
            "board y-coordinate {qml_y} lies outside \
             a {board_size}×{board_size} board"
        ));
    }

    Ok(board_size - 1 - qml_y)
}

fn coordinate_value(name: &str, value: i32) -> Result<u8, String> {
    u8::try_from(value).map_err(|_| format!("{name} coordinate {value} is invalid"))
}

fn dimension_value(name: &str, value: i32) -> Result<u8, String> {
    let value = u8::try_from(value).map_err(|_| format!("pattern {name} {value} is invalid"))?;

    if value == 0 {
        return Err(format!("pattern {name} must be greater than zero"));
    }

    Ok(value)
}

fn count_to_i32(value: usize) -> i32 {
    i32::try_from(value).unwrap_or(i32::MAX)
}

fn optional_text(value: &Option<String>) -> QString {
    QString::from(value.as_deref().unwrap_or(""))
}

fn optional_number<T>(value: &Option<T>) -> QString
where
    T: Display,
{
    match value {
        Some(value) => QString::from(value.to_string()),
        None => QString::default(),
    }
}
