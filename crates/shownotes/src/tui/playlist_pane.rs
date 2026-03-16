use crossterm::event::{KeyCode, KeyEvent};

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
                let text = format_item_line(item, display_mode);
                ListItem::new(text).style(style)
            })
            .collect();

        let title = if has_filter {
            if is_focused {
                " Playlist [filtered] [*] "
            } else {
                " Playlist [filtered] "
            }
        } else if is_focused {
            " Playlist [*] "
        } else {
            " Playlist "
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

        match key.code {
            KeyCode::Char('j') => {
                self.move_down();
                HandleKeyResult::consumed()
            }
            KeyCode::Char('k') => {
                self.move_up();
                HandleKeyResult::consumed()
            }
            KeyCode::Char('J') => {
                self.reorder_down();
                HandleKeyResult::consumed()
            }
            KeyCode::Char('K') => {
                self.reorder_up();
                HandleKeyResult::consumed()
            }
            _ => HandleKeyResult::ignored(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use marked_path::CanonicalPath;
    use std::path::PathBuf;

    fn item(path: &str) -> PlaylistItem {
        PlaylistItem {
            path: ItemPath::File(CanonicalPath::new(PathBuf::from(path))),
            duration: None,
            alias: None,
            mime_type: None,
            is_virtual: false,
        }
    }

    #[allow(dead_code)]
    fn item_with_alias(path: &str, alias: &str) -> PlaylistItem {
        PlaylistItem {
            path: ItemPath::File(CanonicalPath::new(PathBuf::from(path))),
            duration: None,
            alias: Some(alias.to_string()),
            mime_type: None,
            is_virtual: false,
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
        let mut pane = PlaylistPane::new();
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
        let mut pane = PlaylistPane::new();
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
        let mut pane = PlaylistPane::new();
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
        let mut pane = PlaylistPane::new();
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
        let mut pane = PlaylistPane::new();
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
        let mut pane = PlaylistPane::new();
        pane.items = vec![item("apple.mp4"), item("banana.mp4"), item("apricot.mp4")];
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
        let mut pane = PlaylistPane::new();
        pane.items = vec![item("a.mp4"), item("b.mp4"), item("c.mp4")];
        pane.selected = 1;

        // When reordering up.
        pane.reorder_up();

        // Then items are swapped and selection follows.
        assert_eq!(pane.selected, 0);
        assert_eq!(
            pane.items[0].path,
            ItemPath::File(CanonicalPath::new(PathBuf::from("b.mp4")))
        );
        assert_eq!(
            pane.items[1].path,
            ItemPath::File(CanonicalPath::new(PathBuf::from("a.mp4")))
        );
    }

    #[test]
    fn reorder_up_does_nothing_at_first_item() {
        // Given a pane with first item selected.
        let mut pane = PlaylistPane::new();
        pane.items = vec![item("a.mp4"), item("b.mp4")];
        pane.selected = 0;

        // When reordering up.
        pane.reorder_up();

        // Then nothing changes.
        assert_eq!(pane.selected, 0);
        assert_eq!(
            pane.items[0].path,
            ItemPath::File(CanonicalPath::new(PathBuf::from("a.mp4")))
        );
    }

    #[test]
    fn reorder_down_swaps_with_next_item() {
        // Given a pane with 3 items, first selected.
        let mut pane = PlaylistPane::new();
        pane.items = vec![item("a.mp4"), item("b.mp4"), item("c.mp4")];
        pane.selected = 0;

        // When reordering down.
        pane.reorder_down();

        // Then items are swapped and selection follows.
        assert_eq!(pane.selected, 1);
        assert_eq!(
            pane.items[0].path,
            ItemPath::File(CanonicalPath::new(PathBuf::from("b.mp4")))
        );
        assert_eq!(
            pane.items[1].path,
            ItemPath::File(CanonicalPath::new(PathBuf::from("a.mp4")))
        );
    }

    #[test]
    fn reorder_down_does_nothing_at_last_item() {
        // Given a pane with last item selected.
        let mut pane = PlaylistPane::new();
        pane.items = vec![item("a.mp4"), item("b.mp4")];
        pane.selected = 1;

        // When reordering down.
        pane.reorder_down();

        // Then nothing changes.
        assert_eq!(pane.selected, 1);
        assert_eq!(
            pane.items[1].path,
            ItemPath::File(CanonicalPath::new(PathBuf::from("b.mp4")))
        );
    }

    #[test]
    fn add_appends_new_item() {
        // Given an empty pane.
        let mut pane = PlaylistPane::new();

        // When adding an item.
        pane.add(item("test.mp4"));

        // Then item is added.
        assert_eq!(pane.items.len(), 1);
        assert_eq!(
            pane.items[0].path,
            ItemPath::File(CanonicalPath::new(PathBuf::from("test.mp4")))
        );
    }

    #[test]
    fn add_prevents_duplicates() {
        // Given a pane with one item.
        let mut pane = PlaylistPane::new();
        pane.add(item("test.mp4"));

        // When adding the same item again.
        pane.add(item("test.mp4"));

        // Then item is not duplicated.
        assert_eq!(pane.items.len(), 1);
    }

    #[test]
    fn add_allows_different_paths() {
        // Given a pane with one item.
        let mut pane = PlaylistPane::new();
        pane.add(item("a.mp4"));

        // When adding a different item.
        pane.add(item("b.mp4"));

        // Then both items exist.
        assert_eq!(pane.items.len(), 2);
    }

    #[test]
    fn remove_deletes_selected_item() {
        // Given a pane with 3 items, middle selected.
        let mut pane = PlaylistPane::new();
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
        let mut pane = PlaylistPane::new();
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
        let mut pane = PlaylistPane::new();
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
        let mut pane = PlaylistPane::new();
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
        let pane = PlaylistPane::new();

        // When getting selected item.
        let selected = pane.selected_item();

        // Then none is returned.
        assert!(selected.is_none());
    }

    #[test]
    fn selected_item_with_filter_returns_filtered_item() {
        // Given a pane with filter applied.
        let mut pane = PlaylistPane::new();
        pane.items = vec![item("apple.mp4"), item("banana.mp4"), item("apricot.mp4")];
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
        let mut pane = PlaylistPane::new();
        pane.items = vec![item("test.mp4")];

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
        let mut pane = PlaylistPane::new();
        pane.items = vec![item("a.mp4"), item("b.mp4")];

        // When getting paths.
        let paths = pane.paths();

        // Then all paths are returned.
        assert_eq!(paths.len(), 2);
        assert_eq!(
            *paths[0],
            ItemPath::File(CanonicalPath::new(PathBuf::from("a.mp4")))
        );
        assert_eq!(
            *paths[1],
            ItemPath::File(CanonicalPath::new(PathBuf::from("b.mp4")))
        );
    }

    #[test]
    fn get_filtered_returns_all_when_no_filter() {
        // Given a pane without filter.
        let mut pane = PlaylistPane::new();
        pane.items = vec![item("a.mp4"), item("b.mp4")];

        // When getting filtered.
        let filtered = pane.get_filtered();

        // Then all items are returned.
        assert_eq!(filtered.len(), 2);
    }

    #[test]
    fn get_filtered_filters_by_pattern() {
        // Given a pane with filter applied.
        let mut pane = PlaylistPane::new();
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
    fn handle_key_j_moves_down() {
        // Given a pane with 3 items, selection at index 0.
        let mut pane = PlaylistPane::new();
        pane.items = vec![item("a.mp4"), item("b.mp4"), item("c.mp4")];
        pane.selected = 0;

        // When pressing 'j'.
        let result = pane.handle_key(KeyEvent::from(KeyCode::Char('j')));

        // Then selection moves down.
        assert!(result.is_consumed());
        assert_eq!(pane.selected, 1);
    }

    #[test]
    fn handle_key_k_moves_up() {
        // Given a pane with 3 items, selection at index 1.
        let mut pane = PlaylistPane::new();
        pane.items = vec![item("a.mp4"), item("b.mp4"), item("c.mp4")];
        pane.selected = 1;

        // When pressing 'k'.
        let result = pane.handle_key(KeyEvent::from(KeyCode::Char('k')));

        // Then selection moves up.
        assert!(result.is_consumed());
        assert_eq!(pane.selected, 0);
    }

    #[test]
    fn handle_key_j_reorder_down() {
        // Given a pane with 3 items, first selected.
        let mut pane = PlaylistPane::new();
        pane.items = vec![item("a.mp4"), item("b.mp4"), item("c.mp4")];
        pane.selected = 0;

        // When pressing 'J' (shift+j).
        let result = pane.handle_key(KeyEvent::from(KeyCode::Char('J')));

        // Then item is reordered down.
        assert!(result.is_consumed());
        assert_eq!(pane.selected, 1);
        assert_eq!(
            pane.items[0].path,
            ItemPath::File(CanonicalPath::new(PathBuf::from("b.mp4")))
        );
    }

    #[test]
    fn handle_key_shift_k_reorder_up() {
        // Given a pane with 3 items, middle selected.
        let mut pane = PlaylistPane::new();
        pane.items = vec![item("a.mp4"), item("b.mp4"), item("c.mp4")];
        pane.selected = 1;

        // When pressing 'K' (shift+k).
        let result = pane.handle_key(KeyEvent::from(KeyCode::Char('K')));

        // Then item is reordered up.
        assert!(result.is_consumed());
        assert_eq!(pane.selected, 0);
        assert_eq!(
            pane.items[0].path,
            ItemPath::File(CanonicalPath::new(PathBuf::from("b.mp4")))
        );
    }

    #[test]
    fn handle_key_returns_ignored_for_unhandled() {
        // Given a pane.
        let mut pane = PlaylistPane::new();
        pane.items = vec![item("a.mp4")];

        // When pressing an unhandled key.
        let result = pane.handle_key(KeyEvent::from(KeyCode::Char('x')));

        // Then the event is ignored.
        assert!(!result.is_consumed());
    }
}
