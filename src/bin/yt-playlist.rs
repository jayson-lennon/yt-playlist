use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    sync::Arc,
};

use clap::{Parser, Subcommand};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use error_stack::{fmt::ColorMode, Report};
use ratatui::{backend::CrosstermBackend, Terminal};

use yt_playlist::{
    analysis,
    app::App,
    config::{load, Config},
    launcher::FileLauncher,
    media::{CachedMediaBackend, FfprobeBackend, MediaQuery, MediaQueryBackend},
    mpv::{MpvBackend, MpvClient, MpvipcBackend, RealMpvLauncher},
    playlist::{PlaylistData, PlaylistStorage, PlaylistStorageBackend, TomlBackend},
    services::Services,
    ui,
};

#[derive(Parser)]
#[command(name = "yt-playlist")]
#[command(about = "TUI playlist manager for mpv")]
struct Args {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Run the TUI (default when no command specified)
    Tui {
        /// Playlist file path
        #[arg(short, long, default_value = "playlist.toml")]
        playlist: PathBuf,

        /// mpv socket path
        #[arg(long, default_value = "/tmp/mpvsocket")]
        socket: PathBuf,
    },

    /// Perform an action on a file
    Action {
        #[command(subcommand)]
        action: ActionCommands,
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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    Report::set_color_mode(ColorMode::None);

    let args = Args::parse();

    match args.command.unwrap_or(Commands::Tui {
        playlist: PathBuf::from("playlist.toml"),
        socket: PathBuf::from("/tmp/mpvsocket"),
    }) {
        Commands::Tui { playlist, socket } => run_tui(playlist, socket),
        Commands::Action { action } => match action {
            ActionCommands::Mpv { path, socket } => run_action_mpv(&path, &socket),
        },
    }
}

fn run_action_mpv(path: &Path, socket: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let backend = MpvipcBackend::new(socket);
    backend.load_file(path)?;
    println!("Loaded: {}", path.display());
    Ok(())
}

fn run_tui(playlist: PathBuf, socket: PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let config = load()?;

    let storage_backend: Arc<dyn PlaylistStorageBackend> =
        Arc::new(TomlBackend::new(playlist.clone()));
    let playlist_storage = PlaylistStorage::new(storage_backend.clone());

    let playlist_data = playlist_storage.load()?;
    let all_files = collect_all_files(&playlist_data, &config);
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

    let services = build_services(&playlist, &socket, media_backend);

    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(services, config, socket.to_string_lossy().into_owned());
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

fn collect_all_files(playlist_data: &PlaylistData, config: &Config) -> Vec<PathBuf> {
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

    if let Ok(read_dir) = std::fs::read_dir(".") {
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
) -> Services {
    let mpv_backend: Arc<dyn MpvBackend> = Arc::new(MpvipcBackend::new(socket));
    let storage_backend: Arc<dyn PlaylistStorageBackend> =
        Arc::new(TomlBackend::new(playlist.to_path_buf()));

    Services {
        mpv: MpvClient::new(mpv_backend),
        media: MediaQuery::new(media_backend),
        storage: PlaylistStorage::new(storage_backend),
        mpv_launcher: Arc::new(RealMpvLauncher),
        file_launcher: Arc::new(FileLauncher::new()),
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

            let result = std::process::Command::new("notes")
                .args(["add", path.to_str().unwrap_or("")])
                .status();

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
                Ok(status) if status.success() => {
                    app.tui_state.status_message = Some(format!("Note added: {}", path.display()));
                }
                Ok(status) => {
                    app.tui_state.status_message =
                        Some(format!("Notes command failed with code: {status}"));
                }
                Err(e) => {
                    app.tui_state.status_message =
                        Some(format!("Failed to run notes command: {e}"));
                }
            }
        }

        if app.should_quit {
            return Ok(());
        }
    }
}
