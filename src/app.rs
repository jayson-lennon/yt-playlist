use std::path::PathBuf;

use crossterm::event::{Event, KeyCode};

use crate::keymap::{Action, Keymap};
use crate::playlist::PlaylistData;
use crate::services::Services;
use crate::tui_state::TuiState;
use crate::ui::{Pane, PlaylistItem};

pub const DEFAULT_EXTENSIONS: &[&str] = &[
    "mp4", "mkv", "avi", "webm", "mov", "flv", "wmv", "mp3", "flac", "wav", "ogg", "m4a", "aac",
];

pub struct App {
    pub services: Services,
    pub tui_state: TuiState,
    pub extensions: Vec<String>,
    pub should_quit: bool,
    pub pending_notes_path: Option<PathBuf>,
    pub keymap: Keymap,
    pub socket_path: String,
}

impl App {
    pub fn new(services: Services, extensions: Vec<String>, socket_path: String) -> Self {
        let mut app = Self {
            services,
            tui_state: TuiState::new(),
            extensions,
            should_quit: false,
            pending_notes_path: None,
            keymap: Keymap::new(),
            socket_path,
        };
        app.load_playlist();
        app.refresh_directory();
        app.set_initial_focus();
        app
    }

    fn set_initial_focus(&mut self) {
        let playlist_empty = self.tui_state.playlist_pane.items.is_empty();
        let directory_empty = self.tui_state.directory_pane.items.is_empty();
        if playlist_empty && !directory_empty {
            self.tui_state.focused_pane = Pane::Directory;
        } else if directory_empty && !playlist_empty {
            self.tui_state.focused_pane = Pane::Playlist;
        }
    }

