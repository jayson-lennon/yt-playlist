use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    sync::Arc,
};

use marked_path::CanonicalPath;

use crossterm::event::{self, DisableMouseCapture, EnableMouseCapture};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use error_stack::{Report, ResultExt};
use ratatui::{backend::CrosstermBackend, Terminal};

use crate::{
    app::{App, ForkAction},
    command::{Command, CommandResult},
    feat::config::{load, Config},
    feat::media_query::{CachedMedia, Ffprobe, MediaQuery, MediaQueryService},
    feat::mpv::MpvIpc,
    feat::playlist::{FileMetadata, PlaylistData},
    feat::terminal::suspend_and_run,
    services::Services,
    tui::{self},
};

use super::RunError;

enum ForkResult {
    Success(String),
    Failed(String),
    SuspendFailed,
}

fn requires_suspend(action: &ForkAction) -> bool {
    !matches!(action, ForkAction::GenerateNotes { .. })
}

fn execute_fork_action(app: &mut App, action: ForkAction) -> ForkResult {
    match action {
        ForkAction::AddNote { path } => {
            match path.as_file() {
                Some(file_path) => match add_note_for_path(app, file_path.as_path()) {
                    Ok(()) => ForkResult::Success(format!("Note added: {}", path.display())),
                    Err(e) => ForkResult::Failed(format!("Failed to add note: {e}")),
                },
                None => ForkResult::Failed("Cannot add notes to URLs".to_string()),
            }
        }
        ForkAction::FuzzyNotes => match run_fuzzy_notes(app) {
            Ok(count) => ForkResult::Success(format!("Created {count} symlink(s)")),
            Err(e) => ForkResult::Failed(format!("Fuzzy search failed: {e}")),
        },
        ForkAction::EditSources { path } => {
            match path.as_file() {
                Some(file_path) => match edit_sources_for_path(app, file_path.as_path()) {
                    Ok(()) => ForkResult::Success(format!("Updated sources: {}", path.display())),
                    Err(e) => ForkResult::Failed(format!("Failed to edit sources: {e}")),
                },
                None => ForkResult::Failed("Cannot edit sources for URLs".to_string()),
            }
        }
        ForkAction::GenerateNotes { format } => match run_generate_notes(app, &format) {
            Ok(()) => ForkResult::Success(format!("Show notes ({format}) copied to clipboard")),
            Err(e) => ForkResult::Failed(format!("Failed to generate notes: {e}")),
        },
    }
}

fn process_fork(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    app: &mut App,
    action: ForkAction,
) -> Result<(), Report<RunError>> {
    let needs_suspend = requires_suspend(&action);

    let result: Result<ForkResult, std::convert::Infallible> = if needs_suspend {
        suspend_and_run(terminal, || Ok(execute_fork_action(app, action)))
            .unwrap_or(Ok(ForkResult::SuspendFailed))
    } else {
        Ok(execute_fork_action(app, action))
    };

    let keymap = app.runtime.keymap.clone();
    terminal
        .draw(|f| tui::render(f, &app.tui_state, &keymap, &app.services))
        .change_context(RunError)?;

    let message = match result {
        Ok(ForkResult::Success(msg) | ForkResult::Failed(msg)) => msg,
        Ok(ForkResult::SuspendFailed) => "Failed to suspend terminal".to_string(),
    };
    app.tui_state.status_message = Some(message);

    Ok(())
}

