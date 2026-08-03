use anyhow::{Context, Result, bail};
use std::{
    fs,
    path::{Path, PathBuf},
    time::Instant,
};
use walkdir::WalkDir;

use crate::{
    importer::{ImportOutcome, Importer},
    project::Project,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImportStage {
    Discovering,
    Importing,
}

#[derive(Debug, Clone, Default)]
pub struct ImportSummary {
    pub total_sgf_files: usize,
    pub processed: usize,
    pub imported: usize,
    pub added_sources: usize,
    pub duplicates: usize,
    pub skipped: usize,
    pub errors: usize,
    pub elapsed_seconds: f64,
    pub error_log: Option<PathBuf>,
}

impl ImportSummary {
    pub fn rate(&self) -> f64 {
        if self.elapsed_seconds > 0.0 {
            self.processed as f64 / self.elapsed_seconds
        } else {
            0.0
        }
    }
}

#[derive(Debug, Clone)]
pub struct ImportProgress {
    pub stage: ImportStage,
    pub discovered_sgf_files: usize,
    pub total_sgf_files: usize,
    pub processed: usize,
    pub imported: usize,
    pub added_sources: usize,
    pub duplicates: usize,
    pub skipped: usize,
    pub errors: usize,
    pub elapsed_seconds: f64,
    pub current_file: Option<PathBuf>,
}

impl ImportProgress {
    pub fn rate(&self) -> f64 {
        if self.elapsed_seconds > 0.0 {
            self.processed as f64 / self.elapsed_seconds
        } else {
            0.0
        }
    }
}

#[derive(Debug)]
pub enum ImportDirectoryOutcome {
    Completed(ImportSummary),
    Cancelled(ImportSummary),
}

pub fn run(
    project: &Project,
    source: &str,
    version: &str,
    directory: &Path,
) -> Result<ImportSummary> {
    match run_with_progress(project, source, version, directory, || false, |_| {})? {
        ImportDirectoryOutcome::Completed(summary) => Ok(summary),

        ImportDirectoryOutcome::Cancelled(_) => {
            unreachable!("an uncancellable directory import was cancelled")
        }
    }
}

pub fn run_with_progress<C, P>(
    project: &Project,
    source: &str,
    version: &str,
    directory: &Path,
    mut is_cancelled: C,
    mut on_progress: P,
) -> Result<ImportDirectoryOutcome>
where
    C: FnMut() -> bool,
    P: FnMut(ImportProgress),
{
    if !directory.is_dir() {
        bail!("SGF source is not a directory: {}", directory.display());
    }

    if source.trim().is_empty() {
        bail!("source name must not be empty");
    }

    if version.trim().is_empty() {
        bail!("source version must not be empty");
    }

    let started = Instant::now();
    let mut discovered_sgf_files = 0_usize;

    on_progress(progress_snapshot(
        ImportStage::Discovering,
        discovered_sgf_files,
        &ImportSummary::default(),
        started,
        None,
    ));

    for entry in WalkDir::new(directory).follow_links(false) {
        if is_cancelled() {
            let summary = ImportSummary {
                total_sgf_files: discovered_sgf_files,
                elapsed_seconds: started.elapsed().as_secs_f64(),
                ..ImportSummary::default()
            };

            return Ok(ImportDirectoryOutcome::Cancelled(summary));
        }

        let Ok(entry) = entry else {
            // Traversal errors are recorded during the actual import
            // pass, so that they are counted only once.
            continue;
        };

        if entry.file_type().is_file() && is_sgf(entry.path()) {
            discovered_sgf_files = discovered_sgf_files.saturating_add(1);

            if discovered_sgf_files.is_multiple_of(1_000) {
                on_progress(progress_snapshot(
                    ImportStage::Discovering,
                    discovered_sgf_files,
                    &ImportSummary::default(),
                    started,
                    None,
                ));
            }
        }
    }

    let mut summary = ImportSummary {
        total_sgf_files: discovered_sgf_files,
        ..ImportSummary::default()
    };

    on_progress(progress_snapshot(
        ImportStage::Importing,
        discovered_sgf_files,
        &summary,
        started,
        None,
    ));

    if is_cancelled() {
        summary.elapsed_seconds = started.elapsed().as_secs_f64();

        return Ok(ImportDirectoryOutcome::Cancelled(summary));
    }

    let mut importer = Importer::open_project(project)?;
    let mut error_messages = Vec::new();

    for entry in WalkDir::new(directory).follow_links(false) {
        if is_cancelled() {
            let summary = finish_summary(project, summary, started, &error_messages)?;

            return Ok(ImportDirectoryOutcome::Cancelled(summary));
        }

        let entry = match entry {
            Ok(entry) => entry,

            Err(error) => {
                summary.errors = summary.errors.saturating_add(1);

                error_messages.push(format!("directory traversal error: {error}"));

                on_progress(progress_snapshot(
                    ImportStage::Importing,
                    discovered_sgf_files,
                    &summary,
                    started,
                    None,
                ));

                continue;
            }
        };

        if !entry.file_type().is_file() || !is_sgf(entry.path()) {
            continue;
        }

        summary.processed = summary.processed.saturating_add(1);

        match importer.import_file(source, version, entry.path()) {
            Ok(ImportOutcome::Imported { .. }) => {
                summary.imported = summary.imported.saturating_add(1);
            }

            Ok(ImportOutcome::AddedSource { .. }) => {
                summary.added_sources = summary.added_sources.saturating_add(1);
            }

            Ok(ImportOutcome::AlreadyImported { .. }) => {
                summary.duplicates = summary.duplicates.saturating_add(1);
            }

            Ok(ImportOutcome::SkippedBoardSize { .. }) => {
                summary.skipped = summary.skipped.saturating_add(1);
            }

            Err(error) => {
                summary.errors = summary.errors.saturating_add(1);

                error_messages.push(format!("{}: {error:#}", entry.path().display()));
            }
        }

        on_progress(progress_snapshot(
            ImportStage::Importing,
            discovered_sgf_files,
            &summary,
            started,
            Some(entry.path()),
        ));
    }

    let summary = finish_summary(project, summary, started, &error_messages)?;

    on_progress(progress_snapshot(
        ImportStage::Importing,
        discovered_sgf_files,
        &summary,
        started,
        None,
    ));

    Ok(ImportDirectoryOutcome::Completed(summary))
}

fn progress_snapshot(
    stage: ImportStage,
    discovered_sgf_files: usize,
    summary: &ImportSummary,
    started: Instant,
    current_file: Option<&Path>,
) -> ImportProgress {
    ImportProgress {
        stage,
        discovered_sgf_files,
        total_sgf_files: summary.total_sgf_files,
        processed: summary.processed,
        imported: summary.imported,
        added_sources: summary.added_sources,
        duplicates: summary.duplicates,
        skipped: summary.skipped,
        errors: summary.errors,
        elapsed_seconds: started.elapsed().as_secs_f64(),
        current_file: current_file.map(Path::to_path_buf),
    }
}

fn finish_summary(
    project: &Project,
    mut summary: ImportSummary,
    started: Instant,
    error_messages: &[String],
) -> Result<ImportSummary> {
    summary.elapsed_seconds = started.elapsed().as_secs_f64();

    summary.error_log = write_error_log(&project.database_root(), error_messages)?;

    Ok(summary)
}

fn is_sgf(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("sgf"))
}

