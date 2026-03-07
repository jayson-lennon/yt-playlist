use std::path::PathBuf;

use crate::ui::{DirectoryPane, ErrorPopup, Pane, PlaylistItem, PlaylistPane, Rename, WhichKey};

pub struct TuiState {
    pub playlist_pane: PlaylistPane,
    pub directory_pane: DirectoryPane,
    pub focused_pane: Pane,
    pub status_message: Option<String>,
    pub rename: Rename,
    pub which_key: WhichKey,
    pub needs_clear: bool,
    pub error_popup: ErrorPopup,
    pub pending_key: Option<char>,
}

impl TuiState {
    pub fn new() -> Self {
        Self {
            playlist_pane: PlaylistPane::new(),
            directory_pane: DirectoryPane::new(),
            focused_pane: Pane::Playlist,
            status_message: None,
            rename: Rename::new(),
            which_key: WhichKey::default(),
            needs_clear: false,
            error_popup: ErrorPopup::new(),
            pending_key: None,
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
    ) {
        self.playlist_pane.add(PlaylistItem {
            path,
            duration,
            alias,
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
