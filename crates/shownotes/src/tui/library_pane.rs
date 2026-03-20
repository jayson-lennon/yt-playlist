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

/// The library pane showing available media files.
///
/// Displays files from the library directory that are not currently in the playlist,
/// along with any virtual URL items that have been added. Supports filtering,
/// selection navigation, and moving items to the playlist.
#[derive(Debug, Clone)]
pub struct LibraryPane {
    /// Available media items in the library, excluding those already in the playlist.
    pub items: Vec<PlaylistItem>,
    /// Index of the currently selected item in the list.
    pub selected: usize,
    /// Filter state for searching and narrowing the displayed items.
    pub filter: Filter,
}

impl LibraryPane {
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

    pub fn refresh(&mut self, entries: Vec<PlaylistItem>, playlist_paths: &[&ItemPath]) {
        use std::collections::HashSet;

        let entry_paths: HashSet<_> = entries.iter().map(|e| &e.path).collect();

        self.items
            .retain(|item| item.is_virtual || entry_paths.contains(&item.path));

        let existing_paths: HashSet<_> = self.items.iter().map(|i| i.path.clone()).collect();
        for entry in entries {
            if !playlist_paths.contains(&&entry.path) && !existing_paths.contains(&entry.path) {
                self.items.push(entry);
            }
        }

        self.items
            .sort_by(|a, b| a.path.to_string_lossy().cmp(&b.path.to_string_lossy()));
        if self.items.is_empty() {
            self.selected = 0;
        } else if self.selected >= self.items.len() {
            self.selected = self.items.len() - 1;
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
                    1,
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

        let title = pane_title("Library", has_filter, is_focused);

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

impl Default for LibraryPane {
    fn default() -> Self {
        Self::new()
    }
}

impl Render for LibraryPane {
    fn should_render(&self, _ctx: &RenderContext<'_, '_>) -> bool {
        true
    }

    fn render(&self, ctx: &mut RenderContext<'_, '_>) {
        let is_focused = ctx.tui_state.focused_pane == Pane::Library;
        let display_mode = ctx.tui_state.display_mode;
        self.render(ctx.frame, ctx.area, is_focused, display_mode);
    }
}

impl Component for LibraryPane {
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
        paths: Vec<CanonicalPath>,
    }

    impl TestFiles {
        fn new() -> Self {
            Self {
                files: Vec::new(),
                paths: Vec::new(),
            }
        }

        fn create_item(&mut self) -> PlaylistItem {
            let temp = NamedTempFile::new().unwrap();
            let path = CanonicalPath::from_path(temp.path()).unwrap();
            let item = PlaylistItem {
                path: ItemPath::File(path.clone()),
                duration: None,
                alias: None,
                mime_type: None,
                is_virtual: false,
                playlist_count: 0,
                has_sources: true,
            };
            self.paths.push(path);
            self.files.push(temp);
            item
        }

        fn create_items(&mut self, count: usize) -> Vec<PlaylistItem> {
            (0..count).map(|_| self.create_item()).collect()
        }

        fn path(&self, index: usize) -> &CanonicalPath {
            &self.paths[index]
        }
    }

    fn virtual_item(path: &str) -> PlaylistItem {
        PlaylistItem {
            path: ItemPath::Url(path.to_string()),
            duration: None,
            alias: None,
            mime_type: None,
            is_virtual: true,
            playlist_count: 0,
            has_sources: true,
        }
    }

    #[test]
    fn new_creates_empty_pane() {
        // Given a new library pane.
        let pane = LibraryPane::new();

        // Then it is empty with selection at 0.
        assert!(pane.items.is_empty());
        assert_eq!(pane.selected, 0);
    }

    #[test]
    fn move_up_decrements_selection() {
        // Given a pane with 3 items, selection at index 1.
        let mut files = TestFiles::new();
        let mut pane = LibraryPane::new();
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
        let mut pane = LibraryPane::new();
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
        let mut pane = LibraryPane::new();
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
        let mut pane = LibraryPane::new();
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
        let mut pane = LibraryPane::new();
        pane.items = files.create_items(3);
        pane.items[0].alias = Some("apple".to_string());
        pane.items[1].alias = Some("banana".to_string());
        pane.items[2].alias = Some("grape".to_string());
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
        let mut pane = LibraryPane::new();
        pane.items = files.create_items(3);
        pane.items[0].alias = Some("apple".to_string());
        pane.items[1].alias = Some("banana".to_string());
        pane.items[2].alias = Some("grape".to_string());
        pane.filter.applied = Some("ap".to_string());
        pane.selected = 0;

        // When moving down.
        pane.move_down();

        // Then selection moves within filtered results.
        assert_eq!(pane.selected, 1);
    }

    #[test]
    fn remove_deletes_selected_item() {
        // Given a pane with 3 items, middle selected.
        let mut files = TestFiles::new();
        let mut pane = LibraryPane::new();
        pane.items = files.create_items(3);
        pane.selected = 1;

        // When removing.
        pane.remove();

        // Then selected item is removed.
        assert_eq!(pane.items.len(), 2);
        assert_eq!(pane.items[0].path, ItemPath::File(files.path(0).clone()));
        assert_eq!(pane.items[1].path, ItemPath::File(files.path(2).clone()));
    }

    #[test]
    fn remove_adjusts_selection_when_at_end() {
        // Given a pane with 2 items, last selected.
        let mut files = TestFiles::new();
        let mut pane = LibraryPane::new();
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
        let mut pane = LibraryPane::new();
        pane.items = files.create_items(3);
        pane.items[0].alias = Some("apple".to_string());
        pane.items[1].alias = Some("banana".to_string());
        pane.items[2].alias = Some("grape".to_string());
        pane.filter.applied = Some("ap".to_string());
        pane.selected = 1;

        // When removing (should remove item at index 2 in original).
        pane.remove();

        // Then correct item is removed.
        assert_eq!(pane.items.len(), 2);
        let removed_path = ItemPath::File(files.path(2).clone());
        assert!(pane.items.iter().all(|i| i.path != removed_path));
    }

    #[test]
    fn selected_item_returns_current_selection() {
        // Given a pane with items.
        let mut files = TestFiles::new();
        let mut pane = LibraryPane::new();
        pane.items = files.create_items(2);
        pane.selected = 1;

        // When getting selected item.
        let selected = pane.selected_item();

        // Then correct item is returned.
        assert_eq!(
            selected.unwrap().path,
            ItemPath::File(files.path(1).clone())
        );
    }

    #[test]
    fn selected_item_returns_none_when_empty() {
        // Given an empty pane.
        let pane = LibraryPane::new();

        // When getting selected item.
        let selected = pane.selected_item();

        // Then none is returned.
        assert!(selected.is_none());
    }

    #[test]
    fn selected_item_with_filter_returns_filtered_item() {
        // Given a pane with filter applied.
        let mut files = TestFiles::new();
        let mut pane = LibraryPane::new();
        pane.items = files.create_items(3);
        pane.items[0].alias = Some("apple".to_string());
        pane.items[1].alias = Some("banana".to_string());
        pane.items[2].alias = Some("grape".to_string());
        pane.filter.applied = Some("ap".to_string());
        pane.selected = 0;

        // When getting selected item.
        let selected = pane.selected_item();

        // Then first filtered item is returned.
        assert!(selected.is_some());
    }

    #[test]
    fn selected_item_mut_allows_modification() {
        // Given a pane with items.
        let mut files = TestFiles::new();
        let mut pane = LibraryPane::new();
        pane.items = files.create_items(1);

        // When modifying selected item.
        if let Some(item) = pane.selected_item_mut() {
            item.alias = Some("My Alias".to_string());
        }

        // Then item is modified.
        assert_eq!(pane.items[0].alias, Some("My Alias".to_string()));
    }

    #[test]
    fn refresh_resets_selection_when_empty() {
        // Given a pane.
        let mut pane = LibraryPane::new();
        pane.selected = 5;

        // When refreshing with empty entries.
        pane.refresh(vec![], &[]);

        // Then selection is reset to 0.
        assert_eq!(pane.selected, 0);
    }

    #[test]
    fn refresh_adjusts_selection_when_out_of_bounds() {
        // Given a pane with selection out of bounds.
        let mut files = TestFiles::new();
        let mut pane = LibraryPane::new();
        pane.selected = 10;

        // When refreshing with fewer items.
        pane.refresh(files.create_items(2), &[]);

        // Then selection is adjusted to last item.
        assert_eq!(pane.selected, 1);
    }

    #[test]
    fn refresh_keeps_selection_when_valid() {
        // Given a pane with valid selection.
        let mut files = TestFiles::new();
        let mut pane = LibraryPane::new();
        pane.selected = 1;

        // When refreshing with enough items.
        pane.refresh(files.create_items(3), &[]);

        // Then selection is preserved.
        assert_eq!(pane.selected, 1);
    }

    #[test]
    fn get_filtered_returns_all_when_no_filter() {
        // Given a pane without filter.
        let mut files = TestFiles::new();
        let mut pane = LibraryPane::new();
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
        let mut pane = LibraryPane::new();
        pane.items = files.create_items(3);
        pane.items[0].alias = Some("apple".to_string());
        pane.items[1].alias = Some("banana".to_string());
        pane.items[2].alias = Some("grape".to_string());
        pane.filter.applied = Some("an".to_string());

        // When getting filtered.
        let filtered = pane.get_filtered();

        // Then only matching items are returned.
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].1.path, ItemPath::File(files.path(1).clone()));
    }

