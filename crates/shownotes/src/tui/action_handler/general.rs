use super::TuiActionCtx;
use super::TuiActionError;
use crate::command::{Command, CommandResult};
use crate::tui::{ItemDisplayMode, TuiActionResponse};
use error_stack::{Report, ResultExt};

pub fn handle_quit(
    ctx: &mut TuiActionCtx<'_>,
) -> Result<TuiActionResponse, Report<TuiActionError>> {
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
    Ok(TuiActionResponse::ShouldQuit)
}

pub fn handle_save(
    ctx: &mut TuiActionCtx<'_>,
) -> Result<TuiActionResponse, Report<TuiActionError>> {
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
    Ok(TuiActionResponse::Continue)
}

pub fn handle_show_help(
    ctx: &mut TuiActionCtx<'_>,
) -> Result<TuiActionResponse, Report<TuiActionError>> {
    ctx.tui_state.global_handler.toggle_help();
    Ok(TuiActionResponse::Continue)
}

pub fn handle_start_filter(
    ctx: &mut TuiActionCtx<'_>,
) -> Result<TuiActionResponse, Report<TuiActionError>> {
    ctx.tui_state.start_filter();
    Ok(TuiActionResponse::Continue)
}

pub fn handle_show_alias(
    ctx: &mut TuiActionCtx<'_>,
) -> Result<TuiActionResponse, Report<TuiActionError>> {
    ctx.tui_state.display_mode = ItemDisplayMode::Alias;
    Ok(TuiActionResponse::Continue)
}

pub fn handle_show_path(
    ctx: &mut TuiActionCtx<'_>,
) -> Result<TuiActionResponse, Report<TuiActionError>> {
    ctx.tui_state.display_mode = ItemDisplayMode::Path;
    Ok(TuiActionResponse::Continue)
}

pub fn handle_add_url(
    ctx: &mut TuiActionCtx<'_>,
) -> Result<TuiActionResponse, Report<TuiActionError>> {
    ctx.tui_state.start_url_input();
    Ok(TuiActionResponse::Continue)
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

pub fn handle_delete(
    ctx: &mut TuiActionCtx<'_>,
) -> Result<TuiActionResponse, Report<TuiActionError>> {
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
    Ok(TuiActionResponse::Continue)
}
