mod generate;
mod launcher;
mod mpv;
pub mod notes;
mod output;
mod sources;

use std::path::PathBuf;

use error_stack::Report;

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
    /// Adds a source URL to a file.
    SourcesAdd { path: PathBuf, url: String },
    /// Lists source URLs for a file.
    SourcesList { path: PathBuf },
    /// Opens the sources file for editing in external editor.
    SourcesEdit { path: PathBuf },

    /// Adds notes for the specified files.
    NotesAdd { paths: Vec<PathBuf> },
    /// Searches notes matching a query string.
    NotesSearch { query: String, create_symlinks: bool },
    /// Performs fuzzy search on notes.
    NotesFuzzy { create_symlinks: bool },

    /// Generates formatted show notes from a playlist.
    GenerateNotes { format: String, playlist_path: PathBuf },

    /// Loads a file in mpv via the given socket.
    MpvLoad { path: PathBuf, socket: PathBuf },
    /// Launches a file with optional custom command or default opener.
    LaunchFile { path: PathBuf, command: Option<String>, socket_path: String },
    /// Loads the current playlist into mpv.
    MpvLoadPlaylist { paths: Vec<PathBuf> },
    /// Spawns a new mpv process if not already running.
    MpvSpawn { socket_path: String },
}

/// Results from command execution.
///
/// Contains the outcome of executing a command, providing type-safe
/// result variants for each possible command outcome.
#[derive(Debug)]
pub enum CommandResult {
    /// Confirms a source URL was added for the path.
    SourcesAdded { path: PathBuf, url: String },
    /// Contains the list of source URLs for the path.
    SourcesList { path: PathBuf, urls: Vec<String> },
    /// Confirms sources file was edited.
    SourcesEdited { path: PathBuf },

    /// Contains the resolved paths where notes were added.
    NotesAdded { paths: Vec<PathBuf> },
    /// Contains matching paths and count of symlinks created.
    NotesSearch { paths: Vec<String>, symlinks_created: usize },
    /// Contains fuzzy-matched paths and count of symlinks created.
    NotesFuzzy { paths: Vec<String>, symlinks_created: usize },

    /// Contains the generated show notes output.
    NotesGenerated { output: String },

    /// Confirms the file was loaded in mpv.
    MpvLoaded { path: PathBuf },
    /// Contains result of file launch operation.
    FileLaunched { path: PathBuf, used_default_opener: bool },
    /// Confirms playlist was loaded into mpv.
    MpvPlaylistLoaded { count: usize },
    /// Confirms mpv was spawned or was already running.
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
        Command::GenerateNotes { format, playlist_path } => {
            let output = generate::execute(services, &playlist_path, &format).await?;
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
