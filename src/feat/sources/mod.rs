pub mod db;

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use derive_more::Debug;
use error_stack::Report;
use wherror::Error;

#[derive(Debug, Error)]
#[error(debug)]
pub struct SourceDbError;

#[derive(Debug, Clone)]
pub struct Source {
    pub id: i64,
    pub source_url: String,
    pub label: Option<String>,
}

/// Trait for source URL database operations.
///
/// Provides methods for managing source URLs associated with media files,
/// tracking where each file originated from.
#[async_trait]
pub trait SourceDbBackend: Send + Sync {
    /// Retrieves all source URLs for a file path.
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    async fn get_sources(&self, file_path_id: i64) -> Result<Vec<Source>, Report<SourceDbError>>;

    /// Sets the source URLs for a file path, replacing any existing sources.
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    async fn set_sources(
        &self,
        file_path_id: i64,
        urls: &[String],
    ) -> Result<(), Report<SourceDbError>>;

    /// Batch retrieves sources for multiple file paths.
    ///
    /// Returns a map from file path string to its associated sources.
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    async fn get_sources_for_paths(
        &self,
        paths: &[String],
    ) -> Result<HashMap<String, Vec<Source>>, Report<SourceDbError>>;
}

#[derive(Debug, Clone)]
pub struct SourceDb {
    #[debug("<SourceDb>")]
    backend: Arc<dyn SourceDbBackend>,
}

impl SourceDb {
    pub fn new(backend: Arc<dyn SourceDbBackend>) -> Self {
        Self { backend }
    }
}

#[async_trait]
impl SourceDbBackend for SourceDb {
    async fn get_sources(&self, file_path_id: i64) -> Result<Vec<Source>, Report<SourceDbError>> {
        self.backend.get_sources(file_path_id).await
    }

    async fn set_sources(
        &self,
        file_path_id: i64,
        urls: &[String],
    ) -> Result<(), Report<SourceDbError>> {
        self.backend.set_sources(file_path_id, urls).await
    }

    async fn get_sources_for_paths(
        &self,
        paths: &[String],
    ) -> Result<HashMap<String, Vec<Source>>, Report<SourceDbError>> {
        self.backend.get_sources_for_paths(paths).await
    }
}
