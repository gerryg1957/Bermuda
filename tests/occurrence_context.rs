use moyodb::{
    BoardEdges, Colour, GameRecord, LocalActivity, Metadata, Move, Pattern, PatternCell,
    PatternMatch, PatternTransformation, measure_local_activity,
};

fn point(board_size: u8, x: u8, y: u8) -> u16 {
    u16::from(y) * u16::from(board_size) + u16::from(x)
}

fn move_at(board_size: u8, colour: Colour, x: u8, y: u8) -> Move {
    Move {
        colour,
        point: Some(point(board_size, x, y)),
    }
}

fn pass(colour: Colour) -> Move {
    Move {
        colour,
        point: None,
    }
}

fn record(moves: Vec<Move>) -> GameRecord {
    GameRecord {
        board_size: 19,
        setup: Vec::new(),
        moves,
        metadata: Metadata {
            black_player: None,
            white_player: None,
            date: None,
            event: None,
            result: None,
            komi: None,
            handicap: None,
        },
    }
}

fn pattern(width: u8, height: u8) -> Pattern {
    Pattern {
        width,
        height,
        cells: vec![PatternCell::Empty; usize::from(width) * usize::from(height)],
        edges: BoardEdges {
            left: false,
            right: false,
            bottom: false,
            top: false,
        },
    }
}

fn appearance(
    first_move: usize,
    last_move: usize,
    left: u8,
    bottom: u8,
    transformation: PatternTransformation,
) -> PatternMatch {
    PatternMatch {
        game_id: 1,
        move_number: first_move,
        last_move_number: last_move,
        side_to_move: Colour::Black,
        ko_point: None,
        left,
        bottom,
        transformation,
        colours_reversed: false,
    }
}

#[test]
fn records_first_move_at_each_distance_threshold_from_appearance_start() {
    let board_size = 19;

    /*
     * Match rectangle is x=5..7, y=5..7.
     *
     * The appearance begins after move 2 and is declared to persist through
     * move 5. A distance-three move therefore occurs while the exact
     * appearance is still alive. Measurement deliberately starts at the
     * first matching position rather than at the end of the appearance.
     */
    let game = record(vec![
        move_at(board_size, Colour::Black, 0, 0), // 1: before appearance
        move_at(board_size, Colour::White, 18, 18), // 2: appearance begins
        pass(Colour::Black),                      // 3: ignored
        move_at(board_size, Colour::White, 10, 7), // 4: distance 3
        move_at(board_size, Colour::Black, 9, 7), // 5: distance 2
        move_at(board_size, Colour::White, 8, 7), // 6: distance 1
        move_at(board_size, Colour::Black, 7, 7), // 7: inside
    ]);

    let found = appearance(2, 5, 5, 5, PatternTransformation::Identity);
    let activity = measure_local_activity(&game, &pattern(3, 3), &found);

    assert_eq!(
        activity.first_within_three,
        Some(moyodb::NearbyMove {
            move_number: 4,
            x: 10,
            y: 7,
            distance: 3,
        })
    );

    assert_eq!(
        activity.first_within_two,
        Some(moyodb::NearbyMove {
            move_number: 5,
            x: 9,
            y: 7,
            distance: 2,
        })
    );

    assert_eq!(
        activity.first_within_one,
        Some(moyodb::NearbyMove {
            move_number: 6,
            x: 8,
            y: 7,
            distance: 1,
        })
    );

    assert_eq!(
        activity.first_inside,
        Some(moyodb::NearbyMove {
            move_number: 7,
            x: 7,
            y: 7,
            distance: 0,
        })
    );

    assert_eq!(
        activity
            .first_within_three
            .expect("distance-three move")
            .delay_moves_from(found.move_number),
        2
    );
}

#[test]
fn uses_the_transformed_match_rectangle_dimensions() {
    let board_size = 19;

    /*
     * Original pattern is 2 x 4. A clockwise quarter turn makes the matched
     * rectangle 4 x 2 at x=5..8, y=5..6. Therefore (8, 6) is inside it.
     */
    let game = record(vec![
        move_at(board_size, Colour::Black, 0, 0),
        move_at(board_size, Colour::White, 8, 6),
    ]);

    let found = appearance(1, 1, 5, 5, PatternTransformation::Rotate90Clockwise);

    let activity = measure_local_activity(&game, &pattern(2, 4), &found);

    let expected = moyodb::NearbyMove {
        move_number: 2,
        x: 8,
        y: 6,
        distance: 0,
    };

    assert_eq!(activity.first_inside, Some(expected));
    assert_eq!(activity.first_within_one, Some(expected));
    assert_eq!(activity.first_within_two, Some(expected));
    assert_eq!(activity.first_within_three, Some(expected));
}

#[test]
fn leaves_thresholds_empty_when_only_passes_or_distant_moves_follow() {
    let board_size = 19;

    let game = record(vec![
        move_at(board_size, Colour::Black, 0, 0),
        pass(Colour::White),
        move_at(board_size, Colour::Black, 18, 18),
        pass(Colour::White),
    ]);

    let found = appearance(1, 1, 5, 5, PatternTransformation::Identity);

    assert_eq!(
        measure_local_activity(&game, &pattern(3, 3), &found),
        LocalActivity::default()
    );
}
