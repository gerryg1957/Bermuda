#![allow(clippy::too_many_arguments)]
use cxx_qt::{CxxQtType, Threading};

use std::{
    collections::HashMap,
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
    Board, Colour, NextMoveDistribution, NextMovePointCount, Pattern, PatternRect,
    PatternSearchOptions, PatternSearchProgress, PatternSearchQuery, PatternSearchScope,
    PatternTransformation, SearchEngine, SearchOccurrence, SearchPatternSummaryReportOutcome,
    SearchSummaryReport, SearchSummaryResult, project_manager::ProjectManager,
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
}

#[derive(Clone, Debug)]
struct StoredSearchQuery {
    project_path: String,
    board_size: i32,
    stones_json: String,
    left: i32,
    bottom: i32,
    width: i32,
    height: i32,
    include_rotations: bool,
    include_reflections: bool,
    include_reversed_colours: bool,
}

#[derive(Default)]
pub struct SearchResultModelRust {
    rows: Vec<SearchResultRow>,
    all_rows: Vec<SearchResultRow>,
    continuation_game_ids: HashMap<(i16, i16), Vec<i64>>,

    pub(crate) error_message: QString,
    pub(crate) next_move_distribution_json: QString,
    pub(crate) total_occurrences: i32,

    pub(crate) search_in_progress: bool,
    pub(crate) occurrence_load_in_progress: bool,
    pub(crate) cancel_requested: bool,
    pub(crate) search_cancelled: bool,

    pub(crate) games_examined: i32,
    pub(crate) total_games: i32,
    pub(crate) matching_games: i32,
    pub(crate) matches_found: i32,

    search_query: Option<StoredSearchQuery>,
    next_move_distribution: Option<NextMoveDistribution>,
    cancel_token: Option<Arc<AtomicBool>>,
    search_id: u64,
    occurrence_load_id: u64,
}

enum BackgroundSearchResult {
    Completed(SearchSummaryReport),
    Cancelled,
    Failed(String),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ContinuationBoardPoint {
    x: u8,
    core_y: u8,
    count: usize,
}

#[derive(Clone, Debug)]
struct LoadedSearchOccurrence {
    occurrence: SearchOccurrence,
    continuation_points: Vec<ContinuationBoardPoint>,
}

enum BackgroundOccurrenceResult {
    Completed(Vec<LoadedSearchOccurrence>),
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

    pub(crate) fn load_occurrences(mut self: Pin<&mut Self>, row_number: i32) -> bool {
        if row_number < 0 {
            return false;
        }

        let (query, next_move_distribution, game_id, occurrence_load_id) = {
            let mut rust = self.as_mut().rust_mut();

            if rust.occurrence_load_in_progress {
                return false;
            }

            let Some(row) = rust.rows.get(row_number as usize) else {
                return false;
            };

            let game_id = row.game_id;

            let Some(query) = rust.search_query.clone() else {
                return false;
            };

            let next_move_distribution = rust.next_move_distribution.clone().unwrap_or_default();

            rust.occurrence_load_id = rust.occurrence_load_id.wrapping_add(1);

            (
                query,
                next_move_distribution,
                game_id,
                rust.occurrence_load_id,
            )
        };

        self.as_mut().set_occurrence_load_in_progress(true);

        let qt_thread = self.qt_thread();

        std::thread::spawn(move || {
            let completion = match create_game_occurrences(&query, &next_move_distribution, game_id)
            {
                Ok(occurrences) => BackgroundOccurrenceResult::Completed(occurrences),

                Err(error) => BackgroundOccurrenceResult::Failed(error),
            };

            qt_thread
                .queue(move |model| {
                    finish_occurrence_load(model, occurrence_load_id, row_number, completion);
                })
                .ok();
        });

        true
    }

