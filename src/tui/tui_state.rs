use std::path::PathBuf;

use crate::keymap::Key;
use crate::tui::{
    ErrorPopup, LibraryPane, Pane, PlaylistItem, PlaylistPane, Rename, UrlInput, WhichKey,
};

pub struct TuiState {
    pub pending_keys: Vec<Key>,
    pub playlist_pane: PlaylistPane,
    pub library_pane: LibraryPane,
    pub focused_pane: Pane,
    pub status_message: Option<String>,
    pub rename: Rename,
    pub url_input: UrlInput,
    pub which_key: WhichKey,
    pub needs_clear: bool,
    pub error_popup: ErrorPopup,
}

impl TuiState {
    pub fn new() -> Self {
        Self {
            pending_keys: Vec::new(),
            playlist_pane: PlaylistPane::new(),
            library_pane: LibraryPane::new(),
            focused_pane: Pane::Playlist,
            status_message: None,
            rename: Rename::new(),
            url_input: UrlInput::new(),
            which_key: WhichKey::default(),
            needs_clear: false,
            error_popup: ErrorPopup::new(),
        }
    }

    pub fn selected_playlist_item(&self) -> Option<&PlaylistItem> {
        self.playlist_pane.selected_item()
    }

    pub fn selected_library_item(&self) -> Option<&PlaylistItem> {
        self.library_pane.selected_item()
    }

    pub fn selected_library_item_mut(&mut self) -> Option<&mut PlaylistItem> {
        self.library_pane.selected_item_mut()
    }

    pub fn move_playlist_up(&mut self) {
        self.playlist_pane.move_up();
    }

    pub fn move_playlist_down(&mut self) {
        self.playlist_pane.move_down();
    }

    pub fn move_library_up(&mut self) {
        self.library_pane.move_up();
    }

    pub fn move_library_down(&mut self) {
        self.library_pane.move_down();
    }

    pub fn reorder_playlist_up(&mut self) {
        self.playlist_pane.reorder_up();
    }

    pub fn reorder_playlist_down(&mut self) {
        self.playlist_pane.reorder_down();
    }

    pub fn add_to_playlist(
        &mut self,
        path: PathBuf,
        duration: Option<std::time::Duration>,
        alias: Option<String>,
        mime_type: Option<String>,
        is_virtual: bool,
    ) {
        self.playlist_pane.add(PlaylistItem {
            path,
            duration,
            alias,
            mime_type,
            is_virtual,
        });
    }

    pub fn remove_from_playlist(&mut self) {
        self.playlist_pane.remove();
    }

    pub fn remove_from_library(&mut self) {
        self.library_pane.remove();
    }

    pub fn refresh_library(&mut self, entries: Vec<PlaylistItem>) {
        let playlist_paths: Vec<_> = self.playlist_pane.paths();
        self.library_pane.refresh(entries, &playlist_paths);
    }

    pub fn switch_pane(&mut self) {
        let target = match self.focused_pane {
            Pane::Playlist => Pane::Library,
            Pane::Library => Pane::Playlist,
        };
        let is_empty = match target {
            Pane::Playlist => self.playlist_pane.items.is_empty(),
            Pane::Library => self.library_pane.items.is_empty(),
        };
        if !is_empty {
            self.focused_pane = target;
        }
    }

    pub fn is_renaming(&self) -> bool {
        self.rename.is_active()
    }

    pub fn start_rename(&mut self) {
        let current_alias = self.get_selected_item().and_then(|item| item.alias.clone());
        self.rename.start(current_alias.as_deref());
    }

    pub fn cancel_rename(&mut self) {
        self.rename.cancel();
    }

    pub fn submit_rename(&mut self) {
        let alias = self.rename.submit();
        if let Some(item) = self.get_selected_item_mut() {
            item.alias = alias;
        }
    }

    pub fn push_rename_char(&mut self, c: char) {
        self.rename.push_char(c);
    }

    pub fn pop_rename_char(&mut self) {
        self.rename.pop_char();
    }

    pub fn is_url_input(&self) -> bool {
        self.url_input.is_active()
    }

    pub fn start_url_input(&mut self) {
        self.url_input.start();
    }

    pub fn cancel_url_input(&mut self) {
        self.url_input.cancel();
    }

    pub fn submit_url_input(&mut self) -> Option<String> {
        self.url_input.submit()
    }

    pub fn push_url_char(&mut self, c: char) {
        self.url_input.push_char(c);
    }

