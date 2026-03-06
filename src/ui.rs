use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
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
}

fn render_playlist(frame: &mut Frame, state: &TuiState, area: Rect) {
    let total_duration: std::time::Duration =
        state.playlist.iter().filter_map(|item| item.duration).sum();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(area);

    let list_area = chunks[0];
    let footer_area = chunks[1];

    let items: Vec<ListItem> = state
        .playlist
        .iter()
        .enumerate()
        .map(|(i, item)| {
            let style = if i == state.playlist_selected && state.focused_pane == Pane::Playlist {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            let duration_str = format_duration(item.duration);
            let name = item.path.file_name().map_or_else(
                || item.path.to_string_lossy().into_owned(),
                |n| n.to_string_lossy().into_owned(),
            );
            let text = format!("{duration_str} {name}");
            ListItem::new(text).style(style)
        })
        .collect();

    let title = if state.focused_pane == Pane::Playlist {
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
    list_state.select(Some(state.playlist_selected));
    frame.render_stateful_widget(list, list_area, &mut list_state);

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

fn render_directory(frame: &mut Frame, state: &TuiState, area: Rect) {
    let total_duration: std::time::Duration = state
        .directory
        .iter()
        .filter_map(|item| item.duration)
        .sum();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(area);

    let list_area = chunks[0];
    let footer_area = chunks[1];

    let items: Vec<ListItem> = state
        .directory
        .iter()
        .enumerate()
        .map(|(i, item)| {
            let style = if i == state.directory_selected && state.focused_pane == Pane::Directory {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            let duration_str = format_duration(item.duration);
            let name = item.path.file_name().map_or_else(
                || item.path.to_string_lossy().into_owned(),
                |n| n.to_string_lossy().into_owned(),
            );
            let text = format!("{duration_str} {name}");
            ListItem::new(text).style(style)
        })
        .collect();

    let title = if state.focused_pane == Pane::Directory {
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
    list_state.select(Some(state.directory_selected));
    frame.render_stateful_widget(list, list_area, &mut list_state);

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