    pub(crate) fn filter_continuation_at_occurrence(
        mut self: Pin<&mut Self>,
        board_x: i32,
        core_y: i32,
        left: i32,
        bottom: i32,
        transformation: &QString,
    ) -> bool {
        let (pattern_width, pattern_height) = {
            let rust = self.as_ref().get_ref().rust();
            let Some(query) = rust.search_query.as_ref() else {
                return false;
            };
            let Ok(width) = u8::try_from(query.width) else {
                return false;
            };
            let Ok(height) = u8::try_from(query.height) else {
                return false;
            };
            (width, height)
        };

        let name = transformation.to_string();
        let Some(transformation) = transformation_from_name(&name) else {
            return false;
        };
        let Ok(board_x) = i16::try_from(board_x) else {
            return false;
        };
        let Ok(core_y) = i16::try_from(core_y) else {
            return false;
        };
        let Ok(left) = i16::try_from(left) else {
            return false;
        };
        let Ok(bottom) = i16::try_from(bottom) else {
            return false;
        };

        let (normalised_x, normalised_y) = transformation.inverse_relative_point(
            board_x - left,
            core_y - bottom,
            pattern_width,
            pattern_height,
        );

        let filtered_rows = {
            let rust = self.as_ref().get_ref().rust();
            let Some(game_ids) = rust
                .continuation_game_ids
                .get(&(normalised_x, normalised_y))
            else {
                return false;
            };
            rust.all_rows
                .iter()
                .filter(|row| game_ids.binary_search(&row.game_id).is_ok())
                .cloned()
                .collect::<Vec<_>>()
        };

        self.as_mut().begin_reset_model();
        self.as_mut().rust_mut().rows = filtered_rows;
        self.as_mut().end_reset_model();
        true
    }

    pub(crate) fn continuation_game_count_at_occurrence(
        self: Pin<&mut Self>,
        board_x: i32,
        core_y: i32,
        left: i32,
        bottom: i32,
        transformation: &QString,
    ) -> i32 {
        let (pattern_width, pattern_height) = {
            let rust = self.as_ref().get_ref().rust();

            let Some(query) = rust.search_query.as_ref() else {
                return 0;
            };

            let Ok(width) = u8::try_from(query.width) else {
                return 0;
            };

            let Ok(height) = u8::try_from(query.height) else {
                return 0;
            };

            (width, height)
        };

        let transformation_name = transformation.to_string();

        let Some(transformation) = transformation_from_name(&transformation_name) else {
            return 0;
        };

        let Ok(board_x) = i16::try_from(board_x) else {
            return 0;
        };

        let Ok(core_y) = i16::try_from(core_y) else {
            return 0;
        };

        let Ok(left) = i16::try_from(left) else {
            return 0;
        };

        let Ok(bottom) = i16::try_from(bottom) else {
            return 0;
        };

        let relative_x = board_x - left;
        let relative_y = core_y - bottom;

        let (normalised_x, normalised_y) = transformation.inverse_relative_point(
            relative_x,
            relative_y,
            pattern_width,
            pattern_height,
        );

        self.as_ref()
            .get_ref()
            .rust()
            .continuation_game_ids
            .get(&(normalised_x, normalised_y))
            .map_or(0, |game_ids| count_to_i32(game_ids.len()))
    }

