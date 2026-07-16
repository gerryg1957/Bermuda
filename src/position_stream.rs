use crate::{Board, Color, GameRecord, SetupStone, position_fingerprint};
use anyhow::{Context, Result};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PositionOccurrence {
    /// Zero means the initial position after setup stones.
    pub move_number: usize,

    /// The player expected to move from this position.
    pub side_to_move: Color,

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
    let mut board = Board::new(record.board_size).context("creating position-stream board")?;

    apply_setup(&mut board, record)?;

    let mut occurrences = Vec::with_capacity(record.moves.len() + 1);

    let initial_side = record
        .moves
        .first()
        .map(|mv| mv.color)
        .unwrap_or(Color::Black);

    occurrences.push(make_occurrence(&board, 0, initial_side));

    for (index, &mv) in record.moves.iter().enumerate() {
        board
            .play(mv)
            .with_context(|| format!("replaying move {}", index + 1))?;

        let side_to_move = record
            .moves
            .get(index + 1)
            .map(|next| next.color)
            .unwrap_or_else(|| mv.color.opponent());

        occurrences.push(make_occurrence(&board, index + 1, side_to_move));
    }

    Ok(occurrences)
}

fn apply_setup(board: &mut Board, record: &GameRecord) -> Result<()> {
    for setup in &record.setup {
        match *setup {
            SetupStone::Add { color, point } => board
                .set_setup(color, point)
                .with_context(|| format!("applying setup stone at point {point}"))?,

            SetupStone::Remove { point } => board
                .clear_setup(point)
                .with_context(|| format!("removing setup stone at point {point}"))?,
        }
    }

    Ok(())
}

fn make_occurrence(board: &Board, move_number: usize, side_to_move: Color) -> PositionOccurrence {
    PositionOccurrence {
        move_number,
        side_to_move,
        ko_point: board.ko_point(),
        fingerprint: position_fingerprint(board, side_to_move),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{extract_main_variation, parse_collection};

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

        let empty = Board::new(19).unwrap();

        assert_ne!(
            stream[0].fingerprint,
            position_fingerprint(&empty, Color::White)
        );
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

        // The stones are unchanged by the pass, but side to move differs
        // from the position immediately before it.
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
