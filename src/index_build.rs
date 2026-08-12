use anyhow::Result;
use rayon::prelude::*;
use std::{
    fs,
    path::{Path, PathBuf},
    time::Instant,
};

use crate::{
    indexer::{POSITION_INDEX_VERSION, replay_game_for_index},
    project::Project,
};

const REPLAY_CHUNK_SIZE: usize = 256;

#[derive(Debug, Clone, Default)]
pub struct IndexBuildSummary {
    pub index_version: i64,
    pub total_games: usize,
    pub processed_games: usize,
    pub indexed_games: usize,
    pub indexed_positions: u64,
    pub errors: usize,
    pub elapsed_seconds: f64,
    pub error_log: Option<PathBuf>,
}

impl IndexBuildSummary {
    pub fn rate(&self) -> f64 {
        if self.elapsed_seconds > 0.0 {
            self.processed_games as f64 / self.elapsed_seconds
        } else {
            0.0
        }
    }
}

#[derive(Debug, Clone)]
pub struct IndexBuildProgress {
    pub index_version: i64,
    pub total_games: usize,
    pub processed_games: usize,
    pub indexed_games: usize,
    pub indexed_positions: u64,
    pub errors: usize,
    pub elapsed_seconds: f64,
    pub current_game_id: Option<i64>,
    pub current_move_file: Option<PathBuf>,
}

impl IndexBuildProgress {
    pub fn rate(&self) -> f64 {
        if self.elapsed_seconds > 0.0 {
            self.processed_games as f64 / self.elapsed_seconds
        } else {
            0.0
        }
    }
}

#[derive(Debug)]
pub enum IndexBuildOutcome {
    Completed(IndexBuildSummary),
    Cancelled(IndexBuildSummary),
}

pub fn run(project: &Project) -> Result<IndexBuildSummary> {
    match run_with_progress(project, || false, |_| {})? {
        IndexBuildOutcome::Completed(summary) => Ok(summary),

        IndexBuildOutcome::Cancelled(_) => {
            unreachable!("an uncancellable index build was cancelled")
        }
    }
}

pub fn run_with_progress<C, P>(
    project: &Project,
    mut is_cancelled: C,
    mut on_progress: P,
) -> Result<IndexBuildOutcome>
where
    C: FnMut() -> bool,
    P: FnMut(IndexBuildProgress),
{
    let started = Instant::now();

    let mut indexer = project.position_indexer()?;
    let games = indexer.games_to_index(POSITION_INDEX_VERSION)?;

    let bulk_mode = if indexer.position_index_bulk_mode_active()? {
        true
    } else if !games.is_empty() && indexer.position_index_is_empty()? {
        indexer.begin_bulk_position_index_build()?;
        true
    } else {
        false
    };

    let mut summary = IndexBuildSummary {
        index_version: POSITION_INDEX_VERSION,
        total_games: games.len(),
        ..IndexBuildSummary::default()
    };

    let mut error_messages = Vec::new();

    on_progress(progress_snapshot(&summary, started, None, None));

    for chunk in games.chunks(REPLAY_CHUNK_SIZE) {
        if is_cancelled() {
            let summary = finish_summary(project, summary, started, &error_messages)?;

            return Ok(IndexBuildOutcome::Cancelled(summary));
        }

        let replayed = chunk
            .par_iter()
            .map(|game| (game, replay_game_for_index(game)))
            .collect::<Vec<_>>();

        for (game, replay_result) in replayed {
            if is_cancelled() {
                let summary = finish_summary(project, summary, started, &error_messages)?;

                return Ok(IndexBuildOutcome::Cancelled(summary));
            }

            let current_game_id = Some(game.game_id);
            let current_move_file = Some(game.move_file.as_path());

            let result = replay_result.and_then(|stream| {
                if bulk_mode {
                    indexer.index_stream_bulk(&stream, POSITION_INDEX_VERSION)
                } else {
                    indexer.index_stream(&stream, POSITION_INDEX_VERSION)
                }
            });

            match result {
                Ok(occurrence_count) => {
                    summary.indexed_games = summary.indexed_games.saturating_add(1);

                    summary.indexed_positions = summary
                        .indexed_positions
                        .saturating_add(u64::try_from(occurrence_count).unwrap_or(u64::MAX));
                }

                Err(error) => {
                    summary.errors = summary.errors.saturating_add(1);

                    error_messages.push(format!(
                        "Failed to index game {} from {}: \
                         {error:#}",
                        game.game_id,
                        game.move_file.display(),
                    ));
                }
            }

            summary.processed_games = summary.processed_games.saturating_add(1);

            on_progress(progress_snapshot(
                &summary,
                started,
                current_game_id,
                current_move_file,
            ));
        }
    }

    if bulk_mode {
        indexer.finish_bulk_position_index_build()?;
    }

    let summary = finish_summary(project, summary, started, &error_messages)?;

    on_progress(progress_snapshot(&summary, started, None, None));

    Ok(IndexBuildOutcome::Completed(summary))
}

