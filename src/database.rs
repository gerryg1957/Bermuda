use anyhow::{Context, Result, bail};
use rusqlite::{Connection, OptionalExtension};
use std::{
    fs,
    path::{Path, PathBuf},
};

const SCHEMA_VERSION: i64 = 1;

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
            content_hash    BLOB NOT NULL UNIQUE,
            source_path     TEXT NOT NULL,
            black_player    TEXT,
            white_player    TEXT,
            played_date     TEXT,
            event           TEXT,
            result          TEXT,
            komi            REAL,
            handicap        INTEGER,
            board_size      INTEGER NOT NULL,
            move_count      INTEGER NOT NULL,
            move_file       TEXT NOT NULL,
            imported_at     TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
        );

        CREATE INDEX IF NOT EXISTS games_black_player
            ON games(black_player);

        CREATE INDEX IF NOT EXISTS games_white_player
            ON games(white_player);

        CREATE INDEX IF NOT EXISTS games_played_date
            ON games(played_date);
        "#,
        )
        .context("creating database schema")?;

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
            bail!("unsupported database schema version {version}; expected {SCHEMA_VERSION}");
        }
        Some(_) => {}
        None => {
            connection
                .execute(
                    "INSERT INTO schema_info(schema_version) VALUES (?1)",
                    [SCHEMA_VERSION],
                )
                .context("recording schema version")?;
        }
    }

    println!("Initialised database: {}", root.display());
    println!("Metadata: {}", sqlite_path.display());
    println!("Game files: {}", root.join("games").display());

    Ok(())
}

pub fn metadata_path(root: &Path) -> PathBuf {
    root.join("metadata.sqlite3")
}
