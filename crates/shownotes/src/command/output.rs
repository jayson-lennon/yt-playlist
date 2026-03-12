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
    }
}
