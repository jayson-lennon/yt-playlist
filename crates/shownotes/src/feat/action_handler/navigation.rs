use crate::app::App;
use crate::tui::Pane;

pub fn handle_move_up(app: &mut App) {
    match app.tui_state.focused_pane {
        Pane::Playlist => app.tui_state.move_playlist_up(),
        Pane::Library => app.tui_state.move_library_up(),
    }
}

pub fn handle_move_down(app: &mut App) {
    match app.tui_state.focused_pane {
        Pane::Playlist => app.tui_state.move_playlist_down(),
        Pane::Library => app.tui_state.move_library_down(),
    }
}

pub fn handle_switch_pane(app: &mut App) {
    app.tui_state.switch_pane();
}

pub fn handle_focus_playlist(app: &mut App) {
    if !app.tui_state.playlist_pane.items.is_empty() {
        app.tui_state.focused_pane = Pane::Playlist;
    }
}

pub fn handle_focus_library(app: &mut App) {
    if !app.tui_state.library_pane.items.is_empty() {
        app.tui_state.focused_pane = Pane::Library;
    }
}
