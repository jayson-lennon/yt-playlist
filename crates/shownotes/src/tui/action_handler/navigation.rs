// Copyright (C) 2026 Jayson Lennon
// 
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as
// published by the Free Software Foundation, either version 3 of the
// License, or (at your option) any later version.
// 
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
// 
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

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
