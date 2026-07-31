use std::{fmt::Write as _, path::Path, pin::Pin};

use cxx_qt::CxxQtType;
use cxx_qt_lib::QString;
use moyodb::{Board, Colour, PositionState, project_manager::ProjectManager};

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
        #[qproperty(QString, error_message)]
        type MoyoDbApp = super::MoyoDbAppRust;

        #[qinvokable]
        #[cxx_name = "loadGame"]
        fn load_game(self: Pin<&mut MoyoDbApp>, project_path: &QString, game_id: i64) -> bool;

        #[qinvokable]
        #[cxx_name = "showPosition"]
        fn show_position(
            self: Pin<&mut MoyoDbApp>,
            project_path: &QString,
            game_id: i64,
            move_number: i32,
        ) -> bool;
    }
}

pub struct MoyoDbAppRust {
    board_size: i32,
    stones_json: QString,
    move_number: i32,
    move_count: i32,
    error_message: QString,

    cached_project_path: String,
    cached_game_id: Option<i64>,
    positions: Vec<PositionState>,
}

impl Default for MoyoDbAppRust {
    fn default() -> Self {
               Self {
            board_size: 19,
            stones_json: QString::from("[]"),
            move_number: 0,
            move_count: 0,
            error_message: QString::default(),

            cached_project_path: String::new(),
            cached_game_id: None,
            positions: Vec::new(),
        }
    }
}

impl ffi::MoyoDbApp {
    fn load_game(
        mut self: Pin<&mut Self>,
        project_path: &QString,
        game_id: i64,
    ) -> bool {
        let path = project_path.to_string();

        let positions = match load_game_positions(&path, game_id) {
            Ok(positions) => positions,

            Err(error) => {
                {
                    let mut rust = self.as_mut().rust_mut();
                    rust.cached_project_path.clear();
                    rust.cached_game_id = None;
                    rust.positions.clear();
                }

                self.as_mut().reset_position_display();
                self.as_mut()
                    .set_error_message(QString::from(error));

                return false;
            }
        };

        {
            let mut rust = self.as_mut().rust_mut();
            rust.cached_project_path = path;
            rust.cached_game_id = Some(game_id);
            rust.positions = positions;
        }

        self.as_mut().show_cached_position(0)
    }

    fn show_position(
        mut self: Pin<&mut Self>,
        project_path: &QString,
        game_id: i64,
        move_number: i32,
    ) -> bool {
        let path = project_path.to_string();

        let cache_matches = {
            let rust = self.as_ref().rust();

            rust.cached_game_id == Some(game_id)
                && rust.cached_project_path == path
        };

        if !cache_matches {
            let positions = match load_game_positions(&path, game_id) {
                Ok(positions) => positions,

                Err(error) => {
                    {
                        let mut rust = self.as_mut().rust_mut();
                        rust.cached_project_path.clear();
                        rust.cached_game_id = None;
                        rust.positions.clear();
                    }

                    self.as_mut().reset_position_display();
                    self.as_mut()
                        .set_error_message(QString::from(error));

                    return false;
                }
            };

            {
                let mut rust = self.as_mut().rust_mut();
                rust.cached_project_path = path;
                rust.cached_game_id = Some(game_id);
                rust.positions = positions;
            }
        }

        self.as_mut().show_cached_position(move_number)
    }

    fn show_cached_position(
        mut self: Pin<&mut Self>,
        move_number: i32,
    ) -> bool {
        self.as_mut()
            .set_error_message(QString::default());

        let result = {
            let rust = self.as_ref().rust();

            match rust.cached_game_id {
                Some(game_id) => {
                    position_data(&rust.positions, game_id, move_number)
                }

                None => Err("no game is loaded".to_owned()),
            }
        };

        match result {
            Ok(position) => {
                self.as_mut()
                    .set_board_size(position.board_size);

                self.as_mut()
                    .set_stones_json(position.stones_json);

                self.as_mut()
                    .set_move_number(position.move_number);

                self.as_mut()
                    .set_move_count(position.move_count);

                true
            }

            Err(error) => {
                self.as_mut()
                    .set_error_message(QString::from(error));

                false
            }
        }
    }

    fn reset_position_display(mut self: Pin<&mut Self>) {
        self.as_mut().set_board_size(19);
        self.as_mut()
            .set_stones_json(QString::from("[]"));
        self.as_mut().set_move_number(0);
        self.as_mut().set_move_count(0);
    }
}

struct LoadedPosition {
    board_size: i32,
    stones_json: QString,
    move_number: i32,
    move_count: i32,
}

fn load_game_positions(
    project_path: &str,
    game_id: i64,
) -> Result<Vec<PositionState>, String> {
    let project = ProjectManager::new()
        .open(Path::new(project_path))
        .map_err(|error| error.to_string())?;

    let store = project
        .game_store()
        .map_err(|error| error.to_string())?;

    store
        .positions(game_id)
        .map_err(|error| error.to_string())
}

fn position_data(
    positions: &[PositionState],
    game_id: i64,
    move_number: i32,
) -> Result<LoadedPosition, String> {
    let requested_move = usize::try_from(move_number)
        .map_err(|_| "move number cannot be negative".to_owned())?;

    let move_count_usize = positions.len().saturating_sub(1);

    let position = positions.get(requested_move).ok_or_else(|| {
        format!(
            "requested move {move_number}, but game {game_id} contains only \
             {move_count_usize} moves"
        )
    })?;

    let board_size = i32::from(position.board.size());

    let current_move =
        i32::try_from(position.occurrence.move_number).map_err(|_| {
            "move number is too large for the Qt interface".to_owned()
        })?;

    let move_count = i32::try_from(move_count_usize).map_err(|_| {
        "game contains too many moves for the Qt interface".to_owned()
    })?;

    Ok(LoadedPosition {
        board_size,
        stones_json: board_stones_json(&position.board),
        move_number: current_move,
        move_count,
    })
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
        let y = point / size;

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
