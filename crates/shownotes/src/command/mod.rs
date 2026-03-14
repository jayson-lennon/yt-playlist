mod generate;
mod launcher;
mod mpv;
pub mod notes;
mod output;
mod sources;
mod playlist;

use std::path::PathBuf;

use error_stack::Report;
use marked_path::CanonicalPath;

use crate::system_ctx::SystemCtx;
use crate::common::domain::{ItemPath, PlaylistItem};

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

    AliasSet { path: CanonicalPath, workspace: CanonicalPath, alias: String },
    AliasRemove { path: CanonicalPath, workspace: CanonicalPath },

    PlaylistLoad,
    PlaylistSave {
        playlist_items: Vec<PlaylistItem>,
        library_items: Vec<PlaylistItem>,
    },
    LibraryRefresh,
    UrlAdd { url: String },
    AliasRename { path: ItemPath, alias: String },
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

    AliasSet { path: CanonicalPath, alias: String },
    AliasRemoved { path: CanonicalPath },

    PlaylistLoaded {
        playlist_items: Vec<PlaylistItem>,
        virtual_library_items: Vec<PlaylistItem>,
    },
    PlaylistSaved,
    LibraryRefreshed { items: Vec<PlaylistItem> },
    UrlAdded { item: PlaylistItem },
    AliasRenamed { path: ItemPath, alias: String },
}

/// # Errors
///
/// Returns an error if the command execution fails.
pub async fn execute(
    ctx: &SystemCtx,
    command: Command,
) -> Result<CommandResult, Report<CommandError>> {
    match command {
        Command::SourcesAdd { path, url } => {
            sources::add(ctx, &path, &url).await?;
            Ok(CommandResult::SourcesAdded { path, url })
        }
        Command::SourcesList { path } => {
            let urls = sources::list(ctx, &path).await?;
            Ok(CommandResult::SourcesList { path, urls })
        }
        Command::SourcesEdit { path } => {
            sources::edit(ctx, &path).await?;
            Ok(CommandResult::SourcesEdited { path })
        }
        Command::NotesAdd { paths } => {
            let resolved_paths = notes::add(ctx, paths).await?;
            Ok(CommandResult::NotesAdded { paths: resolved_paths })
        }
        Command::NotesSearch { query, create_symlinks } => {
            let (paths, symlinks_created) = notes::search(ctx, &query, create_symlinks).await?;
            Ok(CommandResult::NotesSearch { paths, symlinks_created })
        }
        Command::NotesFuzzy { create_symlinks } => {
            let (paths, symlinks_created) = notes::fuzzy(ctx, create_symlinks).await?;
            Ok(CommandResult::NotesFuzzy { paths, symlinks_created })
        }
        Command::GenerateNotes { format, working_directory } => {
            let output = generate::execute(ctx, &working_directory, &format).await?;
            Ok(CommandResult::NotesGenerated { output })
        }
        Command::MpvLoad { path, socket } => {
            mpv::load(&socket, &path)?;
            Ok(CommandResult::MpvLoaded { path })
        }
        Command::LaunchFile { path, command, socket_path } => {
            let result = launcher::launch(ctx, &path, command.as_deref(), &socket_path)?;
            Ok(CommandResult::FileLaunched { path, used_default_opener: result.used_default_opener })
        }
        Command::MpvLoadPlaylist { paths } => {
            mpv::load_playlist(ctx, &paths)?;
            Ok(CommandResult::MpvPlaylistLoaded { count: paths.len() })
        }
        Command::MpvSpawn { socket_path } => {
            let was_already_running = mpv::spawn(ctx, &socket_path)?;
            Ok(CommandResult::MpvSpawned { was_already_running })
        }
        Command::AliasSet { path, workspace, alias } => {
            let alias_clone = alias.clone();
            notes::set_alias(ctx, &path, &workspace, &alias).await?;
            Ok(CommandResult::AliasSet { path, alias: alias_clone })
        }
        Command::AliasRemove { path, workspace } => {
            let path_clone = path.clone();
            notes::remove_alias(ctx, &path, &workspace).await?;
            Ok(CommandResult::AliasRemoved { path: path_clone })
        }
        Command::PlaylistLoad => {
            playlist::load_playlist(ctx).await
        }
        Command::PlaylistSave { playlist_items, library_items } => {
            playlist::save_playlist(ctx, &playlist_items, &library_items).await
        }
        Command::LibraryRefresh => {
            playlist::refresh_library(ctx).await
        }
        Command::UrlAdd { url } => {
            let item = playlist::add_url(&url);
            Ok(CommandResult::UrlAdded { item })
        }
        Command::AliasRename { path, alias } => {
            playlist::rename_alias(ctx, &path, &alias).await
        }
    }
}
