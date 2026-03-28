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

use crossterm::event::KeyEvent;

use ratatui::{
    layout::Rect,
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};

use super::common::{
    filter_items, focused_border_style, focused_text_style, format_duration, format_item_line,
    item_style, pane_title, split_pane_layout, total_duration, ItemDisplayMode, ItemPath, Pane,
    PlaylistItem,
};
use super::component::Component;
use super::event::HandleKeyResult;
use super::filter::Filter;
use super::render::{Render, RenderContext};

const BORDER_COLUMNS: u16 = 2; // Left + right borders

/// The playlist pane showing queued media files.
///
/// Displays the current playlist with items in order, supporting reordering,
/// filtering, selection navigation, and moving items back to the library.
/// The playlist order determines the sequence for playback in mpv.
#[derive(Debug, Clone)]
pub struct PlaylistPane {
    /// Items in the current playlist, ordered for playback.
    pub items: Vec<PlaylistItem>,
    /// Index of the currently selected item in the list.
    pub selected: usize,
    /// Filter state for searching and narrowing the displayed items.
    pub filter: Filter,
}

impl PlaylistPane {
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            selected: 0,
            filter: Filter::new(),
        }
    }

    pub fn move_up(&mut self) {
        if self.filter.is_active() || self.filter.has_applied() {
            let filtered = self.get_filtered();
            if self.selected > 0 && !filtered.is_empty() {
                self.selected -= 1;
            }
        } else if self.selected > 0 {
            self.selected -= 1;
        }
    }

    pub fn move_down(&mut self) {
        if self.filter.is_active() || self.filter.has_applied() {
            let filtered = self.get_filtered();
            if !filtered.is_empty() && self.selected < filtered.len() - 1 {
                self.selected += 1;
            }
        } else if !self.items.is_empty() && self.selected < self.items.len() - 1 {
            self.selected += 1;
        }
    }

    pub fn reorder_up(&mut self) {
        if self.selected > 0 {
            self.items.swap(self.selected, self.selected - 1);
            self.selected -= 1;
        }
    }

    pub fn reorder_down(&mut self) {
        if !self.items.is_empty() && self.selected < self.items.len() - 1 {
            self.items.swap(self.selected, self.selected + 1);
            self.selected += 1;
        }
    }

    pub fn add(&mut self, item: PlaylistItem) {
        if !self.items.iter().any(|i| i.path == item.path) {
            self.items.push(item);
        }
    }

    pub fn remove(&mut self) {
        let original_idx = if self.filter.is_active() || self.filter.has_applied() {
            let filtered = self.get_filtered();
            if let Some((idx, _)) = filtered.get(self.selected) {
                Some(*idx)
            } else {
                None
            }
        } else if self.selected < self.items.len() {
            Some(self.selected)
        } else {
            None
        };

        if let Some(idx) = original_idx {
            self.items.remove(idx);
            if self.selected >= self.items.len() && !self.items.is_empty() {
                self.selected = self.items.len() - 1;
            }
        }
    }

    pub fn selected_item(&self) -> Option<&PlaylistItem> {
        if self.filter.is_active() || self.filter.has_applied() {
            let filtered = self.get_filtered();
            filtered.get(self.selected).map(|(_, item)| *item)
        } else {
            self.items.get(self.selected)
        }
    }

    pub fn selected_item_mut(&mut self) -> Option<&mut PlaylistItem> {
        let original_idx = if self.filter.is_active() || self.filter.has_applied() {
            let filtered = self.get_filtered();
            filtered.get(self.selected).map(|(idx, _)| *idx)
        } else {
            Some(self.selected)
        };

        original_idx.and_then(|idx| self.items.get_mut(idx))
    }

    pub fn get_filtered(&self) -> Vec<(usize, &PlaylistItem)> {
        filter_items(
            &self.items,
            self.filter.input(),
            self.filter
                .applied()
                .map(std::string::ToString::to_string)
                .as_ref(),
        )
    }

    pub fn filter(&self) -> &Filter {
        &self.filter
    }

    pub fn filter_mut(&mut self) -> &mut Filter {
        &mut self.filter
    }

    pub fn paths(&self) -> Vec<&ItemPath> {
        self.items.iter().map(|item| &item.path).collect()
    }

    fn build_list_items(
        &self,
        filtered: &[(usize, &PlaylistItem)],
        list_area: Rect,
        is_focused: bool,
        is_filtering: bool,
        display_mode: ItemDisplayMode,
    ) -> Vec<ListItem<'_>> {
        filtered
            .iter()
            .enumerate()
            .map(|(display_idx, (_original_idx, item))| {
                let is_selected = display_idx == self.selected && is_focused && !is_filtering;
                let file_missing =
                    !item.path.as_file().is_some_and(|p| p.as_path().exists()) && !item.is_virtual;
                let style = item_style(is_selected, file_missing, item.has_sources);
                let text = format_item_line(
                    item,
                    display_mode,
                    list_area.width.saturating_sub(BORDER_COLUMNS),
                    item.playlist_count,
                    2, // Different threshold for playlist
                );
                ListItem::new(text).style(style)
            })
            .collect()
    }

    fn render_footer(
        &self,
        frame: &mut Frame,
        area: Rect,
        is_filtering: bool,
        is_focused: bool,
        total_duration: std::time::Duration,
    ) {
        if is_filtering {
            self.filter.render(frame, area);
        } else {
            let total_str = format_duration(Some(total_duration));
            let footer =
                Paragraph::new(format!("Total: {total_str}")).style(focused_text_style(is_focused));
            frame.render_widget(footer, area);
        }
    }

    pub fn render(
        &self,
        frame: &mut Frame,
        area: Rect,
        is_focused: bool,
        display_mode: ItemDisplayMode,
    ) {
        let is_filtering = self.filter.is_active();
        let has_filter = self.filter.has_applied();

        let (list_area, footer_area) = split_pane_layout(area);

        let filtered = self.get_filtered();
        let duration = total_duration(filtered.iter().map(|(_, item)| *item));

        let items =
            self.build_list_items(&filtered, list_area, is_focused, is_filtering, display_mode);

        let title = pane_title("Playlist", has_filter, is_focused);

        let list = List::new(items).block(
            Block::default()
                .title(title)
                .borders(Borders::TOP | Borders::LEFT | Borders::RIGHT)
                .border_style(focused_border_style(is_focused)),
        );

        let mut list_state = ListState::default();
        if !is_filtering && !filtered.is_empty() && self.selected < filtered.len() {
            list_state.select(Some(self.selected));
        }
        frame.render_stateful_widget(list, list_area, &mut list_state);

        self.render_footer(frame, footer_area, is_filtering, is_focused, duration);
    }
}

