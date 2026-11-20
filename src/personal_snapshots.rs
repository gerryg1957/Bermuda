//! Fixed game groups and append-only copies of the evidence used by a report.
//! This database belongs to the personal project, not its disposable cache.
use crate::{
    Colour,
    game_analysis::{AnalysisStore, GameAnalysis},
    project::Project,
};
use anyhow::{Context, Result, ensure};
use rusqlite::{Connection, OpenFlags, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::BTreeSet,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Member {
    pub game_id: i64,
    pub black: String,
    pub white: String,
    pub date: String,
    pub result: String,
    pub colour: Option<Colour>,
    pub identity_status: String,
    /// A main-line copy, independent of the source database and name directory.
    #[serde(default)]
    pub sgf: String,
}
impl Member {
    fn metadata(&self) -> Value {
        json!({"game_id":self.game_id, "title":format!("{} — {}",self.black,self.white),
            "date":self.date,"result":self.result,"colour":self.colour})
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameGroup {
    pub name: String,
    pub created_at: u64,
    pub members: Vec<Member>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    pub schema: u32,
    pub group_id: i64,
    pub label: String,
    pub created_at: u64,
    pub group: GameGroup,
    /// One run per member, including excluded members when analysis exists.
    pub runs: Vec<Option<GameAnalysis>>,
    /// Personal decision rows are frozen too, so later scoring policy changes
    /// cannot reinterpret the printed appendix.
    pub decisions: Vec<Vec<Value>>,
    /// Frozen presentation data, rather than a summary recomputed by future code.
    pub report: Value,
}
// Derive graph points from frozen bar-chart counts; never re-score old decisions.
fn snapshot_trend_series(snapshot: &Snapshot) -> Result<Vec<Value>> {
    let mut series = Vec::new();
    for group in snapshot.report["groups"].as_array().into_iter().flatten() {
        let analysed = group["analysed"].as_array().cloned().unwrap_or_default();
        let mut keys = BTreeSet::new();
        let mut dates = Vec::new();
        let mut ids = Vec::new();
        let mut undated = 0;
        for game in &analysed {
            let run = snapshot.runs.iter().flatten()
                .find(|run| Some(run.run_id.as_str()) == game["run_id"].as_str());
            if let Some(run) = run { keys.insert(if snapshot.report.get("period").is_some() { crate::personal_report::comparable_settings_key(run)? } else { crate::personal_report::settings_key(run)? }); }
            else { keys.insert("unknown".to_owned()); }
            if let Some(id) = game["game_id"].as_i64() { ids.push(id); }
            let date = game["date"].as_str().unwrap_or("");
            if valid_trend_date(date) { dates.push(date.to_owned()); } else { undated += 1; }
        }
        dates.sort();
        // Unknown provenance is never silently compared to another report.
        let key = if keys.len() == 1 && !keys.contains("unknown") {
            keys.into_iter().next().unwrap()
        } else { continue; };
        series.push(json!({"key":key,"visits":group["visits"],"rules":group["rules"],
            "komi":group["komi"],"board_size":group["board_size"],
            "decisions":group["decisions"],"buckets":group["buckets"],"games":analysed.len(),
            "complete":analysed.iter().filter(|g| g["complete"] == true).count(),
            "game_ids":ids,"from":dates.first(),"to":dates.last(),"undated":undated}));
    }
    Ok(series)
}

fn valid_trend_date(date: &str) -> bool {
    let bytes = date.as_bytes();
    if bytes.len() != 10 || bytes[4] != b'-' || bytes[7] != b'-'
        || !bytes.iter().enumerate().all(|(i, b)| i == 4 || i == 7 || b.is_ascii_digit()) { return false; }
    let year: u32 = date[..4].parse().unwrap_or(0);
    let month: usize = date[5..7].parse().unwrap_or(0);
    let day: u32 = date[8..].parse().unwrap_or(0);
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let days = [0,31,if leap {29} else {28},31,30,31,30,31,31,30,31,30,31];
    year > 0 && month > 0 && month <= 12 && day > 0 && day <= days[month]
}

fn now() -> Result<u64> {
    Ok(SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs())
}
fn name(value: &str) -> Result<String> {
    let value = value.trim();
    ensure!(
        !value.is_empty() && value.chars().count() <= 120,
        "Use a name of 1–120 characters"
    );
    Ok(value.to_owned())
}

/// All canonical games, including unknown/ambiguous identities. No latest-20 cap.
pub fn available_games(project: &Project) -> Result<Vec<Member>> {
    let me = project
        .player_directory()?
        .personal_player_id()?
        .unwrap_or(-1);
    let db = Connection::open_with_flags(
        project.database_root().join("metadata.sqlite3"),
        OpenFlags::SQLITE_OPEN_READ_ONLY,
    )?;
    let mut query = db.prepare(
        "SELECT g.id, MAX(COALESCE(m.black_player_id=?1,0)),
        MAX(COALESCE(m.white_player_id=?1,0)), MAX(COALESCE(m.played_date_sort,'')) AS played
        FROM games g LEFT JOIN game_sources s ON s.game_id=g.id
        LEFT JOIN game_metadata m ON m.game_source_id=s.id
        GROUP BY g.id ORDER BY played DESC,g.id DESC",
    )?;
    let rows = query.query_map([me], |r| {
        Ok((
            r.get::<_, i64>(0)?,
            r.get::<_, i64>(1)?,
            r.get::<_, i64>(2)?,
        ))
    })?;
    let catalogue = project.catalogue()?;
    let mut members = Vec::new();
    for row in rows {
        let (id, black, white) = row?;
        let game = catalogue.get(id)?;
        let (colour, status) = match (black, white) {
            (1, 0) => (Some(Colour::Black), "Identified"),
            (0, 1) => (Some(Colour::White), "Identified"),
            (1, 1) => (None, "Ambiguous colour"),
            _ => (None, "Name not identified"),
        };
        members.push(Member {
            game_id: id,
            black: game.black_player_display.unwrap_or_else(|| "Black".into()),
            white: game.white_player_display.unwrap_or_else(|| "White".into()),
            date: game.game_date.unwrap_or_default(),
            result: game.result.unwrap_or_default(),
            colour,
            identity_status: status.into(),
            sgf: String::new(),
        });
    }
    Ok(members)
}

pub struct SnapshotStore {
    db: Connection,
}
impl SnapshotStore {
    pub fn open(project: &Project) -> Result<Self> {
        let db = Connection::open(project.database_root().join("personal-snapshots.sqlite3"))?;
        db.busy_timeout(std::time::Duration::from_secs(5))?;
        db.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL;
            CREATE TABLE IF NOT EXISTS report_preferences (key TEXT PRIMARY KEY, value TEXT NOT NULL);
            CREATE TABLE IF NOT EXISTS removed_groups (id INTEGER PRIMARY KEY);
            CREATE TABLE IF NOT EXISTS removed_reports (id INTEGER PRIMARY KEY);
            CREATE TABLE IF NOT EXISTS game_groups (id INTEGER PRIMARY KEY, body TEXT NOT NULL);
            CREATE TABLE IF NOT EXISTS report_snapshots (id INTEGER PRIMARY KEY, group_id INTEGER NOT NULL, body TEXT NOT NULL);
            CREATE TRIGGER IF NOT EXISTS immutable_groups BEFORE UPDATE ON game_groups
                BEGIN SELECT RAISE(ABORT,'Saved groups are immutable'); END;
            CREATE TRIGGER IF NOT EXISTS immutable_snapshots BEFORE UPDATE ON report_snapshots
                BEGIN SELECT RAISE(ABORT,'Saved snapshots are immutable'); END;")?;
        Ok(Self { db })
    }
    pub fn create_group(&self, project: &Project, title: &str, ids: &[i64]) -> Result<i64> {
        let title = name(title)?;
        let ids: BTreeSet<_> = ids.iter().copied().collect();
        ensure!(!ids.is_empty(), "Select at least one game");
        let mut members: Vec<_> = available_games(project)?
            .into_iter()
            .filter(|m| ids.contains(&m.game_id))
            .collect();
        ensure!(
            members.len() == ids.len(),
            "Some selected games no longer exist; refresh the list"
        );
        let games = project.game_store()?;
        let catalogue = project.catalogue()?;
        for member in &mut members {
            let mut record = games.load(member.game_id)?;
            let row = catalogue.get(member.game_id)?;
            record.metadata.black_player = Some(member.black.clone());
            record.metadata.white_player = Some(member.white.clone());
            record.metadata.date = row.game_date;
            record.metadata.result = row.result;
            record.metadata.event = row.event;
            record.metadata.komi = row.komi;
            member.sgf = crate::write_game_record_sgf(&record)?;
            // Reject a broken source before committing any of the group.
            crate::replay_positions(&record)?;
        }
        let group = GameGroup {
            name: title,
            created_at: now()?,
            members,
        };
        self.db.execute(
            "INSERT INTO game_groups(body) VALUES(?1)",
            [serde_json::to_string(&group)?],
        )?;
        Ok(self.db.last_insert_rowid())
    }
    pub fn group(&self, id: i64) -> Result<GameGroup> {
        let body: String =
            self.db
                .query_row("SELECT body FROM game_groups WHERE id=?1 AND id NOT IN (SELECT id FROM removed_groups)", [id], |r| {
                    r.get(0)
                })?;
        Ok(serde_json::from_str(&body)?)
    }
    pub fn capture(&self, group_id: i64, label: &str, analysis_path: &Path) -> Result<i64> {
        self.capture_period(group_id, label, analysis_path, None)
    }
    fn capture_period(&self, group_id: i64, label: &str, analysis_path: &Path, period: Option<Value>) -> Result<i64> {
        let label = name(label)?;
        let group = self.group(group_id)?;
        let mut positions = Vec::new();
        for member in &group.members {
            let record =
                crate::extract_main_variation(&crate::parse_collection(member.sgf.as_bytes())?)?;
            let states = crate::replay_positions(&record)?;
            positions.push(crate::analysis_position_from_states(
                &states,
                states.len() - 1,
            )?);
        }
        let created_at = now()?;
        let runs = AnalysisStore::freeze_latest(analysis_path, &positions)?;
        let evidence: Vec<_> = group
            .members
            .iter()
            .zip(&runs)
            .filter(|(m, _)| m.colour.is_some())
            .map(|(m, r)| (m.metadata(), r.clone()))
            .collect();
        let mut report = crate::personal_report::summarise_with_komi_pooling(
            &evidence,
            group.members.len() as i64,
            evidence.len() as i64,
            period.is_some(),
        )?;
        report["excluded"] = json!(group.members.len() - evidence.len());
        if let Some(period) = period { report["period"] = period; }
        let decisions = group
            .members
            .iter()
            .zip(&runs)
            .map(|(m, run)| {
                run.as_ref()
                    .map(|r| {
                        r.decisions()
                            .into_iter()
                            .filter(|d| m.colour.is_some() && d["player"] == json!(m.colour))
                            .collect()
                    })
                    .unwrap_or_default()
            })
            .collect();
        let snapshot = Snapshot {
            schema: 1,
            group_id,
            label,
            created_at,
            group,
            runs,
            decisions,
            report,
        };
        self.db.execute(
            "INSERT INTO report_snapshots(group_id,body) VALUES(?1,?2)",
            params![group_id, serde_json::to_string(&snapshot)?],
        )?;
        Ok(self.db.last_insert_rowid())
    }
    pub fn snapshot(&self, id: i64) -> Result<Snapshot> {
        let body: String =
            self.db
                .query_row("SELECT body FROM report_snapshots WHERE id=?1 AND id NOT IN (SELECT id FROM removed_reports)", [id], |r| {
                    r.get(0)
                })?;
        let snapshot: Snapshot = serde_json::from_str(&body)?;
        ensure!(
            snapshot.schema == 1
                && snapshot.runs.len() == snapshot.group.members.len()
                && snapshot.decisions.len() == snapshot.group.members.len(),
            "Unsupported or incomplete snapshot"
        );
        Ok(snapshot)
    }
    /// Remove from the library without touching source games or analysis.
    /// Tombstones preserve immutable evidence and prevent reuse of old IDs.
    pub fn remove_report(&self, id: i64) -> Result<()> {
        let changed = self.db.execute("INSERT OR IGNORE INTO removed_reports(id)
            SELECT id FROM report_snapshots WHERE id=?1", [id])?;
        ensure!(changed == 1, "Report is missing or already removed");
        Ok(())
    }

    pub fn remove_empty_group(&self, id: i64) -> Result<()> {
        let changed = self.db.execute("INSERT OR IGNORE INTO removed_groups(id)
            SELECT id FROM game_groups WHERE id=?1 AND NOT EXISTS (
                SELECT 1 FROM report_snapshots r WHERE r.group_id=?1
                AND r.id NOT IN (SELECT id FROM removed_reports))", [id])?;
        ensure!(changed == 1, "Group is missing, already removed, or still has saved reports. Remove its reports first.");
        Ok(())
    }

    pub fn listing(&self) -> Result<Value> {
        let mut groups = Vec::new();
        let mut query = self
            .db
            .prepare("SELECT id,body FROM game_groups WHERE id NOT IN (SELECT id FROM removed_groups) ORDER BY id DESC")?;
        for row in query.query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)))? {
            let (id, body) = row?;
            let group: GameGroup = serde_json::from_str(&body)?;
            // Do not send the archived SGFs to the QML list model.
            let members: Vec<_> = group
                .members
                .iter()
                .map(|m| {
                    let mut m = m.clone();
                    m.sgf.clear();
                    m
                })
                .collect();
            groups.push(
                json!({"id":id,"name":group.name,"created_at":group.created_at,"members":members}),
            );
        }
        let mut snapshots = Vec::new();
        let mut query = self
            .db
            .prepare("SELECT id,group_id,body FROM report_snapshots WHERE id NOT IN (SELECT id FROM removed_reports) ORDER BY id DESC")?;
        for row in query.query_map([], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, i64>(1)?,
                r.get::<_, String>(2)?,
            ))
        })? {
            let (id, group_id, body) = row?;
            let snapshot: Snapshot = serde_json::from_str(&body)?;
            let trend_series = snapshot_trend_series(&snapshot)?;
            snapshots.push(json!({"id":id,"group_id":group_id,"label":snapshot.label,
                "group_name":snapshot.group.name,"trend_series":trend_series,
                "created_at":snapshot.created_at,"report":snapshot.report}));
        }
        Ok(json!({"groups":groups,"snapshots":snapshots,"weekly_anchor":self.weekly_anchor()?}))
    }
    pub fn weekly_anchor(&self) -> Result<Option<String>> {
        Ok(self.db.query_row("SELECT value FROM report_preferences WHERE key='weekly_anchor'", [], |r| r.get(0)).optional()?)
    }
    fn date_number(&self, value: &str) -> Result<i64> {
        ensure!(valid_trend_date(value), "Use a complete valid calendar date: {value}");
        Ok(self.db.query_row("SELECT CAST(julianday(?1)-2440587.5 AS INTEGER)", [value], |r| r.get(0))?)
    }
    fn date_text(&self, day: i64) -> Result<String> {
        Ok(self.db.query_row("SELECT date(2440587.5+?1)", [day], |r| r.get(0))?)
    }
    /// Read-only preview. Dates are supplied by the UI in the user's local zone.
    pub fn preview_weekly(&self, project: &Project, today: &str, reset_anchor: bool) -> Result<Value> {
        let end_day = self.date_number(today)?;
        let members = available_games(project)?;
        let earliest = members.iter().filter(|m| valid_trend_date(&m.date) && m.date.as_str() <= today)
            .map(|m| m.date.clone()).min();
        let anchor = if reset_anchor { earliest } else { self.weekly_anchor()?.or(earliest) }
            .context("No games with a complete date on or before today")?;
        let start = self.date_number(&anchor)?;
        ensure!(start <= end_day, "The reporting start is later than today");
        let count = (end_day-start)/7+1;
        ensure!(count <= 10000, "The date range exceeds 10,000 weeks; check your game dates");
        let mut periods = Vec::new();
        for n in 0..count {
            let first = self.date_text(start+n*7)?;
            let last = self.date_text(start+n*7+6)?;
            let selected: Vec<_> = members.iter().filter(|m| valid_trend_date(&m.date) && m.date >= first && m.date <= last && m.date.as_str() <= today).collect();
            periods.push(json!({"from":first,"to":last,"unfinished":last.as_str()>=today,
                "ids":selected.iter().map(|m| m.game_id).collect::<Vec<_>>(),
                "games":selected.len(),"excluded":selected.iter().filter(|m|m.colour.is_none()).count()}));
        }
        let library = self.listing()?;
        Ok(json!({"anchor":anchor,"today":today,"reset_anchor":reset_anchor,"periods":periods,
            "undated":members.iter().filter(|m|!valid_trend_date(&m.date)).count(),
            "before_anchor":members.iter().filter(|m|valid_trend_date(&m.date) && m.date<anchor).count(),
            "future":members.iter().filter(|m|valid_trend_date(&m.date) && m.date.as_str()>today).count(),
            "replace_reports":library["snapshots"].as_array().unwrap().len(),
            "replace_groups":library["groups"].as_array().unwrap().len(),
            "report_ids":library["snapshots"].as_array().unwrap().iter().map(|s|s["id"].clone()).collect::<Vec<_>>(),
            "group_ids":library["groups"].as_array().unwrap().iter().map(|s|s["id"].clone()).collect::<Vec<_>>(),
            // Revalidate the preview if imports, identity edits or removals changed.
            "membership":members.iter().map(Member::metadata).collect::<Vec<_>>() }))
    }
    /// Replace the report library atomically. Source games and analysis are read-only.
    /// A failure rolls back new reports and removals together; old IDs are retained.
    pub fn replace_with_weekly(&self, project: &Project, analysis: &Path, preview: &Value) -> Result<usize> {
        let today = preview["today"].as_str().context("Missing preview date")?;
        let reset = preview["reset_anchor"].as_bool().unwrap_or(false);
        let tx = self.db.unchecked_transaction()?;
        let plan = self.preview_weekly(project, today, reset)?;
        ensure!(&plan == preview, "Games or reports changed since the preview. Preview the weekly reports again.");
        let periods = plan["periods"].as_array().context("Missing periods")?;
        // Tombstones remove entries from the UI, retaining immutable archived evidence.
        self.db.execute_batch("INSERT OR IGNORE INTO removed_reports SELECT id FROM report_snapshots;
            INSERT OR IGNORE INTO removed_groups SELECT id FROM game_groups;")?;
        for period in periods {
            let title = format!("{} – {}", period["from"].as_str().unwrap(), period["to"].as_str().unwrap());
            let ids: Vec<i64> = serde_json::from_value(period["ids"].clone())?;
            let group_id = if ids.is_empty() {
                let group = GameGroup { name: title, created_at: now()?, members: vec![] };
                self.db.execute("INSERT INTO game_groups(body) VALUES(?1)", [serde_json::to_string(&group)?])?;
                self.db.last_insert_rowid()
            } else { self.create_group(project, &title, &ids)? };
            let label = if period["unfinished"] == true { "Weekly report · unfinished period" }
                else if ids.is_empty() { "Weekly report · no games" } else { "Weekly report" };
            self.capture_period(group_id, label, analysis, Some(period.clone()))?;
        }
        self.db.execute("INSERT INTO report_preferences(key,value) VALUES('weekly_anchor',?1)
            ON CONFLICT(key) DO UPDATE SET value=excluded.value", [plan["anchor"].as_str().unwrap()])?;
        tx.commit()?;
        Ok(periods.len())
    }

    /// Derived export: its source of truth remains the immutable database entry.
    pub fn export_html(&self, id: i64, path: &Path) -> Result<()> {
        let snapshot = self.snapshot(id)?;
        let html = crate::snapshot_html::render(id, &snapshot)?;
        std::fs::write(path, html).with_context(|| format!("Writing {}", path.display()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        AnalysisRequest, AnalysisResult,
        game_analysis::{Provenance, prepare_with_provenance},
    };

    fn import(project: &Project, locator: &str, sgf: &str) -> Result<i64> {
        let record = crate::extract_main_variation(&crate::parse_collection(sgf.as_bytes())?)?;
        match project
            .importer()?
            .import_record("Test", "1", locator, &record)?
        {
            crate::importer::ImportOutcome::Imported { game_id, .. } => Ok(game_id),
            _ => anyhow::bail!("Fixture was not new"),
        }
    }
    fn identify(project: &Project) -> Result<i64> {
        let db = crate::database::open(&project.database_root())?;
        let source: i64 =
            db.query_row("SELECT id FROM sources WHERE name='Test'", [], |r| r.get(0))?;
        project
            .player_directory()?
            .save_my_names(None, "Me", &[(source, "me".into())])?;
        Ok(source)
    }
    fn run(project: &Project, id: i64) -> Result<GameAnalysis> {
        let states = project.game_store()?.positions(id)?;
        let p = crate::analysis_position_from_states(&states, states.len() - 1)?;
        prepare_with_provenance(
            AnalysisRequest {
                id: String::new(),
                board_size: p.board_size,
                initial_stones: p.initial_stones,
                initial_player: p.initial_player,
                moves: p.moves,
                rules: "japanese".into(),
                komi: 6.5,
                max_visits: 200,
            },
            "Fixture".into(),
            Provenance {
                policy: 1,
                bermuda_version: "test".into(),
                executable: "unused".into(),
                engine_sha256: "cpu".into(),
                network: "unused".into(),
                network_sha256: "network".into(),
                config: "unused".into(),
                config_sha256: "config".into(),
                score_perspective: "Black".into(),
                created_at: 1,
            },
        )
    }
    fn append(store: &AnalysisStore, run: &mut GameAnalysis, score: f64, win: f64) -> Result<()> {
        let turn = run.results.len();
        let player = run
            .request
            .moves
            .get(turn)
            .map(|m| m.colour)
            .unwrap_or_else(|| run.request.moves.last().unwrap().colour.opponent());
        let result = AnalysisResult {
            id: String::new(),
            turn_number: turn as u32,
            current_player: player,
            score_lead: score,
            win_rate: win,
            visits: 200,
            candidates: vec![],
            is_during_search: false,
            warnings: vec![],
        };
        store.save_position(run, &result)?;
        run.results.push(result);
        Ok(())
    }

    #[test]
    fn weekly_reports_preserve_analysis_and_anchor_and_reject_stale_previews() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let project = crate::project_manager::ProjectManager::new().create("Personal", temp.path().join("project"))?;
        let id = import(&project, "first", "(;SZ[19]PB[me]PW[other]DT[2026-10-08];B[aa];W[bb])")?;
        identify(&project)?;
        let boundary = import(&project, "boundary", "(;SZ[19]PB[me]PW[next]DT[2026-10-14];B[cc])")?;
        import(&project, "later", "(;SZ[19]PB[me]PW[later]DT[2026-10-29];B[dd])")?;
        let store = SnapshotStore::open(&project)?;
        let path = temp.path().join("analysis.sqlite3");
        let analysis = AnalysisStore::open(&path)?;
        let mut run = analysis.restore(run(&project, id)?)?;
        append(&analysis, &mut run, 0.0, 0.5)?;
        append(&analysis, &mut run, -6.0, 0.3)?;
        let mut second = self::run(&project, boundary)?;
        second.request.komi = 0.5;
        let mut second = analysis.restore(second)?;
        append(&analysis, &mut second, 0.0, 0.5)?;
        append(&analysis, &mut second, -2.0, 0.4)?;
        drop(analysis);
        let bytes = std::fs::read(&path)?;
        let old_group = store.create_group(&project, "Old", &[id])?;
        let old = store.capture(old_group, "Old report", &path)?;
        let plan = store.preview_weekly(&project, "2026-10-29", false)?;
        assert_eq!(plan["periods"].as_array().unwrap().len(), 4);
        assert_eq!(plan["periods"][0]["games"], 2);
        assert_eq!(plan["periods"][0]["to"], "2026-10-14");
        assert_eq!(plan["periods"][1]["games"], 0);
        assert_eq!(plan["periods"][3]["unfinished"], true);
        assert_eq!(store.replace_with_weekly(&project, &path, &plan)?, 4);
        assert!(store.snapshot(old).is_err());
        assert!(store.group(old_group).is_err());
        assert_eq!(std::fs::read(&path)?, bytes);
        assert_eq!(available_games(&project)?.len(), 3);
        let listing = store.listing()?;
        let first_week = &listing["snapshots"][3];
        assert_eq!(first_week["report"]["groups"].as_array().unwrap().len(), 1);
        assert_eq!(first_week["report"]["groups"][0]["komi"], "varies by game");
        assert_eq!(first_week["report"]["groups"][0]["decisions"], 2);
        assert_eq!(first_week["trend_series"].as_array().unwrap().len(), 1);
        let newest = listing["snapshots"][0]["id"].as_i64().unwrap();
        let html = crate::snapshot_html::render(newest, &store.snapshot(newest)?)?;
        assert!(html.contains("Unfinished period"));
        let empty = listing["snapshots"][1]["id"].as_i64().unwrap();
        assert!(crate::snapshot_html::render(empty, &store.snapshot(empty)?)?.contains("No games in this period"));
        drop(store);
        let store = SnapshotStore::open(&project)?;
        let stale = store.preview_weekly(&project, "2026-10-29", false)?;
        import(&project, "older", "(;SZ[19]PB[me]PW[older]DT[2026-10-01];B[ee])")?;
        assert!(store.replace_with_weekly(&project, &path, &stale).is_err());
        let kept = store.preview_weekly(&project, "2026-10-29", false)?;
        assert_eq!(kept["anchor"], "2026-10-08");
        assert_eq!(kept["before_anchor"], 1);
        let reset = store.preview_weekly(&project, "2026-10-29", true)?;
        assert_eq!(reset["anchor"], "2026-10-01");
        assert_eq!(store.weekly_anchor()?.as_deref(), Some("2026-10-08"));
        assert_eq!(store.listing()?["snapshots"].as_array().unwrap().len(), 4);
        Ok(())
    }

    #[test]
    fn weekly_failure_rolls_back_replacement_and_dates_cross_leap_day() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let project = crate::project_manager::ProjectManager::new().create("Personal", temp.path().join("project"))?;
        let id = import(&project, "leap", "(;SZ[19]PB[me]PW[other]DT[2024-02-27];B[aa])")?;
        identify(&project)?;
        import(&project, "unknown", "(;SZ[19]PB[me]PW[unknown];B[bb])")?;
        import(&project, "future", "(;SZ[19]PB[me]PW[future]DT[2024-04-01];B[cc])")?;
        let store = SnapshotStore::open(&project)?;
        let missing = temp.path().join("missing.sqlite3");
        let group = store.create_group(&project, "Keep", &[id])?;
        let old = store.capture(group, "Keep", &missing)?;
        let plan = store.preview_weekly(&project, "2024-03-04", false)?;
        assert_eq!(plan["periods"][0]["to"], "2024-03-04");
        assert_eq!(plan["periods"][0]["unfinished"], true);
        assert_eq!(plan["undated"], 1);
        assert_eq!(plan["future"], 1);
        let broken = temp.path().join("broken.sqlite3");
        std::fs::write(&broken, "not a database")?;
        assert!(store.replace_with_weekly(&project, &broken, &plan).is_err());
        assert!(store.snapshot(old).is_ok());
        assert!(store.group(group).is_ok());
        assert_eq!(store.weekly_anchor()?, None);
        assert_eq!(store.listing()?["snapshots"].as_array().unwrap().len(), 1);
        Ok(())
    }
    #[test]
    fn removal_keeps_games_and_evidence_and_never_reuses_ids() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let project = crate::project_manager::ProjectManager::new().create("Personal", temp.path().join("p"))?;
        let game = import(&project, "a", "(;SZ[19]PB[me]PW[other];B[aa];W[bb])")?;
        identify(&project)?;
        let store = SnapshotStore::open(&project)?;
        let group = store.create_group(&project, "Q4", &[game])?;
        let report = store.capture(group, "Baseline", &temp.path().join("absent.sqlite3"))?;
        assert!(store.remove_empty_group(group).is_err());
        store.remove_report(report)?;
        assert!(store.snapshot(report).is_err());
        assert!(store.remove_report(report).is_err());
        store.remove_empty_group(group)?;
        assert!(store.group(group).is_err());
        assert!(store.capture(group, "Gone", &temp.path().join("absent.sqlite3")).is_err());
        drop(store);
        let store = SnapshotStore::open(&project)?;
        assert_eq!(store.listing()?["snapshots"], json!([]));
        assert_eq!(store.listing()?["groups"], json!([]));
        assert_eq!(available_games(&project)?.len(), 1);
        let retained: i64 = store.db.query_row("SELECT count(*) FROM report_snapshots", [], |r| r.get(0))?;
        assert_eq!(retained, 1);
        let next = store.create_group(&project, "Next", &[game])?;
        assert!(next > group);
        assert!(store.capture(next, "New", &temp.path().join("absent.sqlite3"))? > report);
        Ok(())
    }
    #[test]
    fn trend_uses_frozen_buckets_and_separates_engine_settings() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let project = crate::project_manager::ProjectManager::new().create("Personal", temp.path().join("project"))?;
        let id = import(&project, "one", "(;SZ[19]PB[me]PW[other]DT[2026-10-01];B[aa];W[bb])")?;
        identify(&project)?;
        let archive = SnapshotStore::open(&project)?;
        let group = archive.create_group(&project, "October", &[id])?;
        let path = temp.path().join("analysis.sqlite3");
        let analysis = AnalysisStore::open(&path)?;
        let mut run = analysis.restore(run(&project, id)?)?;
        append(&analysis, &mut run, 0.0, 0.5)?;
        append(&analysis, &mut run, -6.0, 0.3)?;
        let first = archive.capture(group, "Baseline", &path)?;
        let mut snapshot = archive.snapshot(first)?;
        let before = snapshot_trend_series(&snapshot)?;
        assert_eq!(before[0]["games"], 1);
        assert_eq!(before[0]["complete"], 0);
        assert_eq!(before[0]["decisions"], 1);
        assert_eq!(before[0]["buckets"], json!([0,0,0,1]));
        assert_eq!(before[0]["from"], "2026-10-01");
        // Counts come from the preserved report, not a fresh scoring calculation.
        snapshot.report["groups"][0]["buckets"] = json!([1,0,0,0]);
        assert_eq!(snapshot_trend_series(&snapshot)?[0]["buckets"], json!([1,0,0,0]));
        snapshot.runs[0].as_mut().unwrap().provenance.engine_sha256 = "graphics-card".into();
        assert_ne!(snapshot_trend_series(&snapshot)?[0]["key"], before[0]["key"]);
        snapshot.report["groups"][0]["analysed"][0]["date"] = json!("2026-02-30");
        let undated = snapshot_trend_series(&snapshot)?;
        assert_eq!(undated[0]["undated"], 1);
        assert!(undated[0]["to"].is_null());
        let second = archive.capture(group, "Second report", &path)?;
        assert_ne!(first, second);
        assert_eq!(archive.listing()?["snapshots"].as_array().unwrap().len(), 2);
        assert_eq!(archive.snapshot(first)?.report["groups"][0]["buckets"], json!([0,0,0,1]));
        Ok(())
    }

    #[test]
    fn preserves_partial_evidence_across_completion_import_identity_edit_deletion_and_restart()
    -> Result<()> {
        let temp = tempfile::tempdir()?;
        let project = crate::project_manager::ProjectManager::new()
            .create("Personal", temp.path().join("project"))?;
        let id = import(
            &project,
            "october",
            "(;SZ[19]PB[me]PW[other]DT[2026-10-01]RE[W+2.5];B[aa];W[bb])",
        )?;
        let source = identify(&project)?;
        let store = SnapshotStore::open(&project)?;
        let group = store.create_group(&project, "October", &[id, id])?;
        assert_eq!(store.group(group)?.members.len(), 1);
        let path = temp.path().join("analysis.sqlite3");
        let pending = store.capture(group, "Not analysed yet", &path)?;
        assert!(!path.exists());
        assert_eq!(
            store.snapshot(pending)?.report["pending"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
        let analysis = AnalysisStore::open(&path)?;
        let mut run = analysis.restore(run(&project, id)?)?;
        append(&analysis, &mut run, 0.0, 0.7)?;
        append(&analysis, &mut run, -5.0, 0.5)?;
        let first = store.capture(group, "CPU partial", &path)?;
        let original = serde_json::to_string(&store.snapshot(first)?)?;
        assert_eq!(store.snapshot(first)?.report["partial"], 1);
        assert_eq!(store.snapshot(first)?.report["groups"][0]["decisions"], 1);
        append(&analysis, &mut run, 1.0, 0.6)?;
        let second = store.capture(group, "CPU complete", &path)?;
        assert_eq!(store.snapshot(second)?.report["complete"], 1);
        import(
            &project,
            "november",
            "(;SZ[19]PB[me]PW[new]DT[2026-11-01];B[cc];W[dd])",
        )?;
        let me = project.player_directory()?.personal_player_id()?.unwrap();
        project.player_directory()?.save_my_names(
            Some(me),
            "A changed name",
            &[(source, "other".into())],
        )?;
        assert_eq!(store.group(group)?.members[0].colour, Some(Colour::Black));
        assert_eq!(store.group(group)?.members[0].black, "Me");
        project.game_store()?.remove(id)?;
        // Capture still matches the saved main line even after its source is gone.
        let third = store.capture(group, "After source removal", &path)?;
        assert_eq!(store.snapshot(third)?.report["complete"], 1);
        drop(analysis);
        std::fs::remove_file(&path)?;
        let after_cleanup = store.capture(group, "After analysis cleanup", &path)?;
        assert_eq!(
            store.snapshot(after_cleanup)?.report["pending"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
        drop(store);
        let reopened = SnapshotStore::open(&project)?;
        assert_eq!(serde_json::to_string(&reopened.snapshot(first)?)?, original);
        assert_eq!(reopened.group(group)?.members.len(), 1);
        let html = temp.path().join("report.html");
        reopened.export_html(first, &html)?;
        let html = std::fs::read_to_string(html)?;
        assert!(html.contains("CPU partial"));
        assert!(html.contains("Before move 1"));
        assert!(html.contains("70.0% → 50.0%"));
        assert!(html.contains("Download preserved game"));
        // Restarting/exporting never needs an engine or the deleted source files.
        Ok(())
    }
    #[test]
    fn empty_missing_unknown_and_overlapping_selections_are_explicit() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let project = crate::project_manager::ProjectManager::new()
            .create("Personal", temp.path().join("p"))?;
        let a = import(&project, "a", "(;SZ[19]PB[me]PW[other];B[aa];W[bb])")?;
        let b = import(&project, "b", "(;SZ[19]PB[unknown]PW[other];B[cc];W[dd])")?;
        identify(&project)?;
        let store = SnapshotStore::open(&project)?;
        assert!(store.create_group(&project, "Empty", &[]).is_err());
        assert!(store.create_group(&project, "Missing", &[a, -1]).is_err());
        assert!(store.create_group(&project, " ", &[a]).is_err());
        assert!(store.listing()?["groups"].as_array().unwrap().is_empty());
        let one = store.create_group(&project, "One", &[a, b])?;
        let two = store.create_group(&project, "Overlap", &[a])?;
        assert_eq!(store.group(two)?.members[0].game_id, a);
        let id = store.capture(one, "Incomplete", &temp.path().join("absent.sqlite3"))?;
        let snapshot = store.snapshot(id)?;
        assert_eq!(snapshot.report["selected"], 1);
        assert_eq!(snapshot.report["excluded"], 1);
        assert_eq!(snapshot.report["pending"].as_array().unwrap().len(), 1);
        assert_eq!(snapshot.group.members.len(), 2);
        assert!(
            store
                .db
                .execute("UPDATE report_snapshots SET body='{}' WHERE id=?1", [id])
                .is_err()
        );
        assert!(
            store
                .db
                .execute("UPDATE game_groups SET body='{}' WHERE id=?1", [one])
                .is_err()
        );
        Ok(())
    }
    #[test]
    fn newer_analysis_is_a_new_snapshot_and_exports_escape_user_text() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let project = crate::project_manager::ProjectManager::new()
            .create("Personal", temp.path().join("p"))?;
        let id = import(
            &project,
            "a",
            "(;SZ[19]PB[me]PW[<script>alert(1)</script>]DT[2026-10-01];B[aa];W[bb])",
        )?;
        identify(&project)?;
        let store = SnapshotStore::open(&project)?;
        let group = store.create_group(&project, "<img src=x onerror=alert(1)>", &[id])?;
        let path = temp.path().join("a.sqlite3");
        let analysis = AnalysisStore::open(&path)?;
        let mut old = analysis.restore(run(&project, id)?)?;
        append(&analysis, &mut old, 1.0, 0.6)?;
        append(&analysis, &mut old, 0.0, 0.5)?;
        let first = store.capture(group, "CPU", &path)?;
        let mut request = old.request.clone();
        request.max_visits = 1000;
        let mut provenance = old.provenance.clone();
        provenance.engine_sha256 = "gpu".into();
        provenance.created_at = 2;
        let mut new =
            analysis.restore(prepare_with_provenance(request, "New".into(), provenance)?)?;
        append(&analysis, &mut new, 10.0, 0.9)?;
        append(&analysis, &mut new, 0.0, 0.5)?;
        let second = store.capture(group, "GPU", &path)?;
        let a = store.snapshot(first)?;
        let b = store.snapshot(second)?;
        assert_eq!(a.runs[0].as_ref().unwrap().request.max_visits, 200);
        assert_eq!(b.runs[0].as_ref().unwrap().request.max_visits, 1000);
        assert_ne!(a.report, b.report);
        let html = crate::snapshot_html::render(first, &a)?;
        assert!(!html.contains("<script>"));
        assert!(!html.contains("<img src=x"));
        assert!(html.contains("&lt;script&gt;"));
        assert!(html.contains("&lt;img"));
        assert!(!html.contains("https://"));
        Ok(())
    }
}
