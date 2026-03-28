// Copyright (C) 2026 Jayson Lennon
// 
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as
// published by the Free Software Foundation, either version 3 of the
// License, or (at your option) any later version.
// 
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
// 
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

use std::collections::{HashMap, HashSet};

use crossterm::event::KeyEvent;
use derive_more::Debug;
use error_stack::Report;
use tokio::task::JoinHandle;
use wherror::Error;

use super::common::{ItemDisplayMode, ItemPath};
use super::component::{Component, ComponentContext};
use super::event::HandleKeyResult;
use super::{GlobalKeyHandler, StatusBar, WhichKeyConfig};
use crate::tui::{ErrorPopup, LibraryPane, Pane, PlaylistItem, PlaylistPane, Rename, UrlInput};

#[derive(Debug, Error)]
#[error("refresh failed")]
pub struct RefreshError;

/// Complete terminal UI state for the application.
///
/// Holds all mutable state for rendering and interacting with the TUI,
/// including both panes (playlist and library), the currently focused pane,
/// input modes (filtering, renaming, URL input), error display, and the
/// which-key help popup.
#[derive(Debug)]
pub struct TuiState {
    pub playlist_pane: PlaylistPane,
    pub library_pane: LibraryPane,
    pub focused_pane: Pane,
    pub status_bar: StatusBar,
    pub rename: Rename,
    pub url_input: UrlInput,
    pub global_handler: GlobalKeyHandler,
    pub needs_clear: bool,
    pub error_popup: ErrorPopup,
    pub display_mode: ItemDisplayMode,
    #[debug(skip)]
    refresh_task: Option<JoinHandle<Result<usize, Report<RefreshError>>>>,
}

impl TuiState {
    pub fn new() -> Self {
        Self {
            playlist_pane: PlaylistPane::new(),
            library_pane: LibraryPane::new(),
            focused_pane: Pane::Playlist,
            status_bar: StatusBar::new(),
            rename: Rename::new(),
            url_input: UrlInput::new(),
            global_handler: GlobalKeyHandler::new(WhichKeyConfig::default()),
            needs_clear: false,
            error_popup: ErrorPopup::new(),
            display_mode: ItemDisplayMode::default(),
            refresh_task: None,
        }
    }

    pub fn set_status(&mut self, message: impl Into<String>) {
        self.status_bar.set(message);
    }

    pub fn is_refreshing(&self) -> bool {
        self.refresh_task.as_ref().is_some_and(|h| !h.is_finished())
    }

    pub fn start_refresh(&mut self, handle: JoinHandle<Result<usize, Report<RefreshError>>>) {
        self.refresh_task = Some(handle);
    }

