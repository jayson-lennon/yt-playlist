use super::TuiActionCtx;
use super::TuiActionError;
use crate::command::{Command, CommandResult};
use crate::tui::TuiActionResponse;
use error_stack::Report;

pub fn handle_launch_file(
    ctx: &mut TuiActionCtx<'_>,
) -> Result<TuiActionResponse, Report<TuiActionError>> {
    if let Some(item) = ctx.tui_state.get_selected_item().cloned() {
        if let Some(file_path) = item.path.as_file() {
            let cmd = ctx.ctx.config.get_cmd(file_path.as_path());
            let command = Command::LaunchFile {
                path: file_path.clone(),
                command: cmd.map(str::to_string),
                socket_path: ctx.ctx.socket_path.clone(),
            };
            match ctx.execute(command) {
                Ok(CommandResult::FileLaunched {
                    used_default_opener,
                    ..
                }) => {
                    if used_default_opener {
                        ctx.tui_state.status_bar.set(format!(
                            "Opening with default opener: {}",
                            item.path.display()
                        ));
                    } else {
                        ctx.tui_state
                            .status_bar
                            .set(format!("Opening: {}", item.path.display()));
                    }
                }
                Err(e) => {
                    ctx.tui_state
                        .show_error(format!("Failed to open file: {e:?}"));
                }
                _ => unreachable!(),
            }
        }
    }
    Ok(TuiActionResponse::Continue)
}

pub fn handle_load_playlist(
    ctx: &mut TuiActionCtx<'_>,
) -> Result<TuiActionResponse, Report<TuiActionError>> {
    let paths: Vec<marked_path::CanonicalPath> = ctx
        .tui_state
        .playlist_pane
        .items
        .iter()
        .filter(|item| {
            item.path
                .as_file()
                .is_some_and(|p| ctx.ctx.config.is_video_or_audio(p.as_path()))
        })
        .filter_map(|item| item.path.as_file().cloned())
        .collect();

    if paths.is_empty() {
        ctx.tui_state
            .show_error("No video or audio files in playlist".to_string());
        return Ok(TuiActionResponse::Continue);
    }

    let command = Command::MpvLoadPlaylist { paths };
    match ctx.execute(command) {
        Ok(CommandResult::MpvPlaylistLoaded { count }) => {
            ctx.tui_state
                .status_bar
                .set(format!("Loaded {count} items into mpv"));
        }
        Err(e) => {
            ctx.tui_state
                .show_error(format!("Failed to load playlist in mpv: {e:?}"));
        }
        _ => unreachable!(),
    }
    Ok(TuiActionResponse::Continue)
}

pub fn handle_launch_mpv(
    ctx: &mut TuiActionCtx<'_>,
) -> Result<TuiActionResponse, Report<TuiActionError>> {
    let command = Command::MpvSpawn {
        socket_path: ctx.ctx.socket_path.clone(),
    };
    match ctx.execute(command) {
        Ok(CommandResult::MpvSpawned {
            was_already_running: true,
        }) => {
            ctx.tui_state.status_bar.set("MPV already running");
        }
        Ok(CommandResult::MpvSpawned {
            was_already_running: false,
        }) => {
            ctx.tui_state.status_bar.set("MPV launched");
        }
        Err(e) => {
            ctx.tui_state
                .show_error(format!("Failed to launch mpv: {e:?}"));
        }
        _ => unreachable!(),
    }
    Ok(TuiActionResponse::Continue)
}

pub fn handle_toggle_play(
    ctx: &mut TuiActionCtx<'_>,
) -> Result<TuiActionResponse, Report<TuiActionError>> {
    let command = Command::MpvTogglePlay;
    match ctx.execute(command) {
        Ok(CommandResult::MpvToggledPlay) => {
            ctx.tui_state.status_bar.set("Toggled playback");
        }
        Err(e) => {
            ctx.tui_state
                .show_error(format!("Failed to toggle playback: {e:?}"));
        }
        _ => unreachable!(),
    }
    Ok(TuiActionResponse::Continue)
}
