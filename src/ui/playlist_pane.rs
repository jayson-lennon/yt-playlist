use std::path::PathBuf;

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};

use super::common::{filter_items, format_duration, get_display_name, PlaylistItem};
use super::filter::Filter;

#[derive(Debug, Clone)]
pub struct PlaylistPane {
    pub items: Vec<PlaylistItem>,
    pub selected: usize,
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

    pub fn paths(&self) -> Vec<&PathBuf> {
        self.items.iter().map(|item| &item.path).collect()
    }

    pub fn render(&self, frame: &mut Frame, area: Rect, is_focused: bool) {
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
                let style = if is_selected {
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };
                let duration_str = format_duration(item.duration);
                let name = get_display_name(item);
                let text = format!("{duration_str} {name}");
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

impl Default for PlaylistPane {
    fn default() -> Self {
        Self::new()
    }
}