    #[test]
    fn refresh_filters_out_playlist_items() {
        // Given a pane and playlist paths.
        let mut files = TestFiles::new();
        let mut pane = LibraryPane::new();
        let entries = files.create_items(3);
        let b_path = ItemPath::File(files.path(1).clone());
        let playlist_paths: Vec<&ItemPath> = vec![&b_path];

        // When refreshing.
        pane.refresh(entries, &playlist_paths);

        // Then items in playlist are excluded.
        assert_eq!(pane.items.len(), 2);
        let excluded_path = ItemPath::File(files.path(1).clone());
        assert!(pane.items.iter().all(|i| i.path != excluded_path));
    }

    #[test]
    fn refresh_preserves_virtual_items() {
        // Given a pane with virtual items.
        let mut files = TestFiles::new();
        let mut pane = LibraryPane::new();
        let old_item = files.create_item();
        pane.items = vec![virtual_item("https://example.com/video.mp4"), old_item];

        // When refreshing with new disk entries (old.mp4 no longer exists).
        let new_item = files.create_item();
        pane.refresh(vec![new_item.clone()], &[]);

        // Then virtual items are preserved, old non-virtual is removed, new is added.
        assert_eq!(pane.items.len(), 2);
        assert!(pane.items.iter().any(|i| i.path
            == ItemPath::Url("https://example.com/video.mp4".to_string())
            && i.is_virtual));
        assert!(pane.items.iter().any(|i| i.path == new_item.path));
        let old_path = ItemPath::File(files.path(0).clone());
        assert!(!pane.items.iter().any(|i| i.path == old_path));
    }

