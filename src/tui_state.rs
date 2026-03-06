use std::{path::PathBuf, time::Duration};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pane {
    Playlist,
    Directory,
}

#[derive(Debug, Clone)]
pub struct PlaylistItem {
    pub path: PathBuf,
    pub duration: Option<Duration>,
}

#[derive(Debug, Clone)]
pub struct TuiState {
    pub playlist: Vec<PlaylistItem>,
    pub directory: Vec<PlaylistItem>,
    pub playlist_selected: usize,
    pub directory_selected: usize,
    pub focused_pane: Pane,
    pub status_message: Option<String>,
}

impl TuiState {
    pub fn new() -> Self {
        Self {
            playlist: Vec::new(),
            directory: Vec::new(),
            playlist_selected: 0,
            directory_selected: 0,
            focused_pane: Pane::Playlist,
            status_message: None,
        }
    }

    pub fn selected_playlist_item(&self) -> Option<&PlaylistItem> {
        self.playlist.get(self.playlist_selected)
    }

    pub fn selected_directory_item(&self) -> Option<&PlaylistItem> {
        self.directory.get(self.directory_selected)
    }

    pub fn move_playlist_up(&mut self) {
        if self.playlist_selected > 0 {
            self.playlist_selected -= 1;
        }
    }

    pub fn move_playlist_down(&mut self) {
        if !self.playlist.is_empty() && self.playlist_selected < self.playlist.len() - 1 {
            self.playlist_selected += 1;
        }
    }

    pub fn move_directory_up(&mut self) {
        if self.directory_selected > 0 {
            self.directory_selected -= 1;
        }
    }

    pub fn move_directory_down(&mut self) {
        if !self.directory.is_empty() && self.directory_selected < self.directory.len() - 1 {
            self.directory_selected += 1;
        }
    }

    pub fn reorder_playlist_up(&mut self) {
        if self.playlist_selected > 0 {
            self.playlist
                .swap(self.playlist_selected, self.playlist_selected - 1);
            self.playlist_selected -= 1;
        }
    }

    pub fn reorder_playlist_down(&mut self) {
        if !self.playlist.is_empty() && self.playlist_selected < self.playlist.len() - 1 {
            self.playlist
                .swap(self.playlist_selected, self.playlist_selected + 1);
            self.playlist_selected += 1;
        }
    }

    pub fn add_to_playlist(&mut self, path: PathBuf, duration: Option<Duration>) {
        if !self.playlist.iter().any(|item| item.path == path) {
            self.playlist.push(PlaylistItem { path, duration });
        }
    }

    pub fn remove_from_playlist(&mut self) {
        if self.playlist_selected < self.playlist.len() {
            self.playlist.remove(self.playlist_selected);
            if self.playlist_selected >= self.playlist.len() && !self.playlist.is_empty() {
                self.playlist_selected = self.playlist.len() - 1;
            }
        }
    }

    pub fn refresh_directory(&mut self, entries: Vec<PlaylistItem>) {
        let playlist_paths: Vec<_> = self.playlist.iter().map(|item| &item.path).collect();
        self.directory = entries
            .into_iter()
            .filter(|p| !playlist_paths.contains(&&p.path))
            .collect();
        if self.directory.is_empty() {
            self.directory_selected = 0;
        } else if self.directory_selected >= self.directory.len() {
            self.directory_selected = self.directory.len() - 1;
        }
    }

    pub fn switch_pane(&mut self) {
        self.focused_pane = match self.focused_pane {
            Pane::Playlist => Pane::Directory,
            Pane::Directory => Pane::Playlist,
        };
    }
}

impl Default for TuiState {
    fn default() -> Self {
        Self::new()
    }
}
