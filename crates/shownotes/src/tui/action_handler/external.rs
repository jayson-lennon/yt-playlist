use super::TuiActionCtx;
use crate::command::{Command, CommandResult};
use crate::tui::TuiActionResponse;

pub fn handle_launch_file(ctx: &mut TuiActionCtx<'_>) -> TuiActionResponse {
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
        } else if let Some(url) = item.path.as_url() {
            let path = std::path::Path::new(url);
            match ctx
                .ctx
                .services
                .file_launcher
                .launch(path, None, &ctx.ctx.socket_path)
            {
                Ok(_) => {
                    ctx.tui_state.status_bar.set(format!("Opening URL: {url}"));
                }
                Err(e) => {
                    ctx.tui_state
                        .show_error(format!("Failed to open URL: {e:?}"));
                }
            }
        }
    }
    TuiActionResponse::Continue
}

pub fn handle_load_playlist(ctx: &mut TuiActionCtx<'_>) -> TuiActionResponse {
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
        return TuiActionResponse::Continue;
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
    TuiActionResponse::Continue
}

pub fn handle_launch_mpv(ctx: &mut TuiActionCtx<'_>) -> TuiActionResponse {
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
    TuiActionResponse::Continue
}

pub fn handle_toggle_play(ctx: &mut TuiActionCtx<'_>) -> TuiActionResponse {
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
    TuiActionResponse::Continue
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::Fork;
    use crate::common::domain::{ItemPath, PlaylistItem};
    use crate::feat::config::Config;
    use crate::feat::keymap::Keymap;
    use crate::system_ctx::SystemCtx;
    use crate::test_utils::fakes::FakeLauncher;
    use crate::test_utils::services::create_test_services_with_launcher;
    use crate::tui::{Pane, TuiState};
    use marked_path::CanonicalPath;
    use std::path::PathBuf;
    use std::sync::atomic::Ordering;
    use std::sync::Arc;
    use tempfile::NamedTempFile;

    fn url_item(url: &str) -> PlaylistItem {
        PlaylistItem {
            path: ItemPath::Url(url.to_string()),
            duration: None,
            alias: None,
            mime_type: None,
            is_virtual: true,
            playlist_count: 0,
            has_sources: true,
        }
    }

    fn file_item(path: &str) -> PlaylistItem {
        PlaylistItem {
            path: ItemPath::File(CanonicalPath::new(PathBuf::from(path))),
            duration: None,
            alias: None,
            mime_type: None,
            is_virtual: false,
            playlist_count: 0,
            has_sources: true,
        }
    }

    async fn create_test_ctx_with_launcher(launcher: Arc<FakeLauncher>) -> (SystemCtx, NamedTempFile) {
        let services = create_test_services_with_launcher(launcher).await;
        let temp_file = NamedTempFile::new().unwrap();
        let library_path =
            CanonicalPath::from_path(temp_file.path().parent().unwrap()).unwrap();
        let ctx = SystemCtx {
            services,
            config: Config::default(),
            library_path,
            socket_path: String::new(),
            keymap: Keymap::new(),
        };
        (ctx, temp_file)
    }

    #[tokio::test]
    async fn handle_launch_file_opens_url_with_launcher() {
        // Given a TUI state with a URL item selected.
        let launcher = Arc::new(FakeLauncher::new());
        let (ctx, _temp_file) = create_test_ctx_with_launcher(launcher.clone()).await;
        let mut tui_state = TuiState::new();
        tui_state.playlist_pane.items = vec![url_item("https://example.com/video.mp4")];
        tui_state.focused_pane = Pane::Playlist;
        let mut fork = Fork::default();
        let mut action_ctx = TuiActionCtx {
            tui_state: &mut tui_state,
            fork: &mut fork,
            ctx: &ctx,
        };

        // When handling launch file.
        let _response = handle_launch_file(&mut action_ctx);

        // Then the launcher was called with the URL path and no command.
        assert_eq!(launcher.launch_called.load(Ordering::SeqCst), 1);
        let last_path = launcher.last_path.lock().unwrap().clone();
        assert_eq!(last_path, Some(PathBuf::from("https://example.com/video.mp4")));
        let last_command = launcher.last_command.lock().unwrap().clone();
        assert_eq!(last_command, None);
    }

    #[tokio::test]
    async fn handle_launch_file_sets_status_bar_for_url() {
        // Given a TUI state with a URL item selected.
        let launcher = Arc::new(FakeLauncher::new());
        let (ctx, _temp_file) = create_test_ctx_with_launcher(launcher).await;
        let mut tui_state = TuiState::new();
        tui_state.playlist_pane.items = vec![url_item("https://example.com/video.mp4")];
        tui_state.focused_pane = Pane::Playlist;
        let mut fork = Fork::default();
        let mut action_ctx = TuiActionCtx {
            tui_state: &mut tui_state,
            fork: &mut fork,
            ctx: &ctx,
        };

        // When handling launch file.
        let _response = handle_launch_file(&mut action_ctx);

        // Then the status bar shows "Opening URL: {url}".
        assert_eq!(
            tui_state.status_bar.message(),
            Some("Opening URL: https://example.com/video.mp4")
        );
    }

    #[tokio::test]
    async fn handle_launch_file_does_nothing_when_no_item_selected() {
        // Given a TUI state with no items.
        let launcher = Arc::new(FakeLauncher::new());
        let (ctx, _temp_file) = create_test_ctx_with_launcher(launcher.clone()).await;
        let mut tui_state = TuiState::new();
        tui_state.playlist_pane.items = vec![];
        tui_state.focused_pane = Pane::Playlist;
        let mut fork = Fork::default();
        let mut action_ctx = TuiActionCtx {
            tui_state: &mut tui_state,
            fork: &mut fork,
            ctx: &ctx,
        };

        // When handling launch file.
        let _response = handle_launch_file(&mut action_ctx);

        // Then nothing happens (launcher not called, no status bar message).
        assert_eq!(launcher.launch_called.load(Ordering::SeqCst), 0);
        assert!(tui_state.status_bar.message().is_none());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn handle_launch_file_still_works_for_files() {
        // Given a TUI state with a file item selected.
        let launcher = Arc::new(FakeLauncher::new());
        let (ctx, _temp_file) = create_test_ctx_with_launcher(launcher.clone()).await;
        let mut tui_state = TuiState::new();
        tui_state.playlist_pane.items = vec![file_item("/path/to/video.mp4")];
        tui_state.focused_pane = Pane::Playlist;
        let mut fork = Fork::default();
        let mut action_ctx = TuiActionCtx {
            tui_state: &mut tui_state,
            fork: &mut fork,
            ctx: &ctx,
        };

        // When handling launch file.
        tokio::task::block_in_place(|| {
            handle_launch_file(&mut action_ctx)
        });

        // Then the launcher is called and status bar is set.
        assert_eq!(launcher.launch_called.load(Ordering::SeqCst), 1);
        assert!(tui_state.status_bar.message().is_some());
    }
}
