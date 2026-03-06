use std::{collections::HashSet, path::PathBuf, sync::Arc};

use clap::Parser;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};

use mpv_playlist::{
    analysis,
    app::{App, DEFAULT_EXTENSIONS},
    media::{CachedMediaBackend, FfprobeBackend, MediaQuery, MediaQueryBackend},
    mpv::{MpvBackend, MpvClient, MpvipcBackend},
    playlist::{PlaylistData, PlaylistStorage, PlaylistStorageBackend, TomlBackend},
    services::Services,
    ui,
};

#[derive(Parser)]
#[command(name = "mpv-playlist")]
#[command(about = "TUI playlist manager for mpv")]
struct Args {
    /// Playlist file path
    #[arg(short, long, default_value = "playlist.toml")]
    playlist: PathBuf,

    /// mpv socket path
    #[arg(long, default_value = "/tmp/mpvsocket")]
    socket: PathBuf,

    /// Append extensions to common set (comma-separated)
    #[arg(short = 'e', long)]
    extend: Option<String>,

    /// Replace extensions entirely (comma-separated)
    #[arg(short = 'x', long)]
    extensions: Option<String>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let extensions = parse_extensions(&args);

    let storage_backend: Arc<dyn PlaylistStorageBackend> =
        Arc::new(TomlBackend::new(args.playlist.clone()));
    let playlist_storage = PlaylistStorage::new(storage_backend.clone());

    let playlist_data = playlist_storage.load()?;
    let all_files = collect_all_files(&playlist_data, &extensions);
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

    let services = build_services(&args, media_backend);

    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(services, extensions);
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

fn collect_all_files(playlist_data: &PlaylistData, extensions: &[String]) -> Vec<PathBuf> {
    let mut files: HashSet<PathBuf> = HashSet::new();

    for path in &playlist_data.playlist {
        if let Ok(canonical) = path.canonicalize() {
            files.insert(canonical);
        } else {
            files.insert(path.clone());
        }
    }

    for path in playlist_data.files.keys() {
        if let Ok(canonical) = path.canonicalize() {
            files.insert(canonical);
        } else {
            files.insert(path.clone());
        }
    }

    if let Ok(read_dir) = std::fs::read_dir(".") {
        for entry in read_dir.flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                    if extensions.contains(&ext.to_lowercase()) {
                        if let Ok(canonical) = path.canonicalize() {
                            files.insert(canonical);
                        } else {
                            files.insert(path);
                        }
                    }
                }
            }
        }
    }

    files.into_iter().collect()
}

fn parse_extensions(args: &Args) -> Vec<String> {
    match &args.extensions {
        Some(exts) => exts
            .split(',')
            .map(|s| s.trim().to_lowercase())
            .filter(|s| !s.is_empty())
            .collect(),
        None => {
            let mut exts: Vec<String> = DEFAULT_EXTENSIONS
                .iter()
                .map(std::string::ToString::to_string)
                .collect();
            if let Some(extra) = &args.extend {
                for e in extra.split(',') {
                    let e = e.trim().to_lowercase();
                    if !e.is_empty() && !exts.contains(&e) {
                        exts.push(e);
                    }
                }
            }
            exts
        }
    }
}

fn build_services(args: &Args, media_backend: Arc<dyn MediaQueryBackend>) -> Services {
    let mpv_backend: Arc<dyn MpvBackend> = Arc::new(MpvipcBackend::new(&args.socket));
    let storage_backend: Arc<dyn PlaylistStorageBackend> =
        Arc::new(TomlBackend::new(args.playlist.clone()));

    Services {
        mpv: MpvClient::new(mpv_backend),
        media: MediaQuery::new(media_backend),
        storage: PlaylistStorage::new(storage_backend),
    }
}

fn run_app(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    app: &mut App,
) -> Result<(), Box<dyn std::error::Error>> {
    loop {
        terminal.draw(|f| ui::render(f, &app.tui_state))?;

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
            terminal.draw(|f| ui::render(f, &app.tui_state))?;

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
