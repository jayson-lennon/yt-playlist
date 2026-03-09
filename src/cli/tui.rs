use std::{
    collections::HashSet,
    fmt::Write,
    io::Write as IoWrite,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::Arc,
};

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use error_stack::{Report, ResultExt};
use ratatui::{backend::CrosstermBackend, Terminal};

use crate::{
    app::App,
    config::{load, Config},
    feat::media_query::{CachedMediaBackend, FfprobeBackend, MediaQuery, MediaQueryBackend},
    feat::mpv::MpvipcBackend,
    feat::{sources::SourceDb, ExternalEditor, NoteDb, PathResolver},
    playlist::{PlaylistData, PlaylistStorage, PlaylistStorageBackend, TomlBackend},
    services::Services,
    ui,
};

use super::{utils::create_symlink_with_suffix, RunError};

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
    playlist: PathBuf,
    socket: PathBuf,
    db_path: &Path,
    library_path: PathBuf,
) -> Result<(), Report<RunError>> {
    let config = load().change_context(RunError)?;

    let storage_backend: Arc<dyn PlaylistStorageBackend> =
        Arc::new(TomlBackend::new(playlist.clone()));
    let playlist_storage = PlaylistStorage::new(storage_backend.clone());

    let playlist_data = playlist_storage.load().change_context(RunError)?;
    let all_files = collect_all_files(&playlist_data, &config, &library_path);
    let ffprobe_backend: Arc<dyn MediaQueryBackend> = Arc::new(FfprobeBackend);

    let result =
        crate::feat::media_duration_analysis::analyze_files(&all_files, playlist_data.files, ffprobe_backend.as_ref()).change_context(RunError)?;

    let durations: std::collections::HashMap<PathBuf, std::time::Duration> = result
        .files
        .iter()
        .filter_map(|(k, v)| v.duration.map(|d| (k.clone(), d)))
        .collect();

    let media_backend: Arc<dyn MediaQueryBackend> =
        Arc::new(CachedMediaBackend::new(durations, ffprobe_backend));

    let core_services = tokio::runtime::Runtime::new()
        .change_context(RunError)?
        .block_on(Services::new(&db_path.to_string_lossy()))
        .change_context(RunError)?;

    let services = build_services(&playlist, &socket, media_backend, core_services);

    enable_raw_mode().change_context(RunError)?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture).change_context(RunError)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).change_context(RunError)?;

    let mut app = App::new(services, config, socket.to_string_lossy().into_owned(), library_path);
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

    Ok(())
}

fn collect_all_files(playlist_data: &PlaylistData, config: &Config, library_path: &Path) -> Vec<PathBuf> {
    let mut files: HashSet<PathBuf> = HashSet::new();

    for path in &playlist_data.playlist {
        if config.is_video_or_audio(path) {
            if let Ok(canonical) = path.canonicalize() {
                files.insert(canonical);
            } else {
                files.insert(path.clone());
            }
        }
    }

    for path in playlist_data.files.keys() {
        if config.is_video_or_audio(path) {
            if let Ok(canonical) = path.canonicalize() {
                files.insert(canonical);
            } else {
                files.insert(path.clone());
            }
        }
    }

    if let Ok(read_dir) = std::fs::read_dir(library_path) {
        for entry in read_dir.flatten() {
            let path = entry.path();
            if path.is_file() && config.is_video_or_audio(&path) {
                if let Ok(canonical) = path.canonicalize() {
                    files.insert(canonical);
                } else {
                    files.insert(path);
                }
            }
        }
    }

    files.into_iter().collect()
}

fn build_services(
    playlist: &Path,
    socket: &Path,
    media_backend: Arc<dyn MediaQueryBackend>,
    core: Services,
) -> Services {
    use crate::{
        feat::launcher::{FileLauncher, LauncherService},
        feat::mpv::{MpvClient, MpvLauncherService, RealMpvLauncher},
    };

    let mpv_backend: Arc<dyn crate::feat::mpv::MpvBackend> = Arc::new(MpvipcBackend::new(socket));
    let storage_backend: Arc<dyn PlaylistStorageBackend> =
        Arc::new(TomlBackend::new(playlist.to_path_buf()));

    Services {
        mpv: MpvClient::new(mpv_backend),
        media: MediaQuery::new(media_backend),
        storage: PlaylistStorage::new(storage_backend),
        mpv_launcher: MpvLauncherService::new(Arc::new(RealMpvLauncher)),
        file_launcher: LauncherService::new(Arc::new(FileLauncher::new())),
        db: core.db,
        editor: core.editor,
        path_resolver: core.path_resolver,
        sources: core.sources,
    }
}

