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

use super::CommandResult;

pub fn format_output(result: &CommandResult) -> String {
    match result {
        CommandResult::SourcesAdded { path, .. } => {
            format!("Added source to: {}", path.as_path().display())
        }
        CommandResult::SourcesList { path, urls } => {
            if urls.is_empty() {
                format!("No sources found for: {}", path.as_path().display())
            } else {
                urls.join("\n")
            }
        }
        CommandResult::SourcesEdited { path } => {
            format!("Updated sources: {}", path.as_path().display())
        }
        CommandResult::NotesAdded { paths } => paths
            .iter()
            .map(|p| format!("Note added: {}", p.as_path().display()))
            .collect::<Vec<_>>()
            .join("\n"),
        CommandResult::NotesSearch { paths, .. } | CommandResult::NotesFuzzy { paths, .. } => {
            paths.join("\n")
        }
        CommandResult::NotesGenerated { output } => output.clone(),
        CommandResult::MpvLoaded { path } => {
            format!("Loaded: {}", path.as_path().display())
        }
        CommandResult::FileLaunched {
            path,
            used_default_opener,
        } => {
            if *used_default_opener {
                format!("Opening with default opener: {}", path.as_path().display())
            } else {
                format!("Opening: {}", path.as_path().display())
            }
        }
        CommandResult::MpvPlaylistLoaded { count } => {
            format!("Loaded {count} items into mpv")
        }
        CommandResult::MpvSpawned {
            was_already_running,
        } => {
            if *was_already_running {
                "MPV already running".to_string()
            } else {
                "MPV launched".to_string()
            }
        }
        CommandResult::AliasSet { path, alias } => {
            format!("Set alias '{}' for: {}", alias, path.as_path().display())
        }
        CommandResult::AliasRemoved { path } => {
            format!("Removed alias for: {}", path.as_path().display())
        }
        CommandResult::PlaylistLoaded { .. } => "Playlist loaded".to_string(),
        CommandResult::PlaylistSaved => "Playlist saved".to_string(),
        CommandResult::LibraryRefreshed { items } => {
            format!("Library refreshed: {} items", items.len())
        }
        CommandResult::UrlAdded { item } => {
            format!("URL added: {}", item.path.to_string_lossy())
        }
        CommandResult::AliasRenamed { path, alias } => {
            format!("Renamed to '{}': {}", alias, path.to_string_lossy())
        }
        CommandResult::MpvToggledPlay => "Toggled playback".to_string(),
        CommandResult::LibraryAnalyzed { new_files_count } => {
            format!("Library analyzed: {new_files_count} new files")
        }
    }
}
