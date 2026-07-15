use crate::database;
use anyhow::{Context, Result, bail};
use moyodb_core::{
    GameRecord, canonical_hash, canonical_hash_hex, extract_main_variation, parse_collection,
    write_move_file,
};
use rusqlite::{Connection, OptionalExtension, Transaction, params};
use std::{
    fs,
    path::{Path, PathBuf},
};

pub const PROFESSIONAL_BOARD_SIZE: u8 = 19;

#[derive(Debug)]
pub enum ImportOutcome {
    Imported { game_id: i64, move_file: PathBuf },
    AddedSource { game_id: i64 },
    AlreadyImported { game_id: i64 },
    SkippedBoardSize { board_size: u8 },
}

pub struct Importer {
    database_root: PathBuf,
    connection: Connection,
}

impl Importer {
    pub fn open(database_root: &Path) -> Result<Self> {
        let connection = database::open(database_root)?;

        Ok(Self {
            database_root: database_root.to_path_buf(),
            connection,
        })
    }

    pub fn import_file(
        &mut self,
        source_name: &str,
        source_version: &str,
        sgf_path: &Path,
    ) -> Result<ImportOutcome> {
        let bytes =
            fs::read(sgf_path).with_context(|| format!("reading {}", sgf_path.display()))?;

        let collection =
            parse_collection(&bytes).with_context(|| format!("parsing {}", sgf_path.display()))?;

        let record = extract_main_variation(&collection)
            .with_context(|| format!("extracting main variation from {}", sgf_path.display()))?;

        moyodb_core::game::replay(&record)
            .with_context(|| format!("validating {}", sgf_path.display()))?;

        if record.board_size != PROFESSIONAL_BOARD_SIZE {
            return Ok(ImportOutcome::SkippedBoardSize {
                board_size: record.board_size,
            });
        }

        let canonical_hash = canonical_hash(&record).context("computing canonical game hash")?;
        let canonical_hex =
            canonical_hash_hex(&record).context("formatting canonical game hash")?;

        let original_path = sgf_path
            .canonicalize()
            .unwrap_or_else(|_| sgf_path.to_path_buf())
            .to_string_lossy()
            .into_owned();

        let relative_move_file = move_file_path(&canonical_hex);
        let absolute_move_file = self.database_root.join(&relative_move_file);

        let transaction = self
            .connection
            .transaction()
            .context("starting database import transaction")?;

        let source_id = find_or_create_source(&transaction, source_name, source_version)?;

        if let Some(game_id) = find_existing_source_path(&transaction, source_id, &original_path)? {
            transaction
                .commit()
                .context("committing duplicate-source transaction")?;

            return Ok(ImportOutcome::AlreadyImported { game_id });
        }

        let existing_game_id = find_game_by_hash(&transaction, &canonical_hash)?;

        let (game_id, is_new_game) = match existing_game_id {
            Some(game_id) => (game_id, false),
            None => {
                if let Some(parent) = absolute_move_file.parent() {
                    fs::create_dir_all(parent)
                        .with_context(|| format!("creating game directory {}", parent.display()))?;
                }
                write_move_file(&absolute_move_file, &record)
                    .with_context(|| format!("writing {}", absolute_move_file.display()))?;

                let game_id =
                    insert_game(&transaction, &canonical_hash, &record, &relative_move_file)?;

                (game_id, true)
            }
        };

        let game_source_id = insert_game_source(&transaction, game_id, source_id, &original_path)?;

        insert_metadata(&transaction, game_source_id, &record)?;

        transaction
            .commit()
            .context("committing game import transaction")?;

        if is_new_game {
            Ok(ImportOutcome::Imported {
                game_id,
                move_file: absolute_move_file,
            })
        } else {
            Ok(ImportOutcome::AddedSource { game_id })
        }
    }
}

fn move_file_path(canonical_hex: &str) -> PathBuf {
    debug_assert!(canonical_hex.len() >= 4);

    PathBuf::from("games")
        .join(&canonical_hex[0..2])
        .join(&canonical_hex[2..4])
        .join(format!("{canonical_hex}.moves"))
}

