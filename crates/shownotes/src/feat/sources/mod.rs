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

mod db;

pub use db::{SqliteSourceDb, SqliteSourceDbError};

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use derive_more::Debug;
use error_stack::Report;
use wherror::Error;

#[derive(Debug, Error)]
#[error(debug)]
pub struct SourceDbError;

/// A source URL associated with a media file.
///
/// Represents provenance information tracking where a media file
/// originated from, such as a YouTube URL or other source link.
/// Optionally includes a human-readable label.
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
pub trait SourceDb: Send + Sync {
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

/// Service for managing source URL database operations.
///
/// Provides an interface for storing and retrieving source URLs
/// associated with media files. Delegates to a backend implementation
/// for actual database operations.
#[derive(Debug, Clone)]
pub struct SourceDbService {
    #[debug("<SourceDb>")]
    backend: Arc<dyn SourceDb>,
}

impl SourceDbService {
    pub fn new(backend: Arc<dyn SourceDb>) -> Self {
        Self { backend }
    }
}

#[async_trait]
impl SourceDb for SourceDbService {
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