    pub fn load_playlist(&mut self) {
        match self.services.storage.load() {
            Ok(data) => {
                self.tui_state.playlist_pane.items = data
                    .playlist
                    .into_iter()
                    .map(|path| {
                        let metadata = data.files.get(&path);
                        let duration = metadata.and_then(|m| m.duration);
                        let alias = metadata.and_then(|m| m.alias.clone());
                        PlaylistItem {
                            path,
                            duration,
                            alias,
                        }
                    })
                    .collect();
                self.refresh_directory();
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
                },
            );
        }
        for item in &self.tui_state.directory_pane.items {
            files.insert(
                item.path.clone(),
                crate::playlist::FileMetadata {
                    duration: item.duration,
                    alias: item.alias.clone(),
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

    pub fn refresh_directory(&mut self) {
        let mut entries = Vec::new();
        if let Ok(read_dir) = std::fs::read_dir(".") {
            for entry in read_dir.flatten() {
                let path = entry.path();
                if path.is_file() {
                    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                        if self.extensions.contains(&ext.to_lowercase()) {
                            let canonical = path.canonicalize().unwrap_or(path);
                            let duration = self.services.media.get_duration(&canonical).ok();
                            entries.push(PlaylistItem {
                                path: canonical,
                                duration,
                                alias: None,
                            });
                        }
                    }
                }
            }
        }
        entries.sort_by(|a, b| a.path.cmp(&b.path));
        self.tui_state.refresh_directory(entries);
    }

    #[allow(clippy::too_many_lines)]
    pub fn handle_event(&mut self, event: Event) {
        if let Event::Key(key) = event {
            self.tui_state.status_message = None;
            if self.tui_state.which_key.active {
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

            if let KeyCode::Char(c) = key.code {
                if self.tui_state.pending_key == Some('g') {
                    self.tui_state.pending_key = None;
                    if c == 'm' {
                        self.execute_action(Action::LaunchMpv);
                        return;
                    }
                } else if c == 'g' {
                    self.tui_state.pending_key = Some('g');
                    return;
                }
            }

            self.tui_state.pending_key = None;

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
                Pane::Directory => self.tui_state.move_directory_up(),
            },
            Action::MoveDown => match self.tui_state.focused_pane {
                Pane::Playlist => self.tui_state.move_playlist_down(),
                Pane::Directory => self.tui_state.move_directory_down(),
            },
            Action::SwitchPane => {
                self.tui_state.switch_pane();
            }
            Action::FocusPlaylist => {
                if !self.tui_state.playlist_pane.items.is_empty() {
                    self.tui_state.focused_pane = Pane::Playlist;
                }
            }
            Action::FocusDirectory => {
                if !self.tui_state.directory_pane.items.is_empty() {
                    self.tui_state.focused_pane = Pane::Directory;
                }
            }
            Action::ToggleItem => match self.tui_state.focused_pane {
                Pane::Directory => {
                    self.move_from_directory_to_playlist();
                }
                Pane::Playlist => {
                    self.move_from_playlist_to_directory();
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
                }
            }
            Action::ReorderDown => {
                if !self.tui_state.has_active_filter(Pane::Playlist) {
                    self.tui_state.reorder_playlist_down();
                }
            }
            Action::PlayInMpv => {
                self.open_in_mpv();
            }
            Action::MoveToDirectory => {
                self.move_from_playlist_to_directory();
            }
            Action::MoveToPlaylist => {
                self.move_from_directory_to_playlist();
            }
            Action::LaunchMpv => {
                self.launch_mpv();
            }
        }
    }

    fn move_from_directory_to_playlist(&mut self) {
        if let Some(item) = self.tui_state.selected_directory_item().cloned() {
            self.tui_state
                .add_to_playlist(item.path, item.duration, item.alias);
            self.tui_state.remove_from_directory();
            if self.tui_state.directory_pane.items.is_empty() {
                self.tui_state.focused_pane = Pane::Playlist;
            }
            self.tui_state.needs_clear = true;
        }
    }

    fn move_from_playlist_to_directory(&mut self) {
        if let Some(item) = self.tui_state.selected_playlist_item().cloned() {
            self.tui_state.directory_pane.items.push(item);
            self.tui_state
                .directory_pane
                .items
                .sort_by(|a, b| a.path.cmp(&b.path));
            self.tui_state.remove_from_playlist();
            if self.tui_state.playlist_pane.items.is_empty() {
                self.tui_state.focused_pane = Pane::Directory;
            }
            self.tui_state.needs_clear = true;
        }
    }

    fn open_in_mpv(&mut self) {
        if let Some(item) = self.tui_state.selected_playlist_item() {
            match self.services.mpv.load_file(&item.path) {
                Ok(()) => {
                    self.tui_state.status_message =
                        Some(format!("Playing: {}", item.path.display()));
                }
                Err(e) => {
                    self.tui_state
                        .show_error(format!("Failed to open in mpv: {e:?}"));
                }
            }
        }
    }

    fn launch_mpv(&mut self) {
        if crate::mpv::is_mpv_running_with_socket(&self.socket_path) {
            self.tui_state.status_message = Some("MPV already running".to_string());
        } else {
            match crate::mpv::spawn_mpv(&self.socket_path) {
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
    use std::{
        path::Path,
        sync::{Arc, Mutex},
        time::Duration,
    };

    use error_stack::Report;

    use super::*;
    use crate::keymap::{Action, Keymap};
    use crate::media::{MediaError, MediaQuery, MediaQueryBackend};
    use crate::mpv::{MpvBackend, MpvClient, MpvError};
    use crate::playlist::{IoError, PlaylistStorage, PlaylistStorageBackend};

    struct MockMpvBackend {
        load_file_called: Arc<Mutex<Vec<PathBuf>>>,
    }

    impl MockMpvBackend {
        fn new() -> Self {
            Self {
                load_file_called: Arc::new(Mutex::new(Vec::new())),
            }
        }
    }

    impl MpvBackend for MockMpvBackend {
        fn name(&self) -> &'static str {
            "mock"
        }

        fn load_file(&self, path: &Path) -> Result<(), Report<MpvError>> {
            self.load_file_called
                .lock()
                .unwrap()
                .push(path.to_path_buf());
            Ok(())
        }
    }

    struct MockMediaBackend;

    impl MediaQueryBackend for MockMediaBackend {
        fn name(&self) -> &'static str {
            "mock"
        }

        fn get_duration(&self, _path: &Path) -> Result<Duration, Report<MediaError>> {
            Ok(Duration::from_secs(120))
        }
    }

    struct MockStorageBackend {
        saved_data: Arc<Mutex<Option<PlaylistData>>>,
    }

    impl MockStorageBackend {
        fn new() -> Self {
            Self {
                saved_data: Arc::new(Mutex::new(None)),
            }
        }
    }

    impl PlaylistStorageBackend for MockStorageBackend {
        fn name(&self) -> &'static str {
            "mock"
        }

        fn load(&self) -> Result<PlaylistData, Report<IoError>> {
            Ok(PlaylistData::default())
        }

        fn save(&self, data: &PlaylistData) -> Result<(), Report<IoError>> {
            *self.saved_data.lock().unwrap() = Some(data.clone());
            Ok(())
        }
    }

    fn create_test_app(
        playlist_items: Vec<PathBuf>,
        directory_items: Vec<PathBuf>,
    ) -> (App, Arc<MockMpvBackend>, Arc<MockStorageBackend>) {
        let mpv_backend = Arc::new(MockMpvBackend::new());
        let storage_backend = Arc::new(MockStorageBackend::new());
        let media_backend = Arc::new(MockMediaBackend);

        let services = Services {
            mpv: MpvClient::new(mpv_backend.clone()),
            media: MediaQuery::new(media_backend),
            storage: PlaylistStorage::new(storage_backend.clone()),
        };

        let mut app = App {
            services,
            tui_state: TuiState::new(),
            extensions: DEFAULT_EXTENSIONS.iter().map(|s| s.to_string()).collect(),
            should_quit: false,
            pending_notes_path: None,
            keymap: Keymap::new(),
            socket_path: String::from("/tmp/mpvsocket"),
        };

        for path in playlist_items {
            let duration = app.services.media.get_duration(&path).ok();
            app.tui_state.playlist_pane.items.push(PlaylistItem {
                path,
                duration,
                alias: None,
            });
        }

        for path in directory_items {
            let duration = app.services.media.get_duration(&path).ok();
            app.tui_state.directory_pane.items.push(PlaylistItem {
                path,
                duration,
                alias: None,
            });
        }

        app.set_initial_focus();

        (app, mpv_backend, storage_backend)
    }

    #[test]
    fn quit_action_saves_and_exits() {
        // Given an app with an empty playlist.
        let (mut app, _, storage) = create_test_app(vec![], vec![]);

        // When executing Quit action.
        app.execute_action(Action::Quit);

        // Then the app should quit and save the playlist.
        assert!(app.should_quit);
        let saved = storage.saved_data.lock().unwrap();
        assert!(saved.as_ref().map_or(true, |d| d.playlist.is_empty()));
    }

    #[test]
    fn save_action_persists_playlist() {
        // Given an app with one item in the playlist.
        let (mut app, _, storage) = create_test_app(vec![PathBuf::from("test.mp4")], vec![]);

        // When executing Save action.
        app.execute_action(Action::Save);

        // Then the playlist is saved.
        let saved = storage.saved_data.lock().unwrap();
        let data = saved.as_ref().expect("should have saved data");
        assert_eq!(data.playlist.len(), 1);
        assert_eq!(data.playlist[0], PathBuf::from("test.mp4"));
    }

    #[test]
    fn switch_pane_toggles_between_panes() {
        // Given an app focused on the playlist pane with items in both panes.
        let (mut app, _, _) = create_test_app(
            vec![PathBuf::from("playlist.mp4")],
            vec![PathBuf::from("directory.mp4")],
        );
        assert_eq!(app.tui_state.focused_pane, Pane::Playlist);

        // When executing SwitchPane action.
        app.execute_action(Action::SwitchPane);

        // Then focus switches to directory pane.
        assert_eq!(app.tui_state.focused_pane, Pane::Directory);

        // When executing SwitchPane action again.
        app.execute_action(Action::SwitchPane);

        // Then focus switches back to playlist pane.
        assert_eq!(app.tui_state.focused_pane, Pane::Playlist);
    }

    #[test]
    fn focus_playlist_switches_to_playlist_pane() {
        // Given an app focused on the directory pane with items in both panes.
        let (mut app, _, _) = create_test_app(
            vec![PathBuf::from("playlist.mp4")],
            vec![PathBuf::from("directory.mp4")],
        );
        app.tui_state.focused_pane = Pane::Directory;

        // When executing FocusPlaylist action.
        app.execute_action(Action::FocusPlaylist);

        // Then focus switches to playlist pane.
        assert_eq!(app.tui_state.focused_pane, Pane::Playlist);
    }

    #[test]
    fn focus_directory_switches_to_directory_pane() {
        // Given an app focused on the playlist pane with items in both panes.
        let (mut app, _, _) = create_test_app(
            vec![PathBuf::from("playlist.mp4")],
            vec![PathBuf::from("directory.mp4")],
        );

        // When executing FocusDirectory action.
        app.execute_action(Action::FocusDirectory);

        // Then focus switches to directory pane.
        assert_eq!(app.tui_state.focused_pane, Pane::Directory);
    }

    #[test]
    fn move_down_moves_selection_down_in_playlist() {
        // Given a playlist with three items.
        let (mut app, _, _) = create_test_app(
            vec![
                PathBuf::from("a.mp4"),
                PathBuf::from("b.mp4"),
                PathBuf::from("c.mp4"),
            ],
            vec![],
        );

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
        let (mut app, _, _) = create_test_app(
            vec![
                PathBuf::from("a.mp4"),
                PathBuf::from("b.mp4"),
                PathBuf::from("c.mp4"),
            ],
            vec![],
        );
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
    fn move_up_down_navigate_directory() {
        // Given a directory with three items.
        let (mut app, _, _) = create_test_app(
            vec![],
            vec![
                PathBuf::from("x.mp4"),
                PathBuf::from("y.mp4"),
                PathBuf::from("z.mp4"),
            ],
        );
        app.tui_state.focused_pane = Pane::Directory;

        // When navigating with MoveDown/MoveUp.
        assert_eq!(app.tui_state.directory_pane.selected, 0);
        app.execute_action(Action::MoveDown);
        assert_eq!(app.tui_state.directory_pane.selected, 1);
        app.execute_action(Action::MoveUp);

        // Then selection moves correctly.
        assert_eq!(app.tui_state.directory_pane.selected, 0);
    }

    #[test]
    fn reorder_up_moves_playlist_item_up() {
        // Given a playlist with three items and middle item selected.
        let (mut app, _, _) = create_test_app(
            vec![
                PathBuf::from("a.mp4"),
                PathBuf::from("b.mp4"),
                PathBuf::from("c.mp4"),
            ],
            vec![],
        );
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
        let (mut app, _, _) = create_test_app(
            vec![
                PathBuf::from("b.mp4"),
                PathBuf::from("a.mp4"),
                PathBuf::from("c.mp4"),
            ],
            vec![],
        );
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
    fn move_to_playlist_moves_directory_item_to_playlist() {
        // Given a directory with one item and empty playlist.
        let (mut app, _, _) = create_test_app(vec![], vec![PathBuf::from("test.mp4")]);
        app.tui_state.focused_pane = Pane::Directory;

        // When executing MoveToPlaylist action.
        app.execute_action(Action::MoveToPlaylist);

        // Then the item moves to the playlist.
        assert_eq!(app.tui_state.playlist_pane.items.len(), 1);
        assert_eq!(
            app.tui_state.playlist_pane.items[0].path,
            PathBuf::from("test.mp4")
        );
        assert!(app.tui_state.directory_pane.items.is_empty());
    }

    #[test]
    fn move_to_directory_moves_playlist_item_to_directory() {
        // Given a playlist with one item and empty directory.
        let (mut app, _, _) = create_test_app(vec![PathBuf::from("test.mp4")], vec![]);
        app.tui_state.focused_pane = Pane::Playlist;

        // When executing MoveToDirectory action.
        app.execute_action(Action::MoveToDirectory);

        // Then the item moves to the directory.
        assert!(app.tui_state.playlist_pane.items.is_empty());
        assert_eq!(app.tui_state.directory_pane.items.len(), 1);
        assert_eq!(
            app.tui_state.directory_pane.items[0].path,
            PathBuf::from("test.mp4")
        );
    }

    #[test]
    fn toggle_item_moves_item_from_directory_to_playlist() {
        // Given a directory with one item and empty playlist.
        let (mut app, _, _) = create_test_app(vec![], vec![PathBuf::from("test.mp4")]);
        app.tui_state.focused_pane = Pane::Directory;

        // When executing ToggleItem action.
        app.execute_action(Action::ToggleItem);

        // Then the item moves to the playlist.
        assert_eq!(app.tui_state.playlist_pane.items.len(), 1);
        assert_eq!(
            app.tui_state.playlist_pane.items[0].path,
            PathBuf::from("test.mp4")
        );
        assert!(app.tui_state.directory_pane.items.is_empty());
    }

    #[test]
    fn toggle_item_moves_item_from_playlist_to_directory() {
        // Given a playlist with one item and empty directory.
        let (mut app, _, _) = create_test_app(vec![PathBuf::from("test.mp4")], vec![]);
        app.tui_state.focused_pane = Pane::Playlist;

        // When executing ToggleItem action.
        app.execute_action(Action::ToggleItem);

        // Then the item moves to the directory.
        assert!(app.tui_state.playlist_pane.items.is_empty());
        assert_eq!(app.tui_state.directory_pane.items.len(), 1);
    }

    #[test]
    fn play_in_mpv_opens_selected_file_in_mpv() {
        // Given a playlist with one item.
        let (mut app, mpv_backend, _) = create_test_app(vec![PathBuf::from("test.mp4")], vec![]);

        // When executing PlayInMpv action.
        app.execute_action(Action::PlayInMpv);

        // Then mpv receives the file path.
        let calls = mpv_backend.load_file_called.lock().unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0], PathBuf::from("test.mp4"));
        assert!(app
            .tui_state
            .status_message
            .as_ref()
            .unwrap()
            .contains("Playing"));
    }

    #[test]
    fn switch_pane_does_not_switch_to_empty_directory() {
        // Given a playlist with items and empty directory.
        let (mut app, _, _) = create_test_app(vec![PathBuf::from("test.mp4")], vec![]);
        assert_eq!(app.tui_state.focused_pane, Pane::Playlist);

        // When executing SwitchPane action.
        app.execute_action(Action::SwitchPane);

        // Then focus stays on playlist.
        assert_eq!(app.tui_state.focused_pane, Pane::Playlist);
    }

    #[test]
    fn switch_pane_does_not_switch_to_empty_playlist() {
        // Given an empty playlist and directory with items.
        let (mut app, _, _) = create_test_app(vec![], vec![PathBuf::from("test.mp4")]);
        assert_eq!(app.tui_state.focused_pane, Pane::Directory);

        // When executing SwitchPane action.
        app.execute_action(Action::SwitchPane);

        // Then focus stays on directory.
        assert_eq!(app.tui_state.focused_pane, Pane::Directory);
    }

    #[test]
    fn focus_playlist_does_not_switch_to_empty_playlist() {
        // Given an empty playlist and directory with items, focused on directory.
        let (mut app, _, _) = create_test_app(vec![], vec![PathBuf::from("test.mp4")]);
        app.tui_state.focused_pane = Pane::Directory;

        // When executing FocusPlaylist action.
        app.execute_action(Action::FocusPlaylist);

        // Then focus stays on directory.
        assert_eq!(app.tui_state.focused_pane, Pane::Directory);
    }

    #[test]
    fn focus_directory_does_not_switch_to_empty_directory() {
        // Given a playlist with items and empty directory, focused on playlist.
        let (mut app, _, _) = create_test_app(vec![PathBuf::from("test.mp4")], vec![]);

        // When executing FocusDirectory action.
        app.execute_action(Action::FocusDirectory);

        // Then focus stays on playlist.
        assert_eq!(app.tui_state.focused_pane, Pane::Playlist);
    }

    #[test]
    fn initial_focus_directory_when_playlist_empty() {
        // Given an empty playlist and directory with items.
        let (app, _, _) = create_test_app(vec![], vec![PathBuf::from("test.mp4")]);

        // Then focus is on directory.
        assert_eq!(app.tui_state.focused_pane, Pane::Directory);
    }

    #[test]
    fn initial_focus_playlist_when_directory_empty() {
        // Given a playlist with items and empty directory.
        let (app, _, _) = create_test_app(vec![PathBuf::from("test.mp4")], vec![]);

        // Then focus is on playlist.
        assert_eq!(app.tui_state.focused_pane, Pane::Playlist);
    }

    #[test]
    fn initial_focus_playlist_when_both_have_items() {
        // Given both panes with items.
        let (app, _, _) =
            create_test_app(vec![PathBuf::from("a.mp4")], vec![PathBuf::from("b.mp4")]);

        // Then focus is on playlist (default).
        assert_eq!(app.tui_state.focused_pane, Pane::Playlist);
    }

    #[test]
    fn initial_focus_playlist_when_both_empty() {
        // Given both panes empty.
        let (app, _, _) = create_test_app(vec![], vec![]);

        // Then focus is on playlist (default).
        assert_eq!(app.tui_state.focused_pane, Pane::Playlist);
    }

    #[test]
    fn move_last_directory_item_switches_focus_to_playlist() {
        // Given directory with 1 item and playlist with items, focused on directory.
        let (mut app, _, _) = create_test_app(
            vec![PathBuf::from("playlist.mp4")],
            vec![PathBuf::from("directory.mp4")],
        );
        app.tui_state.focused_pane = Pane::Directory;

        // When executing ToggleItem action.
        app.execute_action(Action::ToggleItem);

        // Then focus switches to playlist.
        assert!(app.tui_state.directory_pane.items.is_empty());
        assert_eq!(app.tui_state.focused_pane, Pane::Playlist);
    }

    #[test]
    fn move_last_playlist_item_switches_focus_to_directory() {
        // Given playlist with 1 item and directory with items, focused on playlist.
        let (mut app, _, _) = create_test_app(
            vec![PathBuf::from("playlist.mp4")],
            vec![PathBuf::from("directory.mp4")],
        );
        app.tui_state.focused_pane = Pane::Playlist;

        // When executing ToggleItem action.
        app.execute_action(Action::ToggleItem);

        // Then focus switches to directory.
        assert!(app.tui_state.playlist_pane.items.is_empty());
        assert_eq!(app.tui_state.focused_pane, Pane::Directory);
    }

    #[test]
    fn move_item_keeps_focus_when_pane_not_empty() {
        // Given playlist with 2 items and directory with items, focused on playlist.
        let (mut app, _, _) = create_test_app(
            vec![PathBuf::from("a.mp4"), PathBuf::from("b.mp4")],
            vec![PathBuf::from("c.mp4")],
        );
        app.tui_state.focused_pane = Pane::Playlist;

        // When executing ToggleItem action.
        app.execute_action(Action::ToggleItem);

        // Then focus stays on playlist.
        assert_eq!(app.tui_state.playlist_pane.items.len(), 1);
        assert_eq!(app.tui_state.focused_pane, Pane::Playlist);
    }

    #[test]
    fn move_to_empty_directory_switches_focus() {
        // Given playlist with 1 item and empty directory, focused on playlist.
        let (mut app, _, _) = create_test_app(vec![PathBuf::from("test.mp4")], vec![]);
        app.tui_state.focused_pane = Pane::Playlist;

        // When executing ToggleItem action.
        app.execute_action(Action::ToggleItem);

        // Then focus switches to directory.
        assert_eq!(app.tui_state.focused_pane, Pane::Directory);
    }

    #[test]
    fn move_to_empty_playlist_switches_focus() {
        // Given directory with 1 item and empty playlist, focused on directory.
        let (mut app, _, _) = create_test_app(vec![], vec![PathBuf::from("test.mp4")]);
        app.tui_state.focused_pane = Pane::Directory;

        // When executing ToggleItem action.
        app.execute_action(Action::ToggleItem);

        // Then focus switches to playlist.
        assert_eq!(app.tui_state.focused_pane, Pane::Playlist);
    }
}
