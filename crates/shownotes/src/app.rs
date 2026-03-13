use std::path::PathBuf;

use crossterm::event::Event;
use marked_path::CanonicalPath;

use crate::feat::config::Config;
use crate::feat::keymap::{Action, Keymap};
use crate::feat::playlist::PlaylistData;
use crate::services::Services;
use crate::tui::{ComponentContext, ItemPath, Pane, PlaylistItem, TuiState, get_mime_type};

/// Holds pending actions that require forking from the TUI.
///
/// When certain actions need to spawn an external process (like opening an editor
/// for notes or sources), the TUI must exit and the action is stored here. The
/// main loop checks this struct and executes any pending action before continuing.
#[derive(Default)]
pub struct Fork {
    pub notes_path: Option<ItemPath>,
    pub fuzzy_notes: bool,
    pub sources_path: Option<ItemPath>,
    pub generate_notes: Option<String>,
}

/// Actions that can be taken when forking from the TUI.
///
/// Each variant represents an external operation that requires suspending
/// the terminal UI to interact with an external program or output.
pub enum ForkAction {
    AddNote { path: ItemPath },
    FuzzyNotes,
    EditSources { path: ItemPath },
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

/// Runtime configuration computed at application startup.
///
/// Contains settings that are determined once when the application starts
/// and remain constant throughout the application's lifetime, such as
/// the keymap, socket path, and library/playlist paths.
pub struct RuntimeSettings {
    pub keymap: Keymap,
    pub socket_path: String,
    pub library_path: CanonicalPath,
}

/// The main application state container.
///
/// Holds all the state needed to run the shownotes TUI application, including
/// services for external interactions, TUI state for rendering, user configuration,
/// runtime settings, and the tokio runtime for async operations.
pub struct App {
    pub services: Services,
    pub tui_state: TuiState,
    pub config: Config,
    pub should_quit: bool,
    pub fork: Fork,
    pub runtime: RuntimeSettings,
    pub tokio_runtime: tokio::runtime::Runtime,
}

impl App {
    pub fn new(
        services: Services,
        config: Config,
        socket_path: String,
        library_path: CanonicalPath,
        tokio_runtime: tokio::runtime::Runtime,
    ) -> Self {
        let mut app = Self {
            services,
            tui_state: TuiState::new(),
            config,
            should_quit: false,
            fork: Fork::default(),
            runtime: RuntimeSettings {
                keymap: Keymap::new(),
                socket_path,
                library_path,
            },
            tokio_runtime,
        };
        app.load_playlist();
        app.refresh_library();
        app.set_initial_focus();
        app
    }

    fn set_initial_focus(&mut self) {
        let playlist_empty = self.tui_state.playlist_pane.items.is_empty();
        let library_empty = self.tui_state.library_pane.items.is_empty();
        if playlist_empty && !library_empty {
            self.tui_state.focused_pane = Pane::Library;
        } else if library_empty && !playlist_empty {
            self.tui_state.focused_pane = Pane::Playlist;
        }
    }

    pub fn load_playlist(&mut self) {
        let result = self
            .tokio_runtime
            .block_on(async { self.services.storage.load(&self.runtime.library_path).await });
        match result {
            Ok(data) => {
                let playlist_paths: std::collections::HashSet<_> =
                    data.playlist.iter().cloned().collect();

                self.tui_state.playlist_pane.items = data
                    .playlist
                    .into_iter()
                    .map(|path| {
                        let metadata = data.files.get(&path);
                        let is_virtual = metadata.is_some_and(|m| m.is_virtual);
                        let duration = metadata.and_then(|m| m.duration);
                        let mime_type = metadata
                            .and_then(|m| m.mime_type.clone())
                            .or_else(|| get_mime_type(&path));
                        PlaylistItem {
                            path,
                            duration,
                            alias: metadata.and_then(|m| m.alias.clone()),
                            mime_type,
                            is_virtual,
                        }
                    })
                    .collect();

                let mut virtual_library_items: Vec<PlaylistItem> = data
                    .files
                    .into_iter()
                    .filter(|(path, metadata)| {
                        metadata.is_virtual && !playlist_paths.contains(path)
                    })
                    .map(|(path, metadata)| {
                        let item_path = ItemPath::Url(path.to_string_lossy().to_string());
                        let mime_type = metadata.mime_type.or_else(|| get_mime_type(&item_path));
                        PlaylistItem {
                            path: item_path,
                            duration: metadata.duration,
                            alias: metadata.alias.clone(),
                            mime_type,
                            is_virtual: true,
                        }
                    })
                    .collect();
                virtual_library_items
                    .sort_by(|a, b| a.path.to_string_lossy().cmp(&b.path.to_string_lossy()));

                self.refresh_library();

                for item in virtual_library_items {
                    self.tui_state.library_pane.items.push(item);
                }
                self.tui_state
                    .library_pane
                    .items
                    .sort_by(|a, b| a.path.to_string_lossy().cmp(&b.path.to_string_lossy()));
            }
            Err(e) => {
                self.tui_state
                    .show_error(format!("Failed to load playlist: {e:?}"));
            }
        }
    }

