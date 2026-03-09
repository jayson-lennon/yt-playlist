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

use crate::sources::SourceDbWrapper;
use crate::sources::db::sqlite::SqliteSourceDb;

#[derive(Debug, Error)]
#[error(debug)]
pub struct NoteDbError;

#[derive(Debug, Error)]
#[error(debug)]
pub struct EditorError;

#[derive(Debug, Error)]
#[error(debug)]
pub struct PathResolutionError;

/// Trait for note database operations.
///
/// Provides methods for storing, retrieving, and searching notes associated
/// with file paths in a persistent database.
#[async_trait]
pub trait NoteDb: Send + Sync {
    /// Gets or creates a file path record in the database.
    ///
    /// Returns the internal ID for the file path, creating a new record
    /// if one does not already exist.
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    async fn get_or_create_file_path(&self, path: &str) -> Result<i64, Report<NoteDbError>>;

    /// Retrieves the note content for a file path.
    ///
    /// Returns `Some(content)` if a note exists for the file path,
    /// or `None` if no note has been created.
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    async fn get_note(&self, file_path_id: i64) -> Result<Option<String>, Report<NoteDbError>>;

    /// Creates or updates the note content for a file path.
    ///
    /// If a note already exists for the file path, it will be replaced.
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    async fn upsert_note(
        &self,
        file_path_id: i64,
        content: &str,
    ) -> Result<(), Report<NoteDbError>>;

    /// Searches notes for content matching the query string.
    ///
    /// Returns a set of file paths whose notes contain the search query.
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    async fn search_notes(&self, query: &str) -> Result<HashSet<String>, Report<NoteDbError>>;

    /// Retrieves all notes with their associated file paths.
    ///
    /// Returns a vector of (`file_path`, `note_content`) pairs for all
    /// notes stored in the database.
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    async fn get_all_notes_with_paths(&self) -> Result<Vec<(String, String)>, Report<NoteDbError>>;
}

/// Trait for external editor integration.
///
/// Provides a method to open an external text editor for note editing.
#[async_trait]
pub trait Editor: Send + Sync {
    /// Opens an external editor with the provided initial content.
    ///
    /// Returns `Some(final_content)` if the editor was closed with saved changes,
    /// or `None` if the editor was closed without saving.
    ///
    /// # Errors
    ///
    /// Returns an error if the editor cannot be launched or fails unexpectedly.
    async fn open(&self, initial_content: &str) -> Result<Option<String>, Report<EditorError>>;
}

/// Trait for resolving file paths.
///
/// Provides path resolution to convert relative or symbolic paths
/// into canonical absolute paths.
#[async_trait]
pub trait PathResolver: Send + Sync {
    /// Resolves a path to its canonical absolute form.
    ///
    /// # Errors
    ///
    /// Returns an error if the path cannot be resolved (e.g., broken symlink,
    /// non-existent file).
    async fn resolve(&self, path: &Path) -> Result<PathBuf, Report<PathResolutionError>>;
}

#[derive(Debug, Clone)]
pub struct SystemServicesHandle {
    pub db: db::NoteDbWrapper,
    pub editor: editor::EditorWrapper,
    pub path_resolver: path::PathResolverWrapper,
    pub sources: SourceDbWrapper,
}

impl SystemServicesHandle {
    /// Creates a new system services handle with all dependencies.
    ///
    /// # Errors
    ///
    /// Returns an error if the database connection fails.
    pub async fn new(db_path: &str) -> Result<Self, Report<db::SqliteNoteDbError>> {
        let note_db = Arc::new(db::SqliteNoteDb::new(db_path).await?);
        let source_db = Arc::new(SqliteSourceDb::new(note_db.pool().clone()));
        let editor = Arc::new(editor::SystemEditor);
        let path_resolver = Arc::new(path::SystemPathResolver);

        Ok(Self {
            db: db::NoteDbWrapper::new(note_db),
            editor: editor::EditorWrapper::new(editor),
            path_resolver: path::PathResolverWrapper::new(path_resolver),
            sources: SourceDbWrapper::new(source_db),
        })
    }
}
