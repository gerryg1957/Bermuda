use moyodb::{
    Color, GameRecord, Metadata, Move, Pattern, PatternMatch, PatternRect, PatternSearchQuery,
    PatternSearchScope, SearchEngine, database, indexer::PositionIndexer, write_move_file,
};
use rusqlite::params;
use std::path::Path;
use tempfile::TempDir;

fn create_test_indexer() -> (TempDir, PositionIndexer) {
    let temporary = TempDir::new().expect("create temporary directory");
    let root = temporary.path().join("database");

    database::initialise(&root).expect("initialise test database");

    let connection = database::open(&root).expect("open test database");

    for (game_id, relative_path) in [
        (1_i64, "games/aa/game-one.moves"),
        (2_i64, "games/bb/game-two.moves"),
    ] {
        let hash = vec![game_id as u8; 32];

        connection
            .execute(
                r#"
                INSERT INTO games(
                    id,
                    canonical_hash,
                    board_size,
                    move_count,
                    move_file
                )
                VALUES (?1, ?2, 19, 1, ?3)
                "#,
                params![game_id, hash, relative_path],
            )
            .expect("insert test game");

        write_test_move_file(&root, relative_path);
    }

    drop(connection);

    let indexer = PositionIndexer::open(&root).expect("open position indexer");

    (temporary, indexer)
}

fn write_test_move_file(root: &Path, relative_path: &str) {
    let absolute_path = root.join(relative_path);

    if let Some(parent) = absolute_path.parent() {
        std::fs::create_dir_all(parent).expect("create move-file directory");
    }

    let record = GameRecord {
        board_size: 19,
        setup: Vec::new(),
        moves: vec![Move {
            color: Color::Black,
            point: Some(0),
        }],
        metadata: Metadata {
            black_player: None,
            white_player: None,
            date: None,
            event: None,
            result: None,
            komi: None,
            handicap: None,
        },
    };

    write_move_file(&absolute_path, &record).expect("write test move file");
}

fn test_pattern(indexer: &PositionIndexer) -> Pattern {
    let state = indexer
        .replay_board_position(1, 1)
        .expect("replay source position");

    Pattern::extract(
        &state.board,
        PatternRect {
            left: 0,
            bottom: 0,
            width: 2,
            height: 2,
        },
    )
    .expect("extract test pattern")
}

#[test]
fn game_scope_searches_only_requested_game() {
    let (_temporary, indexer) = create_test_indexer();
    let pattern = test_pattern(&indexer);

    let query = PatternSearchQuery {
        pattern,
        scope: PatternSearchScope::Game(1),
    };

    let matches = SearchEngine::new(&indexer)
        .search_pattern(&query)
        .expect("search one game");

    assert_eq!(
        matches,
        vec![PatternMatch {
            game_id: 1,
            move_number: 1,
            left: 0,
            bottom: 0,
        }]
    );
}

#[test]
fn project_scope_searches_every_game() {
    let (_temporary, indexer) = create_test_indexer();
    let pattern = test_pattern(&indexer);

    let query = PatternSearchQuery {
        pattern,
        scope: PatternSearchScope::Project,
    };

    let matches = SearchEngine::new(&indexer)
        .search_pattern(&query)
        .expect("search project");

    assert_eq!(
        matches,
        vec![
            PatternMatch {
                game_id: 1,
                move_number: 1,
                left: 0,
                bottom: 0,
            },
            PatternMatch {
                game_id: 2,
                move_number: 1,
                left: 0,
                bottom: 0,
            },
        ]
    );
}
