pub mod external_editor;
pub mod generate_show_notes;
pub mod launcher;
pub mod media_duration_analysis;
pub mod media_query;
pub mod mpv;
pub mod note_db;
pub mod path_resolver;
pub mod sources;

pub use generate_show_notes::{generate_show_notes, GenerateShowNotesError};
pub use sources::{Source, SourceDb, SourceDbWrapper};
pub use launcher::{FileLauncher, LaunchError, LaunchResult, Launcher, LauncherService};
pub use media_duration_analysis::{analyze_files, AnalysisResult};
pub use media_query::{CachedMediaBackend, MediaError, MediaQuery, MediaQueryBackend};
pub use external_editor::{ExternalEditor, ExternalEditorError, ExternalEditorWrapper, SystemEditor};
pub use mpv::{MpvBackend, MpvClient, MpvError, MpvLauncher, MpvLauncherService, MpvipcBackend};
pub use note_db::{NoteDb, NoteDbError, NoteDbWrapper, SqliteNoteDb, SqliteNoteDbError};
pub use path_resolver::{PathResolutionError, PathResolver, PathResolverWrapper, SystemPathResolver};
