use crate::database;
use crate::{Color, PositionOccurrence, position_stream, read_move_file};
use anyhow::{Context, Result, bail};
use rusqlite::{Connection, params};
use std::path::{Path, PathBuf};

pub const POSITION_INDEX_VERSION: i64 = 1;

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExactPositionMatch {
    pub game_id: i64,
    pub move_number: usize,
    pub side_to_move: Color,
    pub ko_point: Option<u16>,
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

    pub fn index_game(&mut self, game: &GameToIndex, index_version: i64) -> Result<usize> {
        if index_version <= 0 {
            bail!("index version must be positive");
        }

        let stream = self.replay_game(game)?;
        let occurrence_count = stream.occurrences.len();

        let tx = self.connection.transaction()?;

        tx.execute(
            "DELETE FROM exact_positions WHERE game_id = ?1",
            [game.game_id],
        )?;

        {
            let mut stmt = tx.prepare(
                r#"
            INSERT INTO exact_positions(
                position_hash,
                game_id,
                move_number,
                side_to_move,
                ko_point
            )
            VALUES(?1, ?2, ?3, ?4, ?5)
            "#,
            )?;

            for occurrence in &stream.occurrences {
                stmt.execute(params![
                    occurrence.fingerprint.as_slice(),
                    game.game_id,
                    occurrence.move_number as i64,
                    color_value(occurrence.side_to_move),
                    occurrence.ko_point.map(i64::from),
                ])?;
            }
        }

        tx.execute(
            r#"
        INSERT INTO indexed_games(
            game_id,
            index_version,
            occurrence_count
        )
        VALUES(?1, ?2, ?3)
        ON CONFLICT(game_id) DO UPDATE SET
            index_version = excluded.index_version,
            occurrence_count = excluded.occurrence_count,
            indexed_at = CURRENT_TIMESTAMP
        "#,
            params![game.game_id, index_version, occurrence_count as i64,],
        )?;

        tx.commit()?;

        Ok(occurrence_count)
    }

    pub fn index_game_by_id(&mut self, game_id: i64, index_version: i64) -> Result<usize> {
        let game = self
            .game_by_id(game_id)?
            .with_context(|| format!("game {game_id} does not exist"))?;

        self.index_game(&game, index_version)
    }

    /// Reads and replays a game directly by database ID.
    pub fn replay_game_by_id(&self, game_id: i64) -> Result<IndexedPositionStream> {
        let game = self
            .game_by_id(game_id)?
            .with_context(|| format!("game {game_id} does not exist"))?;

        self.replay_game(&game)
    }

    pub fn position_from_game(
        &self,
        game_id: i64,
        move_number: usize,
    ) -> Result<PositionOccurrence> {
        let stream = self.replay_game_by_id(game_id)?;

        stream
            .occurrences
            .get(move_number)
            .cloned()
            .with_context(|| {
                format!(
                    "requested move {move_number}, but game {game_id} contains only {} moves",
                    stream.occurrences.len().saturating_sub(1)
                )
            })
    }

    pub fn find_matches_from_game(
        &self,
        game_id: i64,
        move_number: usize,
    ) -> Result<Vec<ExactPositionMatch>> {
        let occurrence = self.position_from_game(game_id, move_number)?;

        let fingerprint = occurrence
            .fingerprint
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();

        self.find_exact_position(&fingerprint)
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

    pub fn find_exact_position(&self, fingerprint_hex: &str) -> Result<Vec<ExactPositionMatch>> {
        let fingerprint = decode_fingerprint_hex(fingerprint_hex)?;

        let mut statement = self
            .connection
            .prepare(
                r#"
            SELECT
                game_id,
                move_number,
                side_to_move,
                ko_point
            FROM exact_positions
            WHERE position_hash = ?1
            ORDER BY game_id, move_number
            "#,
            )
            .context("preparing exact-position lookup")?;

        let rows = statement
            .query_map([fingerprint.as_slice()], |row| {
                let move_number: i64 = row.get(1)?;
                let side_to_move: i64 = row.get(2)?;
                let ko_point: Option<i64> = row.get(3)?;

                Ok((row.get::<_, i64>(0)?, move_number, side_to_move, ko_point))
            })
            .context("querying exact-position index")?;

        let mut matches = Vec::new();

        for row in rows {
            let (game_id, move_number, side_to_move, ko_point) =
                row.context("reading exact-position match")?;

            matches.push(ExactPositionMatch {
                game_id,
                move_number: usize::try_from(move_number)
                    .context("negative or oversized move number in database")?,
                side_to_move: color_from_value(side_to_move)?,
                ko_point: ko_point
                    .map(u16::try_from)
                    .transpose()
                    .context("invalid ko point in database")?,
            });
        }

        Ok(matches)
    }
}

fn color_value(color: Color) -> i64 {
    match color {
        Color::Black => 1,
        Color::White => 2,
    }
}

fn color_from_value(value: i64) -> Result<Color> {
    match value {
        1 => Ok(Color::Black),
        2 => Ok(Color::White),
        _ => bail!("invalid colour value: {value}"),
    }
}

fn decode_fingerprint_hex(value: &str) -> Result<[u8; 32]> {
    if value.len() != 64 {
        bail!("position fingerprint must contain exactly 64 hexadecimal characters");
    }

    let mut output = [0u8; 32];
    let bytes = value.as_bytes();

    for index in 0..32 {
        let high = decode_hex_digit(bytes[index * 2])?;
        let low = decode_hex_digit(bytes[index * 2 + 1])?;
        output[index] = (high << 4) | low;
    }

    Ok(output)
}

fn decode_hex_digit(value: u8) -> Result<u8> {
    match value {
        b'0'..=b'9' => Ok(value - b'0'),
        b'a'..=b'f' => Ok(value - b'a' + 10),
        b'A'..=b'F' => Ok(value - b'A' + 10),
        _ => bail!("position fingerprint contains a non-hexadecimal character"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Color, GameRecord, Metadata, Move, write_move_file};
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
    fn selects_position_from_game_by_move_number() {
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
        let occurrence = indexer.position_from_game(1, 1).expect("select position");

        assert_eq!(occurrence.move_number, 1);
        assert_eq!(occurrence.side_to_move, Color::White);
    }

    #[test]
    fn rejects_out_of_range_position_number() {
        let (_temporary, root) = create_test_database();
        let connection = database::open(&root).expect("open test database");

        insert_game(&connection, 1, "games/aa/test-one.moves");

        write_test_move_file(
            &root,
            "games/aa/test-one.moves",
            vec![Move {
                color: Color::Black,
                point: Some(3 * 19 + 3),
            }],
        );

        drop(connection);

        let indexer = PositionIndexer::open(&root).expect("open indexer");
        let error = indexer
            .position_from_game(1, 2)
            .expect_err("move number should be out of range");

        assert!(error.to_string().contains("contains only 1 moves"));
    }
    #[test]
    fn indexes_one_game_transactionally() {
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

        let mut indexer = PositionIndexer::open(&root).expect("open indexer");

        let count = indexer
            .index_game_by_id(1, POSITION_INDEX_VERSION)
            .expect("index game");

        assert_eq!(count, 3);

        let connection = database::open(&root).expect("reopen database");

        let stored_positions: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM exact_positions WHERE game_id = 1",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(stored_positions, 3);

        let indexed: (i64, i64) = connection
            .query_row(
                r#"
            SELECT index_version, occurrence_count
            FROM indexed_games
            WHERE game_id = 1
            "#,
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();

        assert_eq!(indexed, (POSITION_INDEX_VERSION, 3));
    }

    #[test]
    fn reindexing_replaces_existing_position_rows() {
        let (_temporary, root) = create_test_database();
        let connection = database::open(&root).expect("open test database");

        insert_game(&connection, 1, "games/aa/test-one.moves");

        write_test_move_file(
            &root,
            "games/aa/test-one.moves",
            vec![Move {
                color: Color::Black,
                point: Some(3 * 19 + 3),
            }],
        );

        drop(connection);

        let mut indexer = PositionIndexer::open(&root).expect("open indexer");

        assert_eq!(indexer.index_game_by_id(1, 1).unwrap(), 2);
        assert_eq!(indexer.index_game_by_id(1, 2).unwrap(), 2);

        let connection = database::open(&root).expect("reopen database");

        let position_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM exact_positions WHERE game_id = 1",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(position_count, 2);

        let stored_version: i64 = connection
            .query_row(
                "SELECT index_version FROM indexed_games WHERE game_id = 1",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(stored_version, 2);
    }

    #[test]
    fn rejects_non_positive_index_version() {
        let (_temporary, root) = create_test_database();
        let connection = database::open(&root).expect("open test database");

        insert_game(&connection, 1, "games/aa/test-one.moves");
        write_test_move_file(&root, "games/aa/test-one.moves", Vec::new());

        drop(connection);

        let mut indexer = PositionIndexer::open(&root).expect("open indexer");

        let error = indexer
            .index_game_by_id(1, 0)
            .expect_err("zero index version should fail");

        assert!(error.to_string().contains("must be positive"));
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
    fn finds_matches_from_game_position() {
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

        let mut indexer = PositionIndexer::open(&root).expect("open indexer");

        indexer
            .index_game_by_id(1, POSITION_INDEX_VERSION)
            .expect("index game");

        let matches = indexer.find_matches_from_game(1, 1).expect("find matches");

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].game_id, 1);
        assert_eq!(matches[0].move_number, 1);
        assert_eq!(matches[0].side_to_move, Color::White);
    }

    #[test]
    fn find_matches_from_unknown_game_reports_error() {
        let (_temporary, root) = create_test_database();
        let indexer = PositionIndexer::open(&root).expect("open indexer");

        let error = indexer
            .find_matches_from_game(999, 0)
            .expect_err("unknown game should fail");

        assert!(error.to_string().contains("game 999 does not exist"));
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
