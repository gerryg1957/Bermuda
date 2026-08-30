use anyhow::Result;
use bermuda::{
    Colour, GameRecord, Metadata, Move, database, importer::ImportOutcome,
    project_manager::ProjectManager,
};
use tempfile::tempdir;

fn played_record(black_player: &str, result: &str) -> GameRecord {
    GameRecord {
        board_size: 19,

        metadata: Metadata {
            black_player: Some(black_player.to_owned()),
            white_player: Some("Opponent".to_owned()),
            date: Some("2026-08-30".to_owned()),
            event: Some("Played in Bermuda".to_owned()),
            result: Some(result.to_owned()),
            komi: Some(6.5),
            handicap: None,
        },

        setup: Vec::new(),

        moves: vec![
            Move {
                colour: Colour::Black,
                point: Some(3 * 19 + 3),
            },
            Move {
                colour: Colour::White,
                point: None,
            },
            Move {
                colour: Colour::Black,
                point: Some(15 * 19 + 15),
            },
        ],
    }
}

#[test]
fn imports_game_record_without_sgf_round_trip() -> Result<()> {
    let temporary = tempdir()?;
    let project_root = temporary.path().join("personal-games");

    let manager = ProjectManager::new();
    let project = manager.create("Personal Games", &project_root)?;

    let first = played_record("Gerry", "W+R");

    let mut importer = project.importer()?;

    let first_outcome = importer.import_record("Bermuda", "played", "played:test-one", &first)?;

    let (game_id, move_file) = match first_outcome {
        ImportOutcome::Imported { game_id, move_file } => (game_id, move_file),

        other => panic!("first direct record import should create a game, got {other:?}"),
    };

    assert!(move_file.is_file());

    /*
     * Metadata is deliberately excluded from canonical identity.
     * This second record has the same moves but a different source
     * spelling/result and should therefore add another source to the
     * same canonical game rather than another game.
     */
    let second = played_record("Gerry Smith", "B+2.5");

    let second_outcome = importer.import_record("Bermuda", "played", "played:test-two", &second)?;

    match second_outcome {
        ImportOutcome::AddedSource {
            game_id: second_game_id,
        } => assert_eq!(second_game_id, game_id),

        other => panic!("second provenance should be added to the same game, got {other:?}"),
    }

    /*
     * Reusing a source locator is not a third source occurrence.
     */
    let duplicate_outcome =
        importer.import_record("Bermuda", "played", "played:test-two", &second)?;

    match duplicate_outcome {
        ImportOutcome::AlreadyImported {
            game_id: duplicate_game_id,
        } => assert_eq!(duplicate_game_id, game_id),

        other => panic!("reused source locator should be already imported, got {other:?}"),
    }

    drop(importer);

    /*
     * The canonical game itself exists once.
     */
    let connection = database::open(&project.database_root())?;

    let game_count: i64 =
        connection.query_row("SELECT COUNT(*) FROM games", [], |row| row.get(0))?;

    let source_count: i64 =
        connection.query_row("SELECT COUNT(*) FROM game_sources", [], |row| row.get(0))?;

    let metadata_count: i64 =
        connection.query_row("SELECT COUNT(*) FROM game_metadata", [], |row| row.get(0))?;

    assert_eq!(game_count, 1);
    assert_eq!(source_count, 2);
    assert_eq!(metadata_count, 2);

    let metadata = {
        let mut statement = connection.prepare(
            r#"
            SELECT
                black_player,
                result
            FROM game_metadata
            ORDER BY game_source_id
            "#,
        )?;

        let rows = statement.query_map([], |row| {
            Ok((
                row.get::<_, Option<String>>(0)?,
                row.get::<_, Option<String>>(1)?,
            ))
        })?;

        rows.collect::<rusqlite::Result<Vec<_>>>()?
    };

    assert_eq!(
        metadata,
        vec![
            (Some("Gerry".to_owned()), Some("W+R".to_owned()),),
            (Some("Gerry Smith".to_owned()), Some("B+2.5".to_owned()),),
        ],
    );

    drop(connection);

    /*
     * The ordinary GameStore can read the directly ingested game.
     * In particular the recorded pass remains part of its move stream.
     */
    let stored = project.game_store()?.load(game_id)?;

    assert_eq!(stored.moves, first.moves);
    assert_eq!(stored.moves[1].point, None);

    Ok(())
}
