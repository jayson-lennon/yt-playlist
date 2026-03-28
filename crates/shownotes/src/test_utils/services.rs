// Copyright (C) 2026 Jayson Lennon
// 
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as
// published by the Free Software Foundation, either version 3 of the
// License, or (at your option) any later version.
// 
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
// 
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

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