    pub fn save_playlist(&mut self) {
        let mut files = std::collections::HashMap::new();
        for item in &self.tui_state.playlist_pane.items {
            files.insert(
                item.path.clone(),
                crate::feat::playlist::FileMetadata {
                    duration: item.duration,
                    is_virtual: item.is_virtual,
                    deleted: false,
                    mime_type: item.mime_type.clone(),
                    time_added: None,
                    alias: item.alias.clone(),
                },
            );
        }
        for item in &self.tui_state.library_pane.items {
            files.insert(
                item.path.clone(),
                crate::feat::playlist::FileMetadata {
                    duration: item.duration,
                    is_virtual: item.is_virtual,
                    deleted: false,
                    mime_type: item.mime_type.clone(),
                    time_added: None,
                    alias: item.alias.clone(),
                },
            );
        }
        let playlist_paths: Vec<ItemPath> = self
            .tui_state
            .playlist_pane
            .items
            .iter()
            .map(|item| item.path.clone())
            .collect();
        let data = PlaylistData {
            working_directory: self.runtime.library_path.clone(),
            playlist: playlist_paths,
            files,
        };
        let result = self
            .tokio_runtime
            .block_on(async { self.services.storage.save(&data).await });
        match result {
            Ok(()) => {
                self.tui_state.status_bar.set("Playlist saved");
            }
            Err(e) => {
                self.tui_state
                    .show_error(format!("Failed to save playlist: {e:?}"));
            }
        }
    }

    pub fn refresh_library(&mut self) {
        let mut entries = Vec::new();
        if let Ok(read_dir) = std::fs::read_dir(self.runtime.library_path.as_path()) {
            let paths: Vec<_> = read_dir
                .flatten()
                .map(|entry| entry.path())
                .filter(|path| path.is_file())
                .collect();

            let workspace = self.runtime.library_path.clone();
            let services = self.services.clone();
            let aliases: std::collections::HashMap<PathBuf, Option<String>> =
                self.tokio_runtime.block_on(async {
                    let mut result = std::collections::HashMap::new();
                    for path in &paths {
                        let canonical = CanonicalPath::new(
                            path.canonicalize().unwrap_or_else(|_| path.clone()),
                        );
                        let alias = services
                            .storage
                            .resolve_alias(&canonical, &workspace)
                            .await
                            .ok()
                            .flatten();
                        result.insert(canonical.to_path_buf(), alias);
                    }
                    result
                });

            for path in paths {
                let canonical = path.canonicalize().unwrap_or(path);
                let duration = self.services.media.get_duration(&canonical).ok();
                let item_path = ItemPath::File(CanonicalPath::new(canonical.clone()));
                let mime_type = get_mime_type(&item_path);
                let alias = aliases.get(&canonical).cloned().flatten();
                entries.push(PlaylistItem {
                    path: item_path,
                    duration,
                    alias,
                    mime_type,
                    is_virtual: false,
                });
            }
        }
        entries.sort_by(|a, b| a.path.to_string_lossy().cmp(&b.path.to_string_lossy()));
        self.tui_state.refresh_library(entries);
    }

    pub fn handle_event(&mut self, event: Event) {
        if let Event::Key(key) = event {
            self.tui_state.status_bar.clear();

            let ctx = ComponentContext {
                keymap: &self.runtime.keymap,
                focused_pane: self.tui_state.focused_pane,
            };

            self.tui_state.handle_key(key, &ctx);

            if let Some(action) = self.tui_state.global_handler.take_action() {
                self.execute_action(action);
            }
            if let Some(new_alias) = self.tui_state.rename.take_submitted().flatten() {
                self.handle_rename_submit(new_alias);
            }
            if let Some(url) = self.tui_state.url_input.take_submitted().flatten() {
                self.handle_url_submit(url);
            }
        }
    }

    fn handle_rename_submit(&mut self, new_alias: String) {
        if let Some(item) = self.tui_state.get_selected_item_mut() {
            let old_alias = item.alias.clone();
            item.alias = Some(new_alias.clone());

            if old_alias.as_deref() != Some(&new_alias) {
                let path = item.path.clone();
                let services = self.services.clone();
                let workspace = self.runtime.library_path.clone();
                let is_deletion = false;

                let fallback_alias = self.tokio_runtime.block_on(async {
                    if let Some(file_path) = path.as_file() {
                        let _ = crate::command::execute(
                            &services,
                            crate::command::Command::AliasSet {
                                path: file_path.clone(),
                                workspace,
                                alias: new_alias,
                            },
                        )
                        .await;
                    }
                    None::<String>
                });

                if is_deletion {
                    if let Some(item) = self.tui_state.get_selected_item_mut() {
                        item.alias = fallback_alias;
                    }
                }
            }
        }
    }

