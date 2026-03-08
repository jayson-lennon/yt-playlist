use std::path::PathBuf;

use crate::ui::{DirectoryPane, ErrorPopup, Pane, PlaylistItem, PlaylistPane, Rename, WhichKey};

pub struct TuiState {
    pub pending_key: Option<char>,
    pub playlist_pane: PlaylistPane,
    pub directory_pane: DirectoryPane,
    pub focused_pane: Pane,
    pub status_message: Option<String>,
    pub rename: Rename,
    pub which_key: WhichKey,
    pub needs_clear: bool,
    pub error_popup: ErrorPopup,
}

impl TuiState {
    pub fn new() -> Self {
        Self {
            pending_key: None,
            playlist_pane: PlaylistPane::new(),
            directory_pane: DirectoryPane::new(),
            focused_pane: Pane::Playlist,
            status_message: None,
            rename: Rename::new(),
            which_key: WhichKey::default(),
            needs_clear: false,
            error_popup: ErrorPopup::new(),
        }
    }

    pub fn selected_playlist_item(&self) -> Option<&PlaylistItem> {
        self.playlist_pane.selected_item()
    }

    pub fn selected_directory_item(&self) -> Option<&PlaylistItem> {
        self.directory_pane.selected_item()
    }

    pub fn move_playlist_up(&mut self) {
        self.playlist_pane.move_up();
    }

    pub fn move_playlist_down(&mut self) {
        self.playlist_pane.move_down();
    }

    pub fn move_directory_up(&mut self) {
        self.directory_pane.move_up();
    }

    pub fn move_directory_down(&mut self) {
        self.directory_pane.move_down();
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
    ) {
        self.playlist_pane.add(PlaylistItem {
            path,
            duration,
            alias,
            mime_type,
        });
    }

    pub fn remove_from_playlist(&mut self) {
        self.playlist_pane.remove();
    }

    pub fn remove_from_directory(&mut self) {
        self.directory_pane.remove();
    }

    pub fn refresh_directory(&mut self, entries: Vec<PlaylistItem>) {
        let playlist_paths: Vec<_> = self.playlist_pane.paths();
        self.directory_pane.refresh(entries, &playlist_paths);
    }

