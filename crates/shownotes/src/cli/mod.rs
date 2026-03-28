// Copyright (C) 2026 Jayson Lennon
// 
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as
// published by the Free Software Foundation, either version 3 of the
// License, or (at your option) any later version.
// 
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
// 
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

use std::path::PathBuf;

use clap::{Parser, Subcommand};
use clap_verbosity_flag::{Verbosity, WarnLevel};
use error_stack::{Report, ResultExt, fmt::ColorMode};
use marked_path::CanonicalPath;

use crate::app::App;
use crate::feat::tracing;
use crate::cli::{
    action::{ActionCommands, run_action_mpv},
    generate::run_generate,
    notes::{NotesCommand, run_notes_command},
    sources::{SourcesCommands, run_sources_command},
    tui::run_tui,
};
use crate::feat::config::Config;
use crate::system_ctx::SystemCtx;

pub mod action;
pub mod generate;
pub mod notes;
pub mod sources;
pub mod tui;

/// CLI arguments for the shownotes application.
///
/// Defines the command-line interface including global options like
/// the database path and subcommands for different modes of operation.
#[derive(Parser)]
#[command(name = "shownotes")]
#[command(about = "TUI playlist manager for mpv with notes support")]
#[rustfmt::skip]
pub struct Args {
    #[arg(long, env = "SHOWNOTES_DB_PATH", default_value = "/mnt/zed/work/youtube/notes.db")]
    pub db_path: PathBuf,

    #[command(flatten)]
    pub verbosity: Verbosity<WarnLevel>,

    /// Path to the tracing log file
    #[arg(long, default_value = "/mnt/zed/work/youtube/shownotes.log")]
    pub tracing_log: PathBuf,

    /// Also output traces to terminal
    #[arg(long)]
    pub tracing_terminal: bool,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

/// Available CLI subcommands.
///
/// Each variant represents a different mode of operation, from the
/// interactive TUI to one-shot commands for notes and sources management.
#[derive(Subcommand)]
pub enum Commands {
    /// Run the TUI (default when no command specified)
    Tui {
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

        /// Working directory path
        #[arg(default_value = ".")]
        path: PathBuf,
    },
}

#[derive(Debug, wherror::Error)]
#[error(debug)]
pub struct RunError;

fn create_app(
    db_path: &std::path::Path,
    library_path: &std::path::Path,
    socket_path: &str,
    rt: tokio::runtime::Runtime,
) -> Result<App, Report<RunError>> {
    let handle = rt.handle().clone();
    let canonical_library = CanonicalPath::from_path(library_path).change_context(RunError)?;
    let ctx = rt
        .block_on(SystemCtx::new(
            &db_path.to_string_lossy(),
            Config::default(),
            canonical_library,
            socket_path.to_string(),
            handle,
        ))
        .change_context(RunError)?;
    Ok(App::new(ctx, rt))
}

/// Runs the CLI application.
///
/// # Errors
///
/// Returns an error if any command fails to execute.
#[rustfmt::skip]
pub fn run() -> Result<(), Report<RunError>> {
    Report::set_color_mode(ColorMode::None);

    let args = Args::parse();
    tracing::init(args.verbosity, Some(&args.tracing_log), args.tracing_terminal)
        .change_context(RunError)?;

    let rt = tokio::runtime::Runtime::new().change_context(RunError)?;

    match args.command.unwrap_or(Commands::Tui {
        socket: PathBuf::from("/tmp/mpvsocket"),
        path: PathBuf::from("."),
    }) {
        Commands::Tui { socket, path } => run_tui(socket, &args.db_path, path, rt),
        Commands::Action { action } => match action {
            ActionCommands::Mpv { path, socket } => {
                let mut app = create_app(&args.db_path, &std::env::current_dir().change_context(RunError)?, &socket.to_string_lossy(), rt)?;
                run_action_mpv(&path, &socket, &mut app)
            }
        },
        Commands::Notes { notes_cmd } => {
            let mut app = create_app(&args.db_path, &std::env::current_dir().change_context(RunError)?, "", rt)?;
            run_notes_command(notes_cmd, &mut app)
        }
        Commands::Sources { sources_cmd } => {
            let mut app = create_app(&args.db_path, &std::env::current_dir().change_context(RunError)?, "", rt)?;
            run_sources_command(sources_cmd, &mut app)
        }
        Commands::Generate { format, path } => {
            let mut app = create_app(&args.db_path, &path, "", rt)?;
            run_generate(&format, &path, &mut app)
        }
    }
}
