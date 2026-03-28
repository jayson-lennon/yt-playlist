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

use std::collections::HashSet;
use std::sync::Arc;

use async_trait::async_trait;
use derive_more::Debug;
use error_stack::Report;

use super::{NoteDb, NoteDbError};

mod sqlite;

pub use sqlite::{SqliteNoteDb, SqliteNoteDbError};

/// Service for managing file notes database operations.
///
/// Provides an interface for storing and retrieving notes associated
/// with media files. Delegates to a backend implementation (SQLite)
/// for actual database operations.
#[derive(Debug, Clone)]
pub struct NoteDbService {
    #[debug("<NoteDb>")]
    backend: Arc<dyn NoteDb>,
}

impl NoteDbService {
    pub fn new(backend: Arc<dyn NoteDb>) -> Self {
        Self { backend }
    }
}

#[async_trait]
impl NoteDb for NoteDbService {
    async fn get_or_create_file_path(&self, path: &str) -> Result<i64, Report<NoteDbError>> {
        self.backend.get_or_create_file_path(path).await
    }

    async fn get_note(&self, file_path_id: i64) -> Result<Option<String>, Report<NoteDbError>> {
        self.backend.get_note(file_path_id).await
    }

    async fn upsert_note(
        &self,
        file_path_id: i64,
        content: &str,
    ) -> Result<(), Report<NoteDbError>> {
        self.backend.upsert_note(file_path_id, content).await
    }

    async fn search_notes(&self, query: &str) -> Result<HashSet<String>, Report<NoteDbError>> {
        self.backend.search_notes(query).await
    }

    async fn get_all_notes_with_paths(&self) -> Result<Vec<(String, String)>, Report<NoteDbError>> {
        self.backend.get_all_notes_with_paths().await
    }

    async fn get_all_paths_for_fuzzy_search(&self) -> Result<Vec<(String, String)>, Report<NoteDbError>> {
        self.backend.get_all_paths_for_fuzzy_search().await
    }

    async fn close(&self) {
        self.backend.close().await;
    }
}
