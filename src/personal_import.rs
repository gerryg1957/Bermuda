//! Direct personal-library import. Never replaces the open study or starts analysis.
use std::{path::PathBuf, sync::atomic::{AtomicBool, Ordering}};
use anyhow::{Context, Result, ensure};
use serde::Serialize;
use crate::{project::Project, importer::ImportOutcome, indexer::POSITION_INDEX_VERSION};

#[derive(Default, Serialize)]
pub struct ImportSummary {
    pub added: usize,
    pub existing: usize,
    pub processed: usize,
    pub cancelled: bool,
    pub game_ids: Vec<i64>,
    pub errors: Vec<String>,
    pub unidentified: usize,
}

pub fn import_files(project: &Project, paths: &[PathBuf], stop: &AtomicBool,
    mut progress: impl FnMut(usize)) -> Result<ImportSummary> {
    let mut summary = ImportSummary::default();
    for path in paths {
        if stop.load(Ordering::Relaxed) { summary.cancelled = true; break; }
        let result = (|| -> Result<()> {
            let collection = crate::parse_collection(&std::fs::read(path)?)?;
            ensure!(collection.trees.len() == 1, "Select a single-game SGF; this file contains {} games", collection.trees.len());
            let record = crate::extract_main_variation(&collection)?;
            ensure!(!record.moves.is_empty(), "This is a position without game moves");
            let locator = crate::canonical_hash_hex(&record)?;
            let outcome = project.importer()?.import_record("Imported personal games", "1", &locator, &record)?;
            let id = match outcome {
                ImportOutcome::Imported { game_id, .. } => { summary.added += 1; game_id }
                ImportOutcome::AddedSource { game_id } | ImportOutcome::AlreadyImported { game_id } => {
                    summary.existing += 1; game_id
                }
                ImportOutcome::SkippedBoardSize { board_size } => anyhow::bail!("My Games does not yet support {board_size}x{board_size} games"),
            };
            if !summary.game_ids.contains(&id) { summary.game_ids.push(id); }
            // A failed index is reported separately from the successful import;
            // selecting the same SGF again retries the index without duplicating it.
            project.position_indexer()?.index_game_by_id(id, POSITION_INDEX_VERSION)
                .context("Game saved, but search indexing failed; add this file again to retry")?;
            Ok(())
        })();
        if let Err(error) = result { summary.errors.push(format!("{}: {error:#}", path.display())); }
        summary.processed += 1;
        progress(summary.processed);
    }
    match unidentified_games(project, &summary.game_ids) {
        Ok(count) => summary.unidentified = count,
        Err(error) => summary.errors.push(format!("Games saved, but could not check your names: {error:#}")),
    }
    Ok(summary)
}

fn unidentified_games(project: &Project, ids: &[i64]) -> Result<usize> {
    let Some(me) = project.player_directory()?.personal_player_id()? else { return Ok(ids.len()); };
    let db = crate::database::open(&project.database_root())?;
    let mut query = db.prepare("SELECT MAX(COALESCE(m.black_player_id=?1,0)) + MAX(COALESCE(m.white_player_id=?1,0))
        FROM game_sources s JOIN game_metadata m ON m.game_source_id=s.id WHERE s.game_id=?2")?;
    let mut count = 0;
    for id in ids {
        let colours: Option<i64> = query.query_row(rusqlite::params![me, id], |r| r.get(0))?;
        if colours != Some(1) { count += 1; }
    }
    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn imports_and_deduplicates_without_losing_other_files_on_error() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let project = crate::project_manager::ProjectManager::new().create("Personal", temp.path().join("project"))?;
        let a = temp.path().join("a.sgf");
        let b = temp.path().join("b.sgf");
        let bad = temp.path().join("bad.sgf");
        std::fs::write(&a, "(;SZ[19]PB[me]PW[other];B[aa];W[bb])")?;
        std::fs::copy(&a, &b)?;
        std::fs::write(&bad, "not an SGF")?;
        let result = import_files(&project, &[a.clone(), bad, b], &AtomicBool::new(false), |_| {})?;
        assert_eq!((result.added, result.existing, result.processed), (1,1,3));
        assert_eq!(result.errors.len(), 1);
        assert_eq!(result.game_ids.len(), 1);
        assert_eq!(result.unidentified, 1);
        let db = crate::database::open(&project.database_root())?;
        let source: i64 = db.query_row("SELECT id FROM sources WHERE name='Imported personal games'", [], |r| r.get(0))?;
        drop(db);
        project.player_directory()?.save_my_names(None, "Me", &[(source, "me".into())])?;
        assert_eq!(project.game_store()?.positions(result.game_ids[0])?.len(), 3);
        let retry = import_files(&project, &[a], &AtomicBool::new(false), |_| {})?;
        assert_eq!((retry.added, retry.existing), (0,1));
        assert_eq!(retry.unidentified, 0);
        Ok(())
    }
    #[test]
    fn stopping_keeps_completed_imports_and_rejects_collections() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let project = crate::project_manager::ProjectManager::new().create("Personal", temp.path().join("project"))?;
        let a = temp.path().join("a.sgf");
        std::fs::write(&a, "(;SZ[19];B[aa])")?;
        let stop = AtomicBool::new(false);
        let result = import_files(&project, &[a.clone(), a.clone()], &stop, |_| stop.store(true, Ordering::Relaxed))?;
        assert_eq!(result.added, 1);
        assert_eq!(result.processed, 1);
        assert!(result.cancelled);
        std::fs::write(&a, "(;SZ[19];B[bb])(;SZ[19];B[cc])")?;
        let result = import_files(&project, &[a], &AtomicBool::new(false), |_| {})?;
        assert_eq!(result.added, 0);
        assert_eq!(result.errors.len(), 1);
        Ok(())
    }
}
