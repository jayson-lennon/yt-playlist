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
use super::TuiActionError;
use crate::command::{self, Command, CommandResult};
use crate::tui::state::RefreshError;
use crate::tui::{ItemDisplayMode, TuiActionResponse};
use error_stack::{Report, ResultExt};

pub fn handle_quit(ctx: &mut TuiActionCtx<'_>) -> TuiActionResponse {
    match ctx.execute(Command::PlaylistSave {
        playlist_items: ctx.tui_state.playlist_pane.items.clone(),
        library_items: ctx.tui_state.library_pane.items.clone(),
    }) {
        Ok(CommandResult::PlaylistSaved) => {
            ctx.tui_state.status_bar.set("Playlist saved");
        }
        Err(e) => {
            ctx.tui_state.show_error(format!("Failed to save: {e:?}"));
        }
        _ => unreachable!(),
    }
    TuiActionResponse::ShouldQuit
}

pub fn handle_save(ctx: &mut TuiActionCtx<'_>) -> TuiActionResponse {
    match ctx.execute(Command::PlaylistSave {
        playlist_items: ctx.tui_state.playlist_pane.items.clone(),
        library_items: ctx.tui_state.library_pane.items.clone(),
    }) {
        Ok(CommandResult::PlaylistSaved) => {
            ctx.tui_state.status_bar.set("Playlist saved");
        }
        Err(e) => {
            ctx.tui_state.show_error(format!("Failed to save: {e:?}"));
        }
        _ => unreachable!(),
    }
    TuiActionResponse::Continue
}

pub fn handle_show_help(ctx: &mut TuiActionCtx<'_>) -> TuiActionResponse {
    ctx.tui_state.global_handler.toggle_help();
    TuiActionResponse::Continue
}

pub fn handle_start_filter(ctx: &mut TuiActionCtx<'_>) -> TuiActionResponse {
    ctx.tui_state.start_filter();
    TuiActionResponse::Continue
}

pub fn handle_show_alias(ctx: &mut TuiActionCtx<'_>) -> TuiActionResponse {
    ctx.tui_state.display_mode = ItemDisplayMode::Alias;
    TuiActionResponse::Continue
}

pub fn handle_show_path(ctx: &mut TuiActionCtx<'_>) -> TuiActionResponse {
    ctx.tui_state.display_mode = ItemDisplayMode::Path;
    TuiActionResponse::Continue
}

pub fn handle_add_url(ctx: &mut TuiActionCtx<'_>) -> TuiActionResponse {
    ctx.tui_state.start_url_input();
    TuiActionResponse::Continue
}

pub fn handle_url_submit(
    ctx: &mut TuiActionCtx<'_>,
    url: String,
) -> Result<TuiActionResponse, Report<TuiActionError>> {
    match ctx.execute(Command::UrlAdd { url }) {
        Ok(CommandResult::UrlAdded { item }) => {
            ctx.tui_state.library_pane.items.push(item);
            ctx.tui_state
                .library_pane
                .items
                .sort_by(|a, b| a.path.to_string_lossy().cmp(&b.path.to_string_lossy()));
            ctx.execute(Command::PlaylistSave {
                playlist_items: ctx.tui_state.playlist_pane.items.clone(),
                library_items: ctx.tui_state.library_pane.items.clone(),
            })
            .change_context(TuiActionError)?;
            ctx.tui_state.status_bar.set("URL added to library");
        }
        Err(e) => {
            ctx.tui_state
                .show_error(format!("Failed to add URL: {e:?}"));
        }
        _ => unreachable!(),
    }
    Ok(TuiActionResponse::Continue)
}

pub fn handle_delete(ctx: &mut TuiActionCtx<'_>) -> TuiActionResponse {
    if let Some(item) = ctx.tui_state.selected_library_item() {
        if item.is_virtual {
            ctx.tui_state.library_pane.remove();
            match ctx.execute(Command::PlaylistSave {
                playlist_items: ctx.tui_state.playlist_pane.items.clone(),
                library_items: ctx.tui_state.library_pane.items.clone(),
            }) {
                Ok(CommandResult::PlaylistSaved) => {
                    ctx.tui_state.status_bar.set("Virtual entry deleted");
                }
                Err(e) => {
                    ctx.tui_state.show_error(format!("Failed to save: {e:?}"));
                }
                _ => unreachable!(),
            }
        } else {
            ctx.tui_state
                .status_bar
                .set("Only virtual entries (URLs) can be deleted.");
        }
    }
    TuiActionResponse::Continue
}

pub fn handle_refresh(ctx: &mut TuiActionCtx<'_>) -> TuiActionResponse {
    if ctx.tui_state.is_refreshing() {
        ctx.tui_state.status_bar.set("Refresh already in progress...");
        return TuiActionResponse::Continue;
    }

    let system_ctx = ctx.ctx.clone();
    let handle = ctx.ctx.services.rt.spawn(async move {
        command::execute(&system_ctx, Command::LibraryAnalyze)
            .await
            .map(|result| match result {
                CommandResult::LibraryAnalyzed { new_files_count } => new_files_count,
                _ => unreachable!(),
            })
            .change_context(RefreshError)
    });

    ctx.tui_state.start_refresh(handle);
    ctx.tui_state.status_bar.set("Refreshing library...");
    TuiActionResponse::Continue
}
