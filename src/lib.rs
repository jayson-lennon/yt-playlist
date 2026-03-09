pub mod app;
pub mod cli;
pub mod config;
pub mod feat;
pub mod keymap;
pub mod services;
pub mod tui_state;
pub mod ui;

pub use app::App;
pub use config::{Config, ConfigError, load};
pub use feat::generate_show_notes::format::{FormatRegistry, ShowNotesEntry, ShowNotesFormat};
pub use feat::launcher::{
    FileLauncher, FileLauncherBackend, FileLauncherService, LaunchError, LaunchResult,
};
pub use feat::media_query::{CachedMediaBackend, MediaError, MediaQuery, MediaQueryBackend};
pub use feat::mpv::{MpvBackend, MpvClient, MpvError, MpvLauncher};
pub use feat::playlist::{
    FileMetadata, IoError, PlaylistData, PlaylistStorage, PlaylistStorageBackend, TomlBackend,
};
pub use feat::sources::{Source, SourceDb, SourceDbBackend};
pub use feat::{ExternalEditorBackend, ExternalEditorError};
pub use feat::{NoteDbBackend, NoteDbError, PathResolutionError, PathResolverBackend};
pub use keymap::{Action, Key, KeyCategory, KeyContext, Keymap, LeafBinding};
pub use services::Services;
pub use tui_state::TuiState;
pub use ui::{Filter, LibraryPane, Pane, PlaylistItem, PlaylistPane, Rename};
