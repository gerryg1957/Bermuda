#![allow(clippy::too_many_arguments)]

use std::{
    fmt::Write as _,
    fs,
    path::{Path, PathBuf},
    pin::Pin,
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use bermuda::{
    Board, Colour, GameRecord, Metadata, Move, PositionOccurrence, PositionState,
    extract_main_variation, importer::ImportOutcome, indexer::POSITION_INDEX_VERSION,
    parse_collection, position_fingerprint, project::Project, project_manager::ProjectManager,
    replay_positions, write_game_record_sgf,
};
use cxx_qt::CxxQtType;
use cxx_qt_lib::QString;
use directories::ProjectDirs;

const PERSONAL_PROJECT_NAME: &str = "My Games";
const PERSONAL_PROJECT_DIRECTORY: &str = "personal-corpus";

const PLAYED_GAME_SOURCE_NAME: &str = "Bermuda";
const PLAYED_GAME_SOURCE_VERSION: &str = "play-v1";

static PLAYED_GAME_LOCATOR_SEQUENCE: AtomicU64 = AtomicU64::new(0);

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
        #[qproperty(QString, error_message)]
        type BermudaApp = super::BermudaAppRust;

        #[qinvokable]
        #[cxx_name = "projectExists"]
        fn project_exists(self: &BermudaApp, project_path: &QString) -> bool;

        #[qinvokable]
        #[cxx_name = "personalProjectPath"]
        fn personal_project_path(self: &BermudaApp) -> QString;

        #[qinvokable]
        #[cxx_name = "loadGame"]
        fn load_game(self: Pin<&mut BermudaApp>, project_path: &QString, game_id: i64) -> bool;

        #[qinvokable]
        #[cxx_name = "loadSgf"]
        fn load_sgf(self: Pin<&mut BermudaApp>, sgf_path: &QString) -> bool;

        #[qinvokable]
        #[cxx_name = "savePlayedGameSgf"]
        fn save_played_game_sgf(self: Pin<&mut BermudaApp>, sgf_path: &QString) -> bool;

        #[qinvokable]
        #[cxx_name = "addPlayedGameToMyGames"]
        fn add_played_game_to_my_games(self: Pin<&mut BermudaApp>) -> bool;

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
        #[cxx_name = "editPositionPoint"]
        fn edit_position_point(self: Pin<&mut BermudaApp>, x: i32, y: i32, tool: &QString) -> bool;

        #[qinvokable]
        #[cxx_name = "showPosition"]
        fn show_position(self: Pin<&mut BermudaApp>, move_number: i32) -> bool;

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
}

#[derive(Debug, Clone)]
struct LoadedDocument {
    description: String,
    positions: Vec<PositionState>,
    editable: bool,
    playable: bool,
    finished: bool,
    result: Option<String>,
    black_player: Option<String>,
    white_player: Option<String>,
    komi: Option<f32>,
    played_source_locator: Option<String>,
}

#[derive(Debug)]
struct SearchSourceSnapshot {
    document: LoadedDocument,
    move_number: i32,
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
    error_message: QString,

    loaded_document: Option<LoadedDocument>,
    search_source_snapshot: Option<SearchSourceSnapshot>,
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
            error_message: QString::default(),

            loaded_document: None,
            search_source_snapshot: None,
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

    fn load_game(mut self: Pin<&mut Self>, project_path: &QString, game_id: i64) -> bool {
        let path = project_path.to_string();

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
        self.as_mut().show_cached_position(move_number)
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

        let result = {
            let self_ref = self.as_ref();
            let rust = self_ref.rust();

            match rust.loaded_document.as_ref() {
                Some(document) => {
                    position_data(&document.positions, &document.description, move_number)
                }

                None => Err("no game is loaded".to_owned()),
            }
        };

        match result {
            Ok(position) => {
                self.as_mut().set_board_size(position.board_size);

                self.as_mut().set_stones_json(position.stones_json);

                self.as_mut().set_move_number(position.move_number);

                self.as_mut().set_move_count(position.move_count);

                self.as_mut().set_last_move_x(position.last_move_x);
                self.as_mut().set_last_move_y(position.last_move_y);

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
    }
}

struct LoadedPosition {
    board_size: i32,
    stones_json: QString,
    move_number: i32,
    move_count: i32,
    last_move_x: i32,
    last_move_y: i32,
}

fn load_game_document(project_path: &str, game_id: i64) -> Result<LoadedDocument, String> {
    let project = ProjectManager::new()
        .open(Path::new(project_path))
        .map_err(|error| error.to_string())?;

    let store = project.game_store().map_err(|error| error.to_string())?;

    let positions = store
        .positions(game_id)
        .map_err(|error| error.to_string())?;

    Ok(LoadedDocument {
        description: format!("game {game_id}"),
        positions,
        editable: false,
        playable: false,
        finished: false,
        result: None,
        black_player: None,
        white_player: None,
        komi: None,
        played_source_locator: None,
    })
}

fn load_sgf_document(sgf_path: &str) -> Result<LoadedDocument, String> {
    let path = Path::new(sgf_path);

    let bytes = fs::read(path).map_err(|error| format!("reading {}: {error}", path.display()))?;

    let collection =
        parse_collection(&bytes).map_err(|error| format!("parsing {}: {error}", path.display()))?;

    let record = extract_main_variation(&collection).map_err(|error| {
        format!(
            "extracting the main variation from {}: {error}",
            path.display()
        )
    })?;

    let positions = replay_positions(&record)
        .map_err(|error| format!("replaying {}: {error}", path.display()))?;

    Ok(LoadedDocument {
        description: format!("SGF {}", path.display()),
        positions,
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

fn new_position_document(board_size: i32) -> Result<LoadedDocument, String> {
    let board_size =
        u8::try_from(board_size).map_err(|_| format!("invalid board size {board_size}"))?;

    let board = Board::new(board_size).map_err(|error| error.to_string())?;

    Ok(LoadedDocument {
        description: "untitled position".to_owned(),
        positions: vec![editable_position_state(board)],
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
