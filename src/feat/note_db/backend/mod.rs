use std::collections::HashSet;
use std::sync::Arc;

use async_trait::async_trait;
use derive_more::Debug;
use error_stack::Report;

use super::{NoteDbBackend, NoteDbError};

pub mod sqlite;

pub use sqlite::{SqliteNoteDb, SqliteNoteDbError};

#[derive(Debug, Clone)]
pub struct NoteDb {
    #[debug("<NoteDb>")]
    backend: Arc<dyn NoteDbBackend>,
}

impl NoteDb {
    pub fn new(backend: Arc<dyn NoteDbBackend>) -> Self {
        Self { backend }
    }
}

#[async_trait]
impl NoteDbBackend for NoteDb {
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
}
