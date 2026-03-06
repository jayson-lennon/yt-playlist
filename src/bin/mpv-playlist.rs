use std::{path::PathBuf, sync::Arc};

use clap::Parser;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};

use mpv_playlist::{
    app::{App, DEFAULT_EXTENSIONS},
    media::{FfprobeBackend, MediaQuery, MediaQueryBackend},
    mpv::{MpvBackend, MpvClient, MpvipcBackend},
    playlist::{FileBackend, PlaylistStorage, PlaylistStorageBackend},
    services::Services,
    ui,
};

#[derive(Parser)]
#[command(name = "mpv-playlist")]
#[command(about = "TUI playlist manager for mpv")]
struct Args {
    /// Playlist file path
    #[arg(short, long, default_value = "playlist.txt")]
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
    let services = build_services(&args);

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

fn parse_extensions(args: &Args) -> Vec<String> {
    match &args.extensions {
        Some(exts) => exts
            .split(',')
            .map(|s| s.trim().to_lowercase())
            .filter(|s| !s.is_empty())
            .collect(),
        None => {
            let mut exts: Vec<String> = DEFAULT_EXTENSIONS.iter().map(std::string::ToString::to_string).collect();
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

fn build_services(args: &Args) -> Services {
    let mpv_backend: Arc<dyn MpvBackend> = Arc::new(MpvipcBackend::new(&args.socket));
    let media_backend: Arc<dyn MediaQueryBackend> = Arc::new(FfprobeBackend);
    let storage_backend: Arc<dyn PlaylistStorageBackend> =
        Arc::new(FileBackend::new(args.playlist.clone()));

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

        if app.should_quit {
            return Ok(());
        }
    }
}
