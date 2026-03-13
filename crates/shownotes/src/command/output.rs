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
    }
}
