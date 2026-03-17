use crossterm::event::KeyEvent;

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};

use super::common::{
    filter_items, format_duration, format_item_line, ItemDisplayMode, ItemPath, Pane, PlaylistItem,
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

    pub fn render(
        &self,
        frame: &mut Frame,
        area: Rect,
        is_focused: bool,
        display_mode: ItemDisplayMode,
    ) {
        let is_filtering = self.filter.is_active();
        let has_filter = self.filter.has_applied();

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(1)])
            .split(area);

        let list_area = chunks[0];
        let footer_area = chunks[1];

        let filtered = self.get_filtered();
        let total_duration: std::time::Duration =
            filtered.iter().filter_map(|(_, item)| item.duration).sum();

        let items: Vec<ListItem> = filtered
            .iter()
            .enumerate()
            .map(|(display_idx, (_original_idx, item))| {
                let is_selected = display_idx == self.selected && is_focused && !is_filtering;
                let file_missing =
                    !item.path.as_file().is_some_and(|p| p.as_path().exists()) && !item.is_virtual;
                let style = if is_selected {
                    if file_missing {
                        Style::default()
                            .fg(Color::Red)
                            .bg(Color::Yellow)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default()
                            .fg(Color::Black)
                            .bg(Color::Yellow)
                            .add_modifier(Modifier::BOLD)
                    }
                } else {
                    Style::default()
                };
                let text = format_item_line(
                    item,
                    display_mode,
                    list_area.width.saturating_sub(BORDER_COLUMNS),
                    item.playlist_count,
                    1, // Show count when >= 1 for library pane
                );
                ListItem::new(text).style(style)
            })
            .collect();

        let title = if has_filter {
            if is_focused {
                " Library [filtered] [*] "
            } else {
                " Library [filtered] "
            }
        } else if is_focused {
            " Library [*] "
        } else {
            " Library "
        };

        let list = List::new(items).block(
            Block::default()
                .title(title)
                .borders(Borders::TOP | Borders::LEFT | Borders::RIGHT)
                .border_style(if is_focused {
                    Style::default().fg(Color::Cyan)
                } else {
                    Style::default()
                }),
        );

        let mut list_state = ListState::default();
        if !is_filtering && !filtered.is_empty() && self.selected < filtered.len() {
            list_state.select(Some(self.selected));
        }
        frame.render_stateful_widget(list, list_area, &mut list_state);

        if is_filtering {
            self.filter.render(frame, footer_area);
        } else {
            let total_str = format_duration(Some(total_duration));
            let footer = Paragraph::new(format!("Total: {total_str}")).style(if is_focused {
                Style::default().fg(Color::Cyan)
            } else {
                Style::default()
            });
            frame.render_widget(footer, footer_area);
        }
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
    use std::path::PathBuf;

    fn item(path: &str) -> PlaylistItem {
        PlaylistItem {
            path: ItemPath::File(CanonicalPath::new(PathBuf::from(path))),
            duration: None,
            alias: None,
            mime_type: None,
            is_virtual: false,
            playlist_count: 0,
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
        let mut pane = LibraryPane::new();
        pane.items = vec![item("a.mp4"), item("b.mp4"), item("c.mp4")];
        pane.selected = 1;

        // When moving up.
        pane.move_up();

        // Then selection decrements.
        assert_eq!(pane.selected, 0);
    }

    #[test]
    fn move_up_stays_at_first_item() {
        // Given a pane with selection at first item.
        let mut pane = LibraryPane::new();
        pane.items = vec![item("a.mp4"), item("b.mp4")];
        pane.selected = 0;

        // When moving up.
        pane.move_up();

        // Then selection stays at 0.
        assert_eq!(pane.selected, 0);
    }

    #[test]
    fn move_down_increments_selection() {
        // Given a pane with 3 items, selection at index 0.
        let mut pane = LibraryPane::new();
        pane.items = vec![item("a.mp4"), item("b.mp4"), item("c.mp4")];
        pane.selected = 0;

        // When moving down.
        pane.move_down();

        // Then selection increments.
        assert_eq!(pane.selected, 1);
    }

    #[test]
    fn move_down_stays_at_last_item() {
        // Given a pane with selection at last item.
        let mut pane = LibraryPane::new();
        pane.items = vec![item("a.mp4"), item("b.mp4")];
        pane.selected = 1;

        // When moving down.
        pane.move_down();

        // Then selection stays at last index.
        assert_eq!(pane.selected, 1);
    }

    #[test]
    fn move_up_with_filter_navigates_filtered_list() {
        // Given a pane with filter applied.
        let mut pane = LibraryPane::new();
        pane.items = vec![item("apple.mp4"), item("banana.mp4"), item("apricot.mp4")];
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
        let mut pane = LibraryPane::new();
        pane.items = vec![item("apple.mp4"), item("banana.mp4"), item("apricot.mp4")];
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
        let mut pane = LibraryPane::new();
        pane.items = vec![item("a.mp4"), item("b.mp4"), item("c.mp4")];
        pane.selected = 1;

        // When removing.
        pane.remove();

        // Then selected item is removed.
        assert_eq!(pane.items.len(), 2);
        assert_eq!(
            pane.items[0].path,
            ItemPath::File(CanonicalPath::new(PathBuf::from("a.mp4")))
        );
        assert_eq!(
            pane.items[1].path,
            ItemPath::File(CanonicalPath::new(PathBuf::from("c.mp4")))
        );
    }

    #[test]
    fn remove_adjusts_selection_when_at_end() {
        // Given a pane with 2 items, last selected.
        let mut pane = LibraryPane::new();
        pane.items = vec![item("a.mp4"), item("b.mp4")];
        pane.selected = 1;

        // When removing.
        pane.remove();

        // Then selection is adjusted.
        assert_eq!(pane.selected, 0);
    }

    #[test]
    fn remove_with_filter_removes_correct_item() {
        // Given a pane with filter applied.
        let mut pane = LibraryPane::new();
        pane.items = vec![item("apple.mp4"), item("banana.mp4"), item("apricot.mp4")];
        pane.filter.applied = Some("ap".to_string());
        pane.selected = 1;

        // When removing (should remove apricot, index 2 in original).
        pane.remove();

        // Then correct item is removed.
        assert_eq!(pane.items.len(), 2);
        assert!(pane
            .items
            .iter()
            .all(|i| i.path != ItemPath::File(CanonicalPath::new(PathBuf::from("apricot.mp4")))));
    }

    #[test]
    fn selected_item_returns_current_selection() {
        // Given a pane with items.
        let mut pane = LibraryPane::new();
        pane.items = vec![item("a.mp4"), item("b.mp4")];
        pane.selected = 1;

        // When getting selected item.
        let selected = pane.selected_item();

        // Then correct item is returned.
        assert_eq!(
            selected.unwrap().path,
            ItemPath::File(CanonicalPath::new(PathBuf::from("b.mp4")))
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
        let mut pane = LibraryPane::new();
        pane.items = vec![item("apple.mp4"), item("banana.mp4"), item("apricot.mp4")];
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
        let mut pane = LibraryPane::new();
        pane.items = vec![item("test.mp4")];

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
        let mut pane = LibraryPane::new();
        pane.selected = 10;

        // When refreshing with fewer items.
        pane.refresh(vec![item("a.mp4"), item("b.mp4")], &[]);

        // Then selection is adjusted to last item.
        assert_eq!(pane.selected, 1);
    }

    #[test]
    fn refresh_keeps_selection_when_valid() {
        // Given a pane with valid selection.
        let mut pane = LibraryPane::new();
        pane.selected = 1;

        // When refreshing with enough items.
        pane.refresh(vec![item("a.mp4"), item("b.mp4"), item("c.mp4")], &[]);

        // Then selection is preserved.
        assert_eq!(pane.selected, 1);
    }

    #[test]
    fn get_filtered_returns_all_when_no_filter() {
        // Given a pane without filter.
        let mut pane = LibraryPane::new();
        pane.items = vec![item("a.mp4"), item("b.mp4")];

        // When getting filtered.
        let filtered = pane.get_filtered();

        // Then all items are returned.
        assert_eq!(filtered.len(), 2);
    }

    #[test]
    fn get_filtered_filters_by_pattern() {
        // Given a pane with filter applied.
        let mut pane = LibraryPane::new();
        pane.items = vec![item("apple.mp4"), item("banana.mp4"), item("cherry.mp4")];
        pane.filter.applied = Some("an".to_string());

        // When getting filtered.
        let filtered = pane.get_filtered();

        // Then only matching items are returned.
        assert_eq!(filtered.len(), 1);
        assert_eq!(
            filtered[0].1.path,
            ItemPath::File(CanonicalPath::new(PathBuf::from("banana.mp4")))
        );
    }

    #[test]
    fn refresh_filters_out_playlist_items() {
        // Given a pane and playlist paths.
        let mut pane = LibraryPane::new();
        let entries = vec![item("a.mp4"), item("b.mp4"), item("c.mp4")];
        let b_path = ItemPath::File(CanonicalPath::new(PathBuf::from("b.mp4")));
        let playlist_paths: Vec<&ItemPath> = vec![&b_path];

        // When refreshing.
        pane.refresh(entries, &playlist_paths);

        // Then items in playlist are excluded.
        assert_eq!(pane.items.len(), 2);
        assert!(pane
            .items
            .iter()
            .all(|i| i.path != ItemPath::File(CanonicalPath::new(PathBuf::from("b.mp4")))));
    }

    #[test]
    fn refresh_preserves_virtual_items() {
        // Given a pane with virtual items.
        let mut pane = LibraryPane::new();
        pane.items = vec![
            virtual_item("https://example.com/video.mp4"),
            item("old.mp4"),
        ];

        // When refreshing with new disk entries (old.mp4 no longer exists).
        let entries = vec![item("new.mp4")];
        pane.refresh(entries, &[]);

        // Then virtual items are preserved, old non-virtual is removed, new is added.
        assert_eq!(pane.items.len(), 2);
        assert!(pane.items.iter().any(|i| i.path
            == ItemPath::Url("https://example.com/video.mp4".to_string())
            && i.is_virtual));
        assert!(pane
            .items
            .iter()
            .any(|i| i.path == ItemPath::File(CanonicalPath::new(PathBuf::from("new.mp4")))));
        assert!(!pane
            .items
            .iter()
            .any(|i| i.path == ItemPath::File(CanonicalPath::new(PathBuf::from("old.mp4")))));
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