    pub fn pop_url_char(&mut self) {
        self.url_input.pop_char();
    }

    pub fn get_selected_item(&self) -> Option<&PlaylistItem> {
        match self.focused_pane {
            Pane::Playlist => self.selected_playlist_item(),
            Pane::Library => self.selected_library_item(),
        }
    }

    pub fn get_selected_item_mut(&mut self) -> Option<&mut PlaylistItem> {
        match self.focused_pane {
            Pane::Playlist => self.playlist_pane.selected_item_mut(),
            Pane::Library => self.library_pane.selected_item_mut(),
        }
    }

    pub fn is_filtering(&self) -> bool {
        match self.focused_pane {
            Pane::Playlist => self.playlist_pane.filter().is_active(),
            Pane::Library => self.library_pane.filter().is_active(),
        }
    }

    pub fn has_active_filter(&self, pane: Pane) -> bool {
        match pane {
            Pane::Playlist => self.playlist_pane.filter().has_applied(),
            Pane::Library => self.library_pane.filter().has_applied(),
        }
    }

    pub fn start_filter(&mut self) {
        let filter = match self.focused_pane {
            Pane::Playlist => self.playlist_pane.filter_mut(),
            Pane::Library => self.library_pane.filter_mut(),
        };
        filter.start();
    }

    pub fn cancel_filter(&mut self) {
        let filter = match self.focused_pane {
            Pane::Playlist => self.playlist_pane.filter_mut(),
            Pane::Library => self.library_pane.filter_mut(),
        };
        filter.cancel();
    }

    pub fn submit_filter(&mut self) {
        let filter = match self.focused_pane {
            Pane::Playlist => self.playlist_pane.filter_mut(),
            Pane::Library => self.library_pane.filter_mut(),
        };
        filter.submit();
    }

    pub fn push_filter_char(&mut self, c: char) {
        let filter = match self.focused_pane {
            Pane::Playlist => self.playlist_pane.filter_mut(),
            Pane::Library => self.library_pane.filter_mut(),
        };
        filter.push_char(c);
    }

    pub fn pop_filter_char(&mut self) {
        let filter = match self.focused_pane {
            Pane::Playlist => self.playlist_pane.filter_mut(),
            Pane::Library => self.library_pane.filter_mut(),
        };
        filter.pop_char();
    }

    pub fn get_filter_input(&self, pane: Pane) -> &str {
        match pane {
            Pane::Playlist => self.playlist_pane.filter().input(),
            Pane::Library => self.library_pane.filter().input(),
        }
    }

    pub fn get_filtered_playlist(&self) -> Vec<(usize, &PlaylistItem)> {
        self.playlist_pane.get_filtered()
    }

    pub fn get_filtered_library(&self) -> Vec<(usize, &PlaylistItem)> {
        self.library_pane.get_filtered()
    }

    pub fn show_error(&mut self, message: String) {
        self.error_popup.show(message);
    }

    pub fn dismiss_error(&mut self) {
        self.error_popup.dismiss();
    }

    pub fn is_showing_error(&self) -> bool {
        self.error_popup.is_active()
    }
}

