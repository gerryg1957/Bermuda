use crate::{
    board::{Colour, MAX_BOARD_SIZE},
    game::{GameRecord, SetupStone},
};
use thiserror::Error;

#[derive(Debug, Error, PartialEq)]
pub enum SgfWriteError {
    #[error("unsupported board size {0}; expected 1..={MAX_BOARD_SIZE}")]
    UnsupportedBoardSize(u8),

    #[error("point {point} lies outside a {size}x{size} board")]
    PointOutside { point: u16, size: u8 },

    #[error("komi must be a finite number, got {0}")]
    NonFiniteKomi(f32),
}

/// Serialise Bermuda's neutral GameRecord as one SGF FF[4] game tree.
///
/// SGF is an interchange format.  GameRecord remains Bermuda's
/// internal representation of a game.
pub fn write_game_record_sgf(record: &GameRecord) -> Result<String, SgfWriteError> {
    validate_board_size(record.board_size)?;

    if let Some(komi) = record.metadata.komi
        && !komi.is_finite()
    {
        return Err(SgfWriteError::NonFiniteKomi(komi));
    }

    let mut output = String::new();

    /*
     * Root node.
     *
     * CA[UTF-8] makes explicit the encoding Rust Strings naturally use.
     */
    output.push_str("(;GM[1]FF[4]CA[UTF-8]SZ[");
    output.push_str(&record.board_size.to_string());
    output.push(']');

    push_optional_property(&mut output, "PB", record.metadata.black_player.as_deref());

    push_optional_property(&mut output, "PW", record.metadata.white_player.as_deref());

    push_optional_property(&mut output, "DT", record.metadata.date.as_deref());

    push_optional_property(&mut output, "EV", record.metadata.event.as_deref());

    push_optional_property(&mut output, "RE", record.metadata.result.as_deref());

    if let Some(komi) = record.metadata.komi {
        push_property(&mut output, "KM", &komi.to_string());
    }

    if let Some(handicap) = record.metadata.handicap {
        push_property(&mut output, "HA", &handicap.to_string());
    }

    /*
     * Keep each setup operation in its own node.
     *
     * GameRecord stores setup as an ordered Vec.  Writing each entry
     * separately means parsing our SGF reconstructs exactly that Vec,
     * even when a record contains successive operations on one point.
     */
    for stone in &record.setup {
        output.push(';');

        match *stone {
            SetupStone::Add {
                colour: Colour::Black,
                point,
            } => {
                push_coordinate_property(&mut output, "AB", point, record.board_size)?;
            }

            SetupStone::Add {
                colour: Colour::White,
                point,
            } => {
                push_coordinate_property(&mut output, "AW", point, record.board_size)?;
            }

            SetupStone::Remove { point } => {
                push_coordinate_property(&mut output, "AE", point, record.board_size)?;
            }
        }
    }

    /*
     * Every move gets its own node.
     *
     * SGF FF[4] represents a pass with an empty coordinate: B[] / W[].
     */
    for mv in &record.moves {
        output.push(';');

        let property = match mv.colour {
            Colour::Black => "B",
            Colour::White => "W",
        };

        output.push_str(property);
        output.push('[');

        if let Some(point) = mv.point {
            output.push_str(&sgf_coordinate(point, record.board_size)?);
        }

        output.push(']');
    }

    output.push_str(")\n");

    Ok(output)
}

fn validate_board_size(size: u8) -> Result<(), SgfWriteError> {
    if !(1..=MAX_BOARD_SIZE).contains(&size) {
        return Err(SgfWriteError::UnsupportedBoardSize(size));
    }

    Ok(())
}

fn push_optional_property(output: &mut String, identifier: &str, value: Option<&str>) {
    if let Some(value) = value {
        push_property(output, identifier, value);
    }
}

fn push_property(output: &mut String, identifier: &str, value: &str) {
    output.push_str(identifier);
    output.push('[');
    output.push_str(&escape_sgf_value(value));
    output.push(']');
}

fn push_coordinate_property(
    output: &mut String,
    identifier: &str,
    point: u16,
    size: u8,
) -> Result<(), SgfWriteError> {
    output.push_str(identifier);
    output.push('[');
    output.push_str(&sgf_coordinate(point, size)?);
    output.push(']');

    Ok(())
}

fn sgf_coordinate(point: u16, size: u8) -> Result<String, SgfWriteError> {
    validate_board_size(size)?;

    let size_u16 = u16::from(size);
    let point_count = size_u16 * size_u16;

    if point >= point_count {
        return Err(SgfWriteError::PointOutside { point, size });
    }

    let x = (point % size_u16) as u8;
    let y = (point / size_u16) as u8;

    /*
     * Bermuda currently supports boards up to 19x19, so SGF
     * coordinates are always in the lower-case a..s range.
     */
    let mut coordinate = String::with_capacity(2);
    coordinate.push((b'a' + x) as char);
    coordinate.push((b'a' + y) as char);

    Ok(coordinate)
}

