use std::{fmt::Write as _, fs, path::Path, pin::Pin};

use cxx_qt::CxxQtType;
use cxx_qt_lib::QString;
use moyodb::{
    Board, Colour, PositionOccurrence, PositionState, extract_main_variation, parse_collection,
    position_fingerprint, project_manager::ProjectManager, replay_positions,
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
        #[qproperty(i32, board_size)]
        #[qproperty(QString, stones_json)]
        #[qproperty(i32, move_number)]
        #[qproperty(i32, move_count)]
        #[qproperty(i32, last_move_x)]
        #[qproperty(i32, last_move_y)]
        #[qproperty(QString, error_message)]
        type MoyoDbApp = super::MoyoDbAppRust;

        #[qinvokable]
        #[cxx_name = "loadGame"]
        fn load_game(self: Pin<&mut MoyoDbApp>, project_path: &QString, game_id: i64) -> bool;

        #[qinvokable]
        #[cxx_name = "loadSgf"]
        fn load_sgf(self: Pin<&mut MoyoDbApp>, sgf_path: &QString) -> bool;

        #[qinvokable]
        #[cxx_name = "newPosition"]
        fn new_position(self: Pin<&mut MoyoDbApp>, board_size: i32) -> bool;

        #[qinvokable]
        #[cxx_name = "editPositionPoint"]
        fn edit_position_point(self: Pin<&mut MoyoDbApp>, x: i32, y: i32, tool: &QString) -> bool;

        #[qinvokable]
        #[cxx_name = "showPosition"]
        fn show_position(self: Pin<&mut MoyoDbApp>, move_number: i32) -> bool;
    }
}

struct LoadedDocument {
    description: String,
    positions: Vec<PositionState>,
    editable: bool,
}

pub struct MoyoDbAppRust {
    board_size: i32,
    stones_json: QString,
    move_number: i32,
    move_count: i32,
    last_move_x: i32,
    last_move_y: i32,
    error_message: QString,

    loaded_document: Option<LoadedDocument>,
}

impl Default for MoyoDbAppRust {
    fn default() -> Self {
        Self {
            board_size: 19,
            stones_json: QString::from("[]"),
            move_number: 0,
            move_count: 0,
            last_move_x: -1,
            last_move_y: -1,
            error_message: QString::default(),

            loaded_document: None,
        }
    }
}

impl ffi::MoyoDbApp {
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

        self.as_mut().rust_mut().loaded_document = Some(document);

        self.as_mut().show_cached_position(0)
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

        self.as_mut().rust_mut().loaded_document = Some(document);

        self.as_mut().show_cached_position(0)
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
    })
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

    let y = u8::try_from(y).map_err(|_| format!("invalid board coordinate {x},{y}"))?;

    let point = position
        .board
        .point(x, y)
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

            (i32::from(point % size), i32::from(point / size))
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
