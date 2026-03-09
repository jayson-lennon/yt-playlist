mod common;
mod error_popup;
mod filter;
mod library_pane;
mod playlist_pane;
mod rename;
mod state;
mod url_input;
mod which_key;

pub use crate::{keymap::Keymap, tui::state::TuiState};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    widgets::Paragraph,
};

pub use common::{Pane, PlaylistItem, get_mime_type};
pub use error_popup::ErrorPopup;
pub use filter::Filter;
pub use library_pane::LibraryPane;
pub use playlist_pane::PlaylistPane;
pub use rename::Rename;
pub use url_input::UrlInput;
pub use which_key::{WhichKey, WhichKeyConfig, WhichKeyPosition};

pub fn render(frame: &mut Frame, state: &TuiState, keymap: &Keymap) {
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
        .library_pane
        .render(frame, chunks[1], state.focused_pane == Pane::Library);

    let status_text = state.status_message.clone().unwrap_or_default();
    let status = Paragraph::new(status_text).style(Style::default().fg(Color::Yellow));
    frame.render_widget(status, status_area);

    if state.is_renaming() {
        state.rename.render(frame, state.get_selected_item());
    }

    if state.is_url_input() {
        state.url_input.render(frame);
    }

    if state.which_key.active {
        state.which_key.render(frame, keymap, state.focused_pane);
    }

    if state.is_showing_error() {
        state.error_popup.render(frame);
    }
}
