use std::path::PathBuf;

use crossterm::event::{Event, KeyCode};

use crate::config::Config;
use crate::keymap::{Action, Keymap};
use crate::playlist::PlaylistData;
use crate::services::Services;
use crate::tui_state::TuiState;
use crate::ui::{get_mime_type, Pane, PlaylistItem};

pub struct App {
    /// External service dependencies (mpv client, media query, playlist storage, mpv launcher, file launcher).
    pub services: Services,
    /// All UI state for the terminal interface (panes, focus, popups, filters, etc.).
    pub tui_state: TuiState,
    /// User configuration settings (file opener commands, video/audio extensions, etc.).
    pub config: Config,
    /// Flag signaling the main loop should exit.
    pub should_quit: bool,
    /// Path to file for which to open notes; signals the main loop to spawn an editor.
    pub pending_notes_path: Option<PathBuf>,
    /// Flag to trigger fuzzy notes search; signals the main loop to spawn skim.
    pub pending_fuzzy_notes: bool,
    /// Key bindings mapping key combinations to actions.
    pub keymap: Keymap,
    /// Path to mpv's IPC socket for remote control communication.
    pub socket_path: String,
    /// Directory path for scanning library files.
    pub library_path: PathBuf,
}

impl App {
    pub fn new(
        services: Services,
        config: Config,
        socket_path: String,
        library_path: PathBuf,
    ) -> Self {
        let mut app = Self {
            services,
            tui_state: TuiState::new(),
            config,
            should_quit: false,
            pending_notes_path: None,
            pending_fuzzy_notes: false,
            keymap: Keymap::new(),
            socket_path,
            library_path,
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
        match self.services.storage.load() {
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
                        let alias = metadata.and_then(|m| m.alias.clone());
                        let mime_type = metadata
                            .and_then(|m| m.mime_type.clone())
                            .or_else(|| get_mime_type(&path));
                        PlaylistItem {
                            path,
                            duration,
                            alias,
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
                        let mime_type = metadata.mime_type.or_else(|| get_mime_type(&path));
                        PlaylistItem {
                            path,
                            duration: metadata.duration,
                            alias: metadata.alias,
                            mime_type,
                            is_virtual: true,
                        }
                    })
                    .collect();
                virtual_library_items.sort_by(|a, b| a.path.cmp(&b.path));

                self.refresh_library();

                for item in virtual_library_items {
                    self.tui_state.library_pane.items.push(item);
                }
                self.tui_state
                    .library_pane
                    .items
                    .sort_by(|a, b| a.path.cmp(&b.path));
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
                crate::playlist::FileMetadata {
                    duration: item.duration,
                    alias: item.alias.clone(),
                    is_virtual: item.is_virtual,
                    deleted: false,
                    mime_type: item.mime_type.clone(),
                },
            );
        }
        for item in &self.tui_state.library_pane.items {
            files.insert(
                item.path.clone(),
                crate::playlist::FileMetadata {
                    duration: item.duration,
                    alias: item.alias.clone(),
                    is_virtual: item.is_virtual,
                    deleted: false,
                    mime_type: item.mime_type.clone(),
                },
            );
        }
        let playlist_paths: Vec<PathBuf> = self
            .tui_state
            .playlist_pane
            .items
            .iter()
            .map(|item| item.path.clone())
            .collect();
        let data = PlaylistData {
            playlist: playlist_paths,
            files,
        };
        match self.services.storage.save(&data) {
            Ok(()) => {
                self.tui_state.status_message = Some("Playlist saved".to_string());
            }
            Err(e) => {
                self.tui_state
                    .show_error(format!("Failed to save playlist: {e:?}"));
            }
        }
    }

    pub fn refresh_library(&mut self) {
        let mut entries = Vec::new();
        if let Ok(read_dir) = std::fs::read_dir(&self.library_path) {
            for entry in read_dir.flatten() {
                let path = entry.path();
                if path.is_file() {
                    let canonical = path.canonicalize().unwrap_or(path);
                    let duration = self.services.media.get_duration(&canonical).ok();
                    let mime_type = get_mime_type(&canonical);
                    entries.push(PlaylistItem {
                        path: canonical,
                        duration,
                        alias: None,
                        mime_type,
                        is_virtual: false,
                    });
                }
            }
        }
        entries.sort_by(|a, b| a.path.cmp(&b.path));
        self.tui_state.refresh_library(entries);
    }

    #[allow(clippy::too_many_lines)]
    pub fn handle_event(&mut self, event: Event) {
        if let Event::Key(key) = event {
            self.tui_state.status_message = None;
            if self.tui_state.which_key.active && self.tui_state.which_key.pending_prefix.is_none()
            {
                self.tui_state.which_key.dismiss();
                return;
            }

            if self.tui_state.is_showing_error() {
                self.tui_state.dismiss_error();
                return;
            }

            if self.tui_state.is_filtering() {
                match key.code {
                    KeyCode::Esc => {
                        self.tui_state.cancel_filter();
                    }
                    KeyCode::Enter => {
                        self.tui_state.submit_filter();
                    }
                    KeyCode::Backspace => {
                        self.tui_state.pop_filter_char();
                    }
                    KeyCode::Char(c) => {
                        self.tui_state.push_filter_char(c);
                    }
                    _ => {}
                }
                return;
            }

            if self.tui_state.is_renaming() {
                match key.code {
                    KeyCode::Esc => {
                        self.tui_state.cancel_rename();
                    }
                    KeyCode::Enter => {
                        self.tui_state.submit_rename();
                    }
                    KeyCode::Backspace => {
                        self.tui_state.pop_rename_char();
                    }
                    KeyCode::Char(c) => {
                        self.tui_state.push_rename_char(c);
                    }
                    _ => {}
                }
                return;
            }

            if self.tui_state.is_url_input() {
                match key.code {
                    KeyCode::Esc => {
                        self.tui_state.cancel_url_input();
                    }
                    KeyCode::Enter => {
                        self.submit_url_input();
                    }
                    KeyCode::Backspace => {
                        self.tui_state.pop_url_char();
                    }
                    KeyCode::Char(c) => {
                        self.tui_state.push_url_char(c);
                    }
                    _ => {}
                }
                return;
            }

            if let KeyCode::Char(c) = key.code {
                if let Some(prefix) = self.tui_state.pending_key {
                    self.tui_state.pending_key = None;
                    self.tui_state.which_key.dismiss();
                    if let Some(prefix_binding) = self.keymap.get_prefix_binding(prefix) {
                        if let Some(followup) = prefix_binding.followups.iter().find(|f| f.key == c)
                        {
                            self.execute_action(followup.action);
                            return;
                        }
                    }
                } else if self.keymap.get_prefix_binding(c).is_some() {
                    self.tui_state.pending_key = Some(c);
                    self.tui_state.which_key.show_followup(c);
                    return;
                }
            }

            if self.tui_state.pending_key.is_some() {
                self.tui_state.pending_key = None;
                self.tui_state.which_key.dismiss();
            }

            if let Some(action) =
                self.keymap
                    .get_action(key.code, key.modifiers, self.tui_state.focused_pane)
            {
                self.execute_action(action);
            }
        }
    }

    fn execute_action(&mut self, action: Action) {
        match action {
            Action::ShowHelp => {
                self.tui_state.which_key.toggle();
            }
            Action::Quit => {
                self.save_playlist();
                self.should_quit = true;
            }
            Action::Save => {
                self.save_playlist();
            }
            Action::StartFilter => {
                self.tui_state.start_filter();
            }
            Action::MoveUp => match self.tui_state.focused_pane {
                Pane::Playlist => self.tui_state.move_playlist_up(),
                Pane::Library => self.tui_state.move_library_up(),
            },
            Action::MoveDown => match self.tui_state.focused_pane {
                Pane::Playlist => self.tui_state.move_playlist_down(),
                Pane::Library => self.tui_state.move_library_down(),
            },
            Action::SwitchPane => {
                self.tui_state.switch_pane();
            }
            Action::FocusPlaylist => {
                if !self.tui_state.playlist_pane.items.is_empty() {
                    self.tui_state.focused_pane = Pane::Playlist;
                }
            }
            Action::FocusLibrary => {
                if !self.tui_state.library_pane.items.is_empty() {
                    self.tui_state.focused_pane = Pane::Library;
                }
            }
            Action::ToggleItem => match self.tui_state.focused_pane {
                Pane::Library => {
                    self.move_from_library_to_playlist();
                }
                Pane::Playlist => {
                    self.move_from_playlist_to_library();
                }
            },
            Action::Rename => {
                self.tui_state.start_rename();
            }
            Action::Notes => {
                if let Some(item) = self.tui_state.get_selected_item() {
                    self.pending_notes_path = Some(item.path.clone());
                }
            }
            Action::ReorderUp => {
                if !self.tui_state.has_active_filter(Pane::Playlist) {
                    self.tui_state.reorder_playlist_up();
                    self.tui_state.needs_clear = true;
                }
            }
            Action::ReorderDown => {
                if !self.tui_state.has_active_filter(Pane::Playlist) {
                    self.tui_state.reorder_playlist_down();
                    self.tui_state.needs_clear = true;
                }
            }
            Action::LaunchFile => {
                self.launch_file();
            }
            Action::LoadPlaylist => {
                self.load_playlist_in_mpv();
            }
            Action::MoveToLibrary => {
                self.move_from_playlist_to_library();
            }
            Action::MoveToPlaylist => {
                self.move_from_library_to_playlist();
            }
            Action::LaunchMpv => {
                self.launch_mpv();
            }
            Action::AddUrl => {
                self.tui_state.start_url_input();
            }
            Action::Delete => {
                self.delete_library_item();
            }
            Action::FuzzyNotes => {
                self.pending_fuzzy_notes = true;
            }
        }
    }

    fn delete_library_item(&mut self) {
        if let Some(item) = self.tui_state.selected_library_item() {
            if item.is_virtual {
                self.tui_state.library_pane.remove();
                self.save_playlist();
                self.tui_state.status_message = Some("Virtual entry deleted".to_string());
            } else {
                self.tui_state.status_message = Some(
                    "Only virtual entries (URLs) can be deleted. Use 'd' to move to library."
                        .to_string(),
                );
            }
        }
    }

    fn submit_url_input(&mut self) {
        if let Some(url) = self.tui_state.submit_url_input() {
            let path = PathBuf::from(url);
            let item = PlaylistItem {
                path,
                duration: None,
                alias: None,
                mime_type: Some("url".to_string()),
                is_virtual: true,
            };
            self.tui_state.library_pane.items.push(item);
            self.tui_state
                .library_pane
                .items
                .sort_by(|a, b| a.path.cmp(&b.path));
            self.save_playlist();
            self.tui_state.status_message = Some("URL added to library".to_string());
        }
    }

    fn move_from_library_to_playlist(&mut self) {
        if let Some(item) = self.tui_state.selected_library_item().cloned() {
            self.tui_state.add_to_playlist(
                item.path,
                item.duration,
                item.alias,
                item.mime_type,
                item.is_virtual,
            );
            self.tui_state.remove_from_library();
            if self.tui_state.library_pane.items.is_empty() {
                self.tui_state.focused_pane = Pane::Playlist;
            }
            self.tui_state.needs_clear = true;
            self.save_playlist();
        }
    }

    fn move_from_playlist_to_library(&mut self) {
        if let Some(item) = self.tui_state.selected_playlist_item().cloned() {
            let file_missing = !item.path.exists() && !item.is_virtual;
            if !file_missing {
                self.tui_state.library_pane.items.push(item);
                self.tui_state
                    .library_pane
                    .items
                    .sort_by(|a, b| a.path.cmp(&b.path));
            }
            self.tui_state.remove_from_playlist();
            if self.tui_state.playlist_pane.items.is_empty() {
                self.tui_state.focused_pane = Pane::Library;
            }
            self.tui_state.needs_clear = true;
            self.save_playlist();
        }
    }

    fn launch_file(&mut self) {
        if let Some(item) = self.tui_state.selected_playlist_item() {
            let cmd = self.config.get_cmd(&item.path);
            match self
                .services
                .file_launcher
                .launch(&item.path, cmd, &self.socket_path)
            {
                Ok(result) => {
                    if result.used_default_opener {
                        self.tui_state.status_message = Some(format!(
                            "Opening with default opener: {}",
                            item.path.display()
                        ));
                    } else {
                        self.tui_state.status_message =
                            Some(format!("Opening: {}", item.path.display()));
                    }
                }
                Err(e) => {
                    self.tui_state
                        .show_error(format!("Failed to open file: {e:?}"));
                }
            }
        }
    }

    fn load_playlist_in_mpv(&mut self) {
        let paths: Vec<PathBuf> = self
            .tui_state
            .playlist_pane
            .items
            .iter()
            .filter(|item| self.config.is_video_or_audio(&item.path))
            .map(|item| item.path.clone())
            .collect();
        if paths.is_empty() {
            self.tui_state
                .show_error("No video or audio files in playlist".to_string());
            return;
        }
        match self.services.mpv.load_playlist(&paths) {
            Ok(()) => {
                self.tui_state.status_message =
                    Some(format!("Loaded {} items into mpv", paths.len()));
            }
            Err(e) => {
                self.tui_state
                    .show_error(format!("Failed to load playlist in mpv: {e:?}"));
            }
        }
    }

    fn launch_mpv(&mut self) {
        if self.services.mpv_launcher.is_running(&self.socket_path) {
            self.tui_state.status_message = Some("MPV already running".to_string());
        } else {
            match self.services.mpv_launcher.spawn(&self.socket_path) {
                Ok(()) => {
                    self.tui_state.status_message = Some("MPV launched".to_string());
                }
                Err(e) => {
                    self.tui_state
                        .show_error(format!("Failed to launch mpv: {e:?}"));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{path::Path, sync::Arc, time::Duration};

    use crossterm::event::KeyModifiers;
    use error_stack::Report;

    use super::*;
    use crate::keymap::{Action, Keymap};
    use crate::launcher::{LaunchResult, Launcher, LauncherService};
    use crate::media::{MediaError, MediaQuery, MediaQueryBackend};
    use crate::mpv::{MpvBackend, MpvClient, MpvError, MpvLauncher, MpvLauncherService};
    use crate::playlist::{IoError, PlaylistData, PlaylistStorage, PlaylistStorageBackend};

    struct FakeMpvBackend;

    impl MpvBackend for FakeMpvBackend {
        fn name(&self) -> &'static str {
            "fake"
        }

        fn load_file(&self, _path: &Path) -> Result<(), Report<MpvError>> {
            Ok(())
        }

        fn load_playlist(&self, _paths: &[PathBuf]) -> Result<(), Report<MpvError>> {
            Ok(())
        }
    }

    struct FakeMpvLauncher {
        running: bool,
    }

    impl FakeMpvLauncher {
        fn new() -> Self {
            Self { running: false }
        }

        fn running(mut self, value: bool) -> Self {
            self.running = value;
            self
        }
    }

    impl MpvLauncher for FakeMpvLauncher {
        fn name(&self) -> &'static str {
            "fake"
        }

        fn is_running(&self, _socket_path: &str) -> bool {
            self.running
        }

        fn spawn(&self, _socket_path: &str) -> Result<(), Report<MpvError>> {
            Ok(())
        }
    }

    struct FakeMediaBackend;

    impl MediaQueryBackend for FakeMediaBackend {
        fn name(&self) -> &'static str {
            "fake"
        }

        fn get_duration(&self, _path: &Path) -> Result<Duration, Report<MediaError>> {
            Ok(Duration::from_secs(120))
        }
    }

    struct FakeStorageBackend;

    impl PlaylistStorageBackend for FakeStorageBackend {
        fn name(&self) -> &'static str {
            "fake"
        }

        fn load(&self) -> Result<PlaylistData, Report<IoError>> {
            Ok(PlaylistData::default())
        }

        fn save(&self, _data: &PlaylistData) -> Result<(), Report<IoError>> {
            Ok(())
        }
    }

    struct FakeLauncher;

    impl Launcher for FakeLauncher {
        fn name(&self) -> &'static str {
            "fake"
        }

        fn launch(
            &self,
            _path: &Path,
            _command: Option<&str>,
            _socket_path: &str,
        ) -> Result<LaunchResult, Report<crate::launcher::LaunchError>> {
            Ok(LaunchResult {
                used_default_opener: false,
            })
        }
    }

    struct TestAppBuilder {
        playlist_items: Vec<PathBuf>,
        library_items: Vec<PathBuf>,
        library_path: PathBuf,
        mpv_launcher: FakeMpvLauncher,
        mpv_backend: FakeMpvBackend,
        media_backend: FakeMediaBackend,
        storage_backend: FakeStorageBackend,
        file_launcher: FakeLauncher,
    }

    impl TestAppBuilder {
        fn new() -> Self {
            Self {
                playlist_items: vec![],
                library_items: vec![],
                library_path: PathBuf::from("."),
                mpv_launcher: FakeMpvLauncher::new(),
                mpv_backend: FakeMpvBackend,
                media_backend: FakeMediaBackend,
                storage_backend: FakeStorageBackend,
                file_launcher: FakeLauncher,
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
            self.library_path = path;
            self
        }

        fn mpv_launcher(mut self, launcher: FakeMpvLauncher) -> Self {
            self.mpv_launcher = launcher;
            self
        }

        fn mpv_backend(mut self, backend: FakeMpvBackend) -> Self {
            self.mpv_backend = backend;
            self
        }

        fn media_backend(mut self, backend: FakeMediaBackend) -> Self {
            self.media_backend = backend;
            self
        }

        fn storage_backend(mut self, backend: FakeStorageBackend) -> Self {
            self.storage_backend = backend;
            self
        }

        fn build(self) -> App {
            let services = Services {
                mpv: MpvClient::new(Arc::new(self.mpv_backend)),
                media: MediaQuery::new(Arc::new(self.media_backend)),
                storage: PlaylistStorage::new(Arc::new(self.storage_backend)),
                mpv_launcher: MpvLauncherService::new(Arc::new(self.mpv_launcher)),
                file_launcher: LauncherService::new(Arc::new(self.file_launcher)),
                notes: None,
            };

            let mut app = App {
                services,
                tui_state: TuiState::new(),
                config: Config::default(),
                should_quit: false,
                pending_notes_path: None,
                pending_fuzzy_notes: false,
                keymap: Keymap::new(),
                socket_path: String::from("/tmp/mpvsocket"),
                library_path: self.library_path,
            };

            for path in self.playlist_items {
                let duration = app.services.media.get_duration(&path).ok();
                let mime_type = get_mime_type(&path);
                app.tui_state.playlist_pane.items.push(PlaylistItem {
                    path,
                    duration,
                    alias: None,
                    mime_type,
                    is_virtual: false,
                });
            }

            for path in self.library_items {
                let duration = app.services.media.get_duration(&path).ok();
                let mime_type = get_mime_type(&path);
                app.tui_state.library_pane.items.push(PlaylistItem {
                    path,
                    duration,
                    alias: None,
                    mime_type,
                    is_virtual: false,
                });
            }

            app.set_initial_focus();
            app
        }
    }

    #[test]
    fn quit_action_saves_and_exits() {
        // Given an empty app.
        let mut app = TestAppBuilder::new().build();

        // When executing Quit action.
        app.execute_action(Action::Quit);

        // Then the app should quit and show saved message.
        assert!(app.should_quit);
        assert!(app.tui_state.status_message.unwrap().contains("saved"));
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
        assert!(app.tui_state.status_message.unwrap().contains("saved"));
    }

    #[test]
    fn switch_pane_toggles_between_panes() {
        // Given an app focused on the playlist pane with items in both panes.
        let mut app = TestAppBuilder::new()
            .playlist_items(vec![PathBuf::from("playlist.mp4")])
            .library_items(vec![PathBuf::from("library.mp4")])
            .build();
        assert_eq!(app.tui_state.focused_pane, Pane::Playlist);

        // When executing SwitchPane action.
        app.execute_action(Action::SwitchPane);

        // Then focus switches to library pane.
        assert_eq!(app.tui_state.focused_pane, Pane::Library);

        // When executing SwitchPane action again.
        app.execute_action(Action::SwitchPane);

        // Then focus switches back to playlist pane.
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

        // When executing MoveDown action multiple times.
        assert_eq!(app.tui_state.playlist_pane.selected, 0);
        app.execute_action(Action::MoveDown);
        assert_eq!(app.tui_state.playlist_pane.selected, 1);
        app.execute_action(Action::MoveDown);
        assert_eq!(app.tui_state.playlist_pane.selected, 2);
        app.execute_action(Action::MoveDown);

        // Then selection stays at the last item.
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

        // When executing MoveUp action multiple times.
        app.execute_action(Action::MoveUp);
        assert_eq!(app.tui_state.playlist_pane.selected, 1);
        app.execute_action(Action::MoveUp);
        assert_eq!(app.tui_state.playlist_pane.selected, 0);
        app.execute_action(Action::MoveUp);

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
            .build();
        app.tui_state.focused_pane = Pane::Library;

        // When navigating with MoveDown/MoveUp.
        assert_eq!(app.tui_state.library_pane.selected, 0);
        app.execute_action(Action::MoveDown);
        assert_eq!(app.tui_state.library_pane.selected, 1);
        app.execute_action(Action::MoveUp);

        // Then selection moves correctly.
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
            PathBuf::from("b.mp4")
        );
        assert_eq!(
            app.tui_state.playlist_pane.items[1].path,
            PathBuf::from("a.mp4")
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
            PathBuf::from("a.mp4")
        );
        assert_eq!(
            app.tui_state.playlist_pane.items[1].path,
            PathBuf::from("b.mp4")
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
            PathBuf::from("test.mp4")
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
        assert_eq!(app.tui_state.library_pane.items[0].path, temp_path);
    }

    #[test]
    fn toggle_item_moves_item_from_library_to_playlist() {
        // Given a library with one item and empty playlist.
        let mut app = TestAppBuilder::new()
            .library_items(vec![PathBuf::from("test.mp4")])
            .build();
        app.tui_state.focused_pane = Pane::Library;

        // When executing ToggleItem action.
        app.execute_action(Action::ToggleItem);

        // Then the item moves to the playlist.
        assert_eq!(app.tui_state.playlist_pane.items.len(), 1);
        assert_eq!(
            app.tui_state.playlist_pane.items[0].path,
            PathBuf::from("test.mp4")
        );
        assert!(app.tui_state.library_pane.items.is_empty());
    }

    #[test]
    fn toggle_item_moves_item_from_playlist_to_library() {
        // Given a playlist with one item and empty library.
        let temp_file = tempfile::NamedTempFile::new().unwrap();
        let temp_path = temp_file.path().to_path_buf();
        let mut app = TestAppBuilder::new()
            .playlist_items(vec![temp_path.clone()])
            .build();
        app.tui_state.focused_pane = Pane::Playlist;

        // When executing ToggleItem action.
        app.execute_action(Action::ToggleItem);

        // Then the item moves to the library.
        assert!(app.tui_state.playlist_pane.items.is_empty());
        assert_eq!(app.tui_state.library_pane.items.len(), 1);
        assert_eq!(app.tui_state.library_pane.items[0].path, temp_path);
    }

    #[test]
    fn toggle_item_removes_missing_file_from_playlist() {
        // Given a playlist with a missing file.
        let mut app = TestAppBuilder::new()
            .playlist_items(vec![PathBuf::from("/nonexistent/file.mp4")])
            .build();
        app.tui_state.focused_pane = Pane::Playlist;

        // When executing ToggleItem action.
        app.execute_action(Action::ToggleItem);

        // Then the item is removed completely (not added to library).
        assert!(app.tui_state.playlist_pane.items.is_empty());
        assert!(app.tui_state.library_pane.items.is_empty());
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
        assert!(app.tui_state.status_message.unwrap().contains("Opening"));
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
        assert!(app
            .tui_state
            .status_message
            .unwrap()
            .contains("Loaded 2 items"));
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
        assert_eq!(app.tui_state.focused_pane, Pane::Playlist);

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
        assert_eq!(app.tui_state.focused_pane, Pane::Library);

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
    fn move_last_library_item_switches_focus_to_playlist() {
        // Given library with 1 item and playlist with items, focused on library.
        let mut app = TestAppBuilder::new()
            .playlist_items(vec![PathBuf::from("playlist.mp4")])
            .library_items(vec![PathBuf::from("library.mp4")])
            .build();
        app.tui_state.focused_pane = Pane::Library;

        // When executing ToggleItem action.
        app.execute_action(Action::ToggleItem);

        // Then focus switches to playlist.
        assert!(app.tui_state.library_pane.items.is_empty());
        assert_eq!(app.tui_state.focused_pane, Pane::Playlist);
    }

    #[test]
    fn move_last_playlist_item_switches_focus_to_library() {
        // Given playlist with 1 item and library with items, focused on playlist.
        let mut app = TestAppBuilder::new()
            .playlist_items(vec![PathBuf::from("playlist.mp4")])
            .library_items(vec![PathBuf::from("library.mp4")])
            .build();
        app.tui_state.focused_pane = Pane::Playlist;

        // When executing ToggleItem action.
        app.execute_action(Action::ToggleItem);

        // Then focus switches to library.
        assert!(app.tui_state.playlist_pane.items.is_empty());
        assert_eq!(app.tui_state.focused_pane, Pane::Library);
    }

    #[test]
    fn move_item_keeps_focus_when_pane_not_empty() {
        // Given playlist with 2 items and library with items, focused on playlist.
        let mut app = TestAppBuilder::new()
            .playlist_items(vec![PathBuf::from("a.mp4"), PathBuf::from("b.mp4")])
            .library_items(vec![PathBuf::from("c.mp4")])
            .build();
        app.tui_state.focused_pane = Pane::Playlist;

        // When executing ToggleItem action.
        app.execute_action(Action::ToggleItem);

        // Then focus stays on playlist.
        assert_eq!(app.tui_state.playlist_pane.items.len(), 1);
        assert_eq!(app.tui_state.focused_pane, Pane::Playlist);
    }

    #[test]
    fn move_to_empty_library_switches_focus() {
        // Given playlist with 1 item and empty library, focused on playlist.
        let mut app = TestAppBuilder::new()
            .playlist_items(vec![PathBuf::from("test.mp4")])
            .build();
        app.tui_state.focused_pane = Pane::Playlist;

        // When executing ToggleItem action.
        app.execute_action(Action::ToggleItem);

        // Then focus switches to library.
        assert_eq!(app.tui_state.focused_pane, Pane::Library);
    }

    #[test]
    fn move_to_empty_playlist_switches_focus() {
        // Given library with 1 item and empty playlist, focused on library.
        let mut app = TestAppBuilder::new()
            .library_items(vec![PathBuf::from("test.mp4")])
            .build();
        app.tui_state.focused_pane = Pane::Library;

        // When executing ToggleItem action.
        app.execute_action(Action::ToggleItem);

        // Then focus switches to playlist.
        assert_eq!(app.tui_state.focused_pane, Pane::Playlist);
    }

    #[test]
    fn show_help_toggles_which_key() {
        // Given an app with which-key inactive.
        let mut app = TestAppBuilder::new().build();
        assert!(!app.tui_state.which_key.active);

        // When executing ShowHelp action.
        app.execute_action(Action::ShowHelp);

        // Then which-key becomes active.
        assert!(app.tui_state.which_key.active);

        // When executing ShowHelp action again.
        app.execute_action(Action::ShowHelp);

        // Then which-key becomes inactive.
        assert!(!app.tui_state.which_key.active);
    }

    #[test]
    fn start_filter_activates_on_playlist() {
        // Given an app focused on playlist with items, not filtering.
        let mut app = TestAppBuilder::new()
            .playlist_items(vec![PathBuf::from("test.mp4")])
            .build();
        assert!(!app.tui_state.is_filtering());

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
            .build();
        app.tui_state.focused_pane = Pane::Library;
        assert!(!app.tui_state.is_filtering());

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
        assert!(!app.tui_state.is_renaming());

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
        assert!(app.pending_notes_path.is_none());

        // When executing Notes action.
        app.execute_action(Action::Notes);

        // Then pending notes path is set to the selected item's path.
        assert_eq!(
            app.pending_notes_path,
            Some(PathBuf::from("/path/to/video.mp4"))
        );
    }

    #[test]
    fn notes_does_nothing_when_no_selection() {
        // Given an app with no items selected and no pending notes path.
        let mut app = TestAppBuilder::new().build();
        assert!(app.pending_notes_path.is_none());

        // When executing Notes action.
        app.execute_action(Action::Notes);

        // Then pending notes path remains unset.
        assert!(app.pending_notes_path.is_none());
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
        assert!(app
            .tui_state
            .status_message
            .unwrap()
            .contains("MPV launched"));
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
        assert!(app
            .tui_state
            .status_message
            .unwrap()
            .contains("MPV already running"));
    }

    #[test]
    fn g_key_sets_pending_and_shows_followup() {
        // Given an app.
        let mut app = TestAppBuilder::new().build();
        assert!(app.tui_state.pending_key.is_none());
        assert!(!app.tui_state.which_key.active);

        // When pressing 'g' key.
        app.handle_event(Event::Key(crossterm::event::KeyEvent::new(
            KeyCode::Char('g'),
            KeyModifiers::empty(),
        )));

        // Then pending_key is set and which_key shows followup.
        assert_eq!(app.tui_state.pending_key, Some('g'));
        assert!(app.tui_state.which_key.active);
        assert_eq!(app.tui_state.which_key.pending_prefix, Some('g'));
    }

    #[test]
    fn gm_keys_launches_mpv() {
        // Given an app with mpv not running.
        let mut app = TestAppBuilder::new()
            .mpv_launcher(FakeMpvLauncher::new().running(false))
            .build();

        // When pressing 'g' then 'm'.
        app.handle_event(Event::Key(crossterm::event::KeyEvent::new(
            KeyCode::Char('g'),
            KeyModifiers::empty(),
        )));
        app.handle_event(Event::Key(crossterm::event::KeyEvent::new(
            KeyCode::Char('m'),
            KeyModifiers::empty(),
        )));

        // Then mpv is launched and popup is dismissed.
        assert!(app
            .tui_state
            .status_message
            .unwrap()
            .contains("MPV launched"));
        assert!(app.tui_state.pending_key.is_none());
        assert!(!app.tui_state.which_key.active);
    }

    #[test]
    fn g_then_invalid_key_dismisses_popup() {
        // Given an app with 'g' pending.
        let mut app = TestAppBuilder::new().build();
        app.handle_event(Event::Key(crossterm::event::KeyEvent::new(
            KeyCode::Char('g'),
            KeyModifiers::empty(),
        )));
        assert_eq!(app.tui_state.pending_key, Some('g'));

        // When pressing a non-followup key.
        app.handle_event(Event::Key(crossterm::event::KeyEvent::new(
            KeyCode::Char('x'),
            KeyModifiers::empty(),
        )));

        // Then popup is dismissed without action.
        assert!(app.tui_state.pending_key.is_none());
        assert!(!app.tui_state.which_key.active);
    }

    #[test]
    fn a_key_sets_pending_and_shows_followup() {
        // Given an app.
        let mut app = TestAppBuilder::new().build();
        assert!(app.tui_state.pending_key.is_none());
        assert!(!app.tui_state.which_key.active);

        // When pressing 'a' key.
        app.handle_event(Event::Key(crossterm::event::KeyEvent::new(
            KeyCode::Char('a'),
            KeyModifiers::empty(),
        )));

        // Then pending_key is set and which_key shows followup.
        assert_eq!(app.tui_state.pending_key, Some('a'));
        assert!(app.tui_state.which_key.active);
        assert_eq!(app.tui_state.which_key.pending_prefix, Some('a'));
    }

    #[test]
    fn au_keys_starts_url_input() {
        // Given an app.
        let mut app = TestAppBuilder::new().build();
        assert!(!app.tui_state.is_url_input());

        // When pressing 'a' then 'u'.
        app.handle_event(Event::Key(crossterm::event::KeyEvent::new(
            KeyCode::Char('a'),
            KeyModifiers::empty(),
        )));
        app.handle_event(Event::Key(crossterm::event::KeyEvent::new(
            KeyCode::Char('u'),
            KeyModifiers::empty(),
        )));

        // Then url input is active and popup is dismissed.
        assert!(app.tui_state.is_url_input());
        assert!(app.tui_state.pending_key.is_none());
        assert!(!app.tui_state.which_key.active);
    }

    #[test]
    fn a_then_invalid_key_dismisses_popup() {
        // Given an app with 'a' pending.
        let mut app = TestAppBuilder::new().build();
        app.handle_event(Event::Key(crossterm::event::KeyEvent::new(
            KeyCode::Char('a'),
            KeyModifiers::empty(),
        )));
        assert_eq!(app.tui_state.pending_key, Some('a'));

        // When pressing a non-followup key.
        app.handle_event(Event::Key(crossterm::event::KeyEvent::new(
            KeyCode::Char('x'),
            KeyModifiers::empty(),
        )));

        // Then popup is dismissed without action.
        assert!(app.tui_state.pending_key.is_none());
        assert!(!app.tui_state.which_key.active);
        assert!(!app.tui_state.is_url_input());
    }

    #[test]
    fn virtual_item_preserved_when_moved_from_library_to_playlist() {
        // Given a library with a virtual URL item.
        let mut app = TestAppBuilder::new().build();
        let url = PathBuf::from("https://example.com/video.mp4");
        app.tui_state.library_pane.items.push(PlaylistItem {
            path: url.clone(),
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
        assert_eq!(app.tui_state.playlist_pane.items[0].path, url);
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
        let url = PathBuf::from("https://example.com/video.mp4");
        app.tui_state.playlist_pane.items.push(PlaylistItem {
            path: url.clone(),
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
        assert_eq!(app.tui_state.library_pane.items[0].path, url);
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
        let url = PathBuf::from("https://youtube.com/watch?v=abc123");
        app.tui_state.library_pane.items.push(PlaylistItem {
            path: url.clone(),
            duration: Some(std::time::Duration::from_secs(300)),
            alias: Some("My Video".to_string()),
            mime_type: Some("url".to_string()),
            is_virtual: true,
        });
        app.tui_state.focused_pane = Pane::Library;

        // When moving library -> playlist -> library.
        app.execute_action(Action::MoveToPlaylist);
        app.tui_state.focused_pane = Pane::Playlist;
        app.execute_action(Action::MoveToLibrary);

        // Then all properties are preserved.
        assert_eq!(app.tui_state.library_pane.items.len(), 1);
        let item = &app.tui_state.library_pane.items[0];
        assert_eq!(item.path, url);
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
            path: PathBuf::from("/nonexistent/file.mp4"),
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
        let url = PathBuf::from("https://example.com/video.mp4");
        app.tui_state.library_pane.items.push(PlaylistItem {
            path: url.clone(),
            duration: None,
            alias: None,
            mime_type: Some("url".to_string()),
            is_virtual: true,
        });

        // When refreshing the library.
        app.refresh_library();

        // Then both the virtual item and real file are present.
        assert_eq!(app.tui_state.library_pane.items.len(), 2);
        assert!(app
            .tui_state
            .library_pane
            .items
            .iter()
            .any(|i| i.path == url && i.is_virtual));
        assert!(app
            .tui_state
            .library_pane
            .items
            .iter()
            .any(|i| i.path.file_name().unwrap() == "real.mp4" && !i.is_virtual));
    }

    #[test]
    fn refresh_library_removes_missing_non_virtual_items() {
        // Given a temp directory with no files and an app with a non-virtual item.
        let tree = temptree::temptree! {};

        let mut app = TestAppBuilder::new()
            .library_path(tree.path().to_path_buf())
            .build();
        app.tui_state.library_pane.items.push(PlaylistItem {
            path: PathBuf::from("/nonexistent/file.mp4"),
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
        let url = PathBuf::from("https://example.com/video.mp4");
        app.tui_state.library_pane.items.push(PlaylistItem {
            path: url.clone(),
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
        assert!(app
            .tui_state
            .status_message
            .clone()
            .unwrap()
            .contains("virtual"));
    }

    #[test]
    fn toggle_item_preserves_virtual_when_moving_to_playlist() {
        // Given a library with a virtual item.
        let mut app = TestAppBuilder::new().build();
        let url = PathBuf::from("https://youtube.com/watch?v=test");
        app.tui_state.library_pane.items.push(PlaylistItem {
            path: url.clone(),
            duration: Some(std::time::Duration::from_secs(180)),
            alias: Some("Test Video".to_string()),
            mime_type: Some("url".to_string()),
            is_virtual: true,
        });
        app.tui_state.focused_pane = Pane::Library;

        // When toggling (moving to playlist).
        app.execute_action(Action::ToggleItem);

        // Then the virtual item is in the playlist with all properties.
        assert!(app.tui_state.library_pane.items.is_empty());
        assert_eq!(app.tui_state.playlist_pane.items.len(), 1);
        let item = &app.tui_state.playlist_pane.items[0];
        assert_eq!(item.path, url);
        assert!(item.is_virtual);
        assert_eq!(item.alias, Some("Test Video".to_string()));
    }
}
