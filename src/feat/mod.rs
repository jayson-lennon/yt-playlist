pub mod external_editor;
pub mod generate_show_notes;
pub mod launcher;
pub mod media_duration_analysis;
pub mod media_query;
pub mod mpv;
pub mod note_db;
pub mod path_resolver;
pub mod playlist;
pub mod sources;

pub use external_editor::{
    ExternalEditor, ExternalEditorBackend, ExternalEditorError, SystemEditor,
};
pub use generate_show_notes::{GenerateShowNotesError, generate_show_notes};
pub use launcher::{
    FileLauncher, FileLauncherBackend, FileLauncherService, LaunchError, LaunchResult,
};
pub use media_duration_analysis::{AnalysisResult, analyze_files};
pub use media_query::{CachedMediaBackend, MediaError, MediaQuery, MediaQueryBackend};
pub use mpv::{MpvBackend, MpvClient, MpvError, MpvLauncher, MpvLauncherBackend, MpvipcBackend};
pub use note_db::{NoteDb, NoteDbBackend, NoteDbError, SqliteNoteDb, SqliteNoteDbError};
pub use path_resolver::{
    PathResolutionError, PathResolver, PathResolverBackend, SystemPathResolver,
};
pub use playlist::{
    FileMetadata, IoError, PlaylistData, PlaylistStorage, PlaylistStorageBackend, TomlBackend,
};
pub use sources::{Source, SourceDb, SourceDbBackend};