    pub fn take_refresh_result(
        &mut self,
        rt: &tokio::runtime::Runtime,
    ) -> Option<Result<usize, Report<RefreshError>>> {
        let handle = self.refresh_task.take()?;
        if handle.is_finished() {
            Some(
                rt.block_on(handle)
                    .unwrap_or_else(|e| Err(Report::new(RefreshError).attach(e))),
            )
        } else {
            self.refresh_task = Some(handle);
            None
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
        path: ItemPath,
        duration: Option<std::time::Duration>,
        alias: Option<String>,
        mime_type: Option<String>,
        is_virtual: bool,
        playlist_count: usize,
    ) {
        self.playlist_pane.add(PlaylistItem {
            path,
            duration,
            alias,
            mime_type,
            is_virtual,
            playlist_count,
            has_sources: true,
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

    /// Updates playlist occurrence counts for all items in both panes.
    ///
    /// This should be called after database operations that change playlist
    /// membership to ensure counts reflect the current state.
    pub fn update_counts(&mut self, counts: &HashMap<ItemPath, usize>) {
        for item in &mut self.playlist_pane.items {
            item.playlist_count = counts.get(&item.path).copied().unwrap_or(1);
        }
        for item in &mut self.library_pane.items {
            item.playlist_count = counts.get(&item.path).copied().unwrap_or(0);
        }
    }

    /// Updates the source availability status for all items in both panes.
    ///
    /// This should be called after scanning for source files to indicate which
    /// items have corresponding source files available. URLs and virtual items
    /// are always considered to have sources available.
    pub fn update_sources_status(&mut self, paths_with_sources: &HashSet<ItemPath>) {
        for item in &mut self.playlist_pane.items {
            item.has_sources = item.path.is_url() || item.is_virtual || paths_with_sources.contains(&item.path);
        }
        for item in &mut self.library_pane.items {
            item.has_sources = item.path.is_url() || item.is_virtual || paths_with_sources.contains(&item.path);
        }
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

    pub fn handle_key(&mut self, key: KeyEvent, ctx: &ComponentContext<'_>) -> HandleKeyResult {
        if self.error_popup.is_active() {
            return self.error_popup.handle_key(key);
        }

        if self.rename.is_active() {
            return self.rename.handle_key(key);
        }

        if self.url_input.is_active() {
            return self.url_input.handle_key(key);
        }

        let result = match self.focused_pane {
            Pane::Playlist => self.playlist_pane.handle_key(key),
            Pane::Library => self.library_pane.handle_key(key),
        };

        if result.is_consumed() {
            return result;
        }

        self.global_handler.handle_key_with_context(key, ctx)
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
    use marked_path::CanonicalPath;
    use std::time::Duration;
    use tempfile::NamedTempFile;

    fn create_temp_file() -> (NamedTempFile, CanonicalPath) {
        let temp = NamedTempFile::new().unwrap();
        let path = CanonicalPath::from_path(temp.path()).unwrap();
        (temp, path)
    }

    fn item(path: &CanonicalPath) -> PlaylistItem {
        PlaylistItem {
            path: ItemPath::File(path.clone()),
            duration: None,
            alias: None,
            mime_type: None,
            is_virtual: false,
            playlist_count: 0,
            has_sources: true,
        }
    }

    #[test]
    fn new_creates_default_state() {
        // Given a new state.
        let state = TuiState::new();

        // Then defaults are set.
        assert!(state.playlist_pane.items.is_empty());
        assert!(state.library_pane.items.is_empty());
        assert_eq!(state.focused_pane, Pane::Playlist);
        assert!(state.status_bar.message().is_none());
        assert!(!state.is_renaming());
        assert!(!state.is_showing_error());
    }

    #[test]
    fn selected_playlist_item_returns_current() {
        // Given a state with playlist items.
        let (_temp_a, path_a) = create_temp_file();
        let (_temp_b, path_b) = create_temp_file();
        let mut state = TuiState::new();
        state.playlist_pane.items = vec![item(&path_a), item(&path_b)];
        state.playlist_pane.selected = 1;

        // When getting selected item.
        let selected = state.selected_playlist_item();

        // Then correct item is returned.
        assert_eq!(selected.unwrap().path, ItemPath::File(path_b));
    }

    #[test]
    fn selected_library_item_returns_current() {
        // Given a state with library items.
        let (_temp_a, path_a) = create_temp_file();
        let (_temp_b, path_b) = create_temp_file();
        let mut state = TuiState::new();
        state.library_pane.items = vec![item(&path_a), item(&path_b)];
        state.library_pane.selected = 1;

        // When getting selected item.
        let selected = state.selected_library_item();

        // Then correct item is returned.
        assert_eq!(selected.unwrap().path, ItemPath::File(path_b));
    }

    #[test]
    fn move_playlist_up_delegates_to_pane() {
        // Given a state with playlist items.
        let (_temp_a, path_a) = create_temp_file();
        let (_temp_b, path_b) = create_temp_file();
        let mut state = TuiState::new();
        state.playlist_pane.items = vec![item(&path_a), item(&path_b)];
        state.playlist_pane.selected = 1;

        // When moving up.
        state.move_playlist_up();

        // Then selection changes.
        assert_eq!(state.playlist_pane.selected, 0);
    }

    #[test]
    fn move_playlist_down_delegates_to_pane() {
        // Given a state with playlist items.
        let (_temp_a, path_a) = create_temp_file();
        let (_temp_b, path_b) = create_temp_file();
        let mut state = TuiState::new();
        state.playlist_pane.items = vec![item(&path_a), item(&path_b)];
        state.playlist_pane.selected = 0;

        // When moving down.
        state.move_playlist_down();

        // Then selection changes.
        assert_eq!(state.playlist_pane.selected, 1);
    }

    #[test]
    fn move_library_up_delegates_to_pane() {
        // Given a state with library items.
        let (_temp_a, path_a) = create_temp_file();
        let (_temp_b, path_b) = create_temp_file();
        let mut state = TuiState::new();
        state.library_pane.items = vec![item(&path_a), item(&path_b)];
        state.library_pane.selected = 1;

        // When moving up.
        state.move_library_up();

        // Then selection changes.
        assert_eq!(state.library_pane.selected, 0);
    }

    #[test]
    fn move_library_down_delegates_to_pane() {
        // Given a state with library items.
        let (_temp_a, path_a) = create_temp_file();
        let (_temp_b, path_b) = create_temp_file();
        let mut state = TuiState::new();
        state.library_pane.items = vec![item(&path_a), item(&path_b)];
        state.library_pane.selected = 0;

        // When moving down.
        state.move_library_down();

        // Then selection changes.
        assert_eq!(state.library_pane.selected, 1);
    }

    #[test]
    fn reorder_playlist_up_delegates_to_pane() {
        // Given a state with playlist items.
        let (_temp_a, path_a) = create_temp_file();
        let (_temp_b, path_b) = create_temp_file();
        let mut state = TuiState::new();
        state.playlist_pane.items = vec![item(&path_a), item(&path_b)];
        state.playlist_pane.selected = 1;

        // When reordering up.
        state.reorder_playlist_up();

        // Then items are swapped.
        assert_eq!(state.playlist_pane.items[0].path, ItemPath::File(path_b));
    }

    #[test]
    fn reorder_playlist_down_delegates_to_pane() {
        // Given a state with playlist items.
        let (_temp_a, path_a) = create_temp_file();
        let (_temp_b, path_b) = create_temp_file();
        let mut state = TuiState::new();
        state.playlist_pane.items = vec![item(&path_a), item(&path_b)];
        state.playlist_pane.selected = 0;

        // When reordering down.
        state.reorder_playlist_down();

        // Then items are swapped.
        assert_eq!(state.playlist_pane.items[0].path, ItemPath::File(path_b));
    }

    #[test]
    fn add_to_playlist_adds_item() {
        // Given an empty state.
        let (_temp, path) = create_temp_file();
        let mut state = TuiState::new();

        // When adding to playlist.
        state.add_to_playlist(
            ItemPath::File(path.clone()),
            Some(Duration::from_secs(120)),
            Some("alias".to_string()),
            Some("video/mp4".to_string()),
            false,
            1,
        );

        // Then item is added.
        assert_eq!(state.playlist_pane.items.len(), 1);
        assert_eq!(state.playlist_pane.items[0].path, ItemPath::File(path));
        assert_eq!(
            state.playlist_pane.items[0].alias,
            Some("alias".to_string())
        );
    }

    #[test]
    fn remove_from_playlist_removes_item() {
        // Given a state with playlist items.
        let (_temp_a, path_a) = create_temp_file();
        let (_temp_b, path_b) = create_temp_file();
        let mut state = TuiState::new();
        state.playlist_pane.items = vec![item(&path_a), item(&path_b)];
        state.playlist_pane.selected = 0;

        // When removing from playlist.
        state.remove_from_playlist();

        // Then item is removed.
        assert_eq!(state.playlist_pane.items.len(), 1);
    }

    #[test]
    fn remove_from_library_removes_item() {
        // Given a state with library items.
        let (_temp_a, path_a) = create_temp_file();
        let (_temp_b, path_b) = create_temp_file();
        let mut state = TuiState::new();
        state.library_pane.items = vec![item(&path_a), item(&path_b)];
        state.library_pane.selected = 0;

        // When removing from library.
        state.remove_from_library();

        // Then item is removed.
        assert_eq!(state.library_pane.items.len(), 1);
    }

    #[test]
    fn default_creates_default_state() {
        // Given a default state.
        let state = TuiState::default();

        // Then defaults are set.
        assert_eq!(state.focused_pane, Pane::Playlist);
    }

    fn ctx() -> ComponentContext<'static> {
        use crate::feat::keymap::Keymap;
        static KEYMAP: std::sync::OnceLock<Keymap> = std::sync::OnceLock::new();
        let keymap = KEYMAP.get_or_init(Keymap::new);
        ComponentContext {
            keymap,
            focused_pane: Pane::Playlist,
        }
    }

    #[test]
    fn handle_key_error_popup_blocks_everything() {
        // Given a state with an active error popup.
        let mut state = TuiState::new();
        state.error_popup.show("Error message".to_string());
        let ctx = ctx();

        // When handling any key.
        let key = crossterm::event::KeyEvent::from(crossterm::event::KeyCode::Char('a'));
        let result = state.handle_key(key, &ctx);

        // Then the error popup consumes it and dismisses.
        assert!(result.is_consumed());
        assert!(!state.error_popup.is_active());
    }

    #[test]
    fn handle_key_rename_consumes_when_active() {
        // Given a state with active rename mode.
        let mut state = TuiState::new();
        state.rename.start(Some("old name"));
        let ctx = ctx();

        // When handling a character key.
        let key = crossterm::event::KeyEvent::from(crossterm::event::KeyCode::Char('x'));
        let result = state.handle_key(key, &ctx);

        // Then rename consumes it.
        assert!(result.is_consumed());
        assert!(state.rename.input().contains('x'));
    }

    #[test]
    fn handle_key_url_input_consumes_when_active() {
        // Given a state with active url input mode.
        let mut state = TuiState::new();
        state.url_input.start();
        let ctx = ctx();

        // When handling a character key.
        let key = crossterm::event::KeyEvent::from(crossterm::event::KeyCode::Char('h'));
        let result = state.handle_key(key, &ctx);

        // Then url input consumes it.
        assert!(result.is_consumed());
        assert!(state.url_input.input().contains('h'));
    }

    #[test]
    fn handle_key_global_handler_consumes_prefix_key() {
        // Given a state with no active modes.
        let mut state = TuiState::new();
        let ctx = ctx();

        // When handling a prefix key (e.g., 'g').
        let key = crossterm::event::KeyEvent::from(crossterm::event::KeyCode::Char('g'));
        let result = state.handle_key(key, &ctx);

        // Then global handler consumes it (starts which-key sequence).
        assert!(result.is_consumed());
    }

    #[test]
    fn handle_key_returns_move_down_action_for_j() {
        use crate::tui::TuiAction;

        // Given a state focused on playlist with items.
        let (_temp_a, path_a) = create_temp_file();
        let (_temp_b, path_b) = create_temp_file();
        let mut state = TuiState::new();
        state.playlist_pane.items = vec![item(&path_a), item(&path_b)];
        state.focused_pane = Pane::Playlist;
        let ctx = ctx();

        // When handling 'j' key.
        let key = crossterm::event::KeyEvent::from(crossterm::event::KeyCode::Char('j'));
        let result = state.handle_key(key, &ctx);

        // Then the result contains MoveDown action.
        assert!(result.is_consumed());
        assert_eq!(result.actions.len(), 1);
        assert_eq!(result.actions[0], TuiAction::MoveDown);
    }

    #[test]
    fn handle_key_returns_ignored_when_no_component_handles() {
        // Given a default state with no active components.
        let mut state = TuiState::new();
        let ctx = ctx();

        // When handling an unhandled key.
        let key = crossterm::event::KeyEvent::from(crossterm::event::KeyCode::Char('z'));
        let result = state.handle_key(key, &ctx);

        // Then the event is ignored.
        assert!(!result.is_consumed());
    }

    #[test]
    fn is_refreshing_returns_false_initially() {
        // Given a new state.
        let state = TuiState::new();

        // Then is_refreshing returns false.
        assert!(!state.is_refreshing());
    }

    #[test]
    fn start_refresh_stores_handle() {
        // Given a state.
        let mut state = TuiState::new();
        let rt = tokio::runtime::Runtime::new().unwrap();
        let handle = rt.spawn(async {
            std::future::pending::<Result<usize, Report<RefreshError>>>().await
        });

        // When starting refresh.
        state.start_refresh(handle);

        // Then is_refreshing returns true.
        assert!(state.is_refreshing());
    }

    #[test]
    fn take_refresh_result_returns_none_while_running() {
        // Given a state with a running task.
        let mut state = TuiState::new();
        let rt = tokio::runtime::Runtime::new().unwrap();
        let handle = rt.spawn(async {
            std::future::pending::<Result<usize, Report<RefreshError>>>().await
        });
        state.start_refresh(handle);

        // When taking refresh result.
        let result = state.take_refresh_result(&rt);

        // Then none is returned (task still running).
        assert!(result.is_none());
        assert!(state.is_refreshing());
    }

    #[test]
    fn take_refresh_result_returns_result_when_done() {
        // Given a state with a completed task.
        let mut state = TuiState::new();
        let rt = tokio::runtime::Runtime::new().unwrap();
        let handle = rt.spawn(async { Ok::<_, Report<RefreshError>>(42) });
        state.start_refresh(handle);

        // Wait for task to complete.
        std::thread::sleep(std::time::Duration::from_millis(10));

        // When taking refresh result.
        let result = state.take_refresh_result(&rt);

        // Then the result is returned.
        assert!(result.is_some());
        assert_eq!(result.unwrap().unwrap(), 42);
        assert!(!state.is_refreshing());
    }
}
