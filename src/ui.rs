use ratatui::{
    layout::{Constraint, Direction, Flex, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph},
    Frame,
};

use crate::tui_state::{Pane, TuiState};

pub fn render(frame: &mut Frame, state: &TuiState) {
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(frame.area());

    let panes_area = main_chunks[0];
    let status_area = main_chunks[1];

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(panes_area);

    render_playlist(frame, state, chunks[0]);
    render_directory(frame, state, chunks[1]);

    let status_text = state.status_message.clone().unwrap_or_default();
    let status = Paragraph::new(status_text).style(Style::default().fg(Color::Yellow));
    frame.render_widget(status, status_area);

    if state.is_renaming() {
        render_rename_popup(frame, state);
    }
}

fn render_playlist(frame: &mut Frame, state: &TuiState, area: Rect) {
    let is_filtering = state.is_filtering() && state.focused_pane == Pane::Playlist;
    let has_filter = state.has_active_filter(Pane::Playlist);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(area);

    let list_area = chunks[0];
    let footer_area = chunks[1];

    let filtered = state.get_filtered_playlist();
    let total_duration: std::time::Duration =
        filtered.iter().filter_map(|(_, item)| item.duration).sum();

    let items: Vec<ListItem> = filtered
        .iter()
        .enumerate()
        .map(|(display_idx, (_original_idx, item))| {
            let is_selected = display_idx == state.playlist_selected
                && state.focused_pane == Pane::Playlist
                && !is_filtering;
            let style = if is_selected {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            let duration_str = format_duration(item.duration);
            let name = TuiState::get_display_name(item);
            let text = format!("{duration_str} {name}");
            ListItem::new(text).style(style)
        })
        .collect();

    let title = if has_filter {
        if state.focused_pane == Pane::Playlist {
            " Playlist [filtered] [*] "
        } else {
            " Playlist [filtered] "
        }
    } else if state.focused_pane == Pane::Playlist {
        " Playlist [*] "
    } else {
        " Playlist "
    };

    let list = List::new(items).block(
        Block::default()
            .title(title)
            .borders(Borders::TOP | Borders::LEFT | Borders::RIGHT)
            .border_style(if state.focused_pane == Pane::Playlist {
                Style::default().fg(Color::Cyan)
            } else {
                Style::default()
            }),
    );

    let mut list_state = ListState::default();
    if !is_filtering && !filtered.is_empty() && state.playlist_selected < filtered.len() {
        list_state.select(Some(state.playlist_selected));
    }
    frame.render_stateful_widget(list, list_area, &mut list_state);

    if is_filtering {
        let filter_text = format!("Filter: {}█", state.get_filter_input(Pane::Playlist));
        let footer = Paragraph::new(filter_text).style(
            Style::default()
                .fg(Color::Black)
                .bg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        );
        frame.render_widget(footer, footer_area);
    } else {
        let total_str = format_duration(Some(total_duration));
        let footer = Paragraph::new(format!("Total: {total_str}")).style(
            if state.focused_pane == Pane::Playlist {
                Style::default().fg(Color::Cyan)
            } else {
                Style::default()
            },
        );
        frame.render_widget(footer, footer_area);
    }
}

fn render_directory(frame: &mut Frame, state: &TuiState, area: Rect) {
    let is_filtering = state.is_filtering() && state.focused_pane == Pane::Directory;
    let has_filter = state.has_active_filter(Pane::Directory);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(area);

    let list_area = chunks[0];
    let footer_area = chunks[1];

    let filtered = state.get_filtered_directory();
    let total_duration: std::time::Duration =
        filtered.iter().filter_map(|(_, item)| item.duration).sum();

    let items: Vec<ListItem> = filtered
        .iter()
        .enumerate()
        .map(|(display_idx, (_original_idx, item))| {
            let is_selected = display_idx == state.directory_selected
                && state.focused_pane == Pane::Directory
                && !is_filtering;
            let style = if is_selected {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            let duration_str = format_duration(item.duration);
            let name = TuiState::get_display_name(item);
            let text = format!("{duration_str} {name}");
            ListItem::new(text).style(style)
        })
        .collect();

    let title = if has_filter {
        if state.focused_pane == Pane::Directory {
            " Directory [filtered] [*] "
        } else {
            " Directory [filtered] "
        }
    } else if state.focused_pane == Pane::Directory {
        " Directory [*] "
    } else {
        " Directory "
    };

    let list = List::new(items).block(
        Block::default()
            .title(title)
            .borders(Borders::TOP | Borders::LEFT | Borders::RIGHT)
            .border_style(if state.focused_pane == Pane::Directory {
                Style::default().fg(Color::Cyan)
            } else {
                Style::default()
            }),
    );

    let mut list_state = ListState::default();
    if !is_filtering && !filtered.is_empty() && state.directory_selected < filtered.len() {
        list_state.select(Some(state.directory_selected));
    }
    frame.render_stateful_widget(list, list_area, &mut list_state);

    if is_filtering {
        let filter_text = format!("Filter: {}█", state.get_filter_input(Pane::Directory));
        let footer = Paragraph::new(filter_text).style(
            Style::default()
                .fg(Color::Black)
                .bg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        );
        frame.render_widget(footer, footer_area);
    } else {
        let total_str = format_duration(Some(total_duration));
        let footer = Paragraph::new(format!("Total: {total_str}")).style(
            if state.focused_pane == Pane::Directory {
                Style::default().fg(Color::Cyan)
            } else {
                Style::default()
            },
        );
        frame.render_widget(footer, footer_area);
    }
}

fn format_duration(duration: Option<std::time::Duration>) -> String {
    match duration {
        Some(d) => {
            let total_secs = d.as_secs();
            let hours = total_secs / 3600;
            let mins = (total_secs % 3600) / 60;
            let secs = total_secs % 60;
            format!("[{hours:02}:{mins:02}:{secs:02}]")
        }
        None => "[--:--:--]".to_string(),
    }
}

fn popup_area(area: Rect, percent_x: u16, percent_y: u16) -> Rect {
    let vertical = Layout::vertical([Constraint::Percentage(percent_y)]).flex(Flex::Center);
    let horizontal = Layout::horizontal([Constraint::Percentage(percent_x)]).flex(Flex::Center);
    let [area] = vertical.areas(area);
    let [area] = horizontal.areas(area);
    area
}

fn render_rename_popup(frame: &mut Frame, state: &TuiState) {
    let area = popup_area(frame.area(), 50, 20);

    frame.render_widget(Clear, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Length(3)])
        .split(area);

    let filename = state.get_selected_item().map_or_else(
        || "Unknown".to_string(),
        |item| {
            item.path.file_name().map_or_else(
                || item.path.to_string_lossy().into_owned(),
                |n| n.to_string_lossy().into_owned(),
            )
        },
    );

    let title = Paragraph::new(filename).style(Style::default().fg(Color::Cyan));
    frame.render_widget(title, chunks[0]);

    let input_text = format!("{}█", state.rename.input);
    let input = Paragraph::new(input_text).block(
        Block::default()
            .title("Alias")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow)),
    );
    frame.render_widget(input, chunks[1]);
}
