use crate::app::App;
use crate::command::{Command, CommandResult};
use crate::tui::ItemDisplayMode;

pub fn handle_quit(app: &mut App) {
    match app.execute(Command::PlaylistSave {
        playlist_items: app.tui_state.playlist_pane.items.clone(),
        library_items: app.tui_state.library_pane.items.clone(),
    }) {
        Ok(CommandResult::PlaylistSaved) => {
            app.tui_state.status_bar.set("Playlist saved");
        }
        Err(e) => {
            app.tui_state.show_error(format!("Failed to save: {e:?}"));
        }
        _ => unreachable!(),
    }
    app.should_quit = true;
}

pub fn handle_save(app: &mut App) {
    match app.execute(Command::PlaylistSave {
        playlist_items: app.tui_state.playlist_pane.items.clone(),
        library_items: app.tui_state.library_pane.items.clone(),
    }) {
        Ok(CommandResult::PlaylistSaved) => {
            app.tui_state.status_bar.set("Playlist saved");
        }
        Err(e) => {
            app.tui_state.show_error(format!("Failed to save: {e:?}"));
        }
        _ => unreachable!(),
    }
}

pub fn handle_show_help(app: &mut App) {
    app.tui_state.global_handler.toggle_help();
}

pub fn handle_start_filter(app: &mut App) {
    app.tui_state.start_filter();
}

pub fn handle_show_alias(app: &mut App) {
    app.tui_state.display_mode = ItemDisplayMode::Alias;
}

pub fn handle_show_path(app: &mut App) {
    app.tui_state.display_mode = ItemDisplayMode::Path;
}

pub fn handle_add_url(app: &mut App) {
    app.tui_state.start_url_input();
}

pub fn handle_delete(app: &mut App) {
    if let Some(item) = app.tui_state.selected_library_item() {
        if item.is_virtual {
            app.tui_state.library_pane.remove();
            match app.execute(Command::PlaylistSave {
                playlist_items: app.tui_state.playlist_pane.items.clone(),
                library_items: app.tui_state.library_pane.items.clone(),
            }) {
                Ok(CommandResult::PlaylistSaved) => {
                    app.tui_state.status_bar.set("Virtual entry deleted");
                }
                Err(e) => {
                    app.tui_state.show_error(format!("Failed to save: {e:?}"));
                }
                _ => unreachable!(),
            }
        } else {
            app.tui_state
                .status_bar
                .set("Only virtual entries (URLs) can be deleted.");
        }
    }
}
