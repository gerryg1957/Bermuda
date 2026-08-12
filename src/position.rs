use crate::{Board, Colour, PatternTransformation};
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
pub fn position_fingerprint(board: &Board, side_to_move: Colour) -> [u8; 32] {
    fingerprint_from_parts(
        board.size(),
        board.black_words(),
        board.white_words(),
        side_to_move,
        board.ko_point(),
    )
}

/// Computes the exact-position fingerprint after applying a board symmetry.
///
/// The transformation is applied to every stone and to the simple-ko point.
/// Colour and side to move are deliberately unchanged.
pub fn transformed_position_fingerprint(
    board: &Board,
    side_to_move: Colour,
    transformation: PatternTransformation,
) -> [u8; 32] {
    if transformation == PatternTransformation::Identity {
        return position_fingerprint(board, side_to_move);
    }

    let size = board.size();
    let point_count = u16::from(size) * u16::from(size);
    let mut black = [0u64; 6];
    let mut white = [0u64; 6];

    for point in 0..point_count {
        let Some(colour) = board.colour_at(point) else {
            continue;
        };

        let transformed = transform_board_point(point, size, transformation);
        let word = usize::from(transformed / 64);
        let bit = u32::from(transformed % 64);

        match colour {
            Colour::Black => black[word] |= 1u64 << bit,
            Colour::White => white[word] |= 1u64 << bit,
        }
    }

    let ko_point = board
        .ko_point()
        .map(|point| transform_board_point(point, size, transformation));

    fingerprint_from_parts(size, &black, &white, side_to_move, ko_point)
}

fn transform_board_point(point: u16, size: u8, transformation: PatternTransformation) -> u16 {
    let size_u16 = u16::from(size);
    let x = i16::try_from(point % size_u16).expect("board x coordinate fits in i16");
    let y = i16::try_from(point / size_u16).expect("board y coordinate fits in i16");

    let (x, y) = transformation.transform_relative_point(x, y, size, size);

    let x = u16::try_from(x).expect("transformed board x coordinate is non-negative");
    let y = u16::try_from(y).expect("transformed board y coordinate is non-negative");

    y * size_u16 + x
}

fn fingerprint_from_parts(
    size: u8,
    black_words: &[u64],
    white_words: &[u64],
    side_to_move: Colour,
    ko_point: Option<u16>,
) -> [u8; 32] {
    let mut hasher = Sha256::new();

    hasher.update(POSITION_MAGIC);
    hasher.update([POSITION_FORMAT_VERSION]);
    hasher.update([size]);
    hasher.update([colour_byte(side_to_move)]);

    for word in black_words {
        hasher.update(word.to_be_bytes());
    }

    for word in white_words {
        hasher.update(word.to_be_bytes());
    }

    match ko_point {
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

pub fn position_fingerprint_hex(board: &Board, side_to_move: Colour) -> String {
    let fingerprint = position_fingerprint(board, side_to_move);
    let mut text = String::with_capacity(64);

    for byte in fingerprint {
        use std::fmt::Write;
        write!(&mut text, "{byte:02x}").expect("writing to String cannot fail");
    }

    text
}

fn colour_byte(colour: Colour) -> u8 {
    match colour {
        Colour::Black => 1,
        Colour::White => 2,
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
                colour: Colour::Black,
                point: Some(point),
            })
            .unwrap();

        second
            .play(Move {
                colour: Colour::Black,
                point: Some(point),
            })
            .unwrap();

        assert_eq!(
            position_fingerprint(&first, Colour::White),
            position_fingerprint(&second, Colour::White)
        );
    }

    #[test]
    fn side_to_move_changes_fingerprint() {
        let board = Board::new(19).unwrap();

        assert_ne!(
            position_fingerprint(&board, Colour::Black),
            position_fingerprint(&board, Colour::White)
        );
    }

    #[test]
    fn stone_placement_changes_fingerprint() {
        let empty = Board::new(19).unwrap();
        let mut occupied = Board::new(19).unwrap();

        occupied
            .play(Move {
                colour: Colour::Black,
                point: Some(occupied.point(3, 3).unwrap()),
            })
            .unwrap();

        assert_ne!(
            position_fingerprint(&empty, Colour::White),
            position_fingerprint(&occupied, Colour::White)
        );
    }

    #[test]
    fn board_size_changes_fingerprint() {
        let board_9 = Board::new(9).unwrap();
        let board_19 = Board::new(19).unwrap();

        assert_ne!(
            position_fingerprint(&board_9, Colour::Black),
            position_fingerprint(&board_19, Colour::Black)
        );
    }
}
