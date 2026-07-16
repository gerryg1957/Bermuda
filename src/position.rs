use crate::{Board, Color};
use sha2::{Digest, Sha256};

const POSITION_FORMAT_VERSION: u8 = 1;
const POSITION_MAGIC: &[u8] = b"MOYODB-EXACT-POSITION";

/// Computes a deterministic SHA-256 fingerprint for one exact board position.
///
/// The fingerprint includes:
///
/// - format version;
/// - board size;
/// - all black stones;
/// - all white stones;
/// - side to move;
/// - simple-ko point, if present.
///
/// Game metadata and move number are deliberately excluded.
pub fn position_fingerprint(board: &Board, side_to_move: Color) -> [u8; 32] {
    let mut hasher = Sha256::new();

    hasher.update(POSITION_MAGIC);
    hasher.update([POSITION_FORMAT_VERSION]);
    hasher.update([board.size()]);
    hasher.update([color_byte(side_to_move)]);

    for word in board.black_words() {
        hasher.update(word.to_be_bytes());
    }

    for word in board.white_words() {
        hasher.update(word.to_be_bytes());
    }

    match board.ko_point() {
        Some(point) => {
            hasher.update([1]);
            hasher.update(point.to_be_bytes());
        }
        None => {
            hasher.update([0]);
        }
    }

    hasher.finalize().into()
}

pub fn position_fingerprint_hex(board: &Board, side_to_move: Color) -> String {
    let fingerprint = position_fingerprint(board, side_to_move);
    let mut text = String::with_capacity(64);

    for byte in fingerprint {
        use std::fmt::Write;
        write!(&mut text, "{byte:02x}").expect("writing to String cannot fail");
    }

    text
}

fn color_byte(color: Color) -> u8 {
    match color {
        Color::Black => 1,
        Color::White => 2,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Move;

    #[test]
    fn identical_positions_have_identical_fingerprints() {
        let mut first = Board::new(19).unwrap();
        let mut second = Board::new(19).unwrap();

        let point = first.point(3, 3).unwrap();

        first
            .play(Move {
                color: Color::Black,
                point: Some(point),
            })
            .unwrap();

        second
            .play(Move {
                color: Color::Black,
                point: Some(point),
            })
            .unwrap();

        assert_eq!(
            position_fingerprint(&first, Color::White),
            position_fingerprint(&second, Color::White)
        );
    }

    #[test]
    fn side_to_move_changes_fingerprint() {
        let board = Board::new(19).unwrap();

        assert_ne!(
            position_fingerprint(&board, Color::Black),
            position_fingerprint(&board, Color::White)
        );
    }

    #[test]
    fn stone_placement_changes_fingerprint() {
        let empty = Board::new(19).unwrap();
        let mut occupied = Board::new(19).unwrap();

        occupied
            .play(Move {
                color: Color::Black,
                point: Some(occupied.point(3, 3).unwrap()),
            })
            .unwrap();

        assert_ne!(
            position_fingerprint(&empty, Color::White),
            position_fingerprint(&occupied, Color::White)
        );
    }

    #[test]
    fn board_size_changes_fingerprint() {
        let board_9 = Board::new(9).unwrap();
        let board_19 = Board::new(19).unwrap();

        assert_ne!(
            position_fingerprint(&board_9, Color::Black),
            position_fingerprint(&board_19, Color::Black)
        );
    }
}
