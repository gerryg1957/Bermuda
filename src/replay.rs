use crate::{
    Board, Colour, GameRecord, Move, PositionOccurrence, SetupStone, position_fingerprint,
};
use anyhow::{Context, Result};

#[derive(Debug, Clone)]
pub struct PositionState {
    pub board: Board,
    pub occurrence: PositionOccurrence,

    /// The move that produced this position.
    ///
    /// This is `None` for position zero.
    pub last_move: Option<Move>,
}

pub fn replay_positions(record: &GameRecord) -> Result<Vec<PositionState>> {
    let mut board = Board::new(record.board_size).context("creating replay board")?;

    apply_setup(&mut board, record)?;

    let mut states = Vec::with_capacity(record.moves.len() + 1);

    let initial_side = record
        .moves
        .first()
        .map(|mv| mv.colour)
        .unwrap_or(Colour::Black);

    states.push(make_state(&board, 0, initial_side, None));

    for (index, &mv) in record.moves.iter().enumerate() {
        board
            .play(mv)
            .with_context(|| format!("replaying move {}", index + 1))?;

        let side_to_move = record
            .moves
            .get(index + 1)
            .map(|next| next.colour)
            .unwrap_or_else(|| mv.colour.opponent());

        states.push(make_state(&board, index + 1, side_to_move, Some(mv)));
    }

    Ok(states)
}

fn apply_setup(board: &mut Board, record: &GameRecord) -> Result<()> {
    for setup in &record.setup {
        match *setup {
            SetupStone::Add { colour, point } => {
                board.set_setup(colour, point)?;
            }
            SetupStone::Remove { point } => {
                board.clear_setup(point)?;
            }
        }
    }

    Ok(())
}

fn make_state(
    board: &Board,
    move_number: usize,
    side_to_move: Colour,
    last_move: Option<Move>,
) -> PositionState {
    PositionState {
        board: board.clone(),
        occurrence: PositionOccurrence {
            move_number,
            side_to_move,
            ko_point: board.ko_point(),
            fingerprint: position_fingerprint(board, side_to_move),
        },
        last_move,
    }
}
