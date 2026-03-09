pub mod app;
pub mod cli;
pub mod config;
pub mod feat;
pub mod format;
pub mod keymap;
pub mod notes;
pub mod playlist;
pub mod services;
pub mod sources;
pub mod tui_state;
pub mod ui;

pub use app::App;
pub use config::{load, Config, ConfigError};
pub use feat::launcher::{FileLauncher, LaunchError, LaunchResult, Launcher, LauncherService};
pub use format::{FormatRegistry, ShowNotesEntry, ShowNotesFormat};
pub use keymap::{Action, Key, KeyCategory, KeyContext, Keymap, LeafBinding};
pub use feat::media_query::{CachedMediaBackend, MediaError, MediaQuery, MediaQueryBackend};
pub use feat::mpv::{MpvBackend, MpvClient, MpvError, MpvLauncherService};
pub use notes::{
    Editor, EditorError, NoteDb, NoteDbError, PathResolutionError, PathResolver,
    SystemServicesHandle,
};
pub use playlist::{
    FileMetadata, IoError, PlaylistData, PlaylistStorage, PlaylistStorageBackend, TomlBackend,
};
pub use services::Services;
pub use sources::{Source, SourceDb, SourceDbWrapper};
pub use tui_state::TuiState;
pub use ui::{Filter, LibraryPane, Pane, PlaylistItem, PlaylistPane, Rename};