fn write_error_log(database: &Path, errors: &[String]) -> Result<Option<PathBuf>> {
    let log_path = database.join("import-errors.txt");

    if errors.is_empty() {
        if log_path.exists() {
            fs::remove_file(&log_path)
                .with_context(|| format!("removing old error log {}", log_path.display()))?;
        }

        return Ok(None);
    }

    let mut contents = errors.join("\n");
    contents.push('\n');

    fs::write(&log_path, contents)
        .with_context(|| format!("writing error log {}", log_path.display()))?;

    Ok(Some(log_path))
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::{cell::Cell, fs};
    use tempfile::TempDir;

    use crate::project_manager::ProjectManager;

    fn create_project_with_sgfs() -> Result<(TempDir, Project, PathBuf)> {
        let temporary = TempDir::new()?;

        let project_root = temporary.path().join("project");

        let sgf_directory = temporary.path().join("sgfs");

        fs::create_dir(&sgf_directory)?;

        fs::write(
            sgf_directory.join("one.sgf"),
            "(;FF[4]GM[1]SZ[19]PB[A]PW[B];B[pd])",
        )?;

        fs::write(
            sgf_directory.join("two.SGF"),
            "(;FF[4]GM[1]SZ[19]PB[C]PW[D];B[dd])",
        )?;

        let project = ProjectManager::new().create("Import Test", &project_root)?;

        Ok((temporary, project, sgf_directory))
    }

    #[test]
    fn recognises_sgf_extension_case_insensitively() {
        assert!(is_sgf(Path::new("game.sgf")));
        assert!(is_sgf(Path::new("game.SGF")));
        assert!(is_sgf(Path::new("game.SgF")));

        assert!(!is_sgf(Path::new("game.txt")));
        assert!(!is_sgf(Path::new("game")));
    }

    #[test]
    fn calculates_import_rate() {
        let summary = ImportSummary {
            processed: 200,
            elapsed_seconds: 4.0,
            ..ImportSummary::default()
        };

        assert_eq!(summary.rate(), 50.0);
    }

    #[test]
    fn reports_discovery_and_import_progress() -> Result<()> {
        let (_temporary, project, sgf_directory) = create_project_with_sgfs()?;

        let mut progress = Vec::new();

        let outcome = run_with_progress(
            &project,
            "Test Source",
            "1",
            &sgf_directory,
            || false,
            |snapshot| progress.push(snapshot),
        )?;

        let ImportDirectoryOutcome::Completed(summary) = outcome else {
            panic!("import should complete");
        };

        assert_eq!(summary.total_sgf_files, 2);
        assert_eq!(summary.processed, 2);
        assert_eq!(summary.imported, 2);
        assert_eq!(summary.errors, 0);

        assert!(
            progress
                .iter()
                .any(|snapshot| { snapshot.stage == ImportStage::Discovering })
        );

        assert!(progress.iter().any(|snapshot| {
            snapshot.stage == ImportStage::Importing && snapshot.processed == 2
        }));

        Ok(())
    }

    #[test]
    fn cancels_during_sgf_discovery_before_importing() -> Result<()> {
        let (_temporary, project, sgf_directory) = create_project_with_sgfs()?;

        let cancel = Cell::new(false);
        let mut progress = Vec::new();

        let outcome = run_with_progress(
            &project,
            "Test Source",
            "1",
            &sgf_directory,
            || cancel.get(),
            |snapshot| {
                if snapshot.stage == ImportStage::Discovering {
                    cancel.set(true);
                }

                progress.push(snapshot);
            },
        )?;

        let ImportDirectoryOutcome::Cancelled(summary) = outcome else {
            panic!("discovery should be cancelled");
        };

        assert_eq!(summary.total_sgf_files, 0);
        assert_eq!(summary.processed, 0);
        assert_eq!(summary.imported, 0);
        assert_eq!(summary.added_sources, 0);
        assert_eq!(summary.duplicates, 0);
        assert_eq!(summary.skipped, 0);
        assert_eq!(summary.errors, 0);

        assert!(
            progress
                .iter()
                .any(|snapshot| { snapshot.stage == ImportStage::Discovering })
        );

        assert!(
            !progress
                .iter()
                .any(|snapshot| { snapshot.stage == ImportStage::Importing })
        );

        Ok(())
    }

    #[test]
    fn cancels_safely_between_sgf_files() -> Result<()> {
        let (_temporary, project, sgf_directory) = create_project_with_sgfs()?;

        let cancel = Cell::new(false);

        let outcome = run_with_progress(
            &project,
            "Test Source",
            "1",
            &sgf_directory,
            || cancel.get(),
            |snapshot| {
                if snapshot.stage == ImportStage::Importing && snapshot.processed >= 1 {
                    cancel.set(true);
                }
            },
        )?;

        let ImportDirectoryOutcome::Cancelled(summary) = outcome else {
            panic!("import should be cancelled");
        };

        assert_eq!(summary.processed, 1);
        assert_eq!(
            summary.imported
                + summary.added_sources
                + summary.duplicates
                + summary.skipped
                + summary.errors,
            1
        );

        Ok(())
    }
}