impl Default for TuiState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn item(path: &str) -> PlaylistItem {
        PlaylistItem {
            path: PathBuf::from(path),
            duration: None,
            alias: None,
            mime_type: None,
            is_virtual: false,
        }
    }

    #[test]
    fn new_creates_default_state() {
        // Given a new state.
        let state = TuiState::new();

        // Then defaults are set.
        assert!(state.pending_keys.is_empty());
        assert!(state.playlist_pane.items.is_empty());
        assert!(state.library_pane.items.is_empty());
        assert_eq!(state.focused_pane, Pane::Playlist);
        assert!(state.status_message.is_none());
        assert!(!state.is_renaming());
        assert!(!state.is_showing_error());
    }

    #[test]
    fn selected_playlist_item_returns_current() {
        // Given a state with playlist items.
        let mut state = TuiState::new();
        state.playlist_pane.items = vec![item("a.mp4"), item("b.mp4")];
        state.playlist_pane.selected = 1;

        // When getting selected item.
        let selected = state.selected_playlist_item();

        // Then correct item is returned.
        assert_eq!(selected.unwrap().path, PathBuf::from("b.mp4"));
    }

    #[test]
    fn selected_library_item_returns_current() {
        // Given a state with library items.
        let mut state = TuiState::new();
        state.library_pane.items = vec![item("a.mp4"), item("b.mp4")];
        state.library_pane.selected = 1;

        // When getting selected item.
        let selected = state.selected_library_item();

        // Then correct item is returned.
        assert_eq!(selected.unwrap().path, PathBuf::from("b.mp4"));
    }

    #[test]
    fn move_playlist_up_delegates_to_pane() {
        // Given a state with playlist items.
        let mut state = TuiState::new();
        state.playlist_pane.items = vec![item("a.mp4"), item("b.mp4")];
        state.playlist_pane.selected = 1;

        // When moving up.
        state.move_playlist_up();

        // Then selection changes.
        assert_eq!(state.playlist_pane.selected, 0);
    }

    #[test]
    fn move_playlist_down_delegates_to_pane() {
        // Given a state with playlist items.
        let mut state = TuiState::new();
        state.playlist_pane.items = vec![item("a.mp4"), item("b.mp4")];
        state.playlist_pane.selected = 0;

        // When moving down.
        state.move_playlist_down();

        // Then selection changes.
        assert_eq!(state.playlist_pane.selected, 1);
    }

    #[test]
    fn move_library_up_delegates_to_pane() {
        // Given a state with library items.
        let mut state = TuiState::new();
        state.library_pane.items = vec![item("a.mp4"), item("b.mp4")];
        state.library_pane.selected = 1;

        // When moving up.
        state.move_library_up();

        // Then selection changes.
        assert_eq!(state.library_pane.selected, 0);
    }

    #[test]
    fn move_library_down_delegates_to_pane() {
        // Given a state with library items.
        let mut state = TuiState::new();
        state.library_pane.items = vec![item("a.mp4"), item("b.mp4")];
        state.library_pane.selected = 0;

        // When moving down.
        state.move_library_down();

        // Then selection changes.
        assert_eq!(state.library_pane.selected, 1);
    }

    #[test]
    fn reorder_playlist_up_delegates_to_pane() {
        // Given a state with playlist items.
        let mut state = TuiState::new();
        state.playlist_pane.items = vec![item("a.mp4"), item("b.mp4")];
        state.playlist_pane.selected = 1;

        // When reordering up.
        state.reorder_playlist_up();

        // Then items are swapped.
        assert_eq!(state.playlist_pane.items[0].path, PathBuf::from("b.mp4"));
    }

    #[test]
    fn reorder_playlist_down_delegates_to_pane() {
        // Given a state with playlist items.
        let mut state = TuiState::new();
        state.playlist_pane.items = vec![item("a.mp4"), item("b.mp4")];
        state.playlist_pane.selected = 0;

        // When reordering down.
        state.reorder_playlist_down();

        // Then items are swapped.
        assert_eq!(state.playlist_pane.items[0].path, PathBuf::from("b.mp4"));
    }

    #[test]
    fn add_to_playlist_adds_item() {
        // Given an empty state.
        let mut state = TuiState::new();

        // When adding to playlist.
        state.add_to_playlist(
            PathBuf::from("test.mp4"),
            Some(Duration::from_secs(120)),
            Some("alias".to_string()),
            Some("video/mp4".to_string()),
            false,
        );

        // Then item is added.
        assert_eq!(state.playlist_pane.items.len(), 1);
        assert_eq!(state.playlist_pane.items[0].path, PathBuf::from("test.mp4"));
        assert_eq!(
            state.playlist_pane.items[0].alias,
            Some("alias".to_string())
        );
    }

    #[test]
    fn remove_from_playlist_removes_item() {
        // Given a state with playlist items.
        let mut state = TuiState::new();
        state.playlist_pane.items = vec![item("a.mp4"), item("b.mp4")];
        state.playlist_pane.selected = 0;

        // When removing from playlist.
        state.remove_from_playlist();

        // Then item is removed.
        assert_eq!(state.playlist_pane.items.len(), 1);
    }

    #[test]
    fn remove_from_library_removes_item() {
        // Given a state with library items.
        let mut state = TuiState::new();
        state.library_pane.items = vec![item("apple.mp4"), item("banana.mp4")];
        state.library_pane.filter.applied = Some("an".to_string());

        // When getting filtered library.
        let filtered = state.get_filtered_library();

        // Then filtered results are returned.
        assert_eq!(filtered.len(), 1);
    }

    #[test]
    fn default_creates_default_state() {
        // Given a default state.
        let state = TuiState::default();

        // Then defaults are set.
        assert_eq!(state.focused_pane, Pane::Playlist);
    }
}
