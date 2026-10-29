//! Durable sequential queue. Position checkpoints stay in the existing analysis store.
use std::{path::Path, sync::atomic::{AtomicBool, Ordering}};
use anyhow::{Context, Result, ensure};
use rusqlite::{Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use crate::{AnalysisRequest, KataGoConfiguration, analysis_position_from_states,
    game_analysis::{self, AnalysisStore, GameAnalysis}, project::Project};

#[derive(Clone, Serialize, Deserialize)]
pub struct Batch {
    pub jobs: Vec<GameAnalysis>,
    pub completed: usize,
}

fn connection(project: &Project) -> Result<Connection> {
    std::fs::create_dir_all(project.cache_path())?;
    let db = Connection::open(project.cache_path().join("personal-analysis-queue.sqlite3"))?;
    db.execute_batch("PRAGMA synchronous=FULL;
        CREATE TABLE IF NOT EXISTS queue (id INTEGER PRIMARY KEY CHECK(id=1), manifest TEXT NOT NULL);")?;
    Ok(db)
}

pub fn load(project: &Project) -> Result<Option<Batch>> {
    if !project.cache_path().join("personal-analysis-queue.sqlite3").is_file() { return Ok(None); }
    let text: Option<String> = connection(project)?.query_row("SELECT manifest FROM queue WHERE id=1", [], |r| r.get(0)).optional()?;
    let batch: Option<Batch> = text.map(|s| serde_json::from_str(&s)).transpose()?;
    if let Some(batch) = &batch { ensure!(batch.completed <= batch.jobs.len(), "Invalid queue checkpoint"); }
    Ok(batch)
}

fn save(project: &Project, batch: &Batch) -> Result<()> {
    connection(project)?.execute("INSERT OR REPLACE INTO queue VALUES (1,?1)", [serde_json::to_string(batch)?])?;
    Ok(())
}

pub fn clear(project: &Project) -> Result<()> {
    connection(project)?.execute("DELETE FROM queue WHERE id=1", [])?;
    Ok(())
}

pub fn summary(project: &Project) -> Result<Value> {
    Ok(match load(project)? {
        Some(batch) if batch.completed < batch.jobs.len() => {
            let job = &batch.jobs[batch.completed];
            json!({"remaining": batch.jobs.len()-batch.completed, "total": batch.jobs.len(),
                "completed": batch.completed, "visits": job.request.max_visits, "rules": job.request.rules})
        }
        _ => json!({"remaining": 0}),
    })
}

/// Snapshot the pending and partial games from this report, in game-date order.
/// Completed reports are not automatically reanalysed with new settings.
pub fn create(project: &Project, analysis: &Path, config: &KataGoConfiguration,
    visits: u64, rules: &str, fallback_komi: f64, stop: &AtomicBool) -> Result<Batch> {
    ensure!((10..=100_000).contains(&visits) && fallback_komi.is_finite(), "Invalid analysis settings");
    ensure!(["japanese", "chinese", "aga", "tromp-taylor"].contains(&rules), "Select analysis rules");
    ensure!(load(project)?.is_none_or(|b| b.completed == b.jobs.len()), "Resume or clear the saved queue first");
    let report = crate::personal_report::build(project, analysis)?;
    ensure!(report["needs_names"] == false, "Set up your names first");
    let mut entries = report["pending"].as_array().cloned().unwrap_or_default();
    for group in report["groups"].as_array().into_iter().flatten() {
        for game in group["analysed"].as_array().into_iter().flatten() {
            if game["complete"] == false { entries.push(game.clone()); }
        }
    }
    entries.sort_by(|a,b| b["date"].as_str().unwrap_or("").cmp(a["date"].as_str().unwrap_or(""))
        .then_with(|| b["game_id"].as_i64().cmp(&a["game_id"].as_i64())));
    ensure!(!entries.is_empty(), "No pending games in this report");
    let catalogue = project.catalogue()?;
    let store = project.game_store()?;
    let mut jobs: Vec<GameAnalysis> = Vec::new();
    for entry in entries {
        ensure!(!stop.load(Ordering::Relaxed), "Queue preparation paused; no results lost");
        let id = entry["game_id"].as_i64().context("Invalid report game")?;
        let states = store.positions(id)?;
        let position = analysis_position_from_states(&states, states.len().saturating_sub(1))?;
        let game = catalogue.get(id)?;
        let request = AnalysisRequest { id: String::new(), board_size: position.board_size,
            rules: rules.into(), komi: game.komi.map(f64::from).unwrap_or(fallback_komi), initial_stones: position.initial_stones,
            initial_player: position.initial_player, moves: position.moves, max_visits: visits };
        let title = entry["title"].as_str().unwrap_or("My game").to_owned();
        let run = if let Some(first) = jobs.first() {
            game_analysis::prepare_with_provenance(request, title, first.provenance.clone())?
        } else { game_analysis::prepare(request, title, config, stop)? };
        jobs.push(run);
    }
    let batch = Batch { jobs, completed: 0 };
    save(project, &batch)?;
    Ok(batch)
}

pub fn configuration(batch: &Batch, working: &Path) -> Result<KataGoConfiguration> {
    let run = batch.jobs.get(batch.completed).context("Queue already complete")?;
    Ok(KataGoConfiguration::new(&run.provenance.executable, &run.provenance.network,
        &run.provenance.config, working))
}

/// The queue advances only after the last position is saved. Replaying a queue
/// checkpoint after a crash simply restores the already-complete analysis run.
pub fn run(project: &Project, mut batch: Batch, analysis: &Path, config: &KataGoConfiguration,
    stop: &AtomicBool, mut progress: impl FnMut(usize, usize, &GameAnalysis) -> bool) -> Result<Batch> {
    if batch.completed == batch.jobs.len() { return Ok(batch); }
    let first = &batch.jobs[batch.completed];
    let checked = game_analysis::prepare(first.request.clone(), first.title.clone(), config, stop)?;
    ensure!(checked.run_id == first.run_id,
        "Engine files changed since this queue was created. Restore them, or clear the queue and start again; saved analysis is retained");
    let store = AnalysisStore::open(analysis)?;
    while batch.completed < batch.jobs.len() && !stop.load(Ordering::Relaxed) {
        let index = batch.completed;
        let total = batch.jobs.len();
        let finished = game_analysis::analyse_game(batch.jobs[index].clone(), config, &store, stop,
            |run, _| {
                let keep_going = progress(index, total, run);
                if !keep_going { stop.store(true, Ordering::Relaxed); }
                keep_going
            })?;
        if !finished.complete() { break; }
        batch.completed += 1;
        save(project, &batch)?;
    }
    Ok(batch)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AnalysisResult, Colour};

    #[test]
    fn durable_queue_resumes_checkpoints_and_rejects_changed_engine_files() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let project = crate::project_manager::ProjectManager::new().create("Personal", temp.path().join("project"))?;
        for (index, sgf) in [
            "(;SZ[19]PB[me_online]PW[other]KM[7.5]DT[2026-10-29];B[aa];W[bb])",
            "(;SZ[19]PB[other]PW[me_online]DT[2026-10-28];B[cc];W[dd])",
        ].iter().enumerate() {
            let record = crate::extract_main_variation(&crate::parse_collection(sgf.as_bytes())?)?;
            project.importer()?.import_record("Test", "1", &format!("game-{index}"), &record)?;
        }
        let db = crate::database::open(&project.database_root())?;
        let source: i64 = db.query_row("SELECT id FROM sources WHERE name='Test'", [], |r| r.get(0))?;
        drop(db);
        project.player_directory()?.save_my_names(None, "Me", &[(source, "me_online".into())])?;
        let executable = temp.path().join("engine");
        let network = temp.path().join("network");
        let config_file = temp.path().join("config");
        for path in [&executable, &network, &config_file] { std::fs::write(path, "fixture")?; }
        let config = KataGoConfiguration::new(&executable, &network, &config_file, temp.path());
        let analysis = temp.path().join("analysis.sqlite3");
        let stop = AtomicBool::new(false);
        let batch = create(&project, &analysis, &config, 10, "japanese", 6.5, &stop)?;
        assert_eq!(batch.jobs.len(), 2);
        assert_eq!(batch.jobs[0].request.komi, 7.5);
        assert_eq!(batch.jobs[1].request.komi, 6.5);
        assert_eq!(summary(&project)?["remaining"], 2);
        assert!(create(&project, &analysis, &config, 10, "japanese", 6.5, &stop).is_err());
        let store = AnalysisStore::open(&analysis)?;
        let first = store.restore(batch.jobs[0].clone())?;
        let result = |run: &GameAnalysis, turn: usize| AnalysisResult {
            id: "test".into(), turn_number: turn as u32,
            current_player: run.request.moves.get(turn).map(|m| m.colour)
                .unwrap_or(Colour::Black), score_lead: 0.0, win_rate: 0.5,
            visits: 10, candidates: vec![], is_during_search: false, warnings: vec![],
        };
        store.save_position(&first, &result(&first, 0))?;
        // Stop at the first progress callback, before any engine process starts.
        let paused = run(&project, load(&project)?.unwrap(), &analysis, &config, &stop, |_,_,r| {
            assert_eq!(r.results.len(), 1);
            false
        })?;
        assert_eq!(paused.completed, 0);
        assert_eq!(load(&project)?.unwrap().completed, 0);
        stop.store(false, Ordering::Relaxed);
        std::fs::write(&network, "changed")?;
        assert!(run(&project, paused.clone(), &analysis, &config, &stop, |_,_,_| true).is_err());
        assert_eq!(store.load(&first.run_id)?.results.len(), 1);
        std::fs::write(&network, "fixture")?;
        // Simulate a crash after position commits but before the queue advances.
        for job in &batch.jobs {
            let mut saved = store.restore(job.clone())?;
            while !saved.complete() {
                let next = result(&saved, saved.results.len());
                store.save_position(&saved, &next)?;
                saved.results.push(next);
            }
        }
        let complete = run(&project, load(&project)?.unwrap(), &analysis, &config, &stop, |_,_,_| true)?;
        assert_eq!(complete.completed, 2);
        assert_eq!(summary(&project)?["remaining"], 0);
        clear(&project)?;
        assert!(load(&project)?.is_none());
        assert!(store.load(&first.run_id)?.complete());
        assert!(create(&project, &analysis, &config, 10, "japanese", 6.5, &stop).is_err());
        Ok(())
    }
}
