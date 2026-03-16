use super::TuiActionCtx;
use crate::command::{Command, CommandResult};
use crate::tui::{ItemDisplayMode, TuiActionResponse};

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

pub fn handle_url_submit(ctx: &mut TuiActionCtx<'_>, url: String) -> TuiActionResponse {
    match ctx.execute(Command::UrlAdd { url }) {
        Ok(CommandResult::UrlAdded { item }) => {
            ctx.tui_state.library_pane.items.push(item);
            ctx.tui_state
                .library_pane
                .items
                .sort_by(|a, b| a.path.to_string_lossy().cmp(&b.path.to_string_lossy()));
            let _ = ctx.execute(Command::PlaylistSave {
                playlist_items: ctx.tui_state.playlist_pane.items.clone(),
                library_items: ctx.tui_state.library_pane.items.clone(),
            });
            ctx.tui_state.status_bar.set("URL added to library");
        }
        Err(e) => {
            ctx.tui_state
                .show_error(format!("Failed to add URL: {e:?}"));
        }
        _ => unreachable!(),
    }
    TuiActionResponse::Continue
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
