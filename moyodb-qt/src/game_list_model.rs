#![allow(clippy::too_many_arguments)]
use cxx_qt::CxxQtType;
use std::{fmt::Display, path::Path, pin::Pin};

use cxx_qt_lib::{QByteArray, QHash, QHashPair_i32_QByteArray, QModelIndex, QString, QVariant};

use moyodb::{
    game_list::{GameColumn, GameListQuery, SortField},
    project_manager::ProjectManager,
};

pub(crate) use crate::search_result_model::SearchResultModelRust;

#[allow(non_camel_case_types)]
type QHash_i32_QByteArray = QHash<QHashPair_i32_QByteArray>;

const GAME_ID_ROLE: i32 = 0x0100;
const BLACK_PLAYER_ROLE: i32 = GAME_ID_ROLE + 1;
const WHITE_PLAYER_ROLE: i32 = GAME_ID_ROLE + 2;
const BLACK_RANK_ROLE: i32 = GAME_ID_ROLE + 3;
const WHITE_RANK_ROLE: i32 = GAME_ID_ROLE + 4;
const PLAYED_DATE_ROLE: i32 = GAME_ID_ROLE + 5;
const RESULT_ROLE: i32 = GAME_ID_ROLE + 6;
const EVENT_ROLE: i32 = GAME_ID_ROLE + 7;
const KOMI_ROLE: i32 = GAME_ID_ROLE + 8;
const HANDICAP_ROLE: i32 = GAME_ID_ROLE + 9;

#[derive(Clone, Debug, Default)]
struct GameListRow {
    game_id: i64,
    black_player: QString,
    white_player: QString,
    black_rank: QString,
    white_rank: QString,
    played_date: QString,
    result: QString,
    event: QString,
    komi: QString,
    handicap: QString,
}

#[derive(Default)]
pub struct GameListModelRust {
    rows: Vec<GameListRow>,
    error_message: QString,
}

#[cxx_qt::bridge]
pub mod ffi {
    unsafe extern "C++Qt" {
        include!(<QtCore/QAbstractListModel>);

        #[qobject]
        type QAbstractListModel;
    }

    unsafe extern "C++" {
        include!("cxx-qt-lib/qbytearray.h");
        include!("cxx-qt-lib/qhash.h");
        include!("cxx-qt-lib/qmodelindex.h");
        include!("cxx-qt-lib/qstring.h");
        include!("cxx-qt-lib/qvariant.h");

        type QByteArray = cxx_qt_lib::QByteArray;
        type QHash_i32_QByteArray = super::QHash_i32_QByteArray;
        type QModelIndex = cxx_qt_lib::QModelIndex;
        type QString = cxx_qt_lib::QString;
        type QVariant = cxx_qt_lib::QVariant;
    }

    unsafe extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[base = QAbstractListModel]
        #[qproperty(QString, error_message)]
        type GameListModel = super::GameListModelRust;

        #[qinvokable]
        #[cxx_name = "loadProject"]
        fn load_project(self: Pin<&mut GameListModel>, project_path: &QString) -> bool;

        #[qinvokable]
        #[cxx_name = "loadSortedProject"]
        fn load_sorted_project(
            self: Pin<&mut GameListModel>,
            project_path: &QString,
            column: &QString,
            ascending: bool,
        ) -> bool;

        #[cxx_override]
        #[cxx_name = "rowCount"]
        fn row_count(self: &GameListModel, parent: &QModelIndex) -> i32;

        #[cxx_override]
        fn data(self: &GameListModel, index: &QModelIndex, role: i32) -> QVariant;

        #[cxx_override]
        #[cxx_name = "roleNames"]
        fn role_names(self: &GameListModel) -> QHash_i32_QByteArray;

        #[inherit]
        #[rust_name = "begin_reset_model"]
        fn beginResetModel(self: Pin<&mut GameListModel>);

