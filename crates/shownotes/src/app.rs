use crossterm::event::Event;
use error_stack::Report;

use crate::command::{self, Command, CommandError, CommandResult};
use crate::system_ctx::SystemCtx;
use crate::tui::{ComponentContext, TuiState};

/// Manages deferred actions that execute after the TUI suspends.
///
/// When the TUI needs to fork a process (e.g., launching an external editor),
/// it sets flags here to track what action should be taken after resuming.
/// The main loop checks [`Fork::take_action`] to determine what to do next.
#[derive(Default, Debug)]
pub struct Fork {
    /// Path to the item whose notes file should be opened in an editor.
    pub notes_path: Option<crate::common::domain::ItemPath>,
    /// Whether to open the fuzzy notes search interface.
    pub fuzzy_notes: bool,
    /// Path to the item whose sources file should be opened in an editor.
    pub sources_path: Option<crate::common::domain::ItemPath>,
    /// Output format for generating notes (e.g., "markdown").
    pub generate_notes: Option<String>,
}

/// Actions that execute after the TUI resumes from a fork.
///
/// These are extracted from [`Fork`] by calling [`Fork::take_action`],
/// which returns the highest-priority pending action and clears its flag.
pub enum ForkAction {
    /// Open the notes file for the given item path in an external editor.
    AddNote { path: crate::common::domain::ItemPath },
    /// Open the fuzzy notes search interface to find and open a notes file.
    FuzzyNotes,
    /// Open the sources file for the given item path in an external editor.
    EditSources { path: crate::common::domain::ItemPath },
    /// Generate notes for the current playlist in the specified format.
    GenerateNotes { format: String },
}

impl Fork {
    pub fn take_action(&mut self) -> Option<ForkAction> {
        if let Some(path) = self.notes_path.take() {
            return Some(ForkAction::AddNote { path });
        }
        if self.fuzzy_notes {
            self.fuzzy_notes = false;
            return Some(ForkAction::FuzzyNotes);
        }
        if let Some(path) = self.sources_path.take() {
            return Some(ForkAction::EditSources { path });
        }
        if let Some(format) = self.generate_notes.take() {
            return Some(ForkAction::GenerateNotes { format });
        }
        None
    }
}

/// Main application state container.
///
/// Holds all state needed to run the TUI application, including the system
/// context with services, configuration, the current TUI state, and
/// runtime for async operations.
pub struct App {
    /// System context containing services, configuration, and paths.
    pub ctx: SystemCtx,
    /// Current state of the TUI (focused pane, playlist items, etc.).
    pub tui_state: TuiState,
    /// Whether the application should exit on the next loop iteration.
    pub should_quit: bool,
    /// Pending actions to execute after the TUI suspends.
    pub fork: Fork,
    /// Tokio runtime for executing async operations synchronously.
    pub tokio_runtime: tokio::runtime::Runtime,
}

impl std::fmt::Debug for App {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("App")
            .field("ctx", &self.ctx)
            .field("tui_state", &self.tui_state)
            .field("should_quit", &self.should_quit)
            .field("fork", &self.fork)
            .field("tokio_runtime", &"<Runtime>")
            .finish()
    }
}

impl App {
    pub fn new(ctx: SystemCtx, tokio_runtime: tokio::runtime::Runtime) -> Self {
        let mut app = Self {
            ctx,
            tui_state: TuiState::new(),
            should_quit: false,
            fork: Fork::default(),
            tokio_runtime,
        };
        crate::tui::load_playlist(&app.ctx, &mut app.tui_state);
        crate::tui::set_initial_focus(&mut app.tui_state);
        app
    }

    /// Executes a command and returns the result.
    ///
    /// # Errors
    ///
    /// Returns an error if the command execution fails.
    pub fn execute(&mut self, command: Command) -> Result<CommandResult, Report<CommandError>> {
        self.tokio_runtime.block_on(command::execute(&self.ctx, command))
    }

