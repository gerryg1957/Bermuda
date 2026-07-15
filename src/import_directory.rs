use crate::importer::{ImportOutcome, Importer};
use anyhow::{Context, Result};
use std::{fs, path::Path, time::Instant};
use walkdir::WalkDir;

#[derive(Debug, Default)]
struct ImportSummary {
    processed: usize,
    imported: usize,
    added_sources: usize,
    duplicates: usize,
    skipped: usize,
    errors: usize,
}

pub fn run(database: &Path, source: &str, version: &str, directory: &Path) -> Result<()> {
    if !directory.is_dir() {
        anyhow::bail!("SGF source is not a directory: {}", directory.display());
    }

    let mut importer = Importer::open(database)?;
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

            println!(
                "Processed {} games ({rate:.1} games/second)...",
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

            Ok(ImportOutcome::SkippedBoardSize { board_size }) => {
                summary.skipped += 1;

                println!(
                    "Skipped {}: unsupported board size {}x{}",
                    entry.path().display(),
                    board_size,
                    board_size
                );
            }

            Err(error) => {
                summary.errors += 1;

                error_messages.push(format!("{}: {error:#}", entry.path().display()));
            }
        }
    }

    write_error_log(database, &error_messages)?;
    let elapsed = started.elapsed();
    let elapsed_seconds = elapsed.as_secs_f64();

    let rate = if elapsed_seconds > 0.0 {
        summary.processed as f64 / elapsed_seconds
    } else {
        0.0
    };

    println!();
    println!("Import complete");
    println!("Processed    : {}", summary.processed);
    println!("Imported     : {}", summary.imported);
    println!("Added sources: {}", summary.added_sources);
    println!("Duplicates   : {}", summary.duplicates);
    println!("Skipped      : {}", summary.skipped);
    println!("Errors       : {}", summary.errors);
    println!("Elapsed      : {:.2} seconds", elapsed_seconds);
    println!("Rate         : {:.1} games/second", rate);

    if !error_messages.is_empty() {
        println!(
            "Error log    : {}",
            database.join("import-errors.txt").display()
        );
    }

    Ok(())
}

fn is_sgf(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("sgf"))
}

fn write_error_log(database: &Path, errors: &[String]) -> Result<()> {
    let log_path = database.join("import-errors.txt");

    if errors.is_empty() {
        if log_path.exists() {
            fs::remove_file(&log_path)
                .with_context(|| format!("removing old error log {}", log_path.display()))?;
        }

        return Ok(());
    }

    let mut contents = errors.join("\n");
    contents.push('\n');

    fs::write(&log_path, contents)
        .with_context(|| format!("writing error log {}", log_path.display()))?;

    Ok(())
}
