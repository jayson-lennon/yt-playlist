use std::sync::Arc;

use derive_more::Debug;
use error_stack::Report;

use crate::feat::external_editor::{ExternalEditorWrapper, SystemEditor};
use crate::feat::launcher::LauncherService;
use crate::feat::media_query::MediaQuery;
use crate::feat::mpv::{MpvClient, MpvLauncherService};
use crate::feat::note_db::{NoteDbWrapper, SqliteNoteDb, SqliteNoteDbError};
use crate::feat::path_resolver::{PathResolverWrapper, SystemPathResolver};
use crate::feat::sources::{SourceDbWrapper, db::sqlite::SqliteSourceDb};
use crate::playlist::PlaylistStorage;

#[derive(Debug, Clone)]
pub struct Services {
    pub mpv: MpvClient,
    pub media: MediaQuery,
    pub storage: PlaylistStorage,
    pub mpv_launcher: MpvLauncherService,
    pub file_launcher: LauncherService,
    pub db: NoteDbWrapper,
    pub editor: ExternalEditorWrapper,
    pub path_resolver: PathResolverWrapper,
    pub sources: SourceDbWrapper,
}

impl Services {
    pub async fn new(db_path: &str) -> Result<Self, Report<SqliteNoteDbError>> {
        let note_db = Arc::new(SqliteNoteDb::new(db_path).await?);
        let source_db = Arc::new(SqliteSourceDb::new(note_db.pool().clone()));
        let editor = Arc::new(SystemEditor);
        let path_resolver = Arc::new(SystemPathResolver);

        Ok(Self {
            mpv: MpvClient::new(Arc::new(crate::feat::mpv::MpvipcBackend::new(&std::path::PathBuf::new()))),
            media: MediaQuery::new(Arc::new(crate::feat::media_query::FfprobeBackend)),
            storage: PlaylistStorage::new(Arc::new(crate::playlist::TomlBackend::new(std::path::PathBuf::new()))),
            mpv_launcher: MpvLauncherService::new(Arc::new(crate::feat::mpv::RealMpvLauncher)),
            file_launcher: LauncherService::new(Arc::new(crate::feat::launcher::FileLauncher::new())),
            db: NoteDbWrapper::new(note_db),
            editor: ExternalEditorWrapper::new(editor),
            path_resolver: PathResolverWrapper::new(path_resolver),
            sources: SourceDbWrapper::new(source_db),
        })
    }
}
