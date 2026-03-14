mod edit;
mod external;
mod general;
mod navigation;
mod playlist;

use error_stack::Report;

use crate::tui::TuiAction;
use crate::tui::TuiActionResponse;
use crate::Command;
use crate::CommandError;
use crate::CommandResult;

pub struct TuiActionCtx<'a> {
    pub tui_state: &'a mut crate::tui::TuiState,
    pub fork: &'a mut crate::app::Fork,
    pub ctx: &'a crate::system_ctx::SystemCtx,
}

impl TuiActionCtx<'_> {
    /// # Errors
    ///
    /// Returns an error if service execution fails
    pub fn execute(&mut self, command: Command) -> Result<CommandResult, Report<CommandError>> {
        self.ctx
            .services
            .rt
            .block_on(crate::command::execute(self.ctx, command))
    }
}

pub fn dispatch(ctx: &mut TuiActionCtx<'_>, action: TuiAction) -> TuiActionResponse {
    match action {
        TuiAction::ShowHelp => general::handle_show_help(ctx),
        TuiAction::Quit => general::handle_quit(ctx),
        TuiAction::Save => general::handle_save(ctx),
        TuiAction::StartFilter => general::handle_start_filter(ctx),
        TuiAction::MoveUp => navigation::handle_move_up(ctx),
        TuiAction::MoveDown => navigation::handle_move_down(ctx),
        TuiAction::SwitchPane => navigation::handle_switch_pane(ctx),
        TuiAction::FocusPlaylist => navigation::handle_focus_playlist(ctx),
        TuiAction::FocusLibrary => navigation::handle_focus_library(ctx),
        TuiAction::ShowAlias => general::handle_show_alias(ctx),
        TuiAction::ShowPath => general::handle_show_path(ctx),
        TuiAction::Rename => edit::handle_rename(ctx),
        TuiAction::Notes => edit::handle_notes(ctx),
        TuiAction::ReorderUp => playlist::handle_reorder_up(ctx),
        TuiAction::ReorderDown => playlist::handle_reorder_down(ctx),
        TuiAction::LaunchFile => external::handle_launch_file(ctx),
        TuiAction::LoadPlaylist => external::handle_load_playlist(ctx),
        TuiAction::MoveToLibrary => playlist::handle_move_to_library(ctx),
        TuiAction::MoveToPlaylist => playlist::handle_move_to_playlist(ctx),
        TuiAction::LaunchMpv => external::handle_launch_mpv(ctx),
        TuiAction::AddUrl => general::handle_add_url(ctx),
        TuiAction::Delete => general::handle_delete(ctx),
        TuiAction::FuzzyNotes => edit::handle_fuzzy_notes(ctx),
        TuiAction::EditSources => edit::handle_edit_sources(ctx),
        TuiAction::GenerateShowNotes(kind) => edit::handle_generate_show_notes(ctx, kind),
    }
}
