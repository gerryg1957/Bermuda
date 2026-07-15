mod commands;
mod database;
mod importer;

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

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

        Command::Import { sgf, output } => commands::import_sgf(sgf, output),

        Command::Inspect { input } => commands::inspect_move_file(input),

        Command::Replay { input, move_number } => commands::replay_move_file(input, move_number),
    }
}

fn import_one(database: PathBuf, source: String, version: String, sgf: PathBuf) -> Result<()> {
    let mut importer = importer::Importer::open(&database)?;

    match importer.import_file(&source, &version, &sgf)? {
        importer::ImportOutcome::Imported { game_id, move_file } => {
            println!("Imported new game: {game_id}");
            println!("Move file: {}", move_file.display());
        }

        importer::ImportOutcome::AddedSource { game_id } => {
            println!("Game already existed: {game_id}");
            println!("Added source metadata: {source} {version}");
        }

        importer::ImportOutcome::AlreadyImported { game_id } => {
            println!("Already imported from this source and path: game {game_id}");
        }

        importer::ImportOutcome::SkippedBoardSize { board_size } => {
            println!("Skipped: unsupported professional board size {board_size}x{board_size}");
        }
    }

    Ok(())
}
