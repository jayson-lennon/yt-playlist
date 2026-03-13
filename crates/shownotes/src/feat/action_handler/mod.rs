mod edit;
mod external;
mod general;
mod navigation;
mod playlist;

use crate::app::App;
use crate::feat::keymap::Action;

pub fn dispatch(app: &mut App, action: Action) {
    match action {
        Action::ShowHelp => general::handle_show_help(app),
        Action::Quit => general::handle_quit(app),
        Action::Save => general::handle_save(app),
        Action::StartFilter => general::handle_start_filter(app),
        Action::MoveUp => navigation::handle_move_up(app),
        Action::MoveDown => navigation::handle_move_down(app),
        Action::SwitchPane => navigation::handle_switch_pane(app),
        Action::FocusPlaylist => navigation::handle_focus_playlist(app),
        Action::FocusLibrary => navigation::handle_focus_library(app),
        Action::ShowAlias => general::handle_show_alias(app),
        Action::ShowPath => general::handle_show_path(app),
        Action::Rename => edit::handle_rename(app),
        Action::Notes => edit::handle_notes(app),
        Action::ReorderUp => playlist::handle_reorder_up(app),
        Action::ReorderDown => playlist::handle_reorder_down(app),
        Action::LaunchFile => external::handle_launch_file(app),
        Action::LoadPlaylist => external::handle_load_playlist(app),
        Action::MoveToLibrary => playlist::handle_move_to_library(app),
        Action::MoveToPlaylist => playlist::handle_move_to_playlist(app),
        Action::LaunchMpv => external::handle_launch_mpv(app),
        Action::AddUrl => general::handle_add_url(app),
        Action::Delete => general::handle_delete(app),
        Action::FuzzyNotes => edit::handle_fuzzy_notes(app),
        Action::EditSources => edit::handle_edit_sources(app),
        Action::GenerateShowNotes(kind) => edit::handle_generate_show_notes(app, kind),
    }
}
