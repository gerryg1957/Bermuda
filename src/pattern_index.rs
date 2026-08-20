//! Experimental packed-position representation for pattern searching.
//!
//! This module deliberately does not define a persistent index format yet.
//! It proves that the packed board words already produced during normal
//! replay can be retained without replaying the game a second time.

use crate::{GameRecord, replay_positions};
use anyhow::Result;

const BOARD_WORDS: usize = 6;

/// One already-replayed board position in the form needed by fast
/// pattern searching.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatternIndexedPosition {
    pub game_id: i64,
    pub move_number: usize,
    pub side_to_move: crate::Colour,
    pub ko_point: Option<u16>,
    pub black: [u64; BOARD_WORDS],
    pub white: [u64; BOARD_WORDS],
}

/// Replay one game once and retain its packed board representation at
/// every position.
pub fn pattern_positions_from_record(
    game_id: i64,
    record: &GameRecord,
) -> Result<Vec<PatternIndexedPosition>> {
    let positions = replay_positions(record)?
        .into_iter()
        .map(|state| PatternIndexedPosition {
            game_id,
            move_number: state.occurrence.move_number,
            side_to_move: state.occurrence.side_to_move,
            ko_point: state.occurrence.ko_point,
            black: *state.board.black_words(),
            white: *state.board.white_words(),
        })
        .collect();

    Ok(positions)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{extract_main_variation, parse_collection, replay_positions};

    fn record(sgf: &str) -> GameRecord {
        let collection = parse_collection(sgf.as_bytes()).expect("parse SGF");
        extract_main_variation(&collection).expect("extract game")
    }

    #[test]
    fn retains_packed_board_words_for_every_replayed_position() {
        let game = record(
            "(;FF[4]GM[1]SZ[19]
               ;B[pd]
               ;W[dd]
               ;B[qp]
               ;W[dp])",
        );

        let replayed = replay_positions(&game).unwrap();
        let indexed = pattern_positions_from_record(42, &game).unwrap();

        assert_eq!(indexed.len(), replayed.len());

        for (indexed_position, state) in indexed.iter().zip(&replayed) {
            assert_eq!(indexed_position.game_id, 42);
            assert_eq!(indexed_position.move_number, state.occurrence.move_number);
            assert_eq!(indexed_position.side_to_move, state.occurrence.side_to_move);
            assert_eq!(indexed_position.ko_point, state.occurrence.ko_point);
            assert_eq!(indexed_position.black, *state.board.black_words());
            assert_eq!(indexed_position.white, *state.board.white_words());
        }
    }

    #[test]
    fn packed_positions_preserve_capture_results() {
        let game = record(
            "(;FF[4]GM[1]SZ[5]
               ;B[bb]
               ;W[ab]
               ;B[ee]
               ;W[ba]
               ;B[ed]
               ;W[bc]
               ;B[de]
               ;W[cb])",
        );

        let replayed = replay_positions(&game).unwrap();
        let indexed = pattern_positions_from_record(7, &game).unwrap();

        assert_eq!(indexed.len(), replayed.len());

        let final_state = replayed.last().unwrap();
        let final_indexed = indexed.last().unwrap();

        assert_eq!(final_indexed.black, *final_state.board.black_words());
        assert_eq!(final_indexed.white, *final_state.board.white_words());
    }
}
