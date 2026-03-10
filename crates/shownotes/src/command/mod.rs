mod generate;
mod mpv;
mod notes;
mod output;
mod sources;

use std::path::PathBuf;

use error_stack::Report;

use crate::services::Services;

pub use output::format_output;

#[derive(Debug, wherror::Error)]
#[error(debug)]
pub struct CommandError;

pub enum Command {
    SourcesAdd { path: PathBuf, url: String },
    SourcesList { path: PathBuf },
    SourcesEdit { path: PathBuf },

    NotesAdd { paths: Vec<PathBuf> },
    NotesSearch { query: String, create_symlinks: bool },
    NotesFuzzy { create_symlinks: bool },

    GenerateNotes { format: String, playlist_path: PathBuf },

    MpvLoad { path: PathBuf, socket: PathBuf },
}

#[derive(Debug)]
pub enum CommandResult {
    SourcesAdded { path: PathBuf, url: String },
    SourcesList { path: PathBuf, urls: Vec<String> },
    SourcesEdited { path: PathBuf },

    NotesAdded { paths: Vec<PathBuf> },
    NotesSearch { paths: Vec<String>, symlinks_created: usize },
    NotesFuzzy { paths: Vec<String>, symlinks_created: usize },

    NotesGenerated { output: String },

    MpvLoaded { path: PathBuf },
}

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
    }
}
