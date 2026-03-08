pub mod db;
pub mod editor;
pub mod path;

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use async_trait::async_trait;
use derive_more::Debug;
use error_stack::Report;
use wherror::Error;

#[derive(Debug, Error)]
#[error(debug)]
pub struct NoteDbError;

#[derive(Debug, Error)]
#[error(debug)]
pub struct EditorError;

#[derive(Debug, Error)]
#[error(debug)]
pub struct PathResolutionError;

#[async_trait]
pub trait NoteDb: Send + Sync {
    async fn get_or_create_file_path(&self, path: &str) -> Result<i64, Report<NoteDbError>>;
    async fn get_note(&self, file_path_id: i64) -> Result<Option<String>, Report<NoteDbError>>;
    async fn upsert_note(&self, file_path_id: i64, content: &str) -> Result<(), Report<NoteDbError>>;
    async fn search_notes(&self, query: &str) -> Result<HashSet<String>, Report<NoteDbError>>;
    async fn get_all_notes_with_paths(&self) -> Result<Vec<(String, String)>, Report<NoteDbError>>;
}

#[async_trait]
pub trait Editor: Send + Sync {
    async fn open(&self, initial_content: &str) -> Result<Option<String>, Report<EditorError>>;
}

#[async_trait]
pub trait PathResolver: Send + Sync {
    async fn resolve(&self, path: &Path) -> Result<PathBuf, Report<PathResolutionError>>;
}

#[derive(Debug, Clone)]
pub struct SystemServicesHandle {
    pub db: db::NoteDbWrapper,
    pub editor: editor::EditorWrapper,
    pub path_resolver: path::PathResolverWrapper,
}

impl SystemServicesHandle {
    /// Creates a new system services handle.
    ///
    /// # Errors
    ///
    /// Returns an error if the database connection or migration fails.
    pub async fn new(db_path: &str) -> Result<Self, Report<db::SqliteNoteDbError>> {
        let db = Arc::new(db::SqliteNoteDb::new(db_path).await?);
        let editor = Arc::new(editor::SystemEditor);
        let path_resolver = Arc::new(path::SystemPathResolver);

        Ok(Self {
            db: db::NoteDbWrapper::new(db),
            editor: editor::EditorWrapper::new(editor),
            path_resolver: path::PathResolverWrapper::new(path_resolver),
        })
    }
}