#[allow(clippy::too_many_lines)]
fn run_app(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    app: &mut App,
) -> Result<(), Report<RunError>> {
    loop {
        if app.tui_state.needs_clear {
            terminal.clear().change_context(RunError)?;
            app.tui_state.needs_clear = false;
        }
        let keymap = app.keymap.clone();
        terminal.draw(|f| ui::render(f, &app.tui_state, &keymap)).change_context(RunError)?;

        if event::poll(std::time::Duration::from_millis(100)).change_context(RunError)? {
            let event = event::read().change_context(RunError)?;
            app.handle_event(event);
        }

        if let Some(path) = app.pending_notes_path.take() {
            disable_raw_mode().change_context(RunError)?;
            execute!(
                terminal.backend_mut(),
                LeaveAlternateScreen,
                DisableMouseCapture
            )
            .change_context(RunError)?;
            terminal.show_cursor().change_context(RunError)?;

            let result = add_note_for_path(app, &path);

            enable_raw_mode().change_context(RunError)?;
            execute!(
                terminal.backend_mut(),
                EnterAlternateScreen,
                EnableMouseCapture
            )
            .change_context(RunError)?;
            terminal.hide_cursor().change_context(RunError)?;
            terminal.clear().change_context(RunError)?;
            let keymap = app.keymap.clone();
            terminal.draw(|f| ui::render(f, &app.tui_state, &keymap)).change_context(RunError)?;

            match result {
                Ok(()) => {
                    app.tui_state.status_message = Some(format!("Note added: {}", path.display()));
                }
                Err(e) => {
                    app.tui_state.status_message = Some(format!("Failed to add note: {e}"));
                }
            }
        }

        if app.pending_fuzzy_notes {
            app.pending_fuzzy_notes = false;
            disable_raw_mode().change_context(RunError)?;
            execute!(
                terminal.backend_mut(),
                LeaveAlternateScreen,
                DisableMouseCapture
            )
            .change_context(RunError)?;
            terminal.show_cursor().change_context(RunError)?;

            let result = run_fuzzy_notes(app);

            enable_raw_mode().change_context(RunError)?;
            execute!(
                terminal.backend_mut(),
                EnterAlternateScreen,
                EnableMouseCapture
            )
            .change_context(RunError)?;
            terminal.hide_cursor().change_context(RunError)?;
            terminal.clear().change_context(RunError)?;
            let keymap = app.keymap.clone();
            terminal.draw(|f| ui::render(f, &app.tui_state, &keymap)).change_context(RunError)?;

            match result {
                Ok(count) => {
                    app.tui_state.status_message = Some(format!("Created {count} symlink(s)"));
                }
                Err(e) => {
                    app.tui_state.status_message = Some(format!("Fuzzy search failed: {e}"));
                }
            }
        }

        if let Some(path) = app.pending_sources_path.take() {
            disable_raw_mode().change_context(RunError)?;
            execute!(
                terminal.backend_mut(),
                LeaveAlternateScreen,
                DisableMouseCapture
            )
            .change_context(RunError)?;
            terminal.show_cursor().change_context(RunError)?;

            let result = edit_sources_for_path(app, &path);

            enable_raw_mode().change_context(RunError)?;
            execute!(
                terminal.backend_mut(),
                EnterAlternateScreen,
                EnableMouseCapture
            )
            .change_context(RunError)?;
            terminal.hide_cursor().change_context(RunError)?;
            terminal.clear().change_context(RunError)?;
            let keymap = app.keymap.clone();
            terminal.draw(|f| ui::render(f, &app.tui_state, &keymap)).change_context(RunError)?;

            match result {
                Ok(()) => {
                    app.tui_state.status_message = Some(format!("Updated sources: {}", path.display()));
                }
                Err(e) => {
                    app.tui_state.status_message = Some(format!("Failed to edit sources: {e}"));
                }
            }
        }

        if let Some(format) = app.pending_generate_notes.take() {
            let result = run_generate_notes(app, &format);

            match result {
                Ok(()) => {
                    app.tui_state.status_message =
                        Some(format!("Show notes ({format}) copied to clipboard"));
                }
                Err(e) => {
                    app.tui_state.status_message = Some(format!("Failed to generate notes: {e}"));
                }
            }
        }

        if app.should_quit {
            return Ok(());
        }
    }
}

