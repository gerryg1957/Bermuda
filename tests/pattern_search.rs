use std::path::Path;

use moyodb::{
    Colour, GameRecord, Metadata, Move, Pattern, PatternMatch, PatternRect, PatternSearchOptions,
    PatternSearchQuery, PatternSearchScope, PatternSearcher, PatternTransformation, SearchEngine,
    SearchOccurrence, SearchPatternSummaryOutcome, SearchResult, SearchSummaryResult, database,
    project::Project, write_move_file,
};
use rusqlite::params;
use tempfile::TempDir;

fn create_test_project() -> (TempDir, Project) {
    let temporary = TempDir::new().expect("create temporary directory");
    let database_root = temporary.path().join("database");

    database::initialise(&database_root).expect("initialise test database");

    let connection = database::open(&database_root).expect("open test database");

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
                VALUES (?1, ?2, 19, 3, ?3)
                "#,
                params![game_id, hash, relative_path],
            )
            .expect("insert test game");

        write_test_move_file(&database_root, relative_path);
    }

    drop(connection);

    let project = Project::new("Test Project", temporary.path());

    (temporary, project)
}

fn write_test_move_file(root: &Path, relative_path: &str) {
    let absolute_path = root.join(relative_path);

    if let Some(parent) = absolute_path.parent() {
        std::fs::create_dir_all(parent).expect("create move-file directory");
    }

    let record = GameRecord {
        board_size: 19,
        setup: Vec::new(),
        moves: vec![
            Move {
                colour: Colour::Black,
                point: Some(0),
            },
            Move {
                colour: Colour::White,
                point: None,
            },
            Move {
                colour: Colour::Black,
                point: None,
            },
        ],
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

fn test_pattern(project: &Project) -> Pattern {
    let indexer = project
        .position_indexer()
        .expect("open test position indexer");

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

fn synthetic_match(move_number: usize, left: u8, bottom: u8) -> PatternMatch {
    PatternMatch {
        game_id: 1,
        move_number,
        last_move_number: move_number,
        side_to_move: Colour::White,
        ko_point: None,
        left,
        bottom,
        transformation: PatternTransformation::Identity,
        colours_reversed: false,
    }
}

fn synthetic_appearance(
    first_move_number: usize,
    last_move_number: usize,
    left: u8,
    bottom: u8,
) -> PatternMatch {
    let mut found = synthetic_match(first_move_number, left, bottom);
    found.last_move_number = last_move_number;
    found
}

#[test]
fn distinct_appearances_collapse_continuity_and_keep_reappearance() {
    let appearances = PatternSearcher::distinct_appearances(vec![
        synthetic_match(1, 0, 0),
        synthetic_match(1, 5, 5),
        synthetic_match(2, 0, 0),
        synthetic_match(2, 5, 5),
        synthetic_match(4, 0, 0),
    ]);

    assert_eq!(
        appearances,
        vec![
            synthetic_appearance(1, 2, 0, 0),
            synthetic_appearance(1, 2, 5, 5),
            synthetic_match(4, 0, 0),
        ]
    );

    assert_eq!(appearances[0].duration_moves(), 1);
    assert_eq!(appearances[1].duration_moves(), 1);
    assert_eq!(appearances[2].duration_moves(), 0);
}

#[test]
fn game_appearance_search_ignores_unchanged_pass_positions() {
    let (_temporary, project) = create_test_project();
    let pattern = test_pattern(&project);
    let indexer = project
        .position_indexer()
        .expect("open test position indexer");
    let searcher = PatternSearcher::new();

    let raw_matches = searcher
        .search_game(&indexer, 1, &pattern)
        .expect("search raw game positions");

    assert_eq!(
        raw_matches
            .iter()
            .map(|found| found.move_number)
            .collect::<Vec<_>>(),
        vec![1, 2, 3]
    );

    let appearances = searcher
        .search_game_appearances(&indexer, 1, &pattern)
        .expect("search distinct appearances");

    assert_eq!(appearances, vec![synthetic_appearance(1, 3, 0, 0)]);
    assert_eq!(appearances[0].move_number, 1);
    assert_eq!(appearances[0].last_move_number, 3);
    assert_eq!(appearances[0].duration_moves(), 2);
}

#[test]
fn game_scope_groups_occurrences_for_requested_game() {
    let (_temporary, project) = create_test_project();
    let pattern = test_pattern(&project);

    let query = PatternSearchQuery {
        pattern,
        options: PatternSearchOptions::default(),
        scope: PatternSearchScope::Game(1),
    };

    let results = SearchEngine::new(&project)
        .expect("create search engine")
        .search_pattern(&query)
        .expect("search one game");

    assert_eq!(
        results,
        vec![SearchResult {
            game_id: 1,
            black_player: None,
            white_player: None,
            game_date: None,
            result: None,
            event: None,
            komi: None,
            occurrences: vec![SearchOccurrence {
                move_number: 1,
                side_to_move: Some(Colour::White),
                ko_point: None,
                left: Some(0),
                bottom: Some(0),
                transformation: Some(PatternTransformation::Identity,),
                colours_reversed: Some(false),
            }],
        }]
    );
}

#[test]
fn project_summary_search_keeps_only_counts_and_first_matches() {
    let (_temporary, project) = create_test_project();
    let pattern = test_pattern(&project);

    let query = PatternSearchQuery {
        pattern,
        options: PatternSearchOptions::default(),
        scope: PatternSearchScope::Project,
    };

    let outcome = SearchEngine::new(&project)
        .expect("create search engine")
        .search_pattern_summaries_with_progress(&query, || false, |_| {})
        .expect("search project summaries");

    assert_eq!(
        outcome,
        SearchPatternSummaryOutcome::Completed(vec![
            SearchSummaryResult {
                game_id: 1,
                black_player: None,
                white_player: None,
                game_date: None,
                result: None,
                event: None,
                komi: None,
                match_count: 1,
                first_occurrence: SearchOccurrence {
                    move_number: 1,
                    side_to_move: Some(Colour::White),
                    ko_point: None,
                    left: Some(0),
                    bottom: Some(0),
                    transformation: Some(PatternTransformation::Identity,),
                    colours_reversed: Some(false),
                },
            },
            SearchSummaryResult {
                game_id: 2,
                black_player: None,
                white_player: None,
                game_date: None,
                result: None,
                event: None,
                komi: None,
                match_count: 1,
                first_occurrence: SearchOccurrence {
                    move_number: 1,
                    side_to_move: Some(Colour::White),
                    ko_point: None,
                    left: Some(0),
                    bottom: Some(0),
                    transformation: Some(PatternTransformation::Identity,),
                    colours_reversed: Some(false),
                },
            },
        ])
    );
}

#[test]
fn project_scope_returns_one_result_per_matching_game() {
    let (_temporary, project) = create_test_project();
    let pattern = test_pattern(&project);

    let query = PatternSearchQuery {
        pattern,
        options: PatternSearchOptions::default(),
        scope: PatternSearchScope::Project,
    };

    let results = SearchEngine::new(&project)
        .expect("create search engine")
        .search_pattern(&query)
        .expect("search project");

    assert_eq!(
        results,
        vec![
            SearchResult {
                game_id: 1,
                black_player: None,
                white_player: None,
                game_date: None,
                result: None,
                event: None,
                komi: None,
                occurrences: vec![SearchOccurrence {
                    move_number: 1,
                    side_to_move: Some(Colour::White),
                    ko_point: None,
                    left: Some(0),
                    bottom: Some(0),
                    transformation: Some(PatternTransformation::Identity,),
                    colours_reversed: Some(false),
                }],
            },
            SearchResult {
                game_id: 2,
                black_player: None,
                white_player: None,
                game_date: None,
                result: None,
                event: None,
                komi: None,
                occurrences: vec![SearchOccurrence {
                    move_number: 1,
                    side_to_move: Some(Colour::White),
                    ko_point: None,
                    left: Some(0),
                    bottom: Some(0),
                    transformation: Some(PatternTransformation::Identity,),
                    colours_reversed: Some(false),
                }],
            },
        ]
    );
}
