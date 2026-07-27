use crate::{
    board::{Board, BoardError, Colour, Move},
    sgf::{Collection, GameTree, Node},
};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq)]
pub struct Metadata {
    pub black_player: Option<String>,
    pub white_player: Option<String>,
    pub date: Option<String>,
    pub event: Option<String>,
    pub result: Option<String>,
    pub komi: Option<f32>,
    pub handicap: Option<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SetupStone {
    Add { colour: Colour, point: u16 },
    Remove { point: u16 },
}

#[derive(Debug, Clone, PartialEq)]
pub struct GameRecord {
    pub board_size: u8,
    pub metadata: Metadata,
    pub setup: Vec<SetupStone>,
    pub moves: Vec<Move>,
}

#[derive(Debug, Error)]
pub enum GameError {
    #[error("collection contains no game trees")]
    EmptyCollection,
    #[error("root property GM must be 1 for Go")]
    NotGo,
    #[error("invalid board size '{0}'")]
    InvalidBoardSize(String),
    #[error("unsupported board size {0}")]
    UnsupportedBoardSize(u8),
    #[error("invalid SGF coordinate '{value}' for board size {size}")]
    InvalidCoordinate { value: String, size: u8 },
    #[error("node contains both B and W moves")]
    TwoMovesInNode,
    #[error("board replay failed at move {move_number}: {source}")]
    Replay {
        move_number: usize,
        #[source]
        source: BoardError,
    },
}

pub fn extract_main_variation(collection: &Collection) -> Result<GameRecord, GameError> {
    let tree = collection.trees.first().ok_or(GameError::EmptyCollection)?;
    extract_tree_main_variation(tree)
}

fn extract_tree_main_variation(tree: &GameTree) -> Result<GameRecord, GameError> {
    let root = tree
        .sequence
        .first()
        .expect("parser guarantees non-empty sequence");
    if let Some(gm) = root.first("GM")
        && gm != "1"
    {
        return Err(GameError::NotGo);
    }
    let board_size = match root.first("SZ") {
        None => 19,
        Some(value) => value
            .parse::<u8>()
            .map_err(|_| GameError::InvalidBoardSize(value.to_owned()))?,
    };
    if !(1..=19).contains(&board_size) {
        return Err(GameError::UnsupportedBoardSize(board_size));
    }

    let metadata = Metadata {
        black_player: owned(root.first("PB")),
        white_player: owned(root.first("PW")),
        date: owned(root.first("DT")),
        event: owned(root.first("EV")),
        result: owned(root.first("RE")),
        komi: root.first("KM").and_then(|v| v.parse().ok()),
        handicap: root.first("HA").and_then(|v| v.parse().ok()),
    };

    let mut setup = Vec::new();
    let mut moves = Vec::new();
    collect_sequence(&tree.sequence, board_size, &mut setup, &mut moves)?;

    // SGF's "main variation" is conventionally the first child variation.
    let mut branch = tree.variations.first();
    while let Some(next) = branch {
        collect_sequence(&next.sequence, board_size, &mut setup, &mut moves)?;
        branch = next.variations.first();
    }

    Ok(GameRecord {
        board_size,
        metadata,
        setup,
        moves,
    })
}

fn collect_sequence(
    nodes: &[Node],
    size: u8,
    setup: &mut Vec<SetupStone>,
    moves: &mut Vec<Move>,
) -> Result<(), GameError> {
    for node in nodes {
        for value in node.values("AB") {
            setup.push(SetupStone::Add {
                colour: Colour::Black,
                point: coordinate(value, size)?.ok_or_else(|| GameError::InvalidCoordinate {
                    value: value.clone(),
                    size,
                })?,
            });
        }
        for value in node.values("AW") {
            setup.push(SetupStone::Add {
                colour: Colour::White,
                point: coordinate(value, size)?.ok_or_else(|| GameError::InvalidCoordinate {
                    value: value.clone(),
                    size,
                })?,
            });
        }
        for value in node.values("AE") {
            setup.push(SetupStone::Remove {
                point: coordinate(value, size)?.ok_or_else(|| GameError::InvalidCoordinate {
                    value: value.clone(),
                    size,
                })?,
            });
        }

        let black = node.first("B");
        let white = node.first("W");
        if black.is_some() && white.is_some() {
            return Err(GameError::TwoMovesInNode);
        }
        if let Some(value) = black {
            moves.push(Move {
                colour: Colour::Black,
                point: move_coordinate(value, size)?,
            });
        }
        if let Some(value) = white {
            moves.push(Move {
                colour: Colour::White,
                point: move_coordinate(value, size)?,
            });
        }
    }
    Ok(())
}

pub fn replay(record: &GameRecord) -> Result<Board, GameError> {
    let mut board = Board::new(record.board_size).map_err(|source| GameError::Replay {
        move_number: 0,
        source,
    })?;
    for stone in &record.setup {
        match *stone {
            SetupStone::Add { colour, point } => board.set_setup(colour, point),
            SetupStone::Remove { point } => board.clear_setup(point),
        }
        .map_err(|source| GameError::Replay {
            move_number: 0,
            source,
        })?;
    }
    for (index, &mv) in record.moves.iter().enumerate() {
        board.play(mv).map_err(|source| GameError::Replay {
            move_number: index + 1,
            source,
        })?;
    }
    Ok(board)
}
fn move_coordinate(value: &str, size: u8) -> Result<Option<u16>, GameError> {
    if value.is_empty() || (size <= 19 && value.eq_ignore_ascii_case("tt")) {
        return Ok(None);
    }

    coordinate(value, size)
}

fn coordinate(value: &str, size: u8) -> Result<Option<u16>, GameError> {
    if value.is_empty() {
        return Ok(None);
    }
    let bytes = value.as_bytes();
    if bytes.len() != 2 {
        return Err(GameError::InvalidCoordinate {
            value: value.to_owned(),
            size,
        });
    }
    let decode = |b: u8| -> Option<u8> {
        match b {
            b'a'..=b'z' => Some(b - b'a'),
            b'A'..=b'Z' => Some(26 + b - b'A'),
            _ => None,
        }
    };
    let x = decode(bytes[0]).ok_or_else(|| GameError::InvalidCoordinate {
        value: value.to_owned(),
        size,
    })?;
    let y = decode(bytes[1]).ok_or_else(|| GameError::InvalidCoordinate {
        value: value.to_owned(),
        size,
    })?;
    if x >= size || y >= size {
        return Err(GameError::InvalidCoordinate {
            value: value.to_owned(),
            size,
        });
    }
    Ok(Some(u16::from(y) * u16::from(size) + u16::from(x)))
}

fn owned(value: Option<&str>) -> Option<String> {
    value.map(ToOwned::to_owned)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse_collection;

    #[test]
    fn accepts_legacy_tt_as_a_pass_on_19x19() {
        let collection = parse_collection(b"(;FF[4]GM[1]SZ[19];B[pd];W[tt];B[dd])").unwrap();

        let record = extract_main_variation(&collection).unwrap();

        assert_eq!(record.moves.len(), 3);
        assert_eq!(record.moves[1].point, None);
    }

    #[test]
    fn does_not_accept_tt_as_a_setup_point() {
        let collection = parse_collection(b"(;FF[4]GM[1]SZ[19]AB[tt])").unwrap();

        assert!(extract_main_variation(&collection).is_err());
    }
}
