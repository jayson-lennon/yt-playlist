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

pub mod app;
pub mod cli;
pub mod command;
pub mod common;
pub mod feat;
pub mod services;
pub mod system_ctx;
pub mod tui;

pub use app::App;
pub use command::{Command, CommandError, CommandResult, execute, format_output};
pub use feat::config::{Config, ConfigError, load};
pub use feat::fuzzy_search::{FuzzySearch, FuzzySearchError, FuzzySearchResult, FuzzySearchService};
pub use feat::generate_show_notes::{FormatRegistry, ShowNotesEntry, ShowNotesFormat};
pub use feat::keymap::{Key, KeyCategory, KeyContext, Keymap, LeafBinding};
pub use tui::{ShowNoteKind, TuiAction};
pub use feat::launcher::{
    FileLauncher, FileLauncherService, LaunchError, LaunchResult, XdgLauncher,
};
pub use feat::media_query::{CachedMedia, MediaError, MediaQuery, MediaQueryService};
pub use feat::mpv::{MpvClient, MpvClientService, MpvError, MpvLauncherService};
pub use feat::playlist::{
    FileMetadata, IoError, PlaylistData, PlaylistStorage, PlaylistStorageService,
};
pub use feat::sources::{Source, SourceDb, SourceDbService};
pub use feat::terminal::{TerminalGuard, TerminalSuspendError, suspend_and_run};
pub use feat::tracing::{TracingInitError, init as init_tracing};
pub use feat::{ExternalEditor, ExternalEditorError};
pub use feat::{NoteDb, NoteDbError, PathResolutionError, PathResolver};
pub use services::Services;
pub use system_ctx::SystemCtx;
pub use tui::{Filter, LibraryPane, Pane, PlaylistItem, PlaylistPane, Rename, TuiState};

#[cfg(test)]
mod test_utils;
