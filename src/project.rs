use anyhow::Result;
use std::path::{Path, PathBuf};

use crate::{
    game_catalogue::GameCatalogue, game_store::GameStore, importer::Importer,
    indexer::PositionIndexer, player_directory::PlayerDirectory,
};
const CONFIG_FILENAME: &str = "moyodb-project.toml";
const DATABASE_DIRECTORY: &str = "database";
const INDEXES_DIRECTORY: &str = "indexes";
const CACHE_DIRECTORY: &str = "cache";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Project {
    name: String,
    root: PathBuf,
}

impl Project {
    pub fn new(name: impl Into<String>, root: impl Into<PathBuf>) -> Self {
        Self {
            name: name.into(),
            root: root.into(),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn config_path(&self) -> PathBuf {
        self.root.join(CONFIG_FILENAME)
    }

    pub fn database_root(&self) -> PathBuf {
        self.root.join(DATABASE_DIRECTORY)
    }

    pub fn indexes_path(&self) -> PathBuf {
        self.root.join(INDEXES_DIRECTORY)
    }

    pub fn cache_path(&self) -> PathBuf {
        self.root.join(CACHE_DIRECTORY)
    }

    pub fn catalogue(&self) -> Result<GameCatalogue> {
        GameCatalogue::open_project(self)
    }

    pub fn game_store(&self) -> Result<GameStore> {
        GameStore::open_project(self)
    }

    pub fn importer(&self) -> Result<Importer> {
        Importer::open_project(self)
    }

    pub fn player_directory(&self) -> Result<PlayerDirectory> {
        PlayerDirectory::open_project(self)
    }

    pub fn position_indexer(&self) -> Result<PositionIndexer> {
        PositionIndexer::open_project(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_paths_relative_to_project_root() {
        let project = Project::new(
            "Professional Games",
            "/home/gerry/Bermuda/Professional Games",
        );

        assert_eq!(project.name(), "Professional Games");
        assert_eq!(
            project.root(),
            Path::new("/home/gerry/Bermuda/Professional Games")
        );
        assert_eq!(
            project.config_path(),
            PathBuf::from("/home/gerry/Bermuda/Professional Games/moyodb-project.toml")
        );
        assert_eq!(
            project.database_root(),
            PathBuf::from("/home/gerry/Bermuda/Professional Games/database")
        );
        assert_eq!(
            project.indexes_path(),
            PathBuf::from("/home/gerry/Bermuda/Professional Games/indexes")
        );
        assert_eq!(
            project.cache_path(),
            PathBuf::from("/home/gerry/Bermuda/Professional Games/cache")
        );
    }
}
