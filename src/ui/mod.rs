mod common;
mod directory_pane;
mod filter;
mod playlist_pane;
mod rename;
mod which_key;

use crate::keymap::Keymap;
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    widgets::Paragraph,
    Frame,
};

pub use common::{Pane, PlaylistItem};
pub use directory_pane::DirectoryPane;
pub use filter::Filter;
pub use playlist_pane::PlaylistPane;
pub use rename::Rename;
pub use which_key::{WhichKey, WhichKeyConfig, WhichKeyPosition};

pub fn render(frame: &mut Frame, state: &crate::tui_state::TuiState, keymap: &Keymap) {
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

    state
        .playlist_pane
        .render(frame, chunks[0], state.focused_pane == Pane::Playlist);
    state
        .directory_pane
        .render(frame, chunks[1], state.focused_pane == Pane::Directory);

    let status_text = state.status_message.clone().unwrap_or_default();
    let status = Paragraph::new(status_text).style(Style::default().fg(Color::Yellow));
    frame.render_widget(status, status_area);

    if state.is_renaming() {
        state.rename.render(frame, state.get_selected_item());
    }

    if state.which_key.active {
        state.which_key.render(frame, keymap, state.focused_pane);
    }
}
