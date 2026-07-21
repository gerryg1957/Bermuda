use anyhow::{bail, Context, Result};
use std::{fs, path::Path};

use crate::{database, project::Project};

#[derive(Debug, Default)]
pub struct ProjectManager;

impl ProjectManager {
    pub fn new() -> Self {
        Self
    }

    pub fn create(
        &self,
        name: impl Into<String>,
        root: impl AsRef<Path>,
    ) -> Result<Project> {
        let project = Project::new(name, root.as_ref());

        if project.root().exists() {
            bail!("project already exists: {}", project.root().display());
        }

        fs::create_dir_all(project.root())
            .with_context(|| format!("creating project directory {}", project.root().display()))?;

        fs::create_dir(project.indexes_path())
            .with_context(|| format!("creating {}", project.indexes_path().display()))?;

        fs::create_dir(project.cache_path())
            .with_context(|| format!("creating {}", project.cache_path().display()))?;

        database::initialise(&project.database_root())?;

        let config = format!("version = 1\nname = {:?}\n", project.name());

        fs::write(project.config_path(), config)
            .with_context(|| format!("writing {}", project.config_path().display()))?;

        Ok(project)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::{
        env, fs,
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    fn temporary_project_root(test_name: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();

        env::temp_dir().join(format!("moyodb-test-{test_name}-{unique}"))
    }

    #[test]
    fn creates_project_directory_structure() -> anyhow::Result<()> {
        let root = temporary_project_root("create");

        let manager = ProjectManager::new();
        let project = manager.create("Test Project", &root)?;

        assert!(project.root().is_dir());
        assert!(project.config_path().is_file());
        let config = fs::read_to_string(project.config_path())?;

        assert_eq!(
        config,
        "version = 1\nname = \"Test Project\"\n"
        );

        assert!(project.database_root().is_dir());
        assert!(project.database_root().join("metadata.sqlite3").is_file());
        assert!(project.database_root().join("games").is_dir());
        assert!(project.database_root().join("tmp").is_dir());

        assert!(project.indexes_path().is_dir());
        assert!(project.cache_path().is_dir());

        fs::remove_dir_all(&root)?;

        Ok(())
    }
    #[test]
    fn refuses_to_overwrite_existing_path() -> anyhow::Result<()> {
    let root = temporary_project_root("existing");

    fs::create_dir_all(&root)?;
    fs::write(root.join("keep-me.txt"), "existing content")?;

    let manager = ProjectManager::new();
    let result = manager.create("Test Project", &root);

    assert!(result.is_err());
    assert_eq!(
        fs::read_to_string(root.join("keep-me.txt"))?,
        "existing content"
    );

    fs::remove_dir_all(&root)?;

    Ok(())
}

}