    #[test]
    fn refresh_preserves_virtual_items_not_in_disk_entries() {
        // Given a pane with only virtual items.
        let mut pane = LibraryPane::new();
        pane.items = vec![virtual_item("https://youtube.com/watch?v=abc")];

        // When refreshing with empty disk entries.
        pane.refresh(vec![], &[]);

        // Then virtual items are still present.
        assert_eq!(pane.items.len(), 1);
        assert_eq!(
            pane.items[0].path,
            ItemPath::Url("https://youtube.com/watch?v=abc".to_string())
        );
        assert!(pane.items[0].is_virtual);
    }

    #[test]
    fn handle_key_delegates_to_filter_when_active() {
        // Given a pane with active filter.
        let mut pane = LibraryPane::new();
        pane.filter.start();

        // When handling a character key.
        let result = pane.handle_key(KeyEvent::from(KeyCode::Char('x')));

        // Then the filter handles it.
        assert!(result.is_consumed());
        assert_eq!(pane.filter.input(), "x");
    }

    #[test]
    fn handle_key_returns_ignored_for_unhandled() {
        // Given a pane.
        let mut pane = LibraryPane::new();

        // When pressing an unhandled key.
        let result = pane.handle_key(KeyEvent::from(KeyCode::Char('x')));

        // Then the event is ignored.
        assert!(!result.is_consumed());
    }
}