    pub(crate) fn continuation_outcome_summary_at_occurrence(
        self: Pin<&mut Self>,
        board_x: i32,
        core_y: i32,
        left: i32,
        bottom: i32,
        transformation: &QString,
    ) -> QString {
        let rust = self.as_ref().get_ref().rust();

        let Some(query) = rust.search_query.as_ref() else {
            return QString::from("{}");
        };

        let Ok(pattern_width) = u8::try_from(query.width) else {
            return QString::from("{}");
        };

        let Ok(pattern_height) = u8::try_from(query.height) else {
            return QString::from("{}");
        };

        let transformation_name = transformation.to_string();

        let Some(transformation) = transformation_from_name(&transformation_name) else {
            return QString::from("{}");
        };

        let Ok(board_x) = i16::try_from(board_x) else {
            return QString::from("{}");
        };

        let Ok(core_y) = i16::try_from(core_y) else {
            return QString::from("{}");
        };

        let Ok(left) = i16::try_from(left) else {
            return QString::from("{}");
        };

        let Ok(bottom) = i16::try_from(bottom) else {
            return QString::from("{}");
        };

        let (normalised_x, normalised_y) = transformation.inverse_relative_point(
            board_x - left,
            core_y - bottom,
            pattern_width,
            pattern_height,
        );

        let Some(game_ids) = rust
            .continuation_game_ids
            .get(&(normalised_x, normalised_y))
        else {
            return QString::from("{}");
        };

        let mut summary = ContinuationOutcomeSummaryJson {
            games: game_ids.len(),
            black_wins: 0,
            white_wins: 0,
            draws: 0,
            unknown: game_ids.len(),
        };

        for row in &rust.all_rows {
            if game_ids.binary_search(&row.game_id).is_err() {
                continue;
            }

            match classify_game_result(&row.result.to_string()) {
                GameResultClass::BlackWin => {
                    summary.black_wins = summary.black_wins.saturating_add(1);
                    summary.unknown = summary.unknown.saturating_sub(1);
                }

                GameResultClass::WhiteWin => {
                    summary.white_wins = summary.white_wins.saturating_add(1);
                    summary.unknown = summary.unknown.saturating_sub(1);
                }

                GameResultClass::Draw => {
                    summary.draws = summary.draws.saturating_add(1);
                    summary.unknown = summary.unknown.saturating_sub(1);
                }

                GameResultClass::Unknown => {}
            }
        }

        match serde_json::to_string(&summary) {
            Ok(json) => QString::from(json),
            Err(_) => QString::from("{}"),
        }
    }

    pub(crate) fn clear_continuation_filter(mut self: Pin<&mut Self>) {
        let rows = self.as_ref().get_ref().rust().all_rows.clone();
        self.as_mut().begin_reset_model();
        self.as_mut().rust_mut().rows = rows;
        self.as_mut().end_reset_model();
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

        let stored_query = StoredSearchQuery {
            project_path: project_path.clone(),
            board_size,
            stones_json: stones_json.clone(),
            left,
            bottom,
            width,
            height,
            include_rotations: true,
            include_reflections: true,
            include_reversed_colours: true,
        };

        let cancel_token = Arc::new(AtomicBool::new(false));

        let search_id;

        self.as_mut().begin_reset_model();

        {
            let mut rust = self.as_mut().rust_mut();

            rust.rows.clear();
            rust.all_rows.clear();
            rust.continuation_game_ids.clear();
            rust.search_query = Some(stored_query);
            rust.next_move_distribution = None;
            rust.cancel_token = Some(Arc::clone(&cancel_token));

            rust.search_id = rust.search_id.wrapping_add(1);
            rust.occurrence_load_id = rust.occurrence_load_id.wrapping_add(1);

            search_id = rust.search_id;
        }

        self.as_mut().end_reset_model();

        self.as_mut().set_error_message(QString::default());

        self.as_mut()
            .set_next_move_distribution_json(QString::from("{}"));
        self.as_mut().set_total_occurrences(0);
        self.as_mut().set_games_examined(0);
        self.as_mut().set_total_games(0);
        self.as_mut().set_matching_games(0);
        self.as_mut().set_matches_found(0);

        self.as_mut().set_cancel_requested(false);
        self.as_mut().set_search_cancelled(false);
        self.as_mut().set_occurrence_load_in_progress(false);
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
                true,
                true,
                true,
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
                Ok(SearchPatternSummaryReportOutcome::Completed(report)) => {
                    BackgroundSearchResult::Completed(report)
                }

                Ok(SearchPatternSummaryReportOutcome::Cancelled) => {
                    BackgroundSearchResult::Cancelled
                }

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
            rust.occurrence_load_id = rust.occurrence_load_id.wrapping_add(1);

            rust.rows.clear();
            rust.all_rows.clear();
            rust.continuation_game_ids.clear();
            rust.search_query = None;
            rust.next_move_distribution = None;
        }