fn escape_sgf_value(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    let mut characters = value.chars().peekable();

    while let Some(character) = characters.next() {
        match character {
            '\\' => escaped.push_str("\\\\"),
            ']' => escaped.push_str("\\]"),

            /*
             * SGF normalises line endings.  Emit LF consistently.
             */
            '\r' => {
                if characters.peek() == Some(&'\n') {
                    characters.next();
                }

                escaped.push('\n');
            }

            other => escaped.push(other),
        }
    }

    escaped
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::{Metadata, Move, extract_main_variation, parse_collection};

    #[test]
    fn game_record_round_trip_preserves_all_supported_data() {
        let record = GameRecord {
            board_size: 19,

            metadata: Metadata {
                black_player: Some(r"Black ] \ Player".to_owned()),

                white_player: Some("White Player".to_owned()),

                date: Some("2026-08-30".to_owned()),

                event: Some(r"Test ] Event \ London".to_owned()),

                result: Some("B+6.5".to_owned()),
                komi: Some(6.5),
                handicap: Some(2),
            },

            /*
             * Individual setup nodes preserve this exact order.
             */
            setup: vec![
                SetupStone::Add {
                    colour: Colour::Black,
                    point: 0,
                },
                SetupStone::Add {
                    colour: Colour::White,
                    point: 360,
                },
                SetupStone::Remove { point: 180 },
            ],

            moves: vec![
                Move {
                    colour: Colour::Black,
                    point: Some(3 * 19 + 3),
                },
                Move {
                    colour: Colour::White,
                    point: None,
                },
                Move {
                    colour: Colour::Black,
                    point: Some(15 * 19 + 15),
                },
            ],
        };

        let sgf = write_game_record_sgf(&record).expect("write SGF");

        let collection = parse_collection(sgf.as_bytes()).expect("parse written SGF");

        let restored = extract_main_variation(&collection).expect("extract written SGF");

        assert_eq!(restored, record);
    }

    #[test]
    fn writes_ff4_utf8_and_empty_coordinate_for_pass() {
        let record = GameRecord {
            board_size: 19,

            metadata: Metadata {
                black_player: None,
                white_player: None,
                date: None,
                event: None,
                result: None,
                komi: Some(6.5),
                handicap: None,
            },

            setup: Vec::new(),

            moves: vec![
                Move {
                    colour: Colour::Black,
                    point: Some(0),
                },
                Move {
                    colour: Colour::White,
                    point: None,
                },
            ],
        };

        let sgf = write_game_record_sgf(&record).expect("write SGF");

        assert!(sgf.starts_with("(;GM[1]FF[4]CA[UTF-8]SZ[19]"));

        assert!(sgf.contains(";B[aa]"));
        assert!(sgf.contains(";W[]"));
    }

    #[test]
    fn escapes_closing_bracket_and_backslash() {
        let record = GameRecord {
            board_size: 19,

            metadata: Metadata {
                black_player: Some(r"A]B\C".to_owned()),
                white_player: None,
                date: None,
                event: None,
                result: None,
                komi: None,
                handicap: None,
            },

            setup: Vec::new(),
            moves: Vec::new(),
        };

        let sgf = write_game_record_sgf(&record).expect("write SGF");

        assert!(sgf.contains(r"PB[A\]B\\C]"));

        let collection = parse_collection(sgf.as_bytes()).expect("parse SGF");

        let restored = extract_main_variation(&collection).expect("extract SGF");

        assert_eq!(restored.metadata.black_player.as_deref(), Some(r"A]B\C"),);
    }

    #[test]
    fn rejects_point_outside_board() {
        let record = GameRecord {
            board_size: 19,

            metadata: Metadata {
                black_player: None,
                white_player: None,
                date: None,
                event: None,
                result: None,
                komi: None,
                handicap: None,
            },

            setup: Vec::new(),

            moves: vec![Move {
                colour: Colour::Black,
                point: Some(361),
            }],
        };

        assert_eq!(
            write_game_record_sgf(&record),
            Err(SgfWriteError::PointOutside {
                point: 361,
                size: 19,
            }),
        );
    }

    #[test]
    fn rejects_invalid_board_size() {
        let record = GameRecord {
            board_size: 0,

            metadata: Metadata {
                black_player: None,
                white_player: None,
                date: None,
                event: None,
                result: None,
                komi: None,
                handicap: None,
            },

            setup: Vec::new(),
            moves: Vec::new(),
        };

        assert_eq!(
            write_game_record_sgf(&record),
            Err(SgfWriteError::UnsupportedBoardSize(0)),
        );
    }

    #[test]
    fn rejects_non_finite_komi() {
        let record = GameRecord {
            board_size: 19,

            metadata: Metadata {
                black_player: None,
                white_player: None,
                date: None,
                event: None,
                result: None,
                komi: Some(f32::INFINITY),
                handicap: None,
            },

            setup: Vec::new(),
            moves: Vec::new(),
        };

        assert_eq!(
            write_game_record_sgf(&record),
            Err(SgfWriteError::NonFiniteKomi(f32::INFINITY)),
        );
    }
}
