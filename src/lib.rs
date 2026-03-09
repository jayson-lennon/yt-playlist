pub mod app;
pub mod cli;
pub mod config;
pub mod feat;
pub mod keymap;
pub mod services;
pub mod tui;

pub use app::App;
pub use config::{Config, ConfigError, load};
pub use feat::generate_show_notes::format::{FormatRegistry, ShowNotesEntry, ShowNotesFormat};
pub use feat::launcher::{
    FileLauncher, FileLauncherService, LaunchError, LaunchResult, XdgLauncher,
};
pub use feat::media_query::{CachedMedia, MediaError, MediaQuery, MediaQueryService};
pub use feat::mpv::{MpvClient, MpvClientService, MpvError, MpvLauncherService};
pub use feat::playlist::{
    FileMetadata, IoError, PlaylistData, PlaylistStorage, PlaylistStorageService, TomlStorage,
};
pub use feat::sources::{Source, SourceDb, SourceDbService};
pub use feat::{ExternalEditor, ExternalEditorError};
pub use feat::{NoteDb, NoteDbError, PathResolutionError, PathResolver};
pub use keymap::{Action, Key, KeyCategory, KeyContext, Keymap, LeafBinding};
pub use services::Services;
pub use tui::{Filter, LibraryPane, Pane, PlaylistItem, PlaylistPane, Rename, TuiState};
