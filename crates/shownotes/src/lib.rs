pub mod app;
pub mod cli;
pub mod feat;
pub mod services;
pub mod tui;

pub use cli::sources::add::handle_add_command;
pub use cli::sources::common::resolve_and_get_file_path;
pub use cli::sources::edit::handle_edit_command;

pub use app::App;
pub use feat::config::{Config, ConfigError, load};
pub use feat::fuzzy_search::{FuzzySearch, FuzzySearchError, FuzzySearchResult, FuzzySearchService};
pub use feat::generate_show_notes::format::{FormatRegistry, ShowNotesEntry, ShowNotesFormat};
pub use feat::keymap::{Action, Key, KeyCategory, KeyContext, Keymap, LeafBinding};
pub use feat::launcher::{
    FileLauncher, FileLauncherService, LaunchError, LaunchResult, XdgLauncher,
};
pub use feat::media_query::{CachedMedia, MediaError, MediaQuery, MediaQueryService};
pub use feat::mpv::{MpvClient, MpvClientService, MpvError, MpvLauncherService};
pub use feat::playlist::{
    FileMetadata, IoError, PlaylistData, PlaylistStorage, PlaylistStorageService, TomlStorage,
};
pub use feat::sources::{Source, SourceDb, SourceDbService};
pub use feat::terminal::{TerminalGuard, TerminalSuspendError, suspend_and_run};
pub use feat::{ExternalEditor, ExternalEditorError};
pub use feat::{NoteDb, NoteDbError, PathResolutionError, PathResolver};
pub use services::Services;
pub use tui::{Filter, LibraryPane, Pane, PlaylistItem, PlaylistPane, Rename, TuiState};
