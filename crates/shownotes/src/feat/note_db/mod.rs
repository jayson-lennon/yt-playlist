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

use async_trait::async_trait;
use error_stack::Report;
use wherror::Error;

mod backend;

pub use backend::{NoteDbService, SqliteNoteDb, SqliteNoteDbError};

#[derive(Debug, Error)]
#[error(debug)]
pub struct NoteDbError;

#[async_trait]
pub trait NoteDb: Send + Sync {
    async fn get_or_create_file_path(&self, path: &str) -> Result<i64, Report<NoteDbError>>;
    async fn get_note(&self, file_path_id: i64) -> Result<Option<String>, Report<NoteDbError>>;
    async fn upsert_note(
        &self,
        file_path_id: i64,
        content: &str,
    ) -> Result<(), Report<NoteDbError>>;
    async fn search_notes(&self, query: &str) -> Result<HashSet<String>, Report<NoteDbError>>;
    async fn get_all_notes_with_paths(&self) -> Result<Vec<(String, String)>, Report<NoteDbError>>;
    /// Returns all file paths with searchable content for fuzzy search.
    ///
    /// For paths with notes, returns the note content.
    /// For paths without notes, returns just the filename.
    async fn get_all_paths_for_fuzzy_search(&self) -> Result<Vec<(String, String)>, Report<NoteDbError>>;
    async fn close(&self);
}