fn add_note_for_path(app: &App, path: &Path) -> Result<(), String> {
    let services = &app.services;

    let rt = tokio::runtime::Runtime::new().map_err(|e| e.to_string())?;
    rt.block_on(async {
        let resolved = services
            .path_resolver
            .resolve(path)
            .await
            .map_err(|e| format!("Path resolution failed: {e:?}"))?;

        let path_str = resolved.to_string_lossy();
        let file_path_id = services
            .db
            .get_or_create_file_path(&path_str)
            .await
            .map_err(|e| format!("Database error: {e:?}"))?;

        let existing_note = services
            .db
            .get_note(file_path_id)
            .await
            .map_err(|e| format!("Database error: {e:?}"))?;

        let initial_content = existing_note.unwrap_or_default();
        if let Some(new_content) = services
            .editor
            .open(&initial_content)
            .await
            .map_err(|e| format!("Editor error: {e:?}"))?
        {
            services
                .db
                .upsert_note(file_path_id, &new_content)
                .await
                .map_err(|e| format!("Database error: {e:?}"))?;
        }

        Ok(())
    })
}

fn run_fuzzy_notes(app: &App) -> Result<usize, String> {
    let services = &app.services;

    let rt = tokio::runtime::Runtime::new().map_err(|e| e.to_string())?;
    rt.block_on(async {
        let all_notes = services
            .db
            .get_all_notes_with_paths()
            .await
            .map_err(|e| format!("Database error: {e:?}"))?;

        if all_notes.is_empty() {
            return Ok(0);
        }

        let input: String = all_notes
            .iter()
            .fold(String::new(), |mut output, (path, content)| {
                let cleaned: String = content
                    .lines()
                    .filter(|line| !line.trim().is_empty())
                    .collect::<Vec<_>>()
                    .join(". ");
                let _ = writeln!(output, "{path}\t{cleaned}");
                output
            });

        let mut child = Command::new("sk")
            .args([
                "-m",
                "--delimiter=\\t",
                "--with-nth=2..",
                "--color=marker:51,hl+:201,hl:219",
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .map_err(|e| format!("Failed to spawn skim: {e}"))?;

        if let Some(mut stdin) = child.stdin.take() {
            stdin
                .write_all(input.as_bytes())
                .map_err(|e| format!("Failed to write to skim: {e}"))?;
        }

        let output = child
            .wait_with_output()
            .map_err(|e| format!("Failed to read skim output: {e}"))?;

        let selected = String::from_utf8_lossy(&output.stdout);
        let selected_paths: Vec<&str> = selected
            .lines()
            .filter_map(|line| line.split('\t').next())
            .collect();

        let cwd = std::env::current_dir().map_err(|e| format!("Failed to get cwd: {e}"))?;
        let mut count = 0;
        for path in &selected_paths {
            let src = PathBuf::from(path);
            match create_symlink_with_suffix(&src, &cwd) {
                Ok(_) => count += 1,
                Err(e) => eprintln!("Failed to create symlink for {path}: {e:?}"),
            }
        }

        Ok(count)
    })
}

fn edit_sources_for_path(app: &App, path: &Path) -> Result<(), String> {
    let services = &app.services;

    let rt = tokio::runtime::Runtime::new().map_err(|e| e.to_string())?;
    rt.block_on(async {
        let resolved = services
            .path_resolver
            .resolve(path)
            .await
            .map_err(|e| format!("Path resolution failed: {e:?}"))?;

        let path_str = resolved.to_string_lossy();
        let file_path_id = services
            .db
            .get_or_create_file_path(&path_str)
            .await
            .map_err(|e| format!("Database error: {e:?}"))?;

        let existing = services
            .sources
            .get_sources(file_path_id)
            .await
            .map_err(|e| format!("Database error: {e:?}"))?;
        let initial_content = existing
            .iter()
            .map(|s| s.source_url.as_str())
            .collect::<Vec<_>>()
            .join("\n");

        if let Some(new_content) = services
            .editor
            .open(&initial_content)
            .await
            .map_err(|e| format!("Editor error: {e:?}"))?
        {
            let urls: Vec<String> = new_content.lines().map(ToString::to_string).collect();
            services
                .sources
                .set_sources(file_path_id, &urls)
                .await
                .map_err(|e| format!("Database error: {e:?}"))?;
        }

        Ok(())
    })
}

fn run_generate_notes(app: &App, format: &str) -> Result<(), String> {
    let playlist_data = app
        .services
        .storage
        .load()
        .map_err(|e| format!("Failed to load playlist: {e:?}"))?;

    let rt = tokio::runtime::Runtime::new().map_err(|e| e.to_string())?;
    let output = rt
        .block_on(crate::feat::generate_show_notes(
            &playlist_data,
            &app.services.sources,
            format,
        ))
        .map_err(|e| format!("Generation failed: {e:?}"))?;

    let mut clipboard = arboard::Clipboard::new().map_err(|e| format!("Clipboard error: {e}"))?;
    clipboard
        .set_text(&output)
        .map_err(|e| format!("Clipboard error: {e}"))?;
    Ok(())
}
