//! # Command Execution System
//!
//! This module is the central command execution system implementing the Command pattern.
//! It provides a single entry point for all business logic operations in the application.
//!
//! ## Core Types
//!
//! - [`Command`]: An enum defining all executable operations in the application, including
//!   sources management, notes operations, mpv control, playlist operations, and more.
//! - [`CommandResult`]: An enum providing type-safe outcomes for each command variant,
//!   enabling callers to receive structured feedback about command execution.
//! - [`execute()`]: The dispatch function that routes commands to their domain-specific handlers.
//!
//! ## Architecture Role
//!
//! This module acts as the bridge between the presentation layer (CLI/TUI) and the service layer:
//!
//! - Both CLI subcommands and TUI actions funnel through this module
//! - Domain-specific logic is delegated to sub-modules:
//!   - [`sources`]: Source file management operations
//!   - [`notes`]: Notes creation, search, and alias management
//!   - [`mpv`]: MPV player control operations
//!   - [`launcher`]: File launching with fallback handling
//!   - [`generate`]: Notes generation from templates
//!   - [`playlist`]: Playlist and library management
//!
//! ## Usage Flow
//!
//! 1. Presentation layer constructs a [`Command`] variant
//! 2. Calls [`execute(ctx, command)`][`execute()`] with a [`SystemCtx`]
//! 3. Receives a [`CommandResult`] for feedback to the user
//!
//! ## Example Flows
//!
//! ### TUI Flow
//! ```text
//! Keymap → TuiAction → action_handler → Command → execute() → CommandResult
//! ```
//!
//! ### CLI Flow
//! ```text
//! Args → subcommand handler → Command → execute() → CommandResult
//! ```
//!
//! ## Adding New Commands
//!
//! To add a new command:
//! 1. Add a variant to [`Command`]
//! 2. Add a corresponding variant to [`CommandResult`]
//! 3. Add a match arm in [`execute()`] that delegates to the appropriate sub-module
//! 4. Implement the domain logic in the relevant sub-module

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
    MpvTogglePlay,

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
    MpvToggledPlay,

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
        Command::MpvTogglePlay => {
            mpv::toggle_play(ctx)?;
            Ok(CommandResult::MpvToggledPlay)
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
            playlist::refresh_library(ctx, None).await
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
