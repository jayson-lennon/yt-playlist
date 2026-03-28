// Copyright (C) 2026 Jayson Lennon
// 
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as
// published by the Free Software Foundation, either version 3 of the
// License, or (at your option) any later version.
// 
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
// 
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

//! TUI Action System
//!
//! This module defines all user-triggered actions in the TUI layer, representing
//! user intent decoupled from key input handling and execution.
//!
//! # Core Types
//!
//! - [`TuiAction`]: Represents every action a user can trigger via keybindings,
//!   including navigation, editing, playlist management, and external operations.
//! - [`TuiActionResponse`]: Signals whether the application should continue running
//!   or quit after processing an action.
//! - [`ShowNoteKind`]: Specifies output format (HTML/Markdown) for show note generation.
//!
//! # Architecture Role
//!
//! This module sits between the keymap layer and the action handler layer:
//!
//! ```text
//! ┌─────────────┐    ┌─────────────┐    ┌──────────────────┐
//! │   Keymap    │───▶│  TuiAction  │───▶│ Action Handler   │
//! │ (key input) │    │  (intent)   │    │ (dispatch/exec)  │
//! └─────────────┘    └─────────────┘    └──────────────────┘
//! ```
//!
//! This design decouples "what the user wants to do" from "which key triggers it"
//! and "how it's executed", enabling keybinding customization without changing
//! action logic.
//!
//! # Flow Through System
//!
//! 1. User presses key → Keymap lookup → [`TuiAction`] produced
//! 2. [`TuiAction`] → `action_handler::dispatch()` → Domain-specific handler
//! 3. Handler may construct [`Command`] and call `execute()` for side effects
//! 4. Handler returns [`TuiActionResponse`] (Continue or ShouldQuit)
//!
//! # Relationship to Command Module
//!
//! - [`TuiAction`] represents user intent in the TUI context
//! - [`Command`] represents executable operations with business logic
//! - Some [`TuiAction`]s trigger [`Command`]s (e.g., `LaunchFile`, `Save`)
//! - Some [`TuiAction`]s are UI-only (e.g., `MoveUp`, `SwitchPane`)
//!
//! [`Command`]: crate::tui::command::Command

/// Output format for generating show notes.
///
/// Specifies the format to use when exporting playlist notes,
/// supporting HTML, Markdown, and plain text output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ShowNoteKind {
    Html,
    Markdown,
    Youtube,
}

impl ShowNoteKind {
    pub fn as_str(self) -> &'static str {
        match self {
            ShowNoteKind::Html => "html",
            ShowNoteKind::Markdown => "markdown",
            ShowNoteKind::Youtube => "youtube",
        }
    }
}

/// Response from handling a TUI action.
///
/// Indicates whether the application should continue running
/// or quit after processing an action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TuiActionResponse {
    Continue,
    ShouldQuit,
}

/// All available actions in the TUI.
///
/// Represents every user action that can be triggered by keybindings,
/// from navigation and pane switching to launching mpv and editing notes.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TuiAction {
    Quit,
    Save,
    ShowHelp,
    ShowAlias,
    ShowPath,
    StartFilter,
    MoveUp,
    MoveDown,
    SwitchPane,
    TogglePlay,
    FocusPlaylist,
    FocusLibrary,
    Rename,
    Notes,
    ReorderUp,
    ReorderDown,
    LaunchFile,
    LoadPlaylist,
    MoveToLibrary,
    MoveToPlaylist,
    LaunchMpv,
    AddUrl,
    Delete,
    FuzzyNotes,
    EditSources,
    GenerateShowNotes(ShowNoteKind),
    RenameSubmit(String),
    UrlSubmit(String),
    Refresh,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rename_submit_variant_exists() {
        // Given a RenameSubmit action.
        let action = TuiAction::RenameSubmit("new_name".to_string());

        // Then it matches the expected variant.
        assert!(matches!(action, TuiAction::RenameSubmit(_)));
    }

    #[test]
    fn url_submit_variant_exists() {
        // Given a UrlSubmit action.
        let action = TuiAction::UrlSubmit("https://example.com".to_string());

        // Then it matches the expected variant.
        assert!(matches!(action, TuiAction::UrlSubmit(_)));
    }
}
