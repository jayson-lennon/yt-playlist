mod common;
mod component;
mod error_popup;
mod event;
mod filter;
mod library_pane;
mod playlist_pane;
mod rename;
mod render;
mod state;
mod url_input;
mod which_key;

pub use crate::feat::keymap::Keymap;
use crate::services::Services;
pub use crate::tui::state::TuiState;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::Paragraph,
    Frame,
};

pub use common::{get_mime_type, ItemDisplayMode, ItemPath, Pane, PlaylistItem};
pub use component::{Component, ComponentContext};
pub use error_popup::ErrorPopup;
pub use event::EventResult;
pub use filter::Filter;
pub use library_pane::LibraryPane;
pub use playlist_pane::PlaylistPane;
pub use rename::Rename;
pub use render::{AreaRender, Render, RenderContext};
pub use url_input::UrlInput;
pub use which_key::{WhichKey, WhichKeyConfig, WhichKeyPosition};

pub fn render(frame: &mut Frame, state: &TuiState, keymap: &Keymap, services: &Services) {
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

    let mut ctx = RenderContext::new(frame, Rect::default(), keymap, services, state);

    AreaRender::to(chunks[0]).try_render(&mut ctx, &state.playlist_pane);
    AreaRender::to(chunks[1]).try_render(&mut ctx, &state.library_pane);

    let status_text = state.status_message.clone().unwrap_or_default();
    let status = Paragraph::new(status_text).style(Style::default().fg(Color::Yellow));
    ctx.frame.render_widget(status, status_area);

    AreaRender::to(Rect::default()).try_render(&mut ctx, &state.rename);
    AreaRender::to(Rect::default()).try_render(&mut ctx, &state.url_input);
    AreaRender::to(Rect::default()).try_render(&mut ctx, &state.which_key);
    AreaRender::to(Rect::default()).try_render(&mut ctx, &state.error_popup);
}
