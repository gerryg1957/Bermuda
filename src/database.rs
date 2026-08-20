use anyhow::{Context, Result, bail};
use rusqlite::{Connection, OptionalExtension, params};

use crate::game_date::{normalise_played_date, played_date_sort_key};
use std::{
    fs,
    path::{Path, PathBuf},
};

const SCHEMA_VERSION: i64 = 5;

pub fn initialise(root: &Path) -> Result<()> {
    if root.exists() && !root.is_dir() {
        bail!(
            "database path exists but is not a directory: {}",
            root.display()
        );
    }

    fs::create_dir_all(root)
        .with_context(|| format!("creating database directory {}", root.display()))?;

    fs::create_dir_all(root.join("games"))
        .with_context(|| format!("creating {}", root.join("games").display()))?;

    fs::create_dir_all(root.join("tmp"))
        .with_context(|| format!("creating {}", root.join("tmp").display()))?;

    let sqlite_path = metadata_path(root);
    let connection = Connection::open(&sqlite_path)
        .with_context(|| format!("opening {}", sqlite_path.display()))?;

    connection
        .execute_batch(
            r#"
            PRAGMA journal_mode = WAL;
            PRAGMA foreign_keys = ON;

            CREATE TABLE IF NOT EXISTS schema_info (
                schema_version INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS games (
                id              INTEGER PRIMARY KEY,
                canonical_hash  BLOB NOT NULL UNIQUE,
                board_size      INTEGER NOT NULL,
                move_count      INTEGER NOT NULL,
                move_file       TEXT NOT NULL UNIQUE,
                created_at      TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            );

            CREATE TABLE IF NOT EXISTS sources (
                id              INTEGER PRIMARY KEY,
                name            TEXT NOT NULL,
                version         TEXT NOT NULL,
                created_at      TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,

                UNIQUE(name, version)
            );

            CREATE TABLE IF NOT EXISTS game_sources (
                id              INTEGER PRIMARY KEY,
                game_id         INTEGER NOT NULL,
                source_id       INTEGER NOT NULL,
                original_path   TEXT NOT NULL,
                imported_at     TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,

                FOREIGN KEY(game_id) REFERENCES games(id),
                FOREIGN KEY(source_id) REFERENCES sources(id),

                UNIQUE(source_id, original_path)
            );

            CREATE TABLE IF NOT EXISTS game_metadata (
                game_source_id  INTEGER PRIMARY KEY,
                black_player    TEXT,
                white_player    TEXT,
                played_date     TEXT,
                played_date_sort  TEXT,
                event           TEXT,
                result          TEXT,
                komi            REAL,
                handicap        INTEGER,

                FOREIGN KEY(game_source_id)
                    REFERENCES game_sources(id)
                    ON DELETE CASCADE
            );
        CREATE TABLE IF NOT EXISTS indexed_games (
    game_id           INTEGER PRIMARY KEY,
    index_version     INTEGER NOT NULL,
    indexed_at        TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    occurrence_count  INTEGER NOT NULL,

    FOREIGN KEY(game_id)
        REFERENCES games(id)
        ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS exact_positions (
    position_hash     BLOB NOT NULL,
    game_id           INTEGER NOT NULL,
    move_number       INTEGER NOT NULL,
    side_to_move      INTEGER NOT NULL,
    ko_point          INTEGER,

    FOREIGN KEY(game_id)
        REFERENCES games(id)
        ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS exact_positions_hash
    ON exact_positions(position_hash);

CREATE INDEX IF NOT EXISTS exact_positions_game
    ON exact_positions(game_id);

            CREATE INDEX IF NOT EXISTS game_sources_game_id
                ON game_sources(game_id);

            CREATE INDEX IF NOT EXISTS game_sources_source_id
                ON game_sources(source_id);

            CREATE INDEX IF NOT EXISTS game_metadata_black_player
                ON game_metadata(black_player);

            CREATE INDEX IF NOT EXISTS game_metadata_white_player
                ON game_metadata(white_player);

            CREATE INDEX IF NOT EXISTS game_metadata_played_date
                ON game_metadata(played_date);

            CREATE INDEX IF NOT EXISTS game_metadata_played_date_sort
                ON game_metadata(played_date_sort);

            CREATE INDEX IF NOT EXISTS game_metadata_event
                ON game_metadata(event);
            "#,
        )
        .context("creating database schema")?;

    check_or_record_schema_version(&connection)?;

    println!("Initialised database: {}", root.display());
    println!("Schema version: {SCHEMA_VERSION}");
    println!("Metadata: {}", sqlite_path.display());
    println!("Game files: {}", root.join("games").display());

    Ok(())
}

fn migrate(connection: &Connection, from_version: i64) -> Result<()> {
    let transaction = connection
        .unchecked_transaction()
        .context("starting database schema migration")?;

    let mut version = from_version;

    while version < SCHEMA_VERSION {
        match version {
            2 => migrate_2_to_3(&transaction)?,
            3 => migrate_3_to_4(&transaction)?,
            4 => migrate_4_to_5(&transaction)?,
            _ => bail!("no migration available from schema version {version}"),
        }

        version += 1;
    }

    transaction
        .execute(
            "UPDATE schema_info SET schema_version = ?1",
            [SCHEMA_VERSION],
        )
        .context("recording migrated schema version")?;

    transaction
        .commit()
        .context("committing database schema migration")?;

    Ok(())
}

fn migrate_2_to_3(connection: &Connection) -> Result<()> {
    connection.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS indexed_games (
            game_id INTEGER PRIMARY KEY,
            index_version INTEGER NOT NULL,
            indexed_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
            occurrence_count INTEGER NOT NULL,

            FOREIGN KEY(game_id)
                REFERENCES games(id)
                ON DELETE CASCADE
        );

        CREATE TABLE IF NOT EXISTS exact_positions (
            position_hash BLOB NOT NULL,
            game_id INTEGER NOT NULL,
            move_number INTEGER NOT NULL,
            side_to_move INTEGER NOT NULL,
            ko_point INTEGER,

            FOREIGN KEY(game_id)
                REFERENCES games(id)
                ON DELETE CASCADE
        );

        CREATE INDEX IF NOT EXISTS exact_positions_hash
            ON exact_positions(position_hash);

        CREATE INDEX IF NOT EXISTS exact_positions_game
            ON exact_positions(game_id);
        "#,
    )?;

    Ok(())
}

fn migrate_3_to_4(connection: &Connection) -> Result<()> {
    let dates = {
        let mut statement = connection
            .prepare(
                r#"
                SELECT game_source_id, played_date
                FROM game_metadata
                WHERE played_date IS NOT NULL
                "#,
            )
            .context("preparing existing played-date migration")?;

        let rows = statement
            .query_map([], |row| {
                Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
            })
            .context("reading existing played dates")?;

        rows.collect::<rusqlite::Result<Vec<_>>>()
            .context("collecting existing played dates")?
    };

    for (game_source_id, played_date) in dates {
        let normalised = normalise_played_date(&played_date);

        if normalised == played_date {
            continue;
        }

        connection
            .execute(
                r#"
                UPDATE game_metadata
                SET played_date = ?1
                WHERE game_source_id = ?2
                "#,
                params![normalised, game_source_id],
            )
            .with_context(|| format!("normalising played date for game source {game_source_id}"))?;
    }

    Ok(())
}

fn migrate_4_to_5(connection: &Connection) -> Result<()> {
    connection
        .execute(
            "ALTER TABLE game_metadata ADD COLUMN played_date_sort TEXT",
            [],
        )
        .context("adding played-date sort column")?;

    let dates = {
        let mut statement = connection
            .prepare(
                r#"
                SELECT game_source_id, played_date
                FROM game_metadata
                WHERE played_date IS NOT NULL
                "#,
            )
            .context("preparing played-date sort migration")?;

        let rows = statement
            .query_map([], |row| {
                Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
            })
            .context("reading played dates for sort migration")?;

        rows.collect::<rusqlite::Result<Vec<_>>>()
            .context("collecting played dates for sort migration")?
    };

    {
        let mut update = connection
            .prepare(
                r#"
            UPDATE game_metadata
            SET played_date_sort = ?1
            WHERE game_source_id = ?2
            "#,
            )
            .context("preparing played-date sort updates")?;

        for (game_source_id, played_date) in dates {
            let Some(sort_key) = played_date_sort_key(&played_date) else {
                continue;
            };

            update
                .execute(params![sort_key, game_source_id])
                .with_context(|| {
                    format!("setting played-date sort key for game source {game_source_id}")
                })?;
        }
    }

    connection
        .execute(
            r#"
            CREATE INDEX IF NOT EXISTS game_metadata_played_date_sort
            ON game_metadata(played_date_sort)
            "#,
            [],
        )
        .context("creating played-date sort index")?;

    Ok(())
}

fn check_or_record_schema_version(connection: &Connection) -> Result<()> {
    let current_version: Option<i64> = connection
        .query_row(
            "SELECT schema_version FROM schema_info LIMIT 1",
            [],
            |row| row.get(0),
        )
        .optional()
        .context("reading schema version")?;

    match current_version {
        Some(version) if version < SCHEMA_VERSION => migrate(connection, version),

        Some(version) if version > SCHEMA_VERSION => {
            bail!("database schema version {version} is newer than this program supports");
        }

        Some(_) => Ok(()),

        None => {
            connection
                .execute(
                    "INSERT INTO schema_info(schema_version) VALUES (?1)",
                    [SCHEMA_VERSION],
                )
                .context("recording schema version")?;

            Ok(())
        }
    }
}

pub fn open(root: &Path) -> Result<Connection> {
    let sqlite_path = metadata_path(root);

    if !sqlite_path.is_file() {
        bail!("{} is not an initialised Bermuda database", root.display());
    }

    let connection = Connection::open(&sqlite_path)
        .with_context(|| format!("opening {}", sqlite_path.display()))?;

    connection
        .execute_batch("PRAGMA foreign_keys = ON;")
        .context("enabling SQLite foreign keys")?;

    check_or_record_schema_version(&connection)?;

    Ok(connection)
}

pub fn metadata_path(root: &Path) -> PathBuf {
    root.join("metadata.sqlite3")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrates_version_3_dates_and_creates_sort_keys() -> Result<()> {
        let connection = Connection::open_in_memory()?;

        connection.execute_batch(
            r#"
            CREATE TABLE schema_info (
                schema_version INTEGER NOT NULL
            );

            INSERT INTO schema_info(schema_version)
            VALUES (3);

            CREATE TABLE game_metadata (
                game_source_id INTEGER PRIMARY KEY,
                played_date TEXT
            );

            INSERT INTO game_metadata(game_source_id, played_date)
            VALUES
                (1, 'c. 1683'),
                (2, '1683'),
                (3, '1683-07'),
                (4, '1683-07-12'),
                (5, 'Published 2012-09'),
                (6, NULL);
            "#,
        )?;

        migrate(&connection, 3)?;

        let schema_version: i64 =
            connection.query_row("SELECT schema_version FROM schema_info", [], |row| {
                row.get(0)
            })?;

        assert_eq!(schema_version, 5);

        let stored_dates = {
            let mut statement = connection.prepare(
                r#"
        SELECT
            game_source_id,
            played_date,
            played_date_sort
        FROM game_metadata
        ORDER BY game_source_id
        "#,
            )?;

            let rows = statement.query_map([], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, Option<String>>(1)?,
                    row.get::<_, Option<String>>(2)?,
                ))
            })?;

            rows.collect::<rusqlite::Result<Vec<_>>>()?
        };

        assert_eq!(
            stored_dates,
            vec![
                (
                    1,
                    Some("1683-01-01".to_owned()),
                    Some("1683-01-01".to_owned()),
                ),
                (
                    2,
                    Some("1683-01-01".to_owned()),
                    Some("1683-01-01".to_owned()),
                ),
                (
                    3,
                    Some("1683-07-01".to_owned()),
                    Some("1683-07-01".to_owned()),
                ),
                (
                    4,
                    Some("1683-07-12".to_owned()),
                    Some("1683-07-12".to_owned()),
                ),
                (
                    5,
                    Some("Published 2012-09".to_owned()),
                    Some("2012-09-01".to_owned()),
                ),
                (6, None, None),
            ]
        );

        Ok(())
    }
}
