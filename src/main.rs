mod commands;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use moyodb::{
    Pattern, PatternRect, PatternSearchQuery, PatternSearchScope, SearchEngine, board_display,
    game_list::{GameListQuery, GameResultFilter, PlayerColour}, import_directory, importer::ImportOutcome, indexer,
    project_manager::ProjectManager,
};
use std::{path::PathBuf, time::Instant};

#[derive(Debug, Parser)]
#[command(
    name = "moyodb",
    version,
    about = "Professional Go game database tools"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum CliPlayerColour {
    Black,
    White,
    Either,
}

impl From<CliPlayerColour> for PlayerColour {
    fn from(value: CliPlayerColour) -> Self {
        match value {
            CliPlayerColour::Black => Self::Black,
            CliPlayerColour::White => Self::White,
            CliPlayerColour::Either => Self::Either,
        }
    }
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum CliGameResult {
    Any,
    BlackWin,
    WhiteWin,
    Jigo,
    Void,
}

impl From<CliGameResult> for GameResultFilter {
    fn from(value: CliGameResult) -> Self {
        match value {
            CliGameResult::Any => Self::Any,
            CliGameResult::BlackWin => Self::BlackWin,
            CliGameResult::WhiteWin => Self::WhiteWin,
            CliGameResult::Jigo => Self::Jigo,
            CliGameResult::Void => Self::Void,
        }
    }
}


#[derive(Debug, Subcommand)]
enum Command {
    /// Create a new MoyoDB project.
    Init {
        /// Name of the project.
        name: String,

        /// Directory in which to create the project.
        project: PathBuf,
    },

    /// Import one SGF into an existing MoyoDB project.
    ImportOne {
        /// MoyoDB project directory.
        project: PathBuf,

        /// Source collection name, for example GoGoD or go4go.
        source: String,

        /// Source release or update version.
        version: String,

        /// SGF file to import.
        sgf: PathBuf,
    },
    /// Import every SGF below a directory into a MoyoDB project.
    ImportDir {
        /// MoyoDB project directory.
        project: PathBuf,

        /// Source collection name, for example GoGoD or go4go.
        source: String,

        /// Source release or update version.
        version: String,

        /// Directory containing SGF files.
        directory: PathBuf,
    },
      /// List games in a MoyoDB project.
    ListGames {
        /// MoyoDB project directory.
        project: PathBuf,

        /// Show only games involving this player.
        #[arg(long)]
        player: Option<String>,

        /// Player colour to match.
        #[arg(long, value_enum, default_value = "either")]
        colour: CliPlayerColour,

        /// Earliest game date to include, in YYYY-MM-DD form.
        #[arg(long)]
        date_from: Option<String>,

        /// Latest game date to include, in YYYY-MM-DD form.
        #[arg(long)]
        date_to: Option<String>,

        /// Game result to match.
        #[arg(long, value_enum, default_value = "any")]
        result: CliGameResult,

        /// Maximum number of games to display.
        #[arg(long, default_value_t = 200)]
        limit: u32,

        /// Number of matching games to skip.
        #[arg(long, default_value_t = 0)]
        offset: u32,
    },
    
    /// Find an indexed position from a game and move number.
    FindPosition {
        /// MoyoDB project directory.
        project: PathBuf,

        /// Database game ID.
        game_id: i64,

        /// Position after this move number.
        move_number: usize,
    },
    /// Display a position from a game and move number.
    ShowPosition {
        /// MoyoDB project directory.
        project: PathBuf,

        /// Database game ID.
        game_id: i64,

        /// Position after this move number.
        move_number: usize,
    },

    /// Build or resume the exact-position index.
    BuildPositionIndex {
        /// MoyoDB project directory.
        project: PathBuf,
    },
    /// Find occurrences of an exact position fingerprint.
    FindFingerprint {
        /// MoyoDB project directory.
        project: PathBuf,

        /// SHA-256 exact-position fingerprint in hexadecimal.
        fingerprint: String,
    },

    /// Replay a stored game from the database.
    ReplayGame {
        /// MoyoDB project directory.
        project: PathBuf,

        /// Database game ID.
        game_id: i64,

        /// Show only a specific move number.
        #[arg(long)]
        move_number: Option<usize>,

        #[arg(long)]
        from: Option<usize>,

        #[arg(long)]
        to: Option<usize>,
    },

    /// Convert the first game and its main variation from SGF to a compact move file.
    Import {
        /// Source SGF file.
        sgf: PathBuf,

        /// Destination .moves file.
        output: PathBuf,
    },