/// Runs the terminal user interface.
///
/// # Errors
///
/// Returns an error if:
/// - The configuration cannot be loaded
/// - The playlist cannot be loaded
/// - File analysis fails
/// - The database cannot be accessed
/// - Terminal setup fails
pub fn run_tui(
    socket: PathBuf,
    db_path: &Path,
    library_path: PathBuf,
    rt: tokio::runtime::Runtime,
) -> Result<(), Report<RunError>> {
    use crate::tui::ItemPath;

    let config = load().change_context(RunError)?;

    let handle = rt.handle().clone();
    let core_services = rt
        .block_on(Services::new(&db_path.to_string_lossy(), handle))
        .change_context(RunError)?;

    let canonical_library_path = CanonicalPath::from_path(&library_path)
        .change_context(RunError)
        .attach("Failed to canonicalize library path")?;
    let playlist_data = rt.block_on(core_services.storage.load(&canonical_library_path)).change_context(RunError)?;
    let all_files = collect_all_files(&playlist_data, &config, &canonical_library_path);
    let ffprobe_backend: Arc<dyn MediaQuery> = Arc::new(Ffprobe);
    let files_for_migration: std::collections::HashMap<CanonicalPath, _> = playlist_data
        .files
        .iter()
        .filter_map(|(k, v)| k.as_file().map(|cp| (cp.clone(), v.clone())))
        .collect();

    let metadata: std::collections::HashMap<CanonicalPath, _> = playlist_data
        .files
        .iter()
        .filter_map(|(k, v)| k.as_file().map(|cp| (cp.clone(), v.clone())))
        .collect();

    let result = crate::feat::media_duration_analysis::analyze_files(
        &all_files,
        metadata,
        ffprobe_backend.as_ref(),
    )
    .change_context(RunError)?;

    let files: std::collections::HashMap<ItemPath, FileMetadata> = result
        .files
        .into_iter()
        .map(|(k, v)| (ItemPath::File(k), v))
        .collect();

    let analyzed_data = PlaylistData {
        working_directory: canonical_library_path.clone(),
        playlist: playlist_data.playlist.clone(),
        files: files.clone(),
    };
    rt.block_on(core_services.storage.save(&analyzed_data))
        .change_context(RunError)?;

    let durations: std::collections::HashMap<CanonicalPath, std::time::Duration> = files
        .iter()
        .filter_map(|(k, v)| {
            k.as_file()
                .and_then(|cp| v.duration.map(|d| (cp.clone(), d)))
        })
        .collect();

    let media_backend: Arc<dyn MediaQuery> = Arc::new(CachedMedia::new(durations, ffprobe_backend));

    let services = build_services(&socket, media_backend, core_services);

    enable_raw_mode().change_context(RunError)?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture).change_context(RunError)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).change_context(RunError)?;

    let mut app = App::new(
        services,
        config,
        socket.to_string_lossy().into_owned(),
        canonical_library_path,
        rt,
    );

    let migration_task = {
        let services = app.services.clone();
        let files = files_for_migration;
        app.tokio_runtime.spawn(async move {
            let _ = crate::command::notes::migrate_aliases_to_notes(&services, &files).await;
        })
    };

    let res = run_app(&mut terminal, &mut app);

    disable_raw_mode().change_context(RunError)?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )
    .change_context(RunError)?;
    terminal.show_cursor().change_context(RunError)?;

    if let Err(err) = res {
        eprintln!("Error: {err:?}");
    }

    let _ = app.tokio_runtime.block_on(async {
        tokio::time::timeout(std::time::Duration::from_secs(5), migration_task).await
    });

    app.tokio_runtime.block_on(app.services.close());

    Ok(())
}

fn collect_all_files(
    playlist_data: &PlaylistData,
    config: &Config,
    library_path: &CanonicalPath,
) -> Vec<CanonicalPath> {
    let mut files: HashSet<CanonicalPath> = HashSet::new();

    for item_path in &playlist_data.playlist {
        if let Some(canonical) = item_path.as_file() {
            if config.is_video_or_audio(canonical.as_path()) {
                files.insert(canonical.clone());
            }
        }
    }

    for item_path in playlist_data.files.keys() {
        if let Some(canonical) = item_path.as_file() {
            if config.is_video_or_audio(canonical.as_path()) {
                files.insert(canonical.clone());
            }
        }
    }

    if let Ok(read_dir) = std::fs::read_dir(library_path.as_path()) {
        for entry in read_dir.flatten() {
            let path = entry.path();
            if path.is_file() && config.is_video_or_audio(&path) {
                if let Ok(canonical) = CanonicalPath::from_path(&path) {
                    files.insert(canonical);
                }
            }
        }
    }

    files.into_iter().collect()
}

fn build_services(
    socket: &Path,
    media_backend: Arc<dyn MediaQuery>,
    core: Services,
) -> Services {
    use crate::{
        feat::launcher::{FileLauncherService, XdgLauncher},
        feat::mpv::{MpvClientService, MpvLauncherService, RealMpvLauncher},
    };

    let mpv_backend: Arc<dyn crate::feat::mpv::MpvClient> = Arc::new(MpvIpc::new(socket));

    Services {
        mpv: MpvClientService::new(mpv_backend),
        media: MediaQueryService::new(media_backend),
        storage: core.storage,
        mpv_launcher: MpvLauncherService::new(Arc::new(RealMpvLauncher)),
        file_launcher: FileLauncherService::new(Arc::new(XdgLauncher::new())),
        db: core.db,
        editor: core.editor,
        path_resolver: core.path_resolver,
        sources: core.sources,
        fuzzy_search: core.fuzzy_search,
        rt: core.rt,
    }
}

