mod commands;
mod database;
mod indexer;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use moyodb::{
    import_directory,
    importer::{ImportOutcome, Importer},
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

#[derive(Debug, Subcommand)]
enum Command {
    /// Create a new MoyoDB database directory.
    Init {
        /// Directory in which to create the database.
        database: PathBuf,
    },

    /// Import one SGF into an initialised MoyoDB database.
    ImportOne {
        /// MoyoDB database directory.
        database: PathBuf,

        /// Source collection name, for example GoGoD or go4go.
        source: String,

        /// Source release or update version.
        version: String,

        /// SGF file to import.
        sgf: PathBuf,
    },
    /// Import every SGF below a directory.
    ImportDir {
        /// MoyoDB database directory.
        database: PathBuf,

        /// Source collection name, for example GoGoD or go4go.
        source: String,

        /// Source release or update version.
        version: String,

        /// Directory containing SGF files.
        directory: PathBuf,
    },
    /// Explore an indexed position from a game and move number.
    ExplorePosition {
        /// MoyoDB database directory.
        database: PathBuf,

        /// Database game ID.
        game_id: i64,

        /// Position after this move number.
        move_number: usize,
    },
    /// Build or resume the exact-position index.
    BuildPositionIndex {
        /// MoyoDB database directory.
        database: PathBuf,
    },
    /// Find occurrences of an exact position fingerprint.
    FindPosition {
        /// MoyoDB database directory.
        database: PathBuf,

        /// SHA-256 exact-position fingerprint in hexadecimal.
        fingerprint: String,
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
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Init { database } => database::initialise(&database),

        Command::ImportOne {
            database,
            source,
            version,
            sgf,
        } => import_one(database, source, version, sgf),

        Command::ImportDir {
            database,
            source,
            version,
            directory,
        } => import_sgf_directory(database, source, version, directory),

        Command::Import { sgf, output } => commands::import_sgf(sgf, output),

        Command::Inspect { input } => commands::inspect_move_file(input),

        Command::FindPosition {
            database,
            fingerprint,
        } => find_position(database, fingerprint),

        Command::ExplorePosition {
            database,
            game_id,
            move_number,
        } => explore_position(database, game_id, move_number),

        Command::Replay { input, move_number } => commands::replay_move_file(input, move_number),
        Command::BuildPositionIndex { database } => build_position_index(database),
    }
}

fn import_one(project_path: PathBuf, source: String, version: String, sgf: PathBuf) -> Result<()> {
    let project_manager = ProjectManager::new();
    let project = project_manager.open(&project_path)?;

    let mut importer = Importer::open_project(&project)?;

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

    let mut indexer = indexer::PositionIndexer::open(&project.database_root())?;
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

fn find_position(project_path: PathBuf, fingerprint: String) -> Result<()> {
    let project_manager = ProjectManager::new();
    let project = project_manager.open(&project_path)?;

    let indexer = indexer::PositionIndexer::open(&project.database_root())?;
    let matches = indexer.find_exact_position(&fingerprint)?;

    println!("Matches: {}", matches.len());

    for position_match in matches {
        let side = match position_match.side_to_move {
            moyodb::Color::Black => "Black",
            moyodb::Color::White => "White",
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

fn explore_position(project_path: PathBuf, game_id: i64, move_number: usize) -> Result<()> {
    let project_manager = ProjectManager::new();
    let project = project_manager.open(&project_path)?;

    let indexer = indexer::PositionIndexer::open(&project.database_root())?;

    let matches = indexer.find_matches_from_game(game_id, move_number)?;

    println!("Game {} move {}", game_id, move_number);

    println!("Matches: {}", matches.len());
    println!();

    for m in matches {
        let side = match m.side_to_move {
            moyodb::Color::Black => "Black",
            moyodb::Color::White => "White",
        };

        println!(
            "Game {:>6}   Move {:>5}   {} to move",
            m.game_id, m.move_number, side,
        );

        if let Some(ko) = m.ko_point {
            println!("           Ko point: {}", ko);
        }
    }

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
