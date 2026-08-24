use anyhow::{Context, Result, bail};
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};

use crate::game_date::{normalise_played_date, played_date_sort_key};
use std::{
    fs,
    path::{Path, PathBuf},
    time::Duration,
};

const SCHEMA_VERSION: i64 = 7;
const SCHEMA_MIGRATION_BUSY_TIMEOUT: Duration = Duration::from_secs(30);

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
    let mut connection = Connection::open(&sqlite_path)
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
                preferred_name  TEXT NOT NULL,
                catalogue_key   TEXT
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

            CREATE TABLE IF NOT EXISTS player_catalogue_state (
                id              INTEGER PRIMARY KEY CHECK (id = 1),
                data_version    INTEGER NOT NULL CHECK (data_version >= 0)
            );

            CREATE TABLE IF NOT EXISTS player_catalogue_players (
                catalogue_key   TEXT PRIMARY KEY,
                preferred_name  TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS player_catalogue_aliases (
                id              INTEGER PRIMARY KEY,
                catalogue_key   TEXT NOT NULL,
                name            TEXT NOT NULL,
                notes           TEXT,

                FOREIGN KEY(catalogue_key)
                    REFERENCES player_catalogue_players(catalogue_key)
                    ON DELETE CASCADE
            );

            CREATE UNIQUE INDEX IF NOT EXISTS players_catalogue_key
                ON players(catalogue_key)
                WHERE catalogue_key IS NOT NULL;

            CREATE INDEX IF NOT EXISTS player_catalogue_players_preferred_name
                ON player_catalogue_players(preferred_name COLLATE NOCASE);

            CREATE INDEX IF NOT EXISTS player_catalogue_aliases_name
                ON player_catalogue_aliases(name COLLATE NOCASE);

            CREATE UNIQUE INDEX IF NOT EXISTS player_catalogue_alias_assignment
                ON player_catalogue_aliases(catalogue_key, name);

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
                black_player_catalogue_derived INTEGER NOT NULL DEFAULT 0
                    CHECK (black_player_catalogue_derived IN (0, 1)),
                white_player_catalogue_derived INTEGER NOT NULL DEFAULT 0
                    CHECK (white_player_catalogue_derived IN (0, 1)),

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

    check_or_record_schema_version(&mut connection)?;

    println!("Initialised database: {}", root.display());
    println!("Schema version: {SCHEMA_VERSION}");
    println!("Metadata: {}", sqlite_path.display());
    println!("Game files: {}", root.join("games").display());

    Ok(())
}

