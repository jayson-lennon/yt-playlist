use std::{
    collections::HashSet,
    fmt::Write,
    io::Write as IoWrite,
    os::unix::fs as unix_fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::Arc,
};

use clap::{Parser, Subcommand};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use error_stack::{fmt::ColorMode, Report, ResultExt};
use ratatui::{backend::CrosstermBackend, Terminal};

use shownotes::{
    analysis,
    app::App,
    config::{load, Config},
    launcher::{FileLauncher, LauncherService},
    media::{CachedMediaBackend, FfprobeBackend, MediaQuery, MediaQueryBackend},
    mpv::{MpvBackend, MpvClient, MpvLauncherService, MpvipcBackend, RealMpvLauncher},
    notes::{Editor, NoteDb, PathResolver, SystemServicesHandle},
    playlist::{PlaylistData, PlaylistStorage, PlaylistStorageBackend, TomlBackend},
    services::Services,
    ui,
};

#[derive(Parser)]
#[command(name = "shownotes")]
#[command(about = "TUI playlist manager for mpv with notes support")]
struct Args {
    #[arg(long, env = "SHOWNOTES_DB_PATH", default_value = "/mnt/zed/work/youtube/notes.db")]
    db_path: PathBuf,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Run the TUI (default when no command specified)
    Tui {
        /// Playlist file path
        #[arg(short, long, default_value = "shownotes.toml")]
        playlist: PathBuf,

        /// mpv socket path
        #[arg(long, default_value = "/tmp/mpvsocket")]
        socket: PathBuf,

        /// Directory path for library scanning
        #[arg(long, default_value = ".")]
        path: PathBuf,
    },

    /// Perform an action on a file
    Action {
        #[command(subcommand)]
        action: ActionCommands,
    },

    /// Notes commands for managing file notes
    Notes {
        #[command(subcommand)]
        notes_cmd: NotesCommands,
    },
}

#[derive(Subcommand)]
enum ActionCommands {
    /// Load a file in mpv via IPC
    Mpv {
        /// File path to open
        path: PathBuf,

        /// mpv socket path
        #[arg(long, default_value = "/tmp/mpvsocket")]
        socket: PathBuf,
    },
}

#[derive(Subcommand)]
enum NotesCommands {
    /// Add notes to files
    Add { paths: Vec<PathBuf> },

    /// Search for words in notes
    Search {
        /// Search query. Uses AND matching.
        query: String,
        /// Create symlinks to located results in current directory.
        #[arg(long)]
        symlink: bool,
    },

    /// Fuzzy search through all notes
    Fuzzy {
        /// Create symlinks to located results in current directory.
        #[arg(long)]
        symlink: bool,
    },
}

#[derive(Debug, wherror::Error)]
pub enum AppError {
    #[error("failed to initialize database")]
    DbInit,
    #[error("no file paths provided")]
    NoPaths,
    #[error("failed to resolve path")]
    PathResolution,
    #[error("database operation failed")]
    Database,
    #[error("editor operation failed")]
    Editor,
    #[error("fuzzy search failed")]
    FuzzySearch,
    #[error("symlink creation failed")]
    Symlink,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    Report::set_color_mode(ColorMode::None);

    let args = Args::parse();

    match args.command.unwrap_or(Commands::Tui {
        playlist: PathBuf::from("shownotes.toml"),
        socket: PathBuf::from("/tmp/mpvsocket"),
        path: PathBuf::from("."),
    }) {
        Commands::Tui { playlist, socket, path } => run_tui(playlist, socket, &args.db_path, path),
        Commands::Action { action } => match action {
            ActionCommands::Mpv { path, socket } => run_action_mpv(&path, &socket),
        },
        Commands::Notes { notes_cmd } => run_notes_command(notes_cmd, &args.db_path),
    }
}

fn run_action_mpv(path: &Path, socket: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let backend = MpvipcBackend::new(socket);
    backend.load_file(path)?;
    println!("Loaded: {}", path.display());
    Ok(())
}

fn run_notes_command(cmd: NotesCommands, db_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async { run_notes_command_async(cmd, db_path).await })
}

