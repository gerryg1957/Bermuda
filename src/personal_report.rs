//! A bounded, read-only summary of saved evidence for the player's recent games.
use std::{collections::BTreeMap, path::Path};
use anyhow::Result;
use rusqlite::{Connection, OpenFlags};
use serde_json::{Value, json};
use crate::{Colour, analysis_position_from_states, game_analysis::{AnalysisStore, GameAnalysis}, project::Project};

fn settings_key(run: &GameAnalysis) -> Result<String> {
    Ok(serde_json::to_string(&(
        run.provenance.policy, &run.provenance.engine_sha256,
        &run.provenance.network_sha256, &run.provenance.config_sha256,
        &run.provenance.score_perspective, run.request.max_visits,
        &run.request.rules, run.request.komi, run.request.board_size,
    ))?)
}

fn loss_bucket(loss: f64) -> usize {
    if loss < 1.0 { 0 } else if loss < 3.0 { 1 } else if loss < 5.0 { 2 } else { 3 }
}

/// Uses canonical games once, even when imported from several sources.
/// Conflicting colours (including self-play) are excluded rather than guessed.
pub fn build(project: &Project, analysis_path: &Path) -> Result<Value> {
    let directory = project.player_directory()?;
    let Some(me) = directory.personal_player_id()? else {
        return Ok(json!({"needs_names": true}));
    };
    let db = Connection::open_with_flags(project.database_root().join("metadata.sqlite3"), OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    let total: i64 = db.query_row("SELECT COUNT(*) FROM games", [], |row| row.get(0))?;
    let sql = "SELECT s.game_id,
        MAX(COALESCE(m.black_player_id = ?1, 0)), MAX(COALESCE(m.white_player_id = ?1, 0)),
        MAX(COALESCE(m.played_date_sort, '')) AS played
        FROM game_sources s JOIN game_metadata m ON m.game_source_id=s.id
        GROUP BY s.game_id
        HAVING MAX(COALESCE(m.black_player_id = ?1, 0)) + MAX(COALESCE(m.white_player_id = ?1, 0)) = 1
        ORDER BY played DESC, s.game_id DESC";
    let mut query = db.prepare(sql)?;
    let mut rows = query.query([me])?;
    let mut games = Vec::new();
    let mut identified = 0;
    while let Some(row) = rows.next()? {
        identified += 1;
        if games.len() < 20 {
            games.push((row.get::<_, i64>(0)?, if row.get::<_, i64>(1)? == 1 { Colour::Black } else { Colour::White }));
        }
    }
    drop(rows);
    drop(query);
    drop(db);
    let catalogue = project.catalogue()?;
    let game_store = project.game_store()?;
    let mut positions = Vec::new();
    for (id, _) in &games {
        let states = game_store.positions(*id)?;
        positions.push(analysis_position_from_states(&states, states.len().saturating_sub(1))?);
    }
    let store = if analysis_path.is_file() { Some(AnalysisStore::open(analysis_path)?) } else { None };
    let ids = match &store {
        Some(store) => store.latest_whole_game_ids(&positions)?,
        None => vec![None; games.len()],
    };
    let mut complete = 0;
    let mut partial = 0;
    let mut groups: BTreeMap<String, Value> = BTreeMap::new();
    let mut pending = Vec::new();
    for ((game_id, colour), run_id) in games.iter().zip(ids) {
        let game = catalogue.get(*game_id)?;
        let title = format!("{} — {}", game.black_player_display.as_deref().unwrap_or("Black"), game.white_player_display.as_deref().unwrap_or("White"));
        let Some(run_id) = run_id else {
            pending.push(json!({"game_id": game_id, "title": title, "date": game.game_date, "colour": colour, "result": game.result}));
            continue;
        };
        let run = store.as_ref().expect("saved ID requires store").load(&run_id)?;
        if run.complete() { complete += 1; } else { partial += 1; }
        let group = groups.entry(settings_key(&run)?).or_insert_with(|| json!({
            "visits": run.request.max_visits, "rules": run.request.rules,
            "komi": run.request.komi, "board_size": run.request.board_size,
            "games": 0, "decisions": 0, "buckets": [0,0,0,0], "examples": [], "win_examples": [], "analysed": []
        }));
        group["games"] = json!(group["games"].as_u64().unwrap() + 1);
        group["analysed"].as_array_mut().unwrap().push(json!({
            "game_id": game_id, "run_id": run_id, "title": title, "date": game.game_date,
            "colour": colour, "result": game.result, "complete": run.complete(), "positions": run.results.len(), "total": run.total()
        }));
        for decision in run.decisions() {
            if decision["player"] != json!(colour) { continue; }
            let Some(loss) = decision["estimated_loss"].as_f64().filter(|v| v.is_finite()) else { continue; };
            group["decisions"] = json!(group["decisions"].as_u64().unwrap() + 1);
            let bucket = loss_bucket(loss);
            group["buckets"][bucket] = json!(group["buckets"][bucket].as_u64().unwrap() + 1);
            let mut example = decision.clone();
            example["game_id"] = json!(game_id);
            example["run_id"] = json!(run_id);
            example["title"] = json!(title);
            example["date"] = json!(game.game_date);
            example["result"] = json!(game.result);
            example["colour"] = json!(colour);
            example["loss"] = json!(loss);
            for (list, key, threshold) in [("examples", "loss", 1.0), ("win_examples", "win_loss", 5.0)] {
                if example[key].as_f64().unwrap_or(0.0) >= threshold {
                    let examples = group[list].as_array_mut().unwrap();
                    examples.push(example.clone());
                    examples.sort_by(|a,b| b[key].as_f64().unwrap().total_cmp(&a[key].as_f64().unwrap())
                        .then_with(|| a["move"].as_u64().cmp(&b["move"].as_u64())));
                    examples.truncate(20);
                }
            }
        }
    }
    Ok(json!({"needs_names": false, "total": total, "identified": identified,
        "selected": games.len(), "complete": complete, "partial": partial,
        "pending": pending, "groups": groups.into_values().collect::<Vec<_>>() }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AnalysisRequest, AnalysisResult, game_analysis::Provenance};

    #[test]
    fn report_reuses_saved_results_deduplicates_games_and_only_counts_me() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let project = crate::project_manager::ProjectManager::new().create("Personal", temp.path().join("project"))?;
        let path = temp.path().join("analysis.sqlite3");
        assert_eq!(build(&project, &path)?["needs_names"], true);
        let record = crate::extract_main_variation(&crate::parse_collection(
            b"(;SZ[19]PB[my_online_name]PW[opponent]DT[2026-10-28];B[aa];W[bb])")?)?;
        let mut importer = project.importer()?;
        importer.import_record("Test", "1", "one", &record)?;
        importer.import_record("Test", "1", "duplicate", &record)?;
        drop(importer);
        let db = crate::database::open(&project.database_root())?;
        let source: i64 = db.query_row("SELECT id FROM sources WHERE name='Test'", [], |r| r.get(0))?;
        let game_id: i64 = db.query_row("SELECT id FROM games", [], |r| r.get(0))?;
        drop(db);
        let directory = project.player_directory()?;
        let me = directory.save_my_names(None, "Me", &[(source, "my_online_name".into())])?;
        let before = build(&project, &path)?;
        assert_eq!(before["selected"], 1);
        assert_eq!(before["pending"].as_array().unwrap().len(), 1);
        assert!(!path.exists()); // Viewing a report never starts or creates an analysis.
        let states = project.game_store()?.positions(game_id)?;
        let position = analysis_position_from_states(&states, states.len()-1)?;
        let store = AnalysisStore::open(&path)?;
        let mut run = store.restore(GameAnalysis {
            run_id: "test-run".into(), title: "Test".into(), focus_turn: None,
            request: AnalysisRequest { id: "test".into(), board_size: position.board_size,
                initial_stones: position.initial_stones.clone(), initial_player: position.initial_player,
                moves: position.moves.clone(), rules: "japanese".into(), komi: 6.5, max_visits: 10 },
            provenance: Provenance { policy: 1, bermuda_version: "test".into(), executable: "missing".into(),
                engine_sha256: "engine".into(), network: "missing".into(), network_sha256: "network".into(),
                config: "missing".into(), config_sha256: "config".into(), score_perspective: "Black".into(), created_at: 1 },
            results: vec![], refinements: vec![],
        })?;
        for (turn, score) in [0.0, -5.0, 1.0].into_iter().enumerate() {
            let result = AnalysisResult { id: "test".into(), turn_number: turn as u32,
                current_player: if turn % 2 == 0 { Colour::Black } else { Colour::White },
                score_lead: score, win_rate: if turn == 0 { 0.7 } else { 0.5 }, visits: 10, candidates: vec![], is_during_search: false, warnings: vec![] };
            store.save_position(&run, &result)?;
            run.results.push(result);
            if turn == 1 {
                let partial = build(&project, &path)?;
                assert_eq!(partial["partial"], 1);
                assert_eq!(partial["groups"][0]["decisions"], 1);
            }
        }
        let report = build(&project, &path)?;
        assert_eq!(report["complete"], 1);
        assert_eq!(report["groups"][0]["decisions"], 1); // Opponent's 6-point loss is excluded.
        assert_eq!(report["groups"][0]["examples"][0]["loss"], 5.0);
        assert_eq!(report["groups"][0]["examples"][0]["move"], 1);
        assert_eq!(report["groups"][0]["win_examples"].as_array().unwrap().len(), 1);
        assert!((report["groups"][0]["win_examples"][0]["win_loss"].as_f64().unwrap() - 20.0).abs() < 1e-10);
        assert_eq!(report["groups"][0]["analysed"][0]["colour"], "Black");
        // New imports appear on refresh without discarding the old saved result.
        let later = crate::extract_main_variation(&crate::parse_collection(
            b"(;SZ[19]PB[opponent]PW[my_online_name]DT[2026-10-29];B[cc];W[dd])")?)?;
        project.importer()?.import_record("Test", "1", "later", &later)?;
        let refreshed = build(&project, &path)?;
        assert_eq!(refreshed["selected"], 2);
        assert_eq!(refreshed["complete"], 1);
        assert_eq!(refreshed["pending"].as_array().unwrap().len(), 1);
        let db = crate::database::open(&project.database_root())?;
        db.execute("UPDATE game_metadata SET black_player_id=NULL, white_player_id=?1
            WHERE game_source_id IN (SELECT id FROM game_sources WHERE game_id=?2)", rusqlite::params![me, game_id])?;
        assert_eq!(build(&project, &path)?["groups"][0]["examples"][0]["loss"], 6.0);
        db.execute("UPDATE game_metadata SET black_player_id=?1, white_player_id=NULL
            WHERE game_source_id IN (SELECT id FROM game_sources WHERE game_id=?2)", rusqlite::params![me, game_id])?;
        drop(db);
        let mut other_settings = run.clone();
        other_settings.request.max_visits = 100;
        assert_ne!(settings_key(&run)?, settings_key(&other_settings)?);
        // Linking both colours must not silently report both as the player.
        directory.save_my_names(Some(me), "Me", &[(source, "opponent".into())])?;
        assert_eq!(build(&project, &path)?["identified"], 0);
        Ok(())
    }
    #[test]
    fn estimated_gain_is_retained_in_low_loss_bucket() {
        assert_eq!([-2.0, 0.0, 0.99, 1.0, 2.99, 3.0, 4.99, 5.0].map(loss_bucket), [0,0,0,1,1,2,2,3]);
    }
}
