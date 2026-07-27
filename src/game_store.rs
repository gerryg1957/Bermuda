use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use rusqlite::{Connection, OptionalExtension};

use crate::{database, game::GameRecord, move_file::read_move_file, project::Project};

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
        let relative_move_file: Option<String> = self
            .connection
            .query_row(
                "SELECT move_file FROM games WHERE id = ?1",
                [game_id],
                |row| row.get(0),
            )
            .optional()
            .context("reading game from database")?;

        let relative_move_file =
            relative_move_file.with_context(|| format!("game {game_id} does not exist"))?;

        let move_file = self.database_root.join(relative_move_file);

        read_move_file(&move_file).with_context(|| format!("reading move file for game {game_id}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::{
        game::{GameRecord, Metadata},
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
}
