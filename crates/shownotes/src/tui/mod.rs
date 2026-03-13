mod action_handler;
mod commands;
mod common;
mod component;
mod error_popup;
mod event;
mod filter;
mod global_key_handler;
mod library_pane;
mod playlist_pane;
mod rename;
mod render;
mod state;
mod status_bar;
mod tui_action;
mod url_input;
mod which_key;

pub use crate::feat::keymap::Keymap;
use crate::services::Services;
pub use crate::tui::state::TuiState;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    Frame,
};

pub use commands::{
    handle_rename_submit, handle_url_submit, load_playlist, refresh_library, set_initial_focus,
};
pub use common::{get_mime_type, ItemDisplayMode, ItemPath, Pane, PlaylistItem};
pub use component::{Component, ComponentContext};
pub use error_popup::ErrorPopup;
pub use event::EventResult;
pub use filter::Filter;
pub use global_key_handler::GlobalKeyHandler;
pub use library_pane::LibraryPane;
pub use playlist_pane::PlaylistPane;
pub use rename::Rename;
pub use render::{AreaRender, Render, RenderContext};
pub use status_bar::StatusBar;
pub use url_input::UrlInput;
pub use which_key::{WhichKey, WhichKeyConfig, WhichKeyPosition};

pub use action_handler::{dispatch as execute_tui_action, TuiActionCtx};
pub use tui_action::{ShowNoteKind, TuiAction, TuiActionResponse};

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

    AreaRender::to(status_area).try_render(&mut ctx, &state.status_bar);

    AreaRender::to(Rect::default()).try_render(&mut ctx, &state.rename);
    AreaRender::to(Rect::default()).try_render(&mut ctx, &state.url_input);
    AreaRender::to(Rect::default()).try_render(&mut ctx, &state.global_handler);
    AreaRender::to(Rect::default()).try_render(&mut ctx, &state.error_popup);
}
