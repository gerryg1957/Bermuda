use anyhow::{Context, Result, bail};
use rusqlite::{Connection, OptionalExtension};
use std::{
    fs,
    path::{Path, PathBuf},
};

const SCHEMA_VERSION: i64 = 3;

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
        Some(version) if version != SCHEMA_VERSION => {
            bail!(
                "unsupported database schema version {version}; \
                 this program expects version {SCHEMA_VERSION}"
            );
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
        bail!("{} is not an initialised MoyoDB database", root.display());
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