impl Render for PlaylistPane {
    fn should_render(&self, _ctx: &RenderContext<'_, '_>) -> bool {
        true
    }

    fn render(&self, ctx: &mut RenderContext<'_, '_>) {
        let is_focused = ctx.tui_state.focused_pane == Pane::Playlist;
        let display_mode = ctx.tui_state.display_mode;
        self.render(ctx.frame, ctx.area, is_focused, display_mode);
    }
}

impl Default for PlaylistPane {
    fn default() -> Self {
        Self::new()
    }
}

impl Component for PlaylistPane {
    fn is_active(&self) -> bool {
        true
    }

    fn handle_key(&mut self, key: KeyEvent) -> HandleKeyResult {
        if self.filter.is_active() {
            return self.filter.handle_key(key);
        }
        HandleKeyResult::ignored()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::KeyCode;
    use marked_path::CanonicalPath;
    use tempfile::NamedTempFile;

    struct TestFiles {
        files: Vec<NamedTempFile>,
    }

    impl TestFiles {
        fn new() -> Self {
            Self { files: Vec::new() }
        }

        fn create_item(&mut self) -> PlaylistItem {
            let temp = NamedTempFile::new().unwrap();
            let path = CanonicalPath::from_path(temp.path()).unwrap();
            let item = PlaylistItem {
                path: ItemPath::File(path),
                duration: None,
                alias: None,
                mime_type: None,
                is_virtual: false,
                playlist_count: 0,
                has_sources: true,
            };
            self.files.push(temp);
            item
        }

        fn create_item_with_alias(&mut self, alias: &str) -> PlaylistItem {
            let mut item = self.create_item();
            item.alias = Some(alias.to_string());
            item
        }

        fn create_items(&mut self, count: usize) -> Vec<PlaylistItem> {
            (0..count).map(|_| self.create_item()).collect()
        }
    }

    #[test]
    fn new_creates_empty_pane() {
        // Given a new playlist pane.
        let pane = PlaylistPane::new();

        // Then it is empty with selection at 0.
        assert!(pane.items.is_empty());
        assert_eq!(pane.selected, 0);
    }

    #[test]
    fn move_up_decrements_selection() {
        // Given a pane with 3 items, selection at index 1.
        let mut files = TestFiles::new();
        let mut pane = PlaylistPane::new();
        pane.items = files.create_items(3);
        pane.selected = 1;

        // When moving up.
        pane.move_up();

        // Then selection decrements.
        assert_eq!(pane.selected, 0);
    }

    #[test]
    fn move_up_stays_at_first_item() {
        // Given a pane with selection at first item.
        let mut files = TestFiles::new();
        let mut pane = PlaylistPane::new();
        pane.items = files.create_items(2);
        pane.selected = 0;

        // When moving up.
        pane.move_up();

        // Then selection stays at 0.
        assert_eq!(pane.selected, 0);
    }

    #[test]
    fn move_down_increments_selection() {
        // Given a pane with 3 items, selection at index 0.
        let mut files = TestFiles::new();
        let mut pane = PlaylistPane::new();
        pane.items = files.create_items(3);
        pane.selected = 0;

        // When moving down.
        pane.move_down();

        // Then selection increments.
        assert_eq!(pane.selected, 1);
    }

    #[test]
    fn move_down_stays_at_last_item() {
        // Given a pane with selection at last item.
        let mut files = TestFiles::new();
        let mut pane = PlaylistPane::new();
        pane.items = files.create_items(2);
        pane.selected = 1;

        // When moving down.
        pane.move_down();

        // Then selection stays at last index.
        assert_eq!(pane.selected, 1);
    }

    #[test]
    fn move_up_with_filter_navigates_filtered_list() {
        // Given a pane with filter applied.
        let mut files = TestFiles::new();
        let mut pane = PlaylistPane::new();
        pane.items = vec![
            files.create_item_with_alias("apple"),
            files.create_item_with_alias("banana"),
            files.create_item_with_alias("apricot"),
        ];
        pane.filter.applied = Some("ap".to_string());
        pane.selected = 1;

        // When moving up.
        pane.move_up();

        // Then selection moves within filtered results.
        assert_eq!(pane.selected, 0);
    }

    #[test]
    fn move_down_with_filter_navigates_filtered_list() {
        // Given a pane with filter applied.
        let mut files = TestFiles::new();
        let mut pane = PlaylistPane::new();
        pane.items = vec![
            files.create_item_with_alias("apple"),
            files.create_item_with_alias("banana"),
            files.create_item_with_alias("apricot"),
        ];
        pane.filter.applied = Some("ap".to_string());
        pane.selected = 0;

        // When moving down.
        pane.move_down();

        // Then selection moves within filtered results.
        assert_eq!(pane.selected, 1);
    }

    #[test]
    fn reorder_up_swaps_with_previous_item() {
        // Given a pane with 3 items, middle selected.
        let mut files = TestFiles::new();
        let mut pane = PlaylistPane::new();
        let item_a = files.create_item();
        let item_b = files.create_item();
        let item_c = files.create_item();
        pane.items = vec![item_a.clone(), item_b.clone(), item_c];
        pane.selected = 1;

        // When reordering up.
        pane.reorder_up();

        // Then items are swapped and selection follows.
        assert_eq!(pane.selected, 0);
        assert_eq!(pane.items[0].path, item_b.path);
        assert_eq!(pane.items[1].path, item_a.path);
    }

    #[test]
    fn reorder_up_does_nothing_at_first_item() {
        // Given a pane with first item selected.
        let mut files = TestFiles::new();
        let mut pane = PlaylistPane::new();
        let item_a = files.create_item();
        let item_b = files.create_item();
        pane.items = vec![item_a.clone(), item_b];
        pane.selected = 0;

        // When reordering up.
        pane.reorder_up();

        // Then nothing changes.
        assert_eq!(pane.selected, 0);
        assert_eq!(pane.items[0].path, item_a.path);
    }

    #[test]
    fn reorder_down_swaps_with_next_item() {
        // Given a pane with 3 items, first selected.
        let mut files = TestFiles::new();
        let mut pane = PlaylistPane::new();
        let item_a = files.create_item();
        let item_b = files.create_item();
        let item_c = files.create_item();
        pane.items = vec![item_a.clone(), item_b.clone(), item_c];
        pane.selected = 0;

        // When reordering down.
        pane.reorder_down();

        // Then items are swapped and selection follows.
        assert_eq!(pane.selected, 1);
        assert_eq!(pane.items[0].path, item_b.path);
        assert_eq!(pane.items[1].path, item_a.path);
    }

    #[test]
    fn reorder_down_does_nothing_at_last_item() {
        // Given a pane with last item selected.
        let mut files = TestFiles::new();
        let mut pane = PlaylistPane::new();
        let item_a = files.create_item();
        let item_b = files.create_item();
        pane.items = vec![item_a, item_b.clone()];
        pane.selected = 1;

        // When reordering down.
        pane.reorder_down();

        // Then nothing changes.
        assert_eq!(pane.selected, 1);
        assert_eq!(pane.items[1].path, item_b.path);
    }

    #[test]
    fn add_appends_new_item() {
        // Given an empty pane.
        let mut files = TestFiles::new();
        let mut pane = PlaylistPane::new();

        // When adding an item.
        let item = files.create_item();
        let item_path = item.path.clone();
        pane.add(item);

        // Then item is added.
        assert_eq!(pane.items.len(), 1);
        assert_eq!(pane.items[0].path, item_path);
    }

    #[test]
    fn add_prevents_duplicates() {
        // Given a pane with one item.
        let mut files = TestFiles::new();
        let mut pane = PlaylistPane::new();
        let item = files.create_item();
        pane.add(item.clone());

        // When adding the same item again.
        pane.add(item);

        // Then item is not duplicated.
        assert_eq!(pane.items.len(), 1);
    }

    #[test]
    fn add_allows_different_paths() {
        // Given a pane with one item.
        let mut files = TestFiles::new();
        let mut pane = PlaylistPane::new();
        pane.add(files.create_item());

        // When adding a different item.
        pane.add(files.create_item());

        // Then both items exist.
        assert_eq!(pane.items.len(), 2);
    }

    #[test]
    fn remove_deletes_selected_item() {
        // Given a pane with 3 items, middle selected.
        let mut files = TestFiles::new();
        let mut pane = PlaylistPane::new();
        let item_a = files.create_item();
        let item_b = files.create_item();
        let item_c = files.create_item();
        pane.items = vec![item_a.clone(), item_b, item_c.clone()];
        pane.selected = 1;

        // When removing.
        pane.remove();

        // Then selected item is removed.
        assert_eq!(pane.items.len(), 2);
        assert_eq!(pane.items[0].path, item_a.path);
        assert_eq!(pane.items[1].path, item_c.path);
    }

    #[test]
    fn remove_adjusts_selection_when_at_end() {
        // Given a pane with 2 items, last selected.
        let mut files = TestFiles::new();
        let mut pane = PlaylistPane::new();
        pane.items = files.create_items(2);
        pane.selected = 1;

        // When removing.
        pane.remove();

        // Then selection is adjusted.
        assert_eq!(pane.selected, 0);
    }

    #[test]
    fn remove_with_filter_removes_correct_item() {
        // Given a pane with filter applied.
        let mut files = TestFiles::new();
        let mut pane = PlaylistPane::new();
        let item_apple = files.create_item_with_alias("apple");
        let item_banana = files.create_item_with_alias("banana");
        let item_apricot = files.create_item_with_alias("apricot");
        pane.items = vec![item_apple, item_banana, item_apricot.clone()];
        pane.filter.applied = Some("ap".to_string());
        pane.selected = 1;

        // When removing (should remove apricot, index 2 in original).
        pane.remove();

        // Then correct item is removed.
        assert_eq!(pane.items.len(), 2);
        assert!(!pane.items.iter().any(|i| i.path == item_apricot.path));
    }

    #[test]
    fn selected_item_returns_current_selection() {
        // Given a pane with items.
        let mut files = TestFiles::new();
        let mut pane = PlaylistPane::new();
        let item_a = files.create_item();
        let item_b = files.create_item();
        pane.items = vec![item_a, item_b.clone()];
        pane.selected = 1;

        // When getting selected item.
        let selected = pane.selected_item();

        // Then correct item is returned.
        assert_eq!(selected.unwrap().path, item_b.path);
    }

    #[test]
    fn selected_item_returns_none_when_empty() {
        // Given an empty pane.
        let pane = PlaylistPane::new();

        // When getting selected item.
        let selected = pane.selected_item();

        // Then none is returned.
        assert!(selected.is_none());
    }

    #[test]
    fn selected_item_with_filter_returns_filtered_item() {
        // Given a pane with filter applied.
        let mut files = TestFiles::new();
        let mut pane = PlaylistPane::new();
        pane.items = vec![
            files.create_item_with_alias("apple"),
            files.create_item_with_alias("banana"),
            files.create_item_with_alias("apricot"),
        ];
        pane.filter.applied = Some("ap".to_string());
        pane.selected = 0;

        // When getting selected item.
        let selected = pane.selected_item();

        // Then first filtered item is returned (apple or apricot depending on score).
        assert!(selected.is_some());
    }

    #[test]
    fn selected_item_mut_allows_modification() {
        // Given a pane with items.
        let mut files = TestFiles::new();
        let mut pane = PlaylistPane::new();
        pane.items = vec![files.create_item()];

        // When modifying selected item.
        if let Some(item) = pane.selected_item_mut() {
            item.alias = Some("My Alias".to_string());
        }

        // Then item is modified.
        assert_eq!(pane.items[0].alias, Some("My Alias".to_string()));
    }

    #[test]
    fn paths_returns_all_paths() {
        // Given a pane with items.
        let mut files = TestFiles::new();
        let mut pane = PlaylistPane::new();
        let item_a = files.create_item();
        let item_b = files.create_item();
        pane.items = vec![item_a.clone(), item_b.clone()];

        // When getting paths.
        let paths = pane.paths();

        // Then all paths are returned.
        assert_eq!(paths.len(), 2);
        assert_eq!(*paths[0], item_a.path);
        assert_eq!(*paths[1], item_b.path);
    }

    #[test]
    fn get_filtered_returns_all_when_no_filter() {
        // Given a pane without filter.
        let mut files = TestFiles::new();
        let mut pane = PlaylistPane::new();
        pane.items = files.create_items(2);

        // When getting filtered.
        let filtered = pane.get_filtered();

        // Then all items are returned.
        assert_eq!(filtered.len(), 2);
    }

    #[test]
    fn get_filtered_filters_by_pattern() {
        // Given a pane with filter applied.
        let mut files = TestFiles::new();
        let mut pane = PlaylistPane::new();
        pane.items = vec![
            files.create_item_with_alias("apple"),
            files.create_item_with_alias("banana"),
            files.create_item_with_alias("cherry"),
        ];
        pane.filter.applied = Some("an".to_string());

        // When getting filtered.
        let filtered = pane.get_filtered();

        // Then only matching items are returned.
        assert_eq!(filtered.len(), 1);
    }

    #[test]
    fn default_creates_empty_pane() {
        // Given a default pane.
        let pane = PlaylistPane::default();

        // Then it is empty.
        assert!(pane.items.is_empty());
    }

    #[test]
    fn handle_key_delegates_to_filter_when_active() {
        // Given a pane with active filter.
        let mut pane = PlaylistPane::new();
        pane.filter.start();
        pane.filter.push_char('t');

        // When handling a character key.
        let result = pane.handle_key(KeyEvent::from(KeyCode::Char('e')));

        // Then the filter handles it.
        assert!(result.is_consumed());
        assert_eq!(pane.filter.input(), "te");
    }

    #[test]
    fn handle_key_returns_ignored_for_unhandled() {
        // Given a pane.
        let mut files = TestFiles::new();
        let mut pane = PlaylistPane::new();
        pane.items = vec![files.create_item()];

        // When pressing an unhandled key.
        let result = pane.handle_key(KeyEvent::from(KeyCode::Char('x')));

        // Then the event is ignored.
        assert!(!result.is_consumed());
    }
}