        self.as_mut().end_reset_model();

        if let Some(cancel_token) = cancel_token {
            cancel_token.store(true, Ordering::Relaxed);
        }

        self.as_mut().set_error_message(QString::default());

        self.as_mut()
            .set_next_move_distribution_json(QString::from("{}"));
        self.as_mut().set_total_occurrences(0);
        self.as_mut().set_games_examined(0);
        self.as_mut().set_total_games(0);
        self.as_mut().set_matching_games(0);
        self.as_mut().set_matches_found(0);

        self.as_mut().set_cancel_requested(false);
        self.as_mut().set_search_cancelled(false);
        self.as_mut().set_occurrence_load_in_progress(false);
        self.as_mut().set_search_in_progress(false);
    }
}

fn create_search_engine_and_query(
    project_path: &str,
    board_size: i32,
    stones_json: &str,
    left: i32,
    bottom: i32,
    width: i32,
    height: i32,
    include_rotations: bool,
    include_reflections: bool,
    include_reversed_colours: bool,
    scope: PatternSearchScope,
) -> Result<(SearchEngine, PatternSearchQuery), String> {
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
        scope,
        options: PatternSearchOptions {
            include_rotations,
            include_reflections,
            include_reversed_colours,
        },
    };

    let search_engine = SearchEngine::new(&project).map_err(|error| error.to_string())?;

    Ok((search_engine, query))
}

fn create_search_outcome<C, P>(
    project_path: &str,
    board_size: i32,
    stones_json: &str,
    left: i32,
    bottom: i32,
    width: i32,
    height: i32,
    include_rotations: bool,
    include_reflections: bool,
    include_reversed_colours: bool,
    is_cancelled: C,
    on_progress: P,
) -> Result<SearchPatternSummaryReportOutcome, String>
where
    C: FnMut() -> bool,
    P: FnMut(PatternSearchProgress),
{
    let (search_engine, query) = create_search_engine_and_query(
        project_path,
        board_size,
        stones_json,
        left,
        bottom,
        width,
        height,
        include_rotations,
        include_reflections,
        include_reversed_colours,
        PatternSearchScope::Project,
    )?;

    search_engine
        .search_pattern_summary_report_with_progress(&query, is_cancelled, on_progress)
        .map_err(|error| error.to_string())
}

fn finish_occurrence_load(
    mut model: Pin<&mut crate::game_list_model::ffi::SearchResultModel>,
    occurrence_load_id: u64,
    row_number: i32,
    completion: BackgroundOccurrenceResult,
) {
    if model.as_ref().get_ref().rust().occurrence_load_id != occurrence_load_id {
        return;
    }

    model.as_mut().set_occurrence_load_in_progress(false);

    match completion {
        BackgroundOccurrenceResult::Completed(occurrences) => {
            model.as_mut().occurrences_loaded(
                row_number,
                occurrences_to_json(&occurrences),
                QString::default(),
            );
        }

        BackgroundOccurrenceResult::Failed(error) => {
            model.as_mut().occurrences_loaded(
                row_number,
                QString::from("[]"),
                QString::from(error),
            );
        }
    }
}

