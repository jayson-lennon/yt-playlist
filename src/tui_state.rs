use std::{path::PathBuf, time::Duration};

use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pane {
    Playlist,
    Directory,
}

#[derive(Debug, Clone)]
pub struct PlaylistItem {
    pub path: PathBuf,
    pub duration: Option<Duration>,
    pub alias: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct RenameState {
    pub active: bool,
    pub input: String,
}

#[derive(Debug, Clone, Default)]
pub struct FilterState {
    pub active: bool,
    pub input: String,
    pub applied: Option<String>,
    pub previous: Option<String>,
}

#[derive(Debug, Clone)]
pub struct TuiState {
    pub playlist: Vec<PlaylistItem>,
    pub directory: Vec<PlaylistItem>,
    pub playlist_selected: usize,
    pub directory_selected: usize,
    pub focused_pane: Pane,
    pub status_message: Option<String>,
    pub rename: RenameState,
    pub playlist_filter: FilterState,
    pub directory_filter: FilterState,
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
            rename: RenameState::default(),
            playlist_filter: FilterState::default(),
            directory_filter: FilterState::default(),
        }
    }

    pub fn selected_playlist_item(&self) -> Option<&PlaylistItem> {
        if self.has_active_filter(Pane::Playlist) {
            let filtered = self.get_filtered_playlist();
            filtered.get(self.playlist_selected).map(|(_, item)| *item)
        } else {
            self.playlist.get(self.playlist_selected)
        }
    }

    pub fn selected_directory_item(&self) -> Option<&PlaylistItem> {
        if self.has_active_filter(Pane::Directory) {
            let filtered = self.get_filtered_directory();
            filtered.get(self.directory_selected).map(|(_, item)| *item)
        } else {
            self.directory.get(self.directory_selected)
        }
    }

    pub fn move_playlist_up(&mut self) {
        if self.has_active_filter(Pane::Playlist) {
            if self.playlist_selected > 0 {
                self.playlist_selected -= 1;
            }
        } else if self.playlist_selected > 0 {
            self.playlist_selected -= 1;
        }
    }

    pub fn move_playlist_down(&mut self) {
        if self.has_active_filter(Pane::Playlist) {
            let filtered = self.get_filtered_playlist();
            if !filtered.is_empty() && self.playlist_selected < filtered.len() - 1 {
                self.playlist_selected += 1;
            }
        } else if !self.playlist.is_empty() && self.playlist_selected < self.playlist.len() - 1 {
            self.playlist_selected += 1;
        }
    }

    pub fn move_directory_up(&mut self) {
        if self.has_active_filter(Pane::Directory) {
            if self.directory_selected > 0 {
                self.directory_selected -= 1;
            }
        } else if self.directory_selected > 0 {
            self.directory_selected -= 1;
        }
    }

    pub fn move_directory_down(&mut self) {
        if self.has_active_filter(Pane::Directory) {
            let filtered = self.get_filtered_directory();
            if !filtered.is_empty() && self.directory_selected < filtered.len() - 1 {
                self.directory_selected += 1;
            }
        } else if !self.directory.is_empty() && self.directory_selected < self.directory.len() - 1 {
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

    pub fn add_to_playlist(
        &mut self,
        path: PathBuf,
        duration: Option<Duration>,
        alias: Option<String>,
    ) {
        if !self.playlist.iter().any(|item| item.path == path) {
            self.playlist.push(PlaylistItem {
                path,
                duration,
                alias,
            });
        }
    }

    pub fn remove_from_playlist(&mut self) {
        let original_idx = if self.has_active_filter(Pane::Playlist) {
            let filtered = self.get_filtered_playlist();
            if let Some((idx, _)) = filtered.get(self.playlist_selected) {
                Some(*idx)
            } else {
                None
            }
        } else if self.playlist_selected < self.playlist.len() {
            Some(self.playlist_selected)
        } else {
            None
        };

        if let Some(idx) = original_idx {
            self.playlist.remove(idx);
            if self.playlist_selected >= self.playlist.len() && !self.playlist.is_empty() {
                self.playlist_selected = self.playlist.len() - 1;
            }
        }
    }

    pub fn remove_from_directory(&mut self) {
        let original_idx = if self.has_active_filter(Pane::Directory) {
            let filtered = self.get_filtered_directory();
            if let Some((idx, _)) = filtered.get(self.directory_selected) {
                Some(*idx)
            } else {
                None
            }
        } else if self.directory_selected < self.directory.len() {
            Some(self.directory_selected)
        } else {
            None
        };

        if let Some(idx) = original_idx {
            self.directory.remove(idx);
            if self.directory_selected >= self.directory.len() && !self.directory.is_empty() {
                self.directory_selected = self.directory.len() - 1;
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

    pub fn is_renaming(&self) -> bool {
        self.rename.active
    }

    pub fn start_rename(&mut self) {
        let current_alias = self.get_selected_item().and_then(|item| item.alias.clone());
        self.rename.input = current_alias.unwrap_or_default();
        self.rename.active = true;
    }

    pub fn cancel_rename(&mut self) {
        self.rename.active = false;
        self.rename.input.clear();
    }

    pub fn submit_rename(&mut self) {
        let alias = if self.rename.input.is_empty() {
            None
        } else {
            Some(self.rename.input.clone())
        };
        if let Some(item) = self.get_selected_item_mut() {
            item.alias = alias;
        }
        self.rename.active = false;
        self.rename.input.clear();
    }

    pub fn push_rename_char(&mut self, c: char) {
        self.rename.input.push(c);
    }

    pub fn pop_rename_char(&mut self) {
        self.rename.input.pop();
    }

    pub fn get_selected_item(&self) -> Option<&PlaylistItem> {
        match self.focused_pane {
            Pane::Playlist => self.selected_playlist_item(),
            Pane::Directory => self.selected_directory_item(),
        }
    }

    pub fn get_selected_item_mut(&mut self) -> Option<&mut PlaylistItem> {
        let original_idx = match self.focused_pane {
            Pane::Playlist => {
                if self.has_active_filter(Pane::Playlist) {
                    let filtered = self.get_filtered_playlist();
                    filtered.get(self.playlist_selected).map(|(idx, _)| *idx)
                } else {
                    Some(self.playlist_selected)
                }
            }
            Pane::Directory => {
                if self.has_active_filter(Pane::Directory) {
                    let filtered = self.get_filtered_directory();
                    filtered.get(self.directory_selected).map(|(idx, _)| *idx)
                } else {
                    Some(self.directory_selected)
                }
            }
        };

        match self.focused_pane {
            Pane::Playlist => original_idx.and_then(|idx| self.playlist.get_mut(idx)),
            Pane::Directory => original_idx.and_then(|idx| self.directory.get_mut(idx)),
        }
    }

    pub fn is_filtering(&self) -> bool {
        match self.focused_pane {
            Pane::Playlist => self.playlist_filter.active,
            Pane::Directory => self.directory_filter.active,
        }
    }

    pub fn has_active_filter(&self, pane: Pane) -> bool {
        match pane {
            Pane::Playlist => self.playlist_filter.applied.is_some(),
            Pane::Directory => self.directory_filter.applied.is_some(),
        }
    }

    pub fn start_filter(&mut self) {
        let filter = match self.focused_pane {
            Pane::Playlist => &mut self.playlist_filter,
            Pane::Directory => &mut self.directory_filter,
        };
        filter.previous = filter.applied.take();
        filter.input.clear();
        filter.active = true;
    }

    pub fn cancel_filter(&mut self) {
        let filter = match self.focused_pane {
            Pane::Playlist => &mut self.playlist_filter,
            Pane::Directory => &mut self.directory_filter,
        };
        filter.applied = filter.previous.take();
        filter.input.clear();
        filter.active = false;
    }

    pub fn submit_filter(&mut self) {
        let filter = match self.focused_pane {
            Pane::Playlist => &mut self.playlist_filter,
            Pane::Directory => &mut self.directory_filter,
        };
        filter.applied = if filter.input.is_empty() {
            None
        } else {
            Some(filter.input.clone())
        };
        filter.previous = None;
        filter.input.clear();
        filter.active = false;
    }

    pub fn push_filter_char(&mut self, c: char) {
        let filter = match self.focused_pane {
            Pane::Playlist => &mut self.playlist_filter,
            Pane::Directory => &mut self.directory_filter,
        };
        filter.input.push(c);
    }

    pub fn pop_filter_char(&mut self) {
        let filter = match self.focused_pane {
            Pane::Playlist => &mut self.playlist_filter,
            Pane::Directory => &mut self.directory_filter,
        };
        filter.input.pop();
    }

    pub fn get_filter_input(&self, pane: Pane) -> &str {
        match pane {
            Pane::Playlist => &self.playlist_filter.input,
            Pane::Directory => &self.directory_filter.input,
        }
    }

    pub fn get_display_name(item: &PlaylistItem) -> String {
        item.alias.clone().unwrap_or_else(|| {
            item.path.file_name().map_or_else(
                || item.path.to_string_lossy().into_owned(),
                |n| n.to_string_lossy().into_owned(),
            )
        })
    }

    fn filter_items<'a>(
        items: &'a [PlaylistItem],
        filter_input: &str,
        applied_filter: Option<&String>,
    ) -> Vec<(usize, &'a PlaylistItem)> {
        let active_filter = if filter_input.is_empty() {
            applied_filter.map(String::as_str)
        } else {
            Some(filter_input)
        };

        match active_filter {
            None => items.iter().enumerate().collect(),
            Some(pattern) => {
                let matcher = SkimMatcherV2::default();
                let mut results: Vec<(i64, usize, &PlaylistItem)> = items
                    .iter()
                    .enumerate()
                    .filter_map(|(idx, item)| {
                        let name = Self::get_display_name(item);
                        matcher
                            .fuzzy_match(&name, pattern)
                            .map(|score| (score, idx, item))
                    })
                    .collect();
                results.sort_by(|a, b| b.0.cmp(&a.0));
                results
                    .into_iter()
                    .map(|(_, idx, item)| (idx, item))
                    .collect()
            }
        }
    }

    pub fn get_filtered_playlist(&self) -> Vec<(usize, &PlaylistItem)> {
        Self::filter_items(
            &self.playlist,
            &self.playlist_filter.input,
            self.playlist_filter.applied.as_ref(),
        )
    }

    pub fn get_filtered_directory(&self) -> Vec<(usize, &PlaylistItem)> {
        Self::filter_items(
            &self.directory,
            &self.directory_filter.input,
            self.directory_filter.applied.as_ref(),
        )
    }
}

impl Default for TuiState {
    fn default() -> Self {
        Self::new()
    }
}
