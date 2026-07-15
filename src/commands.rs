use anyhow::{Context, Result, bail};
use moyodb_core::{
    Board, Color, GameRecord, SetupStone, extract_main_variation, parse_collection, read_move_file,
    write_move_file,
};
use std::{fs, path::PathBuf};

pub fn import_sgf(sgf: PathBuf, output: PathBuf) -> Result<()> {
    let bytes = fs::read(&sgf).with_context(|| format!("reading {}", sgf.display()))?;
    let collection = parse_collection(&bytes).context("parsing SGF collection")?;
    let record = extract_main_variation(&collection).context("extracting main variation")?;

    moyodb_core::game::replay(&record).context("validating game by replaying it")?;

    if let Some(parent) = output.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent)
            .with_context(|| format!("creating output directory {}", parent.display()))?;
    }

    write_move_file(&output, &record).with_context(|| format!("writing {}", output.display()))?;

    println!(
        "Imported: {} vs {}",
        display_or_unknown(&record.metadata.black_player),
        display_or_unknown(&record.metadata.white_player)
    );
    println!("Board size: {}x{}", record.board_size, record.board_size);
    println!("Setup edits: {}", record.setup.len());
    println!("Moves: {}", record.moves.len());
    println!("Output: {}", output.display());

    Ok(())
}

pub fn inspect_move_file(input: PathBuf) -> Result<()> {
    let record = read_move_file(&input).with_context(|| format!("reading {}", input.display()))?;

    let black_moves = record
        .moves
        .iter()
        .filter(|mv| mv.color == Color::Black)
        .count();

    let white_moves = record.moves.len() - black_moves;
    let passes = record.moves.iter().filter(|mv| mv.point.is_none()).count();

    println!("File: {}", input.display());
    println!("Board size: {}x{}", record.board_size, record.board_size);
    println!(
        "Black: {}",
        display_or_unknown(&record.metadata.black_player)
    );
    println!(
        "White: {}",
        display_or_unknown(&record.metadata.white_player)
    );

    print_optional("Date", &record.metadata.date);
    print_optional("Event", &record.metadata.event);
    print_optional("Result", &record.metadata.result);

    if let Some(komi) = record.metadata.komi {
        println!("Komi: {komi}");
    }

    if let Some(handicap) = record.metadata.handicap {
        println!("Handicap: {handicap}");
    }

    println!("Setup edits: {}", record.setup.len());
    println!("Moves: {}", record.moves.len());
    println!("Black moves: {black_moves}");
    println!("White moves: {white_moves}");
    println!("Passes: {passes}");

    Ok(())
}

pub fn replay_move_file(input: PathBuf, move_number: Option<usize>) -> Result<()> {
    let record = read_move_file(&input).with_context(|| format!("reading {}", input.display()))?;
    let requested = move_number.unwrap_or(record.moves.len());

    if requested > record.moves.len() {
        bail!(
            "requested move {requested}, but the record contains only {} moves",
            record.moves.len()
        );
    }

    let board = replay_to(&record, requested)?;

    println!(
        "{} vs {} — after {} of {} moves",
        display_or_unknown(&record.metadata.black_player),
        display_or_unknown(&record.metadata.white_player),
        requested,
        record.moves.len()
    );

    print_board(&board);

    Ok(())
}

fn replay_to(record: &GameRecord, move_count: usize) -> Result<Board> {
    let mut board = Board::new(record.board_size).context("creating board")?;

    for setup in &record.setup {
        match *setup {
            SetupStone::Add { color, point } => board
                .set_setup(color, point)
                .with_context(|| format!("applying setup stone at point {point}"))?,

            SetupStone::Remove { point } => board
                .clear_setup(point)
                .with_context(|| format!("removing setup stone at point {point}"))?,
        }
    }

    for (index, &mv) in record.moves.iter().take(move_count).enumerate() {
        board
            .play(mv)
            .with_context(|| format!("replaying move {}", index + 1))?;
    }

    Ok(board)
}

fn print_board(board: &Board) {
    let size = board.size();
    let label_width = usize::from(size).to_string().len();

    for y in 0..size {
        let row_number = size - y;
        print!("{row_number:>label_width$} ");

        for x in 0..size {
            let point = u16::from(y) * u16::from(size) + u16::from(x);

            let symbol = match board.color_at(point) {
                Some(Color::Black) => 'X',
                Some(Color::White) => 'O',
                None => '.',
            };

            print!("{symbol} ");
        }

        println!();
    }

    print!("{} ", " ".repeat(label_width));

    for x in 0..size {
        print!("{} ", go_column(x));
    }

    println!();

    if let Some(point) = board.ko_point() {
        let x = (point % u16::from(size)) as u8;
        let y = (point / u16::from(size)) as u8;

        println!("Ko point: {}{}", go_column(x), size - y);
    }
}

fn go_column(x: u8) -> char {
    let offset = if x >= 8 { 1 } else { 0 };
    char::from(b'A' + x + offset)
}

fn display_or_unknown(value: &Option<String>) -> &str {
    value.as_deref().unwrap_or("Unknown")
}

fn print_optional(label: &str, value: &Option<String>) {
    if let Some(value) = value {
        println!("{label}: {value}");
    }
}
