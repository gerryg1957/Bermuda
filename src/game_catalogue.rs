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
        let rows = game_list::list_games(&self.connection, query)?;
        let mut recovered = false;
        for row in &rows { recovered |= self.recover_ranks(row.game_id)?; }
        if recovered { game_list::list_games(&self.connection, query) } else { Ok(rows) }
    }

    pub fn list_occurrences(&self, query: &GameListQuery) -> Result<Vec<GameOccurrenceRow>> {
        let rows = game_list::list_game_occurrences(&self.connection, query)?;
        let mut recovered = false;
        for row in &rows { recovered |= self.recover_ranks(row.game_id)?; }
        if recovered { game_list::list_game_occurrences(&self.connection, query) } else { Ok(rows) }
    }

    // Older imports did not retain BR/WR. Recover only from an unchanged,
    // readable source; hash-based personal imports can instead be re-added.
    fn recover_ranks(&self, game_id: i64) -> Result<bool> {
        let mut stmt = self.connection.prepare("SELECT s.id, s.original_path, g.canonical_hash
            FROM game_sources s JOIN games g ON g.id=s.game_id
            JOIN game_metadata m ON m.game_source_id=s.id
            WHERE s.game_id=?1 AND (m.black_rank IS NULL OR m.white_rank IS NULL)")?;
        let sources = stmt.query_map([game_id], |row| Ok((row.get::<_, i64>(0)?,
            row.get::<_, String>(1)?, row.get::<_, Vec<u8>>(2)?)))?.collect::<rusqlite::Result<Vec<_>>>()?;
        let mut recovered = false;
        for (id, path, hash) in sources {
            let Ok(bytes) = std::fs::read(path) else { continue };
            let Ok(collection) = crate::parse_collection(&bytes) else { continue };
            let Ok(record) = crate::extract_main_variation(&collection) else { continue };
            if crate::canonical_hash(&record)?.as_slice() != hash.as_slice() { continue; }
            if record.metadata.black_rank.is_none() && record.metadata.white_rank.is_none() { continue; }
            self.connection.execute("UPDATE game_metadata SET black_rank=COALESCE(black_rank,?1),
                white_rank=COALESCE(white_rank,?2) WHERE game_source_id=?3",
                rusqlite::params![record.metadata.black_rank, record.metadata.white_rank, id])?;
            recovered = true;
        }
        Ok(recovered)
    }

    pub fn count(&self, query: &GameListQuery) -> Result<u64> {
        game_list::count_games(&self.connection, query)
    }

    pub fn get(&self, game_id: i64) -> Result<GameListRow> {
        self.recover_ranks(game_id)?;
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
