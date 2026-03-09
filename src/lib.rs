pub mod app;
pub mod cli;
pub mod config;
pub mod feat;
pub mod keymap;
pub mod playlist;
pub mod services;
pub mod tui_state;
pub mod ui;

pub use app::App;
pub use config::{load, Config, ConfigError};
pub use feat::launcher::{FileLauncher, LaunchError, LaunchResult, Launcher, LauncherService};
pub use feat::generate_show_notes::format::{FormatRegistry, ShowNotesEntry, ShowNotesFormat};
pub use keymap::{Action, Key, KeyCategory, KeyContext, Keymap, LeafBinding};
pub use feat::media_query::{CachedMediaBackend, MediaError, MediaQuery, MediaQueryBackend};
pub use feat::mpv::{MpvBackend, MpvClient, MpvError, MpvLauncherService};
pub use feat::{NoteDb, NoteDbError, PathResolutionError, PathResolver};
pub use feat::{ExternalEditor, ExternalEditorError};
pub use playlist::{
    FileMetadata, IoError, PlaylistData, PlaylistStorage, PlaylistStorageBackend, TomlBackend,
};
pub use services::Services;
pub use feat::sources::{Source, SourceDb, SourceDbWrapper};
pub use tui_state::TuiState;
pub use ui::{Filter, LibraryPane, Pane, PlaylistItem, PlaylistPane, Rename};