    /// Display metadata and record counts from a compact move file.
    Inspect {
        /// Input .moves file.
        input: PathBuf,
    },

    /// Replay a compact move file and print the resulting board.
    Replay {
        /// Input .moves file.
        input: PathBuf,

        /// Stop after this many moves. By default, replay the complete game.
        #[arg(short = 'n', long)]
        move_number: Option<usize>,
    },

    /// Search a game for a pattern extracted from another position.
    SearchPattern {
        /// MoyoDB project directory.
        project: PathBuf,

        /// Game containing the pattern.
        pattern_game_id: i64,

        /// Move containing the pattern.
        pattern_move_number: usize,

        /// Rectangle left coordinate.
        left: u8,

        /// Rectangle bottom coordinate.
        bottom: u8,

        /// Rectangle width.
        width: u8,

        /// Rectangle height.
        height: u8,

        /// Game to search.
        search_game_id: i64,
    },

    /// Search the complete database for a pattern extracted from a position.
    SearchPatternDatabase {
        /// MoyoDB project directory.
        project: PathBuf,

        /// Game containing the pattern.
        pattern_game_id: i64,

        /// Move containing the pattern.
        pattern_move_number: usize,

        /// Rectangle left coordinate.
        left: u8,

        /// Rectangle bottom coordinate.
        bottom: u8,

        /// Rectangle width.
        width: u8,

        /// Rectangle height.
        height: u8,
    },
}

#[derive(Debug)]
struct PatternSearchRequest {
    project_path: PathBuf,
    pattern_game_id: i64,
    pattern_move_number: usize,
    rect: PatternRect,
}

#[derive(Debug)]
struct SingleGamePatternSearchRequest {
    project_path: PathBuf,
    pattern_game_id: i64,
    pattern_move_number: usize,
    rect: PatternRect,
    search_game_id: i64,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Init { name, project } => {
            let manager = ProjectManager::new();
            manager.create(name, project)?;
            Ok(())
        }
        Command::ImportOne {
            project,
            source,
            version,
            sgf,
        } => import_one(project, source, version, sgf),

        Command::ImportDir {
            project,
            source,
            version,
            directory,
        } => import_sgf_directory(project, source, version, directory),
        Command::ListGames {
            project,
            player,
            colour,
            date_from,
            date_to,
            result,
            limit,
            offset,
        } => list_games(
            project,
            GameListQuery {
                player,
                colour: colour.into(),
                date_from,
                date_to,
                result: result.into(),
                limit,
                offset,
                ..GameListQuery::default()
            },
        ),

        Command::Import { sgf, output } => commands::import_sgf(sgf, output),

        Command::Inspect { input } => commands::inspect_move_file(input),

        Command::FindFingerprint {
            project,
            fingerprint,
        } => find_position_by_fingerprint(project, fingerprint),

        Command::FindPosition {
            project,
            game_id,
            move_number,
        } => find_position(project, game_id, move_number),

        Command::ReplayGame {
            project,
            game_id,
            move_number,
            from,
            to,
        } => replay_game(project, game_id, move_number, from, to),

        Command::SearchPattern {
            project,
            pattern_game_id,
            pattern_move_number,
            left,
            bottom,
            width,
            height,
            search_game_id,
        } => search_pattern(SingleGamePatternSearchRequest {
            project_path: project,
            pattern_game_id,
            pattern_move_number,
            rect: PatternRect {
                left,
                bottom,
                width,
                height,
            },
            search_game_id,
        }),

        Command::SearchPatternDatabase {
            project,
            pattern_game_id,
            pattern_move_number,
            left,
            bottom,
            width,
            height,
        } => search_pattern_database(PatternSearchRequest {
            project_path: project,
            pattern_game_id,
            pattern_move_number,
            rect: PatternRect {
                left,
                bottom,
                width,
                height,
            },
        }),

        Command::ShowPosition {
            project,
            game_id,
            move_number,
        } => show_position(project, game_id, move_number),

        Command::Replay { input, move_number } => commands::replay_move_file(input, move_number),
        Command::BuildPositionIndex { project } => build_position_index(project),
    }
}

fn list_games(project_path: PathBuf, query: GameListQuery) -> Result<()> {
    let project_manager = ProjectManager::new();
    let project = project_manager.open(&project_path)?;

    let catalogue = project.catalogue()?;

    let total = catalogue.count(&query)?;
    let games = catalogue.list(&query)?;

    println!("Games in project: {total}");
    println!("Showing: {}", games.len());
    println!();

    println!("ID\tDate\tBlack\tWhite\tResult");

    for game in games {
        println!(
            "{}\t{}\t{}\t{}\t{}",
            game.game_id,
            game.game_date.as_deref().unwrap_or("-"),
            game.black_player.as_deref().unwrap_or("Unknown"),
            game.white_player.as_deref().unwrap_or("Unknown"),
            game.result.as_deref().unwrap_or("-"),
        );
    }

    Ok(())
}

