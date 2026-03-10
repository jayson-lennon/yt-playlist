use std::sync::Arc;

use derive_more::Debug;
use error_stack::Report;

use crate::feat::external_editor::{ExternalEditorService, SystemEditor};
use crate::feat::fuzzy_search::{FuzzySearchService, backend::SkimBackend};
use crate::feat::launcher::FileLauncherService;
use crate::feat::media_query::MediaQueryService;
use crate::feat::mpv::{MpvClientService, MpvLauncherService};
use crate::feat::note_db::{NoteDbService, SqliteNoteDb, SqliteNoteDbError};
use crate::feat::path_resolver::{PathResolverService, SystemPathResolver};
use crate::feat::playlist::PlaylistStorageService;
use crate::feat::sources::{SourceDbService, db::sqlite::SqliteSourceDb};

#[derive(Debug, Clone)]
pub struct Services {
    pub mpv: MpvClientService,
    pub media: MediaQueryService,
    pub storage: PlaylistStorageService,
    pub mpv_launcher: MpvLauncherService,
    pub file_launcher: FileLauncherService,
    pub db: NoteDbService,
    pub editor: ExternalEditorService,
    pub path_resolver: PathResolverService,
    pub sources: SourceDbService,
    pub fuzzy_search: FuzzySearchService,
    pub rt: tokio::runtime::Handle,
}

impl Services {
    /// # Errors
    /// Returns an error if the database connection or migration fails.
    pub async fn new(db_path: &str, rt: tokio::runtime::Handle) -> Result<Self, Report<SqliteNoteDbError>> {
        let note_db = Arc::new(SqliteNoteDb::new(db_path).await?);
        let source_db = Arc::new(SqliteSourceDb::new(note_db.pool().clone()));
        let editor = Arc::new(SystemEditor);
        let path_resolver = Arc::new(SystemPathResolver);

        Ok(Self {
            mpv: MpvClientService::new(Arc::new(crate::feat::mpv::MpvIpc::new(
                &std::path::PathBuf::new(),
            ))),
            media: MediaQueryService::new(Arc::new(crate::feat::media_query::Ffprobe)),
            storage: PlaylistStorageService::new(Arc::new(
                crate::feat::playlist::TomlStorage::new(std::path::PathBuf::new()),
            )),
            mpv_launcher: MpvLauncherService::new(Arc::new(crate::feat::mpv::RealMpvLauncher)),
            file_launcher: FileLauncherService::new(Arc::new(
                crate::feat::launcher::XdgLauncher::new(),
            )),
            db: NoteDbService::new(note_db),
            editor: ExternalEditorService::new(editor),
            path_resolver: PathResolverService::new(path_resolver),
            sources: SourceDbService::new(source_db),
            fuzzy_search: FuzzySearchService::new(Arc::new(SkimBackend)),
            rt,
        })
    }
}
