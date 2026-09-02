use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use rusqlite::{Connection, OptionalExtension};

use crate::{
    database,
    game::GameRecord,
    move_file::read_move_file,
    project::Project,
    replay::{PositionState, replay_positions},
};

pub struct GameStore {
    connection: Connection,
    database_root: PathBuf,
}

impl GameStore {
    pub fn open(database_root: &Path) -> Result<Self> {
        let connection = database::open(database_root)?;

        Ok(Self {
            connection,
            database_root: database_root.to_path_buf(),
        })
    }

    pub fn open_project(project: &Project) -> Result<Self> {
        Self::open(&project.database_root())
    }

    pub fn load(&self, game_id: i64) -> Result<GameRecord> {
        load_game_record(&self.connection, &self.database_root, game_id)
    }

    pub fn positions(&self, game_id: i64) -> Result<Vec<PositionState>> {
        load_positions(&self.connection, &self.database_root, game_id)
    }

    pub fn position_at(&self, game_id: i64, move_number: usize) -> Result<PositionState> {
        load_position_at(&self.connection, &self.database_root, game_id, move_number)
    }

    /// Removes a game and all of its managed catalogue/index records.
    ///
    /// Source metadata is removed explicitly before the game because
    /// `game_sources` does not cascade from `games`. Metadata beneath
    /// `game_sources`, and position-index rows beneath `games`, do cascade.
    ///
    /// The managed move file is deleted only after the database transaction
    /// commits. This ensures that a database failure cannot leave a live game
    /// record referring to a move file that this operation has already
    /// removed.

    /// Removes one source occurrence of a canonical game.
    ///
    /// Personal game history is occurrence-based: several separately played
    /// games may legitimately share the same canonical move sequence.
    ///
    /// Removing one occurrence therefore deletes only its `game_sources` row
    /// (and its cascading metadata) while another source still refers to the
    /// canonical game.  If this was the final source, the canonical game is
    /// also deleted; its position-index rows then cascade, and its managed
    /// move file is removed after the database transaction commits.
    pub fn remove_source(&mut self, game_source_id: i64) -> Result<()> {
        let transaction = self
            .connection
            .transaction()
            .context("starting game-source removal transaction")?;

        let source_game: Option<(i64, String)> = transaction
            .query_row(
                r#"
                SELECT
                    game_sources.game_id,
                    games.move_file
                FROM game_sources
                JOIN games
                    ON games.id = game_sources.game_id
                WHERE game_sources.id = ?1
                "#,
                [game_source_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()
            .context("reading game source before removal")?;

        let (game_id, relative_move_file) =
            source_game.with_context(|| format!("game source {game_source_id} does not exist"))?;

        let removed = transaction
            .execute("DELETE FROM game_sources WHERE id = ?1", [game_source_id])
            .with_context(|| format!("removing game source {game_source_id}"))?;

        if removed != 1 {
            anyhow::bail!("game source {game_source_id} disappeared during removal");
        }

        let remaining_sources: i64 = transaction
            .query_row(
                "SELECT COUNT(*) FROM game_sources WHERE game_id = ?1",
                [game_id],
                |row| row.get(0),
            )
            .with_context(|| format!("counting remaining sources for game {game_id}"))?;

        if remaining_sources > 0 {
            transaction
                .commit()
                .with_context(|| format!("committing removal of game source {game_source_id}"))?;

            return Ok(());
        }

        let removed_game = transaction
            .execute("DELETE FROM games WHERE id = ?1", [game_id])
            .with_context(|| format!("removing unreferenced game {game_id}"))?;

        if removed_game != 1 {
            anyhow::bail!("game {game_id} disappeared during final source removal");
        }

        transaction
            .commit()
            .with_context(|| format!("committing removal of final source {game_source_id}"))?;

        let move_file = self.database_root.join(relative_move_file);

        match fs::remove_file(&move_file) {
            Ok(()) => {}

            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}

            Err(error) => {
                return Err(error).with_context(|| {
                    format!(
                        "game source {game_source_id} and game {game_id} were removed, \
                         but managed move file {} could not be deleted",
                        move_file.display()
                    )
                });
            }
        }

        Ok(())
    }

    pub fn remove(&mut self, game_id: i64) -> Result<()> {
        let relative_move_file: Option<String> = self
            .connection
            .query_row(
                "SELECT move_file FROM games WHERE id = ?1",
                [game_id],
                |row| row.get(0),
            )
            .optional()
            .context("reading game before removal")?;

        let relative_move_file =
            relative_move_file.with_context(|| format!("game {game_id} does not exist"))?;

        let move_file = self.database_root.join(relative_move_file);

        let transaction = self
            .connection
            .transaction()
            .context("starting game removal transaction")?;

        transaction
            .execute("DELETE FROM game_sources WHERE game_id = ?1", [game_id])
            .with_context(|| format!("removing source records for game {game_id}"))?;

        let removed = transaction
            .execute("DELETE FROM games WHERE id = ?1", [game_id])
            .with_context(|| format!("removing game {game_id}"))?;

        if removed != 1 {
            anyhow::bail!("game {game_id} disappeared during removal");
        }

        transaction
            .commit()
            .with_context(|| format!("committing removal of game {game_id}"))?;

        match fs::remove_file(&move_file) {
            Ok(()) => {}

            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}

            Err(error) => {
                return Err(error).with_context(|| {
                    format!(
                        "game {game_id} was removed, but its managed move file {} could not be deleted",
                        move_file.display()
                    )
                });
            }
        }

        Ok(())
    }
}

