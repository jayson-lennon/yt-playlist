use std::sync::Arc;

use derive_more::Debug;
use error_stack::Report;

use crate::feat::external_editor::{ExternalEditor, SystemEditor};
use crate::feat::launcher::FileLauncherService;
use crate::feat::media_query::MediaQuery;
use crate::feat::mpv::{MpvClient, MpvLauncher};
use crate::feat::note_db::{NoteDb, SqliteNoteDb, SqliteNoteDbError};
use crate::feat::path_resolver::{PathResolver, SystemPathResolver};
use crate::feat::playlist::PlaylistStorage;
use crate::feat::sources::{SourceDb, db::sqlite::SqliteSourceDb};

#[derive(Debug, Clone)]
pub struct Services {
    pub mpv: MpvClient,
    pub media: MediaQuery,
    pub storage: PlaylistStorage,
    pub mpv_launcher: MpvLauncher,
    pub file_launcher: FileLauncherService,
    pub db: NoteDb,
    pub editor: ExternalEditor,
    pub path_resolver: PathResolver,
    pub sources: SourceDb,
}

impl Services {
    pub async fn new(db_path: &str) -> Result<Self, Report<SqliteNoteDbError>> {
        let note_db = Arc::new(SqliteNoteDb::new(db_path).await?);
        let source_db = Arc::new(SqliteSourceDb::new(note_db.pool().clone()));
        let editor = Arc::new(SystemEditor);
        let path_resolver = Arc::new(SystemPathResolver);

        Ok(Self {
            mpv: MpvClient::new(Arc::new(crate::feat::mpv::MpvipcBackend::new(
                &std::path::PathBuf::new(),
            ))),
            media: MediaQuery::new(Arc::new(crate::feat::media_query::FfprobeBackend)),
            storage: PlaylistStorage::new(Arc::new(crate::feat::playlist::TomlBackend::new(
                std::path::PathBuf::new(),
            ))),
            mpv_launcher: MpvLauncher::new(Arc::new(crate::feat::mpv::RealMpvLauncher)),
            file_launcher: FileLauncherService::new(Arc::new(
                crate::feat::launcher::FileLauncher::new(),
            )),
            db: NoteDb::new(note_db),
            editor: ExternalEditor::new(editor),
            path_resolver: PathResolver::new(path_resolver),
            sources: SourceDb::new(source_db),
        })
    }
}