fn replay_game(
    project_path: PathBuf,
    game_id: i64,
    move_number: Option<usize>,
    from: Option<usize>,
    to: Option<usize>,
) -> Result<()> {
    let project_manager = ProjectManager::new();
    let project = project_manager.open(&project_path)?;

    let indexer = project.position_indexer()?;
    let game = indexer.read_game_by_id(game_id)?;
    let states = indexer.replay_game_states_by_id(game_id)?;

    let total_moves = states.len().saturating_sub(1);

    if move_number.is_some() && (from.is_some() || to.is_some()) {
        anyhow::bail!("--move-number cannot be used together with --from or --to");
    }

    if let Some(start) = from
        && start > total_moves
    {
        anyhow::bail!(
            "move {} is outside this game; valid range is 0-{}",
            start,
            total_moves
        );
    }

    if let Some(end) = to
        && end > total_moves
    {
        anyhow::bail!(
            "move {} is outside this game; valid range is 0-{}",
            end,
            total_moves
        );
    }

    if let (Some(start), Some(end)) = (from, to)
        && start > end
    {
        anyhow::bail!("--from cannot be greater than --to");
    }

    if let Some(target) = move_number
        && target > total_moves
    {
        anyhow::bail!(
            "move {} is outside this game; valid range is 0-{}",
            target,
            total_moves
        );
    }

    let range_start = from.unwrap_or(0);
    let range_end = to.unwrap_or(total_moves);

    let metadata = &game.metadata;

    println!(
        "{} vs {}",
        metadata.black_player.as_deref().unwrap_or("Unknown Black"),
        metadata.white_player.as_deref().unwrap_or("Unknown White")
    );

    if let Some(event) = &metadata.event {
        println!("Event: {}", event);
    }

    if let Some(date) = &metadata.date {
        println!("Date: {}", date);
    }

    if let Some(result) = &metadata.result {
        println!("Result: {}", result);
    }

    println!("Board: {}x{}", game.board_size, game.board_size);

    if let Some(komi) = metadata.komi {
        println!("Komi: {}", komi);
    }

    if let Some(handicap) = metadata.handicap {
        println!("Handicap: {}", handicap);
    }

    println!();

    println!("Database game ID: {}", game_id);

    match move_number {
        Some(target) => {
            println!("Move {} of {}", target, total_moves);
        }
        None if from.is_some() || to.is_some() => {
            println!(
                "Showing moves {}-{} of {}",
                range_start, range_end, total_moves
            );
        }
        None => {
            println!("Positions: {}", states.len());
            println!("Moves: {}", total_moves);
        }
    }

    println!();

    for state in states {
        let current = state.occurrence.move_number;

        if let Some(target) = move_number {
            if current != target {
                continue;
            }
        } else if current < range_start || current > range_end {
            continue;
        }

        println!("Move {}", current);

        let side = match state.occurrence.side_to_move {
            moyodb::Colour::Black => "Black",
            moyodb::Colour::White => "White",
        };

        println!("{side} to move");

        if let Some(last_move) = state.last_move {
            let colour = match last_move.colour {
                moyodb::Colour::Black => "Black",
                moyodb::Colour::White => "White",
            };

            match last_move.point {
                Some(point) => {
                    let coordinate = state.board.point_name(point)?;
                    println!("Last move: {colour} {coordinate}");
                }
                None => println!("Last move: {colour} pass"),
            }
        }

        println!();
        println!("{}", board_display::render(&state.board));
        println!();
    }

    Ok(())
}

fn import_one(project_path: PathBuf, source: String, version: String, sgf: PathBuf) -> Result<()> {
    let project_manager = ProjectManager::new();
    let project = project_manager.open(&project_path)?;

    let mut importer = project.importer()?;

    match importer.import_file(&source, &version, &sgf)? {
        ImportOutcome::Imported { game_id, move_file } => {
            println!("Imported new game: {game_id}");
            println!("Move file: {}", move_file.display());
        }

        ImportOutcome::AddedSource { game_id } => {
            println!("Game already existed: {game_id}");
            println!("Added source metadata: {source} {version}");
        }

        ImportOutcome::AlreadyImported { game_id } => {
            println!("Already imported from this source and path: game {game_id}");
        }

        ImportOutcome::SkippedBoardSize { board_size } => {
            println!("Skipped: unsupported professional board size {board_size}x{board_size}");
        }
    }

    Ok(())
}

