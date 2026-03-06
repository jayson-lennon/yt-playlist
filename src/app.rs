use std::path::PathBuf;

use crossterm::event::{Event, KeyCode};

use crate::services::Services;
use crate::tui_state::{Pane, PlaylistItem, TuiState};

pub const DEFAULT_EXTENSIONS: &[&str] = &[
    "mp4", "mkv", "avi", "webm", "mov", "flv", "wmv", "mp3", "flac", "wav", "ogg", "m4a", "aac",
];

pub struct App {
    pub services: Services,
    pub tui_state: TuiState,
    pub extensions: Vec<String>,
    pub should_quit: bool,
}

impl App {
    pub fn new(services: Services, extensions: Vec<String>) -> Self {
        let mut app = Self {
            services,
            tui_state: TuiState::new(),
            extensions,
            should_quit: false,
        };
        app.load_playlist();
        app.refresh_directory();
        app
    }

    pub fn load_playlist(&mut self) {
        match self.services.storage.load() {
            Ok(items) => {
                self.tui_state.playlist = items
                    .into_iter()
                    .map(|path| {
                        let duration = self.services.media.get_duration(&path).ok();
                        PlaylistItem { path, duration }
                    })
                    .collect();
                self.refresh_directory();
            }
            Err(e) => {
                self.tui_state.status_message = Some(format!("Failed to load playlist: {e:?}"));
            }
        }
    }

