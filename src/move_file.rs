use crate::{
    board::{Color, Move},
    game::{GameRecord, Metadata, SetupStone},
};
use std::{
    fs::File,
    io::{self, BufReader, BufWriter, Read, Write},
    path::Path,
};
use thiserror::Error;

const MAGIC: &[u8; 8] = b"MOYOGAME";
const VERSION: u16 = 1;
const PASS: u16 = u16::MAX;

#[derive(Debug, Error)]
pub enum MoveFileError {
    #[error(transparent)]
    Io(#[from] io::Error),
    #[error("not a MoyoDB move file")]
    BadMagic,
    #[error("unsupported move-file version {0}")]
    UnsupportedVersion(u16),
    #[error("invalid color byte {0}")]
    InvalidColor(u8),
    #[error("invalid UTF-8 metadata")]
    InvalidUtf8,
    #[error("metadata field is too large")]
    MetadataTooLarge,
    #[error("record count is too large")]
    CountTooLarge,
}

pub fn write_move_file(path: impl AsRef<Path>, record: &GameRecord) -> Result<(), MoveFileError> {
    let file = File::create(path)?;
    let mut writer = BufWriter::new(file);
    writer.write_all(MAGIC)?;
    write_u16(&mut writer, VERSION)?;
    writer.write_all(&[record.board_size])?;
    write_u32(
        &mut writer,
        u32::try_from(record.setup.len()).map_err(|_| MoveFileError::CountTooLarge)?,
    )?;
    write_u32(
        &mut writer,
        u32::try_from(record.moves.len()).map_err(|_| MoveFileError::CountTooLarge)?,
    )?;

    write_string(&mut writer, record.metadata.black_player.as_deref())?;
    write_string(&mut writer, record.metadata.white_player.as_deref())?;
    write_string(&mut writer, record.metadata.date.as_deref())?;
    write_string(&mut writer, record.metadata.event.as_deref())?;
    write_string(&mut writer, record.metadata.result.as_deref())?;
    writer.write_all(&record.metadata.komi.unwrap_or(f32::NAN).to_le_bytes())?;
    writer.write_all(&[record.metadata.handicap.unwrap_or(u8::MAX)])?;

    for setup in &record.setup {
        match *setup {
            SetupStone::Add { color, point } => {
                writer.write_all(&[0, color_byte(color)])?;
                write_u16(&mut writer, point)?;
            }
            SetupStone::Remove { point } => {
                writer.write_all(&[1, 0])?;
                write_u16(&mut writer, point)?;
            }
        }
    }
    for mv in &record.moves {
        writer.write_all(&[color_byte(mv.color)])?;
        write_u16(&mut writer, mv.point.unwrap_or(PASS))?;
    }
    writer.flush()?;
    Ok(())
}

pub fn read_move_file(path: impl AsRef<Path>) -> Result<GameRecord, MoveFileError> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);
    let mut magic = [0u8; 8];
    reader.read_exact(&mut magic)?;
    if &magic != MAGIC {
        return Err(MoveFileError::BadMagic);
    }
    let version = read_u16(&mut reader)?;
    if version != VERSION {
        return Err(MoveFileError::UnsupportedVersion(version));
    }
    let board_size = read_byte(&mut reader)?;
    let setup_count = read_u32(&mut reader)?;
    let move_count = read_u32(&mut reader)?;

    let black_player = read_string(&mut reader)?;
    let white_player = read_string(&mut reader)?;
    let date = read_string(&mut reader)?;
    let event = read_string(&mut reader)?;
    let result = read_string(&mut reader)?;
    let mut float_bytes = [0u8; 4];
    reader.read_exact(&mut float_bytes)?;
    let komi_value = f32::from_le_bytes(float_bytes);
    let handicap_value = read_byte(&mut reader)?;

    let mut setup = Vec::with_capacity(setup_count as usize);
    for _ in 0..setup_count {
        let kind = read_byte(&mut reader)?;
        let color = read_byte(&mut reader)?;
        let point = read_u16(&mut reader)?;
        setup.push(match kind {
            0 => SetupStone::Add {
                color: parse_color(color)?,
                point,
            },
            1 => SetupStone::Remove { point },
            _ => return Err(MoveFileError::InvalidColor(kind)),
        });
    }
    let mut moves = Vec::with_capacity(move_count as usize);
    for _ in 0..move_count {
        let color = parse_color(read_byte(&mut reader)?)?;
        let point = read_u16(&mut reader)?;
        moves.push(Move {
            color,
            point: (point != PASS).then_some(point),
        });
    }

    Ok(GameRecord {
        board_size,
        metadata: Metadata {
            black_player,
            white_player,
            date,
            event,
            result,
            komi: (!komi_value.is_nan()).then_some(komi_value),
            handicap: (handicap_value != u8::MAX).then_some(handicap_value),
        },
        setup,
        moves,
    })
}

fn color_byte(color: Color) -> u8 {
    match color {
        Color::Black => 0,
        Color::White => 1,
    }
}
fn parse_color(byte: u8) -> Result<Color, MoveFileError> {
    match byte {
        0 => Ok(Color::Black),
        1 => Ok(Color::White),
        other => Err(MoveFileError::InvalidColor(other)),
    }
}
fn write_u16(writer: &mut impl Write, value: u16) -> io::Result<()> {
    writer.write_all(&value.to_le_bytes())
}
fn write_u32(writer: &mut impl Write, value: u32) -> io::Result<()> {
    writer.write_all(&value.to_le_bytes())
}
fn read_u16(reader: &mut impl Read) -> io::Result<u16> {
    let mut b = [0; 2];
    reader.read_exact(&mut b)?;
    Ok(u16::from_le_bytes(b))
}
fn read_u32(reader: &mut impl Read) -> io::Result<u32> {
    let mut b = [0; 4];
    reader.read_exact(&mut b)?;
    Ok(u32::from_le_bytes(b))
}
fn read_byte(reader: &mut impl Read) -> io::Result<u8> {
    let mut b = [0; 1];
    reader.read_exact(&mut b)?;
    Ok(b[0])
}
fn write_string(writer: &mut impl Write, value: Option<&str>) -> Result<(), MoveFileError> {
    match value {
        None => write_u32(writer, u32::MAX)?,
        Some(value) => {
            let bytes = value.as_bytes();
            write_u32(
                writer,
                u32::try_from(bytes.len()).map_err(|_| MoveFileError::MetadataTooLarge)?,
            )?;
            writer.write_all(bytes)?;
        }
    }
    Ok(())
}
fn read_string(reader: &mut impl Read) -> Result<Option<String>, MoveFileError> {
    let len = read_u32(reader)?;
    if len == u32::MAX {
        return Ok(None);
    }
    let mut bytes = vec![0u8; len as usize];
    reader.read_exact(&mut bytes)?;
    String::from_utf8(bytes)
        .map(Some)
        .map_err(|_| MoveFileError::InvalidUtf8)
}
