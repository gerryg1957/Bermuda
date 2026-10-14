//! Discover projects in managed locations and explicitly remembered locations.
//! Never scan game records or recursively search the user's filesystem.
use crate::{database_task, project_manager::ProjectManager};
use serde::Serialize;
use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
};

#[derive(Debug, Serialize)]
pub struct DatabaseChoice {
    pub name: String,
    pub path: PathBuf,
    pub unfinished: bool,
    pub recovery_error: Option<String>,
}

pub fn discover(managed: &Path, legacy: &Path, remembered: &[PathBuf]) -> Vec<DatabaseChoice> {
    let mut candidates = remembered.to_vec();
    for root in [managed, legacy] {
        if root.as_os_str().is_empty() {
            continue;
        }
        candidates.push(root.to_owned());
        if let Some(parent) = root.parent() {
            if let Ok(entries) = fs::read_dir(parent) {
                candidates.extend(entries.filter_map(Result::ok).map(|entry| entry.path()));
            }
        }
    }
    let mut seen = HashSet::new();
    let mut choices = Vec::new();
    for candidate in candidates {
        let Ok(path) = candidate.canonicalize() else {
            continue;
        };
        if !seen.insert(path.clone()) {
            continue;
        }
        let Ok(project) = ProjectManager::new().open(&path) else {
            continue;
        };
        let (unfinished, recovery_error) = match database_task::load(&path) {
            Ok(task) => (task.is_some(), None),
            Err(error) => (false, Some(format!("Cannot read saved setup: {error:#}"))),
        };
        choices.push(DatabaseChoice {
            name: project.name().to_owned(),
            path,
            unfinished,
            recovery_error,
        });
    }
    choices.sort_by(|a, b| {
        a.name
            .to_lowercase()
            .cmp(&b.name.to_lowercase())
            .then_with(|| a.path.cmp(&b.path))
    });
    choices
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn discovers_siblings_and_remembered_projects_without_duplicates_or_recursion()
    -> anyhow::Result<()> {
        let temp = tempfile::tempdir()?;
        let base = temp.path();
        let managed = base.join("Bermuda/games-database");
        let legacy = base.join("MoyoDB/games-database");
        let test = base.join("Bermuda/Test");
        let external = base.join("elsewhere/Collection");
        let manager = ProjectManager::new();
        for path in [&managed, &legacy, &test, &external] {
            manager.create(path.file_name().unwrap().to_string_lossy(), path)?;
        }
        manager.create("Nested", base.join("Bermuda/nested/Hidden"))?;
        database_task::save(&test, &database_task::DatabaseTask::Index)?;
        let choices = discover(
            &managed,
            &legacy,
            &[managed.clone(), external.clone(), base.join("deleted")],
        );
        assert_eq!(choices.len(), 4);
        assert!(
            choices
                .iter()
                .any(|c| c.path == test.canonicalize().unwrap() && c.unfinished)
        );
        assert!(
            choices
                .iter()
                .any(|c| c.path == external.canonicalize().unwrap())
        );
        fs::remove_dir_all(&test)?;
        assert_eq!(discover(&managed, &legacy, &[external]).len(), 3);
        Ok(())
    }
}