    pub fn switch_pane(&mut self) {
        let target = match self.focused_pane {
            Pane::Playlist => Pane::Directory,
            Pane::Directory => Pane::Playlist,
        };
        let is_empty = match target {
            Pane::Playlist => self.playlist_pane.items.is_empty(),
            Pane::Directory => self.directory_pane.items.is_empty(),
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

    pub fn get_selected_item(&self) -> Option<&PlaylistItem> {
        match self.focused_pane {
            Pane::Playlist => self.selected_playlist_item(),
            Pane::Directory => self.selected_directory_item(),
        }
    }

    pub fn get_selected_item_mut(&mut self) -> Option<&mut PlaylistItem> {
        match self.focused_pane {
            Pane::Playlist => self.playlist_pane.selected_item_mut(),
            Pane::Directory => self.directory_pane.selected_item_mut(),
        }
    }

    pub fn is_filtering(&self) -> bool {
        match self.focused_pane {
            Pane::Playlist => self.playlist_pane.filter().is_active(),
            Pane::Directory => self.directory_pane.filter().is_active(),
        }
    }

    pub fn has_active_filter(&self, pane: Pane) -> bool {
        match pane {
            Pane::Playlist => self.playlist_pane.filter().has_applied(),
            Pane::Directory => self.directory_pane.filter().has_applied(),
        }
    }

    pub fn start_filter(&mut self) {
        let filter = match self.focused_pane {
            Pane::Playlist => self.playlist_pane.filter_mut(),
            Pane::Directory => self.directory_pane.filter_mut(),
        };
        filter.start();
    }

    pub fn cancel_filter(&mut self) {
        let filter = match self.focused_pane {
            Pane::Playlist => self.playlist_pane.filter_mut(),
            Pane::Directory => self.directory_pane.filter_mut(),
        };
        filter.cancel();
    }

    pub fn submit_filter(&mut self) {
        let filter = match self.focused_pane {
            Pane::Playlist => self.playlist_pane.filter_mut(),
            Pane::Directory => self.directory_pane.filter_mut(),
        };
        filter.submit();
    }

    pub fn push_filter_char(&mut self, c: char) {
        let filter = match self.focused_pane {
            Pane::Playlist => self.playlist_pane.filter_mut(),
            Pane::Directory => self.directory_pane.filter_mut(),
        };
        filter.push_char(c);
    }

    pub fn pop_filter_char(&mut self) {
        let filter = match self.focused_pane {
            Pane::Playlist => self.playlist_pane.filter_mut(),
            Pane::Directory => self.directory_pane.filter_mut(),
        };
        filter.pop_char();
    }

    pub fn get_filter_input(&self, pane: Pane) -> &str {
        match pane {
            Pane::Playlist => self.playlist_pane.filter().input(),
            Pane::Directory => self.directory_pane.filter().input(),
        }
    }

    pub fn get_filtered_playlist(&self) -> Vec<(usize, &PlaylistItem)> {
        self.playlist_pane.get_filtered()
    }

    pub fn get_filtered_directory(&self) -> Vec<(usize, &PlaylistItem)> {
        self.directory_pane.get_filtered()
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
        }
    }

    #[test]
    fn new_creates_default_state() {
        // Given a new state.
        let state = TuiState::new();

        // Then defaults are set.
        assert!(state.pending_key.is_none());
        assert!(state.playlist_pane.items.is_empty());
        assert!(state.directory_pane.items.is_empty());
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
    fn selected_directory_item_returns_current() {
        // Given a state with directory items.
        let mut state = TuiState::new();
        state.directory_pane.items = vec![item("a.mp4"), item("b.mp4")];
        state.directory_pane.selected = 1;

        // When getting selected item.
        let selected = state.selected_directory_item();

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
    fn move_directory_up_delegates_to_pane() {
        // Given a state with directory items.
        let mut state = TuiState::new();
        state.directory_pane.items = vec![item("a.mp4"), item("b.mp4")];
        state.directory_pane.selected = 1;

        // When moving up.
        state.move_directory_up();

        // Then selection changes.
        assert_eq!(state.directory_pane.selected, 0);
    }

    #[test]
    fn move_directory_down_delegates_to_pane() {
        // Given a state with directory items.
        let mut state = TuiState::new();
        state.directory_pane.items = vec![item("a.mp4"), item("b.mp4")];
        state.directory_pane.selected = 0;

        // When moving down.
        state.move_directory_down();

        // Then selection changes.
        assert_eq!(state.directory_pane.selected, 1);
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
    fn remove_from_directory_removes_item() {
        // Given a state with directory items.
        let mut state = TuiState::new();
        state.directory_pane.items = vec![item("a.mp4"), item("b.mp4")];
        state.directory_pane.selected = 0;

        // When removing from directory.
        state.remove_from_directory();

        // Then item is removed.
        assert_eq!(state.directory_pane.items.len(), 1);
    }

    #[test]
    fn refresh_directory_excludes_playlist_items() {
        // Given a state with playlist items.
        let mut state = TuiState::new();
        state.playlist_pane.items = vec![item("a.mp4")];

        // When refreshing directory.
        let entries = vec![item("a.mp4"), item("b.mp4"), item("c.mp4")];
        state.refresh_directory(entries);

        // Then items in playlist are excluded.
        assert_eq!(state.directory_pane.items.len(), 2);
        assert!(state
            .directory_pane
            .items
            .iter()
            .all(|i| i.path != PathBuf::from("a.mp4")));
    }

    #[test]
    fn switch_pane_toggles_between_panes() {
        // Given a state with items in both panes.
        let mut state = TuiState::new();
        state.playlist_pane.items = vec![item("a.mp4")];
        state.directory_pane.items = vec![item("b.mp4")];
        state.focused_pane = Pane::Playlist;

        // When switching pane.
        state.switch_pane();

        // Then focus changes.
        assert_eq!(state.focused_pane, Pane::Directory);

        // When switching again.
        state.switch_pane();

        // Then focus changes back.
        assert_eq!(state.focused_pane, Pane::Playlist);
    }

    #[test]
    fn switch_pane_does_not_switch_to_empty_pane() {
        // Given a state with only playlist items.
        let mut state = TuiState::new();
        state.playlist_pane.items = vec![item("a.mp4")];
        state.directory_pane.items = vec![];
        state.focused_pane = Pane::Playlist;

        // When switching pane.
        state.switch_pane();

        // Then focus stays on playlist.
        assert_eq!(state.focused_pane, Pane::Playlist);
    }

    #[test]
    fn start_rename_activates_rename_mode() {
        // Given a state with a selected item.
        let mut state = TuiState::new();
        state.playlist_pane.items = vec![PlaylistItem {
            path: PathBuf::from("test.mp4"),
            duration: None,
            alias: Some("existing".to_string()),
            mime_type: None,
        }];

        // When starting rename.
        state.start_rename();

        // Then rename mode is active with existing alias.
        assert!(state.is_renaming());
        assert_eq!(state.rename.input(), "existing");
    }

    #[test]
    fn cancel_rename_deactivates_rename_mode() {
        // Given a state in rename mode.
        let mut state = TuiState::new();
        state.rename.active = true;

        // When canceling rename.
        state.cancel_rename();

        // Then rename mode is inactive.
        assert!(!state.is_renaming());
    }

    #[test]
    fn submit_rename_updates_item_alias() {
        // Given a state with a selected item and rename input.
        let mut state = TuiState::new();
        state.playlist_pane.items = vec![item("test.mp4")];
        state.focused_pane = Pane::Playlist;
        state.rename.input = "new alias".to_string();
        state.rename.active = true;

        // When submitting rename.
        state.submit_rename();

        // Then item alias is updated.
        assert_eq!(
            state.playlist_pane.items[0].alias,
            Some("new alias".to_string())
        );
        assert!(!state.is_renaming());
    }

    #[test]
    fn submit_rename_clears_alias_when_input_empty() {
        // Given a state with a selected item having an alias and empty rename input.
        let mut state = TuiState::new();
        state.playlist_pane.items = vec![PlaylistItem {
            path: PathBuf::from("test.mp4"),
            duration: None,
            alias: Some("old".to_string()),
            mime_type: None,
        }];
        state.focused_pane = Pane::Playlist;
        state.rename.input = String::new();
        state.rename.active = true;

        // When submitting rename.
        state.submit_rename();

        // Then alias is cleared.
        assert_eq!(state.playlist_pane.items[0].alias, None);
    }

    #[test]
    fn push_rename_char_adds_to_input() {
        // Given a state in rename mode.
        let mut state = TuiState::new();
        state.rename.active = true;
        state.rename.input = "ab".to_string();

        // When pushing a character.
        state.push_rename_char('c');

        // Then character is added.
        assert_eq!(state.rename.input(), "abc");
    }

    #[test]
    fn pop_rename_char_removes_from_input() {
        // Given a state in rename mode.
        let mut state = TuiState::new();
        state.rename.active = true;
        state.rename.input = "abc".to_string();

        // When popping a character.
        state.pop_rename_char();

        // Then character is removed.
        assert_eq!(state.rename.input(), "ab");
    }

    #[test]
    fn get_selected_item_returns_playlist_item_when_focused() {
        // Given a state focused on playlist.
        let mut state = TuiState::new();
        state.playlist_pane.items = vec![item("playlist.mp4")];
        state.directory_pane.items = vec![item("directory.mp4")];
        state.focused_pane = Pane::Playlist;

        // When getting selected item.
        let selected = state.get_selected_item();

        // Then playlist item is returned.
        assert_eq!(selected.unwrap().path, PathBuf::from("playlist.mp4"));
    }

    #[test]
    fn get_selected_item_returns_directory_item_when_focused() {
        // Given a state focused on directory.
        let mut state = TuiState::new();
        state.playlist_pane.items = vec![item("playlist.mp4")];
        state.directory_pane.items = vec![item("directory.mp4")];
        state.focused_pane = Pane::Directory;

        // When getting selected item.
        let selected = state.get_selected_item();

        // Then directory item is returned.
        assert_eq!(selected.unwrap().path, PathBuf::from("directory.mp4"));
    }

    #[test]
    fn is_filtering_returns_true_when_playlist_filter_active() {
        // Given a state with active playlist filter.
        let mut state = TuiState::new();
        state.focused_pane = Pane::Playlist;
        state.playlist_pane.filter.active = true;

        // When checking if filtering.
        assert!(state.is_filtering());
    }

    #[test]
    fn is_filtering_returns_true_when_directory_filter_active() {
        // Given a state with active directory filter.
        let mut state = TuiState::new();
        state.focused_pane = Pane::Directory;
        state.directory_pane.filter.active = true;

        // When checking if filtering.
        assert!(state.is_filtering());
    }

    #[test]
    fn is_filtering_returns_false_when_no_filter_active() {
        // Given a state with no active filter.
        let state = TuiState::new();

        // When checking if filtering.
        assert!(!state.is_filtering());
    }

    #[test]
    fn has_active_filter_returns_true_for_playlist() {
        // Given a state with applied playlist filter.
        let mut state = TuiState::new();
        state.playlist_pane.filter.applied = Some("test".to_string());

        // When checking for active filter.
        assert!(state.has_active_filter(Pane::Playlist));
    }

    #[test]
    fn has_active_filter_returns_true_for_directory() {
        // Given a state with applied directory filter.
        let mut state = TuiState::new();
        state.directory_pane.filter.applied = Some("test".to_string());

        // When checking for active filter.
        assert!(state.has_active_filter(Pane::Directory));
    }

    #[test]
    fn start_filter_activates_playlist_filter() {
        // Given a state focused on playlist.
        let mut state = TuiState::new();
        state.focused_pane = Pane::Playlist;

        // When starting filter.
        state.start_filter();

        // Then playlist filter is active.
        assert!(state.playlist_pane.filter.is_active());
    }

    #[test]
    fn start_filter_activates_directory_filter() {
        // Given a state focused on directory.
        let mut state = TuiState::new();
        state.focused_pane = Pane::Directory;

        // When starting filter.
        state.start_filter();

        // Then directory filter is active.
        assert!(state.directory_pane.filter.is_active());
    }

    #[test]
    fn cancel_filter_deactivates_filter() {
        // Given a state with active filter.
        let mut state = TuiState::new();
        state.focused_pane = Pane::Playlist;
        state.playlist_pane.filter.active = true;

        // When canceling filter.
        state.cancel_filter();

        // Then filter is inactive.
        assert!(!state.playlist_pane.filter.is_active());
    }

    #[test]
    fn submit_filter_applies_filter() {
        // Given a state with active filter and input.
        let mut state = TuiState::new();
        state.focused_pane = Pane::Playlist;
        state.playlist_pane.filter.active = true;
        state.playlist_pane.filter.input = "test".to_string();

        // When submitting filter.
        state.submit_filter();

        // Then filter is applied.
        assert_eq!(state.playlist_pane.filter.applied, Some("test".to_string()));
        assert!(!state.playlist_pane.filter.is_active());
    }

    #[test]
    fn show_error_activates_error_popup() {
        // Given a state.
        let mut state = TuiState::new();

        // When showing error.
        state.show_error("Test error".to_string());

        // Then error popup is active.
        assert!(state.is_showing_error());
        assert_eq!(state.error_popup.message, "Test error");
    }

    #[test]
    fn dismiss_error_deactivates_error_popup() {
        // Given a state with active error.
        let mut state = TuiState::new();
        state.show_error("Test error".to_string());

        // When dismissing error.
        state.dismiss_error();

        // Then error popup is inactive.
        assert!(!state.is_showing_error());
    }

    #[test]
    fn get_filtered_playlist_delegates_to_pane() {
        // Given a state with playlist items.
        let mut state = TuiState::new();
        state.playlist_pane.items = vec![item("apple.mp4"), item("banana.mp4")];
        state.playlist_pane.filter.applied = Some("an".to_string());

        // When getting filtered playlist.
        let filtered = state.get_filtered_playlist();

        // Then filtered results are returned.
        assert_eq!(filtered.len(), 1);
    }

    #[test]
    fn get_filtered_directory_delegates_to_pane() {
        // Given a state with directory items.
        let mut state = TuiState::new();
        state.directory_pane.items = vec![item("apple.mp4"), item("banana.mp4")];
        state.directory_pane.filter.applied = Some("an".to_string());

        // When getting filtered directory.
        let filtered = state.get_filtered_directory();

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
