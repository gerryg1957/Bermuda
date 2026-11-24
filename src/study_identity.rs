//! Identify a player before moving an arbitrary study into the personal library.
use anyhow::{ensure, Result};
use rusqlite::{params, OptionalExtension, Transaction};
use serde::Serialize;
use crate::{GameRecord, project::Project, player_directory::PlayerSide,
    importer::ImportOutcome, indexer::POSITION_INDEX_VERSION};

#[derive(Serialize)]
pub struct StudyIdentity {
    pub existing: bool,
    pub side: Option<&'static str>,
    pub black: String,
    pub white: String,
}

fn existing_id(project: &Project, record: &GameRecord) -> Result<Option<i64>> {
    let db = crate::database::open(&project.database_root())?;
    let hash = crate::canonical_hash(record)?;
    Ok(db.query_row("SELECT id FROM games WHERE canonical_hash=?1", [hash.as_slice()],
        |r| r.get(0)).optional()?)
}

pub fn inspect(project: &Project, record: &GameRecord) -> Result<StudyIdentity> {
    let directory = project.player_directory()?;
    let me = directory.personal_player_id()?;
    // Preferred names are presentation labels (often simply "Me"). Only
    // deliberately saved aliases should identify future SGFs automatically.
    let aliases = me.map(|id| directory.aliases_for_player(id)).transpose()?.unwrap_or_default();
    let black = record.metadata.black_player.clone().unwrap_or_default();
    let white = record.metadata.white_player.clone().unwrap_or_default();
    let recognised = |name: &str| -> Result<bool> {
        Ok(match me {
            Some(id) if !name.trim().is_empty() => aliases.iter().any(|a| a.name == name)
                && directory.player_ids_for_search_name(name)? == vec![id],
            _ => false,
        })
    };
    let side = match (recognised(&black)?, recognised(&white)?) {
        (true, false) => Some("black"),
        (false, true) => Some("white"),
        _ => None,
    };
    Ok(StudyIdentity { existing: existing_id(project, record)?.is_some(), side, black, white })
}

/// No selection means names must identify exactly one player. An explicit
/// selection is game-local unless the user also asks to remember the name.
pub fn add(project: &Project, record: &GameRecord, chosen: Option<PlayerSide>, remember: bool) -> Result<i64> {
    ensure!(!record.moves.is_empty(), "This is a position without game moves");
    if let Some(id) = existing_id(project, record)? {
        project.position_indexer()?.index_game_by_id(id, POSITION_INDEX_VERSION)?;
        return Ok(id);
    }
    let state = inspect(project, record)?;
    let side = chosen.or(match state.side {
        Some("black") => Some(PlayerSide::Black), Some("white") => Some(PlayerSide::White), _ => None,
    }).ok_or_else(|| anyhow::anyhow!("Choose which player you are before adding this game"))?;
    ensure!(!remember || chosen.is_some(), "Remembering a name requires an explicit player selection");
    let locator = crate::canonical_hash_hex(record)?;
    let outcome = project.importer()?.import_record_with_identity(
        "Imported personal games", "1", &locator, record, Some((side, remember)))?;
    let id = match outcome {
        ImportOutcome::Imported { game_id, .. } | ImportOutcome::AddedSource { game_id }
        | ImportOutcome::AlreadyImported { game_id } => game_id,
        ImportOutcome::SkippedBoardSize { board_size } => anyhow::bail!("My games does not yet support {board_size}x{board_size} games"),
    };
    use anyhow::Context;
    project.position_indexer()?.index_game_by_id(id, POSITION_INDEX_VERSION)
        .context("Game saved, but search indexing failed; add this file again to retry")?;
    Ok(id)
}

