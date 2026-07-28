//! Common search result types used throughout MoyoDB.
//!
//! All search operations should return these types so that
//! command-line tools, graphical interfaces, and other clients
//! can consume search results through a consistent API.
use crate::database;
use crate::game_store::{load_game_record, load_position_at, load_positions};
use crate::project::Project;
use crate::{
    Colour, GameRecord, PositionOccurrence, PositionState, position_stream, read_move_file,
};
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
    pub side_to_move: Colour,
    pub ko_point: Option<u16>,
}

/// A single exact-position match together with the preferred game metadata.
///
/// This type is currently used by the exact-position search implementation.
/// It will eventually be adapted into the public `SearchResult` API
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PositionSearchResult {
    pub game_id: i64,
    pub move_number: usize,
    pub side_to_move: Colour,
    pub ko_point: Option<u16>,

    pub black_player: Option<String>,
    pub white_player: Option<String>,
    pub date: Option<String>,
    pub event: Option<String>,
    pub result: Option<String>,
}

/// Provides access to a MoyoDB project's position index.
///
/// `PositionIndexer` is responsible for:
///
/// - replaying games and positions;
/// - building and maintaining the position index;
/// - searching indexed positions.
///
/// It forms the primary access point for position-based operations
/// within the MoyoDB library.
pub struct PositionIndexer {
    database_root: PathBuf,
    connection: Connection,
}

impl PositionIndexer {
    /// Opens the MoyoDB database stored at `database_root`.
    ///
    /// The directory must contain an existing MoyoDB database and its
    /// SQLite metadata file.
    pub fn open(database_root: &Path) -> Result<Self> {
        let connection = database::open(database_root)?;

        Ok(Self {
            database_root: database_root.to_path_buf(),
            connection,
        })
    }

    /// Opens the position index for an existing MoyoDB project.
    ///
    /// This is the preferred constructor when the caller already has a
    /// [`Project`] value.
    pub fn open_project(project: &Project) -> Result<Self> {
        Self::open(&project.database_root())
    }

    /// Replays a game's move file and produces every indexed position.
    ///
    /// The returned stream contains the initial position followed by each
    /// subsequent position reached during the game. This method is primarily
    /// used when building or rebuilding the position index.
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

    /// Builds or rebuilds the position index for a single game.
    ///
    /// Any existing index entries for the game are replaced. The number of
    /// indexed positions written is returned.
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
                    colour_value(occurrence.side_to_move),
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

    /// Builds or rebuilds the position index for a game identified by its ID.
    ///
    /// This is a convenience wrapper around [`Self::index_game`] for callers
    /// that do not already have a [`GameToIndex`] value.
    pub fn index_game_by_id(&mut self, game_id: i64, index_version: i64) -> Result<usize> {
        let game = self
            .game_by_id(game_id)?
            .with_context(|| format!("game {game_id} does not exist"))?;

        self.index_game(&game, index_version)
    }

    /// Reads a game record from the database.
    ///
    /// The returned [`GameRecord`] contains the complete move sequence and
    /// associated metadata required to replay the game.
    pub fn replay_game_by_id(&self, game_id: i64) -> Result<IndexedPositionStream> {
        let game = self
            .game_by_id(game_id)?
            .with_context(|| format!("game {game_id} does not exist"))?;

        self.replay_game(&game)
    }

    /// Reads a game record from the database.
    ///
    /// The returned [`GameRecord`] contains the complete move sequence and
    /// associated metadata required to replay the game.
    pub fn read_game_by_id(&self, game_id: i64) -> Result<GameRecord> {
        load_game_record(&self.connection, &self.database_root, game_id)
    }

    /// Replays a game to a specific move and returns the resulting board state.
    ///
    /// This method is useful for displaying or analysing a position at a
    /// particular point within a game.
    pub fn replay_board_position(&self, game_id: i64, move_number: usize) -> Result<PositionState> {
        load_position_at(&self.connection, &self.database_root, game_id, move_number)
    }

    /// Replays a game and returns every board position reached.
    ///
    /// The returned vector contains the initial position followed by the
    /// position after each move in the game.
    pub fn replay_game_states_by_id(&self, game_id: i64) -> Result<Vec<PositionState>> {
        load_positions(&self.connection, &self.database_root, game_id)
    }

    /// Returns a specific indexed position from a game.
    ///
    /// Positions are identified by their move number within the game.
    pub fn position_from_game(
        &self,
        game_id: i64,
        move_number: usize,
    ) -> Result<PositionOccurrence> {
        Ok(self.replay_board_position(game_id, move_number)?.occurrence)
    }

