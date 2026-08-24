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

use bermuda::{
    Board, Colour, GameRecord, LocalActivity, Move, NearbyMove, NextMoveDistribution,
    NextMovePointCount, Pattern, PatternBoardContext, PatternMatch, PatternRect,
    PatternSearchOptions, PatternSearchProgress, PatternSearchQuery, PatternSearchScope,
    PatternTransformation, SearchEngine, SearchOccurrence, SearchPatternSummaryReportOutcome,
    SearchSummaryReport, SearchSummaryResult,
    game_list::{GameListPlayerMetadata, GameListQuery, GameResultFilter, PlayerColour},
    measure_local_activity,
    project_manager::ProjectManager,
    source_long_axis_edge_band,
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
    black_player_id: Option<i64>,
    white_player_id: Option<i64>,
    black_player_display: QString,
    white_player_display: QString,
    played_date: QString,
    result: QString,
    event: QString,
    komi: QString,
    match_count: i32,
    first_match_move: i32,
    first_match_left: i32,
    first_match_bottom: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SearchResultSortColumn {
    BlackPlayer,
    WhitePlayer,
    Date,
    Matches,
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
    include_handicap_games: bool,
}

#[derive(Default)]
pub struct SearchResultModelRust {
    rows: Vec<SearchResultRow>,
    all_rows: Vec<SearchResultRow>,
    continuation_game_ids: HashMap<(i16, i16), Vec<i64>>,
    selected_continuation: Option<(i16, i16)>,
    metadata_filter: GameListQuery,

    pub(crate) error_message: QString,
    pub(crate) next_move_distribution_json: QString,
    pub(crate) outcome_summary_json: QString,
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
    local_activity: LocalActivity,
    selected_continuation_match: bool,
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
            BLACK_PLAYER_ROLE => QVariant::from(&row.black_player_display),
            WHITE_PLAYER_ROLE => QVariant::from(&row.white_player_display),
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

        let (query, next_move_distribution, selected_continuation, game_id, occurrence_load_id) = {
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
            let selected_continuation = rust.selected_continuation;

            rust.occurrence_load_id = rust.occurrence_load_id.wrapping_add(1);

            (
                query,
                next_move_distribution,
                selected_continuation,
                game_id,
                rust.occurrence_load_id,
            )
        };

        self.as_mut().set_occurrence_load_in_progress(true);

        let qt_thread = self.qt_thread();

        std::thread::spawn(move || {
            let completion = match create_game_occurrences(
                &query,
                &next_move_distribution,
                selected_continuation,
                game_id,
            ) {
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

        {
            let rust = self.as_ref().get_ref().rust();

            if !rust
                .continuation_game_ids
                .contains_key(&(normalised_x, normalised_y))
            {
                return false;
            }
        }

        let filtered_rows = {
            let rust = self.as_ref().get_ref().rust();

            filtered_search_rows(
                &rust.all_rows,
                &rust.metadata_filter,
                Some((normalised_x, normalised_y)),
                &rust.continuation_game_ids,
            )
        };

        let outcome_summary_json = outcome_summary_json_for_rows(&filtered_rows);

        self.as_mut().begin_reset_model();
        {
            let mut rust = self.as_mut().rust_mut();
            rust.rows = filtered_rows;
            rust.selected_continuation = Some((normalised_x, normalised_y));
        }
        self.as_mut().end_reset_model();

        self.as_mut()
            .set_outcome_summary_json(outcome_summary_json);

        true
    }

    pub(crate) fn continuation_at_occurrence_is_selected(
        self: Pin<&mut Self>,
        board_x: i32,
        core_y: i32,
        left: i32,
        bottom: i32,
        transformation: &QString,
    ) -> bool {
        let rust = self.as_ref().get_ref().rust();

        let Some(query) = rust.search_query.as_ref() else {
            return false;
        };

        let Ok(pattern_width) = u8::try_from(query.width) else {
            return false;
        };

        let Ok(pattern_height) = u8::try_from(query.height) else {
            return false;
        };

        let transformation_name = transformation.to_string();

        let Some(transformation) = transformation_from_name(&transformation_name) else {
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

        let normalised = transformation.inverse_relative_point(
            board_x - left,
            core_y - bottom,
            pattern_width,
            pattern_height,
        );

        rust.selected_continuation == Some(normalised)
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

        let mut summary = OutcomeSummaryJson {
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
        let rows = {
            let rust = self.as_ref().get_ref().rust();

            filtered_search_rows(
                &rust.all_rows,
                &rust.metadata_filter,
                None,
                &rust.continuation_game_ids,
            )
        };

        let outcome_summary_json = outcome_summary_json_for_rows(&rows);

        self.as_mut().begin_reset_model();
        {
            let mut rust = self.as_mut().rust_mut();
            rust.rows = rows;
            rust.selected_continuation = None;
        }
        self.as_mut().end_reset_model();

        self.as_mut()
            .set_outcome_summary_json(outcome_summary_json);
    }

    pub(crate) fn filter_results(
        mut self: Pin<&mut Self>,
        player: &QString,
        versus: &QString,
        colour: &QString,
        event: &QString,
        date_from: &QString,
        date_to: &QString,
        result: &QString,
    ) -> bool {
        let mut filter = match metadata_filter_from_values(
            player, versus, colour, event, date_from, date_to, result,
        ) {
            Ok(filter) => filter,

            Err(error) => {
                self.as_mut().set_error_message(QString::from(error));
                return false;
            }
        };

        if filter.player.is_some() || filter.versus.is_some() {
            let project_path = {
                let rust = self.as_ref().get_ref().rust();

                rust.search_query
                    .as_ref()
                    .map(|query| query.project_path.clone())
            };

            let Some(project_path) = project_path else {
                self.as_mut().set_error_message(QString::from(
                    "no project search is available for player filtering",
                ));
                return false;
            };

            if let Err(error) = resolve_metadata_player_ids(&project_path, &mut filter) {
                self.as_mut().set_error_message(QString::from(error));
                return false;
            }
        }

        let rows = {
            let rust = self.as_ref().get_ref().rust();

            filtered_search_rows(
                &rust.all_rows,
                &filter,
                rust.selected_continuation,
                &rust.continuation_game_ids,
            )
        };

        let outcome_summary_json = outcome_summary_json_for_rows(&rows);

        self.as_mut().begin_reset_model();
        {
            let mut rust = self.as_mut().rust_mut();
            rust.metadata_filter = filter;
            rust.rows = rows;
        }
        self.as_mut().end_reset_model();

        self.as_mut()
            .set_outcome_summary_json(outcome_summary_json);
        self.as_mut().set_error_message(QString::default());
        true
    }

    pub(crate) fn sort_results(
        mut self: Pin<&mut Self>,
        column: &QString,
        ascending: bool,
    ) -> bool {
        let column_name = column.to_string();

        let Some(sort_column) = search_result_sort_column(column_name.trim()) else {
            self.as_mut().set_error_message(QString::from(format!(
                "unknown search-result sort column: {column_name}"
            )));
            return false;
        };

        /*
         * all_rows is the source from which metadata and continuation
         * filters rebuild the displayed rows.  Keeping it sorted means
         * every later filtering operation naturally preserves the user's
         * chosen result ordering without rerunning the pattern search.
         */
        let (all_rows, rows) = {
            let rust = self.as_ref().get_ref().rust();
            let mut all_rows = rust.all_rows.clone();

            sort_search_result_rows(&mut all_rows, sort_column, ascending);

            let rows = filtered_search_rows(
                &all_rows,
                &rust.metadata_filter,
                rust.selected_continuation,
                &rust.continuation_game_ids,
            );

            (all_rows, rows)
        };

        self.as_mut().begin_reset_model();

        {
            let mut rust = self.as_mut().rust_mut();
            rust.all_rows = all_rows;
            rust.rows = rows;
        }

        self.as_mut().end_reset_model();
        self.as_mut().set_error_message(QString::default());

        true
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
        include_handicap_games: bool,
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
            include_handicap_games,
        };

        let cancel_token = Arc::new(AtomicBool::new(false));

        let search_id;

        self.as_mut().begin_reset_model();

        {
            let mut rust = self.as_mut().rust_mut();

            rust.rows.clear();
            rust.all_rows.clear();
            rust.continuation_game_ids.clear();
            rust.selected_continuation = None;
            rust.metadata_filter = GameListQuery::default();
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
        self.as_mut()
            .set_outcome_summary_json(QString::from("{}"));
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
                include_handicap_games,
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
            rust.selected_continuation = None;
            rust.metadata_filter = GameListQuery::default();
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
        self.as_mut()
            .set_outcome_summary_json(QString::from("{}"));
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

fn optional_filter_value(value: &QString) -> Option<String> {
    let value = value.to_string();
    let value = value.trim();

    (!value.is_empty()).then(|| value.to_owned())
}

fn metadata_filter_from_values(
    player: &QString,
    versus: &QString,
    colour: &QString,
    event: &QString,
    date_from: &QString,
    date_to: &QString,
    result: &QString,
) -> Result<GameListQuery, String> {
    let colour_name = colour.to_string();

    let colour = match colour_name.as_str() {
        "black" => PlayerColour::Black,
        "white" => PlayerColour::White,
        "either" => PlayerColour::Either,

        _ => {
            return Err(format!("unknown player colour filter: {colour_name}"));
        }
    };

    let result_name = result.to_string();

    let result = match result_name.as_str() {
        "any" => GameResultFilter::Any,
        "black-win" => GameResultFilter::BlackWin,
        "white-win" => GameResultFilter::WhiteWin,
        "jigo" => GameResultFilter::Jigo,
        "void" => GameResultFilter::Void,

        _ => {
            return Err(format!("unknown game result filter: {result_name}"));
        }
    };

    Ok(GameListQuery {
        player: optional_filter_value(player),
        versus: optional_filter_value(versus),
        colour,
        event: optional_filter_value(event),
        date_from: optional_filter_value(date_from),
        date_to: optional_filter_value(date_to),
        result,
        ..GameListQuery::default()
    })
}

fn resolve_metadata_player_ids(
    project_path: &str,
    filter: &mut GameListQuery,
) -> Result<(), String> {
    let directory = ProjectManager::new()
        .open(Path::new(project_path))
        .and_then(|project| project.player_directory())
        .map_err(|error| error.to_string())?;

    let player_ids = match filter.player.as_deref() {
        Some(name) => directory
            .player_ids_for_search_name(name)
            .map_err(|error| error.to_string())?,
        None => Vec::new(),
    };

    let versus_ids = match filter.versus.as_deref() {
        Some(name) => directory
            .player_ids_for_search_name(name)
            .map_err(|error| error.to_string())?,
        None => Vec::new(),
    };

    filter.resolved_player_ids = player_ids;
    filter.resolved_versus_ids = versus_ids;

    Ok(())
}

fn search_row_matches_metadata(row: &SearchResultRow, filter: &GameListQuery) -> bool {
    let black_player = row.black_player.to_string();
    let white_player = row.white_player.to_string();
    let played_date = row.played_date.to_string();
    let result = row.result.to_string();
    let event = row.event.to_string();

    filter.matches_metadata(
        GameListPlayerMetadata {
            source_name: (!black_player.is_empty()).then_some(black_player.as_str()),
            player_id: row.black_player_id,
        },
        GameListPlayerMetadata {
            source_name: (!white_player.is_empty()).then_some(white_player.as_str()),
            player_id: row.white_player_id,
        },
        (!played_date.is_empty()).then_some(played_date.as_str()),
        (!result.is_empty()).then_some(result.as_str()),
        (!event.is_empty()).then_some(event.as_str()),
    )
}

fn search_result_sort_column(name: &str) -> Option<SearchResultSortColumn> {
    match name {
        "black" => Some(SearchResultSortColumn::BlackPlayer),
        "white" => Some(SearchResultSortColumn::WhitePlayer),
        "date" => Some(SearchResultSortColumn::Date),
        "matches" => Some(SearchResultSortColumn::Matches),
        _ => None,
    }
}

fn compare_search_text(left: &QString, right: &QString, ascending: bool) -> std::cmp::Ordering {
    let left = left.to_string();
    let right = right.to_string();

    let left = left.trim();
    let right = right.trim();

    /*
     * Missing metadata is less useful for browsing and therefore remains
     * at the end in both ascending and descending orders.
     */
    match (left.is_empty(), right.is_empty()) {
        (true, true) => return std::cmp::Ordering::Equal,
        (true, false) => return std::cmp::Ordering::Greater,
        (false, true) => return std::cmp::Ordering::Less,
        (false, false) => {}
    }

    let ordering = left.to_lowercase().cmp(&right.to_lowercase());

    if ascending {
        ordering
    } else {
        ordering.reverse()
    }
}

fn compare_match_count(left: i32, right: i32, ascending: bool) -> std::cmp::Ordering {
    let ordering = left.cmp(&right);

    if ascending {
        ordering
    } else {
        ordering.reverse()
    }
}

fn compare_search_result_rows(
    left: &SearchResultRow,
    right: &SearchResultRow,
    column: SearchResultSortColumn,
    ascending: bool,
) -> std::cmp::Ordering {
    use std::cmp::Ordering::Equal;

    let primary = match column {
        SearchResultSortColumn::BlackPlayer => compare_search_text(
            &left.black_player_display,
            &right.black_player_display,
            ascending,
        ),

        SearchResultSortColumn::WhitePlayer => compare_search_text(
            &left.white_player_display,
            &right.white_player_display,
            ascending,
        ),

        SearchResultSortColumn::Date => {
            compare_search_text(&left.played_date, &right.played_date, ascending)
        }

        SearchResultSortColumn::Matches => {
            compare_match_count(left.match_count, right.match_count, ascending)
        }
    };

    if primary != Equal {
        return primary;
    }

    /*
     * Secondary ordering is deliberately automatic rather than another
     * Search Results UI.  These fixed tie-breaks make nearby rows useful
     * and deterministic while keeping the result list simple.
     */
    let secondary = match column {
        SearchResultSortColumn::BlackPlayer => compare_search_text(
            &left.white_player_display,
            &right.white_player_display,
            true,
        ),

        SearchResultSortColumn::WhitePlayer => compare_search_text(
            &left.black_player_display,
            &right.black_player_display,
            true,
        ),

        SearchResultSortColumn::Date => compare_search_text(
            &left.black_player_display,
            &right.black_player_display,
            true,
        ),

        SearchResultSortColumn::Matches => {
            compare_search_text(&left.played_date, &right.played_date, false)
        }
    };

    if secondary != Equal {
        return secondary;
    }

    let tertiary = match column {
        SearchResultSortColumn::BlackPlayer | SearchResultSortColumn::WhitePlayer => {
            compare_search_text(&left.played_date, &right.played_date, false)
        }

        SearchResultSortColumn::Date => compare_search_text(
            &left.white_player_display,
            &right.white_player_display,
            true,
        ),

        SearchResultSortColumn::Matches => compare_search_text(
            &left.black_player_display,
            &right.black_player_display,
            true,
        ),
    };

    if tertiary != Equal {
        return tertiary;
    }

    if column == SearchResultSortColumn::Matches {
        let quaternary = compare_search_text(
            &left.white_player_display,
            &right.white_player_display,
            true,
        );

        if quaternary != Equal {
            return quaternary;
        }
    }

    left.game_id.cmp(&right.game_id)
}

fn sort_search_result_rows(
    rows: &mut [SearchResultRow],
    column: SearchResultSortColumn,
    ascending: bool,
) {
    rows.sort_by(|left, right| compare_search_result_rows(left, right, column, ascending));
}

fn filtered_search_rows(
    all_rows: &[SearchResultRow],
    metadata_filter: &GameListQuery,
    selected_continuation: Option<(i16, i16)>,
    continuation_game_ids: &HashMap<(i16, i16), Vec<i64>>,
) -> Vec<SearchResultRow> {
    let continuation_ids =
        selected_continuation.and_then(|point| continuation_game_ids.get(&point));

    all_rows
        .iter()
        .filter(|row| {
            if let Some(game_ids) = continuation_ids
                && game_ids.binary_search(&row.game_id).is_err()
            {
                return false;
            }

            search_row_matches_metadata(row, metadata_filter)
        })
        .cloned()
        .collect()
}

const AUTOMATIC_LONG_AXIS_EDGE_BAND: u8 = 5;

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
    include_handicap_games: bool,
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

    let right = rect
        .left
        .checked_add(rect.width)
        .ok_or_else(|| "pattern rectangle lies outside the board".to_owned())?;

    let top = rect
        .bottom
        .checked_add(rect.height)
        .ok_or_else(|| "pattern rectangle lies outside the board".to_owned())?;

    let board_context = PatternBoardContext {
        left: rect.left,
        right: board.size() - right,
        bottom: rect.bottom,
        top: board.size() - top,
    };

    let query = PatternSearchQuery {
        pattern,
        board_context: Some(board_context),
        scope,
        options: PatternSearchOptions {
            include_rotations,
            include_reflections,
            include_reversed_colours,
            include_handicap_games,
            long_axis_edge_band: source_long_axis_edge_band(
                board.size(),
                rect.width,
                rect.height,
                rect.left,
                rect.bottom,
                AUTOMATIC_LONG_AXIS_EDGE_BAND,
            ),
            max_match_move: None,
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
    include_handicap_games: bool,
    is_cancelled: C,
    on_progress: P,
) -> Result<SearchPatternSummaryReportOutcome, String>
where
    C: FnMut() -> bool,
    P: FnMut(PatternSearchProgress),
{
    let (search_engine, mut query) = create_search_engine_and_query(
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
        include_handicap_games,
        PatternSearchScope::Project,
    )?;

    /*
     * Project-wide pattern search is not tied to the source rectangle's
     * exact board coordinates. Edge-oriented elongated patterns remain
     * constrained by long_axis_edge_band.
     */
    query.board_context = None;

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

    let outcome_summary_json = if keep_query {
        outcome_summary_json_for_rows(&rows)
    } else {
        QString::from("{}")
    };

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
    model
        .as_mut()
        .set_outcome_summary_json(outcome_summary_json);
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
    selected_continuation: Option<(i16, i16)>,
    game_id: i64,
) -> Result<Vec<LoadedSearchOccurrence>, String> {
    let (search_engine, mut pattern_query) = create_search_engine_and_query(
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
        query.include_handicap_games,
        PatternSearchScope::Game(game_id),
    )?;

    /*
     * Keep occurrence reconstruction consistent with the project summary
     * search, which searches without board context. Richer per-occurrence
     * context is calculated below after the matching occurrences are known.
     */
    pattern_query.board_context = None;

    let results = search_engine
        .search_pattern(&pattern_query)
        .map_err(|error| error.to_string())?;

    let occurrences = results
        .into_iter()
        .find(|result| result.game_id == game_id)
        .map(|result| result.occurrences)
        .unwrap_or_default();

    /*
     * Occurrence context is deliberately measured here, after the user has
     * selected one result game. The broad project search remains unchanged.
     */
    let project = ProjectManager::new()
        .open(Path::new(&query.project_path))
        .map_err(|error| error.to_string())?;

    let indexer = project
        .position_indexer()
        .map_err(|error| error.to_string())?;
    let record = indexer
        .read_game_by_id(game_id)
        .map_err(|error| error.to_string())?;

    occurrences
        .into_iter()
        .map(|occurrence| {
            let continuation_points =
                continuation_points_for_occurrence(query, distribution, &occurrence)?;

            let local_activity = local_activity_for_occurrence(
                &record,
                &pattern_query.pattern,
                game_id,
                &occurrence,
            )?;

            let selected_continuation_match = occurrence_has_selected_continuation(
                &record,
                &pattern_query.pattern,
                &occurrence,
                selected_continuation,
            )?;

            Ok(LoadedSearchOccurrence {
                occurrence,
                continuation_points,
                local_activity,
                selected_continuation_match,
            })
        })
        .collect()
}

fn occurrence_has_selected_continuation(
    record: &GameRecord,
    pattern: &Pattern,
    occurrence: &SearchOccurrence,
    selected_continuation: Option<(i16, i16)>,
) -> Result<bool, String> {
    /*
     * Position N is followed by move N + 1 at zero-based record.moves[N].
     * Immediate continuation is measured from the first matching position,
     * not from the end of the appearance span.
     */
    let next_move = record.moves.get(occurrence.move_number).copied();

    next_move_matches_selected_continuation(
        record.board_size,
        next_move,
        pattern,
        occurrence,
        selected_continuation,
    )
}

fn next_move_matches_selected_continuation(
    board_size: u8,
    next_move: Option<Move>,
    pattern: &Pattern,
    occurrence: &SearchOccurrence,
    selected_continuation: Option<(i16, i16)>,
) -> Result<bool, String> {
    let Some(selected_continuation) = selected_continuation else {
        return Ok(false);
    };

    let Some(next_move) = next_move else {
        return Ok(false);
    };

    let Some(point) = next_move.point else {
        return Ok(false);
    };

    let board_size = u16::from(board_size);
    if board_size == 0 {
        return Ok(false);
    }

    let board_points = board_size * board_size;
    if point >= board_points {
        return Ok(false);
    }

    let board_x = i16::try_from(point % board_size).expect("board x coordinate must fit in i16");
    let board_y = i16::try_from(point / board_size).expect("board y coordinate must fit in i16");

    let left = occurrence
        .left
        .ok_or_else(|| "pattern occurrence has no left coordinate".to_string())?;
    let bottom = occurrence
        .bottom
        .ok_or_else(|| "pattern occurrence has no bottom coordinate".to_string())?;
    let transformation = occurrence
        .transformation
        .ok_or_else(|| "pattern occurrence has no transformation".to_string())?;

    let (normalised_x, normalised_y) = transformation.inverse_relative_point(
        board_x - i16::from(left),
        board_y - i16::from(bottom),
        pattern.width,
        pattern.height,
    );

    Ok((normalised_x, normalised_y) == selected_continuation)
}

fn local_activity_for_occurrence(
    record: &GameRecord,
    pattern: &Pattern,
    game_id: i64,
    occurrence: &SearchOccurrence,
) -> Result<LocalActivity, String> {
    let side_to_move = occurrence
        .side_to_move
        .ok_or_else(|| "pattern occurrence has no side to move".to_string())?;

    let left = occurrence
        .left
        .ok_or_else(|| "pattern occurrence has no left coordinate".to_string())?;

    let bottom = occurrence
        .bottom
        .ok_or_else(|| "pattern occurrence has no bottom coordinate".to_string())?;

    let transformation = occurrence
        .transformation
        .ok_or_else(|| "pattern occurrence has no transformation".to_string())?;

    let colours_reversed = occurrence
        .colours_reversed
        .ok_or_else(|| "pattern occurrence has no colour-reversal state".to_string())?;

    let appearance = PatternMatch {
        game_id,
        move_number: occurrence.move_number,
        last_move_number: occurrence.last_move_number,
        side_to_move,
        ko_point: occurrence.ko_point,
        left,
        bottom,
        transformation,
        colours_reversed,
    };

    Ok(measure_local_activity(record, pattern, &appearance))
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

fn outcome_summary_for_rows(rows: &[SearchResultRow]) -> OutcomeSummaryJson {
    let mut summary = OutcomeSummaryJson {
        games: rows.len(),
        black_wins: 0,
        white_wins: 0,
        draws: 0,
        unknown: 0,
    };

    for row in rows {
        match classify_game_result(&row.result.to_string()) {
            GameResultClass::BlackWin => {
                summary.black_wins = summary.black_wins.saturating_add(1);
            }

            GameResultClass::WhiteWin => {
                summary.white_wins = summary.white_wins.saturating_add(1);
            }

            GameResultClass::Draw => {
                summary.draws = summary.draws.saturating_add(1);
            }

            GameResultClass::Unknown => {
                summary.unknown = summary.unknown.saturating_add(1);
            }
        }
    }

    summary
}

fn outcome_summary_json_for_rows(rows: &[SearchResultRow]) -> QString {
    match serde_json::to_string(&outcome_summary_for_rows(rows)) {
        Ok(json) => QString::from(json),
        Err(_) => QString::from("{}"),
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

                last_move_number: i32::try_from(occurrence.last_move_number).unwrap_or(i32::MAX),

                duration_moves: i32::try_from(occurrence.duration_moves()).unwrap_or(i32::MAX),

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

                local_activity: local_activity_to_json(
                    &loaded.local_activity,
                    occurrence.move_number,
                ),

                selected_continuation_match: loaded.selected_continuation_match,
            }
        })
        .collect::<Vec<_>>();

    match serde_json::to_string(&occurrences) {
        Ok(json) => QString::from(json),
        Err(_) => QString::from("[]"),
    }
}

fn nearby_move_to_json(
    nearby: Option<NearbyMove>,
    appearance_move_number: usize,
) -> Option<NearbyMoveJson> {
    nearby.map(|nearby| NearbyMoveJson {
        move_number: i32::try_from(nearby.move_number).unwrap_or(i32::MAX),
        x: nearby.x,
        core_y: nearby.y,
        distance: nearby.distance,
        delay_moves: i32::try_from(nearby.delay_moves_from(appearance_move_number))
            .unwrap_or(i32::MAX),
    })
}

fn local_activity_to_json(
    activity: &LocalActivity,
    appearance_move_number: usize,
) -> LocalActivityJson {
    LocalActivityJson {
        inside: nearby_move_to_json(activity.first_inside, appearance_move_number),
        within_one: nearby_move_to_json(activity.first_within_one, appearance_move_number),
        within_two: nearby_move_to_json(activity.first_within_two, appearance_move_number),
        within_three: nearby_move_to_json(activity.first_within_three, appearance_move_number),
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
                black_player_id: result.black_player_id,
                white_player_id: result.white_player_id,
                black_player_display: optional_text(&result.black_player_display),
                white_player_display: optional_text(&result.white_player_display),
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
struct OutcomeSummaryJson {
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
struct NearbyMoveJson {
    #[serde(rename = "move")]
    move_number: i32,

    x: u8,

    #[serde(rename = "coreY")]
    core_y: u8,

    distance: u8,

    #[serde(rename = "delayMoves")]
    delay_moves: i32,
}

#[derive(Debug, serde::Serialize)]
struct LocalActivityJson {
    inside: Option<NearbyMoveJson>,

    #[serde(rename = "withinOne")]
    within_one: Option<NearbyMoveJson>,

    #[serde(rename = "withinTwo")]
    within_two: Option<NearbyMoveJson>,

    #[serde(rename = "withinThree")]
    within_three: Option<NearbyMoveJson>,
}

#[derive(Debug, serde::Serialize)]
struct SearchOccurrenceJson {
    #[serde(rename = "move")]
    move_number: i32,

    #[serde(rename = "lastMove")]
    last_move_number: i32,

    #[serde(rename = "durationMoves")]
    duration_moves: i32,

    left: i32,
    bottom: i32,
    transformation: &'static str,

    #[serde(rename = "coloursReversed")]
    colours_reversed: bool,

    #[serde(rename = "continuationPoints")]
    continuation_points: Vec<ContinuationPointJson>,

    #[serde(rename = "localActivity")]
    local_activity: LocalActivityJson,

    #[serde(rename = "selectedContinuationMatch")]
    selected_continuation_match: bool,
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
mod search_result_sort_tests {
    use super::*;

    fn row(game_id: i64, black: &str, white: &str, date: &str, matches: i32) -> SearchResultRow {
        SearchResultRow {
            game_id,
            black_player: QString::from(black),
            white_player: QString::from(white),
            black_player_display: QString::from(black),
            white_player_display: QString::from(white),
            played_date: QString::from(date),
            match_count: matches,
            ..SearchResultRow::default()
        }
    }

    fn ids(rows: &[SearchResultRow]) -> Vec<i64> {
        rows.iter().map(|row| row.game_id).collect()
    }

    #[test]
    fn metadata_filter_uses_identity_for_linked_players_and_raw_for_unlinked_players() {
        let identified = SearchResultRow {
            game_id: 1,
            black_player: QString::from("Source Alpha"),
            black_player_id: Some(42),
            ..SearchResultRow::default()
        };

        let preferred = GameListQuery {
            player: Some("Preferred Alpha".to_owned()),
            resolved_player_ids: vec![42],
            colour: PlayerColour::Black,
            ..GameListQuery::default()
        };

        assert!(search_row_matches_metadata(&identified, &preferred));

        /*
         * Several identities may legitimately share the typed search name.
         * Membership in any of those identities is a valid search match.
         */
        let ambiguous = GameListQuery {
            player: Some("Shared Name".to_owned()),
            resolved_player_ids: vec![17, 42, 99],
            colour: PlayerColour::Black,
            ..GameListQuery::default()
        };

        assert!(search_row_matches_metadata(&identified, &ambiguous));

        /*
         * Once linked, the identity is authoritative. A literal PB spelling
         * with no corresponding resolved identity must not bypass it.
         */
        let raw_only_against_identified = GameListQuery {
            player: Some("Source Alpha".to_owned()),
            colour: PlayerColour::Black,
            ..GameListQuery::default()
        };

        assert!(!search_row_matches_metadata(
            &identified,
            &raw_only_against_identified
        ));

        let unresolved = SearchResultRow {
            game_id: 2,
            black_player: QString::from("Unresolved Name"),
            black_player_id: None,
            ..SearchResultRow::default()
        };

        let raw_unresolved = GameListQuery {
            player: Some("Unresolved Name".to_owned()),
            colour: PlayerColour::Black,
            ..GameListQuery::default()
        };

        assert!(search_row_matches_metadata(&unresolved, &raw_unresolved));
    }

    #[test]
    fn player_sort_uses_display_name_not_source_spelling() {
        let mut rows = vec![
            SearchResultRow {
                game_id: 1,
                black_player: QString::from("Zulu Source Spelling"),
                black_player_display: QString::from("Alpha Preferred"),
                white_player: QString::from("Opponent"),
                white_player_display: QString::from("Opponent"),
                ..SearchResultRow::default()
            },
            SearchResultRow {
                game_id: 2,
                black_player: QString::from("Alpha Source Spelling"),
                black_player_display: QString::from("Zulu Preferred"),
                white_player: QString::from("Opponent"),
                white_player_display: QString::from("Opponent"),
                ..SearchResultRow::default()
            },
        ];

        sort_search_result_rows(&mut rows, SearchResultSortColumn::BlackPlayer, true);

        assert_eq!(ids(&rows), vec![1, 2]);
    }

    #[test]
    fn sorts_matches_descending_with_useful_fixed_ties() {
        let mut rows = vec![
            row(1, "Lee", "Cho", "2020-01-01", 2),
            row(2, "Lee", "Cho", "2023-01-01", 4),
            row(3, "Cho", "Lee", "2021-01-01", 4),
            row(4, "Cho", "Lee", "2019-01-01", 1),
        ];

        sort_search_result_rows(&mut rows, SearchResultSortColumn::Matches, false);

        /*
         * Four-match games come first; their tie is broken by newest
         * date.  Remaining rows then follow by match count.
         */
        assert_eq!(ids(&rows), vec![2, 3, 1, 4]);
    }

    #[test]
    fn sorts_player_names_case_insensitively_then_by_opponent_and_date() {
        let mut rows = vec![
            row(4, "Lee", "Zhao", "2020-01-01", 1),
            row(3, "lee", "Cho", "2019-01-01", 1),
            row(2, "Lee", "Cho", "2022-01-01", 1),
            row(1, "Cho", "Lee", "2021-01-01", 1),
        ];

        sort_search_result_rows(&mut rows, SearchResultSortColumn::BlackPlayer, true);

        /*
         * Cho precedes Lee.  Lee/lee are equivalent for the primary
         * comparison, then opponent A-Z and newest date break the ties.
         */
        assert_eq!(ids(&rows), vec![1, 2, 3, 4]);
    }

    #[test]
    fn keeps_missing_dates_at_the_end_in_both_directions() {
        let original = vec![
            row(1, "A", "B", "", 1),
            row(2, "A", "B", "2020-01-01", 1),
            row(3, "A", "B", "2022-01-01", 1),
        ];

        let mut descending = original.clone();
        sort_search_result_rows(&mut descending, SearchResultSortColumn::Date, false);
        assert_eq!(ids(&descending), vec![3, 2, 1]);

        let mut ascending = original;
        sort_search_result_rows(&mut ascending, SearchResultSortColumn::Date, true);
        assert_eq!(ids(&ascending), vec![2, 3, 1]);
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

    fn continuation_test_pattern() -> Pattern {
        let board = Board::new(19).expect("create test board");

        Pattern::extract(
            &board,
            PatternRect {
                left: 0,
                bottom: 0,
                width: 4,
                height: 5,
            },
        )
        .expect("extract test pattern")
    }

    fn continuation_test_occurrence(transformation: PatternTransformation) -> SearchOccurrence {
        SearchOccurrence {
            move_number: 20,
            last_move_number: 20,
            side_to_move: Some(Colour::White),
            ko_point: None,
            left: Some(7),
            bottom: Some(8),
            transformation: Some(transformation),
            colours_reversed: Some(false),
        }
    }

    fn continuation_test_move(x: u8, y: u8) -> Move {
        Move {
            colour: Colour::Black,
            point: Some(u16::from(y) * 19 + u16::from(x)),
        }
    }

    #[test]
    fn selected_continuation_matches_identity_occurrence() {
        let pattern = continuation_test_pattern();
        let occurrence = continuation_test_occurrence(PatternTransformation::Identity);

        /*
         * Normalised (1, 2) at origin (7, 8) is board point (8, 10).
         */
        assert!(
            next_move_matches_selected_continuation(
                19,
                Some(continuation_test_move(8, 10)),
                &pattern,
                &occurrence,
                Some((1, 2)),
            )
            .expect("compare continuation")
        );
    }

    #[test]
    fn selected_continuation_rejects_different_move() {
        let pattern = continuation_test_pattern();
        let occurrence = continuation_test_occurrence(PatternTransformation::Identity);

        assert!(
            !next_move_matches_selected_continuation(
                19,
                Some(continuation_test_move(9, 10)),
                &pattern,
                &occurrence,
                Some((1, 2)),
            )
            .expect("compare continuation")
        );
    }

    #[test]
    fn selected_continuation_matches_rotated_occurrence() {
        let pattern = continuation_test_pattern();
        let occurrence = continuation_test_occurrence(PatternTransformation::Rotate90Clockwise);

        /*
         * For a 4 x 5 pattern, normalised (1, 2) transforms to
         * relative (2, 2). At origin (7, 8), that is board point (9, 10).
         */
        assert!(
            next_move_matches_selected_continuation(
                19,
                Some(continuation_test_move(9, 10)),
                &pattern,
                &occurrence,
                Some((1, 2)),
            )
            .expect("compare rotated continuation")
        );
    }

    #[test]
    fn selected_continuation_rejects_pass_and_game_end() {
        let pattern = continuation_test_pattern();
        let occurrence = continuation_test_occurrence(PatternTransformation::Identity);

        let pass = Move {
            colour: Colour::Black,
            point: None,
        };

        assert!(
            !next_move_matches_selected_continuation(
                19,
                Some(pass),
                &pattern,
                &occurrence,
                Some((1, 2)),
            )
            .expect("compare pass")
        );

        assert!(
            !next_move_matches_selected_continuation(
                19,
                None,
                &pattern,
                &occurrence,
                Some((1, 2)),
            )
            .expect("compare game end")
        );
    }

    #[test]
    fn occurrence_json_exposes_span_and_local_activity() {
        let occurrence = LoadedSearchOccurrence {
            occurrence: SearchOccurrence {
                move_number: 20,
                last_move_number: 25,
                side_to_move: Some(Colour::White),
                ko_point: None,
                left: Some(7),
                bottom: Some(8),
                transformation: Some(PatternTransformation::Identity),
                colours_reversed: Some(false),
            },
            continuation_points: Vec::new(),
            local_activity: LocalActivity {
                first_inside: Some(NearbyMove {
                    move_number: 26,
                    x: 8,
                    y: 9,
                    distance: 0,
                }),
                first_within_one: Some(NearbyMove {
                    move_number: 24,
                    x: 6,
                    y: 9,
                    distance: 1,
                }),
                first_within_two: Some(NearbyMove {
                    move_number: 23,
                    x: 5,
                    y: 9,
                    distance: 2,
                }),
                first_within_three: Some(NearbyMove {
                    move_number: 21,
                    x: 4,
                    y: 9,
                    distance: 3,
                }),
            },
            selected_continuation_match: true,
        };

        let json = occurrences_to_json(&[occurrence]).to_string();
        let value: serde_json::Value = serde_json::from_str(&json).expect("decode occurrence JSON");

        assert_eq!(value[0]["move"], 20);
        assert_eq!(value[0]["lastMove"], 25);
        assert_eq!(value[0]["durationMoves"], 5);
        assert_eq!(value[0]["selectedContinuationMatch"], true);

        assert_eq!(value[0]["localActivity"]["inside"]["move"], 26);
        assert_eq!(value[0]["localActivity"]["inside"]["delayMoves"], 6);
        assert_eq!(value[0]["localActivity"]["inside"]["distance"], 0);
        assert_eq!(value[0]["localActivity"]["inside"]["x"], 8);
        assert_eq!(value[0]["localActivity"]["inside"]["coreY"], 9);

        assert_eq!(value[0]["localActivity"]["withinOne"]["move"], 24);
        assert_eq!(value[0]["localActivity"]["withinOne"]["delayMoves"], 4);
        assert_eq!(value[0]["localActivity"]["withinTwo"]["move"], 23);
        assert_eq!(value[0]["localActivity"]["withinTwo"]["delayMoves"], 3);
        assert_eq!(value[0]["localActivity"]["withinThree"]["move"], 21);
        assert_eq!(value[0]["localActivity"]["withinThree"]["delayMoves"], 1);
    }

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
            include_handicap_games: false,
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
            include_handicap_games: false,
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
mod current_outcome_summary_tests {
    use super::*;

    fn result_row(game_id: i64, result: &str) -> SearchResultRow {
        SearchResultRow {
            game_id,
            result: QString::from(result),
            ..SearchResultRow::default()
        }
    }

    #[test]
    fn summarises_current_search_rows_once_per_row() {
        let rows = vec![
            result_row(1, "B+R"),
            result_row(2, "W+0.5"),
            result_row(3, "B+2.5"),
            result_row(4, "Jigo"),
            result_row(5, "Void"),
        ];

        let summary = outcome_summary_for_rows(&rows);

        assert_eq!(summary.games, 5);
        assert_eq!(summary.black_wins, 2);
        assert_eq!(summary.white_wins, 1);
        assert_eq!(summary.draws, 1);
        assert_eq!(summary.unknown, 1);
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
