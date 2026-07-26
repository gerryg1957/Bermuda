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

#[derive(Debug, Default)]
pub struct ImportSummary {
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

pub fn run(
    project: &Project,
    source: &str,
    version: &str,
    directory: &Path,
) -> Result<ImportSummary> {
    if !directory.is_dir() {
        bail!("SGF source is not a directory: {}", directory.display());
    }

    let mut importer = Importer::open_project(project)?;
    let started = Instant::now();
    let mut summary = ImportSummary::default();
    let mut error_messages = Vec::new();

    for entry in WalkDir::new(directory).follow_links(false) {
        let entry = match entry {
            Ok(entry) => entry,
            Err(error) => {
                summary.errors += 1;
                error_messages.push(format!("directory traversal error: {error}"));
                continue;
            }
        };

        if !entry.file_type().is_file() || !is_sgf(entry.path()) {
            continue;
        }

        summary.processed += 1;

        if summary.processed.is_multiple_of(10_000) {
            let elapsed = started.elapsed().as_secs_f64();
            let rate = if elapsed > 0.0 {
                summary.processed as f64 / elapsed
            } else {
                0.0
            };

            eprintln!(
                "Processed {} games ({rate:.1} SGF files/second)...",
                summary.processed
            );
        }

        match importer.import_file(source, version, entry.path()) {
            Ok(ImportOutcome::Imported { .. }) => {
                summary.imported += 1;
            }

            Ok(ImportOutcome::AddedSource { .. }) => {
                summary.added_sources += 1;
            }

            Ok(ImportOutcome::AlreadyImported { .. }) => {
                summary.duplicates += 1;
            }

            Ok(ImportOutcome::SkippedBoardSize { .. }) => {
                summary.skipped += 1;
            }

            Err(error) => {
                summary.errors += 1;
                error_messages.push(format!("{}: {error:#}", entry.path().display()));
            }
        }
    }

    summary.elapsed_seconds = started.elapsed().as_secs_f64();
    summary.error_log = write_error_log(&project.database_root(), &error_messages)?;

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
}
