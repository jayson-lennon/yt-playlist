use crate::app::App;
use crate::feat::keymap::ShowNoteKind;

pub fn handle_notes(app: &mut App) {
    if let Some(item) = app.tui_state.get_selected_item() {
        app.fork.notes_path = Some(item.path.clone());
    }
}

pub fn handle_fuzzy_notes(app: &mut App) {
    app.fork.fuzzy_notes = true;
}

pub fn handle_edit_sources(app: &mut App) {
    if let Some(item) = app.tui_state.get_selected_item() {
        app.fork.sources_path = Some(item.path.clone());
    }
}

pub fn handle_rename(app: &mut App) {
    app.tui_state.start_rename();
}

pub fn handle_generate_show_notes(app: &mut App, kind: ShowNoteKind) {
    app.fork.generate_notes = Some(kind.as_str().to_string());
}