fn finish_search(
    mut model: Pin<&mut crate::game_list_model::ffi::SearchResultModel>,
    search_id: u64,
    completion: BackgroundSearchResult,
) {
    if !is_current_search(model.as_ref().get_ref(), search_id) {
        return;
    }

    let (
        rows,
        next_move_distribution_json,
        stored_distribution,
        continuation_game_ids,
        error_message,
        cancelled,
    ) = match completion {
        BackgroundSearchResult::Completed(report) => {
            let SearchSummaryReport {
                results,
                next_moves,
                continuation_game_ids,
            } = report;
            let json = next_move_distribution_to_json(&next_moves);
            (
                search_results_to_rows(results),
                json,
                Some(next_moves),
                continuation_game_ids,
                None,
                false,
            )
        }
        BackgroundSearchResult::Cancelled => (
            Vec::new(),
            QString::from("{}"),
            None,
            HashMap::new(),
            None,
            true,
        ),
        BackgroundSearchResult::Failed(error) => (
            Vec::new(),
            QString::from("{}"),
            None,
            HashMap::new(),
            Some(error),
            false,
        ),
    };

    let total_occurrences = rows
        .iter()
        .fold(0_i32, |total, row| total.saturating_add(row.match_count));
    let matching_games = count_to_i32(rows.len());
    let keep_query = error_message.is_none() && !cancelled;

    model.as_mut().begin_reset_model();
    {
        let mut rust = model.as_mut().rust_mut();
        rust.all_rows = rows.clone();
        rust.rows = rows;
        rust.cancel_token = None;
        rust.next_move_distribution = stored_distribution;
        rust.continuation_game_ids = continuation_game_ids;
        if !keep_query {
            rust.search_query = None;
        }
    }
    model.as_mut().end_reset_model();

    model
        .as_mut()
        .set_next_move_distribution_json(next_move_distribution_json);
    model.as_mut().set_total_occurrences(total_occurrences);
    model.as_mut().set_matching_games(matching_games);
    model.as_mut().set_matches_found(total_occurrences);
    match error_message {
        Some(error) => model.as_mut().set_error_message(QString::from(error)),
        None => model.as_mut().set_error_message(QString::default()),
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

fn create_game_occurrences(
    query: &StoredSearchQuery,
    distribution: &NextMoveDistribution,
    game_id: i64,
) -> Result<Vec<LoadedSearchOccurrence>, String> {
    let (search_engine, pattern_query) = create_search_engine_and_query(
        &query.project_path,
        query.board_size,
        &query.stones_json,
        query.left,
        query.bottom,
        query.width,
        query.height,
        query.include_rotations,
        query.include_reflections,
        query.include_reversed_colours,
        PatternSearchScope::Game(game_id),
    )?;

    let results = search_engine
        .search_pattern(&pattern_query)
        .map_err(|error| error.to_string())?;

    let occurrences = results
        .into_iter()
        .find(|result| result.game_id == game_id)
        .map(|result| result.occurrences)
        .unwrap_or_default();

    occurrences
        .into_iter()
        .map(|occurrence| {
            let continuation_points =
                continuation_points_for_occurrence(query, distribution, &occurrence)?;

            Ok(LoadedSearchOccurrence {
                occurrence,
                continuation_points,
            })
        })
        .collect()
}

fn continuation_points_for_occurrence(
    query: &StoredSearchQuery,
    distribution: &NextMoveDistribution,
    occurrence: &SearchOccurrence,
) -> Result<Vec<ContinuationBoardPoint>, String> {
    let board_size = i16::try_from(query.board_size)
        .map_err(|_| format!("invalid continuation-map board size {}", query.board_size))?;

    let pattern_width = u8::try_from(query.width)
        .map_err(|_| format!("invalid continuation-map pattern width {}", query.width))?;

    let pattern_height = u8::try_from(query.height)
        .map_err(|_| format!("invalid continuation-map pattern height {}", query.height))?;

    let left = occurrence
        .left
        .ok_or_else(|| "pattern occurrence has no left coordinate".to_string())?;

    let bottom = occurrence
        .bottom
        .ok_or_else(|| "pattern occurrence has no bottom coordinate".to_string())?;

    let transformation = occurrence
        .transformation
        .unwrap_or(PatternTransformation::Identity);

    let mut mapped = Vec::new();

    for point in &distribution.points {
        let (relative_x, relative_y) = transformation.transform_relative_point(
            point.x,
            point.y,
            pattern_width,
            pattern_height,
        );

        let board_x = i16::from(left) + relative_x;
        let board_y = i16::from(bottom) + relative_y;

        /*
         * Margin points can lie beyond the board when projected onto
         * a particular edge or corner occurrence. Such points simply
         * have no drawable intersection in this occurrence.
         */
        if board_x < 0 || board_y < 0 || board_x >= board_size || board_y >= board_size {
            continue;
        }

        mapped.push(ContinuationBoardPoint {
            x: u8::try_from(board_x).expect("validated board x must fit in u8"),

            core_y: u8::try_from(board_y).expect("validated board y must fit in u8"),

            count: point.count,
        });
    }

    Ok(mapped)
}

fn transformation_name(transformation: Option<PatternTransformation>) -> &'static str {
    match transformation.unwrap_or(PatternTransformation::Identity) {
        PatternTransformation::Identity => "identity",
        PatternTransformation::Rotate90Clockwise => "rotate90Clockwise",
        PatternTransformation::Rotate180 => "rotate180",
        PatternTransformation::Rotate270Clockwise => "rotate270Clockwise",
        PatternTransformation::MirrorLeftRight => "mirrorLeftRight",
        PatternTransformation::MirrorTopBottom => "mirrorTopBottom",
        PatternTransformation::MirrorMainDiagonal => "mirrorMainDiagonal",
        PatternTransformation::MirrorAntiDiagonal => "mirrorAntiDiagonal",
    }
}

fn transformation_from_name(name: &str) -> Option<PatternTransformation> {
    match name {
        "identity" => Some(PatternTransformation::Identity),
        "rotate90Clockwise" => Some(PatternTransformation::Rotate90Clockwise),
        "rotate180" => Some(PatternTransformation::Rotate180),
        "rotate270Clockwise" => Some(PatternTransformation::Rotate270Clockwise),
        "mirrorLeftRight" => Some(PatternTransformation::MirrorLeftRight),
        "mirrorTopBottom" => Some(PatternTransformation::MirrorTopBottom),
        "mirrorMainDiagonal" => Some(PatternTransformation::MirrorMainDiagonal),
        "mirrorAntiDiagonal" => Some(PatternTransformation::MirrorAntiDiagonal),
        _ => None,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum GameResultClass {
    BlackWin,
    WhiteWin,
    Draw,
    Unknown,
}

fn classify_game_result(result: &str) -> GameResultClass {
    let result = result.trim().to_ascii_uppercase();

    if result.starts_with("B+") {
        return GameResultClass::BlackWin;
    }

    if result.starts_with("W+") {
        return GameResultClass::WhiteWin;
    }

    match result.as_str() {
        "0" | "0.0" | "DRAW" | "JIGO" => GameResultClass::Draw,
        _ => GameResultClass::Unknown,
    }
}

fn next_move_distribution_to_json(distribution: &NextMoveDistribution) -> QString {
    let json = NextMoveDistributionJson {
        margin: distribution.margin,
        appearances: distribution.appearances,
        matching_games: distribution.matching_games,
        points: distribution
            .points
            .iter()
            .copied()
            .map(NextMovePointJson::from)
            .collect(),
        outside_displayed_area: distribution.outside_displayed_area,
        passes: distribution.passes,
        game_ended: distribution.game_ended,
    };

    match serde_json::to_string(&json) {
        Ok(json) => QString::from(json),
        Err(_) => QString::from("{}"),
    }
}

fn occurrences_to_json(occurrences: &[LoadedSearchOccurrence]) -> QString {
    let occurrences = occurrences
        .iter()
        .map(|loaded| {
            let occurrence = &loaded.occurrence;

            SearchOccurrenceJson {
                move_number: i32::try_from(occurrence.move_number).unwrap_or(i32::MAX),

                left: occurrence.left.map_or(-1, i32::from),

                bottom: occurrence.bottom.map_or(-1, i32::from),

                transformation: transformation_name(occurrence.transformation),

                colours_reversed: occurrence.colours_reversed.unwrap_or(false),

                continuation_points: loaded
                    .continuation_points
                    .iter()
                    .copied()
                    .map(ContinuationPointJson::from)
                    .collect(),
            }
        })
        .collect::<Vec<_>>();

    match serde_json::to_string(&occurrences) {
        Ok(json) => QString::from(json),
        Err(_) => QString::from("[]"),
    }
}

fn search_results_to_rows(results: Vec<SearchSummaryResult>) -> Vec<SearchResultRow> {
    results
        .into_iter()
        .map(|result| {
            let match_count = i32::try_from(result.match_count).unwrap_or(i32::MAX);

            let first_match_move =
                i32::try_from(result.first_occurrence.move_number).unwrap_or(i32::MAX);

            let first_match_left = result.first_occurrence.left.map_or(-1, i32::from);

            let first_match_bottom = result.first_occurrence.bottom.map_or(-1, i32::from);

            SearchResultRow {
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
            }
        })
        .collect()
}

#[derive(Debug, serde::Serialize)]
struct ContinuationOutcomeSummaryJson {
    games: usize,

    #[serde(rename = "blackWins")]
    black_wins: usize,

    #[serde(rename = "whiteWins")]
    white_wins: usize,

    draws: usize,
    unknown: usize,
}

#[derive(Debug, serde::Serialize)]
struct NextMoveDistributionJson {
    margin: i16,
    appearances: usize,

    #[serde(rename = "matchingGames")]
    matching_games: usize,

    points: Vec<NextMovePointJson>,

    #[serde(rename = "outsideDisplayedArea")]
    outside_displayed_area: usize,

    passes: usize,

    #[serde(rename = "gameEnded")]
    game_ended: usize,
}

#[derive(Debug, serde::Serialize)]
struct NextMovePointJson {
    x: i16,
    y: i16,
    count: usize,
}

impl From<NextMovePointCount> for NextMovePointJson {
    fn from(point: NextMovePointCount) -> Self {
        Self {
            x: point.x,
            y: point.y,
            count: point.count,
        }
    }
}

#[derive(Debug, serde::Serialize)]
struct ContinuationPointJson {
    x: u8,

    #[serde(rename = "coreY")]
    core_y: u8,

    count: usize,
}

impl From<ContinuationBoardPoint> for ContinuationPointJson {
    fn from(point: ContinuationBoardPoint) -> Self {
        Self {
            x: point.x,
            core_y: point.core_y,
            count: point.count,
        }
    }
}

#[derive(Debug, serde::Serialize)]
struct SearchOccurrenceJson {
    #[serde(rename = "move")]
    move_number: i32,
    left: i32,
    bottom: i32,
    transformation: &'static str,

    #[serde(rename = "coloursReversed")]
    colours_reversed: bool,

    #[serde(rename = "continuationPoints")]
    continuation_points: Vec<ContinuationPointJson>,
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

#[cfg(test)]
mod next_move_json_tests {
    use super::*;

    #[test]
    fn serialises_next_move_distribution_for_qml() {
        let distribution = NextMoveDistribution {
            margin: 3,
            appearances: 8,
            matching_games: 6,
            points: vec![
                NextMovePointCount {
                    x: -1,
                    y: 2,
                    count: 3,
                },
                NextMovePointCount {
                    x: 4,
                    y: 1,
                    count: 2,
                },
            ],
            outside_displayed_area: 1,
            passes: 1,
            game_ended: 1,
        };

        let json = next_move_distribution_to_json(&distribution).to_string();

        let value: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(
            value,
            serde_json::json!({
                "margin": 3,
                "appearances": 8,
                "matchingGames": 6,
                "points": [
                    {
                        "x": -1,
                        "y": 2,
                        "count": 3
                    },
                    {
                        "x": 4,
                        "y": 1,
                        "count": 2
                    }
                ],
                "outsideDisplayedArea": 1,
                "passes": 1,
                "gameEnded": 1
            })
        );
    }
}

#[cfg(test)]
mod occurrence_continuation_tests {
    use super::*;

    #[test]
    fn maps_normalised_points_to_rotated_occurrence() {
        let query = StoredSearchQuery {
            project_path: String::new(),
            board_size: 19,
            stones_json: "[]".to_string(),
            left: 0,
            bottom: 0,
            width: 4,
            height: 5,
            include_rotations: true,
            include_reflections: true,
            include_reversed_colours: true,
        };

        let distribution = NextMoveDistribution {
            margin: 3,
            appearances: 1,
            matching_games: 1,
            points: vec![NextMovePointCount {
                x: 1,
                y: 2,
                count: 7,
            }],
            outside_displayed_area: 0,
            passes: 0,
            game_ended: 0,
        };

        let occurrence = SearchOccurrence {
            move_number: 20,
            last_move_number: 20,
            side_to_move: Some(Colour::White),
            ko_point: None,
            left: Some(7),
            bottom: Some(8),
            transformation: Some(PatternTransformation::Rotate90Clockwise),
            colours_reversed: Some(true),
        };

        assert_eq!(
            continuation_points_for_occurrence(&query, &distribution, &occurrence,).unwrap(),
            vec![ContinuationBoardPoint {
                x: 9,
                core_y: 10,
                count: 7,
            }]
        );
    }

    #[test]
    fn omits_margin_points_outside_board() {
        let query = StoredSearchQuery {
            project_path: String::new(),
            board_size: 19,
            stones_json: "[]".to_string(),
            left: 0,
            bottom: 0,
            width: 4,
            height: 4,
            include_rotations: true,
            include_reflections: true,
            include_reversed_colours: true,
        };

        let distribution = NextMoveDistribution {
            margin: 3,
            appearances: 1,
            matching_games: 1,
            points: vec![NextMovePointCount {
                x: -2,
                y: 1,
                count: 3,
            }],
            outside_displayed_area: 0,
            passes: 0,
            game_ended: 0,
        };

        let occurrence = SearchOccurrence {
            move_number: 10,
            last_move_number: 10,
            side_to_move: Some(Colour::Black),
            ko_point: None,
            left: Some(0),
            bottom: Some(0),
            transformation: Some(PatternTransformation::Identity),
            colours_reversed: Some(false),
        };

        assert!(
            continuation_points_for_occurrence(&query, &distribution, &occurrence,)
                .unwrap()
                .is_empty()
        );
    }
}

#[cfg(test)]
mod continuation_outcome_classification_tests {
    use super::{GameResultClass, classify_game_result};

    #[test]
    fn classifies_standard_sgf_results_without_overinterpreting_unknown_values() {
        assert_eq!(classify_game_result("B+R"), GameResultClass::BlackWin);
        assert_eq!(classify_game_result("b+2.5"), GameResultClass::BlackWin);
        assert_eq!(classify_game_result("W+T"), GameResultClass::WhiteWin);
        assert_eq!(classify_game_result("w+0.5"), GameResultClass::WhiteWin);
        assert_eq!(classify_game_result("0"), GameResultClass::Draw);
        assert_eq!(classify_game_result("Jigo"), GameResultClass::Draw);
        assert_eq!(classify_game_result("Draw"), GameResultClass::Draw);
        assert_eq!(classify_game_result("Void"), GameResultClass::Unknown);
        assert_eq!(classify_game_result("?"), GameResultClass::Unknown);
        assert_eq!(classify_game_result(""), GameResultClass::Unknown);
    }
}