    /// Finds games containing the same position as a specified game position.
    ///
    /// The search is performed using the indexed position fingerprint.
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
    /// Returns the IDs of every game in the project.
    pub fn game_ids(&self) -> Result<Vec<i64>> {
        let mut statement = self
            .connection
            .prepare(
                r#"
            SELECT id
            FROM games
            ORDER BY id
            "#,
            )
            .context("preparing game-id query")?;

        let rows = statement
            .query_map([], |row| row.get::<_, i64>(0))
            .context("querying game IDs")?;

        let mut ids = Vec::new();

        for row in rows {
            ids.push(row.context("reading game ID")?);
        }

        Ok(ids)
    }

    /// Returns the games that require indexing for the specified index version.
    ///
    /// Games already indexed at the requested version are excluded from the
    /// returned list.
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

    /// Returns the number of games that require indexing.
    ///
    /// This can be used to report indexing progress before processing begins.
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

    /// Finds every occurrence of an exact board position.
    ///
    /// The search uses the position fingerprint and returns all matching
    /// occurrences without loading or replaying every game.
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
                side_to_move: colour_from_value(side_to_move)?,
                ko_point: ko_point
                    .map(u16::try_from)
                    .transpose()
                    .context("invalid ko point in database")?,
            });
        }

        Ok(matches)
    }

    /// Finds every occurrence of an exact board position together with game metadata.
    ///
    /// This is a convenience method that combines exact-position search with
    /// the preferred metadata for each matching game.
    pub fn find_exact_position_with_metadata(
        &self,
        fingerprint_hex: &str,
    ) -> Result<Vec<PositionSearchResult>> {
        let fingerprint = decode_fingerprint_hex(fingerprint_hex)?;

        let mut statement = self.connection.prepare(
            r#"
        SELECT
            ep.game_id,
            ep.move_number,
            ep.side_to_move,
            ep.ko_point,
            gm.black_player,
            gm.white_player,
            gm.played_date,
            gm.event,
            gm.result
        FROM exact_positions ep
        LEFT JOIN game_sources gs
            ON gs.game_id = ep.game_id
        LEFT JOIN game_metadata gm
            ON gm.game_source_id = gs.id
        WHERE ep.position_hash = ?1
        ORDER BY ep.game_id, ep.move_number
        "#,
        )?;

        let rows = statement.query_map([fingerprint.as_slice()], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, i64>(2)?,
                row.get::<_, Option<i64>>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, Option<String>>(5)?,
                row.get::<_, Option<String>>(6)?,
                row.get::<_, Option<String>>(7)?,
                row.get::<_, Option<String>>(8)?,
            ))
        })?;

        let mut results = Vec::new();

        for row in rows {
            let (
                game_id,
                move_number,
                side_to_move,
                ko_point,
                black_player,
                white_player,
                date,
                event,
                result,
            ) = row.context("reading position search result")?;

            results.push(PositionSearchResult {
                game_id,
                move_number: usize::try_from(move_number).context("invalid move number")?,
                side_to_move: colour_from_value(side_to_move)?,
                ko_point: ko_point
                    .map(u16::try_from)
                    .transpose()
                    .context("invalid ko point")?,
                black_player,
                white_player,
                date,
                event,
                result,
            });
        }

        Ok(results)
    }
}

fn colour_value(colour: Colour) -> i64 {
    match colour {
        Colour::Black => 1,
        Colour::White => 2,
    }
}

fn colour_from_value(value: i64) -> Result<Colour> {
    match value {
        1 => Ok(Colour::Black),
        2 => Ok(Colour::White),
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
    use crate::{Colour, GameRecord, Metadata, Move, write_move_file};
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
                    colour: Colour::Black,
                    point: Some(3 * 19 + 3),
                },
                Move {
                    colour: Colour::White,
                    point: Some(15 * 19 + 15),
                },
            ],
        );

        drop(connection);

        let indexer = PositionIndexer::open(&root).expect("open indexer");
        let occurrence = indexer.position_from_game(1, 1).expect("select position");

        assert_eq!(occurrence.move_number, 1);
        assert_eq!(occurrence.side_to_move, Colour::White);
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
                colour: Colour::Black,
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
                    colour: Colour::Black,
                    point: Some(3 * 19 + 3),
                },
                Move {
                    colour: Colour::White,
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
                colour: Colour::Black,
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
                    colour: Colour::Black,
                    point: Some(3 * 19 + 3),
                },
                Move {
                    colour: Colour::White,
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
        assert_eq!(stream.occurrences[0].side_to_move, Colour::Black);

        assert_eq!(stream.occurrences[1].move_number, 1);
        assert_eq!(stream.occurrences[1].side_to_move, Colour::White);

        assert_eq!(stream.occurrences[2].move_number, 2);
        assert_eq!(stream.occurrences[2].side_to_move, Colour::Black);
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
                    colour: Colour::Black,
                    point: Some(3 * 19 + 3),
                },
                Move {
                    colour: Colour::White,
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
        assert_eq!(matches[0].side_to_move, Colour::White);
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
