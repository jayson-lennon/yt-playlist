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

pub fn set_initial_focus(tui_state: &mut TuiState) {
    let playlist_empty = tui_state.playlist_pane.items.is_empty();
    let library_empty = tui_state.library_pane.items.is_empty();
    if playlist_empty && !library_empty {
        tui_state.focused_pane = Pane::Library;
    } else if library_empty && !playlist_empty {
        tui_state.focused_pane = Pane::Playlist;
    }
}