fn find_or_create_source(transaction: &Transaction<'_>, name: &str, version: &str) -> Result<i64> {
    if name.trim().is_empty() {
        bail!("source name must not be empty");
    }

    if version.trim().is_empty() {
        bail!("source version must not be empty");
    }

    transaction
        .execute(
            r#"
            INSERT INTO sources(name, version)
            VALUES (?1, ?2)
            ON CONFLICT(name, version) DO NOTHING
            "#,
            params![name, version],
        )
        .context("creating source record")?;

    transaction
        .query_row(
            "SELECT id FROM sources WHERE name = ?1 AND version = ?2",
            params![name, version],
            |row| row.get(0),
        )
        .context("reading source record")
}

fn find_existing_source_path(
    transaction: &Transaction<'_>,
    source_id: i64,
    original_path: &str,
) -> Result<Option<i64>> {
    transaction
        .query_row(
            r#"
            SELECT game_id
            FROM game_sources
            WHERE source_id = ?1 AND original_path = ?2
            "#,
            params![source_id, original_path],
            |row| row.get(0),
        )
        .optional()
        .context("checking whether source path was already imported")
}

fn find_game_by_hash(
    transaction: &Transaction<'_>,
    canonical_hash: &[u8; 32],
) -> Result<Option<i64>> {
    transaction
        .query_row(
            "SELECT id FROM games WHERE canonical_hash = ?1",
            params![canonical_hash.as_slice()],
            |row| row.get(0),
        )
        .optional()
        .context("checking for canonical duplicate")
}

fn insert_game(
    transaction: &Transaction<'_>,
    canonical_hash: &[u8; 32],
    record: &GameRecord,
    relative_move_file: &Path,
) -> Result<i64> {
    let move_count =
        i64::try_from(record.moves.len()).context("move count does not fit in SQLite integer")?;

    transaction
        .execute(
            r#"
            INSERT INTO games(
                canonical_hash,
                board_size,
                move_count,
                move_file
            )
            VALUES (?1, ?2, ?3, ?4)
            "#,
            params![
                canonical_hash.as_slice(),
                i64::from(record.board_size),
                move_count,
                relative_move_file.to_string_lossy().as_ref(),
            ],
        )
        .context("inserting game record")?;

    Ok(transaction.last_insert_rowid())
}

fn insert_game_source(
    transaction: &Transaction<'_>,
    game_id: i64,
    source_id: i64,
    original_path: &str,
) -> Result<i64> {
    transaction
        .execute(
            r#"
            INSERT INTO game_sources(
                game_id,
                source_id,
                original_path
            )
            VALUES (?1, ?2, ?3)
            "#,
            params![game_id, source_id, original_path],
        )
        .context("inserting game source record")?;

    Ok(transaction.last_insert_rowid())
}

fn insert_metadata(
    transaction: &Transaction<'_>,
    game_source_id: i64,
    record: &GameRecord,
) -> Result<()> {
    transaction
        .execute(
            r#"
            INSERT INTO game_metadata(
                game_source_id,
                black_player,
                white_player,
                played_date,
                event,
                result,
                komi,
                handicap
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
            "#,
            params![
                game_source_id,
                record.metadata.black_player.as_deref(),
                record.metadata.white_player.as_deref(),
                record.metadata.date.as_deref(),
                record.metadata.event.as_deref(),
                record.metadata.result.as_deref(),
                record.metadata.komi,
                record.metadata.handicap.map(i64::from),
            ],
        )
        .context("inserting source-specific game metadata")?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn move_files_use_hash_subdirectories() {
        let hash = "bf2f065bd025ad7d31e76249b4cb0d63ec92dc6bd76709661a8454fef743fa80";

        assert_eq!(
            move_file_path(hash),
            PathBuf::from("games")
                .join("bf")
                .join("2f")
                .join(format!("{hash}.moves"))
        );
    }
}
