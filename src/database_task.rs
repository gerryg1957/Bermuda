//! Durable continuation instructions, separate from game data and schema.
//! Completed game transactions are the checkpoint; import safely re-scans.
use anyhow::{Context, Result, bail};
use rusqlite::{Connection, OpenFlags, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DatabaseTask {
    Import {
        directory: PathBuf,
        source: String,
        version: String,
        build_index: bool,
    },
    Index,
}

fn path(root: &Path) -> PathBuf {
    root.join("database-task.sqlite3")
}

pub fn load(root: &Path) -> Result<Option<DatabaseTask>> {
    if !path(root).exists() {
        return Ok(None);
    }
    let connection = Connection::open_with_flags(path(root), OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    let row: Option<(i64, String)> = connection
        .query_row(
            "SELECT version, task FROM pending_task WHERE id = 1",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?;
    match row {
        None => Ok(None),
        Some((1, json)) => Ok(Some(
            serde_json::from_str(&json).context("reading saved database task")?,
        )),
        Some(_) => bail!("saved database task requires a newer Bermuda version"),
    }
}

pub fn save(root: &Path, task: &DatabaseTask) -> Result<()> {
    let mut connection = Connection::open(path(root))?;
    let transaction = connection.transaction()?;
    transaction.execute_batch("CREATE TABLE IF NOT EXISTS pending_task (id INTEGER PRIMARY KEY CHECK(id = 1), version INTEGER NOT NULL, task TEXT NOT NULL);")?;
    transaction.execute(
        "INSERT OR REPLACE INTO pending_task(id, version, task) VALUES(1, 1, ?1)",
        params![serde_json::to_string(task)?],
    )?;
    transaction.commit().context("saving database continuation")
}

pub fn clear(root: &Path) -> Result<()> {
    if path(root).exists() {
        let connection = Connection::open(path(root))?;
        connection.execute("DELETE FROM pending_task WHERE id = 1", [])?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn continuation_survives_reopen_phase_changes_and_completion() -> Result<()> {
        let directory = tempfile::tempdir()?;
        let root = directory.path();
        assert_eq!(load(root)?, None);
        assert!(!path(root).exists());
        let task = DatabaseTask::Import {
            directory: PathBuf::from("/games/GoGoD collection"),
            source: "GoGoD".into(),
            version: "2026".into(),
            build_index: true,
        };
        save(root, &task)?;
        assert_eq!(load(root)?, Some(task));
        save(root, &DatabaseTask::Index)?;
        assert_eq!(load(root)?, Some(DatabaseTask::Index));
        clear(root)?;
        assert_eq!(load(root)?, None);
        Ok(())
    }

    #[test]
    fn saved_settings_continue_import_then_index_after_reopening() -> Result<()> {
        use crate::{import_directory, index_build, project_manager::ProjectManager};
        use std::{cell::Cell, fs};
        let temp = tempfile::tempdir()?;
        let source = temp.path().join("sgfs");
        fs::create_dir(&source)?;
        fs::write(source.join("one.sgf"), "(;FF[4]GM[1]SZ[19];B[dd])")?;
        fs::write(source.join("two.sgf"), "(;FF[4]GM[1]SZ[19];B[pp])")?;
        let root = temp.path().join("project");
        let project = ProjectManager::new().create("Continuation test", &root)?;
        save(
            &root,
            &DatabaseTask::Import {
                directory: source.clone(),
                source: "Test".into(),
                version: "1".into(),
                build_index: true,
            },
        )?;
        let cancelled = Cell::new(false);
        let result = import_directory::run_with_progress(
            &project,
            "Test",
            "1",
            &source,
            || cancelled.get(),
            |p| {
                if p.processed == 1 {
                    cancelled.set(true);
                }
            },
        )?;
        assert!(matches!(
            result,
            import_directory::ImportDirectoryOutcome::Cancelled(_)
        ));
        drop(project);
        let reopened = ProjectManager::new().open(&root)?;
        let DatabaseTask::Import {
            directory,
            source,
            version,
            build_index,
        } = load(&root)?.unwrap()
        else {
            panic!("import settings must survive reopening");
        };
        assert!(build_index);
        let result = import_directory::run_with_progress(
            &reopened,
            &source,
            &version,
            &directory,
            || false,
            |_| {},
        )?;
        let import_directory::ImportDirectoryOutcome::Completed(summary) = result else {
            panic!("import failed");
        };
        assert_eq!(summary.imported, 1);
        assert_eq!(summary.duplicates, 1);
        save(&root, &DatabaseTask::Index)?;
        cancelled.set(false);
        let result = index_build::run_with_progress(
            &reopened,
            || cancelled.get(),
            |p| {
                if p.processed_games == 1 {
                    cancelled.set(true);
                }
            },
        )?;
        assert!(matches!(
            result,
            index_build::IndexBuildOutcome::Cancelled(_)
        ));
        drop(reopened);
        assert_eq!(load(&root)?, Some(DatabaseTask::Index));
        let reopened = ProjectManager::new().open(&root)?;
        let summary = index_build::run(&reopened)?;
        assert_eq!(summary.indexed_games, 1);
        clear(&root)?;
        assert_eq!(load(&root)?, None);
        Ok(())
    }

    #[test]
    fn unknown_task_version_is_reported_and_preserved() -> Result<()> {
        let directory = tempfile::tempdir()?;
        save(directory.path(), &DatabaseTask::Index)?;
        Connection::open(path(directory.path()))?
            .execute("UPDATE pending_task SET version = 2", [])?;
        assert!(
            load(directory.path())
                .unwrap_err()
                .to_string()
                .contains("newer Bermuda")
        );
        Ok(())
    }
}
