use super::CommandResult;

pub fn format_output(result: &CommandResult) -> String {
    match result {
        CommandResult::SourcesAdded { path, .. } => {
            format!("Added source to: {}", path.display())
        }
        CommandResult::SourcesList { path, urls } => {
            if urls.is_empty() {
                format!("No sources found for: {}", path.display())
            } else {
                urls.join("\n")
            }
        }
        CommandResult::SourcesEdited { path } => {
            format!("Updated sources: {}", path.display())
        }
        CommandResult::NotesAdded { paths } => paths
            .iter()
            .map(|p| format!("Note added: {}", p.display()))
            .collect::<Vec<_>>()
            .join("\n"),
        CommandResult::NotesSearch { paths, .. } | CommandResult::NotesFuzzy { paths, .. } => {
            paths.join("\n")
        }
        CommandResult::NotesGenerated { output } => output.clone(),
        CommandResult::MpvLoaded { path } => {
            format!("Loaded: {}", path.display())
        }
    }
}