        #[inherit]
        #[rust_name = "end_reset_model"]
        fn endResetModel(self: Pin<&mut GameListModel>);
    }

    unsafe extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[base = QAbstractListModel]
        #[qproperty(QString, error_message)]
        #[qproperty(QString, next_move_distribution_json)]
        #[qproperty(i32, total_occurrences)]
        #[qproperty(bool, search_in_progress)]
        #[qproperty(bool, occurrence_load_in_progress)]
        #[qproperty(bool, cancel_requested)]
        #[qproperty(bool, search_cancelled)]
        #[qproperty(i32, games_examined)]
        #[qproperty(i32, total_games)]
        #[qproperty(i32, matching_games)]
        #[qproperty(i32, matches_found)]
        type SearchResultModel = super::SearchResultModelRust;

        #[qinvokable]
        #[cxx_name = "searchProject"]
        fn search_project(
            self: Pin<&mut SearchResultModel>,
            project_path: &QString,
            board_size: i32,
            stones_json: &QString,
            left: i32,
            bottom: i32,
            width: i32,
            height: i32,
        ) -> bool;

        #[qinvokable]
        #[cxx_name = "clearResults"]
        fn clear_results(self: Pin<&mut SearchResultModel>);

        #[qinvokable]
        #[cxx_name = "filterContinuationAtOccurrence"]
        fn filter_continuation_at_occurrence(
            self: Pin<&mut SearchResultModel>,
            board_x: i32,
            core_y: i32,
            left: i32,
            bottom: i32,
            transformation: &QString,
        ) -> bool;

        #[qinvokable]
        #[cxx_name = "continuationGameCountAtOccurrence"]
        fn continuation_game_count_at_occurrence(
            self: Pin<&mut SearchResultModel>,
            board_x: i32,
            core_y: i32,
            left: i32,
            bottom: i32,
            transformation: &QString,
        ) -> i32;

        #[qinvokable]
        #[cxx_name = "clearContinuationFilter"]
        fn clear_continuation_filter(self: Pin<&mut SearchResultModel>);

        #[qinvokable]
        #[cxx_name = "cancelSearch"]
        fn cancel_search(self: Pin<&mut SearchResultModel>);

        #[qinvokable]
        #[cxx_name = "loadOccurrences"]
        fn load_occurrences(self: Pin<&mut SearchResultModel>, row_number: i32) -> bool;

        #[qsignal]
        #[cxx_name = "occurrencesLoaded"]
        fn occurrences_loaded(
            self: Pin<&mut SearchResultModel>,
            row_number: i32,
            occurrences_json: QString,
            error_message: QString,
        );

        #[cxx_override]
        #[cxx_name = "rowCount"]
        fn row_count(self: &SearchResultModel, parent: &QModelIndex) -> i32;

        #[cxx_override]
        fn data(self: &SearchResultModel, index: &QModelIndex, role: i32) -> QVariant;

        #[cxx_override]
        #[cxx_name = "roleNames"]
        fn role_names(self: &SearchResultModel) -> QHash_i32_QByteArray;

        #[inherit]
        #[rust_name = "begin_reset_model"]
        fn beginResetModel(self: Pin<&mut SearchResultModel>);

        #[inherit]
        #[rust_name = "end_reset_model"]
        fn endResetModel(self: Pin<&mut SearchResultModel>);
    }

    impl cxx_qt::Threading for SearchResultModel {}
}

impl ffi::GameListModel {
    fn row_count(&self, parent: &QModelIndex) -> i32 {
        if parent.is_valid() {
            return 0;
        }

        i32::try_from(self.rust().rows.len()).unwrap_or(i32::MAX)
    }