pub(crate) fn load_game_record(
    connection: &Connection,
    database_root: &Path,
    game_id: i64,
) -> Result<GameRecord> {
    let relative_move_file: Option<String> = connection
        .query_row(
            "SELECT move_file FROM games WHERE id = ?1",
            [game_id],
            |row| row.get(0),
        )
        .optional()
        .context("reading game from database")?;

    let relative_move_file =
        relative_move_file.with_context(|| format!("game {game_id} does not exist"))?;

    let move_file = database_root.join(relative_move_file);

    read_move_file(&move_file).with_context(|| format!("reading move file for game {game_id}"))
}

pub(crate) fn load_positions(
    connection: &Connection,
    database_root: &Path,
    game_id: i64,
) -> Result<Vec<PositionState>> {
    let record = load_game_record(connection, database_root, game_id)?;

    replay_positions(&record).with_context(|| format!("replaying game {game_id}"))
}

pub(crate) fn load_position_at(
    connection: &Connection,
    database_root: &Path,
    game_id: i64,
    move_number: usize,
) -> Result<PositionState> {
    let states = load_positions(connection, database_root, game_id)?;
    states.get(move_number).cloned().with_context(|| {
        format!(
            "requested move {move_number}, but game {game_id} contains only {} moves",
            states.len().saturating_sub(1)
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::{
        board::{Colour, Move},
        game::{GameRecord, Metadata, SetupStone},
        move_file::write_move_file,
        project::Project,
        project_manager::ProjectManager,
    };
    use rusqlite::params;
    use tempfile::TempDir;

    fn create_test_project() -> (TempDir, Project) {
        let temporary = TempDir::new().expect("create temporary directory");
        let project_root = temporary.path().join("test-project");

        let manager = ProjectManager::new();
        let project = manager
            .create("Test Project", &project_root)
            .expect("create test project");

        (temporary, project)
    }

    fn insert_game(connection: &Connection, id: i64, move_file: &str) {
        let hash = vec![id as u8; 32];

        connection
            .execute(
                r#"
                INSERT INTO games (
                    id,
                    canonical_hash,
                    board_size,
                    move_count,
                    move_file
                )
                VALUES (?1, ?2, 19, 0, ?3)
                "#,
                params![id, hash, move_file],
            )
            .expect("insert test game");
    }

    fn test_record() -> GameRecord {
        GameRecord {
            board_size: 19,
            metadata: Metadata {
                black_player: Some("Alpha".to_owned()),
                white_player: Some("Beta".to_owned()),
                date: Some("2026-07-27".to_owned()),
                event: Some("Test Event".to_owned()),
                result: Some("B+R".to_owned()),
                komi: Some(6.5),
                handicap: None,
            },
            setup: Vec::new(),
            moves: Vec::new(),
        }
    }

    fn write_test_move_file(database_root: &Path, relative_path: &str, record: &GameRecord) {
        let absolute_path = database_root.join(relative_path);

        if let Some(parent) = absolute_path.parent() {
            std::fs::create_dir_all(parent).expect("create move-file directory");
        }

        write_move_file(&absolute_path, record).expect("write test move file");
    }

    fn insert_test_record(
        project: &Project,
        game_id: i64,
        relative_path: &str,
        record: &GameRecord,
    ) -> Result<()> {
        write_test_move_file(&project.database_root(), relative_path, record);

        let connection = database::open(&project.database_root())?;
        insert_game(&connection, game_id, relative_path);

        Ok(())
    }

    #[test]
    fn loads_game_by_id() -> Result<()> {
        let (_temporary, project) = create_test_project();
        let relative_path = "games/aa/test.moves";
        let expected = test_record();

        write_test_move_file(&project.database_root(), relative_path, &expected);

        let connection = database::open(&project.database_root())?;
        insert_game(&connection, 1, relative_path);
        drop(connection);

        let store = project.game_store()?;
        let actual = store.load(1)?;

        assert_eq!(actual, expected);

        Ok(())
    }

    #[test]
    fn reports_unknown_game_id() -> Result<()> {
        let (_temporary, project) = create_test_project();
        let store = project.game_store()?;

        let error = store.load(999).expect_err("unknown game should fail");

        assert!(error.to_string().contains("game 999 does not exist"));

        Ok(())
    }

    #[test]
    fn reports_missing_move_file() -> Result<()> {
        let (_temporary, project) = create_test_project();

        let connection = database::open(&project.database_root())?;
        insert_game(&connection, 1, "games/missing.moves");
        drop(connection);

        let store = project.game_store()?;

        let error = store.load(1).expect_err("missing move file should fail");

        assert!(error.to_string().contains("reading move file for game 1"));

        Ok(())
    }

    #[test]
    fn returns_initial_position_after_setup() -> Result<()> {
        let (_temporary, project) = create_test_project();

        let mut record = test_record();
        record.setup = vec![SetupStone::Add {
            colour: Colour::Black,
            point: 60,
        }];
        record.moves = vec![Move {
            colour: Colour::White,
            point: Some(61),
        }];

        insert_test_record(&project, 1, "games/aa/initial.moves", &record)?;

        let store = project.game_store()?;
        let position = store.position_at(1, 0)?;

        assert_eq!(position.occurrence.move_number, 0);
        assert_eq!(position.occurrence.side_to_move, Colour::White);
        assert_eq!(position.board.colour_at(60), Some(Colour::Black));
        assert_eq!(position.board.colour_at(61), None);
        assert_eq!(position.last_move, None);

        Ok(())
    }

    #[test]
    fn returns_position_after_move() -> Result<()> {
        let (_temporary, project) = create_test_project();

        let mut record = test_record();
        record.moves = vec![
            Move {
                colour: Colour::Black,
                point: Some(60),
            },
            Move {
                colour: Colour::White,
                point: Some(61),
            },
        ];

        insert_test_record(&project, 1, "games/aa/moves.moves", &record)?;

        let store = project.game_store()?;
        let position = store.position_at(1, 1)?;

        assert_eq!(position.occurrence.move_number, 1);
        assert_eq!(position.occurrence.side_to_move, Colour::White);
        assert_eq!(position.board.colour_at(60), Some(Colour::Black));
        assert_eq!(position.board.colour_at(61), None);
        assert_eq!(position.last_move, Some(record.moves[0]));

        Ok(())
    }
    #[test]
    fn returns_position_after_pass() -> Result<()> {
        let (_temporary, project) = create_test_project();

        let mut record = test_record();
        record.moves = vec![
            Move {
                colour: Colour::Black,
                point: Some(60),
            },
            Move {
                colour: Colour::White,
                point: None,
            },
            Move {
                colour: Colour::Black,
                point: Some(61),
            },
        ];

        insert_test_record(&project, 1, "games/aa/pass.moves", &record)?;

        let store = project.game_store()?;
        let position = store.position_at(1, 2)?;

        assert_eq!(position.occurrence.move_number, 2);
        assert_eq!(position.occurrence.side_to_move, Colour::Black);
        assert_eq!(position.board.colour_at(60), Some(Colour::Black));
        assert_eq!(position.board.colour_at(61), None);
        assert_eq!(position.last_move, Some(record.moves[1]));

        Ok(())
    }

    #[test]
    fn reports_out_of_range_move_number() -> Result<()> {
        let (_temporary, project) = create_test_project();

        let mut record = test_record();
        record.moves = vec![Move {
            colour: Colour::Black,
            point: Some(60),
        }];

        insert_test_record(&project, 1, "games/aa/short.moves", &record)?;

        let store = project.game_store()?;

        let error = store
            .position_at(1, 2)
            .expect_err("out-of-range move should fail");

        assert!(error.to_string().contains("requested move 2"));
        assert!(error.to_string().contains("contains only 1 moves"));

        Ok(())
    }

    #[test]
    fn returns_all_game_positions() -> Result<()> {
        let (_temporary, project) = create_test_project();

        let mut record = test_record();
        record.moves = vec![
            Move {
                colour: Colour::Black,
                point: Some(60),
            },
            Move {
                colour: Colour::White,
                point: None,
            },
            Move {
                colour: Colour::Black,
                point: Some(61),
            },
        ];

        insert_test_record(&project, 1, "games/aa/positions.moves", &record)?;

        let store = project.game_store()?;
        let positions = store.positions(1)?;

        assert_eq!(positions.len(), 4);

        assert_eq!(positions[0].occurrence.move_number, 0);
        assert_eq!(positions[0].last_move, None);

        assert_eq!(positions[1].occurrence.move_number, 1);
        assert_eq!(positions[1].last_move, Some(record.moves[0]));

        assert_eq!(positions[2].occurrence.move_number, 2);
        assert_eq!(positions[2].last_move, Some(record.moves[1]));

        assert_eq!(positions[3].occurrence.move_number, 3);
        assert_eq!(positions[3].last_move, Some(record.moves[2]));

        assert_eq!(positions[3].board.colour_at(60), Some(Colour::Black));
        assert_eq!(positions[3].board.colour_at(61), Some(Colour::Black));

        Ok(())
    }

    #[test]
    fn removes_game_database_records_and_move_file() -> Result<()> {
        let (_temporary, project) = create_test_project();
        let relative_path = "games/aa/remove-me.moves";
        let record = test_record();

        insert_test_record(&project, 1, relative_path, &record)?;

        let connection = database::open(&project.database_root())?;

        connection.execute(
            "INSERT INTO sources(name, version) VALUES ('Bermuda', 'play-v1')",
            [],
        )?;
        let source_id = connection.last_insert_rowid();

        connection.execute(
            r#"
            INSERT INTO game_sources(game_id, source_id, original_path)
            VALUES (?1, ?2, ?3)
            "#,
            params![1_i64, source_id, "played:test"],
        )?;
        let game_source_id = connection.last_insert_rowid();

        connection.execute(
            "INSERT INTO game_metadata(game_source_id) VALUES (?1)",
            [game_source_id],
        )?;

        connection.execute(
            r#"
            INSERT INTO indexed_games(
                game_id,
                index_version,
                occurrence_count
            )
            VALUES (1, 1, 1)
            "#,
            [],
        )?;

        connection.execute(
            r#"
            INSERT INTO exact_positions(
                position_hash,
                game_id,
                move_number,
                side_to_move,
                ko_point
            )
            VALUES (?1, 1, 0, 0, NULL)
            "#,
            params![vec![0_u8; 32]],
        )?;

        drop(connection);

        let move_file = project.database_root().join(relative_path);
        assert!(move_file.exists());

        let mut store = project.game_store()?;
        store.remove(1)?;

        assert!(!move_file.exists());
        assert!(store.load(1).is_err());

        drop(store);

        let connection = database::open(&project.database_root())?;

        let game_count: i64 =
            connection.query_row("SELECT COUNT(*) FROM games WHERE id = 1", [], |row| {
                row.get(0)
            })?;

        let source_link_count: i64 = connection.query_row(
            "SELECT COUNT(*) FROM game_sources WHERE game_id = 1",
            [],
            |row| row.get(0),
        )?;

        let metadata_count: i64 = connection.query_row(
            "SELECT COUNT(*) FROM game_metadata WHERE game_source_id = ?1",
            [game_source_id],
            |row| row.get(0),
        )?;

        let indexed_game_count: i64 = connection.query_row(
            "SELECT COUNT(*) FROM indexed_games WHERE game_id = 1",
            [],
            |row| row.get(0),
        )?;

        let position_count: i64 = connection.query_row(
            "SELECT COUNT(*) FROM exact_positions WHERE game_id = 1",
            [],
            |row| row.get(0),
        )?;

        let source_count: i64 = connection.query_row(
            "SELECT COUNT(*) FROM sources WHERE id = ?1",
            [source_id],
            |row| row.get(0),
        )?;

        assert_eq!(game_count, 0);
        assert_eq!(source_link_count, 0);
        assert_eq!(metadata_count, 0);
        assert_eq!(indexed_game_count, 0);
        assert_eq!(position_count, 0);

        /*
         * The reusable Bermuda source itself is deliberately retained.
         * Other personal games may refer to the same source record.
         */
        assert_eq!(source_count, 1);

        Ok(())
    }

    #[test]
    fn reports_unknown_game_when_removing() -> Result<()> {
        let (_temporary, project) = create_test_project();
        let mut store = project.game_store()?;

        let error = store
            .remove(999)
            .expect_err("removing an unknown game should fail");

        assert!(error.to_string().contains("game 999 does not exist"));

        Ok(())
    }

    #[test]
    fn removing_one_source_preserves_shared_canonical_game() -> Result<()> {
        let (_temporary, project) = create_test_project();
        let relative_path = "games/aa/shared.moves";
        let record = test_record();

        insert_test_record(&project, 1, relative_path, &record)?;

        let connection = database::open(&project.database_root())?;

        connection.execute(
            "INSERT INTO sources(name, version) VALUES ('Bermuda', 'play-v1')",
            [],
        )?;
        let source_id = connection.last_insert_rowid();

        connection.execute(
            r#"
            INSERT INTO game_sources(game_id, source_id, original_path)
            VALUES (1, ?1, 'played:first')
            "#,
            [source_id],
        )?;
        let first_source = connection.last_insert_rowid();

        connection.execute(
            r#"
            INSERT INTO game_sources(game_id, source_id, original_path)
            VALUES (1, ?1, 'played:second')
            "#,
            [source_id],
        )?;
        let second_source = connection.last_insert_rowid();

        connection.execute(
            "INSERT INTO game_metadata(game_source_id) VALUES (?1)",
            [first_source],
        )?;

        connection.execute(
            "INSERT INTO game_metadata(game_source_id) VALUES (?1)",
            [second_source],
        )?;

        connection.execute(
            r#"
            INSERT INTO indexed_games(
                game_id,
                index_version,
                occurrence_count
            )
            VALUES (1, 1, 1)
            "#,
            [],
        )?;

        connection.execute(
            r#"
            INSERT INTO exact_positions(
                position_hash,
                game_id,
                move_number,
                side_to_move,
                ko_point
            )
            VALUES (?1, 1, 0, 0, NULL)
            "#,
            params![vec![0_u8; 32]],
        )?;

        drop(connection);

        let move_file = project.database_root().join(relative_path);
        assert!(move_file.exists());

        let mut store = project.game_store()?;
        store.remove_source(first_source)?;
        drop(store);

        let connection = database::open(&project.database_root())?;

        let first_source_count: i64 = connection.query_row(
            "SELECT COUNT(*) FROM game_sources WHERE id = ?1",
            [first_source],
            |row| row.get(0),
        )?;

        let second_source_count: i64 = connection.query_row(
            "SELECT COUNT(*) FROM game_sources WHERE id = ?1",
            [second_source],
            |row| row.get(0),
        )?;

        let first_metadata_count: i64 = connection.query_row(
            "SELECT COUNT(*) FROM game_metadata WHERE game_source_id = ?1",
            [first_source],
            |row| row.get(0),
        )?;

        let second_metadata_count: i64 = connection.query_row(
            "SELECT COUNT(*) FROM game_metadata WHERE game_source_id = ?1",
            [second_source],
            |row| row.get(0),
        )?;

        let game_count: i64 =
            connection.query_row("SELECT COUNT(*) FROM games WHERE id = 1", [], |row| {
                row.get(0)
            })?;

        let indexed_count: i64 = connection.query_row(
            "SELECT COUNT(*) FROM indexed_games WHERE game_id = 1",
            [],
            |row| row.get(0),
        )?;

        let position_count: i64 = connection.query_row(
            "SELECT COUNT(*) FROM exact_positions WHERE game_id = 1",
            [],
            |row| row.get(0),
        )?;

        assert_eq!(first_source_count, 0);
        assert_eq!(second_source_count, 1);
        assert_eq!(first_metadata_count, 0);
        assert_eq!(second_metadata_count, 1);
        assert_eq!(game_count, 1);
        assert_eq!(indexed_count, 1);
        assert_eq!(position_count, 1);
        assert!(move_file.exists());

        Ok(())
    }

    #[test]
    fn removing_final_source_removes_canonical_game_and_move_file() -> Result<()> {
        let (_temporary, project) = create_test_project();
        let relative_path = "games/aa/final-source.moves";
        let record = test_record();

        insert_test_record(&project, 1, relative_path, &record)?;

        let connection = database::open(&project.database_root())?;

        connection.execute(
            "INSERT INTO sources(name, version) VALUES ('Bermuda', 'play-v1')",
            [],
        )?;
        let source_id = connection.last_insert_rowid();

        connection.execute(
            r#"
            INSERT INTO game_sources(game_id, source_id, original_path)
            VALUES (1, ?1, 'played:only')
            "#,
            [source_id],
        )?;
        let game_source_id = connection.last_insert_rowid();

        connection.execute(
            "INSERT INTO game_metadata(game_source_id) VALUES (?1)",
            [game_source_id],
        )?;

        connection.execute(
            r#"
            INSERT INTO indexed_games(
                game_id,
                index_version,
                occurrence_count
            )
            VALUES (1, 1, 1)
            "#,
            [],
        )?;

        connection.execute(
            r#"
            INSERT INTO exact_positions(
                position_hash,
                game_id,
                move_number,
                side_to_move,
                ko_point
            )
            VALUES (?1, 1, 0, 0, NULL)
            "#,
            params![vec![0_u8; 32]],
        )?;

        drop(connection);

        let move_file = project.database_root().join(relative_path);
        assert!(move_file.exists());

        let mut store = project.game_store()?;
        store.remove_source(game_source_id)?;
        drop(store);

        let connection = database::open(&project.database_root())?;

        let source_count: i64 = connection.query_row(
            "SELECT COUNT(*) FROM game_sources WHERE id = ?1",
            [game_source_id],
            |row| row.get(0),
        )?;

        let metadata_count: i64 = connection.query_row(
            "SELECT COUNT(*) FROM game_metadata WHERE game_source_id = ?1",
            [game_source_id],
            |row| row.get(0),
        )?;

        let game_count: i64 =
            connection.query_row("SELECT COUNT(*) FROM games WHERE id = 1", [], |row| {
                row.get(0)
            })?;

        let indexed_count: i64 = connection.query_row(
            "SELECT COUNT(*) FROM indexed_games WHERE game_id = 1",
            [],
            |row| row.get(0),
        )?;

        let position_count: i64 = connection.query_row(
            "SELECT COUNT(*) FROM exact_positions WHERE game_id = 1",
            [],
            |row| row.get(0),
        )?;

        assert_eq!(source_count, 0);
        assert_eq!(metadata_count, 0);
        assert_eq!(game_count, 0);
        assert_eq!(indexed_count, 0);
        assert_eq!(position_count, 0);
        assert!(!move_file.exists());

        Ok(())
    }

    #[test]
    fn reports_unknown_game_source_when_removing() -> Result<()> {
        let (_temporary, project) = create_test_project();
        let mut store = project.game_store()?;

        let error = store
            .remove_source(999)
            .expect_err("removing an unknown game source should fail");

        assert!(error.to_string().contains("game source 999 does not exist"));

        Ok(())
    }
}
