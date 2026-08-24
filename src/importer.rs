use anyhow::{Context, Result, bail};

use crate::{
    GameRecord, canonical_hash, canonical_hash_hex, database, extract_main_variation, game,
    game_date::{normalise_played_date, played_date_sort_key},
    parse_collection,
    player_directory::{PlayerAliasResolution, resolve_player_alias_for_source},
    project::Project,
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

    pub fn open_project(project: &Project) -> Result<Self> {
        Self::open(&project.database_root())
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

        game::replay(&record).with_context(|| format!("validating {}", sgf_path.display()))?;

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

        insert_metadata(&transaction, game_source_id, source_id, &record)?;

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
    source_id: i64,
    record: &GameRecord,
) -> Result<()> {
    /*
     * Preserve PB/PW exactly as supplied by the source. Player IDs are
     * Bermuda's separate interpretation, populated only by an unambiguous
     * previously confirmed exact alias.
     */
    let black_player_resolution = record
        .metadata
        .black_player
        .as_deref()
        .map(|name| resolve_player_alias_for_source(transaction, source_id, name))
        .transpose()?;

    let black_player_id = match black_player_resolution {
        Some(PlayerAliasResolution::Unique(player_id)) => Some(player_id),
        Some(PlayerAliasResolution::Unrecognised)
        | Some(PlayerAliasResolution::Ambiguous)
        | None => None,
    };

    let white_player_resolution = record
        .metadata
        .white_player
        .as_deref()
        .map(|name| resolve_player_alias_for_source(transaction, source_id, name))
        .transpose()?;

    let white_player_id = match white_player_resolution {
        Some(PlayerAliasResolution::Unique(player_id)) => Some(player_id),
        Some(PlayerAliasResolution::Unrecognised)
        | Some(PlayerAliasResolution::Ambiguous)
        | None => None,
    };

    let played_date = record.metadata.date.as_deref().map(normalise_played_date);

    let played_date_sort = played_date.as_deref().and_then(played_date_sort_key);

    transaction
        .execute(
            r#"
            INSERT INTO game_metadata(
    game_source_id,
    black_player,
    white_player,
    played_date,
    played_date_sort,
    event,
    result,
    komi,
    handicap,
    black_player_id,
    white_player_id
)
VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
            "#,
            params![
                game_source_id,
                record.metadata.black_player.as_deref(),
                record.metadata.white_player.as_deref(),
                played_date.as_deref(),
                played_date_sort.as_deref(),
                record.metadata.event.as_deref(),
                record.metadata.result.as_deref(),
                record.metadata.komi,
                record.metadata.handicap.map(i64::from),
                black_player_id,
                white_player_id,
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

    #[test]
    fn normalises_approximate_year() {
        assert_eq!(normalise_played_date("c. 1683"), "1683-01-01");
        assert_eq!(normalise_played_date("c.1683"), "1683-01-01");
    }

    #[test]
    fn normalises_year_and_year_month() {
        assert_eq!(normalise_played_date("1683"), "1683-01-01");
        assert_eq!(normalise_played_date("1683-07"), "1683-07-01");
    }

    #[test]
    fn preserves_complete_iso_date() {
        assert_eq!(normalise_played_date("1683-07-12"), "1683-07-12");
    }

    #[test]
    fn preserves_unrecognised_descriptive_dates() {
        assert_eq!(
            normalise_played_date("Published 2012-09"),
            "Published 2012-09"
        );
    }

    #[test]
    fn trims_surrounding_whitespace() {
        assert_eq!(normalise_played_date("  c. 1683  "), "1683-01-01");
        assert_eq!(
            normalise_played_date("  Published 2012-09  "),
            "Published 2012-09"
        );
    }

    #[test]
    fn preserves_invalid_year_month_values() {
        assert_eq!(normalise_played_date("1683-00"), "1683-00");
        assert_eq!(normalise_played_date("1683-13"), "1683-13");
    }

    #[test]
    fn insert_metadata_stores_normalised_played_date() -> Result<()> {
        let mut connection = Connection::open_in_memory()?;

        connection.execute_batch(
            r#"
        CREATE TABLE game_metadata (
            id              INTEGER PRIMARY KEY,
            game_source_id  INTEGER NOT NULL,
            black_player    TEXT,
            white_player    TEXT,
            played_date     TEXT,
            played_date_sort  TEXT,
            event           TEXT,
            result          TEXT,
            komi            REAL,
            handicap        INTEGER,
            black_player_id INTEGER,
            white_player_id INTEGER
        );
        "#,
        )?;

        let collection = parse_collection(b"(;FF[4]GM[1]SZ[19]DT[c. 1683])")?;
        let record = extract_main_variation(&collection)?;

        let transaction = connection.transaction()?;
        insert_metadata(&transaction, 1, 1, &record)?;
        transaction.commit()?;

        let (stored_date, stored_sort_date): (String, String) = connection.query_row(
            r#"
        SELECT played_date, played_date_sort
        FROM game_metadata
        "#,
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;

        assert_eq!(stored_date, "1683-01-01");
        assert_eq!(stored_sort_date, "1683-01-01");
        Ok(())
    }

    #[test]
    fn exact_alias_resolution_prefers_source_specific_and_refuses_ambiguity() -> Result<()> {
        let connection = Connection::open_in_memory()?;

        connection.execute_batch(
            r#"
            CREATE TABLE player_aliases (
                id          INTEGER PRIMARY KEY,
                player_id   INTEGER NOT NULL,
                name        TEXT NOT NULL,
                source_id   INTEGER,
                notes       TEXT
            );

            /*
             * Cho has both a source-specific and a global identity. The
             * source-specific assertion must win.
             */
            INSERT INTO player_aliases(
                id,
                player_id,
                name,
                source_id
            )
            VALUES
                (1, 101, 'Cho Chikun', 7),
                (2, 999, 'Cho Chikun', NULL);

            /*
             * Kobayashi has only a global assertion, so it is a valid
             * fallback for source 7.
             */
            INSERT INTO player_aliases(
                id,
                player_id,
                name,
                source_id
            )
            VALUES
                (3, 201, 'Kobayashi Satoru', NULL);
            "#,
        )?;

        assert_eq!(
            resolve_player_alias_for_source(&connection, 7, "Cho Chikun")?,
            PlayerAliasResolution::Unique(101)
        );

        assert_eq!(
            resolve_player_alias_for_source(&connection, 7, "Kobayashi Satoru")?,
            PlayerAliasResolution::Unique(201)
        );

        assert_eq!(
            resolve_player_alias_for_source(&connection, 7, "Unknown Player")?,
            PlayerAliasResolution::Unrecognised
        );

        /*
         * A second source-specific identity makes Cho ambiguous. Bermuda
         * must leave the name unresolved rather than fall back to the global
         * alias.
         */
        connection.execute(
            r#"
            INSERT INTO player_aliases(
                player_id,
                name,
                source_id
            )
            VALUES (?1, ?2, ?3)
            "#,
            params![102, "Cho Chikun", 7],
        )?;

        assert_eq!(
            resolve_player_alias_for_source(&connection, 7, "Cho Chikun")?,
            PlayerAliasResolution::Ambiguous
        );

        /*
         * Global ambiguity is likewise unresolved when there is no
         * source-specific assertion.
         */
        connection.execute(
            r#"
            INSERT INTO player_aliases(
                player_id,
                name,
                source_id
            )
            VALUES (?1, ?2, NULL)
            "#,
            params![202, "Kobayashi Satoru"],
        )?;

        assert_eq!(
            resolve_player_alias_for_source(&connection, 7, "Kobayashi Satoru")?,
            PlayerAliasResolution::Ambiguous
        );

        Ok(())
    }

    #[test]
    fn insert_metadata_uses_known_aliases_without_rewriting_source_names() -> Result<()> {
        let mut connection = Connection::open_in_memory()?;

        connection.execute_batch(
            r#"
            CREATE TABLE player_aliases (
                id          INTEGER PRIMARY KEY,
                player_id   INTEGER NOT NULL,
                name        TEXT NOT NULL,
                source_id   INTEGER,
                notes       TEXT
            );

            CREATE TABLE game_metadata (
                id                INTEGER PRIMARY KEY,
                game_source_id    INTEGER NOT NULL,
                black_player      TEXT,
                white_player      TEXT,
                played_date       TEXT,
                played_date_sort  TEXT,
                event             TEXT,
                result            TEXT,
                komi              REAL,
                handicap          INTEGER,
                black_player_id   INTEGER,
                white_player_id   INTEGER
            );

            INSERT INTO player_aliases(
                player_id,
                name,
                source_id
            )
            VALUES
                (101, 'Cho Chikun', 7),
                (201, 'Kobayashi Satoru', NULL);
            "#,
        )?;

        let collection =
            parse_collection(b"(;FF[4]GM[1]SZ[19]PB[Cho Chikun]PW[Kobayashi Satoru])")?;

        let record = extract_main_variation(&collection)?;

        let transaction = connection.transaction()?;

        insert_metadata(&transaction, 10, 7, &record)?;

        transaction.commit()?;

        let (black_player, white_player, black_player_id, white_player_id): (
            Option<String>,
            Option<String>,
            Option<i64>,
            Option<i64>,
        ) = connection.query_row(
            r#"
            SELECT
                black_player,
                white_player,
                black_player_id,
                white_player_id
            FROM game_metadata
            WHERE game_source_id = 10
            "#,
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )?;

        /*
         * Imported source strings remain the authoritative source record.
         * The IDs are merely Bermuda's confirmed interpretation alongside.
         */
        assert_eq!(black_player.as_deref(), Some("Cho Chikun"));
        assert_eq!(white_player.as_deref(), Some("Kobayashi Satoru"));
        assert_eq!(black_player_id, Some(101));
        assert_eq!(white_player_id, Some(201));

        Ok(())
    }
}
