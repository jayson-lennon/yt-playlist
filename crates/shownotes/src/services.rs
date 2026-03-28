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

use derive_more::Debug;
use error_stack::Report;

use crate::feat::external_editor::{ExternalEditorService, SystemEditor};
use crate::feat::fuzzy_search::{FuzzySearchService, SkimBackend};
use crate::feat::launcher::FileLauncherService;
use crate::feat::media_query::MediaQueryService;
use crate::feat::mpv::{MpvClientService, MpvLauncherService};
use crate::feat::note_db::{NoteDb, NoteDbService, SqliteNoteDb, SqliteNoteDbError};
use crate::feat::path_resolver::{PathResolverService, SystemPathResolver};
use crate::feat::playlist::{PlaylistStorageService, SqliteStorage};
use crate::feat::sources::{SourceDbService, SqliteSourceDb};

/// Container for all injectable service dependencies.
///
/// Holds references to all the services used by the application, enabling
/// dependency injection and making it easy to swap implementations for testing.
/// Each service wraps a backend trait object that performs the actual work.
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
        let pool = note_db.pool();
        let source_db = Arc::new(SqliteSourceDb::new(pool.clone()));
        let storage = Arc::new(SqliteStorage::new(pool.clone()));
        let editor = Arc::new(SystemEditor);
        let path_resolver = Arc::new(SystemPathResolver);

        Ok(Self {
            mpv: MpvClientService::new(Arc::new(crate::feat::mpv::MpvIpc::new(
                &std::path::PathBuf::new(),
            ))),
            media: MediaQueryService::new(Arc::new(crate::feat::media_query::Ffprobe)),
            storage: PlaylistStorageService::new(storage),
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

    pub async fn close(&self) {
        self.db.close().await;
    }
}