    fn handle_url_submit(&mut self, url: String) {
        let item = PlaylistItem {
            path: ItemPath::Url(url),
            duration: None,
            alias: None,
            mime_type: Some("url".to_string()),
            is_virtual: true,
        };
        self.tui_state.library_pane.items.push(item);
        self.tui_state
            .library_pane
            .items
            .sort_by(|a, b| a.path.to_string_lossy().cmp(&b.path.to_string_lossy()));
        self.save_playlist();
        self.tui_state.status_bar.set("URL added to library");
    }

    fn execute_action(&mut self, action: Action) {
        crate::feat::action_handler::dispatch(self, action);
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crossterm::event::{KeyCode, KeyModifiers};

    use super::*;
    use crate::feat::FileLauncherService;
    use crate::feat::keymap::{Action, Keymap};
    use crate::feat::media_query::MediaQueryService;
    use crate::feat::mpv::{MpvClientService, MpvLauncherService};
    use crate::feat::playlist::FakeStorageBackend;
    use crate::feat::playlist::PlaylistStorageService;
    use crate::test_utils::{FakeLauncher, FakeMediaBackend, FakeMpvBackend, FakeMpvLauncher};

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

            let mut app = App {
                services,
                tui_state: TuiState::new(),
                config: Config::default(),
                should_quit: false,
                fork: Fork::default(),
                runtime: RuntimeSettings {
                    keymap: Keymap::new(),
                    socket_path: String::from("/tmp/mpvsocket"),
                    library_path: self.library_path,
                },
                tokio_runtime: rt,
            };

            for path in self.playlist_items {
                let item_path = ItemPath::File(CanonicalPath::new(path.clone()));
                let duration = app.services.media.get_duration(&path).ok();
                let mime_type = get_mime_type(&item_path);
                app.tui_state.playlist_pane.items.push(PlaylistItem {
                    path: item_path,
                    duration,
                    alias: None,
                    mime_type,
                    is_virtual: false,
                });
            }

            for path in self.library_items {
                let item_path = ItemPath::File(CanonicalPath::new(path.clone()));
                let duration = app.services.media.get_duration(&path).ok();
                let mime_type = get_mime_type(&item_path);
                app.tui_state.library_pane.items.push(PlaylistItem {
                    path: item_path,
                    duration,
                    alias: None,
                    mime_type,
                    is_virtual: false,
                });
            }

            app.set_initial_focus();
            if let Some(pane) = self.focused_pane {
                app.tui_state.focused_pane = pane;
            }
            app
        }
    }

    fn execute_actions(app: &mut App, actions: &[Action]) {
        for action in actions {
            app.execute_action(*action);
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
        app.execute_action(Action::Quit);

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
        app.execute_action(Action::Save);

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
        execute_actions(&mut app, &[Action::SwitchPane, Action::SwitchPane]);

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
        app.execute_action(Action::FocusPlaylist);

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
        app.execute_action(Action::FocusLibrary);

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
            &[Action::MoveDown, Action::MoveDown, Action::MoveDown],
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
        execute_actions(&mut app, &[Action::MoveUp, Action::MoveUp, Action::MoveUp]);

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
        execute_actions(&mut app, &[Action::MoveDown, Action::MoveUp]);

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
        app.execute_action(Action::ReorderUp);

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
        app.execute_action(Action::ReorderDown);

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
        app.execute_action(Action::MoveToPlaylist);

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
        app.execute_action(Action::MoveToLibrary);

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
        app.execute_action(Action::LaunchFile);

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
        app.execute_action(Action::LoadPlaylist);

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
        app.execute_action(Action::LoadPlaylist);

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
        app.execute_action(Action::SwitchPane);

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
        app.execute_action(Action::SwitchPane);

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
        app.execute_action(Action::FocusPlaylist);

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
        app.execute_action(Action::FocusLibrary);

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
        execute_actions(&mut app, &[Action::ShowHelp, Action::ShowHelp]);

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
        app.execute_action(Action::StartFilter);

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
        app.execute_action(Action::StartFilter);

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
        app.execute_action(Action::Rename);

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
        app.execute_action(Action::Notes);

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
        app.execute_action(Action::Notes);

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
        app.execute_action(Action::LaunchMpv);

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
        app.execute_action(Action::LaunchMpv);

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
        });
        app.tui_state.focused_pane = Pane::Library;

        // When moving to playlist.
        app.execute_action(Action::MoveToPlaylist);

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
        });
        app.tui_state.focused_pane = Pane::Playlist;

        // When moving to library.
        app.execute_action(Action::MoveToLibrary);

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
        });
        app.tui_state.focused_pane = Pane::Library;

        // When moving library -> playlist -> library.
        execute_actions(&mut app, &[Action::MoveToPlaylist, Action::MoveToLibrary]);

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
        });
        app.tui_state.focused_pane = Pane::Playlist;

        // When moving to library.
        app.execute_action(Action::MoveToLibrary);

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
        });

        // When refreshing the library.
        app.refresh_library();

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
        });

        // When refreshing the library.
        app.refresh_library();

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
        });
        app.tui_state.focused_pane = Pane::Library;

        // When executing Delete action.
        app.execute_action(Action::Delete);

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
        app.execute_action(Action::Delete);

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
