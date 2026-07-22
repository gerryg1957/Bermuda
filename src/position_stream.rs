use crate::{GameRecord, replay::replay_positions};
use anyhow::Result;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PositionOccurrence {
    /// Zero means the initial position after setup stones.
    pub move_number: usize,

    /// The player expected to move from this position.
    pub side_to_move: crate::Color,

    /// Simple-ko prohibition in this position, if any.
    pub ko_point: Option<u16>,

    /// Exact-position fingerprint.
    pub fingerprint: [u8; 32],
}

/// Replays a game and returns every position reached.
///
/// Position zero is the initial board after setup stones and before the first
/// move. A game with N moves therefore produces N + 1 occurrences.
///
/// Passes also produce occurrences because side-to-move and ko state may
/// change even though the stones remain unchanged.
pub fn position_stream(record: &GameRecord) -> Result<Vec<PositionOccurrence>> {
    Ok(replay_positions(record)?
        .into_iter()
        .map(|state| state.occurrence)
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Color, extract_main_variation, parse_collection};

    fn record(sgf: &str) -> GameRecord {
        let collection = parse_collection(sgf.as_bytes()).expect("parse SGF");
        extract_main_variation(&collection).expect("extract game")
    }

    #[test]
    fn emits_initial_and_post_move_positions() {
        let game = record(
            "(;FF[4]GM[1]SZ[19]
               ;B[pd]
               ;W[dd]
               ;B[qp])",
        );

        let stream = position_stream(&game).unwrap();

        assert_eq!(stream.len(), 4);

        assert_eq!(stream[0].move_number, 0);
        assert_eq!(stream[0].side_to_move, Color::Black);

        assert_eq!(stream[1].move_number, 1);
        assert_eq!(stream[1].side_to_move, Color::White);

        assert_eq!(stream[2].move_number, 2);
        assert_eq!(stream[2].side_to_move, Color::Black);

        assert_eq!(stream[3].move_number, 3);
        assert_eq!(stream[3].side_to_move, Color::White);
    }

    #[test]
    fn uses_recorded_next_colour_in_non_alternating_games() {
        let game = record(
            "(;FF[4]GM[1]SZ[19]
               ;B[pd]
               ;B[dd]
               ;W[qp])",
        );

        let stream = position_stream(&game).unwrap();

        assert_eq!(stream[1].side_to_move, Color::Black);
        assert_eq!(stream[2].side_to_move, Color::White);
    }

    #[test]
    fn includes_setup_stones_in_initial_position() {
        let game = record(
            "(;FF[4]GM[1]SZ[19]
               AB[dd][pd]
               AW[dp]
               ;W[qp])",
        );

        let stream = position_stream(&game).unwrap();

        assert_eq!(stream.len(), 2);
        assert_eq!(stream[0].move_number, 0);
        assert_eq!(stream[0].side_to_move, Color::White);
    }

    #[test]
    fn pass_still_emits_a_new_occurrence() {
        let game = record(
            "(;FF[4]GM[1]SZ[19]
               ;B[pd]
               ;W[]
               ;B[dd])",
        );

        let stream = position_stream(&game).unwrap();

        assert_eq!(stream.len(), 4);
        assert_eq!(stream[2].move_number, 2);

        assert_ne!(stream[1].fingerprint, stream[2].fingerprint);
    }

    #[test]
    fn empty_game_emits_only_initial_position() {
        let game = record("(;FF[4]GM[1]SZ[19])");

        let stream = position_stream(&game).unwrap();

        assert_eq!(stream.len(), 1);
        assert_eq!(stream[0].move_number, 0);
        assert_eq!(stream[0].side_to_move, Color::Black);
    }
}
