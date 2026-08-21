use anyhow::{Context, Result, bail};
use rusqlite::{Connection, OptionalExtension, params};

use crate::game_date::{normalise_played_date, played_date_sort_key};
use std::{
    fs,
    path::{Path, PathBuf},
};

const SCHEMA_VERSION: i64 = 6;

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

            CREATE TABLE IF NOT EXISTS players (
                id              INTEGER PRIMARY KEY,
                preferred_name  TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS player_aliases (
                id              INTEGER PRIMARY KEY,
                player_id       INTEGER NOT NULL,
                name            TEXT NOT NULL,
                source_id       INTEGER,
                notes           TEXT,

                FOREIGN KEY(player_id)
                    REFERENCES players(id)
                    ON DELETE CASCADE,
                FOREIGN KEY(source_id)
                    REFERENCES sources(id)
                    ON DELETE CASCADE
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
                black_player_id INTEGER,
                white_player_id INTEGER,

                FOREIGN KEY(game_source_id)
                    REFERENCES game_sources(id)
                    ON DELETE CASCADE,
                FOREIGN KEY(black_player_id)
                    REFERENCES players(id),
                FOREIGN KEY(white_player_id)
                    REFERENCES players(id)
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

            CREATE INDEX IF NOT EXISTS players_preferred_name
                ON players(preferred_name);

            CREATE INDEX IF NOT EXISTS player_aliases_name_source
                ON player_aliases(name, source_id);

            CREATE INDEX IF NOT EXISTS player_aliases_player_id
                ON player_aliases(player_id);

            CREATE UNIQUE INDEX IF NOT EXISTS player_aliases_global_assignment
                ON player_aliases(player_id, name)
                WHERE source_id IS NULL;

            CREATE UNIQUE INDEX IF NOT EXISTS player_aliases_source_assignment
                ON player_aliases(player_id, source_id, name)
                WHERE source_id IS NOT NULL;

            CREATE INDEX IF NOT EXISTS game_metadata_black_player
                ON game_metadata(black_player);

            CREATE INDEX IF NOT EXISTS game_metadata_white_player
                ON game_metadata(white_player);

            CREATE INDEX IF NOT EXISTS game_metadata_black_player_id
                ON game_metadata(black_player_id);

            CREATE INDEX IF NOT EXISTS game_metadata_white_player_id
                ON game_metadata(white_player_id);

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
            5 => migrate_5_to_6(&transaction)?,
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

fn migrate_5_to_6(connection: &Connection) -> Result<()> {
    connection
        .execute_batch(
            r#"
            CREATE TABLE players (
                id              INTEGER PRIMARY KEY,
                preferred_name  TEXT NOT NULL
            );

            CREATE TABLE player_aliases (
                id              INTEGER PRIMARY KEY,
                player_id       INTEGER NOT NULL,
                name            TEXT NOT NULL,
                source_id       INTEGER,
                notes           TEXT,

                FOREIGN KEY(player_id)
                    REFERENCES players(id)
                    ON DELETE CASCADE,
                FOREIGN KEY(source_id)
                    REFERENCES sources(id)
                    ON DELETE CASCADE
            );

            ALTER TABLE game_metadata
                ADD COLUMN black_player_id INTEGER
                    REFERENCES players(id);

            ALTER TABLE game_metadata
                ADD COLUMN white_player_id INTEGER
                    REFERENCES players(id);

            CREATE INDEX players_preferred_name
                ON players(preferred_name);

            CREATE INDEX player_aliases_name_source
                ON player_aliases(name, source_id);

            CREATE INDEX player_aliases_player_id
                ON player_aliases(player_id);

            CREATE UNIQUE INDEX player_aliases_global_assignment
                ON player_aliases(player_id, name)
                WHERE source_id IS NULL;

            CREATE UNIQUE INDEX player_aliases_source_assignment
                ON player_aliases(player_id, source_id, name)
                WHERE source_id IS NOT NULL;

            CREATE INDEX game_metadata_black_player_id
                ON game_metadata(black_player_id);

            CREATE INDEX game_metadata_white_player_id
                ON game_metadata(white_player_id);
            "#,
        )
        .context("adding player identity schema")?;

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

        assert_eq!(schema_version, 6);

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
    #[test]
    fn migrates_version_5_to_player_identity_without_rewriting_source_names() -> Result<()> {
        let connection = Connection::open_in_memory()?;

        connection.execute_batch(
            r#"
            PRAGMA foreign_keys = ON;

            CREATE TABLE schema_info (
                schema_version INTEGER NOT NULL
            );

            INSERT INTO schema_info(schema_version)
            VALUES (5);

            CREATE TABLE sources (
                id              INTEGER PRIMARY KEY,
                name            TEXT NOT NULL,
                version         TEXT NOT NULL
            );

            INSERT INTO sources(id, name, version)
            VALUES (1, 'GoGoD', '2026');

            CREATE TABLE game_metadata (
                game_source_id    INTEGER PRIMARY KEY,
                black_player      TEXT,
                white_player      TEXT,
                played_date       TEXT,
                played_date_sort  TEXT,
                event             TEXT,
                result            TEXT,
                komi              REAL,
                handicap          INTEGER
            );

            INSERT INTO game_metadata(
                game_source_id,
                black_player,
                white_player
            )
            VALUES (
                10,
                'Cho Chikun',
                'Kobayashi Satoru'
            );
            "#,
        )?;

        migrate(&connection, 5)?;

        let schema_version: i64 =
            connection.query_row("SELECT schema_version FROM schema_info", [], |row| {
                row.get(0)
            })?;

        assert_eq!(schema_version, 6);

        let player_count: i64 =
            connection.query_row("SELECT COUNT(*) FROM players", [], |row| row.get(0))?;

        let alias_count: i64 =
            connection.query_row("SELECT COUNT(*) FROM player_aliases", [], |row| row.get(0))?;

        assert_eq!(player_count, 0);
        assert_eq!(alias_count, 0);

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
         * Migration is structural only.  It neither invents identities
         * nor rewrites the metadata supplied by an SGF source.
         */
        assert_eq!(black_player.as_deref(), Some("Cho Chikun"));
        assert_eq!(white_player.as_deref(), Some("Kobayashi Satoru"));
        assert_eq!(black_player_id, None);
        assert_eq!(white_player_id, None);

        connection.execute(
            "INSERT INTO players(preferred_name) VALUES (?1)",
            ["Cho Chikun"],
        )?;

        let player_id = connection.last_insert_rowid();

        connection.execute(
            r#"
            INSERT INTO player_aliases(
                player_id,
                name,
                source_id,
                notes
            )
            VALUES (?1, ?2, ?3, ?4)
            "#,
            params![player_id, "Cho Chikun", 1_i64, "migration test",],
        )?;

        connection.execute(
            r#"
            UPDATE game_metadata
            SET black_player_id = ?1
            WHERE game_source_id = 10
            "#,
            [player_id],
        )?;

        let (stored_name, stored_player_id): (Option<String>, Option<i64>) = connection.query_row(
            r#"
                SELECT black_player, black_player_id
                FROM game_metadata
                WHERE game_source_id = 10
                "#,
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;

        /*
         * Bermuda's interpretation is stored alongside the source text,
         * never in place of it.
         */
        assert_eq!(stored_name.as_deref(), Some("Cho Chikun"));
        assert_eq!(stored_player_id, Some(player_id));

        Ok(())
    }
}