fn progress_snapshot(
    summary: &IndexBuildSummary,
    started: Instant,
    current_game_id: Option<i64>,
    current_move_file: Option<&Path>,
) -> IndexBuildProgress {
    IndexBuildProgress {
        index_version: summary.index_version,
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
    mut summary: IndexBuildSummary,
    started: Instant,
    error_messages: &[String],
) -> Result<IndexBuildSummary> {
    summary.elapsed_seconds = started.elapsed().as_secs_f64();

    summary.error_log = write_error_log(&project.database_root(), error_messages)?;

    Ok(summary)
}

fn write_error_log(database_root: &Path, errors: &[String]) -> Result<Option<PathBuf>> {
    let log_path = database_root.join("position-index-errors.txt");

    if errors.is_empty() {
        if log_path.exists() {
            fs::remove_file(&log_path)?;
        }

        return Ok(None);
    }

    let mut contents = errors.join("\n");
    contents.push('\n');

    fs::write(&log_path, contents)?;

    Ok(Some(log_path))
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::{cell::Cell, fs};
    use tempfile::TempDir;

    use crate::project_manager::ProjectManager;

    fn create_project_with_games() -> Result<(TempDir, Project)> {
        let temporary = TempDir::new()?;

        let project =
            ProjectManager::new().create("Index Build Test", temporary.path().join("project"))?;

        let sgf_directory = temporary.path().join("sgfs");

        fs::create_dir(&sgf_directory)?;

        let first = sgf_directory.join("first.sgf");
        let second = sgf_directory.join("second.sgf");

        fs::write(&first, "(;FF[4]GM[1]SZ[19]PB[A]PW[B];B[pd])")?;

        fs::write(&second, "(;FF[4]GM[1]SZ[19]PB[C]PW[D];B[dd])")?;

        let mut importer = project.importer()?;

        importer.import_file("Test Source", "1", &first)?;

        importer.import_file("Test Source", "1", &second)?;

        Ok((temporary, project))
    }

    #[test]
    fn builds_pending_games_with_progress() -> Result<()> {
        let (_temporary, project) = create_project_with_games()?;

        let mut progress = Vec::new();

        let outcome = run_with_progress(&project, || false, |snapshot| progress.push(snapshot))?;

        let IndexBuildOutcome::Completed(summary) = outcome else {
            panic!("index build should complete");
        };

        assert_eq!(summary.total_games, 2);
        assert_eq!(summary.processed_games, 2);
        assert_eq!(summary.indexed_games, 2);
        assert_eq!(summary.indexed_positions, 4);
        assert_eq!(summary.errors, 0);

        assert!(
            progress
                .iter()
                .any(|snapshot| { snapshot.processed_games == 0 && snapshot.total_games == 2 })
        );

        assert!(
            progress.iter().any(|snapshot| {
                snapshot.processed_games == 2 && snapshot.indexed_positions == 4
            })
        );

        let indexer = project.position_indexer()?;

        assert_eq!(indexer.count_games_to_index(POSITION_INDEX_VERSION,)?, 0);

        Ok(())
    }

    #[test]
    fn cancels_safely_between_games() -> Result<()> {
        let (_temporary, project) = create_project_with_games()?;

        let cancel = Cell::new(false);

        let outcome = run_with_progress(
            &project,
            || cancel.get(),
            |snapshot| {
                if snapshot.processed_games >= 1 {
                    cancel.set(true);
                }
            },
        )?;

        let IndexBuildOutcome::Cancelled(summary) = outcome else {
            panic!("index build should be cancelled");
        };

        assert_eq!(summary.processed_games, 1);
        assert_eq!(summary.indexed_games, 1);
        assert_eq!(summary.errors, 0);

        let indexer = project.position_indexer()?;

        assert_eq!(indexer.count_games_to_index(POSITION_INDEX_VERSION,)?, 1);

        Ok(())
    }
}
