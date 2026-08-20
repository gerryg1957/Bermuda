//! Common search result types used throughout Bermuda.
//!
//! All search operations should return these types so that
//! command-line tools, graphical interfaces, and other clients
//! can consume search results through a consistent API.
use crate::database;
use crate::game_store::{load_game_record, load_position_at, load_positions};
use crate::project::Project;
use crate::{
    Colour, GameRecord, PatternTransformation, PositionOccurrence, PositionState, position_stream,
    read_move_file, transformed_position_fingerprint,
};
use anyhow::{Context, Result, bail};
use rusqlite::{Connection, params};
use std::collections::HashSet;
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SymmetricExactPositionMatch {
    pub game_id: i64,
    pub move_number: usize,
    pub side_to_move: Colour,
    pub ko_point: Option<u16>,
    pub transformation: PatternTransformation,
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

/// Provides access to a Bermuda project's position index.
///
/// `PositionIndexer` is responsible for:
///
/// - replaying games and positions;
/// - building and maintaining the position index;
/// - searching indexed positions.
///
/// It forms the primary access point for position-based operations
/// within the Bermuda library.
pub struct PositionIndexer {
    database_root: PathBuf,
    connection: Connection,
}

pub(crate) fn replay_game_for_index(game: &GameToIndex) -> Result<IndexedPositionStream> {
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

impl PositionIndexer {
    /// Opens the Bermuda database stored at `database_root`.
    ///
    /// The directory must contain an existing Bermuda database and its
    /// SQLite metadata file.
    pub fn open(database_root: &Path) -> Result<Self> {
        let connection = database::open(database_root)?;

        Ok(Self {
            database_root: database_root.to_path_buf(),
            connection,
        })
    }

    /// Opens the position index for an existing Bermuda project.
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
        replay_game_for_index(game)
    }

    /// Builds or rebuilds the position index for a single game.
    ///
    /// Any existing index entries for the game are replaced. The number of
    /// indexed positions written is returned.
    pub fn index_game(&mut self, game: &GameToIndex, index_version: i64) -> Result<usize> {
        let stream = self.replay_game(game)?;
        self.index_stream(&stream, index_version)
    }

    pub(crate) fn index_stream(
        &mut self,
        stream: &IndexedPositionStream,
        index_version: i64,
    ) -> Result<usize> {
        self.index_stream_inner(stream, index_version, true)
    }

    /// Writes a position stream during a fresh bulk index build.
    ///
    /// Games supplied by the bulk builder have no committed index rows, so
    /// there is nothing to delete. Avoiding the DELETE is essential while
    /// the game-id secondary index is deliberately absent.
    pub(crate) fn index_stream_bulk(
        &mut self,
        stream: &IndexedPositionStream,
        index_version: i64,
    ) -> Result<usize> {
        self.index_stream_inner(stream, index_version, false)
    }

    fn index_stream_inner(
        &mut self,
        stream: &IndexedPositionStream,
        index_version: i64,
        remove_existing: bool,
    ) -> Result<usize> {
        if index_version <= 0 {
            bail!("index version must be positive");
        }

        let occurrence_count = stream.occurrences.len();

        let tx = self.connection.transaction()?;

        if remove_existing {
            tx.execute(
                "DELETE FROM exact_positions WHERE game_id = ?1",
                [stream.game_id],
            )?;
        }

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
                    stream.game_id,
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
            params![stream.game_id, index_version, occurrence_count as i64,],
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

    /// Replays a database game and returns every indexed position.
    ///
    /// The returned stream contains the initial position followed by the
    /// position after each move in the game.
    pub fn replay_game_by_id(&self, game_id: i64) -> Result<IndexedPositionStream> {
        let record = self.read_game_by_id(game_id)?;

        let occurrences = position_stream(&record)
            .with_context(|| format!("building position stream for game {game_id}"))?;

        Ok(IndexedPositionStream {
            game_id,
            occurrences,
        })
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

    /// Finds exact occurrences of a game position under all eight board symmetries.
    ///
    /// The stored position index remains orientation-specific.  The query
    /// position is transformed instead, producing at most eight indexed
    /// fingerprint lookups.  Matches are deduplicated by game and move number.
    pub fn find_symmetric_matches_from_game(
        &self,
        game_id: i64,
        move_number: usize,
    ) -> Result<Vec<SymmetricExactPositionMatch>> {
        let state = self.replay_board_position(game_id, move_number)?;

        let transformations = [
            PatternTransformation::Identity,
            PatternTransformation::Rotate90Clockwise,
            PatternTransformation::Rotate180,
            PatternTransformation::Rotate270Clockwise,
            PatternTransformation::MirrorLeftRight,
            PatternTransformation::MirrorTopBottom,
            PatternTransformation::MirrorMainDiagonal,
            PatternTransformation::MirrorAntiDiagonal,
        ];

        let mut seen_fingerprints = HashSet::new();
        let mut seen_occurrences = HashSet::new();
        let mut matches = Vec::new();

        for transformation in transformations {
            let fingerprint = transformed_position_fingerprint(
                &state.board,
                state.occurrence.side_to_move,
                transformation,
            );

            if !seen_fingerprints.insert(fingerprint) {
                continue;
            }

            let fingerprint_hex = fingerprint
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>();

            for position_match in self.find_exact_position(&fingerprint_hex)? {
                if !seen_occurrences.insert((position_match.game_id, position_match.move_number)) {
                    continue;
                }

                matches.push(SymmetricExactPositionMatch {
                    game_id: position_match.game_id,
                    move_number: position_match.move_number,
                    side_to_move: position_match.side_to_move,
                    ko_point: position_match.ko_point,
                    transformation,
                });
            }
        }

        matches.sort_by_key(|position_match| (position_match.game_id, position_match.move_number));

        Ok(matches)
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

    /// Returns every stored game with its move-file path, ordered by game ID.
    ///
    /// This performs one database query so callers that process the complete
    /// corpus do not need a separate SQLite lookup for every game.
    pub fn games(&self) -> Result<Vec<GameToIndex>> {
        let mut statement = self
            .connection
            .prepare(
                r#"
            SELECT id, move_file
            FROM games
            ORDER BY id
            "#,
            )
            .context("preparing game query")?;

        let rows = statement
            .query_map([], |row| {
                let game_id: i64 = row.get(0)?;
                let relative_move_file: String = row.get(1)?;
                Ok((game_id, relative_move_file))
            })
            .context("querying games")?;

        let mut games = Vec::new();

        for row in rows {
            let (game_id, relative_move_file) = row.context("reading game")?;

            games.push(GameToIndex {
                game_id,
                move_file: self.database_root.join(relative_move_file),
            });
        }

        Ok(games)
    }

    /// Returns the games eligible for a project-wide pattern search.
    ///
    /// By default handicap games are excluded because ordinary professional
    /// pattern research is intended to compare even games.  The games remain
    /// stored and indexed, and callers may explicitly include them.
    ///
    /// A canonical game is considered a handicap game if any of its source
    /// metadata records reports a positive handicap.
    pub fn games_for_pattern_search(
        &self,
        include_handicap_games: bool,
    ) -> Result<Vec<GameToIndex>> {
        if include_handicap_games {
            return self.games();
        }

        let mut statement = self
            .connection
            .prepare(
                r#"
                SELECT
                    g.id,
                    g.move_file
                FROM games AS g
                WHERE NOT EXISTS (
                    SELECT 1
                    FROM game_sources AS gs
                    JOIN game_metadata AS gm
                        ON gm.game_source_id = gs.id
                    WHERE gs.game_id = g.id
                      AND COALESCE(gm.handicap, 0) > 0
                )
                ORDER BY g.id
                "#,
            )
            .context("preparing pattern-search game query")?;

        let rows = statement
            .query_map([], |row| {
                let game_id: i64 = row.get(0)?;
                let relative_move_file: String = row.get(1)?;
                Ok((game_id, relative_move_file))
            })
            .context("querying pattern-search games")?;

        let mut games = Vec::new();

        for row in rows {
            let (game_id, relative_move_file) = row.context("reading pattern-search game")?;

            games.push(GameToIndex {
                game_id,
                move_file: self.database_root.join(relative_move_file),
            });
        }

        Ok(games)
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

    /// Returns true when no position-index work has ever been committed.
    pub(crate) fn position_index_is_empty(&self) -> Result<bool> {
        let empty: i64 = self
            .connection
            .query_row(
                r#"
                SELECT
                    NOT EXISTS(SELECT 1 FROM exact_positions LIMIT 1)
                    AND
                    NOT EXISTS(SELECT 1 FROM indexed_games LIMIT 1)
                "#,
                [],
                |row| row.get(0),
            )
            .context("checking whether the position index is empty")?;

        Ok(empty != 0)
    }

    /// Returns true when a bulk position-index build is in progress.
    ///
    /// The absence of either secondary index is used as durable state, so
    /// an interrupted bulk build can be resumed safely by a later process.
    pub(crate) fn position_index_bulk_mode_active(&self) -> Result<bool> {
        let count: i64 = self
            .connection
            .query_row(
                r#"
                SELECT COUNT(*)
                FROM sqlite_master
                WHERE type = 'index'
                  AND name IN (
                      'exact_positions_hash',
                      'exact_positions_game'
                  )
                "#,
                [],
                |row| row.get(0),
            )
            .context("checking position-index secondary indexes")?;

        Ok(count != 2)
    }

    /// Starts a fresh bulk position-index build.
    ///
    /// Secondary indexes are rebuilt once, after all position rows have
    /// been loaded, rather than being maintained for every INSERT.
    pub(crate) fn begin_bulk_position_index_build(&self) -> Result<()> {
        self.connection
            .execute_batch(
                r#"
                DROP INDEX IF EXISTS exact_positions_hash;
                DROP INDEX IF EXISTS exact_positions_game;
                "#,
            )
            .context("dropping secondary indexes for bulk position indexing")
    }

    /// Finishes a bulk build by restoring the exact-position search indexes.
    pub(crate) fn finish_bulk_position_index_build(&self) -> Result<()> {
        self.connection
            .execute_batch(
                r#"
                CREATE INDEX IF NOT EXISTS exact_positions_hash
                    ON exact_positions(position_hash);

                CREATE INDEX IF NOT EXISTS exact_positions_game
                    ON exact_positions(game_id);
                "#,
            )
            .context("rebuilding secondary indexes after bulk position indexing")
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
    fn pattern_search_games_exclude_handicap_unless_requested() {
        let (_temporary, root) = create_test_database();
        let connection = database::open(&root).expect("open test database");

        insert_game(&connection, 1, "games/aa/even.moves");
        insert_game(&connection, 2, "games/bb/handicap.moves");

        connection
            .execute_batch(
                r#"
                PRAGMA foreign_keys = OFF;

                INSERT INTO game_sources(
                    id,
                    game_id,
                    source_id,
                    original_path,
                    imported_at
                )
                VALUES (
                    100,
                    2,
                    0,
                    'handicap-test.sgf',
                    'test'
                );

                INSERT INTO game_metadata(
                    game_source_id,
                    handicap
                )
                VALUES (
                    100,
                    2
                );

                PRAGMA foreign_keys = ON;
                "#,
            )
            .expect("add handicap metadata");

        drop(connection);

        let indexer = PositionIndexer::open(&root).expect("open indexer");

        let even_games = indexer
            .games_for_pattern_search(false)
            .expect("list even pattern-search games");

        assert_eq!(
            even_games
                .iter()
                .map(|game| game.game_id)
                .collect::<Vec<_>>(),
            vec![1]
        );

        let all_games = indexer
            .games_for_pattern_search(true)
            .expect("list all pattern-search games");

        assert_eq!(
            all_games
                .iter()
                .map(|game| game.game_id)
                .collect::<Vec<_>>(),
            vec![1, 2]
        );
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

        assert!(error.to_string().contains("reading move file for game 1"));
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
