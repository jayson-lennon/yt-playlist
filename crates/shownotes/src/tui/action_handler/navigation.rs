use super::TuiActionCtx;
use crate::tui::{Pane, TuiActionResponse};

pub fn handle_move_up(ctx: &mut TuiActionCtx<'_>) -> TuiActionResponse {
    match ctx.tui_state.focused_pane {
        Pane::Playlist => ctx.tui_state.move_playlist_up(),
        Pane::Library => ctx.tui_state.move_library_up(),
    }
    TuiActionResponse::Continue
}

pub fn handle_move_down(ctx: &mut TuiActionCtx<'_>) -> TuiActionResponse {
    match ctx.tui_state.focused_pane {
        Pane::Playlist => ctx.tui_state.move_playlist_down(),
        Pane::Library => ctx.tui_state.move_library_down(),
    }
    TuiActionResponse::Continue
}

pub fn handle_switch_pane(ctx: &mut TuiActionCtx<'_>) -> TuiActionResponse {
    ctx.tui_state.switch_pane();
    TuiActionResponse::Continue
}

pub fn handle_focus_playlist(ctx: &mut TuiActionCtx<'_>) -> TuiActionResponse {
    if !ctx.tui_state.playlist_pane.items.is_empty() {
        ctx.tui_state.focused_pane = Pane::Playlist;
    }
    TuiActionResponse::Continue
}

pub fn handle_focus_library(ctx: &mut TuiActionCtx<'_>) -> TuiActionResponse {
    if !ctx.tui_state.library_pane.items.is_empty() {
        ctx.tui_state.focused_pane = Pane::Library;
    }
    TuiActionResponse::Continue
}
