use crate::command::{self, Command, CommandResult};
use crate::system_ctx::SystemCtx;
use crate::tui::{Pane, TuiState};

pub fn load_playlist(ctx: &SystemCtx, tui_state: &mut TuiState) {
    match ctx
        .services
        .rt
        .block_on(command::execute(ctx, Command::PlaylistLoad))
    {
        Ok(CommandResult::PlaylistLoaded {
            playlist_items,
            virtual_library_items,
        }) => {
            tui_state.playlist_pane.items = playlist_items;
            refresh_library(ctx, tui_state);
            for item in virtual_library_items {
                tui_state.library_pane.items.push(item);
            }
            tui_state
                .library_pane
                .items
                .sort_by(|a, b| a.path.to_string_lossy().cmp(&b.path.to_string_lossy()));
        }
        Err(e) => {
            tui_state.show_error(format!("Failed to load playlist: {e:?}"));
        }
        _ => unreachable!(),
    }
}

pub fn refresh_library(ctx: &SystemCtx, tui_state: &mut TuiState) {
    match ctx
        .services
        .rt
        .block_on(command::execute(ctx, Command::LibraryRefresh))
    {
        Ok(CommandResult::LibraryRefreshed { items }) => {
            tui_state.refresh_library(items);
        }
        Err(e) => {
            tui_state.show_error(format!("Failed to refresh library: {e:?}"));
        }
        _ => unreachable!(),
    }
}

pub fn handle_rename_submit(ctx: &SystemCtx, tui_state: &mut TuiState, new_alias: String) {
    if let Some(item) = tui_state.get_selected_item_mut() {
        let old_alias = item.alias.clone();
        item.alias = Some(new_alias.clone());

        if old_alias.as_deref() != Some(&new_alias) {
            let path = item.path.clone();

            if let Some(file_path) = path.as_file() {
                let _ = ctx.services.rt.block_on(command::execute(
                    ctx,
                    Command::AliasSet {
                        path: file_path.clone(),
                        workspace: ctx.library_path.clone(),
                        alias: new_alias,
                    },
                ));
            }
        }
    }
}

pub fn handle_url_submit(ctx: &SystemCtx, tui_state: &mut TuiState, url: String) {
    match ctx
        .services
        .rt
        .block_on(command::execute(ctx, Command::UrlAdd { url }))
    {
        Ok(CommandResult::UrlAdded { item }) => {
            tui_state.library_pane.items.push(item);
            tui_state
                .library_pane
                .items
                .sort_by(|a, b| a.path.to_string_lossy().cmp(&b.path.to_string_lossy()));
            let _ = ctx.services.rt.block_on(command::execute(
                ctx,
                Command::PlaylistSave {
                    playlist_items: tui_state.playlist_pane.items.clone(),
                    library_items: tui_state.library_pane.items.clone(),
                },
            ));
            tui_state.status_bar.set("URL added to library");
        }
        Err(e) => {
            tui_state.show_error(format!("Failed to add URL: {e:?}"));
        }
        _ => unreachable!(),
    }
}

pub fn set_initial_focus(tui_state: &mut TuiState) {
    let playlist_empty = tui_state.playlist_pane.items.is_empty();
    let library_empty = tui_state.library_pane.items.is_empty();
    if playlist_empty && !library_empty {
        tui_state.focused_pane = Pane::Library;
    } else if library_empty && !playlist_empty {
        tui_state.focused_pane = Pane::Playlist;
    }
}
