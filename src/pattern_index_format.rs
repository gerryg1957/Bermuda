//! Binary format for the persistent pattern-position index.
//!
//! Version 1 stores one independently decodable block per game.  Board
//! snapshots are kept contiguous so future searches can scan the twelve
//! packed u64 words for each position efficiently.

use crate::{
    Colour, GameRecord, Move,
    pattern_index::{PatternIndexedPosition, pattern_positions_from_record},
};
use anyhow::{Result, anyhow, bail};

pub const PATTERN_INDEX_FORMAT_VERSION: u32 = 1;

const MAGIC: &[u8; 8] = b"MOYOPAT1";
const BOARD_WORDS: usize = 6;
const BOARD_BYTES_PER_POSITION: usize = BOARD_WORDS * 2 * 8;
const METADATA_BYTES_PER_POSITION: usize = 4;
const HEADER_BYTES: usize = 8 + 4 + 8 + 1 + 4;

const NEXT_PASS: u32 = 361;
const NEXT_END: u32 = 362;
const KO_NONE: u32 = 0x1ff;

const NINE_BIT_MASK: u32 = 0x1ff;
const KO_SHIFT: u32 = 9;
const SIDE_SHIFT: u32 = 18;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatternIndexStoredPosition {
    pub move_number: usize,
    pub side_to_move: Colour,
    pub ko_point: Option<u16>,
    pub next_move: Option<Move>,
    pub black: [u64; BOARD_WORDS],
    pub white: [u64; BOARD_WORDS],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatternIndexGameBlock {
    pub game_id: i64,
    pub board_size: u8,
    pub positions: Vec<PatternIndexStoredPosition>,
}

/// Encode one game as an independently decodable pattern-index block.
pub fn encode_game_block(game_id: i64, record: &GameRecord) -> Result<Vec<u8>> {
    let positions = pattern_positions_from_record(game_id, record)?;

    if positions.len() > u32::MAX as usize {
        bail!("too many positions in game {game_id}");
    }

    let position_count = positions.len();
    let capacity = HEADER_BYTES
        .checked_add(
            position_count
                .checked_mul(BOARD_BYTES_PER_POSITION + METADATA_BYTES_PER_POSITION)
                .ok_or_else(|| anyhow!("pattern-index block size overflow"))?,
        )
        .ok_or_else(|| anyhow!("pattern-index block size overflow"))?;

    let mut output = Vec::with_capacity(capacity);

    output.extend_from_slice(MAGIC);
    output.extend_from_slice(&PATTERN_INDEX_FORMAT_VERSION.to_le_bytes());
    output.extend_from_slice(&game_id.to_le_bytes());
    output.push(record.board_size);
    output.extend_from_slice(&(position_count as u32).to_le_bytes());

    /*
     * Keep all board words contiguous.  Searching can therefore stream
     * through board snapshots without interleaved metadata.
     */
    for position in &positions {
        for word in &position.black {
            output.extend_from_slice(&word.to_le_bytes());
        }

        for word in &position.white {
            output.extend_from_slice(&word.to_le_bytes());
        }
    }

    for position in &positions {
        let next_move = record.moves.get(position.move_number).copied();
        let metadata = encode_metadata(position, next_move, record.board_size)?;
        output.extend_from_slice(&metadata.to_le_bytes());
    }

    debug_assert_eq!(output.len(), capacity);

    Ok(output)
}

/// Decode the first game block in `bytes`.
///
/// The returned byte count allows callers to decode concatenated game blocks
/// without requiring a separate directory structure.
pub fn decode_game_block(bytes: &[u8]) -> Result<(PatternIndexGameBlock, usize)> {
    let mut offset = 0_usize;

    let magic = take::<8>(bytes, &mut offset)?;
    if &magic != MAGIC {
        bail!("invalid pattern-index block magic");
    }

    let version = u32::from_le_bytes(take::<4>(bytes, &mut offset)?);
    if version != PATTERN_INDEX_FORMAT_VERSION {
        bail!(
            "unsupported pattern-index format version {version}; expected {}",
            PATTERN_INDEX_FORMAT_VERSION
        );
    }

    let game_id = i64::from_le_bytes(take::<8>(bytes, &mut offset)?);
    let board_size = take::<1>(bytes, &mut offset)?[0];

    if !(1..=crate::board::MAX_BOARD_SIZE).contains(&board_size) {
        bail!("invalid board size {board_size} in pattern-index block");
    }

    let position_count = u32::from_le_bytes(take::<4>(bytes, &mut offset)?) as usize;

    let required = HEADER_BYTES
        .checked_add(
            position_count
                .checked_mul(BOARD_BYTES_PER_POSITION + METADATA_BYTES_PER_POSITION)
                .ok_or_else(|| anyhow!("pattern-index block size overflow"))?,
        )
        .ok_or_else(|| anyhow!("pattern-index block size overflow"))?;

    if bytes.len() < required {
        bail!(
            "truncated pattern-index block: need {required} bytes, have {}",
            bytes.len()
        );
    }

    let mut boards = Vec::with_capacity(position_count);

    for _ in 0..position_count {
        let mut black = [0_u64; BOARD_WORDS];
        let mut white = [0_u64; BOARD_WORDS];

        for word in &mut black {
            *word = u64::from_le_bytes(take::<8>(bytes, &mut offset)?);
        }

        for word in &mut white {
            *word = u64::from_le_bytes(take::<8>(bytes, &mut offset)?);
        }

        boards.push((black, white));
    }

    let mut positions = Vec::with_capacity(position_count);

    for (move_number, (black, white)) in boards.into_iter().enumerate() {
        let metadata = u32::from_le_bytes(take::<4>(bytes, &mut offset)?);

        let (side_to_move, ko_point, next_move) = decode_metadata(metadata, board_size)?;

        positions.push(PatternIndexStoredPosition {
            move_number,
            side_to_move,
            ko_point,
            next_move,
            black,
            white,
        });
    }

    debug_assert_eq!(offset, required);

    Ok((
        PatternIndexGameBlock {
            game_id,
            board_size,
            positions,
        },
        offset,
    ))
}

fn encode_metadata(
    position: &PatternIndexedPosition,
    next_move: Option<Move>,
    board_size: u8,
) -> Result<u32> {
    let maximum_point = u16::from(board_size) * u16::from(board_size);

    let next_code = match next_move {
        Some(mv) => {
            if mv.colour != position.side_to_move {
                bail!(
                    "next-move colour disagrees with side-to-move at game {} position {}",
                    position.game_id,
                    position.move_number
                );
            }

            match mv.point {
                Some(point) if point < maximum_point => u32::from(point),
                Some(point) => {
                    bail!("next-move point {point} lies outside {board_size}x{board_size} board")
                }
                None => NEXT_PASS,
            }
        }
        None => NEXT_END,
    };

    let ko_code = match position.ko_point {
        Some(point) if point < maximum_point => u32::from(point),
        Some(point) => {
            bail!("ko point {point} lies outside {board_size}x{board_size} board")
        }
        None => KO_NONE,
    };

    let side_code = match position.side_to_move {
        Colour::Black => 0_u32,
        Colour::White => 1_u32,
    };

    Ok(next_code | (ko_code << KO_SHIFT) | (side_code << SIDE_SHIFT))
}

fn decode_metadata(metadata: u32, board_size: u8) -> Result<(Colour, Option<u16>, Option<Move>)> {
    let maximum_point = u16::from(board_size) * u16::from(board_size);

    let next_code = metadata & NINE_BIT_MASK;
    let ko_code = (metadata >> KO_SHIFT) & NINE_BIT_MASK;
    let side_code = (metadata >> SIDE_SHIFT) & 1;

    let side_to_move = if side_code == 0 {
        Colour::Black
    } else {
        Colour::White
    };

    let ko_point = if ko_code == KO_NONE {
        None
    } else {
        let point = u16::try_from(ko_code)
            .map_err(|_| anyhow!("invalid ko point in pattern-index metadata"))?;

        if point >= maximum_point {
            bail!("ko point {point} lies outside {board_size}x{board_size} board");
        }

        Some(point)
    };

    let next_move = match next_code {
        0..=360 => {
            let point = u16::try_from(next_code).map_err(|_| anyhow!("invalid next-move point"))?;

            if point >= maximum_point {
                bail!("next-move point {point} lies outside {board_size}x{board_size} board");
            }

            Some(Move {
                colour: side_to_move,
                point: Some(point),
            })
        }

        NEXT_PASS => Some(Move {
            colour: side_to_move,
            point: None,
        }),

        NEXT_END => None,

        _ => bail!("invalid next-move code {next_code}"),
    };

    Ok((side_to_move, ko_point, next_move))
}

fn take<const N: usize>(bytes: &[u8], offset: &mut usize) -> Result<[u8; N]> {
    let end = offset
        .checked_add(N)
        .ok_or_else(|| anyhow!("pattern-index offset overflow"))?;

    let slice = bytes
        .get(*offset..end)
        .ok_or_else(|| anyhow!("truncated pattern-index block"))?;

    let value = slice
        .try_into()
        .map_err(|_| anyhow!("invalid pattern-index field width"))?;

    *offset = end;

    Ok(value)
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
    fn round_trip_preserves_board_and_position_metadata() {
        let game = record(
            "(;FF[4]GM[1]SZ[5]AB[aa]AW[ee]
               ;B[bb]
               ;W[]
               ;B[cc]
               ;W[dd])",
        );

        let expected = pattern_positions_from_record(42, &game).unwrap();
        let encoded = encode_game_block(42, &game).unwrap();
        let (decoded, consumed) = decode_game_block(&encoded).unwrap();

        assert_eq!(consumed, encoded.len());
        assert_eq!(decoded.game_id, 42);
        assert_eq!(decoded.board_size, 5);
        assert_eq!(decoded.positions.len(), expected.len());

        for (stored, expected_position) in decoded.positions.iter().zip(&expected) {
            assert_eq!(stored.move_number, expected_position.move_number);
            assert_eq!(stored.side_to_move, expected_position.side_to_move);
            assert_eq!(stored.ko_point, expected_position.ko_point);
            assert_eq!(stored.black, expected_position.black);
            assert_eq!(stored.white, expected_position.white);

            assert_eq!(
                stored.next_move,
                game.moves.get(stored.move_number).copied()
            );
        }
    }

    #[test]
    fn decoder_reports_bytes_consumed_for_concatenated_blocks() {
        let first = record(
            "(;FF[4]GM[1]SZ[19]
               ;B[pd]
               ;W[dd])",
        );

        let second = record(
            "(;FF[4]GM[1]SZ[19]
               ;B[qp]
               ;W[dp]
               ;B[qq])",
        );

        let first_encoded = encode_game_block(10, &first).unwrap();
        let second_encoded = encode_game_block(20, &second).unwrap();

        let mut combined = first_encoded.clone();
        combined.extend_from_slice(&second_encoded);

        let (first_decoded, first_consumed) = decode_game_block(&combined).unwrap();

        assert_eq!(first_decoded.game_id, 10);
        assert_eq!(first_consumed, first_encoded.len());

        let (second_decoded, second_consumed) =
            decode_game_block(&combined[first_consumed..]).unwrap();

        assert_eq!(second_decoded.game_id, 20);
        assert_eq!(second_consumed, second_encoded.len());
        assert_eq!(first_consumed + second_consumed, combined.len());
    }

    #[test]
    fn encoded_size_is_100_bytes_per_position_plus_header() {
        let game = record(
            "(;FF[4]GM[1]SZ[19]
               ;B[pd]
               ;W[dd]
               ;B[qp])",
        );

        let encoded = encode_game_block(5, &game).unwrap();

        assert_eq!(
            encoded.len(),
            HEADER_BYTES
                + (game.moves.len() + 1) * (BOARD_BYTES_PER_POSITION + METADATA_BYTES_PER_POSITION)
        );
    }
}