fn build_position_index(project_path: PathBuf) -> Result<()> {
    let project_manager = ProjectManager::new();
    let project = project_manager.open(&project_path)?;

    let started = Instant::now();

    let mut indexer = project.position_indexer()?;
    let games = indexer.games_to_index(indexer::POSITION_INDEX_VERSION)?;

    let total_games = games.len();

    println!(
        "Position index version: {}",
        indexer::POSITION_INDEX_VERSION
    );
    println!("Games awaiting indexing: {total_games}");

    if total_games == 0 {
        println!("Position index is already up to date.");
        return Ok(());
    }

    let mut indexed_games = 0usize;
    let mut indexed_positions = 0u64;
    let mut errors = 0usize;

    for game in &games {
        match indexer.index_game(game, indexer::POSITION_INDEX_VERSION) {
            Ok(occurrence_count) => {
                indexed_games += 1;

                indexed_positions += u64::try_from(occurrence_count)
                    .context("position count does not fit in u64")?;
            }

            Err(error) => {
                errors += 1;

                eprintln!(
                    "Failed to index game {} from {}: {error:#}",
                    game.game_id,
                    game.move_file.display()
                );
            }
        }

        let processed = indexed_games + errors;

        if processed.is_multiple_of(1_000) || processed == total_games {
            let elapsed_seconds = started.elapsed().as_secs_f64();

            let rate = if elapsed_seconds > 0.0 {
                processed as f64 / elapsed_seconds
            } else {
                0.0
            };

            println!(
                "Processed {processed}/{total_games} games \
                 ({rate:.1} games/second)..."
            );
        }
    }

    let elapsed_seconds = started.elapsed().as_secs_f64();

    let rate = if elapsed_seconds > 0.0 {
        indexed_games as f64 / elapsed_seconds
    } else {
        0.0
    };

    println!();
    println!("Position indexing complete");
    println!("Games indexed : {indexed_games}");
    println!("Positions     : {indexed_positions}");
    println!("Errors        : {errors}");
    println!("Elapsed       : {elapsed_seconds:.2} seconds");
    println!("Rate          : {rate:.1} games/second");

    Ok(())
}

fn find_position_by_fingerprint(project_path: PathBuf, fingerprint: String) -> Result<()> {
    let project_manager = ProjectManager::new();
    let project = project_manager.open(&project_path)?;

    let indexer = project.position_indexer()?;
    let matches = indexer.find_exact_position(&fingerprint)?;

    println!("Matches: {}", matches.len());

    for position_match in matches {
        let side = match position_match.side_to_move {
            moyodb::Colour::Black => "Black",
            moyodb::Colour::White => "White",
        };

        println!(
            "Game {} — move {} — {} to move",
            position_match.game_id, position_match.move_number, side
        );

        if let Some(ko_point) = position_match.ko_point {
            println!("  Ko point: {ko_point}");
        }
    }

    Ok(())
}

fn find_position(project_path: PathBuf, game_id: i64, move_number: usize) -> Result<()> {
    let project_manager = ProjectManager::new();
    let project = project_manager.open(&project_path)?;

    let indexer = project.position_indexer()?;

    let occurrence = indexer.position_from_game(game_id, move_number)?;

    let fingerprint = occurrence
        .fingerprint
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();

    let matches = indexer.find_exact_position_with_metadata(&fingerprint)?;

    println!("Game {} move {}", game_id, move_number);

    println!("Matches: {}", matches.len());
    println!();

    for m in matches {
        let side = match m.side_to_move {
            moyodb::Colour::Black => "Black",
            moyodb::Colour::White => "White",
        };

        println!("Game {}", m.game_id);
        println!("Move {}", m.move_number);
        println!("{side} to move");

        if let Some(player) = m.black_player {
            println!("Black: {player}");
        }

        if let Some(player) = m.white_player {
            println!("White: {player}");
        }

        if let Some(event) = m.event {
            println!("Event: {event}");
        }

        if let Some(date) = m.date {
            println!("Date: {date}");
        }

        if let Some(result) = m.result {
            println!("Result: {result}");
        }

        if let Some(ko) = m.ko_point {
            println!("Ko point: {ko}");
        }

        println!();
    }

    let state = indexer.replay_board_position(game_id, move_number)?;

    println!();
    println!("{}", board_display::render(&state.board));

    let side = match state.occurrence.side_to_move {
        moyodb::Colour::Black => "Black",
        moyodb::Colour::White => "White",
    };

    println!("{side} to move");

    Ok(())
}

