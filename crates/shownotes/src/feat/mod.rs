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

pub mod config;
pub mod external_editor;
pub mod fuzzy_search;
pub mod generate_show_notes;
pub mod keymap;
pub mod launcher;
pub mod media_duration_analysis;
pub mod media_query;
pub mod mpv;
pub mod note_db;
pub mod path_resolver;
pub mod playlist;
pub mod sources;
pub mod symlink;
pub mod terminal;
pub mod tracing;

pub use config::{Config, ConfigError, load};

pub use external_editor::{
    ExternalEditor, ExternalEditorError, ExternalEditorService, SystemEditor,
};
pub use fuzzy_search::{FuzzySearch, FuzzySearchError, FuzzySearchResult, FuzzySearchService, SkimBackend};
pub use generate_show_notes::{GenerateShowNotesError, generate_show_notes};
pub use launcher::{FileLauncher, FileLauncherService, LaunchError, LaunchResult, XdgLauncher};
pub use media_duration_analysis::{AnalysisResult, analyze_files};
pub use media_query::{CachedMedia, MediaError, MediaQuery, MediaQueryService};
pub use mpv::{
    MpvClient, MpvClientService, MpvError, MpvLauncher, MpvLauncherService, MpvIpc,
};
pub use note_db::{NoteDb, NoteDbError, NoteDbService, SqliteNoteDb, SqliteNoteDbError};
pub use path_resolver::{
    PathResolutionError, PathResolver, PathResolverService, SystemPathResolver,
};
pub use playlist::{
    FileMetadata, IoError, PlaylistData, PlaylistStorage, PlaylistStorageService,
    SqliteStorage,
};
pub use sources::{Source, SourceDb, SourceDbService, SqliteSourceDb, SqliteSourceDbError};
pub use symlink::{SymlinkError, SymlinkResult, create_symlink_with_suffix};
pub use terminal::{TerminalGuard, TerminalSuspendError, suspend_and_run};
