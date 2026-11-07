use anyhow::Result;
use bermuda::{project_manager::ProjectManager, importer::ImportOutcome, game_list::GameListQuery};

#[test]
fn recorded_ranks_survive_import_reopen_export_and_duplicate_enrichment() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let root = temp.path().join("project");
    let project = ProjectManager::new().create("Ranks", &root)?;
    let path = temp.path().join("game.sgf");
    std::fs::write(&path, "(;SZ[19]PB[me]PW[opponent]BR[2k]WR[1d];B[aa];W[bb])")?;
    let collection = bermuda::parse_collection(&std::fs::read(&path)?)?;
    let record = bermuda::extract_main_variation(&collection)?;
    let mut old_record = record.clone();
    old_record.metadata.black_rank = None;
    old_record.metadata.white_rank = None;
    assert_eq!(bermuda::canonical_hash(&record)?, bermuda::canonical_hash(&old_record)?);
    let locator = bermuda::canonical_hash_hex(&record)?;
    let id = match project.importer()?.import_record("Personal", "1", &locator, &old_record)? {
        ImportOutcome::Imported { game_id, .. } => game_id,
        _ => panic!("expected import"),
    };
    assert!(matches!(project.importer()?.import_record("Personal", "1", &locator, &record)?, ImportOutcome::AlreadyImported { .. }));
    let project = ProjectManager::new().open(&root)?;
    let rows = project.catalogue()?.list(&GameListQuery::default())?;
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].black_rank.as_deref(), Some("2k"));
    assert_eq!(rows[0].white_rank.as_deref(), Some("1d"));
    let occurrences = project.catalogue()?.list_occurrences(&GameListQuery::default())?;
    assert_eq!(occurrences[0].white_rank.as_deref(), Some("1d"));
    let exported = bermuda::write_game_record_sgf(&project.game_store()?.load(id)?)?;
    assert!(exported.contains("BR[2k]"));
    assert!(exported.contains("WR[1d]"));
    Ok(())
}

#[test]
fn legacy_source_recovery_checks_game_identity() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let project = ProjectManager::new().create("Ranks", temp.path().join("project"))?;
    let path = temp.path().join("game.sgf");
    std::fs::write(&path, "(;SZ[19]PB[A]PW[B]BR[3k]WR[4k];B[aa])")?;
    project.importer()?.import_file("Source", "1", &path)?;
    let db = bermuda::database::open(&project.database_root())?;
    db.execute("UPDATE game_metadata SET black_rank=NULL,white_rank=NULL", [])?;
    let rows = project.catalogue()?.list(&GameListQuery::default())?;
    assert_eq!(rows[0].white_rank.as_deref(), Some("4k"));
    db.execute("UPDATE game_metadata SET black_rank=NULL,white_rank=NULL", [])?;
    std::fs::write(&path, "(;SZ[19]PB[A]PW[B]BR[9d]WR[9d];B[bb])")?;
    assert!(project.catalogue()?.get(rows[0].game_id)?.white_rank.is_none());
    Ok(())
}

#[test]
fn legacy_database_adds_optional_rank_columns_without_changing_schema_version() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let project = ProjectManager::new().create("Legacy", temp.path().join("project"))?;
    {
        let db = bermuda::database::open(&project.database_root())?;
        db.execute_batch("ALTER TABLE game_metadata DROP COLUMN black_rank; ALTER TABLE game_metadata DROP COLUMN white_rank;")?;
    }
    let db = bermuda::database::open(&project.database_root())?;
    let version: i64 = db.query_row("SELECT schema_version FROM schema_info", [], |r| r.get(0))?;
    assert_eq!(version, 8);
    db.prepare("SELECT black_rank, white_rank FROM game_metadata")?;
    Ok(())
}
