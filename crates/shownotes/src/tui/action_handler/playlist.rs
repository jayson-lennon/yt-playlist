use std::collections::HashMap;

use super::TuiActionCtx;
use super::TuiActionError;
use crate::command::playlist::get_item_counts;
use crate::command::Command;
use crate::common::domain::ItemPath;
use crate::tui::Pane;
use crate::tui::TuiActionResponse;
use error_stack::{Report, ResultExt};

pub fn handle_reorder_up(ctx: &mut TuiActionCtx<'_>) -> TuiActionResponse {
    if !ctx.tui_state.has_active_filter(Pane::Playlist) {
        ctx.tui_state.reorder_playlist_up();
        ctx.tui_state.needs_clear = true;
    }
    TuiActionResponse::Continue
}

pub fn handle_reorder_down(ctx: &mut TuiActionCtx<'_>) -> TuiActionResponse {
    if !ctx.tui_state.has_active_filter(Pane::Playlist) {
        ctx.tui_state.reorder_playlist_down();
        ctx.tui_state.needs_clear = true;
    }
    TuiActionResponse::Continue
}

pub fn handle_move_to_library(
    ctx: &mut TuiActionCtx<'_>,
) -> Result<TuiActionResponse, Report<TuiActionError>> {
    if let Some(item) = ctx.tui_state.selected_playlist_item().cloned() {
        let file_missing =
            !item.path.as_file().is_some_and(|p| p.as_path().exists()) && !item.is_virtual;
        if !file_missing {
            ctx.tui_state.library_pane.items.push(item);
            ctx.tui_state
                .library_pane
                .items
                .sort_by(|a, b| a.path.to_string_lossy().cmp(&b.path.to_string_lossy()));
        }
        ctx.tui_state.remove_from_playlist();
        if ctx.tui_state.playlist_pane.items.is_empty() {
            ctx.tui_state.focused_pane = Pane::Library;
        }
        ctx.tui_state.needs_clear = true;
        ctx.execute(Command::PlaylistSave {
            playlist_items: ctx.tui_state.playlist_pane.items.clone(),
            library_items: ctx.tui_state.library_pane.items.clone(),
        })
        .change_context(TuiActionError)?;

        recalculate_counts(ctx);
    }
    Ok(TuiActionResponse::Continue)
}

pub fn handle_move_to_playlist(
    ctx: &mut TuiActionCtx<'_>,
) -> Result<TuiActionResponse, Report<TuiActionError>> {
    if let Some(item) = ctx.tui_state.selected_library_item().cloned() {
        ctx.tui_state.add_to_playlist(
            item.path,
            item.duration,
            item.alias,
            item.mime_type,
            item.is_virtual,
            item.playlist_count.saturating_add(1),
        );
        ctx.tui_state.remove_from_library();
        if ctx.tui_state.library_pane.items.is_empty() {
            ctx.tui_state.focused_pane = Pane::Playlist;
        }
        ctx.tui_state.needs_clear = true;
        ctx.execute(Command::PlaylistSave {
            playlist_items: ctx.tui_state.playlist_pane.items.clone(),
            library_items: ctx.tui_state.library_pane.items.clone(),
        })
        .change_context(TuiActionError)?;

        recalculate_counts(ctx);
    }
    Ok(TuiActionResponse::Continue)
}

fn recalculate_counts(ctx: &mut TuiActionCtx<'_>) {
    let all_paths: Vec<ItemPath> = ctx
        .tui_state
        .playlist_pane
        .items
        .iter()
        .chain(ctx.tui_state.library_pane.items.iter())
        .map(|i| i.path.clone())
        .collect();

    let counts: HashMap<ItemPath, usize> = ctx
        .ctx
        .services
        .rt
        .block_on(get_item_counts(ctx.ctx, &all_paths));

    ctx.tui_state.update_counts(&counts);
}