fn search_pattern(request: SingleGamePatternSearchRequest) -> Result<()> {
    let SingleGamePatternSearchRequest {
        project_path,
        pattern_game_id,
        pattern_move_number,
        rect,
        search_game_id,
    } = request;

    let project_manager = ProjectManager::new();
    let project = project_manager.open(&project_path)?;

    let indexer = project.position_indexer()?;

    let pattern_state = indexer.replay_board_position(pattern_game_id, pattern_move_number)?;

    let pattern = Pattern::extract(&pattern_state.board, rect)?;

    let query = PatternSearchQuery {
        pattern,
        scope: PatternSearchScope::Game(search_game_id),
    };

    let search_engine = SearchEngine::new(&indexer);
    let matches = search_engine.search_pattern(&query)?;

    println!("Found {} matches", matches.len());

    for found in matches {
        println!(
            "Game {}, move {}, left {}, bottom {}",
            found.game_id, found.move_number, found.left, found.bottom
        );
    }

    Ok(())
}

fn search_pattern_database(request: PatternSearchRequest) -> Result<()> {
    let PatternSearchRequest {
        project_path,
        pattern_game_id,
        pattern_move_number,
        rect,
    } = request;

    let project_manager = ProjectManager::new();
    let project = project_manager.open(&project_path)?;

    let indexer = project.position_indexer()?;

    let pattern_state = indexer.replay_board_position(pattern_game_id, pattern_move_number)?;

    let pattern = Pattern::extract(&pattern_state.board, rect)?;

    let query = PatternSearchQuery {
        pattern,
        scope: PatternSearchScope::Project,
    };

    let search_engine = SearchEngine::new(&indexer);
    let matches = search_engine.search_pattern(&query)?;

    println!("Found {} matches", matches.len());

    for found in matches {
        println!(
            "Game {}, move {}, left {}, bottom {}",
            found.game_id, found.move_number, found.left, found.bottom
        );
    }

    Ok(())
}

fn show_position(project_path: PathBuf, game_id: i64, move_number: usize) -> Result<()> {
    let project_manager = ProjectManager::new();
    let project = project_manager.open(&project_path)?;

    let indexer = project.position_indexer()?;
    let record = indexer.read_game_by_id(game_id)?;
    let state = indexer.replay_board_position(game_id, move_number)?;

    println!("Game {}", game_id);
    println!("Move {}", move_number);

    if let Some(player) = record.metadata.black_player {
        println!("Black: {player}");
    }

    if let Some(player) = record.metadata.white_player {
        println!("White: {player}");
    }

    if let Some(event) = record.metadata.event {
        println!("Event: {event}");
    }

    if let Some(date) = record.metadata.date {
        println!("Date: {date}");
    }

    if let Some(result) = record.metadata.result {
        println!("Result: {result}");
    }

    println!();

    let side = match state.occurrence.side_to_move {
        moyodb::Colour::Black => "Black",
        moyodb::Colour::White => "White",
    };

    println!("{side} to move");
    println!();

    if let Some(last_move) = state.last_move {
        let colour = match last_move.colour {
            moyodb::Colour::Black => "Black",
            moyodb::Colour::White => "White",
        };

        match last_move.point {
            Some(point) => {
                let coordinate = state.board.point_name(point)?;
                println!("Last move: {colour} {coordinate}");
            }
            None => {
                println!("Last move: {colour} pass");
            }
        }

        println!();
    }

    println!("{}", board_display::render(&state.board));

    Ok(())
}

fn import_sgf_directory(
    project_path: PathBuf,
    source: String,
    version: String,
    directory: PathBuf,
) -> Result<()> {
    let project_manager = ProjectManager::new();
    let project = project_manager.open(&project_path)?;

    let summary = import_directory::run(&project, &source, &version, &directory)?;

    println!();
    println!("Import complete");
    println!("Processed    : {}", summary.processed);
    println!("Imported     : {}", summary.imported);
    println!("Added sources: {}", summary.added_sources);
    println!("Duplicates   : {}", summary.duplicates);
    println!("Skipped      : {}", summary.skipped);
    println!("Errors       : {}", summary.errors);
    println!("Elapsed      : {:.2} seconds", summary.elapsed_seconds);
    println!("Rate         : {:.1} games/second", summary.rate());

    if let Some(error_log) = summary.error_log {
        println!("Error log    : {}", error_log.display());
    }

    Ok(())
}