fn migrate_locked(connection: &Connection, from_version: i64) -> Result<()> {
    let mut version = from_version;

    while version < SCHEMA_VERSION {
        match version {
            2 => migrate_2_to_3(connection)?,
            3 => migrate_3_to_4(connection)?,
            4 => migrate_4_to_5(connection)?,
            5 => migrate_5_to_6(connection)?,
            6 => migrate_6_to_7(connection)?,
            _ => bail!("no migration available from schema version {version}"),
        }

        version += 1;
    }

    connection
        .execute(
            "UPDATE schema_info SET schema_version = ?1",
            [SCHEMA_VERSION],
        )
        .context("recording migrated schema version")?;

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

fn migrate_6_to_7(connection: &Connection) -> Result<()> {
    connection
        .execute_batch(
            r#"
            ALTER TABLE players
                ADD COLUMN catalogue_key TEXT;

            ALTER TABLE game_metadata
                ADD COLUMN black_player_catalogue_derived
                    INTEGER NOT NULL DEFAULT 0
                    CHECK (black_player_catalogue_derived IN (0, 1));

            ALTER TABLE game_metadata
                ADD COLUMN white_player_catalogue_derived
                    INTEGER NOT NULL DEFAULT 0
                    CHECK (white_player_catalogue_derived IN (0, 1));

            CREATE TABLE player_catalogue_state (
                id              INTEGER PRIMARY KEY CHECK (id = 1),
                data_version    INTEGER NOT NULL CHECK (data_version >= 0)
            );

            CREATE TABLE player_catalogue_players (
                catalogue_key   TEXT PRIMARY KEY,
                preferred_name  TEXT NOT NULL
            );

            CREATE TABLE player_catalogue_aliases (
                id              INTEGER PRIMARY KEY,
                catalogue_key   TEXT NOT NULL,
                name            TEXT NOT NULL,
                notes           TEXT,

                FOREIGN KEY(catalogue_key)
                    REFERENCES player_catalogue_players(catalogue_key)
                    ON DELETE CASCADE
            );

            CREATE UNIQUE INDEX players_catalogue_key
                ON players(catalogue_key)
                WHERE catalogue_key IS NOT NULL;

            CREATE INDEX player_catalogue_players_preferred_name
                ON player_catalogue_players(preferred_name COLLATE NOCASE);

            CREATE INDEX player_catalogue_aliases_name
                ON player_catalogue_aliases(name COLLATE NOCASE);

            CREATE UNIQUE INDEX player_catalogue_alias_assignment
                ON player_catalogue_aliases(catalogue_key, name);
            "#,
        )
        .context("adding supplied player catalogue foundation")?;

    Ok(())
}

fn read_schema_version(connection: &Connection) -> Result<Option<i64>> {
    connection
        .query_row(
            "SELECT schema_version FROM schema_info LIMIT 1",
            [],
            |row| row.get(0),
        )
        .optional()
        .context("reading schema version")
}

fn check_or_record_schema_version(connection: &mut Connection) -> Result<()> {
    check_or_record_schema_version_after_observation(connection, |_| {})
}

fn check_or_record_schema_version_after_observation<F>(
    connection: &mut Connection,
    after_observation: F,
) -> Result<()>
where
    F: FnOnce(Option<i64>),
{
    let observed_version = read_schema_version(connection)?;

    match observed_version {
        Some(version) if version > SCHEMA_VERSION => {
            bail!("database schema version {version} is newer than this program supports");
        }

        Some(version) if version == SCHEMA_VERSION => return Ok(()),

        _ => {}
    }

    /*
     * This hook is a no-op in normal use.  The database tests use it to
     * deterministically reproduce another connection completing a migration
     * after this connection has observed the old version.
     */
    after_observation(observed_version);

    /*
     * An older schema requires a write transaction.  Acquire the write lock
     * before deciding which migration to perform, then read the version again.
     *
     * Another Bermuda connection may have completed the migration after our
     * first read but before we acquired this lock.  The second read prevents
     * us from applying the same migration twice.
     */
    connection
        .busy_timeout(SCHEMA_MIGRATION_BUSY_TIMEOUT)
        .context("setting database migration busy timeout")?;

    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .context("locking database schema for migration")?;

    let locked_version = read_schema_version(&transaction)?;

    match locked_version {
        Some(version) if version < SCHEMA_VERSION => {
            migrate_locked(&transaction, version)?;
        }

        Some(version) if version > SCHEMA_VERSION => {
            bail!("database schema version {version} is newer than this program supports");
        }

        Some(_) => {}

        None => {
            transaction
                .execute(
                    "INSERT INTO schema_info(schema_version) VALUES (?1)",
                    [SCHEMA_VERSION],
                )
                .context("recording schema version")?;
        }
    }

    transaction
        .commit()
        .context("committing database schema version check")?;

    Ok(())
}

pub fn open(root: &Path) -> Result<Connection> {
    let sqlite_path = metadata_path(root);

    if !sqlite_path.is_file() {
        bail!("{} is not an initialised Bermuda database", root.display());
    }

    let mut connection = Connection::open(&sqlite_path)
        .with_context(|| format!("opening {}", sqlite_path.display()))?;

    connection
        .execute_batch("PRAGMA foreign_keys = ON;")
        .context("enabling SQLite foreign keys")?;

    check_or_record_schema_version(&mut connection)?;

    Ok(connection)
}

pub fn metadata_path(root: &Path) -> PathBuf {
    root.join("metadata.sqlite3")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{sync::mpsc, thread};
    use tempfile::tempdir;

    #[test]
    fn migrates_version_3_dates_and_creates_sort_keys() -> Result<()> {
        let mut connection = Connection::open_in_memory()?;

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

        check_or_record_schema_version(&mut connection)?;

        let schema_version: i64 =
            connection.query_row("SELECT schema_version FROM schema_info", [], |row| {
                row.get(0)
            })?;

        assert_eq!(schema_version, 7);

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
        let mut connection = Connection::open_in_memory()?;

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

        check_or_record_schema_version(&mut connection)?;

        let schema_version: i64 =
            connection.query_row("SELECT schema_version FROM schema_info", [], |row| {
                row.get(0)
            })?;

        assert_eq!(schema_version, 7);

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

    #[test]
    fn migrates_version_6_to_player_catalogue_foundation() -> Result<()> {
        let mut connection = Connection::open_in_memory()?;

        connection.execute_batch(
            r#"
            PRAGMA foreign_keys = ON;

            CREATE TABLE schema_info (
                schema_version INTEGER NOT NULL
            );

            INSERT INTO schema_info(schema_version)
            VALUES (6);

            CREATE TABLE players (
                id              INTEGER PRIMARY KEY,
                preferred_name  TEXT NOT NULL
            );

            CREATE TABLE game_metadata (
                game_source_id  INTEGER PRIMARY KEY,
                black_player    TEXT,
                white_player    TEXT,
                black_player_id INTEGER REFERENCES players(id),
                white_player_id INTEGER REFERENCES players(id)
            );

            INSERT INTO players(id, preferred_name)
            VALUES (1, 'Cho Chikun');

            INSERT INTO game_metadata(
                game_source_id,
                black_player,
                white_player,
                black_player_id,
                white_player_id
            )
            VALUES (
                10,
                'Cho Chikun',
                'Kobayashi Satoru',
                1,
                NULL
            );
            "#,
        )?;

        check_or_record_schema_version(&mut connection)?;

        assert_eq!(read_schema_version(&connection)?, Some(7));

        let (
            black_player,
            black_player_id,
            black_catalogue_derived,
            white_player_id,
            white_catalogue_derived,
        ): (Option<String>, Option<i64>, i64, Option<i64>, i64) = connection.query_row(
            r#"
                SELECT
                    black_player,
                    black_player_id,
                    black_player_catalogue_derived,
                    white_player_id,
                    white_player_catalogue_derived
                FROM game_metadata
                WHERE game_source_id = 10
                "#,
            [],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                ))
            },
        )?;

        /*
         * A pre-schema-7 identity link is local knowledge. The migration
         * therefore preserves both the numeric identity and the original PB
         * text while the new catalogue-derived flag defaults to false.
         */
        assert_eq!(black_player.as_deref(), Some("Cho Chikun"));
        assert_eq!(black_player_id, Some(1));
        assert_eq!(black_catalogue_derived, 0);

        assert_eq!(white_player_id, None);
        assert_eq!(white_catalogue_derived, 0);

        let catalogue_key: Option<String> = connection.query_row(
            "SELECT catalogue_key FROM players WHERE id = 1",
            [],
            |row| row.get(0),
        )?;

        assert_eq!(catalogue_key, None);

        let catalogue_player_count: i64 =
            connection.query_row("SELECT COUNT(*) FROM player_catalogue_players", [], |row| {
                row.get(0)
            })?;

        let catalogue_alias_count: i64 =
            connection.query_row("SELECT COUNT(*) FROM player_catalogue_aliases", [], |row| {
                row.get(0)
            })?;

        assert_eq!(catalogue_player_count, 0);
        assert_eq!(catalogue_alias_count, 0);

        /*
         * Prove that the new catalogue structures can represent one supplied
         * identity and alias without changing the existing local player.
         */
        connection.execute(
            "INSERT INTO player_catalogue_state(id, data_version) VALUES (1, 1)",
            [],
        )?;

        connection.execute(
            r#"
            INSERT INTO player_catalogue_players(catalogue_key, preferred_name)
            VALUES ('kr:lee-sedol', 'Lee Sedol')
            "#,
            [],
        )?;

        connection.execute(
            r#"
            INSERT INTO player_catalogue_aliases(catalogue_key, name)
            VALUES ('kr:lee-sedol', 'Yi Se-tol')
            "#,
            [],
        )?;

        connection.execute(
            r#"
            INSERT INTO players(preferred_name, catalogue_key)
            VALUES ('Lee Sedol', 'kr:lee-sedol')
            "#,
            [],
        )?;

        /*
         * One supplied catalogue identity may be materialised only once in a
         * project database.
         */
        assert!(
            connection
                .execute(
                    r#"
                    INSERT INTO players(preferred_name, catalogue_key)
                    VALUES ('Duplicate Lee Sedol', 'kr:lee-sedol')
                    "#,
                    [],
                )
                .is_err()
        );

        /*
         * The provenance flag is deliberately boolean.
         */
        assert!(
            connection
                .execute(
                    r#"
                    UPDATE game_metadata
                    SET black_player_catalogue_derived = 2
                    WHERE game_source_id = 10
                    "#,
                    [],
                )
                .is_err()
        );

        Ok(())
    }

    #[test]
    fn stale_schema_observation_does_not_repeat_completed_migration() -> Result<()> {
        let temporary_directory = tempdir()?;
        let sqlite_path = temporary_directory.path().join("metadata.sqlite3");

        let mut first_connection = Connection::open(&sqlite_path)?;

        first_connection.execute_batch(
            r#"
            PRAGMA foreign_keys = ON;

            CREATE TABLE schema_info (
                schema_version INTEGER NOT NULL
            );

            INSERT INTO schema_info(schema_version)
            VALUES (5);

            CREATE TABLE sources (
                id       INTEGER PRIMARY KEY,
                name     TEXT NOT NULL,
                version  TEXT NOT NULL
            );

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
            "#,
        )?;

        let (observed_sender, observed_receiver) = mpsc::channel();
        let (continue_sender, continue_receiver) = mpsc::channel();

        let second_sqlite_path = sqlite_path.clone();

        let second_thread = thread::spawn(move || -> Result<()> {
            let mut second_connection = Connection::open(second_sqlite_path)?;

            check_or_record_schema_version_after_observation(
                &mut second_connection,
                move |observed_version| {
                    observed_sender
                        .send(observed_version)
                        .expect("report observed schema version");

                    continue_receiver
                        .recv()
                        .expect("wait for first migration to complete");
                },
            )
        });

        /*
         * Connection B has definitely completed its first schema-version
         * read, and has deliberately been paused before taking the migration
         * lock.  This avoids timing assumptions or sleeps in the test.
         */
        let observed_version = observed_receiver
            .recv_timeout(Duration::from_secs(5))
            .context("waiting for stale schema observation")?;

        assert_eq!(observed_version, Some(5));

        /*
         * Connection A now performs the migration while B still holds the
         * stale observation of schema 5.
         */
        check_or_record_schema_version(&mut first_connection)?;

        let migrated_version = read_schema_version(&first_connection)?;
        assert_eq!(migrated_version, Some(7));

        /*
         * B must now take the IMMEDIATE transaction, re-read schema_info,
         * discover schema 7, and refrain from migrating schema 5 a second time.
         */
        continue_sender
            .send(())
            .context("releasing second schema opener")?;

        second_thread
            .join()
            .expect("second schema opener thread panicked")?;

        assert_eq!(read_schema_version(&first_connection)?, Some(7));

        let player_count: i64 =
            first_connection.query_row("SELECT COUNT(*) FROM players", [], |row| row.get(0))?;

        let alias_count: i64 =
            first_connection
                .query_row("SELECT COUNT(*) FROM player_aliases", [], |row| row.get(0))?;

        assert_eq!(player_count, 0);
        assert_eq!(alias_count, 0);

        Ok(())
    }
}
