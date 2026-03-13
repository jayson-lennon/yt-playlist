use crate::app::App;
use crate::command::{Command, CommandResult};

pub fn handle_launch_file(app: &mut App) {
    if let Some(item) = app.tui_state.get_selected_item().cloned() {
        if let Some(file_path) = item.path.as_file() {
            let cmd = app.ctx.config.get_cmd(file_path.as_path());
            let command = Command::LaunchFile {
                path: file_path.clone(),
                command: cmd.map(str::to_string),
                socket_path: app.ctx.socket_path.clone(),
            };
            match app.execute(command) {
                Ok(CommandResult::FileLaunched {
                    used_default_opener,
                    ..
                }) => {
                    if used_default_opener {
                        app.tui_state.status_bar.set(format!(
                            "Opening with default opener: {}",
                            item.path.display()
                        ));
                    } else {
                        app.tui_state
                            .status_bar
                            .set(format!("Opening: {}", item.path.display()));
                    }
                }
                Err(e) => {
                    app.tui_state
                        .show_error(format!("Failed to open file: {e:?}"));
                }
                _ => unreachable!(),
            }
        }
    }
}

pub fn handle_load_playlist(app: &mut App) {
    let paths: Vec<marked_path::CanonicalPath> = app
        .tui_state
        .playlist_pane
        .items
        .iter()
        .filter(|item| {
            item.path
                .as_file()
                .is_some_and(|p| app.ctx.config.is_video_or_audio(p.as_path()))
        })
        .filter_map(|item| item.path.as_file().cloned())
        .collect();

    if paths.is_empty() {
        app.tui_state
            .show_error("No video or audio files in playlist".to_string());
        return;
    }

    let command = Command::MpvLoadPlaylist { paths };
    match app.execute(command) {
        Ok(CommandResult::MpvPlaylistLoaded { count }) => {
            app.tui_state
                .status_bar
                .set(format!("Loaded {count} items into mpv"));
        }
        Err(e) => {
            app.tui_state
                .show_error(format!("Failed to load playlist in mpv: {e:?}"));
        }
        _ => unreachable!(),
    }
}

pub fn handle_launch_mpv(app: &mut App) {
    let command = Command::MpvSpawn {
        socket_path: app.ctx.socket_path.clone(),
    };
    match app.execute(command) {
        Ok(CommandResult::MpvSpawned {
            was_already_running: true,
        }) => {
            app.tui_state.status_bar.set("MPV already running");
        }
        Ok(CommandResult::MpvSpawned {
            was_already_running: false,
        }) => {
            app.tui_state.status_bar.set("MPV launched");
        }
        Err(e) => {
            app.tui_state
                .show_error(format!("Failed to launch mpv: {e:?}"));
        }
        _ => unreachable!(),
    }
}