    fn data(&self, index: &QModelIndex, role: i32) -> QVariant {
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
            BLACK_RANK_ROLE => QVariant::from(&row.black_rank),
            WHITE_RANK_ROLE => QVariant::from(&row.white_rank),
            PLAYED_DATE_ROLE => QVariant::from(&row.played_date),
            RESULT_ROLE => QVariant::from(&row.result),
            EVENT_ROLE => QVariant::from(&row.event),
            KOMI_ROLE => QVariant::from(&row.komi),
            HANDICAP_ROLE => QVariant::from(&row.handicap),
            _ => QVariant::default(),
        }
    }

    fn role_names(&self) -> QHash_i32_QByteArray {
        let mut roles = QHash_i32_QByteArray::default();

        roles.insert(GAME_ID_ROLE, QByteArray::from("gameId"));
        roles.insert(BLACK_PLAYER_ROLE, QByteArray::from("blackPlayer"));
        roles.insert(WHITE_PLAYER_ROLE, QByteArray::from("whitePlayer"));
        roles.insert(BLACK_RANK_ROLE, QByteArray::from("blackRank"));
        roles.insert(WHITE_RANK_ROLE, QByteArray::from("whiteRank"));
        roles.insert(PLAYED_DATE_ROLE, QByteArray::from("playedDate"));
        roles.insert(RESULT_ROLE, QByteArray::from("result"));
        roles.insert(EVENT_ROLE, QByteArray::from("event"));
        roles.insert(KOMI_ROLE, QByteArray::from("komi"));
        roles.insert(HANDICAP_ROLE, QByteArray::from("handicap"));

        roles
    }

    fn load_project(mut self: Pin<&mut Self>, project_path: &QString) -> bool {
        self.as_mut()
            .load_with_query(project_path, GameListQuery::default())
    }

    fn load_sorted_project(
        mut self: Pin<&mut Self>,
        project_path: &QString,
        column: &QString,
        ascending: bool,
    ) -> bool {
        let column_name = column.to_string();

        let game_column = match column_name.as_str() {
            "black" => GameColumn::BlackPlayer,
            "white" => GameColumn::WhitePlayer,
            "date" => GameColumn::Date,
            "result" => GameColumn::Result,
            "event" => GameColumn::Event,

            _ => {
                self.as_mut().rust_mut().error_message =
                    QString::from(format!("unknown sort column: {column_name}"));

                return false;
            }
        };

        let primary_sort = if ascending {
            SortField::ascending(game_column)
        } else {
            SortField::descending(game_column)
        };

        let query = GameListQuery {
            sort_fields: vec![primary_sort],
            ..GameListQuery::default()
        };

        self.as_mut().load_with_query(project_path, query)
    }

    fn load_with_query(
        mut self: Pin<&mut Self>,
        project_path: &QString,
        query: GameListQuery,
    ) -> bool {
        self.as_mut().begin_reset_model();

        {
            let mut rust = self.as_mut().rust_mut();
            rust.rows.clear();
            rust.error_message = QString::default();
        }

        let path = project_path.to_string();

        if path.trim().is_empty() {
            self.as_mut().end_reset_model();
            return false;
        }

        let result = ProjectManager::new()
            .open(Path::new(&path))
            .and_then(|project| project.catalogue())
            .and_then(|catalogue| catalogue.list(&query));

        let loaded = match result {
            Ok(games) => {
                let rows = games
                    .into_iter()
                    .map(|game| GameListRow {
                        game_id: game.game_id,
                        black_player: optional_text(&game.black_player),
                        white_player: optional_text(&game.white_player),
                        black_rank: QString::default(),
                        white_rank: QString::default(),
                        played_date: optional_text(&game.game_date),
                        result: optional_text(&game.result),
                        event: optional_text(&game.event),
                        komi: optional_number(&game.komi),
                        handicap: QString::default(),
                    })
                    .collect();

                self.as_mut().rust_mut().rows = rows;
                true
            }

            Err(error) => {
                self.as_mut().rust_mut().error_message = QString::from(error.to_string());

                false
            }
        };

        self.as_mut().end_reset_model();

        loaded
    }
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
