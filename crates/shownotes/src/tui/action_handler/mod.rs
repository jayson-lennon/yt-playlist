mod edit;
mod external;
mod general;
mod navigation;
mod playlist;

use crate::app::App;
use crate::tui::TuiAction;

pub fn dispatch(app: &mut App, action: TuiAction) {
    match action {
        TuiAction::ShowHelp => general::handle_show_help(app),
        TuiAction::Quit => general::handle_quit(app),
        TuiAction::Save => general::handle_save(app),
        TuiAction::StartFilter => general::handle_start_filter(app),
        TuiAction::MoveUp => navigation::handle_move_up(app),
        TuiAction::MoveDown => navigation::handle_move_down(app),
        TuiAction::SwitchPane => navigation::handle_switch_pane(app),
        TuiAction::FocusPlaylist => navigation::handle_focus_playlist(app),
        TuiAction::FocusLibrary => navigation::handle_focus_library(app),
        TuiAction::ShowAlias => general::handle_show_alias(app),
        TuiAction::ShowPath => general::handle_show_path(app),
        TuiAction::Rename => edit::handle_rename(app),
        TuiAction::Notes => edit::handle_notes(app),
        TuiAction::ReorderUp => playlist::handle_reorder_up(app),
        TuiAction::ReorderDown => playlist::handle_reorder_down(app),
        TuiAction::LaunchFile => external::handle_launch_file(app),
        TuiAction::LoadPlaylist => external::handle_load_playlist(app),
        TuiAction::MoveToLibrary => playlist::handle_move_to_library(app),
        TuiAction::MoveToPlaylist => playlist::handle_move_to_playlist(app),
        TuiAction::LaunchMpv => external::handle_launch_mpv(app),
        TuiAction::AddUrl => general::handle_add_url(app),
        TuiAction::Delete => general::handle_delete(app),
        TuiAction::FuzzyNotes => edit::handle_fuzzy_notes(app),
        TuiAction::EditSources => edit::handle_edit_sources(app),
        TuiAction::GenerateShowNotes(kind) => edit::handle_generate_show_notes(app, kind),
    }
}