fn run_app(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    app: &mut App,
) -> Result<(), Report<RunError>> {
    loop {
        if app.tui_state.needs_clear {
            terminal.clear().change_context(RunError)?;
            app.tui_state.needs_clear = false;
        }

        let keymap = app.runtime.keymap.clone();
        terminal
            .draw(|f| tui::render(f, &app.tui_state, &keymap, &app.services))
            .change_context(RunError)?;

        if event::poll(std::time::Duration::from_millis(100)).change_context(RunError)? {
            let event = event::read().change_context(RunError)?;
            app.handle_event(event);
        }

        if let Some(action) = app.fork.take_action() {
            process_fork(terminal, app, action)?;
        }

        if app.should_quit {
            return Ok(());
        }
    }
}

fn add_note_for_path(app: &App, path: &Path) -> Result<(), String> {
    let canonical = CanonicalPath::from_path(path).map_err(|e| format!("Failed to canonicalize path: {e:?}"))?;
    let result = app
        .services
        .rt
        .block_on(crate::command::execute(
            &app.services,
            Command::NotesAdd {
                paths: vec![canonical],
            },
        ))
        .map_err(|e| format!("Add note failed: {e:?}"))?;
    match result {
        CommandResult::NotesAdded { .. } => Ok(()),
        _ => Err("Unexpected result type".to_string()),
    }
}

fn run_fuzzy_notes(app: &App) -> Result<usize, String> {
    let result = app
        .services
        .rt
        .block_on(crate::command::execute(
            &app.services,
            Command::NotesFuzzy {
                create_symlinks: true,
            },
        ))
        .map_err(|e| format!("Fuzzy notes failed: {e:?}"))?;
    match result {
        CommandResult::NotesFuzzy {
            symlinks_created, ..
        } => Ok(symlinks_created),
        _ => Err("Unexpected result type".to_string()),
    }
}

fn edit_sources_for_path(app: &App, path: &Path) -> Result<(), String> {
    let canonical = CanonicalPath::from_path(path).map_err(|e| format!("Failed to canonicalize path: {e:?}"))?;
    let result = app
        .services
        .rt
        .block_on(crate::command::execute(
            &app.services,
            Command::SourcesEdit {
                path: canonical,
            },
        ))
        .map_err(|e| format!("Edit sources failed: {e:?}"))?;
    match result {
        CommandResult::SourcesEdited { .. } => Ok(()),
        _ => Err("Unexpected result type".to_string()),
    }
}

fn run_generate_notes(app: &App, format: &str) -> Result<(), String> {
    let library_path = app.runtime.library_path.clone();
    let result = app
        .services
        .rt
        .block_on(crate::command::execute(
            &app.services,
            Command::GenerateNotes {
                format: format.to_owned(),
                working_directory: library_path,
            },
        ))
        .map_err(|e| format!("Generation failed: {e:?}"))?;

    let CommandResult::NotesGenerated { output } = result else {
        return Err("Unexpected result type".to_string());
    };

    let mut clipboard = arboard::Clipboard::new().map_err(|e| format!("Clipboard error: {e}"))?;
    clipboard
        .set_text(&output)
        .map_err(|e| format!("Clipboard error: {e}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::ItemPath;
    use marked_path::CanonicalPath;
    use std::path::PathBuf;

    #[test]
    fn requires_suspend_returns_true_for_add_note() {
        // Given an AddNote action.
        let action = ForkAction::AddNote {
            path: ItemPath::File(CanonicalPath::new(PathBuf::from("/test"))),
        };

        // When checking if suspend is required.
        // Then it returns true.
        assert!(requires_suspend(&action));
    }

    #[test]
    fn requires_suspend_returns_true_for_fuzzy_notes() {
        // Given a FuzzyNotes action.
        let action = ForkAction::FuzzyNotes;

        // When checking if suspend is required.
        // Then it returns true.
        assert!(requires_suspend(&action));
    }

    #[test]
    fn requires_suspend_returns_true_for_edit_sources() {
        // Given an EditSources action.
        let action = ForkAction::EditSources {
            path: ItemPath::File(CanonicalPath::new(PathBuf::from("/test"))),
        };

        // When checking if suspend is required.
        // Then it returns true.
        assert!(requires_suspend(&action));
    }

    #[test]
    fn requires_suspend_returns_false_for_generate_notes() {
        // Given a GenerateNotes action.
        let action = ForkAction::GenerateNotes {
            format: "markdown".to_string(),
        };

        // When checking if suspend is required.
        // Then it returns false (clipboard-only, no terminal suspend).
        assert!(!requires_suspend(&action));
    }
}
