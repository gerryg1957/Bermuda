use anyhow::{Context, Result};
use rayon::prelude::*;
use std::{
    fs::{self, File},
    io::{BufWriter, Write},
    path::{Path, PathBuf},
    process,
    time::Instant,
};

use crate::{
    pattern_index_format::{PATTERN_INDEX_FORMAT_VERSION, encode_game_block},
    project::Project,
    read_move_file,
};

pub const PATTERN_INDEX_FILENAME: &str = "pattern-positions-v1.bin";

const ERROR_LOG_FILENAME: &str = "pattern-index-errors.txt";
const ENCODE_CHUNK_SIZE: usize = 256;
const WRITE_BUFFER_SIZE: usize = 1024 * 1024;

#[derive(Debug, Clone, Default)]
pub struct PatternIndexBuildSummary {
    pub format_version: u32,
    pub total_games: usize,
    pub processed_games: usize,
    pub indexed_games: usize,
    pub indexed_positions: u64,
    pub errors: usize,
    pub elapsed_seconds: f64,
    pub index_path: Option<PathBuf>,
    pub index_bytes: u64,
    pub error_log: Option<PathBuf>,
}

impl PatternIndexBuildSummary {
    pub fn rate(&self) -> f64 {
        if self.elapsed_seconds > 0.0 {
            self.processed_games as f64 / self.elapsed_seconds
        } else {
            0.0
        }
    }
}

#[derive(Debug, Clone)]
pub struct PatternIndexBuildProgress {
    pub format_version: u32,
    pub total_games: usize,
    pub processed_games: usize,
    pub indexed_games: usize,
    pub indexed_positions: u64,
    pub errors: usize,
    pub elapsed_seconds: f64,
    pub current_game_id: Option<i64>,
    pub current_move_file: Option<PathBuf>,
}

impl PatternIndexBuildProgress {
    pub fn rate(&self) -> f64 {
        if self.elapsed_seconds > 0.0 {
            self.processed_games as f64 / self.elapsed_seconds
        } else {
            0.0
        }
    }
}

#[derive(Debug)]
pub enum PatternIndexBuildOutcome {
    Completed(PatternIndexBuildSummary),
    Cancelled(PatternIndexBuildSummary),
}

pub fn run(project: &Project) -> Result<PatternIndexBuildSummary> {
    match run_with_progress(project, || false, |_| {})? {
        PatternIndexBuildOutcome::Completed(summary) => Ok(summary),

        PatternIndexBuildOutcome::Cancelled(_) => {
            unreachable!("an uncancellable pattern-index build was cancelled")
        }
    }
}

