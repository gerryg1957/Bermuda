//! Persistent, one-game analysis. A single request is active at a time.
use std::{fs::File, io::Read, path::Path, sync::{atomic::{AtomicBool, Ordering}, mpsc::TryRecvError}, thread, time::{Duration, Instant, SystemTime, UNIX_EPOCH}};
use anyhow::{Context, Result, bail};
use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use crate::{AnalysisRequest, AnalysisResult, AnalysisOutcome, Colour, KataGoConfiguration, KataGoWorker, KataGoWorkerEvent};

const POLICY: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Provenance {
    pub policy: u32,
    pub bermuda_version: String,
    pub executable: String,
    pub engine_sha256: String,
    pub network: String,
    pub network_sha256: String,
    pub config: String,
    pub config_sha256: String,
    pub score_perspective: String,
    pub created_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameAnalysis {
    pub run_id: String,
    pub title: String,
    pub request: AnalysisRequest,
    pub provenance: Provenance,
    pub results: Vec<AnalysisResult>,
    #[serde(default)]
    pub refinements: Vec<CandidateRefinement>,
    /// None is a complete-game run; Some searches only this recorded position.
    #[serde(default)]
    pub focus_turn: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CandidateRefinement {
    pub turn: u32,
    pub candidate: crate::katago::AnalysisCandidate,
    pub provenance: Provenance,
}

impl GameAnalysis {
    pub fn total(&self) -> usize { if self.focus_turn.is_some() { 1 } else { self.request.moves.len() + 1 } }
    pub fn complete(&self) -> bool { self.results.len() == self.total() }
    fn next_turn(&self) -> usize { self.focus_turn.map(|t| t as usize).unwrap_or(self.results.len()) }

    /// Consecutive position estimates, signed for the player who moved.
    /// Negative changes are retained: these are estimates, not verdicts.
    pub fn decisions(&self) -> Vec<serde_json::Value> {
        self.results.windows(2).filter_map(|pair| {
            let before = &pair[0]; let after = &pair[1];
            if after.turn_number != before.turn_number + 1 { return None; }
            let mv = self.request.moves.get(before.turn_number as usize)?;
            let loss = (before.score_lead - after.score_lead) * if mv.colour == Colour::Black { 1.0 } else { -1.0 };
            let sign = if mv.colour == Colour::Black { 1.0 } else { -1.0 };
            let win_before = if mv.colour == Colour::Black { before.win_rate } else { 1.0 - before.win_rate };
            let win_after = if mv.colour == Colour::Black { after.win_rate } else { 1.0 - after.win_rate };
            Some(serde_json::json!({"move": after.turn_number, "player": mv.colour,
                "estimated_loss": loss, "actual_move": mv.vertex,
                "score_before": before.score_lead * sign, "score_after": after.score_lead * sign,
                "win_before": win_before, "win_after": win_after,
                "win_loss": (win_before - win_after) * 100.0,
                "already_unlikely": win_before <= 0.05}))
        }).collect()
    }
}

fn digest(bytes: &[u8]) -> String { Sha256::digest(bytes).iter().map(|b| format!("{b:02x}")).collect() }
fn file_digest(path: &Path, stop: &AtomicBool) -> Result<String> {
    let mut f = File::open(path).with_context(|| format!("reading {}", path.display()))?;
    let mut hash = Sha256::new(); let mut buffer = [0u8; 65536];
    loop {
        if stop.load(Ordering::Relaxed) { bail!("Analysis paused while identifying engine files; start again to resume"); }
        let n = f.read(&mut buffer)?; if n == 0 { break; } hash.update(&buffer[..n]);
    }
    Ok(hash.finalize().iter().map(|b| format!("{b:02x}")).collect())
}

pub fn prepare(request: AnalysisRequest, title: String, config: &KataGoConfiguration, stop: &AtomicBool) -> Result<GameAnalysis> {
    if request.max_visits == 0 || !request.komi.is_finite() { bail!("Invalid analysis settings"); }
    let provenance = Provenance {
        policy: POLICY, bermuda_version: env!("CARGO_PKG_VERSION").into(),
        executable: config.executable.display().to_string(), engine_sha256: file_digest(&config.executable, stop)?,
        network: config.model.display().to_string(), network_sha256: file_digest(&config.model, stop)?,
        config: config.config.display().to_string(), config_sha256: file_digest(&config.config, stop)?,
        score_perspective: "Black; reportAnalysisWinratesAs=BLACK".into(),
        created_at: SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs(),
    };
    prepare_with_provenance(request, title, provenance)
}

pub(crate) fn prepare_with_provenance(mut request: AnalysisRequest, title: String, provenance: Provenance) -> Result<GameAnalysis> {
    if request.max_visits == 0 || !request.komi.is_finite() { bail!("Invalid analysis settings"); }
    request.id.clear();
    let key = serde_json::to_vec(&(&request, POLICY, &provenance.engine_sha256, &provenance.network_sha256, &provenance.config_sha256))?;
    Ok(GameAnalysis { run_id: digest(&key), title, request, provenance, results: Vec::new(), refinements: Vec::new(), focus_turn: None })
}

pub fn prepare_position(request: AnalysisRequest, title: String, config: &KataGoConfiguration,
    stop: &AtomicBool, turn: u32) -> Result<GameAnalysis> {
    if turn as usize > request.moves.len() { bail!("Invalid analysis position"); }
    let mut run = prepare(request, title, config, stop)?;
    // Keep focused runs distinct from whole-game checkpoints, including turn 0.
    run.run_id = digest(&serde_json::to_vec(&("position", &run.run_id, turn))?);
    run.focus_turn = Some(turn);
    Ok(run)
}

pub struct AnalysisStore { db: Connection }
impl AnalysisStore {
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() { std::fs::create_dir_all(parent)?; }
        let db = Connection::open(path)?;
        db.busy_timeout(Duration::from_secs(5))?;
        db.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL;
          CREATE TABLE IF NOT EXISTS analysis_runs (id TEXT PRIMARY KEY, header TEXT NOT NULL);
          CREATE TABLE IF NOT EXISTS analysis_positions (run_id TEXT NOT NULL, turn INTEGER NOT NULL, result TEXT NOT NULL, PRIMARY KEY(run_id,turn));
          CREATE TABLE IF NOT EXISTS analysis_candidates (run_id TEXT NOT NULL, turn INTEGER NOT NULL, vertex TEXT NOT NULL, result TEXT NOT NULL, PRIMARY KEY(run_id,turn,vertex));")?;
        Ok(Self { db })
    }
    pub fn restore(&self, mut run: GameAnalysis) -> Result<GameAnalysis> {
        self.db.execute("INSERT OR IGNORE INTO analysis_runs VALUES (?1,?2)", params![run.run_id, serde_json::to_string(&run)?])?;
        let header: String = self.db.query_row("SELECT header FROM analysis_runs WHERE id=?1", [&run.run_id], |r| r.get(0))?;
        let saved: GameAnalysis = serde_json::from_str(&header)?;
        // Keep original provenance when resuming the exact same input and settings.
        run.provenance = saved.provenance;
        run.results.clear();
        run.refinements.clear();
        let mut query = self.db.prepare("SELECT result FROM analysis_positions WHERE run_id=?1 ORDER BY turn")?;
        let rows = query.query_map([&run.run_id], |r| r.get::<_, String>(0))?;
        for row in rows { run.results.push(serde_json::from_str(&row?)?); }
        for (turn, result) in run.results.iter().enumerate() {
            if run.focus_turn.is_some() && turn > 0 { bail!("Invalid saved single-position analysis"); }
            validate_result(&run.request, run.focus_turn.map(|t| t as usize).unwrap_or(turn), result)?;
        }
        let mut query = self.db.prepare("SELECT result FROM analysis_candidates WHERE run_id=?1 ORDER BY turn,vertex")?;
        let rows = query.query_map([&run.run_id], |r| r.get::<_, String>(0))?;
        for row in rows { run.refinements.push(serde_json::from_str(&row?)?); }
        Ok(run)
    }
    pub fn load(&self, run_id: &str) -> Result<GameAnalysis> {
        let header: String = self.db.query_row("SELECT header FROM analysis_runs WHERE id=?1", [run_id], |r| r.get(0))?;
        self.restore(serde_json::from_str(&header)?)
    }
    /// Find the newest non-empty report for the exact game history and scope.
    /// Reading saved evidence must not require installed engine files.
    pub fn latest_for_position(&self, position: &crate::katago::AnalysisPosition,
        focus_turn: Option<u32>) -> Result<Option<GameAnalysis>> {
        let mut query = self.db.prepare("SELECT header FROM analysis_runs r
            WHERE EXISTS (SELECT 1 FROM analysis_positions p WHERE p.run_id=r.id)")?;
        let rows = query.query_map([], |r| r.get::<_, String>(0))?;
        let mut latest: Option<GameAnalysis> = None;
        for row in rows {
            let run: GameAnalysis = serde_json::from_str(&row?)?;
            let request = &run.request;
            if run.focus_turn != focus_turn || request.board_size != position.board_size
                || request.initial_player != position.initial_player
                || request.initial_stones != position.initial_stones || request.moves != position.moves {
                continue;
            }
            if latest.as_ref().is_none_or(|previous|
                (run.provenance.created_at, &run.run_id) > (previous.provenance.created_at, &previous.run_id)) {
                latest = Some(run);
            }
        }
        latest.map(|run| self.load(&run.run_id)).transpose()
    }
    /// Scan headers once for a bounded report selection; load position data separately.
    pub fn latest_whole_game_ids(&self, positions: &[crate::katago::AnalysisPosition]) -> Result<Vec<Option<String>>> {
        let mut latest: Vec<Option<(u64, String)>> = vec![None; positions.len()];
        let mut query = self.db.prepare("SELECT header FROM analysis_runs r
            WHERE EXISTS (SELECT 1 FROM analysis_positions p WHERE p.run_id=r.id)")?;
        let rows = query.query_map([], |r| r.get::<_, String>(0))?;
        for row in rows {
            let run: GameAnalysis = serde_json::from_str(&row?)?;
            if run.focus_turn.is_some() { continue; }
            for (index, position) in positions.iter().enumerate() {
                if run.request.board_size == position.board_size
                    && run.request.initial_player == position.initial_player
                    && run.request.initial_stones == position.initial_stones
                    && run.request.moves == position.moves {
                    let key = (run.provenance.created_at, run.run_id.clone());
                    if latest[index].as_ref().is_none_or(|old| key > *old) { latest[index] = Some(key); }
                }
            }
        }
        Ok(latest.into_iter().map(|item| item.map(|(_, id)| id)).collect())
    }

    fn save_candidate(&self, run: &GameAnalysis, refinement: &CandidateRefinement) -> Result<()> {
        self.db.execute("INSERT OR REPLACE INTO analysis_candidates VALUES (?1,?2,?3,?4)",
            params![run.run_id, refinement.turn, serde_json::to_string(&refinement.candidate.vertex)?,
                serde_json::to_string(refinement)?])?;
        Ok(())
    }
    pub fn save_position(&self, run: &GameAnalysis, result: &AnalysisResult) -> Result<()> {
        if run.complete() { bail!("Analysis is already complete"); }
        validate_result(&run.request, run.next_turn(), result)?;
        self.db.execute("INSERT INTO analysis_positions VALUES (?1,?2,?3)", params![run.run_id, result.turn_number, serde_json::to_string(result)?])?;
        Ok(())
    }
}

/// Search AFTER the chosen move. Preserve all earlier moves for ko/history,
/// and keep the whole-game estimates separate from this focused investigation.
fn candidate_request(run: &GameAnalysis, turn: u32, vertex: crate::katago::AnalysisVertex, visits: u64) -> Result<AnalysisRequest> {
    let original = run.results.iter().find(|r| r.turn_number == turn).context("This position has not been analysed")?;
    let candidate = original.candidates.iter().find(|c| c.vertex == vertex);
    let previous_visits = run.refinements.iter().find(|r| r.turn == turn && r.candidate.vertex == vertex)
        .map(|r| r.candidate.visits).unwrap_or(candidate.map(|c| c.visits).unwrap_or(0));
    if visits <= previous_visits || !(10..=100_000).contains(&visits) {
        bail!("Choose more than {previous_visits} visits, up to 100000");
    }
    let mut request = run.request.clone();
    request.id = format!("candidate-{}-{turn}", run.run_id);
    request.moves.truncate(turn as usize);
    request.moves.push(crate::katago::AnalysisMove { colour: original.current_player, vertex });
    request.max_visits = visits;
    Ok(request)
}

fn candidate_from_reply(turn: u32, vertex: crate::katago::AnalysisVertex, result: &AnalysisResult,
    provenance: Provenance) -> CandidateRefinement {
    let mut line = vec![vertex];
    if let Some(reply) = result.candidates.iter().max_by_key(|c| c.visits) {
        if reply.principal_variation.is_empty() { line.push(reply.vertex); }
        else { line.extend_from_slice(&reply.principal_variation); }
    }
    CandidateRefinement { turn, provenance, candidate: crate::katago::AnalysisCandidate {
        vertex, visits: result.visits, score_lead: result.score_lead, win_rate: result.win_rate,
        principal_variation: line,
    }}
}

pub fn analyse_candidate(run_id: &str, turn: u32, vertex: crate::katago::AnalysisVertex,
    visits: u64, config: &KataGoConfiguration, store: &AnalysisStore, stop: &AtomicBool) -> Result<GameAnalysis> {
    let mut run = store.load(run_id)?;
    let request = candidate_request(&run, turn, vertex, visits)?;
    // Refuse to silently mix results from changed engine/network/config files.
    let identified = match run.focus_turn {
        Some(turn) => prepare_position(run.request.clone(), run.title.clone(), config, stop, turn)?,
        None => prepare(run.request.clone(), run.title.clone(), config, stop)?,
    };
    if identified.run_id != run.run_id {
        bail!("Engine, network or configuration changed. Reanalyse the game before deepening a candidate.");
    }
    if stop.load(Ordering::Relaxed) { bail!("Candidate analysis stopped; previous results retained"); }
    let worker = KataGoWorker::start(config)?;
    worker.submit_latest(request.clone())?;
    let started = Instant::now();
    let result = loop {
        if stop.load(Ordering::Relaxed) {
            worker.cancel_all()?;
            bail!("Candidate analysis stopped; previous results retained");
        }
        if started.elapsed() > Duration::from_secs(1800) { bail!("Candidate analysis timed out; previous results retained"); }
        match worker.try_recv() {
            Ok(KataGoWorkerEvent::Analysis { request_id, outcome }) => {
                if request_id != request.id { bail!("Unexpected candidate analysis response"); }
                match outcome {
                    AnalysisOutcome::Complete(result) => break result,
                    AnalysisOutcome::Terminated => bail!("Candidate analysis stopped; previous results retained"),
                }
            }
            Ok(KataGoWorkerEvent::Error { message, .. }) => bail!("{message}"),
            Err(TryRecvError::Disconnected) => bail!("KataGo worker disconnected"),
            Err(TryRecvError::Empty) => thread::sleep(Duration::from_millis(50)),
        }
    };
    validate_result(&request, request.moves.len(), &result)?;
    if stop.load(Ordering::Relaxed) { bail!("Candidate analysis stopped; previous results retained"); }
    let refinement = candidate_from_reply(turn, vertex, &result, identified.provenance);
    store.save_candidate(&run, &refinement)?;
    run.refinements.retain(|r| !(r.turn == turn && r.candidate.vertex == vertex));
    run.refinements.push(refinement);
    Ok(run)
}

fn validate_result(request: &AnalysisRequest, turn: usize, result: &AnalysisResult) -> Result<()> {
    if turn > request.moves.len() || result.turn_number as usize != turn || result.is_during_search
       || !result.score_lead.is_finite() || !result.win_rate.is_finite() || !(0.0..=1.0).contains(&result.win_rate) {
        bail!("Invalid or out-of-order saved analysis at position {turn}");
    }
    let expected = request.moves.get(turn).map(|m| m.colour)
        .or_else(|| request.moves.last().map(|m| m.colour.opponent())).unwrap_or(request.initial_player);
    if result.current_player != expected { bail!("KataGo side-to-move mismatch at position {turn}"); }
    Ok(())
}

/// Callback runs on the background thread; false means the owning UI has gone.
/// Completed positions are committed before progress is reported. Resume uses
/// exactly matching game history, settings, and file-content identities.
pub fn analyse_game(mut run: GameAnalysis, config: &KataGoConfiguration, store: &AnalysisStore,
    stop: &AtomicBool, mut progress: impl FnMut(&GameAnalysis, &str) -> bool) -> Result<GameAnalysis> {
    run = store.restore(run)?;
    if !progress(&run, "Saved results loaded") || run.complete() || stop.load(Ordering::Relaxed) { return Ok(run); }
    let worker = KataGoWorker::start(config)?;
    while run.results.len() < run.total() {
        if stop.load(Ordering::Relaxed) { break; }
        let turn = run.next_turn();
        let mut request = run.request.clone();
        request.id = format!("whole-{}-{turn}", run.run_id);
        request.moves.truncate(turn);
        worker.submit_latest(request.clone())?;
        let started = Instant::now();
        let result = loop {
            if stop.load(Ordering::Relaxed) { worker.cancel_all()?; return Ok(run); }
            if started.elapsed() > Duration::from_secs(1800) { bail!("Position {turn} timed out after 30 minutes; saved positions can be resumed"); }
            match worker.try_recv() {
                Ok(KataGoWorkerEvent::Analysis { request_id, outcome }) => {
                    if request_id != request.id { bail!("Unexpected whole-game analysis response"); }
                    match outcome { AnalysisOutcome::Complete(result) => break result, AnalysisOutcome::Terminated => return Ok(run) }
                }
                Ok(KataGoWorkerEvent::Error { message, .. }) => bail!("{message}"),
                Err(TryRecvError::Disconnected) => bail!("KataGo worker disconnected"),
                Err(TryRecvError::Empty) => thread::sleep(Duration::from_millis(50)),
            }
        };
        validate_result(&run.request, turn, &result)?;
        store.save_position(&run, &result)?;
        run.results.push(result);
        if !progress(&run, "Analysing") { break; }
    }
    Ok(run)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::katago::{AnalysisMove, AnalysisVertex};
    fn fixture(dir: &Path) -> (KataGoConfiguration, AnalysisRequest) {
        for name in ["engine", "network", "config"] { std::fs::write(dir.join(name), name).unwrap(); }
        (KataGoConfiguration::new(dir.join("engine"), dir.join("network"), dir.join("config"), dir),
         AnalysisRequest { id: "test".into(), board_size: 19, rules: "japanese".into(), komi: 6.5,
             initial_stones: vec![], initial_player: Colour::Black,
             moves: vec![AnalysisMove { colour: Colour::Black, vertex: AnalysisVertex::Pass }, AnalysisMove { colour: Colour::White, vertex: AnalysisVertex::Pass }], max_visits: 10 })
    }
    fn result(turn: u32, score: f64) -> AnalysisResult {
        AnalysisResult { id: "test".into(), turn_number: turn, current_player: if turn % 2 == 0 { Colour::Black } else { Colour::White }, score_lead: score, win_rate: 0.5, visits: 10, candidates: vec![], is_during_search: false, warnings: vec![] }
    }
    fn pass_candidate() -> crate::katago::AnalysisCandidate {
        crate::katago::AnalysisCandidate { vertex: AnalysisVertex::Pass, visits: 1,
            score_lead: -2.0, win_rate: 0.4, principal_variation: vec![AnalysisVertex::Pass] }
    }
    #[test]
    fn decisions_keep_player_perspective_and_already_unlikely_context() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let (config, request) = fixture(temp.path());
        let mut run = prepare(request, "Context".into(), &config, &AtomicBool::new(false))?;
        let mut before = result(0, 20.0); before.win_rate = 0.8;
        let mut middle = result(1, 30.0); middle.win_rate = 0.99;
        let mut after = result(2, 100.0); after.win_rate = 0.999;
        run.results = vec![before, middle, after];
        let decisions = run.decisions();
        assert_eq!(decisions[0]["player"], "Black");
        assert_eq!(decisions[0]["estimated_loss"], -10.0);
        assert_eq!(decisions[1]["player"], "White");
        assert_eq!(decisions[1]["estimated_loss"], 70.0);
        assert_eq!(decisions[1]["score_before"], -30.0);
        assert_eq!(decisions[1]["score_after"], -100.0);
        assert!((decisions[1]["win_before"].as_f64().unwrap() - 0.01).abs() < 1e-10);
        assert!((decisions[1]["win_loss"].as_f64().unwrap() - 0.9).abs() < 1e-10);
        assert_eq!(decisions[1]["already_unlikely"], true);
        assert_eq!(decisions[1]["actual_move"], "Pass");
        // Late 70-point loss is not a five-percentage-point turning point.
        assert!(decisions[1]["win_loss"].as_f64().unwrap() < 5.0);
        Ok(())
    }

    #[test]
    fn saved_report_discovery_matches_history_and_scope_without_engine_files() -> Result<()> {
        let temp = tempfile::tempdir()?; let (config, request) = fixture(temp.path());
        let stop = AtomicBool::new(false);
        let store = AnalysisStore::open(&temp.path().join("analysis.sqlite3"))?;
        let mut older = prepare(request.clone(), "Game".into(), &config, &stop)?;
        older.provenance.created_at = 1;
        older = store.restore(older)?;
        store.save_position(&older, &result(0, 2.0))?;
        let mut changed = request.clone(); changed.max_visits = 100;
        let mut newer = prepare(changed, "Game".into(), &config, &stop)?;
        newer.provenance.created_at = 2;
        newer = store.restore(newer)?;
        store.save_position(&newer, &result(0, 3.0))?;
        let focused = store.restore(prepare_position(request.clone(), "Game".into(), &config, &stop, 1)?)?;
        store.save_position(&focused, &result(1, 4.0))?;
        let mut position = crate::katago::AnalysisPosition {
            board_size: request.board_size, initial_stones: request.initial_stones.clone(),
            initial_player: request.initial_player, moves: request.moves.clone(),
            current_player: Colour::Black,
        };
        std::fs::remove_file(&config.executable)?;
        std::fs::remove_file(&config.model)?;
        std::fs::remove_file(&config.config)?;
        assert_eq!(store.latest_for_position(&position, None)?.unwrap().run_id, newer.run_id);
        assert_eq!(store.latest_whole_game_ids(&[position.clone()])?, vec![Some(newer.run_id.clone())]);
        assert_eq!(store.latest_for_position(&position, Some(1))?.unwrap().run_id, focused.run_id);
        assert!(store.latest_for_position(&position, Some(0))?.is_none());
        position.moves.pop();
        assert!(store.latest_for_position(&position, None)?.is_none());
        assert_eq!(store.latest_whole_game_ids(&[position])?, vec![None]);
        Ok(())
    }
    #[test]
    fn candidate_search_keeps_history_side_to_move_and_black_perspective() -> Result<()> {
        let temp = tempfile::tempdir()?; let (config, request) = fixture(temp.path());
        let mut run = prepare(request, "Game".into(), &config, &AtomicBool::new(false))?;
        run.results = vec![result(0, 2.0), result(1, 4.0)];
        run.results[1].candidates.push(pass_candidate());
        let request = candidate_request(&run, 1, AnalysisVertex::Pass, 1000)?;
        assert_eq!(request.moves.len(), 2);
        assert_eq!(request.moves[0], run.request.moves[0]);
        assert_eq!(request.moves[1].colour, Colour::White);
        assert_eq!(request.moves[1].vertex, AnalysisVertex::Pass);
        assert_eq!(request.max_visits, 1000);
        assert_eq!(request.komi, run.request.komi);
        assert!(candidate_request(&run, 1, AnalysisVertex::Pass, 1).is_err());
        let mut reply = result(2, 6.0);
        reply.win_rate = 0.8; reply.visits = 1000;
        reply.candidates.push(pass_candidate());
        let refined = candidate_from_reply(1, AnalysisVertex::Pass, &reply, run.provenance.clone());
        assert_eq!(refined.candidate.score_lead, 6.0); // no sign flip for White's candidate
        assert_eq!(refined.candidate.win_rate, 0.8);
        assert_eq!(refined.candidate.principal_variation, vec![AnalysisVertex::Pass, AnalysisVertex::Pass]);
        Ok(())
    }
    #[cfg(unix)]
    #[test]
    fn candidate_search_persists_separately_and_preserves_results_on_cancel_or_changed_engine() -> Result<()> {
        use std::os::unix::fs::PermissionsExt;
        let temp = tempfile::tempdir()?; let (config, request) = fixture(temp.path());
        std::fs::write(&config.executable, r#"#!/usr/bin/env python3
import sys,json
for line in sys.stdin:
 r=json.loads(line)
 if r.get('action'): continue
 with open('candidate-request.json','w') as f: json.dump(r,f)
 print(json.dumps({'id':r['id'],'turnNumber':len(r['moves']),'isDuringSearch':False,'rootInfo':{'currentPlayer':'B','visits':r['maxVisits'],'scoreLead':6,'winrate':0.8},'moveInfos':[{'move':'D4','visits':r['maxVisits'],'scoreLead':6,'winrate':0.8,'pv':['D4','pass']}]}),flush=True)
"#)?;
        std::fs::set_permissions(&config.executable, std::fs::Permissions::from_mode(0o755))?;
        let stop = AtomicBool::new(false);
        let mut run = prepare(request, "Game".into(), &config, &stop)?;
        let path = temp.path().join("analysis.sqlite3");
        let store = AnalysisStore::open(&path)?;
        run = store.restore(run)?;
        let first = result(0, 2.0);
        store.save_position(&run, &first)?; run.results.push(first);
        let mut second = result(1, 4.0); second.candidates.push(pass_candidate());
        store.save_position(&run, &second)?; run.results.push(second);
        let refined = analyse_candidate(&run.run_id, 1, AnalysisVertex::Pass, 1000, &config, &store, &stop)?;
        assert_eq!(refined.results, run.results);
        assert_eq!(refined.decisions(), run.decisions());
        assert_eq!(refined.refinements[0].candidate.visits, 1000);
        assert_eq!(refined.refinements[0].candidate.score_lead, 6.0);
        assert_eq!(refined.refinements[0].candidate.principal_variation.len(), 3);
        let sent: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(temp.path().join("candidate-request.json"))?)?;
        assert_eq!(sent["moves"], serde_json::json!([["B","pass"],["W","pass"]]));
        assert_eq!(sent["maxVisits"], 1000);
        drop(store);
        let store = AnalysisStore::open(&path)?;
        assert_eq!(store.load(&run.run_id)?.refinements[0].candidate, refined.refinements[0].candidate);
        stop.store(true, Ordering::Relaxed);
        assert!(analyse_candidate(&run.run_id, 1, AnalysisVertex::Pass, 2000, &config, &store, &stop).is_err());
        stop.store(false, Ordering::Relaxed);
        std::fs::write(&config.model, "changed network")?;
        assert!(analyse_candidate(&run.run_id, 1, AnalysisVertex::Pass, 2000, &config, &store, &stop).is_err());
        assert_eq!(store.load(&run.run_id)?.refinements[0].candidate, refined.refinements[0].candidate);
        Ok(())
    }
    #[test]
    fn reopens_partial_results_and_separates_settings_and_networks() -> Result<()> {
        let temp = tempfile::tempdir()?; let (config, request) = fixture(temp.path()); let stop = AtomicBool::new(false);
        let run = prepare(request.clone(), "Game".into(), &config, &stop)?;
        let path = temp.path().join("analysis.sqlite3");
        {
            let store = AnalysisStore::open(&path)?;
            let mut run = store.restore(run.clone())?;
            store.save_position(&run, &result(0, 2.0))?; run.results.push(result(0, 2.0));
            assert!(store.save_position(&run, &result(2, 3.0)).is_err());
            store.save_position(&run, &result(1, -1.0))?;
        }
        let store = AnalysisStore::open(&path)?;
        let restored = store.restore(run.clone())?; assert_eq!(restored.results.len(), 2); assert!(!restored.complete());
        assert_eq!(restored.decisions()[0]["estimated_loss"], 3.0);
        let mut changed = request.clone(); changed.komi = 7.5;
        assert_ne!(prepare(changed, "Game".into(), &config, &stop)?.run_id, run.run_id);
        let mut changed = request.clone(); changed.max_visits = 20;
        assert_ne!(prepare(changed, "Game".into(), &config, &stop)?.run_id, run.run_id);
        std::fs::write(&config.model, "new network")?;
        let different = prepare(request, "Game".into(), &config, &stop)?;
        assert_ne!(different.run_id, run.run_id); assert!(store.restore(different)?.results.is_empty());
        Ok(())
    }
    #[test]
    fn score_changes_have_the_movers_perspective_and_keep_negative_values() -> Result<()> {
        let temp = tempfile::tempdir()?; let (config, request) = fixture(temp.path());
        let mut run = prepare(request, "Game".into(), &config, &AtomicBool::new(false))?;
        run.results = vec![result(0, 2.0), result(1, 4.0), result(2, 7.0)];
        assert_eq!(run.decisions()[0]["estimated_loss"], -2.0);
        assert_eq!(run.decisions()[1]["estimated_loss"], 3.0);
        assert!(run.complete()); Ok(())
    }
    #[cfg(unix)]
    #[test]
    fn single_position_search_saves_only_requested_turn_and_supports_deeper_candidates() -> Result<()> {
        use std::os::unix::fs::PermissionsExt;
        let temp = tempfile::tempdir()?; let (config, request) = fixture(temp.path());
        std::fs::write(&config.executable, r#"#!/usr/bin/env python3
import sys,json
for line in sys.stdin:
 r=json.loads(line)
 if r.get('action'): continue
 turn=len(r['moves'])
 player=('W' if r['moves'][-1][0]=='B' else 'B') if turn else r['initialPlayer']
 with open('position-requests.txt','a') as f: f.write(str(turn)+'\n')
 print(json.dumps({'id':r['id'],'turnNumber':turn,'isDuringSearch':False,'rootInfo':{'currentPlayer':player,'visits':r['maxVisits'],'scoreLead':3,'winrate':0.6},'moveInfos':[{'move':'pass','visits':r['maxVisits'],'scoreLead':3,'winrate':0.6,'pv':['pass']}]}),flush=True)
"#)?;
        std::fs::set_permissions(&config.executable, std::fs::Permissions::from_mode(0o755))?;
        let stop = AtomicBool::new(false);
        let store = AnalysisStore::open(&temp.path().join("analysis.sqlite3"))?;
        let whole = prepare(request.clone(), "Game".into(), &config, &stop)?;
        let single = prepare_position(request.clone(), "Game".into(), &config, &stop, 1)?;
        assert_ne!(single.run_id, whole.run_id);
        assert_ne!(single.run_id, prepare_position(request.clone(), "Game".into(), &config, &stop, 0)?.run_id);
        assert!(prepare_position(request, "Game".into(), &config, &stop, 3).is_err());
        let complete = analyse_game(single.clone(), &config, &store, &stop, |_, _| true)?;
        assert!(complete.complete());
        assert_eq!(complete.total(), 1);
        assert_eq!(complete.results.len(), 1);
        assert_eq!(complete.results[0].turn_number, 1);
        assert!(complete.decisions().is_empty());
        assert_eq!(std::fs::read_to_string(temp.path().join("position-requests.txt"))?, "1\n");
        let refined = analyse_candidate(&single.run_id, 1, AnalysisVertex::Pass, 100, &config, &store, &stop)?;
        assert_eq!(refined.refinements[0].turn, 1);
        assert_eq!(refined.refinements[0].candidate.visits, 100);
        assert_eq!(refined.results, complete.results);
        assert_eq!(std::fs::read_to_string(temp.path().join("position-requests.txt"))?, "1\n2\n");
        // A player's idea need not appear in the engine's candidate list.
        let idea = AnalysisVertex::Point(crate::katago::AnalysisPoint { x: 3, y: 15 });
        let investigated = analyse_candidate(&single.run_id, 1, idea, 100, &config, &store, &stop)?;
        let own = investigated.refinements.iter().find(|r| r.candidate.vertex == idea).unwrap();
        assert_eq!(own.candidate.principal_variation[0], idea);
        assert_eq!(own.candidate.visits, 100);
        assert_eq!(investigated.results, complete.results);
        assert_eq!(std::fs::read_to_string(temp.path().join("position-requests.txt"))?, "1\n2\n2\n");
        // Reload the completed position without starting the engine again.
        std::fs::remove_file(&config.executable)?;
        let restored = analyse_game(single, &config, &store, &stop, |_, _| true)?;
        assert!(restored.complete());
        assert_eq!(restored.refinements[0].candidate, refined.refinements[0].candidate);
        assert!(restored.refinements.iter().any(|r| r.candidate.vertex == idea));
        assert!(store.restore(whole)?.results.is_empty());
        Ok(())
    }
    #[cfg(unix)]
    #[test]
    fn engine_run_resumes_after_pause_and_reuses_complete_results_without_engine() -> Result<()> {
        use std::os::unix::fs::PermissionsExt;
        let temp = tempfile::tempdir()?; let (config, request) = fixture(temp.path());
        std::fs::write(&config.executable, r#"#!/usr/bin/env python3
import sys,json
for line in sys.stdin:
 r=json.loads(line)
 if r.get('action'): continue
 turn=len(r['moves'])
 with open('requests.txt','a') as f: f.write(str(turn)+'\n')
 print(json.dumps({'id':r['id'],'turnNumber':turn,'isDuringSearch':False,'rootInfo':{'currentPlayer':'B' if turn%2==0 else 'W','visits':10,'scoreLead':2-turn,'winrate':0.5},'moveInfos':[{'move':'pass','visits':10,'scoreLead':2-turn,'winrate':0.5,'pv':['pass']}]}),flush=True)
"#)?;
        std::fs::set_permissions(&config.executable, std::fs::Permissions::from_mode(0o755))?;
        let stop = AtomicBool::new(false);
        let run = prepare(request, "Game".into(), &config, &stop)?;
        let store = AnalysisStore::open(&temp.path().join("analysis.sqlite3"))?;
        let partial = analyse_game(run.clone(), &config, &store, &stop, |r, _| { if r.results.len() == 1 { stop.store(true, Ordering::Relaxed); } true })?;
        assert_eq!(partial.results.len(), 1);
        stop.store(false, Ordering::Relaxed);
        let complete = analyse_game(run.clone(), &config, &store, &stop, |_, _| true)?;
        assert!(complete.complete());
        assert_eq!(std::fs::read_to_string(temp.path().join("requests.txt"))?, "0\n1\n2\n");
        std::fs::remove_file(&config.executable)?;
        assert!(analyse_game(run, &config, &store, &stop, |_, _| true)?.complete());
        Ok(())
    }
}