pub(crate) fn attach(db: &Transaction<'_>, source: i64, record: &GameRecord, side: PlayerSide, remember: bool) -> Result<()> {
    let name = match side { PlayerSide::Black => &record.metadata.black_player, PlayerSide::White => &record.metadata.white_player };
    let name = name.as_deref().unwrap_or("");
    db.execute_batch("CREATE TABLE IF NOT EXISTS personal_player (
        id INTEGER PRIMARY KEY CHECK (id = 1),
        player_id INTEGER NOT NULL REFERENCES players(id) ON DELETE CASCADE);")?;
    let existing: Option<i64> = db.query_row("SELECT player_id FROM personal_player WHERE id=1", [], |r| r.get(0)).optional()?;
    if remember {
        ensure!(!name.trim().is_empty(), "An unnamed player can only be identified for this game");
        let owners = crate::player_directory::player_ids_for_search_name_on(db, name)?;
        ensure!(owners.iter().all(|id| Some(*id) == existing),
            "This name already belongs to another identity. Use Just this game, or review Manage my names.");
        let other = match side { PlayerSide::Black => &record.metadata.white_player, PlayerSide::White => &record.metadata.black_player };
        ensure!(!other.as_deref().unwrap_or("").eq_ignore_ascii_case(name),
            "Both players have this name. Use Just this game instead.");
    }
    let me = match existing {
        Some(id) => id,
        None => {
            db.execute("INSERT INTO players(preferred_name) VALUES ('Me')", [])?;
            let id = db.last_insert_rowid();
            db.execute("INSERT INTO personal_player(id,player_id) VALUES(1,?1)", [id])?;
            id
        }
    };
    if remember {
        db.execute("INSERT INTO player_aliases(player_id,name,notes)
            SELECT ?1,?2,'Remembered from Study' WHERE NOT EXISTS
            (SELECT 1 FROM player_aliases WHERE player_id=?1 AND name=?2 AND source_id IS NULL)", params![me,name])?;
    }
    let (selected, other) = match side { PlayerSide::Black => ("black", "white"), PlayerSide::White => ("white", "black") };
    db.execute(&format!("UPDATE game_metadata SET {selected}_player_id=?1,
        {selected}_player_catalogue_derived=0, {selected}_player_catalogue_suppressed=0,
        {other}_player_id=CASE WHEN {other}_player_id=?1 THEN NULL ELSE {other}_player_id END
        WHERE game_source_id=?2"), params![me, source])?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn game(black: &str, white: &str, point: &str) -> GameRecord {
        crate::extract_main_variation(&crate::parse_collection(
            format!("(;SZ[19]PB[{black}]PW[{white}]BR[7k]WR[6k];B[{point}];W[pp])").as_bytes()
        ).unwrap()).unwrap()
    }
    fn project(temp: &tempfile::TempDir) -> Project {
        crate::project_manager::ProjectManager::new().create("Personal", temp.path().join("project")).unwrap()
    }
    fn me_side(project: &Project, id: i64) -> (bool, bool) {
        let me = project.player_directory().unwrap().personal_player_id().unwrap().unwrap();
        let db = crate::database::open(&project.database_root()).unwrap();
        db.query_row("SELECT COALESCE(m.black_player_id=?1,0), COALESCE(m.white_player_id=?1,0)
            FROM game_metadata m JOIN game_sources s ON s.id=m.game_source_id WHERE s.game_id=?2",
            params![me,id], |r| Ok((r.get(0)?,r.get(1)?))).unwrap()
    }

    #[test]
    fn unfamiliar_game_requires_selection_and_just_this_game_stays_local() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let p = project(&temp);
        let record = game("Player 1", "Opponent", "dd");
        assert!(add(&p, &record, None, false).is_err());
        assert!(!inspect(&p, &record)?.existing);
        let id = add(&p, &record, Some(PlayerSide::Black), false)?;
        assert_eq!(me_side(&p,id), (true,false));
        assert!(inspect(&p, &record)?.existing);
        assert_eq!(add(&p, &record, None, false)?, id);
        assert_eq!(inspect(&p, &game("Player 1", "Opponent", "dc"))?.side, None);
        assert_eq!(inspect(&p, &game("Me", "Opponent", "dc"))?.side, None);
        // The existing report machinery sees the game-local identity.
        let report = crate::personal_report::build(&p, &temp.path().join("no-analysis.sqlite3"))?;
        assert_eq!(report["identified"], 1);
        let db = crate::database::open(&p.database_root())?;
        let metadata: (String,String,String) = db.query_row(
            "SELECT black_player,white_player,black_rank FROM game_metadata", [],
            |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?)))?;
        assert_eq!(metadata, ("Player 1".into(),"Opponent".into(),"7k".into()));
        assert_eq!(db.query_row("SELECT COUNT(*) FROM games", [], |r| r.get::<_,i64>(0))?,1);
        Ok(())
    }

    #[test]
    fn remembered_tournament_name_recognises_future_games_and_bulk_imports() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let p = project(&temp);
        let directory = p.player_directory()?;
        let me = directory.create_player("Gerry")?.id;
        directory.set_personal_player(Some(me))?;
        directory.add_alias(me, "gerryg", None, None)?;
        assert_eq!(inspect(&p, &game("gerryg", "Opponent", "dd"))?.side, Some("black"));
        let id = add(&p, &game("Opponent", "G. Gavigan", "dd"), Some(PlayerSide::White), true)?;
        assert_eq!(me_side(&p,id), (false,true));
        let next = game("G. Gavigan", "Other", "dc");
        assert_eq!(inspect(&p, &next)?.side, Some("black"));
        let id = add(&p, &next, None, false)?;
        assert_eq!(me_side(&p,id), (true,false));
        let bulk = game("Opponent", "G. Gavigan", "de");
        let outcome = p.importer()?.import_record("Tournament", "1", "new.sgf", &bulk)?;
        let ImportOutcome::Imported { game_id, .. } = outcome else { panic!("new game expected") };
        assert_eq!(me_side(&p,game_id), (false,true));
        Ok(())
    }

    #[test]
    fn ambiguous_names_require_selection_and_conflicting_remember_rolls_back() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let p = project(&temp);
        let directory = p.player_directory()?;
        let me = directory.create_player("Gerry")?.id;
        directory.set_personal_player(Some(me))?;
        directory.add_alias(me, "Player 1", None, None)?;
        let other = directory.create_player("Someone else")?;
        directory.add_alias(other.id, "Player 1", None, None)?;
        let record = game("Player 1", "Opponent", "dd");
        assert_eq!(inspect(&p, &record)?.side,None);
        assert!(add(&p, &record, Some(PlayerSide::Black), true).is_err());
        assert!(!inspect(&p, &record)?.existing);
        let id = add(&p, &record, Some(PlayerSide::Black), false)?;
        assert_eq!(me_side(&p,id),(true,false));
        let both = game("Gerry", "Gerry", "dc");
        assert_eq!(inspect(&p, &both)?.side,None);
        assert!(add(&p, &both, Some(PlayerSide::White), true).is_err());
        let id = add(&p, &both, Some(PlayerSide::White), false)?;
        assert_eq!(me_side(&p,id),(false,true));
        Ok(())
    }

    #[test]
    fn unnamed_player_can_be_identified_without_learning_an_empty_name() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let p = project(&temp);
        let record = game("", "Opponent", "dd");
        assert!(add(&p, &record, Some(PlayerSide::Black), true).is_err());
        assert_eq!(p.player_directory()?.personal_player_id()?, None);
        let id = add(&p, &record, Some(PlayerSide::Black), false)?;
        assert_eq!(me_side(&p,id),(true,false));
        Ok(())
    }
}
