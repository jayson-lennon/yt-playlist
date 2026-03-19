mod edit;
mod external;
mod general;
mod navigation;
mod playlist;

use error_stack::Report;
use wherror::Error;

use crate::tui::TuiAction;
use crate::tui::TuiActionResponse;
use crate::Command;
use crate::CommandError;
use crate::CommandResult;

/// Error type for TUI action handlers.
#[derive(Debug, Error)]
#[error("TUI action failed")]
pub struct TuiActionError;

/// Context passed to action handlers.
///
/// Provides mutable access to TUI state and fork, and read access to system context
/// for executing commands and accessing services.
pub struct TuiActionCtx<'a> {
    /// Mutable TUI state for reading and modifying UI state.
    pub tui_state: &'a mut crate::tui::TuiState,
    /// Mutable fork for spawning external processes.
    pub fork: &'a mut crate::app::Fork,
    /// System context for executing commands and accessing services.
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

/// # Errors
///
/// Returns an error if action handling fails.
pub fn dispatch(
    ctx: &mut TuiActionCtx<'_>,
    action: TuiAction,
) -> Result<TuiActionResponse, Report<TuiActionError>> {
    match action {
        TuiAction::ShowHelp => Ok(general::handle_show_help(ctx)),
        TuiAction::Quit => Ok(general::handle_quit(ctx)),
        TuiAction::Save => Ok(general::handle_save(ctx)),
        TuiAction::StartFilter => Ok(general::handle_start_filter(ctx)),
        TuiAction::MoveUp => Ok(navigation::handle_move_up(ctx)),
        TuiAction::MoveDown => Ok(navigation::handle_move_down(ctx)),
        TuiAction::SwitchPane => Ok(navigation::handle_switch_pane(ctx)),
        TuiAction::FocusPlaylist => Ok(navigation::handle_focus_playlist(ctx)),
        TuiAction::FocusLibrary => Ok(navigation::handle_focus_library(ctx)),
        TuiAction::ShowAlias => Ok(general::handle_show_alias(ctx)),
        TuiAction::ShowPath => Ok(general::handle_show_path(ctx)),
        TuiAction::Rename => Ok(edit::handle_rename(ctx)),
        TuiAction::Notes => Ok(edit::handle_notes(ctx)),
        TuiAction::ReorderUp => Ok(playlist::handle_reorder_up(ctx)),
        TuiAction::ReorderDown => Ok(playlist::handle_reorder_down(ctx)),
        TuiAction::LaunchFile => Ok(external::handle_launch_file(ctx)),
        TuiAction::LoadPlaylist => Ok(external::handle_load_playlist(ctx)),
        TuiAction::MoveToLibrary => playlist::handle_move_to_library(ctx),
        TuiAction::MoveToPlaylist => playlist::handle_move_to_playlist(ctx),
        TuiAction::LaunchMpv => Ok(external::handle_launch_mpv(ctx)),
        TuiAction::AddUrl => Ok(general::handle_add_url(ctx)),
        TuiAction::Delete => Ok(general::handle_delete(ctx)),
        TuiAction::FuzzyNotes => Ok(edit::handle_fuzzy_notes(ctx)),
        TuiAction::EditSources => Ok(edit::handle_edit_sources(ctx)),
        TuiAction::GenerateShowNotes(kind) => Ok(edit::handle_generate_show_notes(ctx, kind)),
        TuiAction::RenameSubmit(alias) => edit::handle_rename_submit(ctx, alias),
        TuiAction::UrlSubmit(url) => general::handle_url_submit(ctx, url),
        TuiAction::TogglePlay => Ok(external::handle_toggle_play(ctx)),
        TuiAction::Refresh => Ok(general::handle_refresh(ctx)),
    }
}
