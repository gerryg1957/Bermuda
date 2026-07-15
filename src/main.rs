mod commands;
mod database;

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
        Command::Import { sgf, output } => commands::import_sgf(sgf, output),
        Command::Inspect { input } => commands::inspect_move_file(input),
        Command::Replay { input, move_number } => commands::replay_move_file(input, move_number),
    }
}
