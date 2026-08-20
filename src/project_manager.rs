use anyhow::{Context, Result, bail};
use std::{fs, path::Path};

use crate::{database, project::Project};

#[derive(Debug, Default)]
pub struct ProjectManager;

impl ProjectManager {
    pub fn new() -> Self {
        Self
    }

    pub fn create(&self, name: impl Into<String>, root: impl AsRef<Path>) -> Result<Project> {
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

    pub fn open(&self, root: impl AsRef<Path>) -> Result<Project> {
        let root = root.as_ref();

        if !root.is_dir() {
            bail!("project directory does not exist: {}", root.display());
        }

        let config_path = root.join("moyodb-project.toml");

        let config = fs::read_to_string(&config_path)
            .with_context(|| format!("reading {}", config_path.display()))?;

        let name = parse_project_name(&config)
            .with_context(|| format!("parsing {}", config_path.display()))?;

        let project = Project::new(name, root);

        if !project.database_root().is_dir() {
            bail!(
                "project database directory does not exist: {}",
                project.database_root().display()
            );
        }

        let metadata_path = project.database_root().join("metadata.sqlite3");

        if !metadata_path.is_file() {
            bail!(
                "project metadata database does not exist: {}",
                metadata_path.display()
            );
        }

        Ok(project)
    }
}

fn parse_project_name(config: &str) -> Result<String> {
    for line in config.lines() {
        let line = line.trim();

        if let Some(value) = line.strip_prefix("name = ") {
            let name = value
                .strip_prefix('"')
                .and_then(|value| value.strip_suffix('"'))
                .context("project name must be a quoted string")?;

            if name.is_empty() {
                bail!("project name must not be empty");
            }

            return Ok(name.to_owned());
        }
    }

    bail!("project configuration does not contain a name")
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

        env::temp_dir().join(format!("bermuda-test-{test_name}-{unique}"))
    }

    #[test]
    fn creates_project_directory_structure() -> anyhow::Result<()> {
        let root = temporary_project_root("create");

        let manager = ProjectManager::new();
        let project = manager.create("Test Project", &root)?;

        assert!(project.root().is_dir());
        assert!(project.config_path().is_file());

        let config = fs::read_to_string(project.config_path())?;

        assert_eq!(config, "version = 1\nname = \"Test Project\"\n");

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

    #[test]
    fn opens_existing_project() -> anyhow::Result<()> {
        let root = temporary_project_root("open");

        let manager = ProjectManager::new();
        manager.create("Test Project", &root)?;

        let project = manager.open(&root)?;

        assert_eq!(project.name(), "Test Project");
        assert_eq!(project.root(), root.as_path());
        assert!(project.database_root().is_dir());

        fs::remove_dir_all(&root)?;

        Ok(())
    }
}
