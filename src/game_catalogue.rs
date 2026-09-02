use std::path::Path;

use anyhow::Result;
use rusqlite::Connection;

use crate::{
    database,
    game_list::{self, GameListQuery, GameListRow, GameOccurrenceRow},
    player_catalogue::PlayerCatalogue,
    project::Project,
};

pub struct GameCatalogue {
    connection: Connection,
}

impl GameCatalogue {
    pub fn open(database_root: &Path) -> Result<Self> {
        let mut connection = database::open(database_root)?;

        PlayerCatalogue::prepare_supplied(&mut connection)?;

        Ok(Self { connection })
    }

    pub fn open_project(project: &Project) -> Result<Self> {
        Self::open(&project.database_root())
    }

    pub fn list(&self, query: &GameListQuery) -> Result<Vec<GameListRow>> {
        game_list::list_games(&self.connection, query)
    }

    pub fn list_occurrences(&self, query: &GameListQuery) -> Result<Vec<GameOccurrenceRow>> {
        game_list::list_game_occurrences(&self.connection, query)
    }

    pub fn count(&self, query: &GameListQuery) -> Result<u64> {
        game_list::count_games(&self.connection, query)
    }

    pub fn get(&self, game_id: i64) -> Result<GameListRow> {
        game_list::get_game(&self.connection, game_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::{game_list::GameListQuery, project_manager::ProjectManager};
    use tempfile::tempdir;

    #[test]
    fn opens_catalogue_from_project_and_lists_games() -> Result<()> {
        let temporary_directory = tempdir()?;
        let project_root = temporary_directory.path().join("test-project");

        let manager = ProjectManager::new();
        let project = manager.create("Test Project", &project_root)?;

        let catalogue = project.catalogue()?;
        let games = catalogue.list(&GameListQuery::default())?;
        let count = catalogue.count(&GameListQuery::default())?;

        assert_eq!(count, 0);

        assert!(games.is_empty());

        let error = catalogue.get(1).expect_err("unknown game should fail");

        assert!(error.to_string().contains("game 1 does not exist"));

        Ok(())
    }
}
