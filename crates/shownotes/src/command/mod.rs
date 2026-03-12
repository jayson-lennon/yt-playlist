mod generate;
mod launcher;
mod mpv;
pub mod notes;
mod output;
mod sources;

use std::path::PathBuf;

use error_stack::Report;
use marked_path::CanonicalPath;

use crate::services::Services;

pub use output::format_output;

#[derive(Debug, wherror::Error)]
#[error(debug)]
pub struct CommandError;

/// All executable commands in the application.
///
/// Represents the commands that can be executed by the command system,
/// including launching files, controlling mpv, and managing playlists.
pub enum Command {
    SourcesAdd { path: CanonicalPath, url: String },
    SourcesList { path: CanonicalPath },
    SourcesEdit { path: CanonicalPath },

    NotesAdd { paths: Vec<CanonicalPath> },
    NotesSearch { query: String, create_symlinks: bool },
    NotesFuzzy { create_symlinks: bool },

    GenerateNotes { format: String, working_directory: CanonicalPath },

    MpvLoad { path: CanonicalPath, socket: PathBuf },
    LaunchFile { path: CanonicalPath, command: Option<String>, socket_path: String },
    MpvLoadPlaylist { paths: Vec<CanonicalPath> },
    MpvSpawn { socket_path: String },
}

/// Results from command execution.
///
/// Contains the outcome of executing a command, providing type-safe
/// result variants for each possible command outcome.
#[derive(Debug)]
pub enum CommandResult {
    SourcesAdded { path: CanonicalPath, url: String },
    SourcesList { path: CanonicalPath, urls: Vec<String> },
    SourcesEdited { path: CanonicalPath },

    NotesAdded { paths: Vec<CanonicalPath> },
    NotesSearch { paths: Vec<String>, symlinks_created: usize },
    NotesFuzzy { paths: Vec<String>, symlinks_created: usize },

    NotesGenerated { output: String },

    MpvLoaded { path: CanonicalPath },
    FileLaunched { path: CanonicalPath, used_default_opener: bool },
    MpvPlaylistLoaded { count: usize },
    MpvSpawned { was_already_running: bool },
}

/// # Errors
///
/// Returns an error if the command execution fails.
pub async fn execute(
    services: &Services,
    command: Command,
) -> Result<CommandResult, Report<CommandError>> {
    match command {
        Command::SourcesAdd { path, url } => {
            sources::add(services, &path, &url).await?;
            Ok(CommandResult::SourcesAdded { path, url })
        }
        Command::SourcesList { path } => {
            let urls = sources::list(services, &path).await?;
            Ok(CommandResult::SourcesList { path, urls })
        }
        Command::SourcesEdit { path } => {
            sources::edit(services, &path).await?;
            Ok(CommandResult::SourcesEdited { path })
        }
        Command::NotesAdd { paths } => {
            let resolved_paths = notes::add(services, paths).await?;
            Ok(CommandResult::NotesAdded { paths: resolved_paths })
        }
        Command::NotesSearch { query, create_symlinks } => {
            let (paths, symlinks_created) = notes::search(services, &query, create_symlinks).await?;
            Ok(CommandResult::NotesSearch { paths, symlinks_created })
        }
        Command::NotesFuzzy { create_symlinks } => {
            let (paths, symlinks_created) = notes::fuzzy(services, create_symlinks).await?;
            Ok(CommandResult::NotesFuzzy { paths, symlinks_created })
        }
        Command::GenerateNotes { format, working_directory } => {
            let output = generate::execute(services, &working_directory, &format).await?;
            Ok(CommandResult::NotesGenerated { output })
        }
        Command::MpvLoad { path, socket } => {
            mpv::load(&socket, &path)?;
            Ok(CommandResult::MpvLoaded { path })
        }
        Command::LaunchFile { path, command, socket_path } => {
            let result = launcher::launch(services, &path, command.as_deref(), &socket_path)?;
            Ok(CommandResult::FileLaunched { path, used_default_opener: result.used_default_opener })
        }
        Command::MpvLoadPlaylist { paths } => {
            mpv::load_playlist(services, &paths)?;
            Ok(CommandResult::MpvPlaylistLoaded { count: paths.len() })
        }
        Command::MpvSpawn { socket_path } => {
            let was_already_running = mpv::spawn(services, &socket_path)?;
            Ok(CommandResult::MpvSpawned { was_already_running })
        }
    }
}
