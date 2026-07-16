use crate::database;
use anyhow::{Context, Result, bail};
use moyodb_core::{PositionOccurrence, position_stream, read_move_file};
use rusqlite::Connection;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameToIndex {
    pub game_id: i64,
    pub move_file: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexedPositionStream {
    pub game_id: i64,
    pub occurrences: Vec<PositionOccurrence>,
}

pub struct PositionIndexer {
    database_root: PathBuf,
    connection: Connection,
}

impl PositionIndexer {
    /// Opens an existing MoyoDB database.
    pub fn open(database_root: &Path) -> Result<Self> {
        let connection = database::open(database_root)?;

        Ok(Self {
            database_root: database_root.to_path_buf(),
            connection,
        })
    }

    /// Reads and replays one compact move file.
    pub fn replay_game(&self, game: &GameToIndex) -> Result<IndexedPositionStream> {
        if !game.move_file.is_file() {
            bail!(
                "move file for game {} does not exist: {}",
                game.game_id,
                game.move_file.display()
            );
        }

        let record = read_move_file(&game.move_file).with_context(|| {
            format!(
                "reading move file for game {} from {}",
                game.game_id,
                game.move_file.display()
            )
        })?;

        let occurrences = position_stream(&record)
            .with_context(|| format!("building position stream for game {}", game.game_id))?;

        Ok(IndexedPositionStream {
            game_id: game.game_id,
            occurrences,
        })
    }

    /// Reads and replays a game directly by database ID.
    pub fn replay_game_by_id(&self, game_id: i64) -> Result<IndexedPositionStream> {
        let game = self
            .game_by_id(game_id)?
            .with_context(|| format!("game {game_id} does not exist"))?;

        self.replay_game(&game)
    }

    fn game_by_id(&self, game_id: i64) -> Result<Option<GameToIndex>> {
        use rusqlite::OptionalExtension;

        self.connection
            .query_row(
                "SELECT id, move_file FROM games WHERE id = ?1",
                [game_id],
                |row| {
                    let id: i64 = row.get(0)?;
                    let relative_move_file: String = row.get(1)?;

                    Ok(GameToIndex {
                        game_id: id,
                        move_file: self.database_root.join(relative_move_file),
                    })
                },
            )
            .optional()
            .context("reading game from database")
    }

    /// Returns every game that has not yet been indexed with `index_version`.
    pub fn games_to_index(&self, index_version: i64) -> Result<Vec<GameToIndex>> {
        let mut statement = self
            .connection
            .prepare(
                r#"
                SELECT
                    games.id,
                    games.move_file
                FROM games
                LEFT JOIN indexed_games
                    ON indexed_games.game_id = games.id
                    AND indexed_games.index_version = ?1
                WHERE indexed_games.game_id IS NULL
                ORDER BY games.id
                "#,
            )
            .context("preparing unindexed-games query")?;

        let rows = statement
            .query_map([index_version], |row| {
                let game_id: i64 = row.get(0)?;
                let relative_move_file: String = row.get(1)?;

                Ok(GameToIndex {
                    game_id,
                    move_file: self.database_root.join(relative_move_file),
                })
            })
            .context("querying games awaiting position indexing")?;

        let mut games = Vec::new();

        for row in rows {
            games.push(row.context("reading game awaiting position indexing")?);
        }

        Ok(games)
    }

    /// Returns the number of games still requiring indexing.
    pub fn count_games_to_index(&self, index_version: i64) -> Result<u64> {
        let count: i64 = self
            .connection
            .query_row(
                r#"
                SELECT COUNT(*)
                FROM games
                LEFT JOIN indexed_games
                    ON indexed_games.game_id = games.id
                    AND indexed_games.index_version = ?1
                WHERE indexed_games.game_id IS NULL
                "#,
                [index_version],
                |row| row.get(0),
            )
            .context("counting games awaiting position indexing")?;

        u64::try_from(count).context("negative game count returned by SQLite")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use moyodb_core::{Color, GameRecord, Metadata, Move, write_move_file};
    use rusqlite::params;
    use tempfile::TempDir;

    fn create_test_database() -> (TempDir, PathBuf) {
        let temporary = TempDir::new().expect("create temporary directory");
        let root = temporary.path().join("database");

        database::initialise(&root).expect("initialise test database");

        (temporary, root)
    }

    fn insert_game(connection: &Connection, id: i64, move_file: &str) {
        let hash = vec![id as u8; 32];

        connection
            .execute(
                r#"
                INSERT INTO games(
                    id,
                    canonical_hash,
                    board_size,
                    move_count,
                    move_file
                )
                VALUES (?1, ?2, 19, 100, ?3)
                "#,
                params![id, hash, move_file],
            )
            .expect("insert test game");
    }

    fn write_test_move_file(root: &Path, relative_path: &str, moves: Vec<Move>) {
        let absolute_path = root.join(relative_path);

        if let Some(parent) = absolute_path.parent() {
            std::fs::create_dir_all(parent).expect("create move-file directory");
        }

        let record = GameRecord {
            board_size: 19,
            setup: Vec::new(),
            moves,
            metadata: Metadata {
                black_player: None,
                white_player: None,
                date: None,
                event: None,
                result: None,
                komi: None,
                handicap: None,
            },
        };

        write_move_file(&absolute_path, &record).expect("write test move file");
    }

    #[test]
    fn lists_games_in_id_order() {
        let (_temporary, root) = create_test_database();
        let connection = database::open(&root).expect("open test database");

        insert_game(&connection, 2, "games/bb/test-two.moves");
        insert_game(&connection, 1, "games/aa/test-one.moves");

        drop(connection);

        let indexer = PositionIndexer::open(&root).expect("open indexer");
        let games = indexer.games_to_index(1).expect("list games");

        assert_eq!(games.len(), 2);
        assert_eq!(games[0].game_id, 1);
        assert_eq!(games[1].game_id, 2);

        assert_eq!(games[0].move_file, root.join("games/aa/test-one.moves"));
    }

    #[test]
    fn excludes_games_already_indexed_at_requested_version() {
        let (_temporary, root) = create_test_database();
        let connection = database::open(&root).expect("open test database");

        insert_game(&connection, 1, "games/aa/test-one.moves");
        insert_game(&connection, 2, "games/bb/test-two.moves");

        connection
            .execute(
                r#"
                INSERT INTO indexed_games(
                    game_id,
                    index_version,
                    occurrence_count
                )
                VALUES (1, 1, 101)
                "#,
                [],
            )
            .expect("mark game indexed");

        drop(connection);

        let indexer = PositionIndexer::open(&root).expect("open indexer");
        let games = indexer.games_to_index(1).expect("list games");

        assert_eq!(
            games,
            vec![GameToIndex {
                game_id: 2,
                move_file: root.join("games/bb/test-two.moves"),
            }]
        );

        assert_eq!(indexer.count_games_to_index(1).unwrap(), 1);
    }

    #[test]
    fn older_index_version_does_not_hide_game() {
        let (_temporary, root) = create_test_database();
        let connection = database::open(&root).expect("open test database");

        insert_game(&connection, 1, "games/aa/test-one.moves");

        connection
            .execute(
                r#"
                INSERT INTO indexed_games(
                    game_id,
                    index_version,
                    occurrence_count
                )
                VALUES (1, 1, 101)
                "#,
                [],
            )
            .expect("mark game indexed");

        drop(connection);

        let indexer = PositionIndexer::open(&root).expect("open indexer");

        assert_eq!(indexer.count_games_to_index(1).unwrap(), 0);
        assert_eq!(indexer.count_games_to_index(2).unwrap(), 1);
    }

    #[test]
    fn replays_one_database_game_into_positions() {
        let (_temporary, root) = create_test_database();
        let connection = database::open(&root).expect("open test database");

        insert_game(&connection, 1, "games/aa/test-one.moves");

        write_test_move_file(
            &root,
            "games/aa/test-one.moves",
            vec![
                Move {
                    color: Color::Black,
                    point: Some(3 * 19 + 3),
                },
                Move {
                    color: Color::White,
                    point: Some(15 * 19 + 15),
                },
            ],
        );

        drop(connection);

        let indexer = PositionIndexer::open(&root).expect("open indexer");
        let stream = indexer.replay_game_by_id(1).expect("replay game");

        assert_eq!(stream.game_id, 1);
        assert_eq!(stream.occurrences.len(), 3);

        assert_eq!(stream.occurrences[0].move_number, 0);
        assert_eq!(stream.occurrences[0].side_to_move, Color::Black);

        assert_eq!(stream.occurrences[1].move_number, 1);
        assert_eq!(stream.occurrences[1].side_to_move, Color::White);

        assert_eq!(stream.occurrences[2].move_number, 2);
        assert_eq!(stream.occurrences[2].side_to_move, Color::Black);
    }

    #[test]
    fn reports_missing_move_file() {
        let (_temporary, root) = create_test_database();
        let connection = database::open(&root).expect("open test database");

        insert_game(&connection, 1, "games/missing.moves");

        drop(connection);

        let indexer = PositionIndexer::open(&root).expect("open indexer");
        let error = indexer
            .replay_game_by_id(1)
            .expect_err("missing move file should fail");

        assert!(error.to_string().contains("does not exist"));
    }

    #[test]
    fn reports_unknown_game_id() {
        let (_temporary, root) = create_test_database();
        let indexer = PositionIndexer::open(&root).expect("open indexer");

        let error = indexer
            .replay_game_by_id(999)
            .expect_err("unknown game should fail");

        assert!(error.to_string().contains("game 999 does not exist"));
    }
}
