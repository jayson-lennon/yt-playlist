use std::sync::Arc;

use crate::feat::external_editor::{ExternalEditorService, SystemEditor};
use crate::feat::fuzzy_search::{FuzzySearchService, SkimBackend};
use crate::feat::launcher::FileLauncherService;
use crate::feat::media_query::MediaQueryService;
use crate::feat::mpv::{MpvClientService, MpvLauncherService};
use crate::feat::note_db::{NoteDbService, SqliteNoteDb};
use crate::feat::path_resolver::{PathResolverService, SystemPathResolver};
use crate::feat::playlist::PlaylistStorageService;
use crate::feat::sources::{SourceDbService, SqliteSourceDb};
use crate::services::Services;

use super::fakes::{FakeLauncher, FakeMediaBackend, FakeMpvBackend, FakeMpvLauncher, FakeStorageBackend};

pub async fn create_test_services() -> Services {
    let launcher = Arc::new(FakeLauncher::new());
    create_test_services_with_launcher(launcher).await
}

pub async fn create_test_services_with_launcher(launcher: Arc<FakeLauncher>) -> Services {
    let db = Arc::new(SqliteNoteDb::new("sqlite::memory:").await.unwrap());
    let path_resolver = Arc::new(SystemPathResolver);
    let rt = tokio::runtime::Handle::current();

    Services {
        mpv: MpvClientService::new(Arc::new(FakeMpvBackend)),
        media: MediaQueryService::new(Arc::new(FakeMediaBackend)),
        storage: PlaylistStorageService::new(Arc::new(FakeStorageBackend)),
        mpv_launcher: MpvLauncherService::new(Arc::new(FakeMpvLauncher::new())),
        file_launcher: FileLauncherService::new(launcher),
        db: NoteDbService::new(db.clone()),
        editor: ExternalEditorService::new(Arc::new(SystemEditor)),
        path_resolver: PathResolverService::new(path_resolver),
        sources: SourceDbService::new(Arc::new(SqliteSourceDb::new(db.pool().clone()))),
        fuzzy_search: FuzzySearchService::new(Arc::new(SkimBackend)),
        rt,
    }
}