#[allow(clippy::too_many_lines)]
async fn run_notes_command_async(
    cmd: NotesCommands,
    db_path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let services = SystemServicesHandle::new(&db_path.to_string_lossy())
        .await
        .change_context(AppError::DbInit)?;

    match cmd {
        NotesCommands::Add { paths } => {
            if paths.is_empty() {
                return Err(Box::new(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "no file paths provided",
                )));
            }

            let mut resolved_paths = Vec::with_capacity(paths.len());
            for path in paths {
                let resolved = services
                    .path_resolver
                    .resolve(&path)
                    .await
                    .change_context(AppError::PathResolution)?;
                resolved_paths.push(resolved);
            }

            if resolved_paths.len() == 1 {
                let resolved_path = &resolved_paths[0];
                let path_str = resolved_path.to_string_lossy();
                let file_path_id = services
                    .db
                    .get_or_create_file_path(&path_str)
                    .await
                    .change_context(AppError::Database)?;

                let existing_note = services
                    .db
                    .get_note(file_path_id)
                    .await
                    .change_context(AppError::Database)?;

                let initial_content = existing_note.unwrap_or_default();
                if let Some(new_content) = services
                    .editor
                    .open(&initial_content)
                    .await
                    .change_context(AppError::Editor)?
                {
                    services
                        .db
                        .upsert_note(file_path_id, &new_content)
                        .await
                        .change_context(AppError::Database)?;
                }
            } else if let Some(new_content) = services
                .editor
                .open("")
                .await
                .change_context(AppError::Editor)?
            {
                for resolved_path in resolved_paths {
                    let path_str = resolved_path.to_string_lossy();
                    let file_path_id = services
                        .db
                        .get_or_create_file_path(&path_str)
                        .await
                        .change_context(AppError::Database)?;

                    let existing_note = services
                        .db
                        .get_note(file_path_id)
                        .await
                        .change_context(AppError::Database)?;

                    let final_content = match existing_note {
                        Some(existing) => format!("{existing}\n\n{new_content}"),
                        None => new_content.clone(),
                    };

                    services
                        .db
                        .upsert_note(file_path_id, &final_content)
                        .await
                        .change_context(AppError::Database)?;
                }
            }
        }
        NotesCommands::Search { query, symlink } => {
            let results = services
                .db
                .search_notes(&query)
                .await
                .change_context(AppError::Database)?;

            let cwd = std::env::current_dir().change_context(AppError::Symlink)?;

            for path in &results {
                println!("{path}");
            }

            if symlink {
                for path in &results {
                    let src = PathBuf::from(path);
                    match create_symlink_with_suffix(&src, &cwd) {
                        Ok(dest) => eprintln!("Created symlink: {}", dest.display()),
                        Err(e) => eprintln!("Failed to create symlink for {path}: {e:?}"),
                    }
                }
            }
        }
        NotesCommands::Fuzzy { symlink } => {
            let notes = services
                .db
                .get_all_notes_with_paths()
                .await
                .change_context(AppError::Database)?;

            if notes.is_empty() {
                return Ok(());
            }

            let input: String = notes.iter().fold(String::new(), |mut output, (path, content)| {
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
                .change_context(AppError::FuzzySearch)?;

            if let Some(mut stdin) = child.stdin.take() {
                stdin
                    .write_all(input.as_bytes())
                    .change_context(AppError::FuzzySearch)?;
            }

            let output = child
                .wait_with_output()
                .change_context(AppError::FuzzySearch)?;

            let selected = String::from_utf8_lossy(&output.stdout);
            let selected_paths: Vec<&str> = selected
                .lines()
                .filter_map(|line| line.split('\t').next())
                .collect();

            for path in &selected_paths {
                println!("{path}");
            }

            if symlink {
                let cwd = std::env::current_dir().change_context(AppError::Symlink)?;
                for path in &selected_paths {
                    let src = PathBuf::from(path);
                    match create_symlink_with_suffix(&src, &cwd) {
                        Ok(dest) => eprintln!("Created symlink: {}", dest.display()),
                        Err(e) => eprintln!("Failed to create symlink for {path}: {e:?}"),
                    }
                }
            }
        }
    }

    Ok(())
}

fn create_symlink_with_suffix(target: &Path, dest_dir: &Path) -> Result<PathBuf, Report<AppError>> {
    let basename = target
        .file_name()
        .ok_or_else(|| Report::new(AppError::Symlink))?;

    let mut dest_path = dest_dir.join(basename);
    let mut suffix = 0;

    while dest_path.exists() || dest_path.symlink_metadata().is_ok() {
        suffix += 1;
        let stem = target
            .file_stem()
            .ok_or_else(|| Report::new(AppError::Symlink))?;
        let new_name = if let Some(ext) = target.extension() {
            format!(
                "{}_{}.{}",
                stem.to_string_lossy(),
                suffix,
                ext.to_string_lossy()
            )
        } else {
            format!("{}_{}", stem.to_string_lossy(), suffix)
        };
        dest_path = dest_dir.join(new_name);
    }

    unix_fs::symlink(target, &dest_path).change_context(AppError::Symlink)?;
    Ok(dest_path)
}

fn run_tui(
    playlist: PathBuf,
    socket: PathBuf,
    db_path: &Path,
    library_path: PathBuf,
) -> Result<(), Box<dyn std::error::Error>> {
    let config = load()?;

    let storage_backend: Arc<dyn PlaylistStorageBackend> =
        Arc::new(TomlBackend::new(playlist.clone()));
    let playlist_storage = PlaylistStorage::new(storage_backend.clone());

    let playlist_data = playlist_storage.load()?;
    let all_files = collect_all_files(&playlist_data, &config, &library_path);
    let ffprobe_backend: Arc<dyn MediaQueryBackend> = Arc::new(FfprobeBackend);

    let result =
        analysis::analyze_files(&all_files, playlist_data.files, ffprobe_backend.as_ref())?;

    let durations: std::collections::HashMap<PathBuf, std::time::Duration> = result
        .files
        .iter()
        .filter_map(|(k, v)| v.duration.map(|d| (k.clone(), d)))
        .collect();

    let media_backend: Arc<dyn MediaQueryBackend> =
        Arc::new(CachedMediaBackend::new(durations, ffprobe_backend));

    let notes_handle = tokio::runtime::Runtime::new()?
        .block_on(SystemServicesHandle::new(&db_path.to_string_lossy()))
        .map_err(|e| format!("Failed to initialize notes database: {e:?}"))?;

    let services = build_services(&playlist, &socket, media_backend, Some(notes_handle));

    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(services, config, socket.to_string_lossy().into_owned(), library_path);
    let res = run_app(&mut terminal, &mut app);

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

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
    notes: Option<SystemServicesHandle>,
) -> Services {
    let mpv_backend: Arc<dyn MpvBackend> = Arc::new(MpvipcBackend::new(socket));
    let storage_backend: Arc<dyn PlaylistStorageBackend> =
        Arc::new(TomlBackend::new(playlist.to_path_buf()));

    Services {
        mpv: MpvClient::new(mpv_backend),
        media: MediaQuery::new(media_backend),
        storage: PlaylistStorage::new(storage_backend),
        mpv_launcher: MpvLauncherService::new(Arc::new(RealMpvLauncher)),
        file_launcher: LauncherService::new(Arc::new(FileLauncher::new())),
        notes,
    }
}

fn run_app(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    app: &mut App,
) -> Result<(), Box<dyn std::error::Error>> {
    loop {
        if app.tui_state.needs_clear {
            terminal.clear()?;
            app.tui_state.needs_clear = false;
        }
        let keymap = app.keymap.clone();
        terminal.draw(|f| ui::render(f, &app.tui_state, &keymap))?;

        if event::poll(std::time::Duration::from_millis(100))? {
            let event = event::read()?;
            app.handle_event(event);
        }

        if let Some(path) = app.pending_notes_path.take() {
            disable_raw_mode()?;
            execute!(
                terminal.backend_mut(),
                LeaveAlternateScreen,
                DisableMouseCapture
            )?;
            terminal.show_cursor()?;

            let result = add_note_for_path(app, &path);

            enable_raw_mode()?;
            execute!(
                terminal.backend_mut(),
                EnterAlternateScreen,
                EnableMouseCapture
            )?;
            terminal.hide_cursor()?;
            terminal.clear()?;
            let keymap = app.keymap.clone();
            terminal.draw(|f| ui::render(f, &app.tui_state, &keymap))?;

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
            disable_raw_mode()?;
            execute!(
                terminal.backend_mut(),
                LeaveAlternateScreen,
                DisableMouseCapture
            )?;
            terminal.show_cursor()?;

            let result = run_fuzzy_notes(app);

            enable_raw_mode()?;
            execute!(
                terminal.backend_mut(),
                EnterAlternateScreen,
                EnableMouseCapture
            )?;
            terminal.hide_cursor()?;
            terminal.clear()?;
            let keymap = app.keymap.clone();
            terminal.draw(|f| ui::render(f, &app.tui_state, &keymap))?;

            match result {
                Ok(count) => {
                    app.tui_state.status_message = Some(format!("Created {count} symlink(s)"));
                }
                Err(e) => {
                    app.tui_state.status_message = Some(format!("Fuzzy search failed: {e}"));
                }
            }
        }

        if app.should_quit {
            return Ok(());
        }
    }
}

fn add_note_for_path(app: &App, path: &Path) -> Result<(), String> {
    let notes = app
        .services
        .notes
        .as_ref()
        .ok_or("Notes service not initialized")?;

    let rt = tokio::runtime::Runtime::new().map_err(|e| e.to_string())?;
    rt.block_on(async {
        let resolved = notes
            .path_resolver
            .resolve(path)
            .await
            .map_err(|e| format!("Path resolution failed: {e:?}"))?;

        let path_str = resolved.to_string_lossy();
        let file_path_id = notes
            .db
            .get_or_create_file_path(&path_str)
            .await
            .map_err(|e| format!("Database error: {e:?}"))?;

        let existing_note = notes
            .db
            .get_note(file_path_id)
            .await
            .map_err(|e| format!("Database error: {e:?}"))?;

        let initial_content = existing_note.unwrap_or_default();
        if let Some(new_content) = notes
            .editor
            .open(&initial_content)
            .await
            .map_err(|e| format!("Editor error: {e:?}"))?
        {
            notes
                .db
                .upsert_note(file_path_id, &new_content)
                .await
                .map_err(|e| format!("Database error: {e:?}"))?;
        }

        Ok(())
    })
}

fn run_fuzzy_notes(app: &App) -> Result<usize, String> {
    let notes = app
        .services
        .notes
        .as_ref()
        .ok_or("Notes service not initialized")?;

    let rt = tokio::runtime::Runtime::new().map_err(|e| e.to_string())?;
    rt.block_on(async {
        let all_notes = notes
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