    pub fn handle_event(&mut self, event: Event) {
        if let Event::Key(key) = event {
            self.tui_state.status_bar.clear();

            let ctx = ComponentContext {
                keymap: &self.ctx.keymap,
                focused_pane: self.tui_state.focused_pane,
            };

            let result = self.tui_state.handle_key(key, &ctx);

            for action in result.actions {
                let response = {
                    let mut tui_ctx = crate::tui::TuiActionCtx {
                        tui_state: &mut self.tui_state,
                        fork: &mut self.fork,
                        ctx: &self.ctx,
                    };
                    crate::tui::execute_tui_action(&mut tui_ctx, action)
                };
                if matches!(response, Ok(crate::tui::TuiActionResponse::ShouldQuit)) {
                    self.should_quit = true;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::sync::Arc;

    use crossterm::event::{KeyCode, KeyModifiers};
    use marked_path::CanonicalPath;

    use super::*;
    use crate::feat::config::Config;
    use crate::feat::FileLauncherService;
    use crate::feat::keymap::Keymap;
    use crate::tui::TuiAction;
    use crate::feat::media_query::MediaQueryService;
    use crate::feat::mpv::{MpvClientService, MpvLauncherService};
    use crate::feat::playlist::FakeStorageBackend;
    use crate::feat::playlist::PlaylistStorageService;
    use crate::services::Services;
    use crate::test_utils::{FakeLauncher, FakeMediaBackend, FakeMpvBackend, FakeMpvLauncher};
    use crate::common::domain::{ItemPath, PlaylistItem};
    use crate::tui::{get_mime_type, Pane};

    struct TestAppBuilder {
        playlist_items: Vec<PathBuf>,
        library_items: Vec<PathBuf>,
        library_path: CanonicalPath,
        mpv_launcher: FakeMpvLauncher,
        mpv_backend: FakeMpvBackend,
        media_backend: FakeMediaBackend,
        storage_backend: FakeStorageBackend,
        file_launcher: FakeLauncher,
        focused_pane: Option<Pane>,
    }

    impl TestAppBuilder {
        fn new() -> Self {
            Self {
                playlist_items: vec![],
                library_items: vec![],
                library_path: CanonicalPath::new(PathBuf::from(".")),
                mpv_launcher: FakeMpvLauncher::new(),
                mpv_backend: FakeMpvBackend,
                media_backend: FakeMediaBackend,
                storage_backend: FakeStorageBackend::new(),
                file_launcher: FakeLauncher,
                focused_pane: None,
            }
        }

        fn playlist_items(mut self, items: Vec<PathBuf>) -> Self {
            self.playlist_items = items;
            self
        }

        fn library_items(mut self, items: Vec<PathBuf>) -> Self {
            self.library_items = items;
            self
        }

        fn library_path(mut self, path: PathBuf) -> Self {
            self.library_path = CanonicalPath::new(path);
            self
        }

        fn mpv_launcher(mut self, launcher: FakeMpvLauncher) -> Self {
            self.mpv_launcher = launcher;
            self
        }

        fn focused_on(mut self, pane: Pane) -> Self {
            self.focused_pane = Some(pane);
            self
        }

        fn build(self) -> App {
            let rt = tokio::runtime::Runtime::new().unwrap();
            let handle = rt.handle().clone();
            let core =
                rt.block_on(async { Services::new("sqlite::memory:", handle).await.unwrap() });

            let services = Services {
                mpv: MpvClientService::new(Arc::new(self.mpv_backend)),
                media: MediaQueryService::new(Arc::new(self.media_backend)),
                storage: PlaylistStorageService::new(Arc::new(self.storage_backend)),
                mpv_launcher: MpvLauncherService::new(Arc::new(self.mpv_launcher)),
                file_launcher: FileLauncherService::new(Arc::new(self.file_launcher)),
                db: core.db,
                editor: core.editor,
                path_resolver: core.path_resolver,
                sources: core.sources,
                fuzzy_search: core.fuzzy_search,
                rt: core.rt,
            };

            let ctx = SystemCtx {
                services,
                config: Config::default(),
                library_path: self.library_path,
                socket_path: String::from("/tmp/mpvsocket"),
                keymap: Keymap::new(),
            };

            let mut app = App {
                ctx,
                tui_state: TuiState::new(),
                should_quit: false,
                fork: Fork::default(),
                tokio_runtime: rt,
            };

            for path in self.playlist_items {
                let item_path = ItemPath::File(CanonicalPath::new(path.clone()));
                let duration = app.ctx.services.media.get_duration(&path).ok();
                let mime_type = get_mime_type(&item_path);
                app.tui_state.playlist_pane.items.push(PlaylistItem {
                    path: item_path,
                    duration,
                    alias: None,
                    mime_type,
                    is_virtual: false,
                    playlist_count: 0,
                });
            }

            for path in self.library_items {
                let item_path = ItemPath::File(CanonicalPath::new(path.clone()));
                let duration = app.ctx.services.media.get_duration(&path).ok();
                let mime_type = get_mime_type(&item_path);
                app.tui_state.library_pane.items.push(PlaylistItem {
                    path: item_path,
                    duration,
                    alias: None,
                    mime_type,
                    is_virtual: false,
                    playlist_count: 0,
                });
            }

            crate::tui::set_initial_focus(&mut app.tui_state);
            if let Some(pane) = self.focused_pane {
                app.tui_state.focused_pane = pane;
            }
            app
        }
    }

    fn execute_actions(app: &mut App, actions: &[TuiAction]) {
        for action in actions {
            let response = {
                let mut tui_ctx = crate::tui::TuiActionCtx {
                    tui_state: &mut app.tui_state,
                    fork: &mut app.fork,
                    ctx: &app.ctx,
                };
                crate::tui::execute_tui_action(&mut tui_ctx, action.clone())
            };
            match response {
                Ok(crate::tui::TuiActionResponse::ShouldQuit) => app.should_quit = true,
                Ok(crate::tui::TuiActionResponse::Continue) => {}
                Err(e) => panic!("Action failed in test: {e:?}"),
            }
        }
    }

    fn execute_action(app: &mut App, action: TuiAction) {
        let response = {
            let mut tui_ctx = crate::tui::TuiActionCtx {
                tui_state: &mut app.tui_state,
                fork: &mut app.fork,
                ctx: &app.ctx,
            };
            crate::tui::execute_tui_action(&mut tui_ctx, action)
        };
        match response {
            Ok(crate::tui::TuiActionResponse::ShouldQuit) => app.should_quit = true,
            Ok(crate::tui::TuiActionResponse::Continue) => {}
            Err(e) => panic!("Action failed in test: {e:?}"),
        }
    }

    fn key_event(c: char) -> Event {
        Event::Key(crossterm::event::KeyEvent::new(
            KeyCode::Char(c),
            KeyModifiers::empty(),
        ))
    }

    #[test]
    fn quit_action_saves_and_exits() {
        // Given an empty app.
        let mut app = TestAppBuilder::new().build();

        // When executing Quit action.
        execute_action(&mut app, TuiAction::Quit);

        // Then the app should quit and show saved message.
        assert!(app.should_quit);
        assert!(
            app.tui_state
                .status_bar
                .message()
                .unwrap()
                .contains("saved")
        );
    }

    #[test]
    fn save_action_shows_status_message() {
        // Given an app with one item in the playlist.
        let mut app = TestAppBuilder::new()
            .playlist_items(vec![PathBuf::from("test.mp4")])
            .build();

        // When executing Save action.
        execute_action(&mut app, TuiAction::Save);

        // Then a saved status message is shown.
        assert!(
            app.tui_state
                .status_bar
                .message()
                .unwrap()
                .contains("saved")
        );
    }

    #[test]
    fn switch_pane_toggles_between_panes() {
        // Given an app focused on the playlist pane with items in both panes.
        let mut app = TestAppBuilder::new()
            .playlist_items(vec![PathBuf::from("playlist.mp4")])
            .library_items(vec![PathBuf::from("library.mp4")])
            .build();

        // When executing SwitchPane action twice.
        execute_actions(&mut app, &[TuiAction::SwitchPane, TuiAction::SwitchPane]);

        // Then focus switches to library then back to playlist.
        assert_eq!(app.tui_state.focused_pane, Pane::Playlist);
    }

    #[test]
    fn focus_playlist_switches_to_playlist_pane() {
        // Given an app focused on the library pane with items in both panes.
        let mut app = TestAppBuilder::new()
            .playlist_items(vec![PathBuf::from("playlist.mp4")])
            .library_items(vec![PathBuf::from("library.mp4")])
            .build();
        app.tui_state.focused_pane = Pane::Library;

        // When executing FocusPlaylist action.
        execute_action(&mut app, TuiAction::FocusPlaylist);

        // Then focus switches to playlist pane.
        assert_eq!(app.tui_state.focused_pane, Pane::Playlist);
    }

    #[test]
    fn focus_library_switches_to_library_pane() {
        // Given an app focused on the playlist pane with items in both panes.
        let mut app = TestAppBuilder::new()
            .playlist_items(vec![PathBuf::from("playlist.mp4")])
            .library_items(vec![PathBuf::from("library.mp4")])
            .build();

        // When executing FocusLibrary action.
        execute_action(&mut app, TuiAction::FocusLibrary);

        // Then focus switches to library pane.
        assert_eq!(app.tui_state.focused_pane, Pane::Library);
    }

    #[test]
    fn move_down_moves_selection_down_in_playlist() {
        // Given a playlist with three items.
        let mut app = TestAppBuilder::new()
            .playlist_items(vec![
                PathBuf::from("a.mp4"),
                PathBuf::from("b.mp4"),
                PathBuf::from("c.mp4"),
            ])
            .build();

        // When executing MoveDown action three times.
        execute_actions(
            &mut app,
            &[TuiAction::MoveDown, TuiAction::MoveDown, TuiAction::MoveDown],
        );

        // Then selection starts at 0 and stays at the last item (2).
        assert_eq!(app.tui_state.playlist_pane.selected, 2);
    }

    #[test]
    fn move_up_moves_selection_up_in_playlist() {
        // Given a playlist with three items and selection on the last item.
        let mut app = TestAppBuilder::new()
            .playlist_items(vec![
                PathBuf::from("a.mp4"),
                PathBuf::from("b.mp4"),
                PathBuf::from("c.mp4"),
            ])
            .build();
        app.tui_state.playlist_pane.selected = 2;

        // When executing MoveUp action three times.
        execute_actions(&mut app, &[TuiAction::MoveUp, TuiAction::MoveUp, TuiAction::MoveUp]);

        // Then selection stays at the first item.
        assert_eq!(app.tui_state.playlist_pane.selected, 0);
    }

    #[test]
    fn move_up_down_navigate_library() {
        // Given a library with three items.
        let mut app = TestAppBuilder::new()
            .library_items(vec![
                PathBuf::from("x.mp4"),
                PathBuf::from("y.mp4"),
                PathBuf::from("z.mp4"),
            ])
            .focused_on(Pane::Library)
            .build();

        // When navigating with MoveDown then MoveUp.
        execute_actions(&mut app, &[TuiAction::MoveDown, TuiAction::MoveUp]);

        // Then selection starts at 0 and returns to 0.
        assert_eq!(app.tui_state.library_pane.selected, 0);
    }

    #[test]
    fn reorder_up_moves_playlist_item_up() {
        // Given a playlist with three items and middle item selected.
        let mut app = TestAppBuilder::new()
            .playlist_items(vec![
                PathBuf::from("a.mp4"),
                PathBuf::from("b.mp4"),
                PathBuf::from("c.mp4"),
            ])
            .build();
        app.tui_state.focused_pane = Pane::Playlist;
        app.tui_state.playlist_pane.selected = 1;

        // When executing ReorderUp action.
        execute_action(&mut app, TuiAction::ReorderUp);

        // Then the item moves up and selection follows.
        assert_eq!(app.tui_state.playlist_pane.selected, 0);
        assert_eq!(
            app.tui_state.playlist_pane.items[0].path,
            ItemPath::File(CanonicalPath::new(PathBuf::from("b.mp4")))
        );
        assert_eq!(
            app.tui_state.playlist_pane.items[1].path,
            ItemPath::File(CanonicalPath::new(PathBuf::from("a.mp4")))
        );
    }

    #[test]
    fn reorder_down_moves_playlist_item_down() {
        // Given a playlist with items reordered and first item selected.
        let mut app = TestAppBuilder::new()
            .playlist_items(vec![
                PathBuf::from("b.mp4"),
                PathBuf::from("a.mp4"),
                PathBuf::from("c.mp4"),
            ])
            .build();
        app.tui_state.focused_pane = Pane::Playlist;
        app.tui_state.playlist_pane.selected = 0;

        // When executing ReorderDown action.
        execute_action(&mut app, TuiAction::ReorderDown);

        // Then the item moves down and selection follows.
        assert_eq!(app.tui_state.playlist_pane.selected, 1);
        assert_eq!(
            app.tui_state.playlist_pane.items[0].path,
            ItemPath::File(CanonicalPath::new(PathBuf::from("a.mp4")))
        );
        assert_eq!(
            app.tui_state.playlist_pane.items[1].path,
            ItemPath::File(CanonicalPath::new(PathBuf::from("b.mp4")))
        );
    }

    #[test]
    fn move_to_playlist_moves_library_item_to_playlist() {
        // Given a library with one item and empty playlist.
        let mut app = TestAppBuilder::new()
            .library_items(vec![PathBuf::from("test.mp4")])
            .build();
        app.tui_state.focused_pane = Pane::Library;

        // When executing MoveToPlaylist action.
        execute_action(&mut app, TuiAction::MoveToPlaylist);

        // Then the item moves to the playlist.
        assert_eq!(app.tui_state.playlist_pane.items.len(), 1);
        assert_eq!(
            app.tui_state.playlist_pane.items[0].path,
            ItemPath::File(CanonicalPath::new(PathBuf::from("test.mp4")))
        );
        assert!(app.tui_state.library_pane.items.is_empty());
    }

    #[test]
    fn move_to_library_moves_playlist_item_to_library() {
        // Given a playlist with one item and empty library.
        let temp_file = tempfile::NamedTempFile::new().unwrap();
        let temp_path = temp_file.path().to_path_buf();
        let mut app = TestAppBuilder::new()
            .playlist_items(vec![temp_path.clone()])
            .build();
        app.tui_state.focused_pane = Pane::Playlist;

        // When executing MoveToLibrary action.
        execute_action(&mut app, TuiAction::MoveToLibrary);

        // Then the item moves to the library.
        assert!(app.tui_state.playlist_pane.items.is_empty());
        assert_eq!(app.tui_state.library_pane.items.len(), 1);
        assert_eq!(
            app.tui_state.library_pane.items[0].path,
            ItemPath::File(CanonicalPath::new(temp_path))
        );
    }

    #[test]
    fn launch_file_shows_status_message() {
        // Given a playlist with one item.
        let mut app = TestAppBuilder::new()
            .playlist_items(vec![PathBuf::from("test.mp4")])
            .build();

        // When executing LaunchFile action.
        execute_action(&mut app, TuiAction::LaunchFile);

        // Then an opening status message is shown.
        assert!(
            app.tui_state
                .status_bar
                .message()
                .unwrap()
                .contains("Opening")
        );
    }

    #[test]
    fn load_playlist_shows_status_message() {
        // Given a playlist with items.
        let mut app = TestAppBuilder::new()
            .playlist_items(vec![PathBuf::from("a.mp4"), PathBuf::from("b.mp4")])
            .build();

        // When executing LoadPlaylist action.
        execute_action(&mut app, TuiAction::LoadPlaylist);

        // Then a loaded status message is shown.
        assert!(
            app.tui_state
                .status_bar
                .message()
                .unwrap()
                .contains("Loaded 2 items")
        );
    }

    #[test]
    fn load_playlist_shows_error_when_empty() {
        // Given an empty playlist.
        let mut app = TestAppBuilder::new().build();

        // When executing LoadPlaylist action.
        execute_action(&mut app, TuiAction::LoadPlaylist);

        // Then an error is shown.
        assert!(app.tui_state.is_showing_error());
    }

    #[test]
    fn switch_pane_does_not_switch_to_empty_library() {
        // Given a playlist with items and empty library.
        let mut app = TestAppBuilder::new()
            .playlist_items(vec![PathBuf::from("test.mp4")])
            .build();

        // When executing SwitchPane action.
        execute_action(&mut app, TuiAction::SwitchPane);

        // Then focus stays on playlist.
        assert_eq!(app.tui_state.focused_pane, Pane::Playlist);
    }

    #[test]
    fn switch_pane_does_not_switch_to_empty_playlist() {
        // Given an empty playlist and library with items.
        let mut app = TestAppBuilder::new()
            .library_items(vec![PathBuf::from("test.mp4")])
            .build();

        // When executing SwitchPane action.
        execute_action(&mut app, TuiAction::SwitchPane);

        // Then focus stays on library.
        assert_eq!(app.tui_state.focused_pane, Pane::Library);
    }

    #[test]
    fn focus_playlist_does_not_switch_to_empty_playlist() {
        // Given an empty playlist and library with items, focused on library.
        let mut app = TestAppBuilder::new()
            .library_items(vec![PathBuf::from("test.mp4")])
            .build();
        app.tui_state.focused_pane = Pane::Library;

        // When executing FocusPlaylist action.
        execute_action(&mut app, TuiAction::FocusPlaylist);

        // Then focus stays on library.
        assert_eq!(app.tui_state.focused_pane, Pane::Library);
    }

    #[test]
    fn focus_library_does_not_switch_to_empty_library() {
        // Given a playlist with items and empty library, focused on playlist.
        let mut app = TestAppBuilder::new()
            .playlist_items(vec![PathBuf::from("test.mp4")])
            .build();

        // When executing FocusLibrary action.
        execute_action(&mut app, TuiAction::FocusLibrary);

        // Then focus stays on playlist.
        assert_eq!(app.tui_state.focused_pane, Pane::Playlist);
    }

    #[test]
    fn initial_focus_library_when_playlist_empty() {
        // Given an empty playlist and library with items.
        let app = TestAppBuilder::new()
            .library_items(vec![PathBuf::from("test.mp4")])
            .build();

        // Then focus is on library.
        assert_eq!(app.tui_state.focused_pane, Pane::Library);
    }

    #[test]
    fn initial_focus_playlist_when_library_empty() {
        // Given a playlist with items and empty library.
        let app = TestAppBuilder::new()
            .playlist_items(vec![PathBuf::from("test.mp4")])
            .build();

        // Then focus is on playlist.
        assert_eq!(app.tui_state.focused_pane, Pane::Playlist);
    }

    #[test]
    fn initial_focus_playlist_when_both_have_items() {
        // Given both panes with items.
        let app = TestAppBuilder::new()
            .playlist_items(vec![PathBuf::from("a.mp4")])
            .library_items(vec![PathBuf::from("b.mp4")])
            .build();

        // Then focus is on playlist (default).
        assert_eq!(app.tui_state.focused_pane, Pane::Playlist);
    }

    #[test]
    fn initial_focus_playlist_when_both_empty() {
        // Given both panes empty.
        let app = TestAppBuilder::new().build();

        // Then focus is on playlist (default).
        assert_eq!(app.tui_state.focused_pane, Pane::Playlist);
    }

    #[test]
    fn show_help_toggles_which_key() {
        // Given an app with which-key inactive.
        let mut app = TestAppBuilder::new().build();

        // When executing ShowHelp action twice.
        execute_actions(&mut app, &[TuiAction::ShowHelp, TuiAction::ShowHelp]);

        // Then which-key starts inactive, becomes active, then inactive again.
        assert!(!app.tui_state.global_handler.is_showing_help());
    }

    #[test]
    fn start_filter_activates_on_playlist() {
        // Given an app focused on playlist with items, not filtering.
        let mut app = TestAppBuilder::new()
            .playlist_items(vec![PathBuf::from("test.mp4")])
            .build();

        // When executing StartFilter action.
        execute_action(&mut app, TuiAction::StartFilter);

        // Then filter mode is active.
        assert!(app.tui_state.is_filtering());
    }

    #[test]
    fn start_filter_activates_on_library() {
        // Given an app focused on library with items, not filtering.
        let mut app = TestAppBuilder::new()
            .library_items(vec![PathBuf::from("test.mp4")])
            .focused_on(Pane::Library)
            .build();

        // When executing StartFilter action.
        execute_action(&mut app, TuiAction::StartFilter);

        // Then filter mode is active.
        assert!(app.tui_state.is_filtering());
    }

    #[test]
    fn rename_starts_rename_mode() {
        // Given an app with a selected item, not renaming.
        let mut app = TestAppBuilder::new()
            .playlist_items(vec![PathBuf::from("test.mp4")])
            .build();

        // When executing Rename action.
        execute_action(&mut app, TuiAction::Rename);

        // Then rename mode is active.
        assert!(app.tui_state.is_renaming());
    }

    #[test]
    fn notes_sets_pending_path_when_item_selected() {
        // Given an app with a selected item and no pending notes path.
        let mut app = TestAppBuilder::new()
            .playlist_items(vec![PathBuf::from("/path/to/video.mp4")])
            .build();

        // When executing Notes action.
        execute_action(&mut app, TuiAction::Notes);

        // Then pending notes path is set to the selected item's path.
        assert_eq!(
            app.fork.notes_path,
            Some(ItemPath::File(CanonicalPath::new(PathBuf::from(
                "/path/to/video.mp4"
            ))))
        );
    }

    #[test]
    fn notes_does_nothing_when_no_selection() {
        // Given an app with no items selected and no pending notes path.
        let mut app = TestAppBuilder::new().build();

        // When executing Notes action.
        execute_action(&mut app, TuiAction::Notes);

        // Then pending notes path remains unset.
        assert!(app.fork.notes_path.is_none());
    }

    #[test]
    fn launch_mpv_shows_message_when_not_running() {
        // Given an app with mpv not running.
        let mut app = TestAppBuilder::new()
            .mpv_launcher(FakeMpvLauncher::new().running(false))
            .build();

        // When executing LaunchMpv action.
        execute_action(&mut app, TuiAction::LaunchMpv);

        // Then status message shows mpv launched.
        assert!(
            app.tui_state
                .status_bar
                .message()
                .unwrap()
                .contains("MPV launched")
        );
    }

    #[test]
    fn launch_mpv_shows_message_when_already_running() {
        // Given an app with mpv already running.
        let mut app = TestAppBuilder::new()
            .mpv_launcher(FakeMpvLauncher::new().running(true))
            .build();

        // When executing LaunchMpv action.
        execute_action(&mut app, TuiAction::LaunchMpv);

        // Then status message shows mpv already running.
        assert!(
            app.tui_state
                .status_bar
                .message()
                .unwrap()
                .contains("MPV already running")
        );
    }

    #[test]
    fn g_key_sets_pending_and_shows_followup() {
        let mut app = TestAppBuilder::new().build();

        app.handle_event(key_event('g'));

        assert!(app.tui_state.global_handler.is_showing_help());
    }

    #[test]
    fn gm_keys_launches_mpv() {
        let mut app = TestAppBuilder::new()
            .mpv_launcher(FakeMpvLauncher::new().running(false))
            .build();

        app.handle_event(key_event('g'));
        app.handle_event(key_event('m'));

        assert!(
            app.tui_state
                .status_bar
                .message()
                .unwrap()
                .contains("MPV launched")
        );
        assert!(!app.tui_state.global_handler.is_showing_help());
    }

    #[test]
    fn g_then_invalid_key_dismisses_popup() {
        let mut app = TestAppBuilder::new().build();
        app.handle_event(key_event('g'));

        app.handle_event(key_event('x'));

        assert!(!app.tui_state.global_handler.is_showing_help());
    }

    #[test]
    fn a_key_sets_pending_and_shows_followup() {
        let mut app = TestAppBuilder::new().build();

        app.handle_event(key_event('a'));

        assert!(app.tui_state.global_handler.is_showing_help());
    }

    #[test]
    fn au_keys_starts_url_input() {
        let mut app = TestAppBuilder::new().build();

        app.handle_event(key_event('a'));
        app.handle_event(key_event('u'));

        assert!(app.tui_state.is_url_input());
        assert!(!app.tui_state.global_handler.is_showing_help());
    }

    #[test]
    fn a_then_invalid_key_dismisses_popup() {
        let mut app = TestAppBuilder::new().build();
        app.handle_event(key_event('a'));

        app.handle_event(key_event('x'));

        assert!(!app.tui_state.global_handler.is_showing_help());
        assert!(!app.tui_state.is_url_input());
    }

    #[test]
    fn virtual_item_preserved_when_moved_from_library_to_playlist() {
        // Given a library with a virtual URL item.
        let mut app = TestAppBuilder::new().build();
        let url = "https://example.com/video.mp4";
        app.tui_state.library_pane.items.push(PlaylistItem {
            path: ItemPath::Url(url.to_string()),
            duration: None,
            alias: None,
            mime_type: Some("url".to_string()),
            is_virtual: true,
            playlist_count: 0,
        });
        app.tui_state.focused_pane = Pane::Library;

        // When moving to playlist.
        execute_action(&mut app, TuiAction::MoveToPlaylist);

        // Then the item is in playlist with is_virtual preserved.
        assert_eq!(app.tui_state.playlist_pane.items.len(), 1);
        assert!(app.tui_state.library_pane.items.is_empty());
        assert_eq!(
            app.tui_state.playlist_pane.items[0].path,
            ItemPath::Url(url.to_string())
        );
        assert!(app.tui_state.playlist_pane.items[0].is_virtual);
        assert_eq!(
            app.tui_state.playlist_pane.items[0].mime_type,
            Some("url".to_string())
        );
    }

    #[test]
    fn virtual_item_preserved_when_moved_from_playlist_to_library() {
        // Given a playlist with a virtual URL item.
        let mut app = TestAppBuilder::new().build();
        let url = "https://example.com/video.mp4";
        app.tui_state.playlist_pane.items.push(PlaylistItem {
            path: ItemPath::Url(url.to_string()),
            duration: None,
            alias: None,
            mime_type: Some("url".to_string()),
            is_virtual: true,
            playlist_count: 0,
        });
        app.tui_state.focused_pane = Pane::Playlist;

        // When moving to library.
        execute_action(&mut app, TuiAction::MoveToLibrary);

        // Then the item is in library with is_virtual preserved.
        assert!(app.tui_state.playlist_pane.items.is_empty());
        assert_eq!(app.tui_state.library_pane.items.len(), 1);
        assert_eq!(
            app.tui_state.library_pane.items[0].path,
            ItemPath::Url(url.to_string())
        );
        assert!(app.tui_state.library_pane.items[0].is_virtual);
        assert_eq!(
            app.tui_state.library_pane.items[0].mime_type,
            Some("url".to_string())
        );
    }

    #[test]
    fn virtual_item_roundtrip_preserves_all_properties() {
        // Given a library with a virtual URL item with all properties.
        let mut app = TestAppBuilder::new().build();
        let url = "https://youtube.com/watch?v=abc123";
        app.tui_state.library_pane.items.push(PlaylistItem {
            path: ItemPath::Url(url.to_string()),
            duration: Some(std::time::Duration::from_secs(300)),
            alias: Some("My Video".to_string()),
            mime_type: Some("url".to_string()),
            is_virtual: true,
            playlist_count: 0,
        });
        app.tui_state.focused_pane = Pane::Library;

        // When moving library -> playlist -> library.
        execute_actions(&mut app, &[TuiAction::MoveToPlaylist, TuiAction::MoveToLibrary]);

        // Then all properties are preserved.
        assert_eq!(app.tui_state.library_pane.items.len(), 1);
        let item = &app.tui_state.library_pane.items[0];
        assert_eq!(item.path, ItemPath::Url(url.to_string()));
        assert!(item.is_virtual);
        assert_eq!(item.mime_type, Some("url".to_string()));
        assert_eq!(item.alias, Some("My Video".to_string()));
        assert_eq!(item.duration, Some(std::time::Duration::from_secs(300)));
    }

    #[test]
    fn missing_file_removed_when_moved_from_playlist_to_library() {
        // Given a playlist with a missing non-virtual file.
        let mut app = TestAppBuilder::new().build();
        app.tui_state.playlist_pane.items.push(PlaylistItem {
            path: ItemPath::File(CanonicalPath::new(PathBuf::from("/nonexistent/file.mp4"))),
            duration: None,
            alias: None,
            mime_type: Some("video/mp4".to_string()),
            is_virtual: false,
            playlist_count: 0,
        });
        app.tui_state.focused_pane = Pane::Playlist;

        // When moving to library.
        execute_action(&mut app, TuiAction::MoveToLibrary);

        // Then the item is removed (not added to library).
        assert!(app.tui_state.playlist_pane.items.is_empty());
        assert!(app.tui_state.library_pane.items.is_empty());
    }

    #[test]
    fn refresh_library_preserves_virtual_items() {
        // Given a temp directory with one file and an app with a virtual item.
        let tree = temptree::temptree! {
            "real.mp4": "video content",
        };

        let mut app = TestAppBuilder::new()
            .library_path(tree.path().to_path_buf())
            .build();
        let url = "https://example.com/video.mp4";
        app.tui_state.library_pane.items.push(PlaylistItem {
            path: ItemPath::Url(url.to_string()),
            duration: None,
            alias: None,
            mime_type: Some("url".to_string()),
            is_virtual: true,
            playlist_count: 0,
        });

        // When refreshing the library.
        crate::tui::refresh_library(&app.ctx, &mut app.tui_state);

        // Then both the virtual item and real file are present.
        assert_eq!(app.tui_state.library_pane.items.len(), 2);
        assert!(
            app.tui_state
                .library_pane
                .items
                .iter()
                .any(|i| i.path == ItemPath::Url(url.to_string()) && i.is_virtual)
        );
        assert!(app.tui_state.library_pane.items.iter().any(|i| {
            i.path
                .as_file()
                .is_some_and(|p| p.as_path().file_name().unwrap() == "real.mp4")
                && !i.is_virtual
        }));
    }

    #[test]
    fn refresh_library_removes_missing_non_virtual_items() {
        // Given a temp directory with no files and an app with a non-virtual item.
        let tree = temptree::temptree! {};

        let mut app = TestAppBuilder::new()
            .library_path(tree.path().to_path_buf())
            .build();
        app.tui_state.library_pane.items.push(PlaylistItem {
            path: ItemPath::File(CanonicalPath::new(PathBuf::from("/nonexistent/file.mp4"))),
            duration: None,
            alias: None,
            mime_type: Some("video/mp4".to_string()),
            is_virtual: false,
            playlist_count: 0,
        });

        // When refreshing the library.
        crate::tui::refresh_library(&app.ctx, &mut app.tui_state);

        // Then the missing non-virtual item is removed.
        assert!(app.tui_state.library_pane.items.is_empty());
    }

    #[test]
    fn delete_action_removes_virtual_item_from_library() {
        // Given an app with a virtual item in the library.
        let mut app = TestAppBuilder::new().build();
        let url = "https://example.com/video.mp4";
        app.tui_state.library_pane.items.push(PlaylistItem {
            path: ItemPath::Url(url.to_string()),
            duration: None,
            alias: None,
            mime_type: Some("url".to_string()),
            is_virtual: true,
            playlist_count: 0,
        });
        app.tui_state.focused_pane = Pane::Library;

        // When executing Delete action.
        execute_action(&mut app, TuiAction::Delete);

        // Then the virtual item is removed.
        assert!(app.tui_state.library_pane.items.is_empty());
    }

    #[test]
    fn delete_action_rejected_for_non_virtual_items() {
        // Given an app with a non-virtual item in the library.
        let temp_file = tempfile::NamedTempFile::new().unwrap();
        let temp_path = temp_file.path().to_path_buf();
        let mut app = TestAppBuilder::new()
            .library_items(vec![temp_path.clone()])
            .build();
        app.tui_state.focused_pane = Pane::Library;

        // When executing Delete action.
        execute_action(&mut app, TuiAction::Delete);

        // Then the item is NOT removed (only virtual items can be deleted).
        assert_eq!(app.tui_state.library_pane.items.len(), 1);
        assert!(
            app.tui_state
                .status_bar
                .message()
                .unwrap()
                .contains("virtual")
        );
    }

    #[test]
    fn fork_take_action_returns_none_when_empty() {
        // Given an empty fork struct.
        let mut fork = Fork::default();

        // When taking an action.
        let action = fork.take_action();

        // Then no action is returned.
        assert!(action.is_none());
    }

    #[test]
    fn fork_take_action_returns_add_note_and_clears_flag() {
        // Given a fork with notes_path set.
        let mut fork = Fork {
            notes_path: Some(ItemPath::File(CanonicalPath::new(PathBuf::from(
                "/test/path",
            )))),
            ..Default::default()
        };

        // When taking an action.
        let action = fork.take_action();

        // Then AddNote action is returned and flag is cleared.
        assert!(matches!(action, Some(ForkAction::AddNote { .. })));
        assert!(fork.notes_path.is_none());
    }

    #[test]
    fn fork_take_action_returns_fuzzy_notes_and_clears_flag() {
        // Given a fork with fuzzy_notes set.
        let mut fork = Fork {
            fuzzy_notes: true,
            ..Default::default()
        };

        // When taking an action.
        let action = fork.take_action();

        // Then FuzzyNotes action is returned and flag is cleared.
        assert!(matches!(action, Some(ForkAction::FuzzyNotes)));
        assert!(!fork.fuzzy_notes);
    }

    #[test]
    fn fork_take_action_returns_edit_sources_and_clears_flag() {
        // Given a fork with sources_path set.
        let mut fork = Fork {
            sources_path: Some(ItemPath::File(CanonicalPath::new(PathBuf::from(
                "/test/sources",
            )))),
            ..Default::default()
        };

        // When taking an action.
        let action = fork.take_action();

        // Then EditSources action is returned and flag is cleared.
        assert!(matches!(action, Some(ForkAction::EditSources { .. })));
        assert!(fork.sources_path.is_none());
    }

    #[test]
    fn fork_take_action_returns_generate_notes_and_clears_flag() {
        // Given a fork with generate_notes set.
        let mut fork = Fork {
            generate_notes: Some("markdown".to_string()),
            ..Default::default()
        };

        // When taking an action.
        let action = fork.take_action();

        // Then GenerateNotes action is returned and flag is cleared.
        assert!(matches!(action, Some(ForkAction::GenerateNotes { .. })));
        assert!(fork.generate_notes.is_none());
    }

    #[test]
    fn fork_take_action_priority_order() {
        // Given a fork with multiple flags set.
        let mut fork = Fork {
            notes_path: Some(ItemPath::File(CanonicalPath::new(PathBuf::from("/note")))),
            fuzzy_notes: true,
            ..Default::default()
        };

        // When taking an action.
        let action = fork.take_action();

        // Then notes_path has highest priority.
        assert!(matches!(action, Some(ForkAction::AddNote { .. })));
    }
}
