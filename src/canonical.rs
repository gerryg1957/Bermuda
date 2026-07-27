use crate::{Board, Colour, GameRecord, SetupStone};
use anyhow::{Context, Result};
use sha2::{Digest, Sha256};

/// Version of the canonical game representation.
///
/// Increment this only if the canonical byte format changes.
const CANONICAL_FORMAT_VERSION: u8 = 1;

/// Domain separator to ensure these hashes cannot be confused with hashes
/// calculated for some other MoyoDB data format.
const CANONICAL_MAGIC: &[u8] = b"MOYODB-CANONICAL-GAME";

/// Computes the canonical SHA-256 identity of a game.
///
/// The hash includes:
///
/// - canonical format version;
/// - board size;
/// - final initial board position after all setup edits;
/// - ordered move sequence.
///
/// It deliberately excludes player names, dates, events, results, comments,
/// filenames, SGF formatting, and all other metadata.
pub fn canonical_hash(record: &GameRecord) -> Result<[u8; 32]> {
    let mut hasher = Sha256::new();

    hasher.update(CANONICAL_MAGIC);
    hasher.update([CANONICAL_FORMAT_VERSION]);
    hasher.update([record.board_size]);

    hash_initial_position(&mut hasher, record)?;
    hash_moves(&mut hasher, record);

    Ok(hasher.finalize().into())
}

/// Returns the canonical game hash as lowercase hexadecimal text.
pub fn canonical_hash_hex(record: &GameRecord) -> Result<String> {
    let hash = canonical_hash(record)?;

    let mut text = String::with_capacity(hash.len() * 2);

    for byte in hash {
        use std::fmt::Write;
        write!(&mut text, "{byte:02x}").expect("writing to String cannot fail");
    }

    Ok(text)
}

fn hash_initial_position(hasher: &mut Sha256, record: &GameRecord) -> Result<()> {
    let mut board = Board::new(record.board_size).context("creating canonical setup board")?;

    for setup in &record.setup {
        match *setup {
            SetupStone::Add { colour, point } => board
                .set_setup(colour, point)
                .with_context(|| format!("applying canonical setup stone at point {point}"))?,

            SetupStone::Remove { point } => board
                .clear_setup(point)
                .with_context(|| format!("removing canonical setup stone at point {point}"))?,
        }
    }

    let point_count = u16::from(record.board_size) * u16::from(record.board_size);

    let mut black_points = Vec::new();
    let mut white_points = Vec::new();

    for point in 0..point_count {
        match board.colour_at(point) {
            Some(Colour::Black) => black_points.push(point),
            Some(Colour::White) => white_points.push(point),
            None => {}
        }
    }

    hash_points(hasher, Colour::Black, &black_points);
    hash_points(hasher, Colour::White, &white_points);

    Ok(())
}

fn hash_points(hasher: &mut Sha256, colour: Colour, points: &[u16]) {
    hasher.update([colour_byte(colour)]);

    let count = u16::try_from(points.len())
        .expect("a supported Go board cannot contain more than u16::MAX points");

    hasher.update(count.to_be_bytes());

    for point in points {
        hasher.update(point.to_be_bytes());
    }
}

fn hash_moves(hasher: &mut Sha256, record: &GameRecord) {
    let move_count =
        u32::try_from(record.moves.len()).expect("game contains more than u32::MAX moves");

    hasher.update(move_count.to_be_bytes());

    for mv in &record.moves {
        hasher.update([colour_byte(mv.colour)]);

        match mv.point {
            Some(point) => {
                hasher.update([1]);
                hasher.update(point.to_be_bytes());
            }
            None => {
                hasher.update([0]);
            }
        }
    }
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
    use crate::{extract_main_variation, parse_collection};

    fn record(sgf: &str) -> GameRecord {
        let collection = parse_collection(sgf.as_bytes()).expect("parse SGF");
        extract_main_variation(&collection).expect("extract main variation")
    }

    #[test]
    fn metadata_does_not_change_hash() {
        let first = record(
            "(;FF[4]GM[1]SZ[19]PB[Alpha]PW[Beta]DT[1923-01-01]EV[Event One]
               ;B[pd];W[dd];B[qp];W[dp])",
        );

        let second = record(
            "(;FF[4]GM[1]SZ[19]PB[Different Black]PW[Different White]
               DT[2026-07-15]EV[Completely Different Event]RE[W+R]
               ;B[pd];W[dd];B[qp];W[dp])",
        );

        assert_eq!(
            canonical_hash(&first).unwrap(),
            canonical_hash(&second).unwrap()
        );
    }

    #[test]
    fn equivalent_setup_positions_have_same_hash() {
        let first = record(
            "(;FF[4]GM[1]SZ[19]
               AB[aa][bb]
               AW[cc]
               ;B[dd];W[ee])",
        );

        let second = record(
            "(;FF[4]GM[1]SZ[19]
               AB[bb][ff][aa]
               AE[ff]
               AW[cc]
               ;B[dd];W[ee])",
        );

        assert_eq!(
            canonical_hash(&first).unwrap(),
            canonical_hash(&second).unwrap()
        );
    }

    #[test]
    fn different_move_sequences_have_different_hashes() {
        let first = record(
            "(;FF[4]GM[1]SZ[19]
               ;B[pd];W[dd];B[qp])",
        );

        let second = record(
            "(;FF[4]GM[1]SZ[19]
               ;B[pd];W[dd];B[pp])",
        );

        assert_ne!(
            canonical_hash(&first).unwrap(),
            canonical_hash(&second).unwrap()
        );
    }

    #[test]
    fn pass_is_part_of_game_identity() {
        let with_pass = record(
            "(;FF[4]GM[1]SZ[19]
               ;B[pd];W[];B[qp])",
        );

        let without_pass = record(
            "(;FF[4]GM[1]SZ[19]
               ;B[pd];B[qp])",
        );

        assert_ne!(
            canonical_hash(&with_pass).unwrap(),
            canonical_hash(&without_pass).unwrap()
        );
    }
}
