use std::path::PathBuf;

use clap::{Parser, Subcommand};
use error_stack::{Report, fmt::ColorMode};

use crate::cli::{
    action::{ActionCommands, run_action_mpv},
    generate::run_generate,
    notes::{NotesCommand, run_notes_command},
    sources::{SourcesCommands, run_sources_command},
    tui::run_tui,
};

pub mod action;
pub mod generate;
pub mod notes;
pub mod sources;
pub mod tui;

#[derive(Parser)]
#[command(name = "shownotes")]
#[command(about = "TUI playlist manager for mpv with notes support")]
#[rustfmt::skip]
pub struct Args {
    #[arg(long, env = "SHOWNOTES_DB_PATH", default_value = "/mnt/zed/work/youtube/notes.db")]
    pub db_path: PathBuf,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Run the TUI (default when no command specified)
    Tui {
        /// Playlist file path
        #[arg(short, long, default_value = "shownotes.toml")]
        playlist: PathBuf,

        /// mpv socket path
        #[arg(long, default_value = "/tmp/mpvsocket")]
        socket: PathBuf,

        /// Directory path for library scanning
        #[arg(default_value = ".")]
        path: PathBuf,
    },

    /// Perform an action on a file
    Action {
        #[command(subcommand)]
        action: ActionCommands,
    },

    /// Notes commands for managing file notes
    Notes {
        #[command(subcommand)]
        notes_cmd: NotesCommand,
    },

    /// Source URL commands for managing file provenance
    Sources {
        #[command(subcommand)]
        sources_cmd: SourcesCommands,
    },

    /// Generate show notes from playlist
    Generate {
        /// Output format (markdown, plain, html)
        #[arg(short, long, default_value = "markdown")]
        format: String,

        /// Playlist file path
        #[arg(short, long, default_value = "shownotes.toml")]
        playlist: PathBuf,
    },
}

#[derive(Debug, wherror::Error)]
#[error(debug)]
pub struct RunError;

/// Runs the CLI application.
///
/// # Errors
///
/// Returns an error if any command fails to execute.
#[rustfmt::skip]
pub fn run() -> Result<(), Report<RunError>> {
    Report::set_color_mode(ColorMode::None);

    let args = Args::parse();
    let rt = tokio::runtime::Runtime::new().map_err(|_| Report::new(RunError))?;
    let handle = rt.handle().clone();

    match args.command.unwrap_or(Commands::Tui {
        playlist: PathBuf::from("shownotes.toml"),
        socket: PathBuf::from("/tmp/mpvsocket"),
        path: PathBuf::from("."),
    }) {
        Commands::Tui { playlist, socket, path } => run_tui(playlist, socket, &args.db_path, path, rt),
        Commands::Action { action } => match action {
            ActionCommands::Mpv { path, socket } => run_action_mpv(&path, &socket),
        },
        Commands::Notes { notes_cmd } => run_notes_command(notes_cmd, &args.db_path, &handle),
        Commands::Sources { sources_cmd } => {
            run_sources_command(sources_cmd, &args.db_path, &handle)
        }
        Commands::Generate { format, playlist } => {
            run_generate(&format, &playlist, &args.db_path, &handle)
        }
    }
}
