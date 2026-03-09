use std::collections::HashSet;

use async_trait::async_trait;
use error_stack::Report;
use wherror::Error;

pub mod backend;

pub use backend::{NoteDb, SqliteNoteDb, SqliteNoteDbError};

#[derive(Debug, Error)]
#[error(debug)]
pub struct NoteDbError;

#[async_trait]
pub trait NoteDbBackend: Send + Sync {
    async fn get_or_create_file_path(&self, path: &str) -> Result<i64, Report<NoteDbError>>;
    async fn get_note(&self, file_path_id: i64) -> Result<Option<String>, Report<NoteDbError>>;
    async fn upsert_note(
        &self,
        file_path_id: i64,
        content: &str,
    ) -> Result<(), Report<NoteDbError>>;
    async fn search_notes(&self, query: &str) -> Result<HashSet<String>, Report<NoteDbError>>;
    async fn get_all_notes_with_paths(&self) -> Result<Vec<(String, String)>, Report<NoteDbError>>;
}
