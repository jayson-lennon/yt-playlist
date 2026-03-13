use crate::app::App;
use crate::tui::Pane;

pub fn handle_reorder_up(app: &mut App) {
    if !app.tui_state.has_active_filter(Pane::Playlist) {
        app.tui_state.reorder_playlist_up();
        app.tui_state.needs_clear = true;
    }
}

pub fn handle_reorder_down(app: &mut App) {
    if !app.tui_state.has_active_filter(Pane::Playlist) {
        app.tui_state.reorder_playlist_down();
        app.tui_state.needs_clear = true;
    }
}

pub fn handle_move_to_library(app: &mut App) {
    if let Some(item) = app.tui_state.selected_playlist_item().cloned() {
        let file_missing =
            !item.path.as_file().is_some_and(|p| p.as_path().exists()) && !item.is_virtual;
        if !file_missing {
            app.tui_state.library_pane.items.push(item);
            app.tui_state
                .library_pane
                .items
                .sort_by(|a, b| a.path.to_string_lossy().cmp(&b.path.to_string_lossy()));
        }
        app.tui_state.remove_from_playlist();
        if app.tui_state.playlist_pane.items.is_empty() {
            app.tui_state.focused_pane = Pane::Library;
        }
        app.tui_state.needs_clear = true;
        app.save_playlist();
    }
}

pub fn handle_move_to_playlist(app: &mut App) {
    if let Some(item) = app.tui_state.selected_library_item().cloned() {
        app.tui_state.add_to_playlist(
            item.path,
            item.duration,
            item.alias,
            item.mime_type,
            item.is_virtual,
        );
        app.tui_state.remove_from_library();
        if app.tui_state.library_pane.items.is_empty() {
            app.tui_state.focused_pane = Pane::Playlist;
        }
        app.tui_state.needs_clear = true;
        app.save_playlist();
    }
}