pub fn run_with_progress<C, P>(
    project: &Project,
    mut is_cancelled: C,
    mut on_progress: P,
) -> Result<PatternIndexBuildOutcome>
where
    C: FnMut() -> bool,
    P: FnMut(PatternIndexBuildProgress),
{
    let started = Instant::now();

    let indexer = project.position_indexer()?;
    let games = indexer.games()?;

    let index_path = project.indexes_path().join(PATTERN_INDEX_FILENAME);
    let temp_path = project.indexes_path().join(format!(
        "{PATTERN_INDEX_FILENAME}.tmp-{}",
        process::id()
    ));

    let mut summary = PatternIndexBuildSummary {
        format_version: PATTERN_INDEX_FORMAT_VERSION,
        total_games: games.len(),
        ..PatternIndexBuildSummary::default()
    };

    let mut error_messages = Vec::new();

    let file = File::create(&temp_path)
        .with_context(|| format!("creating {}", temp_path.display()))?;

    let mut writer = BufWriter::with_capacity(WRITE_BUFFER_SIZE, file);

    on_progress(progress_snapshot(&summary, started, None, None));

    for chunk in games.chunks(ENCODE_CHUNK_SIZE) {
        if is_cancelled() {
            drop(writer);
            remove_if_exists(&temp_path)?;

            let summary =
                finish_summary(project, summary, started, &error_messages)?;

            return Ok(PatternIndexBuildOutcome::Cancelled(summary));
        }

        /*
         * Move files are independent, so replay and encoding can run in
         * parallel. Collecting from this indexed parallel iterator preserves
         * the chunk's game-ID order; writing remains deterministic.
         */
        let encoded = chunk
            .par_iter()
            .map(|game| {
                let record = read_move_file(&game.move_file).with_context(|| {
                    format!("reading {}", game.move_file.display())
                })?;

                let position_count = record.moves.len().saturating_add(1);

                let bytes =
                    encode_game_block(game.game_id, &record).with_context(|| {
                        format!(
                            "encoding game {} from {}",
                            game.game_id,
                            game.move_file.display()
                        )
                    })?;

                Ok::<_, anyhow::Error>((position_count, bytes))
            })
            .collect::<Vec<_>>();

        for (game, result) in chunk.iter().zip(encoded) {
            if is_cancelled() {
                drop(writer);
                remove_if_exists(&temp_path)?;

                let summary =
                    finish_summary(project, summary, started, &error_messages)?;

                return Ok(PatternIndexBuildOutcome::Cancelled(summary));
            }

            match result {
                Ok((position_count, bytes)) => {
                    writer.write_all(&bytes).with_context(|| {
                        format!("writing {}", temp_path.display())
                    })?;

                    summary.indexed_games =
                        summary.indexed_games.saturating_add(1);

                    summary.indexed_positions =
                        summary.indexed_positions.saturating_add(
                            u64::try_from(position_count).unwrap_or(u64::MAX),
                        );
                }

                Err(error) => {
                    summary.errors = summary.errors.saturating_add(1);

                    error_messages.push(format!(
                        "Failed to index game {} from {}: {error:#}",
                        game.game_id,
                        game.move_file.display(),
                    ));
                }
            }

            summary.processed_games =
                summary.processed_games.saturating_add(1);

            on_progress(progress_snapshot(
                &summary,
                started,
                Some(game.game_id),
                Some(&game.move_file),
            ));
        }
    }

    writer
        .flush()
        .with_context(|| format!("flushing {}", temp_path.display()))?;

    writer
        .get_ref()
        .sync_all()
        .with_context(|| format!("syncing {}", temp_path.display()))?;

    drop(writer);

    /*
     * A partial index is never published. If any game failed, discard the
     * temporary build and leave an existing valid index untouched.
     */
    if summary.errors != 0 {
        remove_if_exists(&temp_path)?;

        let summary =
            finish_summary(project, summary, started, &error_messages)?;

        on_progress(progress_snapshot(&summary, started, None, None));

        return Ok(PatternIndexBuildOutcome::Completed(summary));
    }

    fs::rename(&temp_path, &index_path).with_context(|| {
        format!(
            "publishing pattern index {} as {}",
            temp_path.display(),
            index_path.display()
        )
    })?;

    summary.index_bytes = fs::metadata(&index_path)
        .with_context(|| format!("reading metadata for {}", index_path.display()))?
        .len();

    summary.index_path = Some(index_path);

    let summary =
        finish_summary(project, summary, started, &error_messages)?;

    on_progress(progress_snapshot(&summary, started, None, None));

    Ok(PatternIndexBuildOutcome::Completed(summary))
}

fn progress_snapshot(
    summary: &PatternIndexBuildSummary,
    started: Instant,
    current_game_id: Option<i64>,
    current_move_file: Option<&Path>,
) -> PatternIndexBuildProgress {
    PatternIndexBuildProgress {
        format_version: summary.format_version,
        total_games: summary.total_games,
        processed_games: summary.processed_games,
        indexed_games: summary.indexed_games,
        indexed_positions: summary.indexed_positions,
        errors: summary.errors,
        elapsed_seconds: started.elapsed().as_secs_f64(),
        current_game_id,
        current_move_file: current_move_file.map(Path::to_path_buf),
    }
}

fn finish_summary(
    project: &Project,
    mut summary: PatternIndexBuildSummary,
    started: Instant,
    error_messages: &[String],
) -> Result<PatternIndexBuildSummary> {
    summary.elapsed_seconds = started.elapsed().as_secs_f64();
    summary.error_log =
        write_error_log(&project.database_root(), error_messages)?;

    Ok(summary)
}

fn write_error_log(
    database_root: &Path,
    errors: &[String],
) -> Result<Option<PathBuf>> {
    let log_path = database_root.join(ERROR_LOG_FILENAME);

    if errors.is_empty() {
        if log_path.exists() {
            fs::remove_file(&log_path).with_context(|| {
                format!("removing old {}", log_path.display())
            })?;
        }

        return Ok(None);
    }

    let mut contents = errors.join("\n\n");
    contents.push('\n');

    fs::write(&log_path, contents)
        .with_context(|| format!("writing {}", log_path.display()))?;

    Ok(Some(log_path))
}

fn remove_if_exists(path: &Path) -> Result<()> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),

        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),

        Err(error) => Err(error)
            .with_context(|| format!("removing {}", path.display())),
    }
}
