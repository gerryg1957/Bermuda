use moyodb_core::{
    board::{BoardError, Color, Move},
    extract_main_variation,
    game::replay,
    parse_collection,
    read_move_file,
    write_move_file,
};

#[test]
fn parses_escaped_values_and_variations() {
    let sgf = br#"(;FF[4]GM[1]SZ[19]PB[Lee\] Sedol];B[pd](;W[dd])(;W[qp]))"#;
    let collection = parse_collection(sgf).unwrap();
    assert_eq!(collection.trees[0].sequence[0].first("PB"), Some("Lee] Sedol"));
    assert_eq!(collection.trees[0].variations.len(), 2);
}

#[test]
fn extracts_first_variation_and_pass() {
    let sgf = b"(;FF[4]GM[1]SZ[19]AB[dd]AW[pd];B[](;W[qq];B[dc])(;W[pp]))";
    let record = extract_main_variation(&parse_collection(sgf).unwrap()).unwrap();
    assert_eq!(record.setup.len(), 2);
    assert_eq!(record.moves.len(), 3);
    assert_eq!(record.moves[0].point, None);
    assert_eq!(record.moves[1].point, Some(16 * 19 + 16));
}

#[test]
fn captures_a_stone() {
    let mut board = moyodb_core::Board::new(5).unwrap();
    let p = |x: u8, y: u8| u16::from(y) * 5 + u16::from(x);
    board.play(Move { color: Color::White, point: Some(p(1, 1)) }).unwrap();
    board.play(Move { color: Color::Black, point: Some(p(0, 1)) }).unwrap();
    board.play(Move { color: Color::Black, point: Some(p(1, 0)) }).unwrap();
    board.play(Move { color: Color::Black, point: Some(p(2, 1)) }).unwrap();
    let captured = board.play(Move { color: Color::Black, point: Some(p(1, 2)) }).unwrap();
    assert_eq!(captured, vec![p(1, 1)]);
    assert_eq!(board.color_at(p(1, 1)), None);
}

#[test]
fn enforces_simple_ko_and_pass_clears_it() {
    let mut board = moyodb_core::Board::new(5).unwrap();
    let p = |x: u8, y: u8| u16::from(y) * 5 + u16::from(x);
    for &(c, x, y) in &[
        (Color::Black, 0, 1), (Color::Black, 1, 0), (Color::Black, 1, 2),
        (Color::White, 1, 1), (Color::White, 2, 0), (Color::White, 2, 2), (Color::White, 3, 1),
    ] { board.set_setup(c, p(x, y)).unwrap(); }
    board.play(Move { color: Color::Black, point: Some(p(2, 1)) }).unwrap();
    assert_eq!(board.ko_point(), Some(p(1, 1)));
    assert_eq!(board.play(Move { color: Color::White, point: Some(p(1, 1)) }), Err(BoardError::Ko(p(1, 1))));
    board.play(Move { color: Color::White, point: None }).unwrap();
    assert_eq!(board.ko_point(), None);
}

#[test]
fn compact_move_file_round_trip() {
    let sgf = b"(;FF[4]GM[1]SZ[9]PB[Black]PW[White]KM[6.5]HA[2]AB[cc][gg];W[];B[dd])";
    let record = extract_main_variation(&parse_collection(sgf).unwrap()).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("game.moves");
    write_move_file(&path, &record).unwrap();
    let decoded = read_move_file(&path).unwrap();
    assert_eq!(decoded, record);
    replay(&decoded).unwrap();
}
