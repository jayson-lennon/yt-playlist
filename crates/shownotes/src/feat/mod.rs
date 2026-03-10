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

pub use config::{Config, ConfigError, load};

pub use external_editor::{
    ExternalEditor, ExternalEditorError, ExternalEditorService, SystemEditor,
};
pub use fuzzy_search::{FuzzySearch, FuzzySearchError, FuzzySearchResult, FuzzySearchService};
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
    TomlStorage,
};
pub use sources::{Source, SourceDb, SourceDbService};
pub use symlink::{SymlinkError, SymlinkResult, create_symlink_with_suffix};
pub use terminal::{TerminalGuard, TerminalSuspendError, suspend_and_run};