    pub fn save_playlist(&mut self) {
        let items: Vec<PathBuf> = self
            .tui_state
            .playlist
            .iter()
            .map(|item| item.path.clone())
            .collect();
        match self.services.storage.save(&items) {
            Ok(()) => {
                self.tui_state.status_message = Some("Playlist saved".to_string());
            }
            Err(e) => {
                self.tui_state.status_message = Some(format!("Failed to save playlist: {e:?}"));
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
                            let duration = self.services.media.get_duration(&path).ok();
                            entries.push(PlaylistItem { path, duration });
                        }
                    }
                }
            }
        }
        entries.sort_by(|a, b| a.path.cmp(&b.path));
        self.tui_state.refresh_directory(entries);
    }

    pub fn handle_event(&mut self, event: Event) {
        if let Event::Key(key) = event {
            match key.code {
                KeyCode::Char('q') => {
                    self.save_playlist();
                    self.should_quit = true;
                }
                KeyCode::Char('s') => {
                    self.save_playlist();
                }
                KeyCode::Char('o') => {
                    self.open_in_mpv();
                }
                KeyCode::Tab => {
                    self.tui_state.switch_pane();
                }
                KeyCode::Char('h') => {
                    self.tui_state.focused_pane = Pane::Playlist;
                }
                KeyCode::Char('l') => {
                    self.tui_state.focused_pane = Pane::Directory;
                }
                KeyCode::Char('H') => {
                    if self.tui_state.focused_pane == Pane::Directory {
                        self.move_from_directory_to_playlist();
                    }
                }
                KeyCode::Char('L') => {
                    if self.tui_state.focused_pane == Pane::Playlist {
                        self.move_from_playlist_to_directory();
                    }
                }
                KeyCode::Char('j') => match self.tui_state.focused_pane {
                    Pane::Playlist => self.tui_state.move_playlist_down(),
                    Pane::Directory => self.tui_state.move_directory_down(),
                },
                KeyCode::Char('k') => match self.tui_state.focused_pane {
                    Pane::Playlist => self.tui_state.move_playlist_up(),
                    Pane::Directory => self.tui_state.move_directory_up(),
                },
                KeyCode::Char('J') => {
                    if self.tui_state.focused_pane == Pane::Playlist {
                        self.tui_state.reorder_playlist_down();
                    }
                }
                KeyCode::Char('K') => {
                    if self.tui_state.focused_pane == Pane::Playlist {
                        self.tui_state.reorder_playlist_up();
                    }
                }
                KeyCode::Char('x') => {
                    self.tui_state.remove_from_playlist();
                }
                KeyCode::Char(' ') | KeyCode::Enter => match self.tui_state.focused_pane {
                    Pane::Directory => {
                        self.move_from_directory_to_playlist();
                    }
                    Pane::Playlist => {
                        self.move_from_playlist_to_directory();
                    }
                },
                _ => {}
            }
        }
    }

    fn move_from_directory_to_playlist(&mut self) {
        if let Some(item) = self.tui_state.selected_directory_item().cloned() {
            self.tui_state
                .add_to_playlist(item.path.clone(), item.duration);
            self.tui_state
                .directory
                .remove(self.tui_state.directory_selected);
            if self.tui_state.directory_selected >= self.tui_state.directory.len()
                && !self.tui_state.directory.is_empty()
            {
                self.tui_state.directory_selected = self.tui_state.directory.len() - 1;
            }
        }
    }

    fn move_from_playlist_to_directory(&mut self) {
        if let Some(item) = self.tui_state.selected_playlist_item().cloned() {
            self.tui_state.directory.push(item);
            self.tui_state.directory.sort_by(|a, b| a.path.cmp(&b.path));
            self.tui_state.remove_from_playlist();
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
                    self.tui_state.status_message = Some(format!("Failed to open in mpv: {e:?}"));
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

    use crossterm::event::{KeyEvent, KeyModifiers};
    use error_stack::Report;
    use rstest::rstest;

    use super::*;
    use crate::media::{MediaError, MediaQuery, MediaQueryBackend};
    use crate::mpv::{MpvBackend, MpvClient, MpvError};
    use crate::playlist::{IoError, PlaylistStorage, PlaylistStorageBackend};

    fn key_event(code: KeyCode) -> Event {
        Event::Key(KeyEvent::new(code, KeyModifiers::empty()))
    }

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
        saved_items: Arc<Mutex<Vec<PathBuf>>>,
    }

    impl MockStorageBackend {
        fn new() -> Self {
            Self {
                saved_items: Arc::new(Mutex::new(Vec::new())),
            }
        }
    }

    impl PlaylistStorageBackend for MockStorageBackend {
        fn name(&self) -> &'static str {
            "mock"
        }

        fn load(&self) -> Result<Vec<PathBuf>, Report<IoError>> {
            Ok(Vec::new())
        }

        fn save(&self, items: &[PathBuf]) -> Result<(), Report<IoError>> {
            *self.saved_items.lock().unwrap() = items.to_vec();
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
        };

        for path in playlist_items {
            let duration = app.services.media.get_duration(&path).ok();
            app.tui_state.playlist.push(PlaylistItem { path, duration });
        }

        for path in directory_items {
            let duration = app.services.media.get_duration(&path).ok();
            app.tui_state
                .directory
                .push(PlaylistItem { path, duration });
        }

        (app, mpv_backend, storage_backend)
    }

    #[test]
    fn quit_key_saves_and_exits() {
        // Given an app with an empty playlist.
        let (mut app, _, storage) = create_test_app(vec![], vec![]);

        // When pressing 'q'.
        app.handle_event(key_event(KeyCode::Char('q')));

        // Then the app should quit and save the playlist.
        assert!(app.should_quit);
        let saved = storage.saved_items.lock().unwrap();
        assert!(saved.is_empty());
    }

    #[test]
    fn save_key_persists_playlist() {
        // Given an app with one item in the playlist.
        let (mut app, _, storage) = create_test_app(vec![PathBuf::from("test.mp4")], vec![]);

        // When pressing 's'.
        app.handle_event(key_event(KeyCode::Char('s')));

        // Then the playlist is saved.
        let saved = storage.saved_items.lock().unwrap();
        assert_eq!(saved.len(), 1);
        assert_eq!(saved[0], PathBuf::from("test.mp4"));
    }

    #[test]
    fn tab_key_toggles_between_panes() {
        // Given an app focused on the playlist pane.
        let (mut app, _, _) = create_test_app(vec![], vec![]);
        assert_eq!(app.tui_state.focused_pane, Pane::Playlist);

        // When pressing Tab.
        app.handle_event(key_event(KeyCode::Tab));

        // Then focus switches to directory pane.
        assert_eq!(app.tui_state.focused_pane, Pane::Directory);

        // When pressing Tab again.
        app.handle_event(key_event(KeyCode::Tab));

        // Then focus switches back to playlist pane.
        assert_eq!(app.tui_state.focused_pane, Pane::Playlist);
    }

    #[test]
    fn h_key_switches_to_playlist_pane() {
        // Given an app focused on the directory pane.
        let (mut app, _, _) = create_test_app(vec![], vec![]);
        app.tui_state.focused_pane = Pane::Directory;

        // When pressing 'h'.
        app.handle_event(key_event(KeyCode::Char('h')));

        // Then focus switches to playlist pane.
        assert_eq!(app.tui_state.focused_pane, Pane::Playlist);
    }

    #[test]
    fn l_key_switches_to_directory_pane() {
        // Given an app focused on the playlist pane.
        let (mut app, _, _) = create_test_app(vec![], vec![]);

        // When pressing 'l'.
        app.handle_event(key_event(KeyCode::Char('l')));

        // Then focus switches to directory pane.
        assert_eq!(app.tui_state.focused_pane, Pane::Directory);
    }

    #[test]
    fn j_key_moves_selection_down_in_playlist() {
        // Given a playlist with three items.
        let (mut app, _, _) = create_test_app(
            vec![
                PathBuf::from("a.mp4"),
                PathBuf::from("b.mp4"),
                PathBuf::from("c.mp4"),
            ],
            vec![],
        );

        // When pressing 'j' multiple times.
        assert_eq!(app.tui_state.playlist_selected, 0);
        app.handle_event(key_event(KeyCode::Char('j')));
        assert_eq!(app.tui_state.playlist_selected, 1);
        app.handle_event(key_event(KeyCode::Char('j')));
        assert_eq!(app.tui_state.playlist_selected, 2);
        app.handle_event(key_event(KeyCode::Char('j')));

        // Then selection stays at the last item.
        assert_eq!(app.tui_state.playlist_selected, 2);
    }

    #[test]
    fn k_key_moves_selection_up_in_playlist() {
        // Given a playlist with three items and selection on the last item.
        let (mut app, _, _) = create_test_app(
            vec![
                PathBuf::from("a.mp4"),
                PathBuf::from("b.mp4"),
                PathBuf::from("c.mp4"),
            ],
            vec![],
        );
        app.tui_state.playlist_selected = 2;

        // When pressing 'k' multiple times.
        app.handle_event(key_event(KeyCode::Char('k')));
        assert_eq!(app.tui_state.playlist_selected, 1);
        app.handle_event(key_event(KeyCode::Char('k')));
        assert_eq!(app.tui_state.playlist_selected, 0);
        app.handle_event(key_event(KeyCode::Char('k')));

        // Then selection stays at the first item.
        assert_eq!(app.tui_state.playlist_selected, 0);
    }

    #[test]
    fn j_k_keys_navigate_directory() {
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

        // When navigating with j/k.
        assert_eq!(app.tui_state.directory_selected, 0);
        app.handle_event(key_event(KeyCode::Char('j')));
        assert_eq!(app.tui_state.directory_selected, 1);
        app.handle_event(key_event(KeyCode::Char('k')));

        // Then selection moves correctly.
        assert_eq!(app.tui_state.directory_selected, 0);
    }

    #[test]
    fn shift_k_moves_playlist_item_up() {
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
        app.tui_state.playlist_selected = 1;

        // When pressing 'K'.
        app.handle_event(key_event(KeyCode::Char('K')));

        // Then the item moves up and selection follows.
        assert_eq!(app.tui_state.playlist_selected, 0);
        assert_eq!(app.tui_state.playlist[0].path, PathBuf::from("b.mp4"));
        assert_eq!(app.tui_state.playlist[1].path, PathBuf::from("a.mp4"));
    }

    #[test]
    fn shift_j_moves_playlist_item_down() {
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
        app.tui_state.playlist_selected = 0;

        // When pressing 'J'.
        app.handle_event(key_event(KeyCode::Char('J')));

        // Then the item moves down and selection follows.
        assert_eq!(app.tui_state.playlist_selected, 1);
        assert_eq!(app.tui_state.playlist[0].path, PathBuf::from("a.mp4"));
        assert_eq!(app.tui_state.playlist[1].path, PathBuf::from("b.mp4"));
    }

    #[test]
    fn x_key_removes_selected_item_from_playlist() {
        // Given a playlist with two items.
        let (mut app, _, _) =
            create_test_app(vec![PathBuf::from("a.mp4"), PathBuf::from("b.mp4")], vec![]);

        // When pressing 'x'.
        app.handle_event(key_event(KeyCode::Char('x')));

        // Then the first item is removed.
        assert_eq!(app.tui_state.playlist.len(), 1);
        assert_eq!(app.tui_state.playlist[0].path, PathBuf::from("b.mp4"));
    }

    #[test]
    fn shift_h_moves_directory_item_to_playlist() {
        // Given a directory with one item and empty playlist.
        let (mut app, _, _) = create_test_app(vec![], vec![PathBuf::from("test.mp4")]);
        app.tui_state.focused_pane = Pane::Directory;

        // When pressing 'H'.
        app.handle_event(key_event(KeyCode::Char('H')));

        // Then the item moves to the playlist.
        assert_eq!(app.tui_state.playlist.len(), 1);
        assert_eq!(app.tui_state.playlist[0].path, PathBuf::from("test.mp4"));
        assert!(app.tui_state.directory.is_empty());
    }

    #[test]
    fn shift_l_moves_playlist_item_to_directory() {
        // Given a playlist with one item and empty directory.
        let (mut app, _, _) = create_test_app(vec![PathBuf::from("test.mp4")], vec![]);
        app.tui_state.focused_pane = Pane::Playlist;

        // When pressing 'L'.
        app.handle_event(key_event(KeyCode::Char('L')));

        // Then the item moves to the directory.
        assert!(app.tui_state.playlist.is_empty());
        assert_eq!(app.tui_state.directory.len(), 1);
        assert_eq!(app.tui_state.directory[0].path, PathBuf::from("test.mp4"));
    }

    #[rstest]
    #[case(KeyCode::Char(' '))]
    #[case(KeyCode::Enter)]
    fn key_moves_item_from_directory_to_playlist(#[case] key: KeyCode) {
        // Given a directory with one item and empty playlist.
        let (mut app, _, _) = create_test_app(vec![], vec![PathBuf::from("test.mp4")]);
        app.tui_state.focused_pane = Pane::Directory;

        // When pressing the key.
        app.handle_event(key_event(key));

        // Then the item moves to the playlist.
        assert_eq!(app.tui_state.playlist.len(), 1);
        assert_eq!(app.tui_state.playlist[0].path, PathBuf::from("test.mp4"));
        assert!(app.tui_state.directory.is_empty());
    }

    #[rstest]
    #[case(KeyCode::Char(' '))]
    #[case(KeyCode::Enter)]
    fn key_moves_item_from_playlist_to_directory(#[case] key: KeyCode) {
        // Given a playlist with one item and empty directory.
        let (mut app, _, _) = create_test_app(vec![PathBuf::from("test.mp4")], vec![]);
        app.tui_state.focused_pane = Pane::Playlist;

        // When pressing the key.
        app.handle_event(key_event(key));

        // Then the item moves to the directory.
        assert!(app.tui_state.playlist.is_empty());
        assert_eq!(app.tui_state.directory.len(), 1);
    }

    #[test]
    fn o_key_opens_selected_file_in_mpv() {
        // Given a playlist with one item.
        let (mut app, mpv_backend, _) = create_test_app(vec![PathBuf::from("test.mp4")], vec![]);

        // When pressing 'o'.
        app.handle_event(key_event(KeyCode::Char('o')));

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
}
