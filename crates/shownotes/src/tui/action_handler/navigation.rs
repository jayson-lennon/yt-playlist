use super::TuiActionCtx;
use super::TuiActionError;
use crate::tui::{Pane, TuiActionResponse};
use error_stack::Report;

pub fn handle_move_up(
    ctx: &mut TuiActionCtx<'_>,
) -> Result<TuiActionResponse, Report<TuiActionError>> {
    match ctx.tui_state.focused_pane {
        Pane::Playlist => ctx.tui_state.move_playlist_up(),
        Pane::Library => ctx.tui_state.move_library_up(),
    }
    Ok(TuiActionResponse::Continue)
}

pub fn handle_move_down(
    ctx: &mut TuiActionCtx<'_>,
) -> Result<TuiActionResponse, Report<TuiActionError>> {
    match ctx.tui_state.focused_pane {
        Pane::Playlist => ctx.tui_state.move_playlist_down(),
        Pane::Library => ctx.tui_state.move_library_down(),
    }
    Ok(TuiActionResponse::Continue)
}

pub fn handle_switch_pane(
    ctx: &mut TuiActionCtx<'_>,
) -> Result<TuiActionResponse, Report<TuiActionError>> {
    ctx.tui_state.switch_pane();
    Ok(TuiActionResponse::Continue)
}

pub fn handle_focus_playlist(
    ctx: &mut TuiActionCtx<'_>,
) -> Result<TuiActionResponse, Report<TuiActionError>> {
    if !ctx.tui_state.playlist_pane.items.is_empty() {
        ctx.tui_state.focused_pane = Pane::Playlist;
    }
    Ok(TuiActionResponse::Continue)
}

pub fn handle_focus_library(
    ctx: &mut TuiActionCtx<'_>,
) -> Result<TuiActionResponse, Report<TuiActionError>> {
    if !ctx.tui_state.library_pane.items.is_empty() {
        ctx.tui_state.focused_pane = Pane::Library;
    }
    Ok(TuiActionResponse::Continue)
}
